//! Core types for the Petit orchestrator.

mod job;
mod state;
mod task;

pub use job::{Job, JobMetadata};
pub use state::State;
pub use task::{OnFailure, RetryPolicy, Task, TaskOutput};
