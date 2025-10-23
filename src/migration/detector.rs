// Configuration version detection
use super::{ConfigVersion, MigrationError, MigrationResult};
use std::path::Path;

/// Detect configuration version from file content
pub fn detect_version(config_path: &Path) -> MigrationResult<ConfigVersion> {
    // Check if file exists
    if !config_path.exists() {
        return Err(MigrationError::ConfigNotFound(config_path.to_path_buf()));
    }

    // Read config file content
    let content = std::fs::read_to_string(config_path)?;

    // Normalize line endings for cross-platform compatibility
    let normalized = content.replace("\r\n", "\n");

    // Detect version based on content patterns
    detect_version_from_content(&normalized)
}

/// Detect version from normalized configuration content
fn detect_version_from_content(content: &str) -> MigrationResult<ConfigVersion> {
    // V2 indicators (check these first as they are more specific)
    let has_domain_field =
        content.contains("[auth]") && (content.contains("domain =") || content.contains("domain="));

    let has_client_secret = content.contains("client_secret") || content.contains("clientSecret");

    let has_batch_section = content.contains("[batch]");

    let has_v2_endpoints = content.contains("batch_start")
        || content.contains("batch_upload")
        || content.contains("batch_complete");

    // V1 indicators
    let has_username = content.contains("[auth]")
        && (content.contains("username =") || content.contains("username="));

    let has_password = content.contains("password =") || content.contains("password=");

    // Determine version based on indicators
    if has_domain_field || has_client_secret || has_batch_section || has_v2_endpoints {
        // Strong indicators of v2.0
        Ok(ConfigVersion::V2)
    } else if has_username || has_password {
        // Strong indicators of v1.0
        Ok(ConfigVersion::V1)
    } else {
        // Cannot determine version
        Err(MigrationError::UnknownVersion)
    }
}

/// Check if configuration file exists and is readable
pub fn validate_config_file(config_path: &Path) -> MigrationResult<()> {
    if !config_path.exists() {
        return Err(MigrationError::ConfigNotFound(config_path.to_path_buf()));
    }

    if !config_path.is_file() {
        return Err(MigrationError::InvalidConfig(format!(
            "Path is not a file: {:?}",
            config_path
        )));
    }

    // Try to read the file to ensure it's readable
    std::fs::read_to_string(config_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_config(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_detect_v1_config_with_username() {
        let content = r#"
[auth]
username = "user123"
password = "pass123"

[api]
base_url = "https://api.example.com"
"#;
        let temp_file = create_temp_config(content);
        let version = detect_version(temp_file.path()).unwrap();
        assert_eq!(version, ConfigVersion::V1);
    }

    #[test]
    fn test_detect_v2_config_with_domain() {
        let content = r#"
[auth]
domain = "store-01.example.com"
client_secret = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"

[api]
base_url = "https://api.example.com"
"#;
        let temp_file = create_temp_config(content);
        let version = detect_version(temp_file.path()).unwrap();
        assert_eq!(version, ConfigVersion::V2);
    }

    #[test]
    fn test_detect_v2_config_with_batch_section() {
        let content = r#"
[auth]
domain = "store-01.example.com"
client_secret = "test-uuid"

[batch]
max_files_per_batch = 500
"#;
        let temp_file = create_temp_config(content);
        let version = detect_version(temp_file.path()).unwrap();
        assert_eq!(version, ConfigVersion::V2);
    }

    #[test]
    fn test_detect_v2_config_with_endpoints() {
        let content = r#"
[auth]
domain = "test.com"

[api]
base_url = "https://api.example.com"
batch_start = "/api/v1/batch/start"
"#;
        let temp_file = create_temp_config(content);
        let version = detect_version(temp_file.path()).unwrap();
        assert_eq!(version, ConfigVersion::V2);
    }

    #[test]
    fn test_detect_version_windows_line_endings() {
        let content = "[auth]\r\nusername = \"user\"\r\npassword = \"pass\"\r\n";
        let temp_file = create_temp_config(content);
        let version = detect_version(temp_file.path()).unwrap();
        assert_eq!(version, ConfigVersion::V1);
    }

    #[test]
    fn test_detect_version_unknown() {
        let content = r#"
[source]
directory = "/tmp/data"

[schedule]
cron = "*/5 * * * *"
"#;
        let temp_file = create_temp_config(content);
        let result = detect_version(temp_file.path());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MigrationError::UnknownVersion
        ));
    }

    #[test]
    fn test_detect_version_file_not_found() {
        let result = detect_version(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MigrationError::ConfigNotFound(_)
        ));
    }

    #[test]
    fn test_validate_config_file_exists() {
        let content = "[auth]\nusername = \"test\"";
        let temp_file = create_temp_config(content);
        let result = validate_config_file(temp_file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_file_not_found() {
        let result = validate_config_file(Path::new("/nonexistent.toml"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MigrationError::ConfigNotFound(_)
        ));
    }

    #[test]
    fn test_detect_version_from_content_v1() {
        let content = "[auth]\nusername = \"test\"\npassword = \"test\"";
        let version = detect_version_from_content(content).unwrap();
        assert_eq!(version, ConfigVersion::V1);
    }

    #[test]
    fn test_detect_version_from_content_v2() {
        let content = "[auth]\ndomain = \"test.com\"\nclient_secret = \"uuid\"";
        let version = detect_version_from_content(content).unwrap();
        assert_eq!(version, ConfigVersion::V2);
    }

    #[test]
    fn test_detect_version_from_content_unknown() {
        let content = "[source]\ndirectory = \"/tmp\"";
        let result = detect_version_from_content(content);
        assert!(result.is_err());
    }
}
