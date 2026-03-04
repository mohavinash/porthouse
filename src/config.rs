use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PorthouseConfig {
    pub daemon: DaemonConfig,
    pub alerts: AlertConfig,
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaemonConfig {
    pub scan_interval_secs: u64,
    pub port_range: (u16, u16),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertConfig {
    pub macos_notifications: bool,
    pub terminal_bell: bool,
    pub log_file: String,
    pub webhook_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefaultsConfig {
    pub ports_per_project: u16,
}

impl Default for PorthouseConfig {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig {
                scan_interval_secs: 3,
                port_range: (1024, 49151),
            },
            alerts: AlertConfig {
                macos_notifications: true,
                terminal_bell: true,
                log_file: "~/.porthouse/alerts.log".to_string(),
                webhook_url: String::new(),
            },
            defaults: DefaultsConfig {
                ports_per_project: 10,
            },
        }
    }
}

impl PorthouseConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_or_default(path: &Path) -> Self {
        Self::load(path).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}
