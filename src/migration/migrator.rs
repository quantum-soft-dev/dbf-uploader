// Configuration migration from v1 to v2
use super::{MigrationError, MigrationGuide, MigrationResult};
use crate::config::v2::{
    ApiConfigV2, AuthConfigV2, BatchConfig, ConfigV2, EncodingConfig, LoggingConfig,
    ScheduleConfig, SourceConfig,
};
use crate::models::config::Config as ConfigV1;
use std::path::{Path, PathBuf};

/// Migrate v1 configuration to v2 format
pub fn migrate_v1_to_v2(v1_config_path: &Path) -> MigrationResult<MigrationGuide> {
    // Read v1 configuration
    let v1_config = ConfigV1::from_file(v1_config_path).map_err(|e| {
        MigrationError::ParseError(format!("Failed to load v1 config: {}", e))
    })?;

    // Generate v2 configuration template
    let v2_config = generate_v2_template(&v1_config);

    // Serialize to TOML
    let template_toml = toml::to_string_pretty(&v2_config).map_err(|e| {
        MigrationError::ParseError(format!("Failed to serialize v2 config: {}", e))
    })?;

    // Create migration guide
    let mut guide = MigrationGuide::new(v1_config_path.to_path_buf(), template_toml);

    // Add preserved settings
    guide.add_preserved_setting(format!(
        "Source directory: {:?}",
        v1_config.src.source_dir
    ));
    guide.add_preserved_setting(format!("Cron schedule: {}", v1_config.scheduler.crontab));
    guide.add_preserved_setting(format!(
        "DBF encoding: {}",
        v1_config.encoding.dbf_encoding
    ));
    guide.add_preserved_setting(format!("API base URL: {}", v1_config.api.base_url));

    // Add manual actions
    guide.add_manual_action(
        "Log in to middleware admin UI at your base_url".to_string(),
    );
    guide.add_manual_action(
        "Navigate to Accounts → Your Account → Sites".to_string(),
    );
    guide.add_manual_action(
        "Create a new Site for this uploader instance".to_string(),
    );
    guide.add_manual_action(
        "Copy the site domain and clientSecret UUID".to_string(),
    );
    guide.add_manual_action(
        "Update config.toml with your domain and client_secret".to_string(),
    );

    // Add migration instructions
    guide.add_instruction(
        "Backup your current config.toml file".to_string(),
    );
    guide.add_instruction(
        "Create site credentials in middleware admin UI".to_string(),
    );
    guide.add_instruction(
        "Replace config.toml with the new v2 template".to_string(),
    );
    guide.add_instruction(
        "Update [auth] section with your domain and client_secret".to_string(),
    );
    guide.add_instruction(
        "Test authentication: dbf-uploader test-auth".to_string(),
    );
    guide.add_instruction(
        "Restart the Windows service to apply changes".to_string(),
    );

    Ok(guide)
}

/// Generate v2 configuration template from v1 config
fn generate_v2_template(v1_config: &ConfigV1) -> ConfigV2 {
    ConfigV2 {
        auth: AuthConfigV2 {
            domain: "REPLACE_WITH_YOUR_SITE_DOMAIN".to_string(),
            client_secret: "REPLACE_WITH_CLIENT_SECRET_UUID".to_string(),
        },
        api: ApiConfigV2 {
            base_url: v1_config.api.base_url.clone(),
            ..Default::default()
        },
        source: SourceConfig {
            directory: v1_config.src.source_dir.clone(),
        },
        schedule: ScheduleConfig {
            cron: v1_config.scheduler.crontab.clone(),
        },
        encoding: EncodingConfig {
            fallback: v1_config.encoding.dbf_encoding.clone(),
        },
        batch: BatchConfig::default(),
        logging: LoggingConfig {
            error_log_path: PathBuf::from("C:\\Program Files\\dbf-uploader\\error.log"),
        },
    }
}

/// Write migration guide to file
pub fn write_migration_guide(guide: &MigrationGuide, output_path: &Path) -> MigrationResult<()> {
    let mut content = String::new();

    // Add header
    content.push_str("# DBF Uploader v1 → v2 Migration Guide\n\n");

    // Add breaking changes warning
    content.push_str("## ⚠️  BREAKING CHANGES\n\n");
    content.push_str("- Username/password authentication REMOVED\n");
    content.push_str("- Site credentials (domain + clientSecret) REQUIRED\n");
    content.push_str("- Batch protocol lifecycle REQUIRED\n\n");

    // Add preserved settings
    if !guide.preserved_settings.is_empty() {
        content.push_str("## ✅ Preserved Settings\n\n");
        for setting in &guide.preserved_settings {
            content.push_str(&format!("- {}\n", setting));
        }
        content.push_str("\n");
    }

    // Add manual actions
    if !guide.manual_actions.is_empty() {
        content.push_str("## ⚠️  Manual Actions Required\n\n");
        for action in &guide.manual_actions {
            content.push_str(&format!("- {}\n", action));
        }
        content.push_str("\n");
    }

    // Add migration steps
    content.push_str("## 📋 Migration Steps\n\n");
    for (i, instruction) in guide.instructions.iter().enumerate() {
        content.push_str(&format!("{}. {}\n", i + 1, instruction));
    }
    content.push_str("\n");

    // Add v2 config template
    content.push_str("## 📄 New Configuration Template (config.toml)\n\n");
    content.push_str("```toml\n");
    content.push_str(&guide.template_config);
    content.push_str("\n```\n");

    // Write to file
    std::fs::write(output_path, content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    fn create_v1_config() -> (NamedTempFile, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap();

        let toml_content = format!(
            r#"
[scheduler]
crontab = "0 8,12,16,18 * * *"

[src]
source_dir = "{}"

[credential]
username = "olduser"
password = "oldpass"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP866"
        "#,
            temp_dir_path.replace('\\', "\\\\")
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        (temp_file, temp_dir)
    }

    #[test]
    fn test_migrate_v1_to_v2_success() {
        let (temp_file, _temp_dir) = create_v1_config();
        let guide = migrate_v1_to_v2(temp_file.path()).unwrap();

        // Check that template config contains v2 structure
        assert!(guide.template_config.contains("[auth]"));
        assert!(guide.template_config.contains("domain ="));
        assert!(guide.template_config.contains("client_secret ="));
        assert!(guide.template_config.contains("[batch]"));
        assert!(guide.template_config.contains("max_files_per_batch"));

        // Check preserved settings
        assert_eq!(guide.preserved_settings.len(), 4);
        assert!(guide.preserved_settings[1].contains("0 8,12,16,18 * * *"));
        assert!(guide.preserved_settings[2].contains("CP866"));
        assert!(guide.preserved_settings[3].contains("https://api.example.com"));

        // Check manual actions
        assert!(!guide.manual_actions.is_empty());
        assert!(guide
            .manual_actions
            .iter()
            .any(|a| a.contains("middleware admin UI")));

        // Check instructions
        assert!(!guide.instructions.is_empty());
        assert!(guide
            .instructions
            .iter()
            .any(|i| i.contains("Backup your current config.toml")));
    }

    #[test]
    fn test_migrate_v1_to_v2_preserves_values() {
        let (temp_file, temp_dir) = create_v1_config();
        let guide = migrate_v1_to_v2(temp_file.path()).unwrap();

        // Parse the generated v2 config
        let v2_config: ConfigV2 = toml::from_str(&guide.template_config).unwrap();

        // Check preserved values
        assert_eq!(v2_config.schedule.cron, "0 8,12,16,18 * * *");
        assert_eq!(v2_config.encoding.fallback, "CP866");
        assert_eq!(v2_config.api.base_url, "https://api.example.com");
        assert_eq!(v2_config.source.directory, temp_dir.path());

        // Check new v2 values
        assert_eq!(
            v2_config.auth.domain,
            "REPLACE_WITH_YOUR_SITE_DOMAIN"
        );
        assert_eq!(
            v2_config.auth.client_secret,
            "REPLACE_WITH_CLIENT_SECRET_UUID"
        );
        assert_eq!(v2_config.batch.max_files_per_batch, 500);
        assert_eq!(v2_config.batch.retry_locked_files, true);
    }

    #[test]
    fn test_migrate_v1_to_v2_invalid_config() {
        let result = migrate_v1_to_v2(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_v2_template() {
        let temp_dir = TempDir::new().unwrap();

        let v1_config = ConfigV1 {
            scheduler: crate::models::config::SchedulerConfig {
                crontab: "*/5 * * * *".to_string(),
            },
            src: crate::models::config::SourceConfig {
                source_dir: temp_dir.path().to_path_buf(),
            },
            credential: crate::models::config::CredentialConfig {
                username: "test".to_string(),
                password: "test".to_string(),
            },
            api: crate::models::config::ApiConfig {
                base_url: "https://test.example.com".to_string(),
            },
            encoding: crate::models::config::EncodingConfig {
                dbf_encoding: "UTF-8".to_string(),
            },
        };

        let v2_config = generate_v2_template(&v1_config);

        assert_eq!(v2_config.schedule.cron, "*/5 * * * *");
        assert_eq!(v2_config.source.directory, temp_dir.path());
        assert_eq!(v2_config.api.base_url, "https://test.example.com");
        assert_eq!(v2_config.encoding.fallback, "UTF-8");
    }

    #[test]
    fn test_write_migration_guide() {
        let (temp_file, _temp_dir) = create_v1_config();
        let guide = migrate_v1_to_v2(temp_file.path()).unwrap();

        let output_file = NamedTempFile::new().unwrap();
        write_migration_guide(&guide, output_file.path()).unwrap();

        let content = std::fs::read_to_string(output_file.path()).unwrap();

        // Check content structure
        assert!(content.contains("# DBF Uploader v1 → v2 Migration Guide"));
        assert!(content.contains("## ⚠️  BREAKING CHANGES"));
        assert!(content.contains("## ✅ Preserved Settings"));
        assert!(content.contains("## ⚠️  Manual Actions Required"));
        assert!(content.contains("## 📋 Migration Steps"));
        assert!(content.contains("## 📄 New Configuration Template"));
        assert!(content.contains("```toml"));
    }
}
