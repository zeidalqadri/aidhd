//! Task executor with worker pool for parallel task execution.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use parking_lot::Mutex;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{PetitError, Result};
use crate::storage::Storage;
use crate::types::{State, Task, TaskOutput};

/// Result of a task execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Job ID the task belongs to.
    pub job_id: Uuid,
    /// Task ID.
    pub task_id: String,
    /// Whether the task succeeded.
    pub success: bool,
    /// Task output.
    pub output: TaskOutput,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Message sent to the executor.
#[derive(Debug)]
pub enum ExecutorMessage {
    /// Execute a task.
    Execute { job_id: Uuid, task_id: String },
    /// Shutdown the executor.
    Shutdown,
}

/// Configuration for the executor.
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Number of worker threads.
    pub workers: usize,
    /// Default task timeout.
    pub default_timeout: Duration,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            workers: 4,
            default_timeout: Duration::from_secs(3600),
        }
    }
}

/// Task executor that manages a pool of workers.
pub struct Executor<S: Storage> {
    storage: Arc<S>,
    config: ExecutorConfig,
    semaphore: Arc<Semaphore>,
    active_tasks: Arc<Mutex<Vec<(Uuid, String)>>>,
    result_tx: mpsc::Sender<ExecutionResult>,
    result_rx: Arc<Mutex<Option<mpsc::Receiver<ExecutionResult>>>>,
}

impl<S: Storage + 'static> Executor<S> {
    /// Create a new executor with the given storage and configuration.
    pub fn new(storage: Arc<S>, config: ExecutorConfig) -> Self {
        let (result_tx, result_rx) = mpsc::channel(100);

        Self {
            storage,
            semaphore: Arc::new(Semaphore::new(config.workers)),
            active_tasks: Arc::new(Mutex::new(Vec::new())),
            config,
            result_tx,
            result_rx: Arc::new(Mutex::new(Some(result_rx))),
        }
    }

    /// Take the result receiver (can only be called once).
    pub fn take_result_receiver(&self) -> Option<mpsc::Receiver<ExecutionResult>> {
        self.result_rx.lock().take()
    }

    /// Get the number of active tasks.
    pub fn active_count(&self) -> usize {
        self.active_tasks.lock().len()
    }

    /// Check if a specific task is currently executing.
    pub fn is_executing(&self, job_id: Uuid, task_id: &str) -> bool {
        self.active_tasks
            .lock()
            .iter()
            .any(|(j, t)| *j == job_id && t == task_id)
    }

    /// Execute a task.
    pub async fn execute(&self, job_id: Uuid, task_id: String) -> Result<()> {
        // Acquire a permit from the semaphore
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| PetitError::WorkerPoolExhausted)?;

        // Get the task from storage
        let task = self
            .storage
            .get_task(job_id, &task_id)
            .await?
            .ok_or_else(|| PetitError::TaskNotFound {
                job_id,
                task_id: task_id.clone(),
            })?;

        // Mark task as active
        self.active_tasks.lock().push((job_id, task_id.clone()));

        // Clone what we need for the spawned task
        let storage = self.storage.clone();
        let active_tasks = self.active_tasks.clone();
        let result_tx = self.result_tx.clone();
        let default_timeout = self.config.default_timeout;

        // Spawn the execution
        tokio::spawn(async move {
            let result = execute_task(&task, default_timeout).await;

            // Update task in storage
            let mut updated_task = task.clone();
            if result.success {
                updated_task.output = Some(result.output.clone());
                if let Err(e) = updated_task.transition_to(State::Success) {
                    error!("Failed to transition task to success: {}", e);
                }
            } else {
                updated_task.error = result.error.clone();
                if let Err(e) = updated_task.transition_to(State::Failed) {
                    error!("Failed to transition task to failed: {}", e);
                }
            }

            if let Err(e) = storage.update_task(job_id, &updated_task).await {
                error!("Failed to update task in storage: {}", e);
            }

            // Remove from active tasks
            active_tasks
                .lock()
                .retain(|(j, t)| !(*j == job_id && t == &task_id));

            // Send result
            if result_tx.send(result).await.is_err() {
                warn!("Result receiver dropped");
            }

            // Release the permit
            drop(permit);
        });

        Ok(())
    }

    /// Cancel a running task.
    pub async fn cancel(&self, job_id: Uuid, task_id: &str) -> Result<()> {
        // Note: In a real implementation, we'd track the JoinHandle and abort it
        // For now, we just update the storage state

        let mut task = self
            .storage
            .get_task(job_id, task_id)
            .await?
            .ok_or_else(|| PetitError::TaskNotFound {
                job_id,
                task_id: task_id.to_string(),
            })?;

        if task.state == State::Running || task.state == State::Queued {
            task.transition_to(State::Cancelled)?;
            self.storage.update_task(job_id, &task).await?;
        }

        Ok(())
    }
}

/// Execute a single task and return the result.
async fn execute_task(task: &Task, default_timeout: Duration) -> ExecutionResult {
    let task_timeout = if task.timeout.as_secs() > 0 {
        task.timeout
    } else {
        default_timeout
    };

    info!(
        "Executing task {} (job: {}, command: {})",
        task.id, task.job_id, task.command
    );

    match timeout(task_timeout, run_command(&task.command)).await {
        Ok(Ok(output)) => {
            let exit_code = output.exit_code;
            let success = exit_code.map_or(false, |c| c == 0);
            debug!(
                "Task {} completed with exit code {:?}",
                task.id, exit_code
            );

            ExecutionResult {
                job_id: task.job_id,
                task_id: task.id.clone(),
                success,
                output,
                error: if success {
                    None
                } else {
                    Some(format!("Exit code: {:?}", exit_code))
                },
            }
        }
        Ok(Err(e)) => {
            error!("Task {} execution error: {}", task.id, e);
            ExecutionResult {
                job_id: task.job_id,
                task_id: task.id.clone(),
                success: false,
                output: TaskOutput::default(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            error!("Task {} timed out after {:?}", task.id, task_timeout);
            ExecutionResult {
                job_id: task.job_id,
                task_id: task.id.clone(),
                success: false,
                output: TaskOutput::default(),
                error: Some(format!("Timeout after {} seconds", task_timeout.as_secs())),
            }
        }
    }
}

/// Run a shell command and capture its output.
async fn run_command(command: &str) -> Result<TaskOutput> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdout = String::new();
    let mut stderr = String::new();

    if let Some(mut stdout_pipe) = child.stdout.take() {
        stdout_pipe.read_to_string(&mut stdout).await?;
    }

    if let Some(mut stderr_pipe) = child.stderr.take() {
        stderr_pipe.read_to_string(&mut stderr).await?;
    }

    let status = child.wait().await?;

    Ok(TaskOutput {
        stdout,
        stderr,
        exit_code: status.code(),
    })
}

/// Custom command handler trait for extending task execution.
pub trait CommandHandler: Send + Sync {
    /// Execute a custom command.
    fn execute<'a>(&'a self, command: &'a str) -> BoxFuture<'a, Result<TaskOutput>>;
}

/// Default shell command handler.
pub struct ShellHandler;

impl CommandHandler for ShellHandler {
    fn execute<'a>(&'a self, command: &'a str) -> BoxFuture<'a, Result<TaskOutput>> {
        Box::pin(run_command(command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;
    use crate::types::Job;

    async fn setup_with_job() -> (Arc<InMemoryStorage>, Job) {
        let storage = Arc::new(InMemoryStorage::new());
        let mut job = Job::new("test-job");
        let job_id = job.id;

        let mut task = Task::new("echo-task", job_id, "echo 'hello world'");
        task.state = State::Running; // Set to running since executor expects this
        job.add_task(task).unwrap();

        storage.create_job(job.clone()).await.unwrap();
        (storage, job)
    }

    #[tokio::test]
    async fn test_executor_creation() {
        let storage = Arc::new(InMemoryStorage::new());
        let config = ExecutorConfig {
            workers: 2,
            default_timeout: Duration::from_secs(60),
        };
        let executor = Executor::new(storage, config);
        assert_eq!(executor.active_count(), 0);
    }

    #[tokio::test]
    async fn test_run_command() {
        let output = run_command("echo hello").await.unwrap();
        assert_eq!(output.stdout.trim(), "hello");
        assert_eq!(output.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_run_command_failure() {
        let output = run_command("exit 1").await.unwrap();
        assert_eq!(output.exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_execute_task() {
        let (storage, job) = setup_with_job().await;
        let executor = Executor::new(storage.clone(), ExecutorConfig::default());
        let mut rx = executor.take_result_receiver().unwrap();

        executor.execute(job.id, "echo-task".to_string()).await.unwrap();

        // Wait for result
        let result = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(result.success);
        assert!(result.output.stdout.contains("hello world"));
    }

    #[tokio::test]
    async fn test_execute_with_timeout() {
        let storage = Arc::new(InMemoryStorage::new());
        let mut job = Job::new("test-job");
        let job_id = job.id;

        let mut task = Task::new("slow-task", job_id, "sleep 10");
        task.state = State::Running;
        task.timeout = Duration::from_millis(100);
        job.add_task(task).unwrap();
        storage.create_job(job.clone()).await.unwrap();

        let config = ExecutorConfig {
            workers: 1,
            default_timeout: Duration::from_millis(100),
        };
        let executor = Executor::new(storage.clone(), config);
        let mut rx = executor.take_result_receiver().unwrap();

        executor.execute(job.id, "slow-task".to_string()).await.unwrap();

        let result = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("Timeout"));
    }

    #[tokio::test]
    async fn test_worker_pool_concurrency() {
        let storage = Arc::new(InMemoryStorage::new());
        let mut job = Job::new("test-job");
        let job_id = job.id;

        // Add multiple tasks
        for i in 0..5 {
            let mut task = Task::new(format!("task-{}", i), job_id, "echo done");
            task.state = State::Running;
            job.add_task(task).unwrap();
        }
        storage.create_job(job.clone()).await.unwrap();

        let config = ExecutorConfig {
            workers: 2, // Only 2 workers
            default_timeout: Duration::from_secs(60),
        };
        let executor = Executor::new(storage, config);
        let mut rx = executor.take_result_receiver().unwrap();

        // Submit all tasks
        for i in 0..5 {
            executor
                .execute(job_id, format!("task-{}", i))
                .await
                .unwrap();
        }

        // Collect all results
        let mut results = Vec::new();
        for _ in 0..5 {
            let result = tokio::time::timeout(Duration::from_secs(10), rx.recv())
                .await
                .unwrap()
                .unwrap();
            results.push(result);
        }

        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.success));
    }
}
