use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "porthouse", about = "A lighthouse for your ports", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Show all listening ports (one-shot)
    Status,
    /// Check for port conflicts (exit 1 if any)
    Check {
        #[arg(short, long)]
        quiet: bool,
        #[arg(long)]
        json: bool,
    },
    /// Suggest free ports
    Suggest {
        #[arg(default_value = "1")]
        count: usize,
        #[arg(short, long, default_value = "1024")]
        from: u16,
        #[arg(short, long, default_value = "65535")]
        to: u16,
    },
    /// Register a project with port reservations
    Register {
        name: String,
        #[arg(short, long)]
        range: Option<String>,
        #[arg(short, long)]
        ports: Option<String>,
    },
    /// Kill process on a specific port
    Kill {
        port: u16,
    },
    /// Check if a specific port is free
    Free {
        port: u16,
    },
}

#[derive(Subcommand, Debug)]
pub enum DaemonAction {
    Start,
    Stop,
    Status,
}
