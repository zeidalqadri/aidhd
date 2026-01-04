//! Terminal User Interface for Petit orchestrator.

use std::io;
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState, Tabs},
    Frame, Terminal,
};

use crate::error::Result;
use crate::storage::{JobFilter, Storage};
use crate::types::{Job, State, Task};

/// TUI application state.
pub struct App<S: Storage> {
    storage: Arc<S>,
    /// Currently selected tab.
    current_tab: usize,
    /// Job list state.
    job_list_state: ListState,
    /// Jobs loaded from storage.
    jobs: Vec<Job>,
    /// Selected job for detail view.
    selected_job: Option<Job>,
    /// Task table state for selected job.
    task_list_state: TableState,
    /// Should quit the application.
    should_quit: bool,
    /// Last error message.
    last_error: Option<String>,
    /// Help text visible.
    show_help: bool,
}

impl<S: Storage + 'static> App<S> {
    /// Create a new TUI application.
    pub fn new(storage: Arc<S>) -> Self {
        let mut job_list_state = ListState::default();
        job_list_state.select(Some(0));

        Self {
            storage,
            current_tab: 0,
            job_list_state,
            jobs: Vec::new(),
            selected_job: None,
            task_list_state: TableState::default(),
            should_quit: false,
            last_error: None,
            show_help: false,
        }
    }

    /// Refresh data from storage.
    pub async fn refresh(&mut self) -> Result<()> {
        self.jobs = self.storage.list_jobs(JobFilter::new().with_limit(100)).await?;

        // Update selected job if it exists
        if let Some(ref selected) = self.selected_job {
            self.selected_job = self.storage.get_job(selected.id).await?;
        }

        Ok(())
    }

    /// Run the TUI application.
    pub async fn run(mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Initial data load
        self.refresh().await?;

        // Main loop
        let tick_rate = Duration::from_millis(250);
        loop {
            terminal.draw(|f| self.ui(f))?;

            if event::poll(tick_rate)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_input(key.code, key.modifiers).await;
                }
            }

            // Refresh data periodically
            if let Err(e) = self.refresh().await {
                self.last_error = Some(e.to_string());
            }

            if self.should_quit {
                break;
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    /// Handle keyboard input.
    async fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        // Clear error on any input
        self.last_error = None;

        if self.show_help {
            self.show_help = false;
            return;
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.selected_job.is_some() {
                    self.selected_job = None;
                    self.task_list_state.select(Some(0));
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                self.current_tab = (self.current_tab + 1) % 3;
            }
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.show_help = true;
            }
            KeyCode::Char('r') => {
                if let Err(e) = self.refresh().await {
                    self.last_error = Some(e.to_string());
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
            }
            KeyCode::Enter => {
                self.select_item();
            }
            KeyCode::Char('1') => self.current_tab = 0,
            KeyCode::Char('2') => self.current_tab = 1,
            KeyCode::Char('3') => self.current_tab = 2,
            _ => {}
        }
    }

    /// Select previous item in list.
    fn select_previous(&mut self) {
        if self.selected_job.is_some() {
            let i = match self.task_list_state.selected() {
                Some(i) => i.saturating_sub(1),
                None => 0,
            };
            self.task_list_state.select(Some(i));
        } else {
            let i = match self.job_list_state.selected() {
                Some(i) => i.saturating_sub(1),
                None => 0,
            };
            self.job_list_state.select(Some(i));
        }
    }

    /// Select next item in list.
    fn select_next(&mut self) {
        if let Some(ref job) = self.selected_job {
            let max = job.tasks.len().saturating_sub(1);
            let i = match self.task_list_state.selected() {
                Some(i) => (i + 1).min(max),
                None => 0,
            };
            self.task_list_state.select(Some(i));
        } else {
            let max = self.jobs.len().saturating_sub(1);
            let i = match self.job_list_state.selected() {
                Some(i) => (i + 1).min(max),
                None => 0,
            };
            self.job_list_state.select(Some(i));
        }
    }

    /// Select the current item (enter job detail or task detail).
    fn select_item(&mut self) {
        if self.selected_job.is_none() {
            if let Some(i) = self.job_list_state.selected() {
                if i < self.jobs.len() {
                    self.selected_job = Some(self.jobs[i].clone());
                    self.task_list_state.select(Some(0));
                }
            }
        }
    }

    /// Render the UI.
    fn ui(&mut self, f: &mut Frame) {
        let size = f.area();

        // Main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Status bar
            ])
            .split(size);

        // Tabs
        self.render_tabs(f, chunks[0]);

        // Content based on tab
        match self.current_tab {
            0 => self.render_jobs_tab(f, chunks[1]),
            1 => self.render_status_tab(f, chunks[1]),
            2 => self.render_logs_tab(f, chunks[1]),
            _ => {}
        }

        // Status bar
        self.render_status_bar(f, chunks[2]);

        // Help overlay
        if self.show_help {
            self.render_help(f, size);
        }
    }

    /// Render the tab bar.
    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        let titles = vec!["Jobs", "Status", "Logs"];
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Petit Orchestrator"))
            .select(self.current_tab)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(tabs, area);
    }

    /// Render the jobs tab.
    fn render_jobs_tab(&mut self, f: &mut Frame, area: Rect) {
        if let Some(ref job) = self.selected_job {
            self.render_job_detail(f, area, job.clone());
        } else {
            self.render_job_list(f, area);
        }
    }

    /// Render the job list.
    fn render_job_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .jobs
            .iter()
            .map(|job| {
                let state_icon = state_icon(job.state);
                let (completed, total) = job.progress();
                let progress = if total > 0 {
                    format!(" [{}/{}]", completed, total)
                } else {
                    String::new()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(state_icon, state_style(job.state)),
                    Span::raw(" "),
                    Span::styled(&job.name, Style::default().fg(Color::White)),
                    Span::styled(progress, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(
                        format!("[{}]", &job.id.to_string()[..8]),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Jobs"))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, area, &mut self.job_list_state);
    }

    /// Render job detail view.
    fn render_job_detail(&mut self, f: &mut Frame, area: Rect, job: Job) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)])
            .split(area);

        // Job info
        let info = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("Name: "),
                Span::styled(&job.name, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::raw("ID: "),
                Span::styled(job.id.to_string(), Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::raw("State: "),
                Span::styled(format!("{}", job.state), state_style(job.state)),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL).title("Job Details"));
        f.render_widget(info, chunks[0]);

        // Task list
        let tasks: Vec<&Task> = job.tasks.values().collect();
        let rows: Vec<Row> = tasks
            .iter()
            .map(|task| {
                Row::new(vec![
                    Cell::from(Span::styled(state_icon(task.state), state_style(task.state))),
                    Cell::from(&*task.id),
                    Cell::from(&*task.command),
                    Cell::from(format!("{}", task.state)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(3),
                Constraint::Percentage(20),
                Constraint::Percentage(50),
                Constraint::Percentage(20),
            ],
        )
        .header(
            Row::new(vec!["", "ID", "Command", "State"])
                .style(Style::default().fg(Color::Yellow)),
        )
        .block(Block::default().borders(Borders::ALL).title("Tasks (q to go back)"))
        .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(table, chunks[1], &mut self.task_list_state);
    }

    /// Render the status tab.
    fn render_status_tab(&self, f: &mut Frame, area: Rect) {
        let mut counts = std::collections::HashMap::new();
        for job in &self.jobs {
            *counts.entry(job.state).or_insert(0) += 1;
        }

        let text = vec![
            Line::from(vec![
                Span::styled("Total Jobs: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", self.jobs.len()),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(state_icon(State::Pending), state_style(State::Pending)),
                Span::raw(format!(" Pending: {}", counts.get(&State::Pending).unwrap_or(&0))),
            ]),
            Line::from(vec![
                Span::styled(state_icon(State::Queued), state_style(State::Queued)),
                Span::raw(format!(" Queued: {}", counts.get(&State::Queued).unwrap_or(&0))),
            ]),
            Line::from(vec![
                Span::styled(state_icon(State::Running), state_style(State::Running)),
                Span::raw(format!(" Running: {}", counts.get(&State::Running).unwrap_or(&0))),
            ]),
            Line::from(vec![
                Span::styled(state_icon(State::Success), state_style(State::Success)),
                Span::raw(format!(" Success: {}", counts.get(&State::Success).unwrap_or(&0))),
            ]),
            Line::from(vec![
                Span::styled(state_icon(State::Failed), state_style(State::Failed)),
                Span::raw(format!(" Failed: {}", counts.get(&State::Failed).unwrap_or(&0))),
            ]),
            Line::from(vec![
                Span::styled(state_icon(State::Cancelled), state_style(State::Cancelled)),
                Span::raw(format!(
                    " Cancelled: {}",
                    counts.get(&State::Cancelled).unwrap_or(&0)
                )),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("System Status"));
        f.render_widget(paragraph, area);
    }

    /// Render the logs tab.
    fn render_logs_tab(&self, f: &mut Frame, area: Rect) {
        let text = if let Some(ref job) = self.selected_job {
            // Show logs for selected job
            let mut lines = vec![Line::from(vec![
                Span::styled("Job: ", Style::default().fg(Color::Gray)),
                Span::styled(&job.name, Style::default().fg(Color::Cyan)),
            ])];

            for task in job.tasks.values() {
                if let Some(ref output) = task.output {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("--- {} ---", task.id),
                            Style::default().fg(Color::Yellow),
                        ),
                    ]));
                    for line in output.stdout.lines().take(10) {
                        lines.push(Line::from(line.to_string()));
                    }
                    if !output.stderr.is_empty() {
                        lines.push(Line::from(Span::styled(
                            "stderr:",
                            Style::default().fg(Color::Red),
                        )));
                        for line in output.stderr.lines().take(5) {
                            lines.push(Line::from(Span::styled(
                                line.to_string(),
                                Style::default().fg(Color::Red),
                            )));
                        }
                    }
                }
            }
            lines
        } else {
            vec![Line::from("Select a job to view logs")]
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Logs"));
        f.render_widget(paragraph, area);
    }

    /// Render the status bar.
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status = if let Some(ref err) = self.last_error {
            Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red)),
                Span::raw(err),
            ])
        } else {
            Line::from(vec![
                Span::styled("?", Style::default().fg(Color::Yellow)),
                Span::raw(" Help  "),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::raw(" Switch tab  "),
                Span::styled("j/k", Style::default().fg(Color::Yellow)),
                Span::raw(" Navigate  "),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" Select  "),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::raw(" Quit"),
            ])
        };

        let paragraph = Paragraph::new(status)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(paragraph, area);
    }

    /// Render help overlay.
    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_area = centered_rect(60, 60, area);

        let help_text = vec![
            Line::from(Span::styled(
                "Keyboard Shortcuts",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("j/↓     Move down"),
            Line::from("k/↑     Move up"),
            Line::from("Enter   Select item"),
            Line::from("q/Esc   Go back / Quit"),
            Line::from("Tab     Switch tab"),
            Line::from("1-3     Jump to tab"),
            Line::from("r       Refresh data"),
            Line::from("?/F1    Show this help"),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to close",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help")
                    .style(Style::default().bg(Color::Black)),
            )
            .style(Style::default().bg(Color::Black));

        f.render_widget(ratatui::widgets::Clear, help_area);
        f.render_widget(paragraph, help_area);
    }
}

/// Get icon for state.
fn state_icon(state: State) -> &'static str {
    match state {
        State::Pending => "○",
        State::Queued => "◐",
        State::Running => "●",
        State::Success => "✓",
        State::Failed => "✗",
        State::Cancelled => "⊘",
    }
}

/// Get style for state.
fn state_style(state: State) -> Style {
    match state {
        State::Pending => Style::default().fg(Color::Gray),
        State::Queued => Style::default().fg(Color::Blue),
        State::Running => Style::default().fg(Color::Yellow),
        State::Success => Style::default().fg(Color::Green),
        State::Failed => Style::default().fg(Color::Red),
        State::Cancelled => Style::default().fg(Color::DarkGray),
    }
}

/// Helper to create a centered rect.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
