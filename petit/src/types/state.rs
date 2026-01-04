//! State machine types for tasks and jobs.

use serde::{Deserialize, Serialize};
use std::fmt;

/// State of a task or job in the orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Initial state, not yet scheduled.
    Pending,
    /// Scheduled and waiting for execution.
    Queued,
    /// Currently being executed.
    Running,
    /// Completed successfully.
    Success,
    /// Execution failed.
    Failed,
    /// Cancelled by user or system.
    Cancelled,
}

impl State {
    /// Check if the state is terminal (no more transitions possible except retry).
    pub fn is_terminal(&self) -> bool {
        matches!(self, State::Success | State::Failed | State::Cancelled)
    }

    /// Check if the state indicates success.
    pub fn is_success(&self) -> bool {
        matches!(self, State::Success)
    }

    /// Check if the state indicates failure.
    pub fn is_failure(&self) -> bool {
        matches!(self, State::Failed)
    }

    /// Check if the task is currently active (running or queued).
    pub fn is_active(&self) -> bool {
        matches!(self, State::Queued | State::Running)
    }

    /// Get valid transitions from this state.
    pub fn valid_transitions(&self) -> &'static [State] {
        match self {
            State::Pending => &[State::Queued, State::Cancelled],
            State::Queued => &[State::Running, State::Cancelled],
            State::Running => &[State::Success, State::Failed, State::Cancelled],
            State::Failed => &[State::Queued], // Retry
            State::Success | State::Cancelled => &[],
        }
    }

    /// Check if transition to the target state is valid.
    pub fn can_transition_to(&self, target: State) -> bool {
        self.valid_transitions().contains(&target)
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Pending => write!(f, "Pending"),
            State::Queued => write!(f, "Queued"),
            State::Running => write!(f, "Running"),
            State::Success => write!(f, "Success"),
            State::Failed => write!(f, "Failed"),
            State::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_is_terminal() {
        assert!(!State::Pending.is_terminal());
        assert!(!State::Queued.is_terminal());
        assert!(!State::Running.is_terminal());
        assert!(State::Success.is_terminal());
        assert!(State::Failed.is_terminal());
        assert!(State::Cancelled.is_terminal());
    }

    #[test]
    fn test_state_is_active() {
        assert!(!State::Pending.is_active());
        assert!(State::Queued.is_active());
        assert!(State::Running.is_active());
        assert!(!State::Success.is_active());
        assert!(!State::Failed.is_active());
    }

    #[test]
    fn test_valid_transitions_from_pending() {
        assert!(State::Pending.can_transition_to(State::Queued));
        assert!(State::Pending.can_transition_to(State::Cancelled));
        assert!(!State::Pending.can_transition_to(State::Running));
        assert!(!State::Pending.can_transition_to(State::Success));
    }

    #[test]
    fn test_valid_transitions_from_running() {
        assert!(State::Running.can_transition_to(State::Success));
        assert!(State::Running.can_transition_to(State::Failed));
        assert!(State::Running.can_transition_to(State::Cancelled));
        assert!(!State::Running.can_transition_to(State::Pending));
    }

    #[test]
    fn test_failed_can_retry() {
        assert!(State::Failed.can_transition_to(State::Queued));
    }

    #[test]
    fn test_success_no_transitions() {
        assert!(State::Success.valid_transitions().is_empty());
    }

    #[test]
    fn test_state_display() {
        assert_eq!(State::Pending.to_string(), "Pending");
        assert_eq!(State::Running.to_string(), "Running");
        assert_eq!(State::Success.to_string(), "Success");
    }

    #[test]
    fn test_state_serde() {
        let state = State::Running;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"running\"");

        let parsed: State = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, State::Running);
    }
}
