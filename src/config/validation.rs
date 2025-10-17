// Configuration validation module
use super::v2::ConfigV2;
use std::path::Path;
use uuid::Uuid;

/// Validation errors for configuration
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Invalid domain format
    InvalidDomain(String),
    /// Invalid client secret (not a UUID)
    InvalidClientSecret(String),
    /// Source directory does not exist
    SourceDirectoryNotFound(String),
    /// Invalid cron expression
    InvalidCron(String),
    /// Invalid batch size (must be 1-1000)
    InvalidBatchSize(usize),
    /// Invalid base URL format
    InvalidBaseUrl(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidDomain(msg) => write!(f, "Invalid domain: {}", msg),
            ValidationError::InvalidClientSecret(msg) => {
                write!(f, "Invalid client secret: {}", msg)
            }
            ValidationError::SourceDirectoryNotFound(msg) => {
                write!(f, "Source directory not found: {}", msg)
            }
            ValidationError::InvalidCron(msg) => write!(f, "Invalid cron expression: {}", msg),
            ValidationError::InvalidBatchSize(size) => {
                write!(f, "Invalid batch size: {} (must be 1-1000)", size)
            }
            ValidationError::InvalidBaseUrl(msg) => write!(f, "Invalid base URL: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

impl ConfigV2 {
    /// Validate the configuration
    /// Returns Ok(()) if configuration is valid, Err with details if not
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Validate domain format (DNS-compliant)
        if let Err(e) = validate_domain(&self.auth.domain) {
            errors.push(e);
        }

        // Validate client_secret as UUID
        if let Err(e) = validate_uuid(&self.auth.client_secret) {
            errors.push(e);
        }

        // Validate base URL starts with https://
        if let Err(e) = validate_base_url(&self.api.base_url) {
            errors.push(e);
        }

        // Validate source directory exists
        if let Err(e) = validate_directory_exists(&self.source.directory) {
            errors.push(e);
        }

        // Validate cron expression format
        if let Err(e) = validate_cron(&self.schedule.cron) {
            errors.push(e);
        }

        // Validate max_files_per_batch range (1-1000)
        if let Err(e) = validate_batch_size(self.batch.max_files_per_batch) {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validate domain format (DNS-compliant)
fn validate_domain(domain: &str) -> Result<(), ValidationError> {
    // Domain cannot be empty
    if domain.is_empty() {
        return Err(ValidationError::InvalidDomain(
            "Domain cannot be empty".to_string(),
        ));
    }

    // Domain must not contain spaces
    if domain.contains(' ') {
        return Err(ValidationError::InvalidDomain(
            "Domain cannot contain spaces".to_string(),
        ));
    }

    // Domain should contain at least one dot for FQDN
    if !domain.contains('.') {
        return Err(ValidationError::InvalidDomain(
            "Domain must be a fully qualified domain name (e.g., store-01.example.com)"
                .to_string(),
        ));
    }

    // Domain must not start or end with a dot or hyphen
    if domain.starts_with('.') || domain.ends_with('.') {
        return Err(ValidationError::InvalidDomain(
            "Domain cannot start or end with a dot".to_string(),
        ));
    }

    if domain.starts_with('-') || domain.ends_with('-') {
        return Err(ValidationError::InvalidDomain(
            "Domain cannot start or end with a hyphen".to_string(),
        ));
    }

    // Check that domain parts are valid
    for part in domain.split('.') {
        if part.is_empty() {
            return Err(ValidationError::InvalidDomain(
                "Domain contains empty labels".to_string(),
            ));
        }

        // Each label must not start or end with a hyphen
        if part.starts_with('-') || part.ends_with('-') {
            return Err(ValidationError::InvalidDomain(format!(
                "Domain label cannot start or end with hyphen: {}",
                part
            )));
        }

        // Each label must contain only alphanumeric and hyphens
        if !part.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(ValidationError::InvalidDomain(format!(
                "Invalid characters in domain label: {}",
                part
            )));
        }
    }

    Ok(())
}

/// Validate client_secret as UUID
fn validate_uuid(client_secret: &str) -> Result<(), ValidationError> {
    Uuid::parse_str(client_secret).map_err(|_| {
        ValidationError::InvalidClientSecret(
            "Client secret must be a valid UUID (e.g., a1b2c3d4-e5f6-7890-abcd-ef1234567890)"
                .to_string(),
        )
    })?;

    Ok(())
}

/// Validate base URL format
fn validate_base_url(base_url: &str) -> Result<(), ValidationError> {
    if !base_url.starts_with("https://") {
        return Err(ValidationError::InvalidBaseUrl(
            "Base URL must start with https://".to_string(),
        ));
    }

    if base_url.len() < 10 {
        // "https://x" is minimum valid URL
        return Err(ValidationError::InvalidBaseUrl(
            "Base URL is too short".to_string(),
        ));
    }

    Ok(())
}

/// Validate directory exists
fn validate_directory_exists(path: &Path) -> Result<(), ValidationError> {
    if !path.exists() {
        return Err(ValidationError::SourceDirectoryNotFound(format!(
            "Directory does not exist: {:?}",
            path
        )));
    }

    if !path.is_dir() {
        return Err(ValidationError::SourceDirectoryNotFound(format!(
            "Path is not a directory: {:?}",
            path
        )));
    }

    Ok(())
}

/// Validate cron expression format
fn validate_cron(cron_expr: &str) -> Result<(), ValidationError> {
    if cron_expr.is_empty() {
        return Err(ValidationError::InvalidCron(
            "Cron expression cannot be empty".to_string(),
        ));
    }

    // Basic validation: cron should have 5 or 6 parts separated by spaces
    let parts: Vec<&str> = cron_expr.split_whitespace().collect();
    if parts.len() < 5 || parts.len() > 6 {
        return Err(ValidationError::InvalidCron(format!(
            "Cron expression must have 5 or 6 fields, got {}",
            parts.len()
        )));
    }

    // Note: More detailed cron parsing will be done by tokio-cron-scheduler at runtime
    // This is just a basic sanity check

    Ok(())
}

/// Validate batch size is within acceptable range
fn validate_batch_size(size: usize) -> Result<(), ValidationError> {
    if size == 0 {
        return Err(ValidationError::InvalidBatchSize(size));
    }

    if size > 1000 {
        return Err(ValidationError::InvalidBatchSize(size));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_validate_domain_valid() {
        assert!(validate_domain("store-01.example.com").is_ok());
        assert!(validate_domain("api.example.com").is_ok());
        assert!(validate_domain("sub.domain.example.com").is_ok());
        assert!(validate_domain("my-store.example.com").is_ok());
    }

    #[test]
    fn test_validate_domain_invalid() {
        assert!(validate_domain("").is_err());
        assert!(validate_domain("no-dots").is_err());
        assert!(validate_domain(".starts-with-dot.com").is_err());
        assert!(validate_domain("ends-with-dot.com.").is_err());
        assert!(validate_domain("-starts-with-hyphen.com").is_err());
        assert!(validate_domain("ends-with-hyphen-.com").is_err());
        assert!(validate_domain("has spaces.com").is_err());
        assert!(validate_domain("invalid..dots.com").is_err());
    }

    #[test]
    fn test_validate_uuid_valid() {
        assert!(validate_uuid("a1b2c3d4-e5f6-7890-abcd-ef1234567890").is_ok());
        assert!(validate_uuid("00000000-0000-0000-0000-000000000000").is_ok());
        assert!(validate_uuid("123e4567-e89b-12d3-a456-426614174000").is_ok());
    }

    #[test]
    fn test_validate_uuid_invalid() {
        assert!(validate_uuid("").is_err());
        assert!(validate_uuid("not-a-uuid").is_err());
        assert!(validate_uuid("12345678-1234-1234-1234").is_err());
        assert!(validate_uuid("12345678-1234-1234-1234-1234567890123").is_err());
    }

    #[test]
    fn test_validate_base_url_valid() {
        assert!(validate_base_url("https://api.example.com").is_ok());
        assert!(validate_base_url("https://localhost:8080").is_ok());
        assert!(validate_base_url("https://192.168.1.1").is_ok());
    }

    #[test]
    fn test_validate_base_url_invalid() {
        assert!(validate_base_url("http://api.example.com").is_err());
        assert!(validate_base_url("ftp://api.example.com").is_err());
        assert!(validate_base_url("https://").is_err());
        assert!(validate_base_url("api.example.com").is_err());
    }

    #[test]
    fn test_validate_directory_exists() {
        let temp_dir = TempDir::new().unwrap();
        assert!(validate_directory_exists(temp_dir.path()).is_ok());
    }

    #[test]
    fn test_validate_directory_not_exists() {
        let non_existent = PathBuf::from("/path/that/does/not/exist");
        assert!(validate_directory_exists(&non_existent).is_err());
    }

    #[test]
    fn test_validate_cron_valid() {
        assert!(validate_cron("0 8,12,16,18 * * *").is_ok());
        assert!(validate_cron("*/5 * * * *").is_ok());
        assert!(validate_cron("0 0 * * * *").is_ok()); // 6 fields
    }

    #[test]
    fn test_validate_cron_invalid() {
        assert!(validate_cron("").is_err());
        assert!(validate_cron("* * *").is_err());
        assert!(validate_cron("* * * * * * *").is_err());
    }

    #[test]
    fn test_validate_batch_size_valid() {
        assert!(validate_batch_size(1).is_ok());
        assert!(validate_batch_size(500).is_ok());
        assert!(validate_batch_size(1000).is_ok());
    }

    #[test]
    fn test_validate_batch_size_invalid() {
        assert!(validate_batch_size(0).is_err());
        assert!(validate_batch_size(1001).is_err());
        assert!(validate_batch_size(5000).is_err());
    }

    #[test]
    fn test_config_v2_validate_success() {
        let temp_dir = TempDir::new().unwrap();

        let config = ConfigV2 {
            auth: super::super::v2::AuthConfigV2 {
                domain: "store-01.example.com".to_string(),
                client_secret: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
            },
            api: super::super::v2::ApiConfigV2 {
                base_url: "https://api.example.com".to_string(),
                ..Default::default()
            },
            source: super::super::v2::SourceConfig {
                directory: temp_dir.path().to_path_buf(),
            },
            schedule: super::super::v2::ScheduleConfig {
                cron: "0 8,12,16,18 * * *".to_string(),
            },
            encoding: super::super::v2::EncodingConfig {
                fallback: "CP866".to_string(),
            },
            batch: super::super::v2::BatchConfig {
                max_files_per_batch: 500,
                ..Default::default()
            },
            logging: super::super::v2::LoggingConfig {
                error_log_path: PathBuf::from("/tmp/error.log"),
            },
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_v2_validate_multiple_errors() {
        let config = ConfigV2 {
            auth: super::super::v2::AuthConfigV2 {
                domain: "invalid domain with spaces".to_string(),
                client_secret: "not-a-uuid".to_string(),
            },
            api: super::super::v2::ApiConfigV2 {
                base_url: "http://api.example.com".to_string(), // Should be https
                ..Default::default()
            },
            source: super::super::v2::SourceConfig {
                directory: PathBuf::from("/path/that/does/not/exist"),
            },
            schedule: super::super::v2::ScheduleConfig {
                cron: "invalid cron".to_string(),
            },
            encoding: super::super::v2::EncodingConfig {
                fallback: "CP866".to_string(),
            },
            batch: super::super::v2::BatchConfig {
                max_files_per_batch: 5000, // Too large
                ..Default::default()
            },
            logging: super::super::v2::LoggingConfig {
                error_log_path: PathBuf::from("/tmp/error.log"),
            },
        };

        let result = config.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 6); // All validation errors should be collected
    }
}
