use crate::alert::{AlertEvent, AlertManager};
use crate::config::PorthouseConfig;
use crate::conflict;
use crate::registry::Registry;
use crate::scanner;
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

pub fn start(config: &PorthouseConfig, config_dir: &Path) -> Result<()> {
    let pid_file = config_dir.join("daemon.pid");

    // Check if daemon already running
    if pid_file.exists() {
        let existing_pid: i32 = std::fs::read_to_string(&pid_file)?.trim().parse()?;
        let alive = unsafe { libc::kill(existing_pid, 0) == 0 };
        if alive {
            println!("Daemon already running (PID {})", existing_pid);
            return Ok(());
        }
        // Stale PID file, remove it
        let _ = std::fs::remove_file(&pid_file);
    }

    let pid = std::process::id();
    std::fs::create_dir_all(config_dir)?;
    std::fs::write(&pid_file, pid.to_string())?;

    let alert_manager = AlertManager::new(config.alerts.clone());
    let interval = std::time::Duration::from_secs(config.daemon.scan_interval_secs);
    let _registry = Registry::load_or_default(&config_dir.join("registry.toml"));

    // Signal handling for clean shutdown
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, shutdown.clone())?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, shutdown.clone())?;

    let mut prev_ports: HashSet<u16> = HashSet::new();
    let mut prev_conflict_ports: HashSet<u16> = HashSet::new();

    println!(
        "Porthouse daemon running (PID {}). Scanning every {}s.",
        pid, config.daemon.scan_interval_secs
    );

    while !shutdown.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(entries) = scanner::scan_ports() {
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
    let pid: i32 = std::fs::read_to_string(&pid_file)?.trim().parse()?;
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
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
    let pid: i32 = std::fs::read_to_string(&pid_file)?.trim().parse()?;
    let alive = unsafe { libc::kill(pid, 0) == 0 };
    if alive {
        println!("Daemon is running (PID {}).", pid);
    } else {
        println!("Daemon is not running (stale PID file).");
        let _ = std::fs::remove_file(&pid_file);
    }
    Ok(())
}
