//! # Petit - A Lightweight Task Orchestrator
//!
//! Petit is a memory-efficient, lightweight task orchestrator written in Rust.
//! It provides a simple yet powerful way to define, schedule, and execute jobs
//! composed of dependent tasks.
//!
//! ## Features
//!
//! - **DAG-based task scheduling**: Define dependencies between tasks
//! - **Multiple storage backends**: In-memory and SQLite support
//! - **YAML job definitions**: Easy-to-write job configuration
//! - **CLI interface**: Full job and task management
//! - **TUI dashboard**: Real-time monitoring with keyboard navigation
//! - **Retry policies**: Configurable retry with exponential backoff
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use petit::{Job, Task, InMemoryStorage, Orchestrator, SchedulerConfig};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> petit::Result<()> {
//!     // Create storage backend
//!     let storage = Arc::new(InMemoryStorage::new());
//!
//!     // Create orchestrator
//!     let orchestrator = Orchestrator::new(storage.clone(), SchedulerConfig::default());
//!
//!     // Create a job with tasks
//!     let mut job = Job::new("my-pipeline");
//!     let job_id = job.id;
//!
//!     job.add_task(Task::new("extract", job_id, "echo 'extracting data'"))?;
//!     job.add_task(Task::new("transform", job_id, "echo 'transforming'").depends_on("extract"))?;
//!     job.add_task(Task::new("load", job_id, "echo 'loading'").depends_on("transform"))?;
//!
//!     // Submit the job
//!     orchestrator.submit(job).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod cli;
pub mod error;
pub mod executor;
pub mod parser;
pub mod scheduler;
pub mod storage;
pub mod tui;
pub mod types;

// Re-exports for convenience
pub use error::{PetitError, Result};
pub use executor::{ExecutionResult, Executor, ExecutorConfig};
pub use parser::{ConfigDefinition, JobDefinition, TaskDefinition};
pub use scheduler::{Orchestrator, Scheduler, SchedulerConfig, SchedulerEvent};
pub use storage::{InMemoryStorage, JobFilter, SqliteStorage, Storage};
pub use types::{Job, JobMetadata, OnFailure, RetryPolicy, State, Task, TaskOutput};
