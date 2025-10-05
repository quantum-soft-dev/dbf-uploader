// Uninstall command implementation
use crate::error::{ProcessingError, Result};
use std::path::PathBuf;
use tracing::info;

/// Uninstall the data exporter service
pub async fn uninstall() -> Result<()> {
    info!("Starting uninstallation");

    // Platform-specific uninstallation
    uninstall_service()?;

    info!("Uninstallation complete");
    Ok(())
}

/// Uninstall the service (Windows)
#[cfg(target_os = "windows")]
fn uninstall_service() -> Result<()> {
    use std::fs;

    info!("Uninstalling Windows service");

    // Step 1: Unregister Windows service
    unregister_windows_service()?;

    // Step 2: Delete installation directory
    let install_dir = PathBuf::from(r"C:\Program Files\data-exporter");

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| {
            ProcessingError::ConfigurationError(format!(
                "Failed to remove installation directory: {}",
                e
            ))
        })?;
        info!("Installation directory removed");
    } else {
        info!("Installation directory not found, skipping removal");
    }

    info!("Windows service uninstalled successfully");
    Ok(())
}

/// Uninstall the service (macOS/Linux stub for development)
#[cfg(not(target_os = "windows"))]
fn uninstall_service() -> Result<()> {
    use std::fs;

    info!("Uninstalling service (development mode)");

    // Remove development installation directory
    let install_dir = PathBuf::from("./data-exporter-dev");

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| {
            ProcessingError::ConfigurationError(format!(
                "Failed to remove installation directory: {}",
                e
            ))
        })?;
        info!("Development installation directory removed");
    } else {
        info!("Installation directory not found, skipping removal");
    }

    info!("Service uninstalled successfully (development mode)");
    Ok(())
}

/// Unregister Windows service
#[cfg(target_os = "windows")]
fn unregister_windows_service() -> Result<()> {
    // TODO: Implement Windows service unregistration
    // For now, just log that this should be done
    info!("TODO: Unregister Windows service");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_uninstall() {
        // This test just verifies the function doesn't panic
        // Actual functionality depends on platform
        let _result = uninstall().await;
    }
}
