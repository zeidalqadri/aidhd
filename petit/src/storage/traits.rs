//! Storage trait definitions.

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{Job, State, Task};

/// Filter options for listing jobs.
#[derive(Debug, Default, Clone)]
pub struct JobFilter {
    /// Filter by state.
    pub state: Option<State>,
    /// Filter by name (substring match).
    pub name: Option<String>,
    /// Maximum number of results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

impl JobFilter {
    /// Create a new filter with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by state.
    pub fn with_state(mut self, state: State) -> Self {
        self.state = Some(state);
        self
    }

    /// Filter by name substring.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Limit results.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set offset for pagination.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Storage backend trait for job and task persistence.
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store a new job.
    async fn create_job(&self, job: Job) -> Result<Job>;

    /// Get a job by ID.
    async fn get_job(&self, id: Uuid) -> Result<Option<Job>>;

    /// Update an existing job.
    async fn update_job(&self, job: &Job) -> Result<()>;

    /// Delete a job and all its tasks.
    async fn delete_job(&self, id: Uuid) -> Result<()>;

    /// List jobs with optional filtering.
    async fn list_jobs(&self, filter: JobFilter) -> Result<Vec<Job>>;

    /// Get a task by job ID and task ID.
    async fn get_task(&self, job_id: Uuid, task_id: &str) -> Result<Option<Task>>;

    /// Update a task.
    async fn update_task(&self, job_id: Uuid, task: &Task) -> Result<()>;

    /// Get jobs in a specific state.
    async fn get_jobs_by_state(&self, state: State) -> Result<Vec<Job>> {
        self.list_jobs(JobFilter::new().with_state(state)).await
    }

    /// Count jobs by state.
    async fn count_jobs_by_state(&self) -> Result<std::collections::HashMap<State, usize>>;

    /// Clear all data (useful for testing).
    async fn clear(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_filter_builder() {
        let filter = JobFilter::new()
            .with_state(State::Running)
            .with_name("pipeline")
            .with_limit(10)
            .with_offset(5);

        assert_eq!(filter.state, Some(State::Running));
        assert_eq!(filter.name, Some("pipeline".to_string()));
        assert_eq!(filter.limit, Some(10));
        assert_eq!(filter.offset, Some(5));
    }
}
