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
    use std::process::Command;

    info!("Unregistering Windows service");

    let service_name = "data-exporter";

    // First, try to stop the service if it's running
    let stop_output = Command::new("sc.exe")
        .args(["stop", service_name])
        .output()
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to execute sc.exe stop: {}", e))
        })?;

    if !stop_output.status.success() {
        let stderr = String::from_utf8_lossy(&stop_output.stderr);
        info!("Service stop command returned error (may not be running): {}", stderr);
        // Continue anyway - service might not be running
    }

    // Delete the service
    let delete_output = Command::new("sc.exe")
        .args(["delete", service_name])
        .output()
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to execute sc.exe delete: {}", e))
        })?;

    if !delete_output.status.success() {
        let stderr = String::from_utf8_lossy(&delete_output.stderr);
        let stdout = String::from_utf8_lossy(&delete_output.stdout);

        // Check if service doesn't exist (not an error)
        if stderr.contains("1060") || stdout.contains("does not exist") {
            info!("Service does not exist, skipping deletion");
            return Ok(());
        }

        return Err(ProcessingError::ConfigurationError(format!(
            "Failed to delete service: stdout={}, stderr={}",
            stdout, stderr
        )));
    }

    info!("Windows service deletion initiated");

    // Wait a few seconds for Windows to complete the deletion
    info!("Waiting for service deletion to complete...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Windows service unregistered successfully");
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
