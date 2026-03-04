use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::process;

use porthouse::cli::{Cli, Commands, DaemonAction};
use porthouse::config::PorthouseConfig;
use porthouse::conflict;
use porthouse::registry::Registry;
use porthouse::scanner;

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".porthouse")
}

fn main() {
    let cli = Cli::parse();

    let config_dir = config_dir();
    let config_path = config_dir.join("config.toml");
    let registry_path = config_dir.join("registry.toml");

    let config = PorthouseConfig::load_or_default(&config_path);
    let registry = Registry::load_or_default(&registry_path);

    let result = match cli.command {
        None => porthouse::tui::run(config, registry),
        Some(Commands::Status) => cmd_status(),
        Some(Commands::Check { quiet, json }) => cmd_check(quiet, json),
        Some(Commands::Suggest { count, from, to }) => cmd_suggest(count, from, to),
        Some(Commands::Register { name, range, ports }) => {
            cmd_register(registry, &registry_path, &name, range.as_deref(), ports.as_deref())
        }
        Some(Commands::Kill { port }) => cmd_kill(port),
        Some(Commands::Free { port }) => cmd_free(port),
        Some(Commands::Daemon { action }) => match action {
            DaemonAction::Start => porthouse::daemon::start(&config, &config_dir),
            DaemonAction::Stop => porthouse::daemon::stop(&config_dir),
            DaemonAction::Status => porthouse::daemon::status(&config_dir),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        process::exit(1);
    }
}

/// Scan ports and print a table of PORT/PID/PROCESS/PROTO/ADDRESS, then show any conflicts.
fn cmd_status() -> Result<()> {
    let entries = scanner::scan_ports().context("Failed to scan ports")?;

    if entries.is_empty() {
        println!("No listening ports found.");
        return Ok(());
    }

    // Print header
    println!(
        "{:<8} {:<8} {:<20} {:<8} {}",
        "PORT", "PID", "PROCESS", "PROTO", "ADDRESS"
    );
    println!("{}", "-".repeat(60));

    for entry in &entries {
        println!(
            "{:<8} {:<8} {:<20} {:<8} {}",
            entry.port, entry.pid, entry.process_name, entry.protocol, entry.address
        );
    }

    // Show conflicts
    let conflicts = conflict::detect_conflicts(&entries);
    if !conflicts.is_empty() {
        println!();
        println!("CONFLICTS DETECTED:");
        println!("{}", "-".repeat(60));
        for c in &conflicts {
            println!("  Port {}:", c.port);
            for entry in &c.entries {
                println!(
                    "    - {} (PID {}) [{}] on {}",
                    entry.process_name, entry.pid, entry.protocol, entry.address
                );
            }
            let suggestion = conflict::suggest_resolution(c.port, &entries);
            println!("    Suggested alternative: port {}", suggestion);
        }
    }

    Ok(())
}

/// Scan ports, detect conflicts, exit 0 if none, exit 1 if conflicts.
/// With --json, output JSON. With --quiet, no output.
fn cmd_check(quiet: bool, json: bool) -> Result<()> {
    let entries = scanner::scan_ports().context("Failed to scan ports")?;
    let conflicts = conflict::detect_conflicts(&entries);

    if json {
        let json_conflicts: Vec<serde_json::Value> = conflicts
            .iter()
            .map(|c| {
                let procs: Vec<serde_json::Value> = c
                    .entries
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "pid": e.pid,
                            "process": e.process_name,
                            "protocol": e.protocol,
                            "address": e.address,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "port": c.port,
                    "processes": procs,
                })
            })
            .collect();

        let output = serde_json::json!({
            "conflicts": json_conflicts,
            "count": conflicts.len(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if !quiet && !conflicts.is_empty() {
        println!("Found {} conflict(s):", conflicts.len());
        for c in &conflicts {
            println!("  Port {}:", c.port);
            for entry in &c.entries {
                println!(
                    "    - {} (PID {})",
                    entry.process_name, entry.pid
                );
            }
        }
    } else if !quiet {
        println!("No conflicts detected.");
    }

    if !conflicts.is_empty() {
        process::exit(1);
    }

    Ok(())
}

/// Suggest free ports in the given range.
fn cmd_suggest(count: usize, from: u16, to: u16) -> Result<()> {
    let ports = scanner::suggest_free_ports(count, (from, to))
        .context("Failed to suggest free ports")?;
    for port in ports {
        println!("{}", port);
    }
    Ok(())
}

/// Register a project with port reservations.
fn cmd_register(
    mut registry: Registry,
    registry_path: &PathBuf,
    name: &str,
    range: Option<&str>,
    ports: Option<&str>,
) -> Result<()> {
    let parsed_range = match range {
        Some(r) => {
            let parts: Vec<&str> = r.split('-').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid range format '{}'. Expected format: 3000-3010", r);
            }
            let lo: u16 = parts[0]
                .trim()
                .parse()
                .context("Invalid start of range")?;
            let hi: u16 = parts[1]
                .trim()
                .parse()
                .context("Invalid end of range")?;
            Some((lo, hi))
        }
        None => None,
    };

    let parsed_ports: Vec<u16> = match ports {
        Some(p) => p
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<u16>()
                    .context(format!("Invalid port number: '{}'", s.trim()))
            })
            .collect::<Result<Vec<u16>>>()?,
        None => Vec::new(),
    };

    registry.register(name, None, parsed_ports.clone(), parsed_range);
    registry.save(registry_path).context("Failed to save registry")?;

    println!("Registered project '{}'", name);
    if let Some((lo, hi)) = parsed_range {
        println!("  Range: {}-{}", lo, hi);
    }
    if !parsed_ports.is_empty() {
        let port_strs: Vec<String> = parsed_ports.iter().map(|p| p.to_string()).collect();
        println!("  Ports: {}", port_strs.join(", "));
    }

    Ok(())
}

/// Kill the process on a specific port.
fn cmd_kill(port: u16) -> Result<()> {
    let entries = scanner::scan_ports().context("Failed to scan ports")?;
    let on_port: Vec<_> = entries.iter().filter(|e| e.port == port).collect();

    if on_port.is_empty() {
        println!("No process found listening on port {}", port);
        return Ok(());
    }

    for entry in &on_port {
        println!(
            "Killing {} (PID {}) on port {}...",
            entry.process_name, entry.pid, port
        );
        let ret = unsafe { libc::kill(entry.pid as i32, libc::SIGTERM) };
        if ret == 0 {
            println!("  Sent SIGTERM to PID {}", entry.pid);
        } else {
            let err = std::io::Error::last_os_error();
            eprintln!("  Failed to kill PID {}: {}", entry.pid, err);
        }
    }

    Ok(())
}

/// Check if a specific port is free.
fn cmd_free(port: u16) -> Result<()> {
    let free = scanner::is_port_free(port).context("Failed to check port")?;
    if free {
        println!("Port {} is free", port);
    } else {
        println!("Port {} is in use", port);
        // Show what's using it
        let entries = scanner::scan_ports()?;
        for entry in entries.iter().filter(|e| e.port == port) {
            println!(
                "  {} (PID {}) [{}] on {}",
                entry.process_name, entry.pid, entry.protocol, entry.address
            );
        }
    }
    Ok(())
}
