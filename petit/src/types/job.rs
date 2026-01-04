//! Job type representing a collection of tasks with dependencies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::state::State;
use super::task::Task;
use crate::error::{PetitError, Result};

/// Metadata associated with a job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobMetadata {
    /// Job owner or creator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Priority level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    /// Additional labels.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
}

/// A job is a collection of tasks forming a DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique identifier for the job.
    pub id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Current state of the job.
    pub state: State,
    /// Tasks in this job, keyed by task ID.
    pub tasks: HashMap<String, Task>,
    /// Job metadata.
    #[serde(default)]
    pub metadata: JobMetadata,
    /// When the job was created.
    pub created_at: DateTime<Utc>,
    /// When the job started executing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// When the job finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
}

impl Job {
    /// Create a new job with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            state: State::Pending,
            tasks: HashMap::new(),
            metadata: JobMetadata::default(),
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
        }
    }

    /// Create a job with a specific ID.
    pub fn with_id(id: Uuid, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: None,
            state: State::Pending,
            tasks: HashMap::new(),
            metadata: JobMetadata::default(),
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
        }
    }

    /// Set the job description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the job metadata.
    pub fn with_metadata(mut self, metadata: JobMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a task to the job.
    pub fn add_task(&mut self, task: Task) -> Result<()> {
        if self.tasks.contains_key(&task.id) {
            return Err(PetitError::validation(format!(
                "Task '{}' already exists in job",
                task.id
            )));
        }
        self.tasks.insert(task.id.clone(), task);
        Ok(())
    }

    /// Get a task by ID.
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    /// Get a mutable reference to a task by ID.
    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut Task> {
        self.tasks.get_mut(task_id)
    }

    /// Get all tasks in the job.
    pub fn tasks(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }

    /// Validate the job's task dependencies form a valid DAG.
    pub fn validate_dag(&self) -> Result<()> {
        // Check all dependencies exist
        for task in self.tasks.values() {
            for dep in &task.depends_on {
                if !self.tasks.contains_key(dep) {
                    return Err(PetitError::DependencyNotFound {
                        task_id: task.id.clone(),
                        dependency: dep.clone(),
                    });
                }
            }
        }

        // Check for cycles using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in self.tasks.keys() {
            if self.has_cycle(task_id, &mut visited, &mut rec_stack) {
                return Err(PetitError::CyclicDependency(self.name.clone()));
            }
        }

        Ok(())
    }

    /// Check for cycles starting from a given task.
    fn has_cycle(
        &self,
        task_id: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        if rec_stack.contains(task_id) {
            return true;
        }
        if visited.contains(task_id) {
            return false;
        }

        visited.insert(task_id.to_string());
        rec_stack.insert(task_id.to_string());

        if let Some(task) = self.tasks.get(task_id) {
            for dep in &task.depends_on {
                if self.has_cycle(dep, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(task_id);
        false
    }

    /// Get tasks that are ready to execute (all dependencies satisfied).
    pub fn ready_tasks(&self) -> Vec<&Task> {
        let completed: Vec<&str> = self
            .tasks
            .values()
            .filter(|t| t.state == State::Success)
            .map(|t| t.id.as_str())
            .collect();

        self.tasks
            .values()
            .filter(|t| t.state == State::Queued && t.dependencies_satisfied(&completed))
            .collect()
    }

    /// Get the topological order of tasks.
    pub fn topological_order(&self) -> Result<Vec<&str>> {
        self.validate_dag()?;

        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize
        for task in self.tasks.values() {
            in_degree.entry(task.id.as_str()).or_insert(0);
            adj.entry(task.id.as_str()).or_insert_with(Vec::new);
            for dep in &task.depends_on {
                adj.entry(dep.as_str())
                    .or_insert_with(Vec::new)
                    .push(task.id.as_str());
                *in_degree.entry(task.id.as_str()).or_insert(0) += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut result = Vec::new();

        while let Some(node) = queue.pop() {
            result.push(node);
            if let Some(neighbors) = adj.get(node) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Update job state based on task states.
    pub fn update_state(&mut self) {
        let task_states: Vec<State> = self.tasks.values().map(|t| t.state).collect();

        if task_states.is_empty() {
            return;
        }

        let new_state = if task_states.iter().all(|s| *s == State::Success) {
            State::Success
        } else if task_states.iter().any(|s| *s == State::Failed) {
            // Check if any non-continue-on-failure task failed
            let critical_failure = self.tasks.values().any(|t| {
                t.state == State::Failed && t.on_failure == super::task::OnFailure::Stop
            });
            if critical_failure {
                State::Failed
            } else if task_states
                .iter()
                .all(|s| s.is_terminal() || *s == State::Pending)
            {
                State::Success
            } else {
                self.state
            }
        } else if task_states.iter().any(|s| *s == State::Cancelled) {
            State::Cancelled
        } else if task_states.iter().any(|s| *s == State::Running) {
            State::Running
        } else if task_states.iter().any(|s| *s == State::Queued) {
            State::Queued
        } else {
            self.state
        };

        // Update timestamps
        if new_state == State::Running && self.started_at.is_none() {
            self.started_at = Some(Utc::now());
        }
        if new_state.is_terminal() && self.finished_at.is_none() {
            self.finished_at = Some(Utc::now());
        }

        self.state = new_state;
    }

    /// Queue all pending tasks that have no dependencies or whose dependencies are met.
    pub fn queue_ready_tasks(&mut self) -> Vec<String> {
        let completed: Vec<String> = self
            .tasks
            .values()
            .filter(|t| t.state == State::Success)
            .map(|t| t.id.clone())
            .collect();

        let completed_refs: Vec<&str> = completed.iter().map(|s| s.as_str()).collect();

        let ready: Vec<String> = self
            .tasks
            .values()
            .filter(|t| t.state == State::Pending && t.dependencies_satisfied(&completed_refs))
            .map(|t| t.id.clone())
            .collect();

        for task_id in &ready {
            if let Some(task) = self.tasks.get_mut(task_id) {
                let _ = task.queue();
            }
        }

        ready
    }

    /// Get job execution duration.
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.finished_at) {
            (Some(start), Some(end)) => Some(end - start),
            (Some(start), None) if self.state == State::Running => Some(Utc::now() - start),
            _ => None,
        }
    }

    /// Get progress as (completed, total) task count.
    pub fn progress(&self) -> (usize, usize) {
        let completed = self
            .tasks
            .values()
            .filter(|t| t.state == State::Success)
            .count();
        (completed, self.tasks.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::task::OnFailure;

    fn create_test_job() -> Job {
        let mut job = Job::new("test-pipeline");
        let job_id = job.id;

        let extract = Task::new("extract", job_id, "extract.sh");
        let transform = Task::new("transform", job_id, "transform.sh").depends_on("extract");
        let load = Task::new("load", job_id, "load.sh").depends_on("transform");

        job.add_task(extract).unwrap();
        job.add_task(transform).unwrap();
        job.add_task(load).unwrap();

        job
    }

    #[test]
    fn test_job_creation() {
        let job = Job::new("my-job");
        assert_eq!(job.name, "my-job");
        assert_eq!(job.state, State::Pending);
        assert!(job.tasks.is_empty());
    }

    #[test]
    fn test_add_tasks() {
        let mut job = Job::new("test");
        let job_id = job.id;

        let task = Task::new("task1", job_id, "echo hello");
        assert!(job.add_task(task.clone()).is_ok());
        assert!(job.get_task("task1").is_some());

        // Duplicate task should fail
        let duplicate = Task::new("task1", job_id, "echo world");
        assert!(job.add_task(duplicate).is_err());
    }

    #[test]
    fn test_validate_dag_valid() {
        let job = create_test_job();
        assert!(job.validate_dag().is_ok());
    }

    #[test]
    fn test_validate_dag_missing_dependency() {
        let mut job = Job::new("test");
        let job_id = job.id;

        let task = Task::new("task1", job_id, "cmd").depends_on("nonexistent");
        job.add_task(task).unwrap();

        let result = job.validate_dag();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PetitError::DependencyNotFound { .. }));
    }

    #[test]
    fn test_validate_dag_cycle() {
        let mut job = Job::new("test");
        let job_id = job.id;

        let task1 = Task::new("task1", job_id, "cmd1").depends_on("task2");
        let task2 = Task::new("task2", job_id, "cmd2").depends_on("task1");

        job.add_task(task1).unwrap();
        job.add_task(task2).unwrap();

        let result = job.validate_dag();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PetitError::CyclicDependency(_)));
    }

    #[test]
    fn test_topological_order() {
        let job = create_test_job();
        let order = job.topological_order().unwrap();

        // Extract should come before transform, transform before load
        let extract_pos = order.iter().position(|&t| t == "extract").unwrap();
        let transform_pos = order.iter().position(|&t| t == "transform").unwrap();
        let load_pos = order.iter().position(|&t| t == "load").unwrap();

        assert!(extract_pos < transform_pos);
        assert!(transform_pos < load_pos);
    }

    #[test]
    fn test_ready_tasks() {
        let mut job = create_test_job();

        // Queue all tasks first
        for task in job.tasks.values_mut() {
            let _ = task.queue();
        }

        // Only extract should be ready (no dependencies)
        let ready = job.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "extract");
    }

    #[test]
    fn test_update_state() {
        let mut job = create_test_job();

        // All pending -> job pending
        job.update_state();
        assert_eq!(job.state, State::Pending);

        // Mark one as running -> job running
        job.get_task_mut("extract").unwrap().queue().unwrap();
        job.get_task_mut("extract").unwrap().start().unwrap();
        job.update_state();
        assert_eq!(job.state, State::Running);
    }

    #[test]
    fn test_job_progress() {
        let mut job = create_test_job();
        assert_eq!(job.progress(), (0, 3));

        job.get_task_mut("extract").unwrap().queue().unwrap();
        job.get_task_mut("extract").unwrap().start().unwrap();
        job.get_task_mut("extract")
            .unwrap()
            .succeed(Default::default())
            .unwrap();

        assert_eq!(job.progress(), (1, 3));
    }

    #[test]
    fn test_continue_on_failure() {
        let mut job = Job::new("test");
        let job_id = job.id;

        let task1 = Task::new("task1", job_id, "cmd1").with_on_failure(OnFailure::Continue);
        let task2 = Task::new("task2", job_id, "cmd2").depends_on("task1");

        job.add_task(task1).unwrap();
        job.add_task(task2).unwrap();

        // Fail task1 with continue-on-failure
        job.get_task_mut("task1").unwrap().queue().unwrap();
        job.get_task_mut("task1").unwrap().start().unwrap();
        job.get_task_mut("task1").unwrap().fail("error").unwrap();

        job.update_state();
        // Job should not be failed because task1 has continue-on-failure
        assert_ne!(job.state, State::Failed);
    }

    #[test]
    fn test_job_metadata() {
        let metadata = JobMetadata {
            owner: Some("data-team".to_string()),
            priority: Some("high".to_string()),
            labels: [("env".to_string(), "prod".to_string())]
                .into_iter()
                .collect(),
        };

        let job = Job::new("test").with_metadata(metadata);
        assert_eq!(job.metadata.owner, Some("data-team".to_string()));
        assert_eq!(job.metadata.labels.get("env"), Some(&"prod".to_string()));
    }
}
