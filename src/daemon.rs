use crate::alert::{AlertEvent, AlertManager};
use crate::config::PorthouseConfig;
use crate::conflict;
use crate::process;
use crate::registry::Registry;
use crate::scanner;
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

/// Reject PID file if it's a symlink (prevents symlink attacks).
fn reject_symlink(path: &std::path::Path) -> Result<()> {
    if path.exists() {
        let meta = std::fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            anyhow::bail!(
                "PID file {} is a symlink — refusing to use it (possible symlink attack)",
                path.display()
            );
        }
    }
    Ok(())
}

pub fn start(config: &PorthouseConfig, config_dir: &Path) -> Result<()> {
    let pid_file = config_dir.join("daemon.pid");

    reject_symlink(&pid_file)?;

    // Check if daemon already running
    if pid_file.exists() {
        let existing_pid: u32 = std::fs::read_to_string(&pid_file)?.trim().parse()?;
        if process::is_process_alive(existing_pid) {
            println!("Daemon already running (PID {})", existing_pid);
            return Ok(());
        }
        // Stale PID file, remove it
        let _ = std::fs::remove_file(&pid_file);
    }

    let pid = std::process::id();
    std::fs::create_dir_all(config_dir)?;
    std::fs::write(&pid_file, pid.to_string())?;

    // Set restrictive permissions on PID file and config dir
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(config_dir, std::fs::Permissions::from_mode(0o700));
        let _ = std::fs::set_permissions(&pid_file, std::fs::Permissions::from_mode(0o600));
    }

    let alert_manager = AlertManager::new(config.alerts.clone());
    let scan_secs = config.daemon.scan_interval_secs.max(1); // minimum 1s to prevent busy-loop
    let interval = std::time::Duration::from_secs(scan_secs);
    let _registry = Registry::load_or_default(&config_dir.join("registry.toml"));

    // Signal handling for clean shutdown
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    #[cfg(unix)]
    {
        signal_hook::flag::register(signal_hook::consts::SIGTERM, shutdown.clone())?;
        signal_hook::flag::register(signal_hook::consts::SIGINT, shutdown.clone())?;
    }

    #[cfg(windows)]
    {
        let s = shutdown.clone();
        ctrlc::set_handler(move || {
            s.store(true, std::sync::atomic::Ordering::Relaxed);
        })?;
    }

    let mut prev_ports: HashSet<u16> = HashSet::new();
    let mut prev_conflict_ports: HashSet<u16> = HashSet::new();

    println!(
        "Porthouse daemon running (PID {}). Scanning every {}s.",
        pid, config.daemon.scan_interval_secs
    );

    while !shutdown.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(entries) = scanner::scan_ports_in_range(config.daemon.port_range.0, config.daemon.port_range.1) {
            let current_ports: HashSet<u16> = entries.iter().map(|e| e.port).collect();

            // Only alert after the first scan (skip initial state)
            if !prev_ports.is_empty() {
                // Detect new listeners
                for entry in &entries {
                    if !prev_ports.contains(&entry.port) {
                        alert_manager.fire(&AlertEvent::NewListener {
                            port: entry.port,
                            process: entry.process_name.clone(),
                            pid: entry.pid,
                        });
                    }
                }

                // Detect freed ports
                for port in &prev_ports {
                    if !current_ports.contains(port) {
                        alert_manager.fire(&AlertEvent::PortFreed { port: *port });
                    }
                }
            }

            // Detect conflicts (always, including first scan)
            let conflicts = conflict::detect_conflicts(&entries);
            let conflict_ports: HashSet<u16> = conflicts.iter().map(|c| c.port).collect();
            for c in &conflicts {
                if !prev_conflict_ports.contains(&c.port) {
                    let processes: Vec<(String, u32)> = c
                        .entries
                        .iter()
                        .map(|e| (e.process_name.clone(), e.pid))
                        .collect();
                    alert_manager.fire(&AlertEvent::Conflict {
                        port: c.port,
                        processes,
                    });
                }
            }

            prev_ports = current_ports;
            prev_conflict_ports = conflict_ports;
        }

        std::thread::sleep(interval);
    }

    // Cleanup
    let _ = std::fs::remove_file(&pid_file);
    println!("Porthouse daemon stopped.");
    Ok(())
}

pub fn stop(config_dir: &Path) -> Result<()> {
    let pid_file = config_dir.join("daemon.pid");
    if !pid_file.exists() {
        println!("No daemon running.");
        return Ok(());
    }
    reject_symlink(&pid_file)?;
    let pid: u32 = std::fs::read_to_string(&pid_file)?.trim().parse()?;
    if !process::is_process_alive(pid) {
        println!("Daemon is not running (stale PID file).");
        let _ = std::fs::remove_file(&pid_file);
        return Ok(());
    }
    process::kill_process(pid)?;
    let _ = std::fs::remove_file(&pid_file);
    println!("Stopped daemon (PID {}).", pid);
    Ok(())
}

pub fn status(config_dir: &Path) -> Result<()> {
    let pid_file = config_dir.join("daemon.pid");
    if !pid_file.exists() {
        println!("Daemon is not running.");
        return Ok(());
    }
    reject_symlink(&pid_file)?;
    let pid: u32 = std::fs::read_to_string(&pid_file)?.trim().parse()?;
    if process::is_process_alive(pid) {
        println!("Daemon is running (PID {}).", pid);
    } else {
        println!("Daemon is not running (stale PID file).");
        let _ = std::fs::remove_file(&pid_file);
    }
    Ok(())
}
