//! In-memory storage backend.

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

use super::traits::{JobFilter, Storage};
use crate::error::{PetitError, Result};
use crate::types::{Job, State, Task};

/// In-memory storage backend for testing and ephemeral workflows.
#[derive(Debug, Default)]
pub struct InMemoryStorage {
    jobs: RwLock<HashMap<Uuid, Job>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage.
    pub fn new() -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn create_job(&self, job: Job) -> Result<Job> {
        let mut jobs = self.jobs.write();
        if jobs.contains_key(&job.id) {
            return Err(PetitError::JobAlreadyExists(job.id));
        }
        jobs.insert(job.id, job.clone());
        Ok(job)
    }

    async fn get_job(&self, id: Uuid) -> Result<Option<Job>> {
        let jobs = self.jobs.read();
        Ok(jobs.get(&id).cloned())
    }

    async fn update_job(&self, job: &Job) -> Result<()> {
        let mut jobs = self.jobs.write();
        if !jobs.contains_key(&job.id) {
            return Err(PetitError::JobNotFound(job.id));
        }
        jobs.insert(job.id, job.clone());
        Ok(())
    }

    async fn delete_job(&self, id: Uuid) -> Result<()> {
        let mut jobs = self.jobs.write();
        if jobs.remove(&id).is_none() {
            return Err(PetitError::JobNotFound(id));
        }
        Ok(())
    }

    async fn list_jobs(&self, filter: JobFilter) -> Result<Vec<Job>> {
        let jobs = self.jobs.read();
        let mut result: Vec<Job> = jobs
            .values()
            .filter(|job| {
                // Filter by state
                if let Some(state) = filter.state {
                    if job.state != state {
                        return false;
                    }
                }
                // Filter by name
                if let Some(ref name) = filter.name {
                    if !job.name.contains(name) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Sort by creation time (newest first)
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply pagination
        if let Some(offset) = filter.offset {
            if offset < result.len() {
                result = result.into_iter().skip(offset).collect();
            } else {
                result.clear();
            }
        }

        if let Some(limit) = filter.limit {
            result.truncate(limit);
        }

        Ok(result)
    }

    async fn get_task(&self, job_id: Uuid, task_id: &str) -> Result<Option<Task>> {
        let jobs = self.jobs.read();
        if let Some(job) = jobs.get(&job_id) {
            Ok(job.get_task(task_id).cloned())
        } else {
            Err(PetitError::JobNotFound(job_id))
        }
    }

    async fn update_task(&self, job_id: Uuid, task: &Task) -> Result<()> {
        let mut jobs = self.jobs.write();
        if let Some(job) = jobs.get_mut(&job_id) {
            if job.tasks.contains_key(&task.id) {
                job.tasks.insert(task.id.clone(), task.clone());
                Ok(())
            } else {
                Err(PetitError::TaskNotFound {
                    job_id,
                    task_id: task.id.clone(),
                })
            }
        } else {
            Err(PetitError::JobNotFound(job_id))
        }
    }

    async fn count_jobs_by_state(&self) -> Result<HashMap<State, usize>> {
        let jobs = self.jobs.read();
        let mut counts = HashMap::new();

        for job in jobs.values() {
            *counts.entry(job.state).or_insert(0) += 1;
        }

        Ok(counts)
    }

    async fn clear(&self) -> Result<()> {
        let mut jobs = self.jobs.write();
        jobs.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Task;

    async fn setup() -> InMemoryStorage {
        InMemoryStorage::new()
    }

    fn create_test_job(name: &str) -> Job {
        let mut job = Job::new(name);
        let job_id = job.id;
        job.add_task(Task::new("task1", job_id, "echo hello"))
            .unwrap();
        job
    }

    #[tokio::test]
    async fn test_create_and_get_job() {
        let storage = setup().await;
        let job = create_test_job("test-job");
        let job_id = job.id;

        let created = storage.create_job(job.clone()).await.unwrap();
        assert_eq!(created.id, job_id);

        let retrieved = storage.get_job(job_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-job");
    }

    #[tokio::test]
    async fn test_create_duplicate_job() {
        let storage = setup().await;
        let job = create_test_job("test-job");

        storage.create_job(job.clone()).await.unwrap();
        let result = storage.create_job(job).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PetitError::JobAlreadyExists(_)));
    }

    #[tokio::test]
    async fn test_update_job() {
        let storage = setup().await;
        let mut job = create_test_job("test-job");
        let job_id = job.id;

        storage.create_job(job.clone()).await.unwrap();

        job.state = State::Running;
        storage.update_job(&job).await.unwrap();

        let retrieved = storage.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(retrieved.state, State::Running);
    }

    #[tokio::test]
    async fn test_delete_job() {
        let storage = setup().await;
        let job = create_test_job("test-job");
        let job_id = job.id;

        storage.create_job(job).await.unwrap();
        storage.delete_job(job_id).await.unwrap();

        let retrieved = storage.get_job(job_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_jobs_with_filter() {
        let storage = setup().await;

        let mut job1 = create_test_job("pipeline-1");
        let mut job2 = create_test_job("pipeline-2");
        let job3 = create_test_job("other-job");

        job1.state = State::Running;
        job2.state = State::Running;

        storage.create_job(job1).await.unwrap();
        storage.create_job(job2).await.unwrap();
        storage.create_job(job3).await.unwrap();

        // Filter by state
        let running = storage
            .list_jobs(JobFilter::new().with_state(State::Running))
            .await
            .unwrap();
        assert_eq!(running.len(), 2);

        // Filter by name
        let pipelines = storage
            .list_jobs(JobFilter::new().with_name("pipeline"))
            .await
            .unwrap();
        assert_eq!(pipelines.len(), 2);

        // Pagination
        let paginated = storage
            .list_jobs(JobFilter::new().with_limit(2))
            .await
            .unwrap();
        assert_eq!(paginated.len(), 2);
    }

    #[tokio::test]
    async fn test_get_and_update_task() {
        let storage = setup().await;
        let job = create_test_job("test-job");
        let job_id = job.id;

        storage.create_job(job).await.unwrap();

        let task = storage.get_task(job_id, "task1").await.unwrap();
        assert!(task.is_some());
        assert_eq!(task.unwrap().id, "task1");

        let mut updated_task = Task::new("task1", job_id, "echo updated");
        updated_task.state = State::Running;
        storage.update_task(job_id, &updated_task).await.unwrap();

        let task = storage.get_task(job_id, "task1").await.unwrap().unwrap();
        assert_eq!(task.state, State::Running);
    }

    #[tokio::test]
    async fn test_count_jobs_by_state() {
        let storage = setup().await;

        let mut job1 = create_test_job("job1");
        let mut job2 = create_test_job("job2");
        let job3 = create_test_job("job3");

        job1.state = State::Running;
        job2.state = State::Running;

        storage.create_job(job1).await.unwrap();
        storage.create_job(job2).await.unwrap();
        storage.create_job(job3).await.unwrap();

        let counts = storage.count_jobs_by_state().await.unwrap();
        assert_eq!(counts.get(&State::Running), Some(&2));
        assert_eq!(counts.get(&State::Pending), Some(&1));
    }

    #[tokio::test]
    async fn test_clear() {
        let storage = setup().await;

        storage.create_job(create_test_job("job1")).await.unwrap();
        storage.create_job(create_test_job("job2")).await.unwrap();

        storage.clear().await.unwrap();

        let jobs = storage.list_jobs(JobFilter::new()).await.unwrap();
        assert!(jobs.is_empty());
    }
}
