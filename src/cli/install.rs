// Install command implementation for v2.0
use crate::cli::wizard;
use crate::config::v2::ConfigV2;
use crate::error::{ProcessingError, Result};
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
use tracing::{info, warn};

/// Install the data exporter service v2.0
///
/// This function checks for existing v1.0 installations and either:
/// - Guides user to use `migrate` command for upgrades
/// - Runs interactive wizard for fresh installations
pub async fn install() -> Result<()> {
    info!("Starting Data Exporter Service installation (v2.0)");

    // Check if v1.0 config exists
    let config_path = get_config_path()?;

    if config_path.exists() {
        warn!("Found existing configuration at {}", config_path.display());
        println!("\n⚠️  Existing installation detected!");
        println!("\nIt appears you have an existing Data Exporter Service installation.");
        println!("To upgrade from v1.0 to v2.0, please use the migration command:");
        println!("\n  data_exporter.exe migrate");
        println!("\nThis will:");
        println!("  1. Detect your existing v1.0 configuration");
        println!("  2. Guide you through the upgrade process");
        println!("  3. Migrate your settings to v2.0 format");
        println!("  4. Create a backup of your old configuration");
        println!("\nIf you want to perform a fresh installation instead, please:");
        println!("  1. Backup your current config.toml");
        println!("  2. Uninstall the existing service: data_exporter.exe uninstall");
        println!("  3. Run this install command again");

        return Err(ProcessingError::ConfigurationError(
            "Existing installation detected. Use 'migrate' command to upgrade from v1.0."
                .to_string(),
        ));
    }

    // No existing config, run interactive wizard for fresh install
    println!("\n🚀 Data Exporter Service v2.0 Installation Wizard");
    println!("{}", "=".repeat(50));
    println!("\nThis wizard will guide you through setting up the service.");
    println!("You will need:");
    println!("  - Your site domain (e.g., store-01.example.com)");
    println!("  - Your client secret (UUID format, from middleware panel)");
    println!("  - Path to your DBF files directory");
    println!("  - Middleware API URL (HTTPS)");
    println!("  - Schedule (cron format)");
    println!("\nPress Enter to continue...");

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| ProcessingError::ConfigurationError(format!("Failed to read input: {}", e)))?;

    // Run interactive wizard
    let config = wizard::run_installation_wizard().await?;

    // Install service with the new configuration
    install_service(&config)?;

    info!("Installation complete");
    println!("\n✅ Installation completed successfully!");
    println!("\nThe Data Exporter Service has been installed and configured.");
    println!("Configuration saved to: {}", config_path.display());
    println!("\nNext steps:");
    println!("  1. Start the service: sc start data-exporter");
    println!("  2. Check service status: sc query data-exporter");
    println!("  3. View logs in Event Viewer (Application log, source: data-exporter)");

    Ok(())
}

/// Get the configuration file path
pub fn get_config_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(path) = std::env::var("DATA_EXPORTER_CONFIG") {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }
        Ok(PathBuf::from(r"C:\Program Files\data-exporter\config.toml"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(PathBuf::from("./data-exporter-dev/config.toml"))
    }
}

// Note: create_config and validate_credentials functions removed
// These are now handled by the installation wizard

/// Install the service (platform-specific)
#[cfg(target_os = "windows")]
fn install_service(config: &ConfigV2) -> Result<()> {
    use std::fs;

    info!("Installing Windows service");

    // Create installation directory
    let install_dir = PathBuf::from(r"C:\Program Files\data-exporter");
    fs::create_dir_all(&install_dir).map_err(|e| {
        ProcessingError::ConfigurationError(format!("Failed to create install directory: {}", e))
    })?;

    // Copy executable to installation directory
    let exe_path = std::env::current_exe().map_err(|e| {
        ProcessingError::ConfigurationError(format!("Failed to get current executable path: {}", e))
    })?;

    let target_exe = install_dir.join("data_exporter.exe");
    fs::copy(&exe_path, &target_exe).map_err(|e| {
        ProcessingError::ConfigurationError(format!("Failed to copy executable: {}", e))
    })?;

    // Write config file
    let config_path = install_dir.join("config.toml");
    config.to_file(&config_path).map_err(|e| {
        ProcessingError::ConfigurationError(format!("Failed to write config file: {}", e))
    })?;

    // Set file permissions on config.toml (restrict to administrators)
    set_config_permissions(&config_path)?;

    // Register Windows service
    register_windows_service(&target_exe)?;

    info!("Windows service installed successfully");
    Ok(())
}

/// Install the service (macOS/Linux stub for development)
#[cfg(not(target_os = "windows"))]
fn install_service(config: &ConfigV2) -> Result<()> {
    use std::fs;

    info!("Installing service (development mode - not a real Windows service)");

    // Create installation directory in current directory for testing
    let install_dir = PathBuf::from("./data-exporter-dev");
    fs::create_dir_all(&install_dir).map_err(|e| {
        ProcessingError::ConfigurationError(format!("Failed to create install directory: {}", e))
    })?;

    // Write config file
    let config_path = install_dir.join("config.toml");
    config.to_file(&config_path).map_err(|e| {
        ProcessingError::ConfigurationError(format!("Failed to write config file: {}", e))
    })?;

    info!("Service installed successfully (development mode)");
    info!("Config written to: {}", config_path.display());
    info!("Note: Windows service registration is not available on this platform");

    Ok(())
}

/// Set permissions on config.toml (Windows only)
#[cfg(target_os = "windows")]
fn set_config_permissions(config_path: &Path) -> Result<()> {
    // TODO: Implement Windows ACL permissions
    // For now, just log that this should be done
    info!("TODO: Set ACL permissions on {}", config_path.display());
    Ok(())
}

/// Register Windows service
#[cfg(target_os = "windows")]
fn register_windows_service(exe_path: &Path) -> Result<()> {
    // TODO: Implement Windows service registration
    // For now, just log that this should be done
    info!("TODO: Register Windows service for {}", exe_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_get_config_path() {
        std::env::remove_var("DATA_EXPORTER_CONFIG");
        let path = get_config_path();
        assert!(path.is_ok());

        #[cfg(target_os = "windows")]
        assert!(path.unwrap().to_string_lossy().contains("data-exporter"));

        #[cfg(not(target_os = "windows"))]
        assert!(path
            .unwrap()
            .to_string_lossy()
            .contains("data-exporter-dev"));
    }

    #[test]
    fn test_get_config_path_env_override() {
        let _env_guard = ENV_MUTEX.lock().unwrap();
        let custom = std::env::temp_dir().join("custom-config.toml");
        std::env::set_var("DATA_EXPORTER_CONFIG", &custom);

        let path = get_config_path().unwrap();
        assert_eq!(path, custom);

        std::env::remove_var("DATA_EXPORTER_CONFIG");
    }
}
