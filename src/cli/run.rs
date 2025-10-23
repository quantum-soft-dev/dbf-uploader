use super::install;
use crate::config::ConfigV2;
use crate::error::{ProcessingError, Result};
use crate::service::UploaderService;
use std::fmt::Write as _;
use tracing::info;

/// Execute a single batch upload using the existing configuration.
pub async fn run_once() -> Result<()> {
    let config_path = install::get_config_path()?;

    if !config_path.exists() {
        return Err(ProcessingError::ConfigurationError(format!(
            "Configuration file not found at {}",
            config_path.display()
        )));
    }

    let config = ConfigV2::from_file(&config_path).map_err(|e| {
        ProcessingError::ConfigurationError(format!("Failed to load configuration: {}", e))
    })?;

    if let Err(errors) = config.validate() {
        let mut message = String::from("Configuration validation failed:");
        for error in errors {
            let _ = write!(message, " {}", error);
        }
        return Err(ProcessingError::ConfigurationError(message));
    }

    let service = UploaderService::from_config(config).await?;
    let summary = service.run_scheduled_batch().await?;

    info!(
        batch_id = %summary.batch_id,
        processed = summary.processed_count,
        failed = summary.failed_count,
        size_bytes = summary.total_size,
        duration_secs = summary.duration_secs,
        "Batch completed successfully"
    );

    println!(
        "Batch {} complete. Processed: {}, failed: {}, total size: {} bytes, duration: {}s",
        summary.batch_id,
        summary.processed_count,
        summary.failed_count,
        summary.total_size,
        summary.duration_secs
    );

    Ok(())
}
