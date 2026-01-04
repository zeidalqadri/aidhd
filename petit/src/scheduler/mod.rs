//! Job scheduler with DAG resolution and task queue management.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{PetitError, Result};
use crate::executor::{ExecutionResult, Executor, ExecutorConfig};
use crate::storage::{JobFilter, Storage};
use crate::types::{Job, OnFailure, State};

/// Configuration for the scheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// How often to poll for ready tasks.
    pub poll_interval: Duration,
    /// Executor configuration.
    pub executor_config: ExecutorConfig,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            executor_config: ExecutorConfig::default(),
        }
    }
}

/// Event emitted by the scheduler.
#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    /// A job was submitted.
    JobSubmitted { job_id: Uuid },
    /// A job started executing.
    JobStarted { job_id: Uuid },
    /// A job completed.
    JobCompleted { job_id: Uuid, success: bool },
    /// A task started executing.
    TaskStarted { job_id: Uuid, task_id: String },
    /// A task completed.
    TaskCompleted {
        job_id: Uuid,
        task_id: String,
        success: bool,
    },
}

/// The main scheduler that coordinates job and task execution.
pub struct Scheduler<S: Storage + 'static> {
    storage: Arc<S>,
    executor: Arc<Executor<S>>,
    config: SchedulerConfig,
    /// Jobs currently being processed.
    active_jobs: Arc<RwLock<HashSet<Uuid>>>,
    /// Channel to send events.
    event_tx: Option<mpsc::Sender<SchedulerEvent>>,
    /// Shutdown signal.
    shutdown: Arc<RwLock<bool>>,
}

impl<S: Storage + 'static> Scheduler<S> {
    /// Create a new scheduler.
    pub fn new(storage: Arc<S>, config: SchedulerConfig) -> Self {
        let executor = Arc::new(Executor::new(storage.clone(), config.executor_config.clone()));

        Self {
            storage,
            executor,
            config,
            active_jobs: Arc::new(RwLock::new(HashSet::new())),
            event_tx: None,
            shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Set an event channel for receiving scheduler events.
    pub fn with_event_channel(mut self, tx: mpsc::Sender<SchedulerEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Get a reference to the executor.
    pub fn executor(&self) -> &Arc<Executor<S>> {
        &self.executor
    }

    /// Submit a new job for execution.
    pub async fn submit(&self, job: Job) -> Result<Uuid> {
        // Validate DAG before submission
        job.validate_dag()?;

        let job_id = job.id;
        self.storage.create_job(job).await?;

        self.active_jobs.write().insert(job_id);
        self.emit_event(SchedulerEvent::JobSubmitted { job_id }).await;

        info!("Job {} submitted", job_id);
        Ok(job_id)
    }

    /// Get job status.
    pub async fn get_job(&self, job_id: Uuid) -> Result<Option<Job>> {
        self.storage.get_job(job_id).await
    }

    /// Cancel a job.
    pub async fn cancel_job(&self, job_id: Uuid) -> Result<()> {
        let mut job = self
            .storage
            .get_job(job_id)
            .await?
            .ok_or(PetitError::JobNotFound(job_id))?;

        // Cancel all active tasks
        for task in job.tasks.values_mut() {
            if task.state == State::Running || task.state == State::Queued {
                self.executor.cancel(job_id, &task.id).await?;
                task.state = State::Cancelled;
            }
        }

        job.state = State::Cancelled;
        job.finished_at = Some(chrono::Utc::now());
        self.storage.update_job(&job).await?;

        self.active_jobs.write().remove(&job_id);
        info!("Job {} cancelled", job_id);

        Ok(())
    }

    /// Retry a failed job.
    pub async fn retry_job(&self, job_id: Uuid) -> Result<()> {
        let mut job = self
            .storage
            .get_job(job_id)
            .await?
            .ok_or(PetitError::JobNotFound(job_id))?;

        if job.state != State::Failed {
            return Err(PetitError::validation(
                "Can only retry failed jobs",
            ));
        }

        // Reset failed tasks that can retry
        for task in job.tasks.values_mut() {
            if task.state == State::Failed && task.can_retry() {
                task.retry()?;
            }
        }

        // Reset job state
        job.state = State::Pending;
        job.finished_at = None;
        self.storage.update_job(&job).await?;

        self.active_jobs.write().insert(job_id);
        info!("Job {} queued for retry", job_id);

        Ok(())
    }

    /// Run the scheduler loop.
    pub async fn run(&self) -> Result<()> {
        info!("Scheduler starting");

        let mut result_rx = self
            .executor
            .take_result_receiver()
            .ok_or_else(|| PetitError::config("Result receiver already taken"))?;

        loop {
            // Check shutdown
            if *self.shutdown.read() {
                info!("Scheduler shutting down");
                break;
            }

            // Process any completed tasks
            while let Ok(result) = result_rx.try_recv() {
                self.handle_task_completion(result).await?;
            }

            // Schedule ready tasks from active jobs
            self.schedule_ready_tasks().await?;

            // Small sleep to prevent busy-waiting
            tokio::time::sleep(self.config.poll_interval).await;
        }

        Ok(())
    }

    /// Run one scheduling iteration (useful for testing).
    pub async fn tick(&self) -> Result<()> {
        self.schedule_ready_tasks().await
    }

    /// Signal shutdown.
    pub fn shutdown(&self) {
        *self.shutdown.write() = true;
    }

    /// Handle a completed task.
    async fn handle_task_completion(&self, result: ExecutionResult) -> Result<()> {
        debug!(
            "Task {} completed (success: {})",
            result.task_id, result.success
        );

        self.emit_event(SchedulerEvent::TaskCompleted {
            job_id: result.job_id,
            task_id: result.task_id.clone(),
            success: result.success,
        })
        .await;

        // Get and update the job
        let mut job = match self.storage.get_job(result.job_id).await? {
            Some(job) => job,
            None => {
                warn!("Job {} not found for completed task", result.job_id);
                return Ok(());
            }
        };

        // Check if task failed with stop-on-failure
        let task = job.get_task(&result.task_id);
        let should_stop = !result.success
            && task.map_or(true, |t| t.on_failure == OnFailure::Stop);

        if should_stop {
            // Cancel remaining queued tasks and fail the job
            for task in job.tasks.values_mut() {
                if task.state == State::Queued || task.state == State::Pending {
                    task.state = State::Cancelled;
                }
            }
            job.state = State::Failed;
            job.finished_at = Some(chrono::Utc::now());
            self.active_jobs.write().remove(&result.job_id);

            self.emit_event(SchedulerEvent::JobCompleted {
                job_id: result.job_id,
                success: false,
            })
            .await;
        } else {
            // Update job state
            job.update_state();

            if job.state.is_terminal() {
                self.active_jobs.write().remove(&result.job_id);
                self.emit_event(SchedulerEvent::JobCompleted {
                    job_id: result.job_id,
                    success: job.state == State::Success,
                })
                .await;
            }
        }

        self.storage.update_job(&job).await?;
        Ok(())
    }

    /// Schedule ready tasks for all active jobs.
    async fn schedule_ready_tasks(&self) -> Result<()> {
        let active_jobs: Vec<Uuid> = self.active_jobs.read().iter().cloned().collect();

        for job_id in active_jobs {
            if let Err(e) = self.schedule_job_tasks(job_id).await {
                error!("Error scheduling tasks for job {}: {}", job_id, e);
            }
        }

        Ok(())
    }

    /// Schedule ready tasks for a specific job.
    async fn schedule_job_tasks(&self, job_id: Uuid) -> Result<()> {
        let mut job = match self.storage.get_job(job_id).await? {
            Some(job) => job,
            None => {
                self.active_jobs.write().remove(&job_id);
                return Ok(());
            }
        };

        // Check if job is still active
        if job.state.is_terminal() {
            self.active_jobs.write().remove(&job_id);
            return Ok(());
        }

        // Queue pending tasks that have satisfied dependencies
        let queued = job.queue_ready_tasks();
        if !queued.is_empty() {
            debug!("Queued {} tasks for job {}", queued.len(), job_id);

            // Check if job just started
            if job.state == State::Pending {
                job.state = State::Queued;
                self.emit_event(SchedulerEvent::JobStarted { job_id }).await;
            }
        }

        // Get IDs of tasks ready to execute
        let ready_task_ids: Vec<String> = job
            .ready_tasks()
            .iter()
            .map(|t| t.id.clone())
            .collect();

        // Execute each ready task
        for task_id in ready_task_ids {
            if !self.executor.is_executing(job_id, &task_id) {
                // Update task state to Running
                if let Some(task_mut) = job.get_task_mut(&task_id) {
                    if task_mut.start().is_ok() {
                        info!("Starting task {} for job {}", task_id, job_id);
                        self.emit_event(SchedulerEvent::TaskStarted {
                            job_id,
                            task_id: task_id.clone(),
                        })
                        .await;

                        // Execute the task
                        if let Err(e) = self.executor.execute(job_id, task_id.clone()).await {
                            error!("Failed to execute task {}: {}", task_id, e);
                            task_mut.error = Some(e.to_string());
                            let _ = task_mut.transition_to(State::Failed);
                        }
                    }
                }
            }
        }

        // Update job state
        job.update_state();
        self.storage.update_job(&job).await?;

        Ok(())
    }

    /// Emit a scheduler event.
    async fn emit_event(&self, event: SchedulerEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

/// Orchestrator combining scheduler, storage, and configuration.
pub struct Orchestrator<S: Storage + 'static> {
    scheduler: Arc<Scheduler<S>>,
    storage: Arc<S>,
}

impl<S: Storage + 'static> Orchestrator<S> {
    /// Create a new orchestrator.
    pub fn new(storage: Arc<S>, config: SchedulerConfig) -> Self {
        let scheduler = Arc::new(Scheduler::new(storage.clone(), config));
        Self { scheduler, storage }
    }

    /// Get a reference to the scheduler.
    pub fn scheduler(&self) -> &Arc<Scheduler<S>> {
        &self.scheduler
    }

    /// Get a reference to storage.
    pub fn storage(&self) -> &Arc<S> {
        &self.storage
    }

    /// Submit a job.
    pub async fn submit(&self, job: Job) -> Result<Uuid> {
        self.scheduler.submit(job).await
    }

    /// Get a job by ID.
    pub async fn get_job(&self, job_id: Uuid) -> Result<Option<Job>> {
        self.storage.get_job(job_id).await
    }

    /// List jobs with optional filter.
    pub async fn list_jobs(&self, filter: JobFilter) -> Result<Vec<Job>> {
        self.storage.list_jobs(filter).await
    }

    /// Cancel a job.
    pub async fn cancel_job(&self, job_id: Uuid) -> Result<()> {
        self.scheduler.cancel_job(job_id).await
    }

    /// Retry a failed job.
    pub async fn retry_job(&self, job_id: Uuid) -> Result<()> {
        self.scheduler.retry_job(job_id).await
    }

    /// Run the orchestrator.
    pub async fn run(&self) -> Result<()> {
        self.scheduler.run().await
    }

    /// Signal shutdown.
    pub fn shutdown(&self) {
        self.scheduler.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;
    use crate::types::Task;

    fn create_pipeline_job() -> Job {
        let mut job = Job::new("test-pipeline");
        let job_id = job.id;

        let extract = Task::new("extract", job_id, "echo extract");
        let transform = Task::new("transform", job_id, "echo transform").depends_on("extract");
        let load = Task::new("load", job_id, "echo load").depends_on("transform");

        job.add_task(extract).unwrap();
        job.add_task(transform).unwrap();
        job.add_task(load).unwrap();

        job
    }

    #[tokio::test]
    async fn test_submit_job() {
        let storage = Arc::new(InMemoryStorage::new());
        let scheduler = Scheduler::new(storage.clone(), SchedulerConfig::default());

        let job = create_pipeline_job();
        let job_id = scheduler.submit(job).await.unwrap();

        let stored = storage.get_job(job_id).await.unwrap();
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_invalid_dag_rejected() {
        let storage = Arc::new(InMemoryStorage::new());
        let scheduler = Scheduler::new(storage, SchedulerConfig::default());

        let mut job = Job::new("cyclic");
        let job_id = job.id;
        job.add_task(Task::new("a", job_id, "echo a").depends_on("b"))
            .unwrap();
        job.add_task(Task::new("b", job_id, "echo b").depends_on("a"))
            .unwrap();

        let result = scheduler.submit(job).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PetitError::CyclicDependency(_)
        ));
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let storage = Arc::new(InMemoryStorage::new());
        let scheduler = Scheduler::new(storage.clone(), SchedulerConfig::default());

        let job = create_pipeline_job();
        let job_id = scheduler.submit(job).await.unwrap();

        scheduler.cancel_job(job_id).await.unwrap();

        let stored = storage.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(stored.state, State::Cancelled);
    }

    #[tokio::test]
    async fn test_orchestrator() {
        let storage = Arc::new(InMemoryStorage::new());
        let orchestrator = Orchestrator::new(storage, SchedulerConfig::default());

        let job = create_pipeline_job();
        let job_id = orchestrator.submit(job).await.unwrap();

        let retrieved = orchestrator.get_job(job_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_schedule_ready_tasks() {
        let storage = Arc::new(InMemoryStorage::new());
        let scheduler = Scheduler::new(storage.clone(), SchedulerConfig::default());

        let job = create_pipeline_job();
        let job_id = scheduler.submit(job).await.unwrap();

        // Run one tick to schedule tasks
        scheduler.tick().await.unwrap();

        let job = storage.get_job(job_id).await.unwrap().unwrap();
        // Extract should be queued or running (first task with no deps)
        let extract = job.get_task("extract").unwrap();
        assert!(
            extract.state == State::Queued || extract.state == State::Running,
            "Expected Queued or Running, got {:?}",
            extract.state
        );
    }
}
