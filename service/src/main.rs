// Data Exporter Windows Service
// This binary runs ONLY as a Windows service - no CLI, no install/uninstall logic

use anyhow::Result;
use data_exporter_service::service;

fn main() -> Result<()> {
    // NO CLI parsing
    // NO install/uninstall commands
    // ONLY run as Windows service
    service::windows_service::run_service()?;
    Ok(())
}
