//! Petit CLI entry point.

use std::sync::Arc;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use petit::{
    cli::{Cli, Commands, ConfigCommands, JobCommands, TaskCommands},
    error::Result,
    parser::{ConfigDefinition, JobDefinition},
    scheduler::{Orchestrator, SchedulerConfig},
    storage::{InMemoryStorage, JobFilter, SqliteStorage, Storage},
    tui::App,
    types::State,
    PetitError,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Load config if provided
    let config = if let Some(ref config_path) = cli.config {
        ConfigDefinition::from_file(config_path)?
    } else {
        ConfigDefinition::default()
    };

    // Dispatch to appropriate handler based on storage backend
    match config.storage.backend.as_str() {
        "sqlite" => {
            let path = config
                .storage
                .sqlite
                .as_ref()
                .map(|s| s.path.clone())
                .unwrap_or_else(|| "petit.db".to_string());
            let storage = Arc::new(SqliteStorage::new(path)?);
            run_command(cli.command, storage, &config).await
        }
        _ => {
            let storage = Arc::new(InMemoryStorage::new());
            run_command(cli.command, storage, &config).await
        }
    }
}

async fn run_command<S: Storage + 'static>(
    command: Commands,
    storage: Arc<S>,
    config: &ConfigDefinition,
) -> Result<()> {
    match command {
        Commands::Job { command } => run_job_command(command, storage, config).await,
        Commands::Task { command } => run_task_command(command, storage).await,
        Commands::Status => run_status(storage, config).await,
        Commands::Config { command } => run_config_command(command),
        Commands::Tui => run_tui(storage).await,
        Commands::Run { foreground: _ } => run_daemon(storage).await,
    }
}

async fn run_job_command<S: Storage + 'static>(
    command: JobCommands,
    storage: Arc<S>,
    _config: &ConfigDefinition,
) -> Result<()> {
    let orchestrator = Orchestrator::new(storage.clone(), SchedulerConfig::default());

    match command {
        JobCommands::Submit { file, no_wait: _ } => {
            let def = JobDefinition::from_file(&file)?;
            let job = def.into_job()?;
            let job_id = orchestrator.submit(job).await?;
            println!("Job submitted: {}", job_id);
        }

        JobCommands::List {
            state,
            name,
            limit,
            format,
        } => {
            let mut filter = JobFilter::new().with_limit(limit);

            if let Some(state_str) = state {
                let state = parse_state(&state_str)?;
                filter = filter.with_state(state);
            }

            if let Some(name) = name {
                filter = filter.with_name(name);
            }

            let jobs = storage.list_jobs(filter).await?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&jobs)?);
                }
                "yaml" => {
                    println!("{}", serde_yaml::to_string(&jobs)?);
                }
                _ => {
                    println!("{:<36} {:<20} {:<12} {:<10}", "ID", "NAME", "STATE", "PROGRESS");
                    println!("{}", "-".repeat(80));
                    for job in jobs {
                        let (completed, total) = job.progress();
                        println!(
                            "{:<36} {:<20} {:<12} {}/{}",
                            job.id,
                            truncate(&job.name, 20),
                            job.state,
                            completed,
                            total
                        );
                    }
                }
            }
        }

        JobCommands::Status { job_id, format } => {
            let id = petit::cli::parse_job_id(&job_id)?;
            let job = storage
                .get_job(id)
                .await?
                .ok_or_else(|| PetitError::JobNotFound(id))?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&job)?);
                }
                "yaml" => {
                    println!("{}", serde_yaml::to_string(&job)?);
                }
                _ => {
                    println!("Job: {}", job.name);
                    println!("ID: {}", job.id);
                    println!("State: {}", petit::cli::format_state(&job.state));
                    println!("Description: {}", job.description.as_deref().unwrap_or("-"));
                    println!("Created: {}", job.created_at);
                    if let Some(started) = job.started_at {
                        println!("Started: {}", started);
                    }
                    if let Some(finished) = job.finished_at {
                        println!("Finished: {}", finished);
                    }
                    if let Some(duration) = job.duration() {
                        println!("Duration: {}", petit::cli::format_duration(duration));
                    }
                    println!("\nTasks:");
                    println!("{:<20} {:<12} {:<40}", "ID", "STATE", "COMMAND");
                    println!("{}", "-".repeat(75));
                    for task in job.tasks.values() {
                        println!(
                            "{:<20} {:<12} {:<40}",
                            task.id,
                            task.state,
                            truncate(&task.command, 40)
                        );
                    }
                }
            }
        }

        JobCommands::Cancel { job_id } => {
            let id = petit::cli::parse_job_id(&job_id)?;
            orchestrator.cancel_job(id).await?;
            println!("Job {} cancelled", id);
        }

        JobCommands::Retry { job_id } => {
            let id = petit::cli::parse_job_id(&job_id)?;
            orchestrator.retry_job(id).await?;
            println!("Job {} queued for retry", id);
        }

        JobCommands::Delete { job_id, force } => {
            let id = petit::cli::parse_job_id(&job_id)?;

            if !force {
                println!("Are you sure you want to delete job {}? (y/N)", id);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim().to_lowercase() != "y" {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            storage.delete_job(id).await?;
            println!("Job {} deleted", id);
        }

        JobCommands::Watch { job_id } => {
            let id = petit::cli::parse_job_id(&job_id)?;
            println!("Watching job {} (Ctrl+C to stop)...", id);

            loop {
                let job = storage.get_job(id).await?;
                if let Some(job) = job {
                    print!("\x1B[2J\x1B[H"); // Clear screen
                    println!("Job: {} ({})", job.name, job.state);
                    let (completed, total) = job.progress();
                    println!("Progress: {}/{}", completed, total);
                    println!();
                    for task in job.tasks.values() {
                        println!(
                            "  {} {} - {}",
                            petit::cli::format_state(&task.state),
                            task.id,
                            task.command
                        );
                    }

                    if job.state.is_terminal() {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}

async fn run_task_command<S: Storage + 'static>(
    command: TaskCommands,
    storage: Arc<S>,
) -> Result<()> {
    match command {
        TaskCommands::List {
            job_id,
            state,
            format,
        } => {
            let id = petit::cli::parse_job_id(&job_id)?;
            let job = storage
                .get_job(id)
                .await?
                .ok_or_else(|| PetitError::JobNotFound(id))?;

            let tasks: Vec<_> = if let Some(state_str) = state {
                let state = parse_state(&state_str)?;
                job.tasks.values().filter(|t| t.state == state).collect()
            } else {
                job.tasks.values().collect()
            };

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&tasks)?);
                }
                "yaml" => {
                    println!("{}", serde_yaml::to_string(&tasks)?);
                }
                _ => {
                    println!("{:<20} {:<12} {:<40} {:<15}", "ID", "STATE", "COMMAND", "DEPS");
                    println!("{}", "-".repeat(90));
                    for task in tasks {
                        println!(
                            "{:<20} {:<12} {:<40} {:<15}",
                            task.id,
                            task.state,
                            truncate(&task.command, 40),
                            task.depends_on.join(", ")
                        );
                    }
                }
            }
        }

        TaskCommands::Status {
            job_id,
            task_id,
            format,
        } => {
            let id = petit::cli::parse_job_id(&job_id)?;
            let task = storage
                .get_task(id, &task_id)
                .await?
                .ok_or_else(|| PetitError::TaskNotFound {
                    job_id: id,
                    task_id: task_id.clone(),
                })?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&task)?);
                }
                "yaml" => {
                    println!("{}", serde_yaml::to_string(&task)?);
                }
                _ => {
                    println!("Task: {}", task.id);
                    println!("State: {}", petit::cli::format_state(&task.state));
                    println!("Command: {}", task.command);
                    println!("Dependencies: {}", task.depends_on.join(", "));
                    println!("Timeout: {}s", task.timeout.as_secs());
                    println!(
                        "Retry: {}/{}",
                        task.retry_policy.current_attempt, task.retry_policy.max_attempts
                    );
                    if let Some(ref error) = task.error {
                        println!("Error: {}", error);
                    }
                }
            }
        }

        TaskCommands::Logs {
            job_id,
            task_id,
            follow: _,
            stderr,
        } => {
            let id = petit::cli::parse_job_id(&job_id)?;
            let task = storage
                .get_task(id, &task_id)
                .await?
                .ok_or_else(|| PetitError::TaskNotFound {
                    job_id: id,
                    task_id: task_id.clone(),
                })?;

            if let Some(output) = task.output {
                if stderr {
                    println!("{}", output.stderr);
                } else {
                    println!("{}", output.stdout);
                    if !output.stderr.is_empty() {
                        eprintln!("\n--- stderr ---\n{}", output.stderr);
                    }
                }
            } else {
                println!("No output available");
            }
        }

        TaskCommands::Retry { job_id, task_id } => {
            let id = petit::cli::parse_job_id(&job_id)?;
            let mut task = storage
                .get_task(id, &task_id)
                .await?
                .ok_or_else(|| PetitError::TaskNotFound {
                    job_id: id,
                    task_id: task_id.clone(),
                })?;

            task.retry()?;
            storage.update_task(id, &task).await?;
            println!("Task {} queued for retry", task_id);
        }
    }

    Ok(())
}

async fn run_status<S: Storage + 'static>(
    storage: Arc<S>,
    config: &ConfigDefinition,
) -> Result<()> {
    let counts = storage.count_jobs_by_state().await?;

    println!("Petit Orchestrator Status");
    println!("=========================");
    println!();
    println!("Storage: {}", config.storage.backend);
    println!("Workers: {}", config.executor.workers);
    println!();
    println!("Jobs by State:");

    let states = [
        State::Pending,
        State::Queued,
        State::Running,
        State::Success,
        State::Failed,
        State::Cancelled,
    ];

    for state in states {
        let count = counts.get(&state).unwrap_or(&0);
        println!("  {}: {}", petit::cli::format_state(&state), count);
    }

    Ok(())
}

fn run_config_command(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Show { format } => {
            let config = ConfigDefinition::default();
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&config)?),
                _ => println!("{}", serde_yaml::to_string(&config)?),
            }
        }

        ConfigCommands::Validate { file } => match ConfigDefinition::from_file(&file) {
            Ok(_) => println!("Configuration is valid"),
            Err(e) => {
                eprintln!("Configuration error: {}", e);
                std::process::exit(1);
            }
        },

        ConfigCommands::Init { output, force } => {
            if output.exists() && !force {
                eprintln!(
                    "File {} already exists. Use --force to overwrite.",
                    output.display()
                );
                std::process::exit(1);
            }

            let config = ConfigDefinition::default();
            let yaml = serde_yaml::to_string(&config)?;
            std::fs::write(&output, yaml)?;
            println!("Configuration written to {}", output.display());
        }
    }

    Ok(())
}

async fn run_tui<S: Storage + 'static>(storage: Arc<S>) -> Result<()> {
    let app = App::new(storage);
    app.run().await
}

async fn run_daemon<S: Storage + 'static>(storage: Arc<S>) -> Result<()> {
    let orchestrator = Orchestrator::new(storage, SchedulerConfig::default());

    println!("Petit daemon starting...");
    println!("Press Ctrl+C to stop");

    // Handle Ctrl+C
    let scheduler = orchestrator.scheduler().clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\nShutting down...");
        scheduler.shutdown();
    });

    orchestrator.run().await
}

fn parse_state(s: &str) -> Result<State> {
    match s.to_lowercase().as_str() {
        "pending" => Ok(State::Pending),
        "queued" => Ok(State::Queued),
        "running" => Ok(State::Running),
        "success" => Ok(State::Success),
        "failed" => Ok(State::Failed),
        "cancelled" => Ok(State::Cancelled),
        _ => Err(PetitError::validation(format!("Invalid state: {}", s))),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max - 3])
    } else {
        s.to_string()
    }
}
