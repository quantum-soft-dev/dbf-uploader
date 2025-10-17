// Configuration V2 for middleware batch protocol compatibility
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration structure for DBF Uploader v2.0
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigV2 {
    /// Authentication configuration
    pub auth: AuthConfigV2,
    /// API endpoint configuration
    pub api: ApiConfigV2,
    /// Source directory configuration
    pub source: SourceConfig,
    /// Scheduling configuration
    pub schedule: ScheduleConfig,
    /// Encoding configuration
    pub encoding: EncodingConfig,
    /// Batch processing configuration
    pub batch: BatchConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Authentication configuration for site credentials
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfigV2 {
    /// Site domain (e.g., "store-01.example.com")
    pub domain: String,
    /// Client secret UUID from middleware admin API
    pub client_secret: String,
}

/// API endpoint configuration for middleware v2 protocol
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfigV2 {
    /// Base URL for middleware API (must start with https://)
    pub base_url: String,
    /// Authentication endpoint path (default: "/api/v1/auth/token")
    #[serde(default = "default_auth_endpoint")]
    pub auth_endpoint: String,
    /// Batch start endpoint path (default: "/api/v1/batch/start")
    #[serde(default = "default_batch_start")]
    pub batch_start: String,
    /// Batch upload endpoint path template (default: "/api/v1/batch/{batchId}/upload")
    #[serde(default = "default_batch_upload")]
    pub batch_upload: String,
    /// Batch complete endpoint path template (default: "/api/v1/batch/{batchId}/complete")
    #[serde(default = "default_batch_complete")]
    pub batch_complete: String,
    /// Batch fail endpoint path template (default: "/api/v1/batch/{batchId}/fail")
    #[serde(default = "default_batch_fail")]
    pub batch_fail: String,
    /// Batch cancel endpoint path template (default: "/api/v1/batch/{batchId}/cancel")
    #[serde(default = "default_batch_cancel")]
    pub batch_cancel: String,
    /// Error logging endpoint path (default: "/api/v1/error")
    #[serde(default = "default_error_log")]
    pub error_log: String,
}

/// Source directory configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SourceConfig {
    /// Directory containing DBF files to process
    pub directory: PathBuf,
}

/// Scheduling configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScheduleConfig {
    /// Cron expression for scheduling batch operations
    /// Example: "0 8,12,16,18 * * *" for 8am, 12pm, 4pm, 6pm daily
    pub cron: String,
}

/// Encoding configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EncodingConfig {
    /// Fallback encoding for DBF files without encoding in header
    /// Default: CP866
    #[serde(default = "default_dbf_encoding")]
    pub fallback: String,
}

/// Batch processing configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BatchConfig {
    /// Maximum files per batch (middleware limit: 1000, recommended: 500)
    #[serde(default = "default_max_files_per_batch")]
    pub max_files_per_batch: usize,
    /// Retry failed files at end of batch
    #[serde(default = "default_retry_locked_files")]
    pub retry_locked_files: bool,
    /// Batch timeout duration in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_batch_timeout_secs")]
    #[serde(rename = "batch_timeout_secs")]
    pub batch_timeout: u64,
    /// Maximum retry attempts for failed operations (default: 3)
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// HTTP request timeout in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_http_timeout_secs")]
    pub http_timeout_secs: u64,
    /// Delay in seconds before retrying locked files (default: 5)
    #[serde(default = "default_locked_file_retry_delay_secs")]
    pub locked_file_retry_delay_secs: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Local error log file path for fallback
    pub error_log_path: PathBuf,
}

// Default value functions for serde
fn default_auth_endpoint() -> String {
    "/api/v1/auth/token".to_string()
}

fn default_batch_start() -> String {
    "/api/v1/batch/start".to_string()
}

fn default_batch_upload() -> String {
    "/api/v1/batch/{batchId}/upload".to_string()
}

fn default_batch_complete() -> String {
    "/api/v1/batch/{batchId}/complete".to_string()
}

fn default_batch_fail() -> String {
    "/api/v1/batch/{batchId}/fail".to_string()
}

fn default_batch_cancel() -> String {
    "/api/v1/batch/{batchId}/cancel".to_string()
}

fn default_error_log() -> String {
    "/api/v1/error".to_string()
}

fn default_dbf_encoding() -> String {
    "CP866".to_string()
}

fn default_max_files_per_batch() -> usize {
    500
}

fn default_retry_locked_files() -> bool {
    true
}

fn default_batch_timeout_secs() -> u64 {
    3600 // 1 hour
}

fn default_max_retries() -> u32 {
    3
}

fn default_http_timeout_secs() -> u64 {
    300 // 5 minutes
}

fn default_locked_file_retry_delay_secs() -> u64 {
    5 // 5 seconds
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_files_per_batch: default_max_files_per_batch(),
            retry_locked_files: default_retry_locked_files(),
            batch_timeout: default_batch_timeout_secs(),
            max_retries: default_max_retries(),
            http_timeout_secs: default_http_timeout_secs(),
            locked_file_retry_delay_secs: default_locked_file_retry_delay_secs(),
        }
    }
}

impl ConfigV2 {
    /// Load configuration from TOML file
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: ConfigV2 = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }

    /// Get batch timeout as Duration
    pub fn batch_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.batch.batch_timeout)
    }
}

impl Default for ApiConfigV2 {
    fn default() -> Self {
        Self {
            base_url: "https://api.example.com".to_string(),
            auth_endpoint: default_auth_endpoint(),
            batch_start: default_batch_start(),
            batch_upload: default_batch_upload(),
            batch_complete: default_batch_complete(),
            batch_fail: default_batch_fail(),
            batch_cancel: default_batch_cancel(),
            error_log: default_error_log(),
        }
    }
}

impl Default for ConfigV2 {
    fn default() -> Self {
        Self {
            auth: AuthConfigV2 {
                domain: "REPLACE_WITH_YOUR_SITE_DOMAIN".to_string(),
                client_secret: "REPLACE_WITH_CLIENT_SECRET_UUID".to_string(),
            },
            api: ApiConfigV2::default(),
            source: SourceConfig {
                directory: PathBuf::from("C:\\Program Files\\abc\\data"),
            },
            schedule: ScheduleConfig {
                cron: "0 8,12,16,18 * * *".to_string(),
            },
            encoding: EncodingConfig {
                fallback: default_dbf_encoding(),
            },
            batch: BatchConfig::default(),
            logging: LoggingConfig {
                error_log_path: PathBuf::from("C:\\Program Files\\dbf-uploader\\error.log"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_v2_default() {
        let config = ConfigV2::default();

        assert_eq!(config.auth.domain, "REPLACE_WITH_YOUR_SITE_DOMAIN");
        assert_eq!(config.auth.client_secret, "REPLACE_WITH_CLIENT_SECRET_UUID");
        assert_eq!(config.api.base_url, "https://api.example.com");
        assert_eq!(config.api.auth_endpoint, "/api/v1/auth/token");
        assert_eq!(config.api.batch_start, "/api/v1/batch/start");
        assert_eq!(config.batch.max_files_per_batch, 500);
        assert_eq!(config.batch.retry_locked_files, true);
        assert_eq!(config.batch.batch_timeout, 3600);
        assert_eq!(config.batch.max_retries, 3);
        assert_eq!(config.encoding.fallback, "CP866");
    }

    #[test]
    fn test_config_v2_from_toml() {
        let toml_content = r#"
[auth]
domain = "store-01.example.com"
client_secret = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"

[api]
base_url = "https://api.example.com"

[source]
directory = "/tmp/data"

[schedule]
cron = "*/5 * * * *"

[encoding]
fallback = "CP866"

[batch]
max_files_per_batch = 500
retry_locked_files = true
batch_timeout_secs = 3600
max_retries = 3

[logging]
error_log_path = "/tmp/error.log"
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = ConfigV2::from_file(temp_file.path()).unwrap();

        assert_eq!(config.auth.domain, "store-01.example.com");
        assert_eq!(config.auth.client_secret, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert_eq!(config.api.base_url, "https://api.example.com");
        assert_eq!(config.api.auth_endpoint, "/api/v1/auth/token");
        assert_eq!(config.source.directory, PathBuf::from("/tmp/data"));
        assert_eq!(config.schedule.cron, "*/5 * * * *");
        assert_eq!(config.encoding.fallback, "CP866");
        assert_eq!(config.batch.max_files_per_batch, 500);
        assert_eq!(config.batch.retry_locked_files, true);
        assert_eq!(config.batch.batch_timeout, 3600);
        assert_eq!(config.batch.max_retries, 3);
        assert_eq!(config.logging.error_log_path, PathBuf::from("/tmp/error.log"));
    }

    #[test]
    fn test_config_v2_default_endpoints() {
        let toml_content = r#"
[auth]
domain = "store-01.example.com"
client_secret = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"

[api]
base_url = "https://api.example.com"

[source]
directory = "/tmp/data"

[schedule]
cron = "*/5 * * * *"

[encoding]

[batch]

[logging]
error_log_path = "/tmp/error.log"
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = ConfigV2::from_file(temp_file.path()).unwrap();

        // Test that default endpoints are applied
        assert_eq!(config.api.auth_endpoint, "/api/v1/auth/token");
        assert_eq!(config.api.batch_start, "/api/v1/batch/start");
        assert_eq!(config.api.batch_upload, "/api/v1/batch/{batchId}/upload");
        assert_eq!(config.api.batch_complete, "/api/v1/batch/{batchId}/complete");
        assert_eq!(config.api.batch_fail, "/api/v1/batch/{batchId}/fail");
        assert_eq!(config.api.batch_cancel, "/api/v1/batch/{batchId}/cancel");
        assert_eq!(config.api.error_log, "/api/v1/error");

        // Test default batch settings
        assert_eq!(config.batch.max_files_per_batch, 500);
        assert_eq!(config.batch.retry_locked_files, true);
        assert_eq!(config.batch.batch_timeout, 3600);
        assert_eq!(config.batch.max_retries, 3);

        // Test default encoding
        assert_eq!(config.encoding.fallback, "CP866");
    }

    #[test]
    fn test_config_v2_to_file() {
        let config = ConfigV2 {
            auth: AuthConfigV2 {
                domain: "test.example.com".to_string(),
                client_secret: "test-uuid-123".to_string(),
            },
            api: ApiConfigV2 {
                base_url: "https://test.api.com".to_string(),
                auth_endpoint: "/api/v1/auth/token".to_string(),
                batch_start: "/api/v1/batch/start".to_string(),
                batch_upload: "/api/v1/batch/{batchId}/upload".to_string(),
                batch_complete: "/api/v1/batch/{batchId}/complete".to_string(),
                batch_fail: "/api/v1/batch/{batchId}/fail".to_string(),
                batch_cancel: "/api/v1/batch/{batchId}/cancel".to_string(),
                error_log: "/api/v1/error".to_string(),
            },
            source: SourceConfig {
                directory: PathBuf::from("/tmp/test"),
            },
            schedule: ScheduleConfig {
                cron: "*/10 * * * *".to_string(),
            },
            encoding: EncodingConfig {
                fallback: "UTF-8".to_string(),
            },
            batch: BatchConfig {
                max_files_per_batch: 100,
                retry_locked_files: false,
                batch_timeout: 1800,
                max_retries: 5,
                http_timeout_secs: 300,
                locked_file_retry_delay_secs: 5,
            },
            logging: LoggingConfig {
                error_log_path: PathBuf::from("/tmp/test_error.log"),
            },
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.to_file(temp_file.path()).unwrap();

        // Read back and verify
        let loaded_config = ConfigV2::from_file(temp_file.path()).unwrap();
        assert_eq!(loaded_config.auth.domain, "test.example.com");
        assert_eq!(loaded_config.batch.max_files_per_batch, 100);
        assert_eq!(loaded_config.batch.batch_timeout, 1800);
        assert_eq!(loaded_config.batch.max_retries, 5);
    }

    #[test]
    fn test_batch_timeout_duration() {
        let config = ConfigV2 {
            batch: BatchConfig {
                batch_timeout: 7200,
                ..Default::default()
            },
            ..Default::default()
        };

        let duration = config.batch_timeout_duration();
        assert_eq!(duration, Duration::from_secs(7200));
    }
}
