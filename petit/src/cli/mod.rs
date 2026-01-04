//! Command-line interface for Petit orchestrator.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

/// Petit - A lightweight task orchestrator
#[derive(Parser, Debug)]
#[command(name = "petit")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Job management commands
    Job {
        #[command(subcommand)]
        command: JobCommands,
    },

    /// Task management commands
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },

    /// Show system status
    Status,

    /// Show or validate configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Launch interactive TUI
    Tui,

    /// Run the orchestrator daemon
    Run {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum JobCommands {
    /// Submit a new job from YAML file
    Submit {
        /// Path to job YAML file
        file: PathBuf,

        /// Don't wait for job completion
        #[arg(long)]
        no_wait: bool,
    },

    /// List all jobs
    List {
        /// Filter by state (pending, running, success, failed, cancelled)
        #[arg(short, long)]
        state: Option<String>,

        /// Filter by name substring
        #[arg(short, long)]
        name: Option<String>,

        /// Maximum number of jobs to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Get detailed job status
    Status {
        /// Job ID
        job_id: String,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Cancel a running job
    Cancel {
        /// Job ID
        job_id: String,
    },

    /// Retry a failed job
    Retry {
        /// Job ID
        job_id: String,
    },

    /// Delete a job
    Delete {
        /// Job ID
        job_id: String,

        /// Force delete without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Watch job progress in real-time
    Watch {
        /// Job ID
        job_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum TaskCommands {
    /// List tasks in a job
    List {
        /// Job ID
        job_id: String,

        /// Filter by state
        #[arg(short, long)]
        state: Option<String>,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Get task status
    Status {
        /// Job ID
        job_id: String,

        /// Task ID
        task_id: String,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// View task logs
    Logs {
        /// Job ID
        job_id: String,

        /// Task ID
        task_id: String,

        /// Follow logs in real-time
        #[arg(short, long)]
        follow: bool,

        /// Show stderr only
        #[arg(long)]
        stderr: bool,
    },

    /// Retry a specific task
    Retry {
        /// Job ID
        job_id: String,

        /// Task ID
        task_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show current configuration
    Show {
        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },

    /// Validate configuration file
    Validate {
        /// Path to configuration file
        file: PathBuf,
    },

    /// Initialize a new configuration file
    Init {
        /// Output path
        #[arg(short, long, default_value = "petit.yaml")]
        output: PathBuf,

        /// Overwrite existing file
        #[arg(short, long)]
        force: bool,
    },
}

/// Parse a job ID from string (supports both UUID and short format).
pub fn parse_job_id(s: &str) -> Result<Uuid, String> {
    // Try full UUID first
    if let Ok(id) = Uuid::parse_str(s) {
        return Ok(id);
    }

    // Try short format (first 8 characters)
    if s.len() == 8 {
        // This would need lookup in storage - for now just error
        return Err("Short job IDs not yet supported".to_string());
    }

    Err(format!("Invalid job ID: {}", s))
}

/// Format state for display with colors.
pub fn format_state(state: &crate::types::State) -> String {
    use crate::types::State;
    match state {
        State::Pending => "⏳ Pending".to_string(),
        State::Queued => "📋 Queued".to_string(),
        State::Running => "🔄 Running".to_string(),
        State::Success => "✅ Success".to_string(),
        State::Failed => "❌ Failed".to_string(),
        State::Cancelled => "🚫 Cancelled".to_string(),
    }
}

/// Format duration for display.
pub fn format_duration(duration: chrono::Duration) -> String {
    let secs = duration.num_seconds();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_job_id_valid_uuid() {
        let uuid = Uuid::new_v4();
        let parsed = parse_job_id(&uuid.to_string()).unwrap();
        assert_eq!(parsed, uuid);
    }

    #[test]
    fn test_parse_job_id_invalid() {
        assert!(parse_job_id("not-a-uuid").is_err());
    }

    #[test]
    fn test_format_state() {
        use crate::types::State;
        assert!(format_state(&State::Success).contains("Success"));
        assert!(format_state(&State::Failed).contains("Failed"));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(chrono::Duration::seconds(45)), "45s");
        assert_eq!(format_duration(chrono::Duration::seconds(125)), "2m 5s");
        assert_eq!(format_duration(chrono::Duration::seconds(3725)), "1h 2m");
    }

    #[test]
    fn test_cli_parsing() {
        // Test that CLI parses correctly
        let cli = Cli::parse_from(["petit", "status"]);
        assert!(matches!(cli.command, Commands::Status));

        let cli = Cli::parse_from(["petit", "job", "list"]);
        assert!(matches!(cli.command, Commands::Job { .. }));

        let cli = Cli::parse_from(["petit", "-v", "tui"]);
        assert!(cli.verbose);
    }
}
