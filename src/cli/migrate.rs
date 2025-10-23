// Migration command implementation for v1.0 → v2.0
use crate::error::{ProcessingError, Result};
use std::path::PathBuf;
use tracing::{info, warn};

/// Migrate from v1.0 to v2.0
///
/// This function performs the following steps:
/// 1. Detects existing v1.0 configuration
/// 2. Prompts for new site credentials (domain + client_secret)
/// 3. Migrates all other settings automatically
/// 4. Creates backup of old configuration (config.toml.v1.backup)
/// 5. Writes new v2.0 configuration
/// 6. Restarts Windows service with new configuration
pub async fn migrate() -> Result<()> {
    info!("Starting migration from v1.0 to v2.0");

    // Get config path
    let config_path = get_config_path()?;

    // Check if v1.0 config exists
    if !config_path.exists() {
        warn!(
            "No existing configuration found at {}",
            config_path.display()
        );
        println!("\n⚠️  No existing installation detected!");
        println!("\nThe migration command is for upgrading existing v1.0 installations.");
        println!("If you are installing for the first time, please use:");
        println!("\n  data_exporter.exe install");

        return Err(ProcessingError::ConfigurationError(
            "No existing v1.0 configuration found. Use 'install' command for fresh installations."
                .to_string(),
        ));
    }

    println!("\n🔄 Data Exporter Service v1.0 → v2.0 Migration Wizard");
    println!("{}", "=".repeat(50));
    println!("\nThis wizard will guide you through upgrading to v2.0.");
    println!("\nWhat will happen:");
    println!("  1. Backup your current v1.0 configuration");
    println!("  2. Convert settings to v2.0 format");
    println!("  3. Request new site credentials (domain + client_secret)");
    println!("  4. Install v2.0 service");
    println!("\nPress Enter to continue...");

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| ProcessingError::ConfigurationError(format!("Failed to read input: {}", e)))?;

    // TODO: Implement full migration logic:
    // - Read v1.0 config.toml
    // - Parse v1.0 format
    // - Prompt for new credentials
    // - Convert to v2.0 format
    // - Create backup
    // - Write new config
    // - Restart service

    info!("Migration wizard not yet fully implemented");
    println!("\n⚠️  Migration wizard implementation in progress.");
    println!("For manual migration, see MIGRATION_GUIDE.md");

    Err(ProcessingError::ConfigurationError(
        "Migration wizard not yet fully implemented. See MIGRATION_GUIDE.md for manual migration steps.".to_string()
    ))
}

/// Get the configuration file path
fn get_config_path() -> Result<PathBuf> {
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
        let custom = std::env::temp_dir().join("migrate-config.toml");
        std::env::set_var("DATA_EXPORTER_CONFIG", &custom);

        let path = get_config_path().unwrap();
        assert_eq!(path, custom);

        std::env::remove_var("DATA_EXPORTER_CONFIG");
    }
}
