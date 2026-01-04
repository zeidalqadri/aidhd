//! Task type and related structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use super::state::State;
use crate::error::{PetitError, Result};

/// Configuration for task retry behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Current attempt number (1-indexed).
    pub current_attempt: u32,
    /// Delay between retries.
    #[serde(with = "humantime_serde")]
    pub delay: Duration,
    /// Whether to use exponential backoff.
    pub exponential_backoff: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            current_attempt: 1,
            delay: Duration::from_secs(5),
            exponential_backoff: true,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with specified max attempts.
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// Create a policy with no retries.
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            current_attempt: 1,
            delay: Duration::ZERO,
            exponential_backoff: false,
        }
    }

    /// Check if more retry attempts are available.
    pub fn can_retry(&self) -> bool {
        self.current_attempt < self.max_attempts
    }

    /// Get the delay for the current retry attempt.
    pub fn current_delay(&self) -> Duration {
        if self.exponential_backoff {
            self.delay * 2u32.pow(self.current_attempt.saturating_sub(1))
        } else {
            self.delay
        }
    }

    /// Increment the attempt counter.
    pub fn increment_attempt(&mut self) {
        self.current_attempt = self.current_attempt.saturating_add(1);
    }
}

/// Behavior when a task fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnFailure {
    /// Stop the job and mark it as failed (default).
    #[default]
    Stop,
    /// Continue with dependent tasks.
    Continue,
}

/// Task execution output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskOutput {
    /// Standard output from the task.
    pub stdout: String,
    /// Standard error from the task.
    pub stderr: String,
    /// Exit code if the task was a command.
    pub exit_code: Option<i32>,
}

/// A single unit of work within a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier within the parent job.
    pub id: String,
    /// Parent job ID.
    pub job_id: Uuid,
    /// Command to execute.
    pub command: String,
    /// Current state of the task.
    pub state: State,
    /// IDs of tasks this task depends on.
    pub depends_on: Vec<String>,
    /// Task execution timeout.
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    /// Retry policy for this task.
    pub retry_policy: RetryPolicy,
    /// Behavior when this task fails.
    pub on_failure: OnFailure,
    /// Task execution output.
    pub output: Option<TaskOutput>,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
    /// When the task started executing.
    pub started_at: Option<DateTime<Utc>>,
    /// When the task finished.
    pub finished_at: Option<DateTime<Utc>>,
    /// Optional error message if failed.
    pub error: Option<String>,
}

impl Task {
    /// Create a new task with the given ID and command.
    pub fn new(id: impl Into<String>, job_id: Uuid, command: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            job_id,
            command: command.into(),
            state: State::Pending,
            depends_on: Vec::new(),
            timeout: Duration::from_secs(3600),
            retry_policy: RetryPolicy::default(),
            on_failure: OnFailure::default(),
            output: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    /// Add a dependency on another task.
    pub fn depends_on(mut self, task_id: impl Into<String>) -> Self {
        self.depends_on.push(task_id.into());
        self
    }

    /// Set the timeout for this task.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the retry policy.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Set the on_failure behavior.
    pub fn with_on_failure(mut self, on_failure: OnFailure) -> Self {
        self.on_failure = on_failure;
        self
    }

    /// Transition to a new state.
    pub fn transition_to(&mut self, new_state: State) -> Result<()> {
        if !self.state.can_transition_to(new_state) {
            return Err(PetitError::InvalidStateTransition {
                from: self.state.to_string(),
                to: new_state.to_string(),
            });
        }

        // Update timestamps based on transition
        match new_state {
            State::Running => {
                self.started_at = Some(Utc::now());
            }
            State::Success | State::Failed | State::Cancelled => {
                self.finished_at = Some(Utc::now());
            }
            _ => {}
        }

        self.state = new_state;
        Ok(())
    }

    /// Mark the task as running.
    pub fn start(&mut self) -> Result<()> {
        self.transition_to(State::Running)
    }

    /// Mark the task as successful.
    pub fn succeed(&mut self, output: TaskOutput) -> Result<()> {
        self.output = Some(output);
        self.transition_to(State::Success)
    }

    /// Mark the task as failed.
    pub fn fail(&mut self, error: impl Into<String>) -> Result<()> {
        self.error = Some(error.into());
        self.transition_to(State::Failed)
    }

    /// Cancel the task.
    pub fn cancel(&mut self) -> Result<()> {
        self.transition_to(State::Cancelled)
    }

    /// Queue the task for execution.
    pub fn queue(&mut self) -> Result<()> {
        self.transition_to(State::Queued)
    }

    /// Check if the task can be retried.
    pub fn can_retry(&self) -> bool {
        self.state == State::Failed && self.retry_policy.can_retry()
    }

    /// Retry the task (transition from Failed to Queued).
    pub fn retry(&mut self) -> Result<()> {
        if !self.can_retry() {
            return Err(PetitError::InvalidStateTransition {
                from: self.state.to_string(),
                to: "Queued (retry)".to_string(),
            });
        }
        self.retry_policy.increment_attempt();
        self.error = None;
        self.transition_to(State::Queued)
    }

    /// Get the execution duration if available.
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.finished_at) {
            (Some(start), Some(end)) => Some(end - start),
            (Some(start), None) if self.state == State::Running => Some(Utc::now() - start),
            _ => None,
        }
    }

    /// Check if all dependencies are satisfied (all in Success state).
    pub fn dependencies_satisfied(&self, completed_tasks: &[&str]) -> bool {
        self.depends_on.iter().all(|dep| completed_tasks.contains(&dep.as_str()))
    }
}

/// Humantime serde module for Duration serialization.
mod humantime_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let job_id = Uuid::new_v4();
        let task = Task::new("extract", job_id, "echo hello");

        assert_eq!(task.id, "extract");
        assert_eq!(task.job_id, job_id);
        assert_eq!(task.command, "echo hello");
        assert_eq!(task.state, State::Pending);
        assert!(task.depends_on.is_empty());
    }

    #[test]
    fn test_task_builder_pattern() {
        let job_id = Uuid::new_v4();
        let task = Task::new("transform", job_id, "transform.sh")
            .depends_on("extract")
            .with_timeout(Duration::from_secs(600))
            .with_on_failure(OnFailure::Continue);

        assert_eq!(task.depends_on, vec!["extract"]);
        assert_eq!(task.timeout, Duration::from_secs(600));
        assert_eq!(task.on_failure, OnFailure::Continue);
    }

    #[test]
    fn test_task_state_transitions() {
        let job_id = Uuid::new_v4();
        let mut task = Task::new("test", job_id, "test.sh");

        // Pending -> Queued
        assert!(task.queue().is_ok());
        assert_eq!(task.state, State::Queued);

        // Queued -> Running
        assert!(task.start().is_ok());
        assert_eq!(task.state, State::Running);
        assert!(task.started_at.is_some());

        // Running -> Success
        let output = TaskOutput {
            stdout: "done".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
        };
        assert!(task.succeed(output).is_ok());
        assert_eq!(task.state, State::Success);
        assert!(task.finished_at.is_some());
    }

    #[test]
    fn test_task_failure_and_retry() {
        let job_id = Uuid::new_v4();
        let mut task = Task::new("test", job_id, "fail.sh")
            .with_retry_policy(RetryPolicy::new(3));

        task.queue().unwrap();
        task.start().unwrap();
        task.fail("command failed").unwrap();

        assert_eq!(task.state, State::Failed);
        assert!(task.can_retry());

        // First retry
        assert!(task.retry().is_ok());
        assert_eq!(task.state, State::Queued);
        assert_eq!(task.retry_policy.current_attempt, 2);
    }

    #[test]
    fn test_retry_policy_exponential_backoff() {
        let mut policy = RetryPolicy {
            max_attempts: 5,
            current_attempt: 1,
            delay: Duration::from_secs(1),
            exponential_backoff: true,
        };

        assert_eq!(policy.current_delay(), Duration::from_secs(1));
        policy.increment_attempt();
        assert_eq!(policy.current_delay(), Duration::from_secs(2));
        policy.increment_attempt();
        assert_eq!(policy.current_delay(), Duration::from_secs(4));
    }

    #[test]
    fn test_dependencies_satisfied() {
        let job_id = Uuid::new_v4();
        let task = Task::new("load", job_id, "load.sh")
            .depends_on("extract")
            .depends_on("transform");

        assert!(!task.dependencies_satisfied(&["extract"]));
        assert!(task.dependencies_satisfied(&["extract", "transform"]));
        assert!(task.dependencies_satisfied(&["extract", "transform", "other"]));
    }

    #[test]
    fn test_invalid_state_transition() {
        let job_id = Uuid::new_v4();
        let mut task = Task::new("test", job_id, "test.sh");

        // Can't go directly from Pending to Running
        let result = task.start();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PetitError::InvalidStateTransition { .. }));
    }
}
