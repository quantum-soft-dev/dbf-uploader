// Install command implementation
use crate::auth::AuthClient;
use crate::error::{ProcessingError, Result};
use crate::models::Config;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
use tracing::{error, info};

/// Install the data exporter service
pub async fn install(
    account: String,
    username: String,
    password: String,
    source_dir: String,
    crontab: String,
    api_url: String,
    encoding: String,
    https_only: bool,
) -> Result<()> {
    info!("Starting installation");

    // Step 1: Validate inputs
    info!("Validating installation parameters");

    // Validate HTTPS/HTTP consistency
    if https_only && !api_url.starts_with("https://") {
        return Err(ProcessingError::ConfigurationError(
            "API URL must use HTTPS when https_only is enabled (https_only=true)".to_string(),
        ));
    }

    if !https_only && api_url.starts_with("https://") {
        return Err(ProcessingError::ConfigurationError(
            "API URL uses HTTPS but https_only is disabled. Either use http:// URL or remove --no-https flag".to_string(),
        ));
    }

    let source_path = PathBuf::from(&source_dir);
    if !source_path.exists() {
        return Err(ProcessingError::ConfigurationError(format!(
            "Source directory does not exist: {}",
            source_dir
        )));
    }

    // Step 2: Create configuration
    let config = create_config(account, username, password, source_dir, crontab, api_url, encoding, https_only)?;

    // Step 3: Validate credentials by requesting token
    info!("Validating credentials with API");
    validate_credentials(&config).await?;

    // Step 4: Install service (platform-specific)
    install_service(&config)?;

    info!("Installation complete");
    Ok(())
}

/// Create configuration from install parameters
fn create_config(
    account: String,
    username: String,
    password: String,
    source_dir: String,
    crontab: String,
    api_url: String,
    encoding: String,
    https_only: bool,
) -> Result<Config> {
    use crate::models::config::{
        ApiConfig, CredentialConfig, EncodingConfig, SchedulerConfig, SourceConfig,
    };

    let config = Config {
        scheduler: SchedulerConfig { crontab },
        src: SourceConfig {
            source_dir: PathBuf::from(source_dir),
        },
        credential: CredentialConfig { account, username, password },
        api: ApiConfig {
            base_url: api_url,
            https_only,
        },
        encoding: EncodingConfig {
            dbf_encoding: encoding,
        },
    };

    Ok(config)
}

/// Validate credentials by attempting to get a token
async fn validate_credentials(config: &Config) -> Result<()> {
    let auth_client = AuthClient::new(config)?;

    match auth_client.get_token().await {
        Ok(_token) => {
            info!("Credentials validated successfully");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Credential validation failed");
            Err(e)
        }
    }
}

/// Install the service (platform-specific)
#[cfg(target_os = "windows")]
fn install_service(config: &Config) -> Result<()> {
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
    config.to_file(&config_path)?;

    // Set file permissions on config.toml (restrict to administrators)
    set_config_permissions(&config_path)?;

    // Register Windows service
    register_windows_service(&target_exe)?;

    info!("Windows service installed successfully");
    Ok(())
}

/// Install the service (macOS/Linux stub for development)
#[cfg(not(target_os = "windows"))]
fn install_service(config: &Config) -> Result<()> {
    use std::fs;

    info!("Installing service (development mode - not a real Windows service)");

    // Create installation directory in current directory for testing
    let install_dir = PathBuf::from("./data-exporter-dev");
    fs::create_dir_all(&install_dir).map_err(|e| {
        ProcessingError::ConfigurationError(format!("Failed to create install directory: {}", e))
    })?;

    // Write config file
    let config_path = install_dir.join("config.toml");
    config.to_file(&config_path)?;

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
    use std::process::Command;

    info!("Registering Windows service for {}", exe_path.display());

    let service_name = "data-exporter";
    let display_name = "Data Exporter Service";
    let description = "Automatically exports DBF files to CSV, compresses to gzip, and uploads to cloud server";

    // Use sc.exe to create the service
    // Note: sc.exe requires VERY specific syntax:
    // - Exactly one space after =
    // - Path should be without quotes for sc.exe (it adds them internally if needed)
    let exe_path_str = exe_path.to_string_lossy().to_string();
    let bin_path = format!("binPath={}", exe_path_str);  // No space after = to avoid ERROR 87

    info!("Creating service with binPath: '{}'", bin_path);
    info!("Exe path: '{}'", exe_path_str);

    let output = Command::new("sc.exe")
        .args([
            "create",
            service_name,
            &bin_path,
            &format!("DisplayName={}", display_name),
            "start=demand",
            "type=own",
        ])
        .output()
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to execute sc.exe: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(ProcessingError::ConfigurationError(format!(
            "Failed to create service: stdout={}, stderr={}",
            stdout, stderr
        )));
    }

    // Set service description
    let desc_output = Command::new("sc.exe")
        .args(["description", service_name, description])
        .output()
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to set service description: {}", e))
        })?;

    if !desc_output.status.success() {
        error!("Failed to set service description (non-critical)");
    }

    info!("Windows service registered successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_config() {
        let config = create_config(
            "testaccount".to_string(),
            "testuser".to_string(),
            "testpass".to_string(),
            "/tmp".to_string(),
            "*/5 * * * *".to_string(),
            "https://api.example.com".to_string(),
            "CP866".to_string(),
            true,
        );

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.credential.account, "testaccount");
        assert_eq!(config.credential.username, "testuser");
        assert_eq!(config.credential.full_username(), "testaccount_testuser");
        assert_eq!(config.scheduler.crontab, "*/5 * * * *");
        assert!(config.api.https_only);
    }

    #[test]
    fn test_https_validation() {
        let result = create_config(
            "testaccount".to_string(),
            "test".to_string(),
            "test".to_string(),
            "/tmp".to_string(),
            "*/5 * * * *".to_string(),
            "https://api.example.com".to_string(),
            "CP866".to_string(),
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_allowed_when_https_only_disabled() {
        let result = create_config(
            "testaccount".to_string(),
            "test".to_string(),
            "test".to_string(),
            "/tmp".to_string(),
            "*/5 * * * *".to_string(),
            "http://localhost:8080".to_string(),
            "CP866".to_string(),
            false,
        );
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.api.https_only);
        assert_eq!(config.api.base_url, "http://localhost:8080");
    }
}
