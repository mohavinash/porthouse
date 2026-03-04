use crate::config::AlertConfig;
use anyhow::Result;
use std::io::Write;

/// Strip control characters from process names to prevent log injection.
fn sanitize(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

#[derive(Debug, Clone)]
pub enum AlertEvent {
    Conflict {
        port: u16,
        processes: Vec<(String, u32)>,
    },
    NewListener {
        port: u16,
        process: String,
        pid: u32,
    },
    PortFreed {
        port: u16,
    },
}

impl AlertEvent {
    pub fn to_message(&self) -> String {
        match self {
            AlertEvent::Conflict { port, processes } => {
                let names: Vec<String> = processes.iter().map(|(name, _)| sanitize(name)).collect();
                format!(
                    "{} are both fighting over port {}. Run `porthouse kill {}` or reassign one.",
                    names.join(" and "),
                    port,
                    port
                )
            }
            AlertEvent::NewListener { port, process, .. } => {
                format!("{} just started on port {}.", sanitize(process), port)
            }
            AlertEvent::PortFreed { port } => {
                format!("Port {} is free again.", port)
            }
        }
    }

    pub fn title(&self) -> &str {
        match self {
            AlertEvent::Conflict { .. } => "Port Conflict",
            AlertEvent::NewListener { .. } => "New Service",
            AlertEvent::PortFreed { .. } => "Port Freed",
        }
    }
}

pub struct AlertManager {
    config: AlertConfig,
}

impl AlertManager {
    pub fn new(config: AlertConfig) -> Self {
        Self { config }
    }

    pub fn fire(&self, event: &AlertEvent) {
        if self.config.macos_notifications {
            let _ = self.send_macos_notification(event);
        }
        if self.config.terminal_bell {
            self.send_terminal_bell(event);
        }
        if !self.config.log_file.is_empty() {
            let _ = self.log_to_file(event);
        }
        if !self.config.webhook_url.is_empty() {
            let _ = self.send_webhook(event);
        }
    }

    fn send_macos_notification(&self, event: &AlertEvent) -> Result<()> {
        notify_rust::Notification::new()
            .summary(event.title())
            .body(&event.to_message())
            .show()?;
        Ok(())
    }

    fn send_terminal_bell(&self, event: &AlertEvent) {
        eprint!("\x07");
        eprintln!("[porthouse] {}", event.to_message());
    }

    pub fn log_to_file(&self, event: &AlertEvent) -> Result<()> {
        let expanded = shellexpand::tilde(&self.config.log_file).to_string();
        let path = std::path::Path::new(&expanded);

        // Security: validate log path is within ~/.porthouse/
        if let Some(home) = dirs::home_dir() {
            let porthouse_dir = home.join(".porthouse");
            let canonical_parent = if let Some(parent) = path.parent() {
                // Create parent dirs first so canonicalize works
                let _ = std::fs::create_dir_all(parent);
                parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf())
            } else {
                return Err(anyhow::anyhow!("Invalid log file path"));
            };
            let safe_dir = porthouse_dir.canonicalize().unwrap_or(porthouse_dir);
            if !canonical_parent.starts_with(&safe_dir) {
                return Err(anyhow::anyhow!(
                    "Log file path must be within ~/.porthouse/ (got: {})",
                    path.display()
                ));
            }
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        writeln!(file, "[{}] {}", timestamp, event.to_message())?;
        Ok(())
    }

    fn send_webhook(&self, event: &AlertEvent) -> Result<()> {
        let url = &self.config.webhook_url;
        // Security: only allow http/https schemes to prevent SSRF via file:// etc.
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(anyhow::anyhow!(
                "Webhook URL must use http:// or https:// scheme (got: {})",
                url
            ));
        }
        let body = serde_json::json!({ "text": event.to_message() });
        let body_str = serde_json::to_string(&body)?;
        // Use -- to prevent URL being interpreted as a flag
        std::process::Command::new("curl")
            .args(["-s", "-X", "POST", "-H", "Content-Type: application/json", "-d", &body_str, "--"])
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()?; // .output() waits for child (prevents zombies)
        Ok(())
    }
}
