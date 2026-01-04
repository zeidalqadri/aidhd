# Petit

A lightweight, memory-efficient task orchestrator written in Rust.

## Features

- **DAG-based task scheduling**: Define dependencies between tasks
- **Multiple storage backends**: In-memory and SQLite support
- **YAML job definitions**: Easy-to-write job configuration
- **CLI interface**: Full job and task management
- **TUI dashboard**: Real-time monitoring with keyboard navigation
- **Retry policies**: Configurable retry with exponential backoff

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/petit`.

## Quick Start

### 1. Create a job definition

```yaml
# pipeline.yaml
name: my-pipeline
description: Example data pipeline

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
```

### 2. Submit the job

```bash
petit job submit pipeline.yaml
```

### 3. Monitor progress

```bash
# List all jobs
petit job list

# Watch a specific job
petit job watch <job-id>

# Launch the TUI dashboard
petit tui
```

## CLI Commands

### Job Management

```bash
petit job submit <file>          # Submit a job from YAML
petit job list                   # List all jobs
petit job status <job-id>        # Get job status
petit job cancel <job-id>        # Cancel a running job
petit job retry <job-id>         # Retry a failed job
petit job delete <job-id>        # Delete a job
petit job watch <job-id>         # Watch job progress
```

### Task Management

```bash
petit task list <job-id>              # List tasks in a job
petit task status <job-id> <task-id>  # Get task status
petit task logs <job-id> <task-id>    # View task logs
petit task retry <job-id> <task-id>   # Retry a specific task
```

### System Commands

```bash
petit status                     # System status
petit config show                # Show configuration
petit config init                # Create default config
petit tui                        # Launch TUI dashboard
petit run                        # Run orchestrator daemon
```

## Configuration

Create a `petit.yaml` file:

```yaml
storage:
  backend: sqlite  # or "memory"
  sqlite:
    path: ./petit.db

executor:
  workers: 4
  task_timeout: 3600s

scheduler:
  poll_interval: 100ms

logging:
  level: info
  format: pretty
```

## Job Definition Reference

```yaml
name: job-name                    # Required: Job name
description: Job description      # Optional

metadata:                         # Optional
  owner: team-name
  priority: high
  custom_key: custom_value

tasks:
  - id: task-id                   # Required: Unique task ID
    command: echo "hello"         # Required: Shell command
    depends_on:                   # Optional: Task dependencies
      - other-task
    timeout: 300s                 # Optional: Task timeout (default: 1h)
    retries: 3                    # Optional: Retry count (default: 3)
    on_failure: stop              # Optional: stop or continue (default: stop)
```

## TUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j/↓` | Move down |
| `k/↑` | Move up |
| `Enter` | Select item |
| `q/Esc` | Go back / Quit |
| `Tab` | Switch tab |
| `1-3` | Jump to tab |
| `r` | Refresh data |
| `?/F1` | Show help |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                         CLI/TUI                          │
├─────────────────────────────────────────────────────────┤
│                      Orchestrator                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │  Scheduler  │──│   Executor  │──│  Event Handler  │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
├─────────────────────────────────────────────────────────┤
│                    Storage Backend                       │
│  ┌─────────────────────┐  ┌──────────────────────────┐  │
│  │      InMemory       │  │         SQLite           │  │
│  └─────────────────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## License

MIT
