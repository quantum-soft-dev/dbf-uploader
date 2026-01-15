// Install command implementation
use crate::auth::AuthClient;
use crate::error::{ProcessingError, Result};
use crate::models::Config;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
use tracing::info;

/// Installation parameters
pub struct InstallParams {
    pub use_device_flow: bool,
    pub site_name: Option<String>,
    pub site_description: Option<String>,
    pub account: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub source_dir: String,
    pub crontab: String,
    pub api_url: String,
    pub encoding: String,
    pub https_only: bool,
}

/// Install the data exporter service
pub async fn install(params: InstallParams) -> Result<()> {
    info!("Installing Data Exporter Service...");

    // Validate HTTPS/HTTP consistency
    if params.https_only && !params.api_url.starts_with("https://") {
        return Err(ProcessingError::ConfigurationError(
            "API URL must use HTTPS when https_only is enabled (https_only=true)".to_string(),
        ));
    }

    if !params.https_only && params.api_url.starts_with("https://") {
        return Err(ProcessingError::ConfigurationError(
            "API URL uses HTTPS but https_only is disabled. Either use http:// URL or remove --no-https flag".to_string(),
        ));
    }

    let source_path = PathBuf::from(&params.source_dir);
    if !source_path.exists() {
        return Err(ProcessingError::ConfigurationError(format!(
            "Source directory does not exist: {}",
            params.source_dir
        )));
    }

    // Route to device flow or traditional installation
    if params.use_device_flow {
        install_with_device_flow(params).await
    } else {
        install_with_traditional_auth(params).await
    }
}

/// Install with traditional credentials
async fn install_with_traditional_auth(params: InstallParams) -> Result<()> {
    info!("Installing with traditional credentials...");

    // Create configuration
    let config = create_config_traditional(&params)?;

    // Validate credentials by requesting token
    info!("Validating credentials...");
    validate_credentials(&config).await?;

    // Install service (platform-specific)
    install_service(&config)?;

    info!("Installation completed successfully");
    Ok(())
}

/// Install with Device Authorization Flow
async fn install_with_device_flow(params: InstallParams) -> Result<()> {
    use crate::auth::device_flow::{DeviceFlowClient, SiteInfo};

    info!("Starting device authorization flow installation...");

    // Create Device Flow client
    let client = DeviceFlowClient::new(params.api_url.clone(), params.https_only)?;

    // Prepare site information
    let site_name = params.site_name.clone().ok_or_else(|| {
        ProcessingError::ConfigurationError("Site name required for device flow".to_string())
    })?;
    let site_description = params.site_description.clone();

    let site_info = SiteInfo {
        site_name,
        site_description,
    };

    // Step 1: Request authorization codes
    let auth_response = client.authorize(site_info).await?;

    // Step 2: Display instructions to user
    DeviceFlowClient::display_instructions(&auth_response);

    // Step 3: Poll for credentials
    println!("Waiting for user approval...");
    let credentials = loop {
        tokio::time::sleep(std::time::Duration::from_secs(auth_response.interval)).await;

        match client.poll_for_token(&auth_response.device_code).await? {
            Some(creds) => break creds,
            None => {
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).ok();
                continue;
            }
        }
    };

    println!("\n\n✓ Device authorized successfully!");
    println!("  Site ID: {}", credentials.site_id);
    println!("  Domain: {}", credentials.domain);

    // Create configuration with device credentials
    let config = create_config_device_flow(&params, credentials)?;

    // Validate credentials by requesting token
    info!("Validating credentials...");
    validate_credentials(&config).await?;

    // Install service (platform-specific)
    install_service(&config)?;

    info!("Installation completed successfully");
    Ok(())
}

/// Create configuration from install parameters (traditional auth)
fn create_config_traditional(params: &InstallParams) -> Result<Config> {
    use crate::models::config::{
        ApiConfig, CredentialConfig, EncodingConfig, SchedulerConfig, SourceConfig,
    };

    let account = params.account.clone().ok_or_else(|| {
        ProcessingError::ConfigurationError("Account required for traditional auth".to_string())
    })?;
    let username = params.username.clone().ok_or_else(|| {
        ProcessingError::ConfigurationError("Username required for traditional auth".to_string())
    })?;
    let password = params.password.clone().ok_or_else(|| {
        ProcessingError::ConfigurationError("Password required for traditional auth".to_string())
    })?;

    let config = Config {
        scheduler: SchedulerConfig {
            crontab: params.crontab.clone(),
        },
        src: SourceConfig {
            source_dir: PathBuf::from(&params.source_dir),
            include_patterns: None,
            exclude_patterns: None,
        },
        credential: CredentialConfig {
            account,
            username,
            password,
            device: None,
        },
        api: ApiConfig {
            base_url: params.api_url.clone(),
            https_only: params.https_only,
        },
        encoding: EncodingConfig {
            dbf_encoding: params.encoding.clone(),
        },
    };

    Ok(config)
}

/// Create configuration from install parameters (device flow)
fn create_config_device_flow(
    params: &InstallParams,
    credentials: crate::auth::device_flow::DeviceCredentials,
) -> Result<Config> {
    use crate::models::config::{
        ApiConfig, CredentialConfig, DeviceCredentials, EncodingConfig, SchedulerConfig,
        SourceConfig,
    };

    let config = Config {
        scheduler: SchedulerConfig {
            crontab: params.crontab.clone(),
        },
        src: SourceConfig {
            source_dir: PathBuf::from(&params.source_dir),
            include_patterns: None,
            exclude_patterns: None,
        },
        credential: CredentialConfig {
            account: String::new(),
            username: String::new(),
            password: String::new(),
            device: Some(DeviceCredentials {
                site_id: credentials.site_id,
                domain: credentials.domain,
                client_secret: credentials.client_secret,
            }),
        },
        api: ApiConfig {
            base_url: credentials.api_base_url,
            https_only: params.https_only,
        },
        encoding: EncodingConfig {
            dbf_encoding: params.encoding.clone(),
        },
    };

    Ok(config)
}

/// Validate credentials by attempting to get a token
async fn validate_credentials(config: &Config) -> Result<()> {
    let auth_client = AuthClient::new(config)?;

    match auth_client.get_token().await {
        Ok(_token) => {
            info!("Credentials validated");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Install the service (platform-specific)
#[cfg(target_os = "windows")]
fn install_service(config: &Config) -> Result<()> {
    use std::fs;

    info!("Registering Windows service...");

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

    info!("Service registered successfully");
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
/// Restricts access to SYSTEM and Administrators only
#[cfg(target_os = "windows")]
fn set_config_permissions(config_path: &Path) -> Result<()> {
    use std::process::Command;

    info!("Setting ACL permissions on config file...");

    let config_path_str = config_path.to_string_lossy().to_string();

    // Remove inheritance and copy existing permissions
    let disable_inheritance = Command::new("icacls")
        .args([&config_path_str, "/inheritance:d"])
        .output()
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to run icacls: {}", e))
        })?;

    if !disable_inheritance.status.success() {
        return Err(ProcessingError::ConfigurationError(format!(
            "Failed to disable inheritance: {}",
            String::from_utf8_lossy(&disable_inheritance.stderr)
        )));
    }

    // Remove all existing permissions for Users and Everyone
    let _ = Command::new("icacls")
        .args([&config_path_str, "/remove:g", "Users"])
        .output();
    let _ = Command::new("icacls")
        .args([&config_path_str, "/remove:g", "Everyone"])
        .output();

    // Grant full control to SYSTEM
    let grant_system = Command::new("icacls")
        .args([&config_path_str, "/grant", "SYSTEM:(F)"])
        .output()
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to run icacls: {}", e))
        })?;

    if !grant_system.status.success() {
        return Err(ProcessingError::ConfigurationError(format!(
            "Failed to grant SYSTEM permissions: {}",
            String::from_utf8_lossy(&grant_system.stderr)
        )));
    }

    // Grant full control to Administrators
    let grant_admins = Command::new("icacls")
        .args([&config_path_str, "/grant", "Administrators:(F)"])
        .output()
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to run icacls: {}", e))
        })?;

    if !grant_admins.status.success() {
        return Err(ProcessingError::ConfigurationError(format!(
            "Failed to grant Administrators permissions: {}",
            String::from_utf8_lossy(&grant_admins.stderr)
        )));
    }

    info!("ACL permissions set successfully - config file is now protected");
    Ok(())
}

/// Register Windows service
#[cfg(target_os = "windows")]
fn register_windows_service(exe_path: &Path) -> Result<()> {
    use std::process::Command;

    let service_name = "data-exporter";
    let display_name = "Data Exporter Service";
    let description =
        "Automatically exports DBF files to CSV, compresses to gzip, and uploads to cloud server";

    // Use sc.exe to create the service
    // Note: sc.exe requires VERY specific syntax:
    // - Exactly one space after =
    // - Path should be without quotes for sc.exe (it adds them internally if needed)
    let exe_path_str = exe_path.to_string_lossy().to_string();
    let bin_path = format!("binPath={}", exe_path_str); // No space after = to avoid ERROR 87

    let output = Command::new("sc.exe")
        .args([
            "create",
            service_name,
            &bin_path,
            &format!("DisplayName={}", display_name),
            "start=auto", // Changed from 'demand' to 'auto' for automatic startup
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
        // Non-critical, just ignore
    }

    // Start the service immediately
    info!("Starting service...");
    let start_output = Command::new("sc.exe")
        .args(["start", service_name])
        .output()
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to start service: {}", e))
        })?;

    if !start_output.status.success() {
        let stderr = String::from_utf8_lossy(&start_output.stderr);
        let stdout = String::from_utf8_lossy(&start_output.stdout);
        return Err(ProcessingError::ConfigurationError(format!(
            "Failed to start service: stdout={}, stderr={}",
            stdout, stderr
        )));
    }

    info!("Service started successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_config() {
        let params = InstallParams {
            use_device_flow: false,
            site_name: None,
            site_description: None,
            account: Some("testaccount".to_string()),
            username: Some("testuser".to_string()),
            password: Some("testpass".to_string()),
            source_dir: "/tmp".to_string(),
            crontab: "*/5 * * * *".to_string(),
            api_url: "https://api.example.com".to_string(),
            encoding: "CP866".to_string(),
            https_only: true,
        };

        let config = create_config_traditional(&params);

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
        let params = InstallParams {
            use_device_flow: false,
            site_name: None,
            site_description: None,
            account: Some("testaccount".to_string()),
            username: Some("test".to_string()),
            password: Some("test".to_string()),
            source_dir: "/tmp".to_string(),
            crontab: "*/5 * * * *".to_string(),
            api_url: "https://api.example.com".to_string(),
            encoding: "CP866".to_string(),
            https_only: true,
        };
        let result = create_config_traditional(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_allowed_when_https_only_disabled() {
        let params = InstallParams {
            use_device_flow: false,
            site_name: None,
            site_description: None,
            account: Some("testaccount".to_string()),
            username: Some("test".to_string()),
            password: Some("test".to_string()),
            source_dir: "/tmp".to_string(),
            crontab: "*/5 * * * *".to_string(),
            api_url: "http://localhost:8080".to_string(),
            encoding: "CP866".to_string(),
            https_only: false,
        };
        let result = create_config_traditional(&params);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.api.https_only);
        assert_eq!(config.api.base_url, "http://localhost:8080");
    }
}
