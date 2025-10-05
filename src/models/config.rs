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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialConfig {
    /// Username for API authentication
    pub username: String,
    /// Password for API authentication
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    /// Base URL for API server (must start with https://)
    pub base_url: String,
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

    /// Validate configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate crontab expression can be parsed
        // Note: Actual parsing will be done by tokio-cron-scheduler
        if self.scheduler.crontab.is_empty() {
            return Err("Crontab expression cannot be empty".into());
        }

        // Validate source directory exists
        if !self.src.source_dir.exists() {
            return Err(format!("Source directory does not exist: {:?}", self.src.source_dir).into());
        }

        // Validate API base URL starts with https://
        if !self.api.base_url.starts_with("https://") {
            return Err("API base URL must start with https://".into());
        }

        // Validate credentials are not empty
        if self.credential.username.is_empty() {
            return Err("Username cannot be empty".into());
        }
        if self.credential.password.is_empty() {
            return Err("Password cannot be empty".into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_from_toml() {
        let toml_content = r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "/tmp"

[credential]
username = "test_user"
password = "test_password"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP866"
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = Config::from_file(temp_file.path()).unwrap();

        assert_eq!(config.scheduler.crontab, "*/5 * * * *");
        assert_eq!(config.src.source_dir, PathBuf::from("/tmp"));
        assert_eq!(config.credential.username, "test_user");
        assert_eq!(config.credential.password, "test_password");
        assert_eq!(config.api.base_url, "https://api.example.com");
        assert_eq!(config.encoding.dbf_encoding, "CP866");
    }

    #[test]
    fn test_config_validation_invalid_url() {
        let toml_content = r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "/tmp"

[credential]
username = "test_user"
password = "test_password"

[api]
base_url = "http://api.example.com"

[encoding]
dbf_encoding = "CP866"
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = Config::from_file(temp_file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("https://"));
    }

    #[test]
    fn test_config_default_encoding() {
        let toml_content = r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "/tmp"

[credential]
username = "test_user"
password = "test_password"

[api]
base_url = "https://api.example.com"

[encoding]
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = Config::from_file(temp_file.path()).unwrap();
        assert_eq!(config.encoding.dbf_encoding, "CP866");
    }
}
