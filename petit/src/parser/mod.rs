//! YAML parser for job definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::error::{PetitError, Result};
use crate::types::{Job, JobMetadata, OnFailure, RetryPolicy, Task};

/// YAML definition of a task.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskDefinition {
    /// Task identifier.
    pub id: String,
    /// Command to execute.
    pub command: String,
    /// Task dependencies (IDs of other tasks).
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Timeout in seconds or human-readable duration.
    #[serde(default)]
    pub timeout: Option<String>,
    /// Number of retry attempts.
    #[serde(default)]
    pub retries: Option<u32>,
    /// Behavior on failure.
    #[serde(default)]
    pub on_failure: Option<String>,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
}

/// YAML definition of job metadata.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MetadataDefinition {
    /// Job owner.
    #[serde(default)]
    pub owner: Option<String>,
    /// Priority level.
    #[serde(default)]
    pub priority: Option<String>,
    /// Additional labels.
    #[serde(default, flatten)]
    pub labels: HashMap<String, String>,
}

/// YAML definition of a job.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JobDefinition {
    /// Job name.
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Job metadata.
    #[serde(default)]
    pub metadata: Option<MetadataDefinition>,
    /// List of tasks.
    pub tasks: Vec<TaskDefinition>,
}

impl JobDefinition {
    /// Parse a job definition from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).map_err(PetitError::from)
    }

    /// Parse a job definition from a YAML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }

    /// Convert to a Job instance.
    pub fn into_job(self) -> Result<Job> {
        let mut job = Job::new(&self.name);

        if let Some(desc) = self.description {
            job.description = Some(desc);
        }

        if let Some(meta) = self.metadata {
            job.metadata = JobMetadata {
                owner: meta.owner,
                priority: meta.priority,
                labels: meta.labels,
            };
        }

        // Validate task IDs are unique
        let mut seen_ids = std::collections::HashSet::new();
        for task_def in &self.tasks {
            if !seen_ids.insert(&task_def.id) {
                return Err(PetitError::validation(format!(
                    "Duplicate task ID: {}",
                    task_def.id
                )));
            }
        }

        // Convert tasks
        for task_def in self.tasks {
            let task = task_def.into_task(job.id)?;
            job.add_task(task)?;
        }

        // Validate DAG
        job.validate_dag()?;

        Ok(job)
    }

    /// Serialize to YAML string.
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).map_err(PetitError::from)
    }
}

impl TaskDefinition {
    /// Convert to a Task instance.
    pub fn into_task(self, job_id: uuid::Uuid) -> Result<Task> {
        let mut task = Task::new(&self.id, job_id, &self.command);

        // Set dependencies
        for dep in self.depends_on {
            task.depends_on.push(dep);
        }

        // Parse timeout
        if let Some(timeout_str) = self.timeout {
            task.timeout = parse_duration(&timeout_str)?;
        }

        // Set retry policy
        if let Some(retries) = self.retries {
            task.retry_policy = RetryPolicy::new(retries + 1); // +1 because max_attempts includes initial try
        }

        // Parse on_failure behavior
        if let Some(on_failure) = self.on_failure {
            task.on_failure = match on_failure.to_lowercase().as_str() {
                "continue" => OnFailure::Continue,
                "stop" => OnFailure::Stop,
                other => {
                    return Err(PetitError::validation(format!(
                        "Invalid on_failure value: {}. Expected 'continue' or 'stop'",
                        other
                    )));
                }
            };
        }

        Ok(task)
    }
}

/// Parse a duration string like "300s", "5m", "1h".
fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return Err(PetitError::validation("Empty duration string"));
    }

    // Try parsing as plain number (seconds)
    if let Ok(secs) = s.parse::<u64>() {
        return Ok(Duration::from_secs(secs));
    }

    // Parse with suffix
    let (num_str, suffix) = s.split_at(s.len() - 1);
    let num: u64 = num_str
        .parse()
        .map_err(|_| PetitError::validation(format!("Invalid duration: {}", s)))?;

    let secs = match suffix {
        "s" => num,
        "m" => num * 60,
        "h" => num * 3600,
        "d" => num * 86400,
        _ => return Err(PetitError::validation(format!("Invalid duration suffix: {}", suffix))),
    };

    Ok(Duration::from_secs(secs))
}

/// Configuration definition in YAML.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigDefinition {
    /// Storage configuration.
    #[serde(default)]
    pub storage: StorageConfig,
    /// Executor configuration.
    #[serde(default)]
    pub executor: ExecutorConfig,
    /// Scheduler configuration.
    #[serde(default)]
    pub scheduler: SchedulerConfigDef,
    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for ConfigDefinition {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            executor: ExecutorConfig::default(),
            scheduler: SchedulerConfigDef::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Storage backend configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    /// Backend type: "memory" or "sqlite".
    #[serde(default = "default_backend")]
    pub backend: String,
    /// SQLite-specific configuration.
    #[serde(default)]
    pub sqlite: Option<SqliteConfig>,
}

fn default_backend() -> String {
    "memory".to_string()
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            sqlite: None,
        }
    }
}

/// SQLite configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqliteConfig {
    /// Database file path.
    pub path: String,
}

/// Executor configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutorConfig {
    /// Number of worker threads.
    #[serde(default = "default_workers")]
    pub workers: usize,
    /// Default task timeout.
    #[serde(default = "default_timeout")]
    pub task_timeout: String,
}

fn default_workers() -> usize {
    4
}

fn default_timeout() -> String {
    "3600s".to_string()
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            workers: default_workers(),
            task_timeout: default_timeout(),
        }
    }
}

/// Scheduler configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchedulerConfigDef {
    /// Poll interval.
    #[serde(default = "default_poll_interval")]
    pub poll_interval: String,
}

fn default_poll_interval() -> String {
    "100ms".to_string()
}

impl Default for SchedulerConfigDef {
    fn default() -> Self {
        Self {
            poll_interval: default_poll_interval(),
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Log level.
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log format.
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "pretty".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

impl ConfigDefinition {
    /// Parse configuration from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).map_err(PetitError::from)
    }

    /// Parse configuration from a file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JOB_YAML: &str = r#"
name: data-pipeline
description: Daily data processing pipeline
metadata:
  owner: data-team
  priority: high

tasks:
  - id: extract
    command: ./scripts/extract.sh
    timeout: 300s
    retries: 3

  - id: transform
    command: ./scripts/transform.sh
    depends_on:
      - extract
    timeout: 600s

  - id: load
    command: ./scripts/load.sh
    depends_on:
      - transform
    timeout: 300s

  - id: notify
    command: ./scripts/notify.sh
    depends_on:
      - load
    on_failure: continue
"#;

    #[test]
    fn test_parse_job_yaml() {
        let def = JobDefinition::from_yaml(SAMPLE_JOB_YAML).unwrap();

        assert_eq!(def.name, "data-pipeline");
        assert_eq!(def.description, Some("Daily data processing pipeline".to_string()));
        assert_eq!(def.tasks.len(), 4);
    }

    #[test]
    fn test_job_definition_to_job() {
        let def = JobDefinition::from_yaml(SAMPLE_JOB_YAML).unwrap();
        let job = def.into_job().unwrap();

        assert_eq!(job.name, "data-pipeline");
        assert_eq!(job.tasks.len(), 4);

        let extract = job.get_task("extract").unwrap();
        assert_eq!(extract.timeout, Duration::from_secs(300));
        assert_eq!(extract.retry_policy.max_attempts, 4); // 3 retries + 1 initial

        let notify = job.get_task("notify").unwrap();
        assert_eq!(notify.on_failure, OnFailure::Continue);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("60").unwrap(), Duration::from_secs(60));
        assert_eq!(parse_duration("300s").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
    }

    #[test]
    fn test_invalid_duration() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("5x").is_err());
    }

    #[test]
    fn test_duplicate_task_id_rejected() {
        let yaml = r#"
name: test
tasks:
  - id: task1
    command: echo 1
  - id: task1
    command: echo 2
"#;
        let def = JobDefinition::from_yaml(yaml).unwrap();
        let result = def.into_job();
        assert!(result.is_err());
    }

    #[test]
    fn test_cyclic_dependency_rejected() {
        let yaml = r#"
name: cyclic
tasks:
  - id: a
    command: echo a
    depends_on: [b]
  - id: b
    command: echo b
    depends_on: [a]
"#;
        let def = JobDefinition::from_yaml(yaml).unwrap();
        let result = def.into_job();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_parsing() {
        let yaml = r#"
storage:
  backend: sqlite
  sqlite:
    path: ./petit.db

executor:
  workers: 8
  task_timeout: 1h

scheduler:
  poll_interval: 50ms

logging:
  level: debug
  format: json
"#;
        let config = ConfigDefinition::from_yaml(yaml).unwrap();
        assert_eq!(config.storage.backend, "sqlite");
        assert_eq!(config.executor.workers, 8);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_default_config() {
        let config = ConfigDefinition::default();
        assert_eq!(config.storage.backend, "memory");
        assert_eq!(config.executor.workers, 4);
    }

    #[test]
    fn test_job_to_yaml() {
        let def = JobDefinition::from_yaml(SAMPLE_JOB_YAML).unwrap();
        let yaml = def.to_yaml().unwrap();

        // Parse it back
        let reparsed = JobDefinition::from_yaml(&yaml).unwrap();
        assert_eq!(reparsed.name, "data-pipeline");
    }
}
