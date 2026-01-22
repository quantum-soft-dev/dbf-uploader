// Configuration model for data_exporter
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub scheduler: SchedulerConfig,
    pub src: SourceConfig,
    pub credential: CredentialConfig,
    pub api: ApiConfig,
    pub encoding: EncodingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchedulerConfig {
    /// Cron expression for scheduling batch operations
    /// Example: "0 8,12,16,18 * * *" for 8am, 12pm, 4pm, 6pm daily
    pub crontab: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SourceConfig {
    /// Directory containing DBF files to process
    pub source_dir: PathBuf,

    /// Whitelist patterns: only process files matching these patterns (glob syntax)
    /// Example: ["*.dbf", "data_*.DBF"]
    /// If not specified or empty, all DBF files are included
    #[serde(default)]
    pub include_patterns: Option<Vec<String>>,

    /// Blacklist patterns: exclude files matching these patterns (glob syntax)
    /// Example: ["temp_*.dbf", "*.bak", "nsfcli.DBF"]
    /// Exclude patterns are applied after include patterns
    #[serde(default)]
    pub exclude_patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialConfig {
    /// Account identifier (combined with username for uniqueness)
    /// Optional when using device flow credentials
    #[serde(default)]
    pub account: String,
    /// Username for API authentication
    /// Optional when using device flow credentials
    #[serde(default)]
    pub username: String,
    /// Password for API authentication
    /// Optional when using device flow credentials
    #[serde(default)]
    pub password: String,
    /// Device flow credentials (domain + client_secret)
    /// Obtained via Device Authorization Grant (RFC 8628)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<DeviceCredentials>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceCredentials {
    /// UUID of the site
    pub site_id: String,
    /// Domain for the site (username for Basic Auth)
    pub domain: String,
    /// Client secret for authentication (password for Basic Auth)
    pub client_secret: String,
}

impl CredentialConfig {
    /// Get the full username in format: account_username or domain from device credentials
    pub fn full_username(&self) -> String {
        if let Some(ref device) = self.device {
            device.domain.clone()
        } else {
            format!("{}_{}", self.account, self.username)
        }
    }

    /// Get the password or client_secret from device credentials
    pub fn password(&self) -> &str {
        if let Some(ref device) = self.device {
            &device.client_secret
        } else {
            &self.password
        }
    }

    /// Check if using device flow credentials
    pub fn is_device_flow(&self) -> bool {
        self.device.is_some()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    /// Base URL for API server (must start with https:// if https_only is true)
    pub base_url: String,
    /// Enforce HTTPS-only connections (default: true)
    /// WARNING: Setting this to false is insecure and should only be used for local testing
    #[serde(default = "default_https_only")]
    pub https_only: bool,
}

fn default_https_only() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EncodingConfig {
    /// Fallback encoding for DBF files without encoding in header
    /// Default: CP866
    #[serde(default = "default_dbf_encoding")]
    pub dbf_encoding: String,
}

fn default_dbf_encoding() -> String {
    "CP866".to_string()
}

impl Config {
    /// Load configuration from TOML file
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn to_file(&self, path: &std::path::Path) -> Result<(), crate::error::ProcessingError> {
        let toml_string = toml::to_string_pretty(self).map_err(|e| {
            crate::error::ProcessingError::ConfigurationError(format!(
                "Failed to serialize config: {}",
                e
            ))
        })?;

        std::fs::write(path, toml_string).map_err(crate::error::ProcessingError::FileReadError)?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate crontab expression can be parsed
        // Note: Actual parsing will be done by tokio-cron-scheduler
        if self.scheduler.crontab.is_empty() {
            return Err("Crontab expression cannot be empty".into());
        }

        // Validate source directory exists
        if !self.src.source_dir.exists() {
            return Err(
                format!("Source directory does not exist: {:?}", self.src.source_dir).into(),
            );
        }

        // Validate API base URL starts with https:// if https_only is true
        if self.api.https_only && !self.api.base_url.starts_with("https://") {
            return Err("API base URL must start with https:// when https_only is enabled".into());
        }

        // Validate API base URL has valid protocol
        if !self.api.base_url.starts_with("http://") && !self.api.base_url.starts_with("https://") {
            return Err("API base URL must start with http:// or https://".into());
        }

        // Validate credentials - either traditional or device flow must be present
        if self.credential.device.is_none() {
            // Traditional credentials validation
            if self.credential.account.is_empty() {
                return Err("Account cannot be empty when not using device flow".into());
            }
            if self.credential.username.is_empty() {
                return Err("Username cannot be empty when not using device flow".into());
            }
            if self.credential.password.is_empty() {
                return Err("Password cannot be empty when not using device flow".into());
            }
        } else {
            // Device flow credentials validation
            let device = self.credential.device.as_ref().unwrap();
            if device.domain.is_empty() {
                return Err("Device domain cannot be empty".into());
            }
            if device.client_secret.is_empty() {
                return Err("Device client_secret cannot be empty".into());
            }
        }

        // Validate include/exclude patterns are valid glob patterns
        if let Some(ref patterns) = self.src.include_patterns {
            for pattern in patterns {
                globset::Glob::new(pattern)
                    .map_err(|e| format!("Invalid include pattern '{}': {}", pattern, e))?;
            }
        }

        if let Some(ref patterns) = self.src.exclude_patterns {
            for pattern in patterns {
                globset::Glob::new(pattern)
                    .map_err(|e| format!("Invalid exclude pattern '{}': {}", pattern, e))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_config_from_toml() {
        // Create temporary directory that actually exists
        let temp_dir = TempDir::new().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap();

        let toml_content = format!(
            r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "{}"

[credential]
account = "test_account"
username = "test_user"
password = "test_password"

[api]
base_url = "https://api.example.com"
https_only = true

[encoding]
dbf_encoding = "CP866"
        "#,
            temp_dir_path.replace('\\', "\\\\")
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = Config::from_file(temp_file.path()).unwrap();

        assert_eq!(config.scheduler.crontab, "*/5 * * * *");
        assert_eq!(config.src.source_dir, temp_dir.path());
        assert_eq!(config.credential.username, "test_user");
        assert_eq!(config.credential.password, "test_password");
        assert_eq!(config.api.base_url, "https://api.example.com");
        assert_eq!(config.encoding.dbf_encoding, "CP866");
    }

    #[test]
    fn test_config_validation_invalid_url() {
        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap();

        let toml_content = format!(
            r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "{}"

[credential]
account = "test_account"
username = "test_user"
password = "test_password"

[api]
base_url = "http://api.example.com"
https_only = true

[encoding]
dbf_encoding = "CP866"
        "#,
            temp_dir_path.replace('\\', "\\\\")
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = Config::from_file(temp_file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("https://"));
    }

    #[test]
    fn test_config_http_allowed_when_https_only_disabled() {
        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap();

        let toml_content = format!(
            r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "{}"

[credential]
account = "test_account"
username = "test_user"
password = "test_password"

[api]
base_url = "http://localhost:8080"
https_only = false

[encoding]
dbf_encoding = "CP866"
        "#,
            temp_dir_path.replace('\\', "\\\\")
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = Config::from_file(temp_file.path()).unwrap();
        assert_eq!(config.api.base_url, "http://localhost:8080");
        assert!(!config.api.https_only);
    }

    #[test]
    fn test_config_default_encoding() {
        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap();

        let toml_content = format!(
            r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "{}"

[credential]
account = "test_account"
username = "test_user"
password = "test_password"

[api]
base_url = "https://api.example.com"

[encoding]
        "#,
            temp_dir_path.replace('\\', "\\\\")
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = Config::from_file(temp_file.path()).unwrap();
        assert_eq!(config.encoding.dbf_encoding, "CP866");
    }
}
