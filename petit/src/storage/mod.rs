//! Storage backends for job and task persistence.

mod memory;
mod sqlite;
mod traits;

pub use memory::InMemoryStorage;
pub use sqlite::SqliteStorage;
pub use traits::{JobFilter, Storage};
