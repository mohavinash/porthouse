use anyhow::{Context, Result};

/// A single entry representing a process listening on a port.
#[derive(Debug, Clone)]
pub struct PortEntry {
    /// The port number being listened on.
    pub port: u16,
    /// The process ID of the listener.
    pub pid: u32,
    /// The name of the listening process.
    pub process_name: String,
    /// The protocol (TCP or UDP).
    pub protocol: String,
    /// The bound address (e.g. "0.0.0.0", "127.0.0.1", "::").
    pub address: String,
}

/// Scan all listening ports on the system.
///
/// Uses the `listeners` crate to enumerate every active listener, maps each
/// one to a [`PortEntry`], and returns them sorted by port number.
pub fn scan_ports() -> Result<Vec<PortEntry>> {
    let all = listeners::get_all()
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Failed to enumerate listening ports")?;

    let mut entries: Vec<PortEntry> = all
        .into_iter()
        .filter(|l| l.socket.port() > 0) // port 0 entries are not real listeners
        .map(|l| PortEntry {
            port: l.socket.port(),
            pid: l.process.pid,
            process_name: l.process.name.clone(),
            protocol: l.protocol.to_string(),
            address: l.socket.ip().to_string(),
        })
        .collect();

    entries.sort_by_key(|e| e.port);
    Ok(entries)
}

/// Scan listening ports that fall within the inclusive range `[lo, hi]`.
pub fn scan_ports_in_range(lo: u16, hi: u16) -> Result<Vec<PortEntry>> {
    let all = scan_ports()?;
    Ok(all.into_iter().filter(|e| e.port >= lo && e.port <= hi).collect())
}

/// Check whether a given port is free (no process is listening on it).
///
/// A port is considered free when no TCP **and** no UDP listener is bound to it.
pub fn is_port_free(port: u16) -> Result<bool> {
    let entries = scan_ports()?;
    Ok(!entries.iter().any(|e| e.port == port))
}

/// Suggest `count` free ports within the inclusive range `[range.0, range.1]`.
///
/// Returns an error if there aren't enough free ports in the range, or if the
/// range is inverted (start > end). Requesting 0 ports returns an empty list.
pub fn suggest_free_ports(count: usize, range: (u16, u16)) -> Result<Vec<u16>> {
    if count == 0 {
        return Ok(Vec::new());
    }

    if range.0 > range.1 {
        anyhow::bail!(
            "Invalid port range: start ({}) is greater than end ({})",
            range.0,
            range.1
        );
    }

    let occupied: std::collections::HashSet<u16> = scan_ports()?
        .iter()
        .map(|e| e.port)
        .collect();

    let mut free = Vec::with_capacity(count);
    for port in range.0..=range.1 {
        if !occupied.contains(&port) {
            free.push(port);
            if free.len() == count {
                return Ok(free);
            }
        }
    }

    anyhow::bail!(
        "Only found {} free ports in range {}-{}, but {} were requested",
        free.len(),
        range.0,
        range.1,
        count
    );
}
