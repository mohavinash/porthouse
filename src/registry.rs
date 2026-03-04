use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    #[serde(default, rename = "project")]
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<(u16, u16)>,
}

impl Registry {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let registry: Self = toml::from_str(&content)?;
        Ok(registry)
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

    pub fn register(
        &mut self,
        name: &str,
        path: Option<&str>,
        ports: Vec<u16>,
        range: Option<(u16, u16)>,
    ) {
        self.projects.push(Project {
            name: name.to_string(),
            path: path.map(|p| p.to_string()),
            ports,
            range,
        });
    }

    pub fn find_by_name(&self, name: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.name == name)
    }

    pub fn find_by_port(&self, port: u16) -> Option<&Project> {
        self.projects.iter().find(|p| {
            p.ports.contains(&port)
                || p.range
                    .map(|(lo, hi)| port >= lo && port <= hi)
                    .unwrap_or(false)
        })
    }

    pub fn is_port_reserved(&self, port: u16) -> bool {
        self.find_by_port(port).is_some()
    }
}
