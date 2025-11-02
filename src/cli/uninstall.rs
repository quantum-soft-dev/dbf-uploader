// Uninstall command implementation
use crate::error::{ProcessingError, Result};
use std::path::PathBuf;
use tracing::info;

/// Uninstall the data exporter service
pub async fn uninstall() -> Result<()> {
    info!("Uninstalling Data Exporter Service...");

    // Platform-specific uninstallation
    uninstall_service()?;

    info!("Uninstallation completed successfully");
    Ok(())
}

/// Uninstall the service (Windows)
#[cfg(target_os = "windows")]
fn uninstall_service() -> Result<()> {
    use std::fs;
    use std::process::Command;

    info!("Stopping and removing service...");

    // Step 1: Unregister Windows service
    unregister_windows_service()?;

    // Step 2: Schedule deletion of installation directory
    // We cannot delete the directory while the exe is running from it
    // So we create a batch file that will delete it after the process exits
    info!("Scheduling removal of installation files...");
    let install_dir = PathBuf::from(r"C:\Program Files\data-exporter");

    if install_dir.exists() {
        // Create a temporary batch file to delete the directory after process exits
        let temp_dir = std::env::temp_dir();
        let batch_path = temp_dir.join("uninstall_data_exporter.bat");

        let batch_content = format!(
            r#"@echo off
rem Wait for the process to exit
timeout /t 2 /nobreak >nul
rem Delete the installation directory
rmdir /s /q "{}"
rem Delete this batch file itself
del "%~f0"
"#,
            install_dir.display()
        );

        fs::write(&batch_path, batch_content).map_err(|e| {
            ProcessingError::ConfigurationError(format!(
                "Failed to create uninstall batch file: {}",
                e
            ))
        })?;

        // Execute the batch file in a detached process
        Command::new("cmd.exe")
            .args(["/C", "start", "/B", batch_path.to_str().unwrap()])
            .spawn()
            .map_err(|e| {
                ProcessingError::ConfigurationError(format!(
                    "Failed to start cleanup process: {}",
                    e
                ))
            })?;

        info!("Installation files will be removed after process exits");
    }

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

    let service_name = "data-exporter";

    // First, try to stop the service if it's running
    let _stop_output = Command::new("sc.exe")
        .args(["stop", service_name])
        .output()
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to execute sc.exe stop: {}", e))
        })?;

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
            return Ok(());
        }

        return Err(ProcessingError::ConfigurationError(format!(
            "Failed to delete service: stdout={}, stderr={}",
            stdout, stderr
        )));
    }

    // Wait a few seconds for Windows to complete the deletion
    std::thread::sleep(std::time::Duration::from_secs(3));

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
