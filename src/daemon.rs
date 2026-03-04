use crate::config::PorthouseConfig;
use anyhow::Result;
use std::path::Path;

pub fn start(config: &PorthouseConfig, config_dir: &Path) -> Result<()> {
    let _ = (config, config_dir);
    println!("Daemon start not yet implemented");
    Ok(())
}

pub fn stop(config_dir: &Path) -> Result<()> {
    let _ = config_dir;
    println!("Daemon stop not yet implemented");
    Ok(())
}

pub fn status(config_dir: &Path) -> Result<()> {
    let _ = config_dir;
    println!("Daemon status not yet implemented");
    Ok(())
}
