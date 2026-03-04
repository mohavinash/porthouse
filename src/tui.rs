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
                    let _ = crate::process::kill_process(entry.pid);
                    self.last_scan = Instant::now() - Duration::from_secs(100); // force refresh
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

            // Footer
            let footer =
                Paragraph::new(" [q]uit  [r]efresh  [j/k]navigate  [K]ill  [s]uggest")
                    .style(Style::default().fg(Color::DarkGray));
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
