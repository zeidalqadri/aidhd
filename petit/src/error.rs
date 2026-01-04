//! Error types for the Petit orchestrator.

use thiserror::Error;
use uuid::Uuid;

/// Result type alias for Petit operations.
pub type Result<T> = std::result::Result<T, PetitError>;

/// Main error type for Petit orchestrator.
#[derive(Error, Debug)]
pub enum PetitError {
    #[error("Job not found: {0}")]
    JobNotFound(Uuid),

    #[error("Task not found: job={job_id}, task={task_id}")]
    TaskNotFound { job_id: Uuid, task_id: String },

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Cyclic dependency detected in job: {0}")]
    CyclicDependency(String),

    #[error("Task dependency not found: {task_id} depends on {dependency}")]
    DependencyNotFound { task_id: String, dependency: String },

    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Task timeout after {0} seconds")]
    TaskTimeout(u64),

    #[error("YAML parsing error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Job already exists: {0}")]
    JobAlreadyExists(Uuid),

    #[error("Worker pool exhausted")]
    WorkerPoolExhausted,

    #[error("Orchestrator shutdown")]
    Shutdown,
}

impl PetitError {
    /// Create a storage error with a message.
    pub fn storage(msg: impl Into<String>) -> Self {
        PetitError::Storage(msg.into())
    }

    /// Create a config error with a message.
    pub fn config(msg: impl Into<String>) -> Self {
        PetitError::Config(msg.into())
    }

    /// Create a validation error with a message.
    pub fn validation(msg: impl Into<String>) -> Self {
        PetitError::Validation(msg.into())
    }
}

impl From<String> for PetitError {
    fn from(msg: String) -> Self {
        PetitError::Validation(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PetitError::JobNotFound(Uuid::nil());
        assert!(err.to_string().contains("Job not found"));

        let err = PetitError::InvalidStateTransition {
            from: "Pending".to_string(),
            to: "Success".to_string(),
        };
        assert!(err.to_string().contains("Invalid state transition"));
    }

    #[test]
    fn test_error_helpers() {
        let err = PetitError::storage("test storage error");
        assert!(matches!(err, PetitError::Storage(_)));

        let err = PetitError::config("test config error");
        assert!(matches!(err, PetitError::Config(_)));
    }
}
