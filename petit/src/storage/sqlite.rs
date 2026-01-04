//! SQLite storage backend for persistent job and task storage.

use async_trait::async_trait;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use super::traits::{JobFilter, Storage};
use crate::error::{PetitError, Result};
use crate::types::{Job, State, Task};

/// SQLite storage backend for durable workflows.
pub struct SqliteStorage {
    conn: Mutex<Connection>,
}

impl SqliteStorage {
    /// Create a new SQLite storage with the given database path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Create an in-memory SQLite database (useful for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Initialize the database schema.
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                state TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                started_at TEXT,
                finished_at TEXT
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT NOT NULL,
                job_id TEXT NOT NULL,
                command TEXT NOT NULL,
                state TEXT NOT NULL,
                depends_on TEXT NOT NULL,
                timeout_secs INTEGER NOT NULL,
                retry_policy TEXT NOT NULL,
                on_failure TEXT NOT NULL,
                output TEXT,
                created_at TEXT NOT NULL,
                started_at TEXT,
                finished_at TEXT,
                error TEXT,
                PRIMARY KEY (job_id, id),
                FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_jobs_state ON jobs(state);
            CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at);
            CREATE INDEX IF NOT EXISTS idx_tasks_job_id ON tasks(job_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_state ON tasks(state);
            "#,
        )?;
        Ok(())
    }

    /// Serialize a job to database format.
    fn serialize_job(job: &Job) -> Result<(String, String, Option<String>, String, String, String, Option<String>, Option<String>)> {
        Ok((
            job.id.to_string(),
            job.name.clone(),
            job.description.clone(),
            serde_json::to_string(&job.state)?,
            serde_json::to_string(&job.metadata)?,
            job.created_at.to_rfc3339(),
            job.started_at.map(|t| t.to_rfc3339()),
            job.finished_at.map(|t| t.to_rfc3339()),
        ))
    }

    /// Deserialize a job from database row.
    fn deserialize_job(row: &rusqlite::Row) -> rusqlite::Result<Job> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let description: Option<String> = row.get(2)?;
        let state_json: String = row.get(3)?;
        let metadata_json: String = row.get(4)?;
        let created_at: String = row.get(5)?;
        let started_at: Option<String> = row.get(6)?;
        let finished_at: Option<String> = row.get(7)?;

        Ok(Job {
            id: Uuid::parse_str(&id).unwrap_or_default(),
            name,
            description,
            state: serde_json::from_str(&state_json).unwrap_or(State::Pending),
            tasks: HashMap::new(), // Tasks loaded separately
            metadata: serde_json::from_str(&metadata_json).unwrap_or_default(),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|t| t.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            started_at: started_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .ok()
            }),
            finished_at: finished_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .ok()
            }),
        })
    }

    /// Serialize a task to database format.
    fn serialize_task(task: &Task) -> Result<(String, String, String, String, String, i64, String, String, Option<String>, String, Option<String>, Option<String>, Option<String>)> {
        Ok((
            task.id.clone(),
            task.job_id.to_string(),
            task.command.clone(),
            serde_json::to_string(&task.state)?,
            serde_json::to_string(&task.depends_on)?,
            task.timeout.as_secs() as i64,
            serde_json::to_string(&task.retry_policy)?,
            serde_json::to_string(&task.on_failure)?,
            task.output.as_ref().map(|o| serde_json::to_string(o)).transpose()?,
            task.created_at.to_rfc3339(),
            task.started_at.map(|t| t.to_rfc3339()),
            task.finished_at.map(|t| t.to_rfc3339()),
            task.error.clone(),
        ))
    }

    /// Deserialize a task from database row.
    fn deserialize_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
        let id: String = row.get(0)?;
        let job_id: String = row.get(1)?;
        let command: String = row.get(2)?;
        let state_json: String = row.get(3)?;
        let depends_on_json: String = row.get(4)?;
        let timeout_secs: i64 = row.get(5)?;
        let retry_policy_json: String = row.get(6)?;
        let on_failure_json: String = row.get(7)?;
        let output_json: Option<String> = row.get(8)?;
        let created_at: String = row.get(9)?;
        let started_at: Option<String> = row.get(10)?;
        let finished_at: Option<String> = row.get(11)?;
        let error: Option<String> = row.get(12)?;

        Ok(Task {
            id,
            job_id: Uuid::parse_str(&job_id).unwrap_or_default(),
            command,
            state: serde_json::from_str(&state_json).unwrap_or(State::Pending),
            depends_on: serde_json::from_str(&depends_on_json).unwrap_or_default(),
            timeout: std::time::Duration::from_secs(timeout_secs as u64),
            retry_policy: serde_json::from_str(&retry_policy_json).unwrap_or_default(),
            on_failure: serde_json::from_str(&on_failure_json).unwrap_or_default(),
            output: output_json.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|t| t.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            started_at: started_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .ok()
            }),
            finished_at: finished_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .ok()
            }),
            error,
        })
    }

    /// Load tasks for a job.
    fn load_tasks(&self, job_id: Uuid) -> Result<HashMap<String, Task>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, job_id, command, state, depends_on, timeout_secs, retry_policy,
             on_failure, output, created_at, started_at, finished_at, error
             FROM tasks WHERE job_id = ?",
        )?;

        let tasks = stmt
            .query_map([job_id.to_string()], Self::deserialize_task)?
            .filter_map(|r| r.ok())
            .map(|t| (t.id.clone(), t))
            .collect();

        Ok(tasks)
    }

    /// Save tasks for a job.
    fn save_tasks(&self, conn: &Connection, job: &Job) -> Result<()> {
        // Delete existing tasks
        conn.execute("DELETE FROM tasks WHERE job_id = ?", [job.id.to_string()])?;

        // Insert new tasks
        let mut stmt = conn.prepare(
            "INSERT INTO tasks (id, job_id, command, state, depends_on, timeout_secs,
             retry_policy, on_failure, output, created_at, started_at, finished_at, error)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;

        for task in job.tasks.values() {
            let params = Self::serialize_task(task)?;
            stmt.execute(params![
                params.0, params.1, params.2, params.3, params.4, params.5,
                params.6, params.7, params.8, params.9, params.10, params.11, params.12
            ])?;
        }

        Ok(())
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn create_job(&self, job: Job) -> Result<Job> {
        let conn = self.conn.lock();

        // Check if job already exists
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM jobs WHERE id = ?",
                [job.id.to_string()],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if exists {
            return Err(PetitError::JobAlreadyExists(job.id));
        }

        let params = Self::serialize_job(&job)?;
        conn.execute(
            "INSERT INTO jobs (id, name, description, state, metadata, created_at, started_at, finished_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![params.0, params.1, params.2, params.3, params.4, params.5, params.6, params.7],
        )?;

        self.save_tasks(&conn, &job)?;

        Ok(job)
    }

    async fn get_job(&self, id: Uuid) -> Result<Option<Job>> {
        let job = {
            let conn = self.conn.lock();
            let mut stmt = conn.prepare(
                "SELECT id, name, description, state, metadata, created_at, started_at, finished_at
                 FROM jobs WHERE id = ?",
            )?;

            stmt.query_row([id.to_string()], Self::deserialize_job)
                .optional()?
        };

        if let Some(mut job) = job {
            job.tasks = self.load_tasks(id)?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn update_job(&self, job: &Job) -> Result<()> {
        let conn = self.conn.lock();

        let params = Self::serialize_job(job)?;
        let updated = conn.execute(
            "UPDATE jobs SET name = ?, description = ?, state = ?, metadata = ?,
             created_at = ?, started_at = ?, finished_at = ?
             WHERE id = ?",
            params![params.1, params.2, params.3, params.4, params.5, params.6, params.7, params.0],
        )?;

        if updated == 0 {
            return Err(PetitError::JobNotFound(job.id));
        }

        self.save_tasks(&conn, job)?;

        Ok(())
    }

    async fn delete_job(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.lock();

        // Tasks are deleted via CASCADE
        let deleted = conn.execute("DELETE FROM jobs WHERE id = ?", [id.to_string()])?;

        if deleted == 0 {
            return Err(PetitError::JobNotFound(id));
        }

        Ok(())
    }

    async fn list_jobs(&self, filter: JobFilter) -> Result<Vec<Job>> {
        let jobs = {
            let conn = self.conn.lock();

            let mut sql = String::from(
                "SELECT id, name, description, state, metadata, created_at, started_at, finished_at
                 FROM jobs WHERE 1=1",
            );
            let mut params: Vec<String> = Vec::new();

            if let Some(ref state) = filter.state {
                sql.push_str(" AND state = ?");
                params.push(serde_json::to_string(state)?);
            }

            if let Some(ref name) = filter.name {
                sql.push_str(" AND name LIKE ?");
                params.push(format!("%{}%", name));
            }

            sql.push_str(" ORDER BY created_at DESC");

            if let Some(limit) = filter.limit {
                sql.push_str(&format!(" LIMIT {}", limit));
            }

            if let Some(offset) = filter.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
            }

            let mut stmt = conn.prepare(&sql)?;

            let param_refs: Vec<&dyn rusqlite::ToSql> = params
                .iter()
                .map(|s| s as &dyn rusqlite::ToSql)
                .collect();

            let jobs: Vec<Job> = stmt
                .query_map(param_refs.as_slice(), Self::deserialize_job)?
                .filter_map(|r| r.ok())
                .collect();

            jobs
        };

        // Load tasks for each job
        let mut result = Vec::new();
        for mut job in jobs {
            job.tasks = self.load_tasks(job.id)?;
            result.push(job);
        }

        Ok(result)
    }

    async fn get_task(&self, job_id: Uuid, task_id: &str) -> Result<Option<Task>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, job_id, command, state, depends_on, timeout_secs, retry_policy,
             on_failure, output, created_at, started_at, finished_at, error
             FROM tasks WHERE job_id = ? AND id = ?",
        )?;

        let task = stmt
            .query_row([job_id.to_string(), task_id.to_string()], Self::deserialize_task)
            .optional()?;

        if task.is_none() {
            // Check if job exists
            let job_exists: bool = conn
                .query_row(
                    "SELECT 1 FROM jobs WHERE id = ?",
                    [job_id.to_string()],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);

            if !job_exists {
                return Err(PetitError::JobNotFound(job_id));
            }
        }

        Ok(task)
    }

    async fn update_task(&self, job_id: Uuid, task: &Task) -> Result<()> {
        let conn = self.conn.lock();

        let params = Self::serialize_task(task)?;
        let updated = conn.execute(
            "UPDATE tasks SET command = ?, state = ?, depends_on = ?, timeout_secs = ?,
             retry_policy = ?, on_failure = ?, output = ?, created_at = ?,
             started_at = ?, finished_at = ?, error = ?
             WHERE job_id = ? AND id = ?",
            params![
                params.2, params.3, params.4, params.5, params.6, params.7, params.8,
                params.9, params.10, params.11, params.12, job_id.to_string(), params.0
            ],
        )?;

        if updated == 0 {
            // Check if job exists
            let job_exists: bool = conn
                .query_row(
                    "SELECT 1 FROM jobs WHERE id = ?",
                    [job_id.to_string()],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);

            if !job_exists {
                return Err(PetitError::JobNotFound(job_id));
            }
            return Err(PetitError::TaskNotFound {
                job_id,
                task_id: task.id.clone(),
            });
        }

        Ok(())
    }

    async fn count_jobs_by_state(&self) -> Result<HashMap<State, usize>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT state, COUNT(*) FROM jobs GROUP BY state")?;

        let mut counts = HashMap::new();
        let rows = stmt.query_map([], |row| {
            let state_json: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((state_json, count as usize))
        })?;

        for row in rows {
            let (state_json, count) = row?;
            if let Ok(state) = serde_json::from_str(&state_json) {
                counts.insert(state, count);
            }
        }

        Ok(counts)
    }

    async fn clear(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM tasks", [])?;
        conn.execute("DELETE FROM jobs", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Task;

    async fn setup() -> SqliteStorage {
        SqliteStorage::in_memory().unwrap()
    }

    fn create_test_job(name: &str) -> Job {
        let mut job = Job::new(name);
        let job_id = job.id;
        job.add_task(Task::new("task1", job_id, "echo hello"))
            .unwrap();
        job.add_task(Task::new("task2", job_id, "echo world").depends_on("task1"))
            .unwrap();
        job
    }

    #[tokio::test]
    async fn test_create_and_get_job() {
        let storage = setup().await;
        let job = create_test_job("test-job");
        let job_id = job.id;

        storage.create_job(job).await.unwrap();

        let retrieved = storage.get_job(job_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "test-job");
        assert_eq!(retrieved.tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_update_job() {
        let storage = setup().await;
        let mut job = create_test_job("test-job");
        let job_id = job.id;

        storage.create_job(job.clone()).await.unwrap();

        job.state = State::Running;
        job.get_task_mut("task1").unwrap().state = State::Queued;
        storage.update_job(&job).await.unwrap();

        let retrieved = storage.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(retrieved.state, State::Running);
        assert_eq!(retrieved.get_task("task1").unwrap().state, State::Queued);
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
    }

    #[tokio::test]
    async fn test_task_persistence() {
        let storage = setup().await;
        let mut job = create_test_job("test-job");
        let job_id = job.id;

        // Add task with all fields
        let mut task = Task::new("task3", job_id, "echo test");
        task.state = State::Success;
        task.error = Some("previous error".to_string());
        job.add_task(task).unwrap();

        storage.create_job(job).await.unwrap();

        let retrieved = storage.get_job(job_id).await.unwrap().unwrap();
        let task3 = retrieved.get_task("task3").unwrap();
        assert_eq!(task3.state, State::Success);
        assert_eq!(task3.error, Some("previous error".to_string()));
    }

    #[tokio::test]
    async fn test_delete_cascades_to_tasks() {
        let storage = setup().await;
        let job = create_test_job("test-job");
        let job_id = job.id;

        storage.create_job(job).await.unwrap();
        storage.delete_job(job_id).await.unwrap();

        let task = storage.get_task(job_id, "task1").await;
        assert!(task.is_err() || task.unwrap().is_none());
    }
}
