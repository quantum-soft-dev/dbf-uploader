// Interactive migration wizard
use super::{test_auth, MigrationError, MigrationResult};
use crate::config::v2::{
    ApiConfigV2, AuthConfigV2, BatchConfig, ConfigV2, EncodingConfig, LoggingConfig,
    ScheduleConfig, SourceConfig,
};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Prompt user for input with a message
pub fn prompt(message: &str) -> MigrationResult<String> {
    print!("{}", message);
    io::stdout()
        .flush()
        .map_err(|e| MigrationError::InvalidConfig(format!("IO error: {}", e)))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| MigrationError::InvalidConfig(format!("Failed to read input: {}", e)))?;

    Ok(input.trim().to_string())
}

/// Prompt user for input with validation
pub fn prompt_with_validation<F>(message: &str, validator: F) -> MigrationResult<String>
where
    F: Fn(&str) -> Result<(), String>,
{
    loop {
        let input = prompt(message)?;

        match validator(&input) {
            Ok(()) => return Ok(input),
            Err(error_msg) => {
                println!("❌ Invalid input: {}", error_msg);
                println!("Please try again.");
            }
        }
    }
}

/// Prompt for domain with validation
pub fn prompt_domain() -> MigrationResult<String> {
    prompt_with_validation(
        "Enter site domain (e.g., store-01.example.com): ",
        |input| {
            if input.is_empty() {
                return Err("Domain cannot be empty".to_string());
            }

            if !input.contains('.') {
                return Err(
                    "Domain must be a fully qualified domain name (e.g., store-01.example.com)"
                        .to_string(),
                );
            }

            if input.contains(' ') {
                return Err("Domain cannot contain spaces".to_string());
            }

            Ok(())
        },
    )
}

/// Prompt for client secret with UUID validation
pub fn prompt_client_secret() -> MigrationResult<String> {
    prompt_with_validation(
        "Enter client secret UUID (e.g., a1b2c3d4-e5f6-7890-abcd-ef1234567890): ",
        |input| {
            if input.is_empty() {
                return Err("Client secret cannot be empty".to_string());
            }

            // Basic UUID format check (8-4-4-4-12 hex characters)
            let parts: Vec<&str> = input.split('-').collect();
            if parts.len() != 5 {
                return Err("Client secret must be a valid UUID format (e.g., a1b2c3d4-e5f6-7890-abcd-ef1234567890)".to_string());
            }

            if parts[0].len() != 8
                || parts[1].len() != 4
                || parts[2].len() != 4
                || parts[3].len() != 4
                || parts[4].len() != 12
            {
                return Err(
                    "Invalid UUID format. Expected format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                        .to_string(),
                );
            }

            // Check all characters are hexadecimal
            if !input.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
                return Err("UUID must contain only hexadecimal characters and hyphens".to_string());
            }

            Ok(())
        },
    )
}

/// Prompt for base URL with validation
pub fn prompt_base_url() -> MigrationResult<String> {
    prompt_with_validation(
        "Enter API base URL (e.g., https://api.example.com): ",
        |input| {
            if input.is_empty() {
                return Err("Base URL cannot be empty".to_string());
            }

            if !input.starts_with("https://") && !input.starts_with("http://") {
                return Err("Base URL must start with https:// or http://".to_string());
            }

            if input.len() < 10 {
                return Err("Base URL is too short".to_string());
            }

            Ok(())
        },
    )
}

/// Prompt for source directory with validation
pub fn prompt_source_directory() -> MigrationResult<PathBuf> {
    let input = prompt_with_validation("Enter source directory path: ", |input| {
        if input.is_empty() {
            return Err("Source directory cannot be empty".to_string());
        }

        let path = Path::new(input);
        if !path.exists() {
            return Err(format!("Directory does not exist: {}", input));
        }

        if !path.is_dir() {
            return Err(format!("Path is not a directory: {}", input));
        }

        Ok(())
    })?;

    Ok(PathBuf::from(input))
}

/// Prompt for optional cron schedule with default
pub fn prompt_cron_schedule(default: &str) -> MigrationResult<String> {
    let message = format!(
        "Enter cron schedule (or press Enter for default '{}'): ",
        default
    );

    let input = prompt(&message)?;

    if input.is_empty() {
        Ok(default.to_string())
    } else {
        // Basic cron validation (5 or 6 fields)
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() < 5 || parts.len() > 6 {
            return Err(MigrationError::InvalidConfig(format!(
                "Invalid cron expression. Expected 5 or 6 fields, got {}",
                parts.len()
            )));
        }

        Ok(input)
    }
}

/// Run interactive migration wizard
pub async fn run_wizard() -> MigrationResult<ConfigV2> {
    println!("╔═══════════════════════════════════════════════════╗");
    println!("║   DBF Uploader Configuration Migration Wizard    ║");
    println!("╚═══════════════════════════════════════════════════╝");
    println!();
    println!("This wizard will help you create a v2.0 configuration.");
    println!("You will need site credentials from the middleware admin UI.");
    println!();

    // Prompt for configuration values
    let domain = prompt_domain()?;
    let client_secret = prompt_client_secret()?;
    let base_url = prompt_base_url()?;

    // Test authentication
    println!();
    println!("🔍 Testing authentication...");

    match test_auth(&base_url, &domain, &client_secret).await {
        Ok(result) if result.success => {
            println!("✅ Authentication successful!");
            if let Some(expires_in) = result.expires_in {
                println!("   Token expires in: {} seconds", expires_in);
            }
        }
        Ok(result) => {
            println!("❌ Authentication failed: {}", result.message);
            println!();
            println!("Please verify your credentials and try again.");
            return Err(MigrationError::AuthTestFailed(result.message));
        }
        Err(e) => {
            println!("❌ Authentication test error: {}", e);
            return Err(e);
        }
    }

    println!();

    // Prompt for source directory
    let source_dir = prompt_source_directory()?;

    // Prompt for optional cron schedule
    let cron = prompt_cron_schedule("0 8,12,16,18 * * *")?;

    // Generate ConfigV2
    let config = ConfigV2 {
        auth: AuthConfigV2 {
            domain,
            client_secret,
        },
        api: ApiConfigV2 {
            base_url,
            ..Default::default()
        },
        source: SourceConfig {
            directory: source_dir,
        },
        schedule: ScheduleConfig { cron },
        encoding: EncodingConfig {
            fallback: "CP866".to_string(),
        },
        batch: BatchConfig::default(),
        logging: LoggingConfig {
            error_log_path: PathBuf::from("C:\\Program Files\\dbf-uploader\\error.log"),
        },
    };

    println!();
    println!("✅ Configuration wizard completed successfully!");
    println!();

    Ok(config)
}

/// Write configuration to file
pub fn write_config(config: &ConfigV2, config_path: &Path) -> MigrationResult<()> {
    // Serialize to TOML
    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| MigrationError::InvalidConfig(format!("Failed to serialize config: {}", e)))?;

    // Write to file
    std::fs::write(config_path, toml_string)?;

    println!("✅ Configuration written to: {:?}", config_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_domain_validation_empty() {
        let validator = |input: &str| {
            if input.is_empty() {
                return Err("Domain cannot be empty".to_string());
            }
            Ok(())
        };

        assert!(validator("").is_err());
        assert!(validator("test.com").is_ok());
    }

    #[test]
    fn test_prompt_domain_validation_no_dot() {
        let validator = |input: &str| {
            if !input.contains('.') {
                return Err("Domain must contain a dot".to_string());
            }
            Ok(())
        };

        assert!(validator("nodot").is_err());
        assert!(validator("has.dot").is_ok());
    }

    #[test]
    fn test_prompt_domain_validation_spaces() {
        let validator = |input: &str| {
            if input.contains(' ') {
                return Err("Domain cannot contain spaces".to_string());
            }
            Ok(())
        };

        assert!(validator("has spaces.com").is_err());
        assert!(validator("nospaces.com").is_ok());
    }

    #[test]
    fn test_client_secret_uuid_validation() {
        let validator = |input: &str| {
            let parts: Vec<&str> = input.split('-').collect();
            if parts.len() != 5 {
                return Err("Not a UUID".to_string());
            }
            Ok(())
        };

        assert!(validator("").is_err());
        assert!(validator("not-a-uuid").is_err());
        assert!(validator("a1b2c3d4-e5f6-7890-abcd-ef1234567890").is_ok());
    }

    #[test]
    fn test_base_url_validation() {
        let validator = |input: &str| {
            if !input.starts_with("https://") && !input.starts_with("http://") {
                return Err("Must start with https:// or http://".to_string());
            }
            Ok(())
        };

        assert!(validator("api.example.com").is_err());
        assert!(validator("ftp://api.example.com").is_err());
        assert!(validator("https://api.example.com").is_ok());
        assert!(validator("http://localhost:8080").is_ok());
    }

    #[test]
    fn test_cron_validation() {
        let validate_cron = |input: &str| {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() < 5 || parts.len() > 6 {
                return Err(format!("Expected 5-6 fields, got {}", parts.len()));
            }
            Ok(())
        };

        assert!(validate_cron("* * * * *").is_ok());
        assert!(validate_cron("0 8,12,16,18 * * *").is_ok());
        assert!(validate_cron("0 0 * * * *").is_ok()); // 6 fields
        assert!(validate_cron("* * *").is_err()); // Too few
        assert!(validate_cron("* * * * * * *").is_err()); // Too many
    }

    #[test]
    fn test_write_config() {
        use tempfile::NamedTempFile;

        let temp_dir = tempfile::TempDir::new().unwrap();

        let config = ConfigV2 {
            auth: AuthConfigV2 {
                domain: "test.example.com".to_string(),
                client_secret: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
            },
            api: ApiConfigV2::default(),
            source: SourceConfig {
                directory: temp_dir.path().to_path_buf(),
            },
            schedule: ScheduleConfig {
                cron: "*/5 * * * *".to_string(),
            },
            encoding: EncodingConfig {
                fallback: "CP866".to_string(),
            },
            batch: BatchConfig::default(),
            logging: LoggingConfig {
                error_log_path: PathBuf::from("/tmp/error.log"),
            },
        };

        let temp_file = NamedTempFile::new().unwrap();
        let result = write_config(&config, temp_file.path());
        assert!(result.is_ok());

        // Verify file was written
        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("test.example.com"));
        assert!(content.contains("a1b2c3d4-e5f6-7890-abcd-ef1234567890"));
    }
}
