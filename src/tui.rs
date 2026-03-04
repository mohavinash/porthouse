use crate::config::PorthouseConfig;
use crate::conflict;
use crate::registry::Registry;
use crate::scanner::{self, PortEntry};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::*,
};
use std::io::stdout;
use std::time::{Duration, Instant};

struct App {
    config: PorthouseConfig,
    registry: Registry,
    entries: Vec<PortEntry>,
    conflicts: Vec<conflict::Conflict>,
    selected_port_index: usize,
    should_quit: bool,
    last_scan: Instant,
    confirm_kill: bool,
    status_msg: Option<(String, Instant)>,
}

impl App {
    fn new(config: PorthouseConfig, registry: Registry) -> Self {
        Self {
            config,
            registry,
            entries: Vec::new(),
            conflicts: Vec::new(),
            selected_port_index: 0,
            should_quit: false,
            last_scan: Instant::now() - Duration::from_secs(100), // force immediate scan
            confirm_kill: false,
            status_msg: None,
        }
    }

    fn tick(&mut self) {
        let interval = Duration::from_secs(self.config.daemon.scan_interval_secs);
        if self.last_scan.elapsed() >= interval {
            if let Ok(entries) = scanner::scan_ports() {
                self.conflicts = conflict::detect_conflicts(&entries);
                self.entries = entries;
                // Clamp selection
                if !self.entries.is_empty() && self.selected_port_index >= self.entries.len() {
                    self.selected_port_index = self.entries.len() - 1;
                }
            }
            self.last_scan = Instant::now();
        }
    }

    fn handle_key(&mut self, key: KeyCode) {
        // If awaiting kill confirmation
        if self.confirm_kill {
            self.confirm_kill = false;
            if key == KeyCode::Char('y') || key == KeyCode::Char('Y') {
                if let Some(entry) = self.entries.get(self.selected_port_index) {
                    let name = entry.process_name.clone();
                    let pid = entry.pid;
                    match crate::process::kill_process(pid) {
                        Ok(()) => {
                            self.status_msg = Some((
                                format!("Killed {} (PID {})", name, pid),
                                Instant::now(),
                            ));
                        }
                        Err(e) => {
                            self.status_msg = Some((
                                format!("Failed to kill PID {}: {}", pid, e),
                                Instant::now(),
                            ));
                        }
                    }
                    self.last_scan = Instant::now() - Duration::from_secs(100);
                }
            } else {
                self.status_msg = Some(("Kill cancelled.".to_string(), Instant::now()));
            }
            return;
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('r') => {
                self.last_scan = Instant::now() - Duration::from_secs(100); // force refresh
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_port_index > 0 {
                    self.selected_port_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_port_index < self.entries.len().saturating_sub(1) {
                    self.selected_port_index += 1;
                }
            }
            KeyCode::Char('K') => {
                if let Some(entry) = self.entries.get(self.selected_port_index) {
                    self.status_msg = Some((
                        format!("Kill {} (PID {}) on port {}? [y/N]", entry.process_name, entry.pid, entry.port),
                        Instant::now(),
                    ));
                    self.confirm_kill = true;
                }
            }
            _ => {}
        }
    }
}

pub fn run(config: PorthouseConfig, registry: Registry) -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = App::new(config, registry);

    loop {
        app.tick();

        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),  // title
                    Constraint::Min(10),   // ports table
                    Constraint::Length(6),  // conflicts
                    Constraint::Length(4),  // registry
                    Constraint::Length(1),  // footer
                ])
                .split(area);

            // Title
            let title = Paragraph::new(" Porthouse - Port Monitor & Manager")
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
            frame.render_widget(title, chunks[0]);

            // Ports table
            let conflict_ports: std::collections::HashSet<u16> =
                app.conflicts.iter().map(|c| c.port).collect();

            let rows: Vec<Row> = app
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let project = app
                        .registry
                        .find_by_port(e.port)
                        .map(|p| p.name.as_str())
                        .unwrap_or("-");
                    let status = if conflict_ports.contains(&e.port) {
                        "CONFLICT"
                    } else {
                        "OK"
                    };
                    let style = if conflict_ports.contains(&e.port) {
                        Style::default().fg(Color::Red)
                    } else if i == app.selected_port_index {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    Row::new(vec![
                        Cell::from(e.port.to_string()),
                        Cell::from(e.pid.to_string()),
                        Cell::from(e.process_name.clone()),
                        Cell::from(project.to_string()),
                        Cell::from(status.to_string()),
                    ])
                    .style(style)
                })
                .collect();

            let widths = [
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(20),
                Constraint::Length(18),
                Constraint::Length(12),
            ];

            let table = Table::new(rows, widths)
                .header(
                    Row::new(vec!["PORT", "PID", "PROCESS", "PROJECT", "STATUS"])
                        .style(
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Cyan),
                        ),
                )
                .block(Block::default().borders(Borders::ALL).title(" Active Ports "))
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

            let mut table_state = TableState::default();
            table_state.select(Some(app.selected_port_index));
            frame.render_stateful_widget(table, chunks[1], &mut table_state);

            // Conflicts panel
            let conflict_text: Vec<Line> = if app.conflicts.is_empty() {
                vec![Line::from("  No conflicts detected.")
                    .style(Style::default().fg(Color::Green))]
            } else {
                app.conflicts
                    .iter()
                    .flat_map(|c| {
                        let procs: Vec<String> = c
                            .entries
                            .iter()
                            .map(|e| format!("{} (PID {})", e.process_name, e.pid))
                            .collect();
                        let suggestion =
                            conflict::suggest_resolution(c.port, &app.entries);
                        vec![
                            Line::from(format!(
                                "  Port {}: {}",
                                c.port,
                                procs.join(" vs ")
                            ))
                            .style(Style::default().fg(Color::Red)),
                            Line::from(format!(
                                "  Suggestion: Move to port {} (free)",
                                suggestion
                            ))
                            .style(Style::default().fg(Color::Yellow)),
                        ]
                    })
                    .collect()
            };
            let conflicts_widget = Paragraph::new(conflict_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Conflicts ({}) ", app.conflicts.len())),
            );
            frame.render_widget(conflicts_widget, chunks[2]);

            // Registry panel
            let reg_text: Vec<Line> = if app.registry.projects.is_empty() {
                vec![Line::from(
                    "  No projects registered. Use 'porthouse register' to add one.",
                )
                .style(Style::default().fg(Color::DarkGray))]
            } else {
                app.registry
                    .projects
                    .iter()
                    .map(|p| {
                        let range_str = p
                            .range
                            .map(|(lo, hi)| format!("{}-{}", lo, hi))
                            .unwrap_or_else(|| {
                                if p.ports.is_empty() {
                                    "none".to_string()
                                } else {
                                    p.ports
                                        .iter()
                                        .map(|port| port.to_string())
                                        .collect::<Vec<_>>()
                                        .join(",")
                                }
                            });
                        Line::from(format!("  {}:  {}", p.name, range_str))
                    })
                    .collect()
            };
            let registry_widget = Paragraph::new(reg_text)
                .block(Block::default().borders(Borders::ALL).title(" Registry "));
            frame.render_widget(registry_widget, chunks[3]);

            // Footer — show status message if recent, otherwise keybindings
            let footer_text = if let Some((ref msg, at)) = app.status_msg {
                if at.elapsed() < Duration::from_secs(5) {
                    format!(" {}", msg)
                } else {
                    " [q]uit  [r]efresh  [j/k]navigate  [K]ill  [s]uggest".to_string()
                }
            } else {
                " [q]uit  [r]efresh  [j/k]navigate  [K]ill  [s]uggest".to_string()
            };
            let footer_style = if app.confirm_kill {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let footer = Paragraph::new(footer_text).style(footer_style);
            frame.render_widget(footer, chunks[4]);
        })?;

        // Handle input with timeout
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
