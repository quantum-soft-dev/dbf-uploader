// Data Exporter Windows Service
// This binary runs ONLY as a Windows service - no CLI, no install/uninstall logic

mod config;
mod processor;
mod service;
mod vss;

use anyhow::Result;

fn main() -> Result<()> {
    // NO CLI parsing
    // NO install/uninstall commands
    // ONLY run as Windows service
    service::windows_service::run_service()?;
    Ok(())
}
