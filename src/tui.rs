use crate::config::PorthouseConfig;
use crate::registry::Registry;
use anyhow::Result;

pub fn run(config: PorthouseConfig, registry: Registry) -> Result<()> {
    let _ = (&config, &registry);
    println!("TUI not yet implemented. Use 'porthouse status' for now.");
    Ok(())
}
