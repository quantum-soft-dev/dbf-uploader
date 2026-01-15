// Device authorization command implementation
use crate::auth::device_flow::{DeviceFlowClient, SiteInfo};
use crate::error::{ProcessingError, Result};
use crate::models::config::{Config, DeviceCredentials};
use std::path::PathBuf;

/// Parameters for device authorization
pub struct AuthorizeDeviceParams {
    pub site_name: String,
    pub site_description: Option<String>,
    pub api_url: Option<String>,
}

/// Re-authorize device to connect to a different site
pub async fn authorize_device(params: AuthorizeDeviceParams) -> Result<()> {
    println!("Re-authorizing device with new site...\n");

    // Get config directory path
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join("config.toml");

    // Read existing config to get api_url and https_only
    let existing_config = if config_path.exists() {
        Some(
            Config::from_file(&config_path).map_err(|e| {
                ProcessingError::ConfigurationError(format!("Failed to read config: {}", e))
            })?,
        )
    } else {
        None
    };

    // Use provided api_url or existing one, default to production
    let api_url = params
        .api_url
        .or_else(|| existing_config.as_ref().map(|c| c.api.base_url.clone()))
        .unwrap_or_else(|| "https://dev.dfm.bitbi.io".to_string());

    let https_only = existing_config
        .as_ref()
        .map(|c| c.api.https_only)
        .unwrap_or(true);

    // Create device flow client
    let client = DeviceFlowClient::new(api_url.clone(), https_only)?;

    // Create site information
    let site_info = SiteInfo {
        site_name: params.site_name.clone(),
        site_description: params.site_description.clone(),
    };

    // Step 1: Request device codes
    println!("Requesting device authorization codes...");
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

    println!("\n\n✓ Device re-authorized successfully!");
    println!("  Site ID: {}", credentials.site_id);
    println!("  Domain: {}", credentials.domain);

    // Update configuration with new device credentials
    let mut config = existing_config.ok_or_else(|| {
        ProcessingError::ConfigurationError(
            "No existing configuration found. Please run 'install' command first.".to_string(),
        )
    })?;

    // Update ONLY the device credentials section
    config.credential.device = Some(DeviceCredentials {
        site_id: credentials.site_id.clone(),
        domain: credentials.domain.clone(),
        client_secret: credentials.client_secret,
    });

    // Update API base URL if it changed (from credentials response)
    config.api.base_url = credentials.api_base_url;

    // Save updated configuration
    config.to_file(&config_path)?;

    println!("\nConfiguration updated: {}", config_path.display());
    println!("\nThe service will automatically use the new credentials.");
    println!("No need to restart the service - config hot-reload is enabled.");

    Ok(())
}

/// Get configuration directory path
fn get_config_dir() -> Result<PathBuf> {
    // On Windows: C:\ProgramData\data_exporter or C:\Program Files\data-exporter
    #[cfg(windows)]
    {
        // Try C:\Program Files\data-exporter first (where service is installed)
        let install_dir = PathBuf::from(r"C:\Program Files\data-exporter");
        if install_dir.exists() {
            return Ok(install_dir);
        }

        // Fallback to ProgramData
        let program_data =
            std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
        Ok(PathBuf::from(program_data).join("data_exporter"))
    }

    // On Unix: /etc/data_exporter
    #[cfg(not(windows))]
    {
        Ok(PathBuf::from("/etc/data_exporter"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_dir() {
        let config_dir = get_config_dir().unwrap();
        assert!(config_dir.to_string_lossy().contains("data"));
    }
}
