use crate::registry::Registry;
use crate::scanner::PortEntry;
use std::collections::HashMap;

/// A group of port entries that share the same port number, indicating a conflict.
#[derive(Debug, Clone)]
pub struct Conflict {
    /// The port number where the conflict exists.
    pub port: u16,
    /// The processes competing for this port.
    pub entries: Vec<PortEntry>,
}

/// Detect port conflicts: ports where multiple distinct processes are listening.
///
/// A conflict is when different PIDs share the same port. The same process
/// binding to multiple addresses (e.g., `::` and `0.0.0.0` for dual-stack)
/// is normal and NOT a conflict.
///
/// Returns a list of [`Conflict`] structs sorted by port number.
pub fn detect_conflicts(entries: &[PortEntry]) -> Vec<Conflict> {
    let mut by_port: HashMap<u16, Vec<PortEntry>> = HashMap::new();
    for entry in entries {
        by_port.entry(entry.port).or_default().push(entry.clone());
    }

    let mut conflicts: Vec<Conflict> = by_port
        .into_iter()
        .filter(|(_, entries)| {
            // Only a conflict if there are multiple distinct PIDs on the same port
            let mut pids: Vec<u32> = entries.iter().map(|e| e.pid).collect();
            pids.sort();
            pids.dedup();
            pids.len() > 1
        })
        .map(|(port, entries)| Conflict { port, entries })
        .collect();

    conflicts.sort_by_key(|c| c.port);
    conflicts
}

/// A violation where a port reserved in the registry is used by an unexpected process.
#[derive(Debug, Clone)]
pub struct RegistryViolation {
    /// The port that is registered.
    pub port: u16,
    /// The project name expected by the registry.
    pub expected_project: String,
    /// The actual process name found on the port.
    pub actual_process: String,
    /// The PID of the actual process.
    pub actual_pid: u32,
}

/// Compare running port entries against a [`Registry`] and find violations.
///
/// A violation occurs when a port is reserved for a project in the registry but
/// the process occupying it does not match the project name (case-insensitive
/// substring match in either direction).
pub fn detect_registry_violations(
    entries: &[PortEntry],
    registry: &Registry,
) -> Vec<RegistryViolation> {
    let mut violations = Vec::new();
    for entry in entries {
        if let Some(project) = registry.find_by_port(entry.port) {
            let project_lower = project.name.to_lowercase();
            let process_lower = entry.process_name.to_lowercase();
            if !process_lower.contains(&project_lower)
                && !project_lower.contains(&process_lower)
            {
                violations.push(RegistryViolation {
                    port: entry.port,
                    expected_project: project.name.clone(),
                    actual_process: entry.process_name.clone(),
                    actual_pid: entry.pid,
                });
            }
        }
    }
    violations
}

/// Suggest an alternative port for a conflicted port.
///
/// Starts searching from `conflicted_port + 1` upwards and returns the first
/// port that is not already in use by any entry. Stops at 65535.
pub fn suggest_resolution(conflicted_port: u16, entries: &[PortEntry]) -> u16 {
    let used: std::collections::HashSet<u16> = entries.iter().map(|e| e.port).collect();
    let mut candidate = conflicted_port + 1;
    while used.contains(&candidate) && candidate < 65535 {
        candidate += 1;
    }
    candidate
}
