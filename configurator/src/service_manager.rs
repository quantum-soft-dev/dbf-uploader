// Windows Service Management for Configurator
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

const SERVICE_NAME: &str = "data-exporter";
const INSTALL_DIR: &str = r"C:\Program Files\data-exporter";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    NotInstalled,
    Unknown,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceStatus::Running => write!(f, "Running"),
            ServiceStatus::Stopped => write!(f, "Stopped"),
            ServiceStatus::NotInstalled => write!(f, "Not Installed"),
            ServiceStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

pub struct ServiceManager;

impl ServiceManager {
    /// Check if error indicates access denied (need admin rights)
    fn format_error(stdout: &str, stderr: &str) -> String {
        let combined = format!("{}\n{}", stdout, stderr);
        if combined.contains("Access is denied") || combined.contains("FAILED 5") {
            "Access is denied.\n\nPlease run the Configurator as Administrator\n(Right-click → Run as administrator)".to_string()
        } else {
            combined
        }
    }

    /// Get current service status
    pub fn get_status() -> ServiceStatus {
        let output = match Command::new("sc.exe")
            .args(["query", SERVICE_NAME])
            .output()
        {
            Ok(output) => output,
            Err(_) => return ServiceStatus::Unknown,
        };

        if !output.status.success() {
            return ServiceStatus::NotInstalled;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.contains("RUNNING") {
            ServiceStatus::Running
        } else if stdout.contains("STOPPED") {
            ServiceStatus::Stopped
        } else {
            ServiceStatus::Unknown
        }
    }

    /// Install the service (requires service executable at default location)
    pub fn install() -> Result<String> {
        // Check if service already exists
        if Self::get_status() != ServiceStatus::NotInstalled {
            return Err(anyhow::anyhow!("Service is already installed"));
        }

        // Check if service executable exists in installation directory
        let exe_path = PathBuf::from(INSTALL_DIR).join("data_exporter.exe");
        if !exe_path.exists() {
            return Err(anyhow::anyhow!(
                "Service executable not found at: {}\n\n\
                Please copy data_exporter.exe to the installation directory first.",
                exe_path.display()
            ));
        }

        // Check if config exists
        let config_path = PathBuf::from(INSTALL_DIR).join("config.toml");
        if !config_path.exists() {
            return Err(anyhow::anyhow!(
                "Configuration file not found at: {}\n\n\
                Please save the configuration first.",
                config_path.display()
            ));
        }

        // Create service using sc.exe
        let display_name = "Data Exporter Service";
        let description = "Automatically exports DBF files to CSV, compresses to gzip, and uploads to cloud server";

        let output = Command::new("sc.exe")
            .args([
                "create",
                SERVICE_NAME,
                &format!("binPath={}", exe_path.display()),
                &format!("DisplayName={}", display_name),
                "start=auto",
                "type=own",
            ])
            .output()
            .context("Failed to execute sc.exe")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow::anyhow!(
                "Failed to create service:\n{}",
                Self::format_error(&stdout, &stderr)
            ));
        }

        // Set service description
        let _ = Command::new("sc.exe")
            .args(["description", SERVICE_NAME, description])
            .output();

        Ok("Service installed successfully".to_string())
    }

    /// Start the service
    pub fn start() -> Result<String> {
        let status = Self::get_status();

        if status == ServiceStatus::NotInstalled {
            return Err(anyhow::anyhow!("Service is not installed"));
        }

        if status == ServiceStatus::Running {
            return Err(anyhow::anyhow!("Service is already running"));
        }

        let output = Command::new("sc.exe")
            .args(["start", SERVICE_NAME])
            .output()
            .context("Failed to execute sc.exe")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow::anyhow!(
                "Failed to start service:\n{}",
                Self::format_error(&stdout, &stderr)
            ));
        }

        Ok("Service started successfully".to_string())
    }

    /// Stop the service
    pub fn stop() -> Result<String> {
        let status = Self::get_status();

        if status == ServiceStatus::NotInstalled {
            return Err(anyhow::anyhow!("Service is not installed"));
        }

        if status == ServiceStatus::Stopped {
            return Err(anyhow::anyhow!("Service is already stopped"));
        }

        let output = Command::new("sc.exe")
            .args(["stop", SERVICE_NAME])
            .output()
            .context("Failed to execute sc.exe")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow::anyhow!(
                "Failed to stop service:\n{}",
                Self::format_error(&stdout, &stderr)
            ));
        }

        Ok("Service stopped successfully".to_string())
    }

    /// Get detailed service information for display
    pub fn get_detailed_info() -> String {
        let status = Self::get_status();
        let mut info = String::new();

        info.push_str(&format!("Service Name: {}\n", SERVICE_NAME));
        info.push_str(&format!("Status: {}\n\n", status));

        // Get detailed query from sc.exe
        if let Ok(output) = Command::new("sc.exe")
            .args(["query", SERVICE_NAME])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse and format the output
                for line in stdout.lines() {
                    let line = line.trim();
                    if line.starts_with("STATE") {
                        info.push_str(&format!("{}\n", line));
                    } else if line.starts_with("WIN32_EXIT_CODE") {
                        info.push_str(&format!("{}\n", line));
                    } else if line.starts_with("SERVICE_EXIT_CODE") {
                        info.push_str(&format!("{}\n", line));
                    }
                }
            }
        }

        // Check installation paths
        let exe_path = PathBuf::from(INSTALL_DIR).join("data_exporter.exe");
        let config_path = PathBuf::from(INSTALL_DIR).join("config.toml");
        let log_dir = PathBuf::from(INSTALL_DIR).join("logs");

        info.push_str("\n--- Installation ---\n");
        info.push_str(&format!(
            "Executable: {} {}\n",
            exe_path.display(),
            if exe_path.exists() { "(found)" } else { "(not found)" }
        ));
        info.push_str(&format!(
            "Config: {} {}\n",
            config_path.display(),
            if config_path.exists() { "(found)" } else { "(not found)" }
        ));
        info.push_str(&format!(
            "Logs: {} {}\n",
            log_dir.display(),
            if log_dir.exists() { "(exists)" } else { "(not created)" }
        ));

        // Check for recent log files
        if log_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&log_dir) {
                let log_files: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "log")
                            .unwrap_or(false)
                    })
                    .collect();
                if !log_files.is_empty() {
                    info.push_str(&format!("\nLog files found: {}\n", log_files.len()));
                    // Show most recent log file
                    if let Some(recent) = log_files.iter().max_by_key(|e| {
                        e.metadata()
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                    }) {
                        info.push_str(&format!("Most recent: {}\n", recent.file_name().to_string_lossy()));
                    }
                }
            }
        }

        info
    }

    /// Uninstall the service
    pub fn uninstall() -> Result<String> {
        let status = Self::get_status();

        if status == ServiceStatus::NotInstalled {
            return Err(anyhow::anyhow!("Service is not installed"));
        }

        // Try to stop the service first
        if status == ServiceStatus::Running {
            let _ = Command::new("sc.exe").args(["stop", SERVICE_NAME]).output();

            // Wait for service to stop
            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        // Delete the service
        let output = Command::new("sc.exe")
            .args(["delete", SERVICE_NAME])
            .output()
            .context("Failed to execute sc.exe")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Check if service doesn't exist (not an error)
            if stderr.contains("1060") || stdout.contains("does not exist") {
                return Ok("Service not found (already uninstalled)".to_string());
            }

            return Err(anyhow::anyhow!(
                "Failed to delete service:\n{}",
                Self::format_error(&stdout, &stderr)
            ));
        }

        Ok("Service uninstalled successfully".to_string())
    }
}
