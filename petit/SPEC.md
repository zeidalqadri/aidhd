# Petit - A Lightweight Rust Orchestrator

## Overview

Petit is a memory-efficient, lightweight task orchestrator written in Rust. It provides a simple yet powerful way to define, schedule, and execute jobs composed of dependent tasks. The system supports both in-memory and SQLite storage backends, making it suitable for both ephemeral and persistent workflows.

## Design Goals

1. **Memory Efficiency**: Minimal memory footprint, suitable for resource-constrained environments
2. **Lightweight**: Small binary size, few dependencies
3. **Type Safety**: Leverage Rust's type system for correctness
4. **Flexibility**: Support multiple storage backends and execution strategies
5. **Observability**: Real-time monitoring via TUI and comprehensive logging
6. **Testability**: High test coverage with TDD approach

## Core Concepts

### Task
A Task is the smallest unit of work in Petit. Each task:
- Has a unique identifier within its parent job
- Contains executable logic (shell command or custom handler)
- May depend on other tasks within the same job
- Has configurable retry policies
- Tracks its execution state and output

### Job
A Job is a collection of tasks with defined dependencies forming a DAG (Directed Acyclic Graph):
- Has a unique identifier
- Contains one or more tasks
- Defines task dependencies
- Has overall state derived from task states
- Supports metadata and labels for organization

### State Machine
Both Jobs and Tasks follow a state machine:

```
         ┌─────────┐
         │ Pending │
         └────┬────┘
              │ schedule
              ▼
         ┌─────────┐
    ┌────│ Queued  │────┐
    │    └────┬────┘    │
    │         │ start   │ cancel
    │         ▼         │
    │    ┌─────────┐    │
    │    │ Running │────┤
    │    └────┬────┘    │
    │         │         │
    │    ┌────┴────┐    │
    │    │         │    │
    ▼    ▼         ▼    ▼
┌──────────┐  ┌────────┐  ┌───────────┐
│ Success  │  │ Failed │  │ Cancelled │
└──────────┘  └────┬───┘  └───────────┘
                   │ retry
                   ▼
              ┌─────────┐
              │ Queued  │
              └─────────┘
```

### Storage Backend
Petit supports pluggable storage backends:
- **InMemory**: Fast, ephemeral storage for testing and short-lived workflows
- **SQLite**: Persistent storage for durable workflows with recovery support

### Executor
The executor is responsible for:
- Pulling ready tasks from the scheduler
- Managing a pool of workers
- Executing tasks with proper isolation
- Handling task completion, failure, and retries

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

## YAML Job Definition

Jobs are defined using YAML files:

```yaml
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
    on_failure: continue  # Run even if load fails
```

## CLI Interface

```bash
# Job management
petit job submit pipeline.yaml          # Submit a job from YAML
petit job list                          # List all jobs
petit job status <job-id>               # Get job status
petit job cancel <job-id>               # Cancel a running job
petit job retry <job-id>                # Retry a failed job

# Task management
petit task list <job-id>                # List tasks in a job
petit task status <job-id> <task-id>    # Get task status
petit task logs <job-id> <task-id>      # View task logs
petit task retry <job-id> <task-id>     # Retry a specific task

# System management
petit status                            # System status
petit config show                       # Show configuration

# TUI
petit tui                               # Launch interactive TUI
```

## TUI Features

The TUI provides:
- Real-time job and task status dashboard
- Job submission and management
- Task log viewing
- System metrics and health indicators
- Keyboard navigation and vim-like bindings

## Configuration

```yaml
# petit.yaml
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
  format: json
```

## Implementation Phases

### Phase 1: Core Types
- Define Task, Job, and State types
- Implement state machine transitions
- Create error types
- Unit tests for all types

### Phase 2: Storage Traits
- Define Storage trait
- Implement InMemory backend
- Unit tests for storage operations

### Phase 3: SQLite Backend
- Implement SQLite storage
- Schema migrations
- Integration tests

### Phase 4: Executor
- Task execution logic
- Worker pool management
- Output capture
- Unit tests

### Phase 5: Scheduler
- DAG dependency resolution
- Task queue management
- Priority scheduling
- Unit tests

### Phase 6: Orchestrator
- Tie together scheduler, executor, storage
- Event handling
- Integration tests

### Phase 7: YAML Parser
- Job definition parsing
- Validation
- Unit tests

### Phase 8: CLI
- Command structure using clap
- All management commands
- Integration tests

### Phase 9: TUI
- Dashboard layout using ratatui
- Real-time updates
- Keyboard navigation
- Visual tests

### Phase 10: Polish
- Error handling refinement
- Retry logic
- Documentation
- End-to-end tests

## Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
clap = { version = "4", features = ["derive"] }
ratatui = "0.28"
crossterm = "0.28"
rusqlite = { version = "0.32", features = ["bundled"] }
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

## Testing Strategy

1. **Unit Tests**: Every module has comprehensive unit tests
2. **Integration Tests**: Storage and orchestrator integration
3. **End-to-End Tests**: CLI and full workflow tests
4. **Property Tests**: State machine invariants
5. **Benchmark Tests**: Performance regression testing

## Success Criteria

- [ ] All core types implemented with >90% test coverage
- [ ] Both storage backends fully functional
- [ ] DAG resolution handles complex dependencies
- [ ] CLI provides full job/task management
- [ ] TUI shows real-time status updates
- [ ] YAML parser handles all documented features
- [ ] Documentation complete with examples
- [ ] Memory usage < 10MB for 1000 queued tasks
- [ ] Binary size < 5MB
