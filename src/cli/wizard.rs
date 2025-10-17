// Installation wizard for v2.0
use crate::config::v2::{
    ApiConfigV2, AuthConfigV2, BatchConfig, ConfigV2, EncodingConfig, LoggingConfig,
    ScheduleConfig, SourceConfig,
};
use crate::error::Result;
use std::path::PathBuf;
use tracing::info;

/// Run interactive installation wizard for fresh v2.0 installations
///
/// This wizard guides the user through:
/// - Site credentials setup (domain + client_secret)
/// - Source directory configuration
/// - Middleware API URL
/// - Schedule (cron format)
/// - Encoding settings
///
/// Returns a fully configured ConfigV2 ready for installation
pub async fn run_installation_wizard() -> Result<ConfigV2> {
    info!("Starting interactive installation wizard");

    // TODO: Implement interactive wizard
    // For now, return a minimal valid configuration for testing
    let config = ConfigV2 {
        auth: AuthConfigV2 {
            domain: "test.example.com".to_string(),
            client_secret: "00000000-0000-0000-0000-000000000000".to_string(),
        },
        api: ApiConfigV2::default(),
        source: SourceConfig {
            directory: PathBuf::from("/tmp/test"),
        },
        schedule: ScheduleConfig {
            cron: "0 * * * *".to_string(),
        },
        encoding: EncodingConfig {
            fallback: "CP866".to_string(),
        },
        batch: BatchConfig::default(),
        logging: LoggingConfig {
            error_log_path: PathBuf::from("error.log"),
        },
    };

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wizard_returns_valid_config() {
        let result = run_installation_wizard().await;
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.auth.domain, "test.example.com");
        assert_eq!(
            config.auth.client_secret,
            "00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(config.schedule.cron, "0 * * * *");
    }
}
