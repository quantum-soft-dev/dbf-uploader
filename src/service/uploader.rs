// Uploader service with v2 batch protocol
use crate::auth::{AuthClient, SiteCredentials, TokenManager};
use crate::batch::BatchManager;
use crate::config::v2::ConfigV2;
use crate::error::{ErrorReporter, ProcessingError, Result};
use crate::processor::{compress_csv, convert_dbf_to_csv, scan_directory};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// Uploader service orchestrating batch processing with v2 protocol
pub struct UploaderService {
    /// Batch manager for batch lifecycle
    batch_manager: Arc<Mutex<BatchManager>>,
    /// Error reporter for logging errors
    error_reporter: Arc<ErrorReporter>,
    /// Configuration
    config: ConfigV2,
}

impl UploaderService {
    /// Create a new uploader service
    pub fn new(
        batch_manager: Arc<Mutex<BatchManager>>,
        error_reporter: Arc<ErrorReporter>,
        config: ConfigV2,
    ) -> Self {
        Self {
            batch_manager,
            error_reporter,
            config,
        }
    }

    /// Create uploader service from configuration
    pub async fn from_config(config: ConfigV2) -> Result<Self> {
        // Create AuthClient with SiteCredentials
        let credentials = SiteCredentials {
            domain: config.auth.domain.clone(),
            client_secret: config.auth.client_secret.clone(),
        };

        let auth_client = Arc::new(AuthClient::from_credentials(
            config.api.base_url.clone(),
            credentials,
        )?);

        // Create TokenManager from AuthClient
        let token_manager = Arc::new(TokenManager::from_auth_client(Arc::clone(&auth_client)));

        // Create BatchManager with TokenManager
        let batch_manager = Arc::new(Mutex::new(BatchManager::new(
            config.api.base_url.clone(),
            token_manager,
        )?));

        // Create ErrorReporter (TODO Phase 1.3: update when ErrorReporter v2 is implemented)
        let error_reporter = Arc::new(ErrorReporter::without_auth(
            config.api.base_url.clone(),
            config.logging.error_log_path.clone(),
        )?);

        Ok(Self {
            batch_manager,
            error_reporter,
            config,
        })
    }

    /// Execute scheduled batch upload
    pub async fn run_scheduled_batch(&self) -> Result<BatchSummary> {
        info!("Starting scheduled batch upload");

        // Step 1: Start new batch
        let batch = {
            let batch_mgr = self.batch_manager.lock().await;
            match batch_mgr.start_batch().await {
                Ok(batch) => batch,
                Err(e) => {
                    error!("Failed to start batch: {}", e);
                    // TODO Phase 1.3: use error_reporter.report_standalone_error()
                    return Err(ProcessingError::BatchError(format!(
                        "Failed to start batch: {}",
                        e
                    )));
                }
            }
        };

        let batch_id = batch.id;
        info!(batch_id = %batch_id, "Batch started");

        // Step 2: Scan for DBF files
        let dbf_files = match scan_directory(&self.config.source.directory) {
            Ok(files) => files,
            Err(e) => {
                error!("Failed to scan directory: {}", e);
                // Fail the batch
                let batch_mgr = self.batch_manager.lock().await;
                batch_mgr
                    .fail_batch(&format!("Directory scan failed: {}", e))
                    .await?;
                return Err(e);
            }
        };

        info!(file_count = dbf_files.len(), "DBF files found");

        if dbf_files.is_empty() {
            warn!("No DBF files found, cancelling batch");
            let batch_mgr = self.batch_manager.lock().await;
            batch_mgr.cancel_batch().await?;
            return Ok(BatchSummary {
                batch_id,
                processed_count: 0,
                failed_count: 0,
                total_size: 0,
                duration_secs: 0,
            });
        }

        // Step 3: Convert DBF → CSV → gzip
        let mut converted_files = Vec::new();
        let mut locked_files = Vec::new();
        let mut failed_count = 0;

        for dbf_file in &dbf_files {
            match self.convert_and_compress(dbf_file).await {
                Ok(compressed_path) => {
                    converted_files.push(compressed_path);
                }
                Err(e) if is_file_locked(&e) => {
                    warn!(file = %dbf_file.path.display(), "File is locked, will retry");
                    locked_files.push(dbf_file.clone());
                }
                Err(e) => {
                    error!(file = %dbf_file.path.display(), error = %e, "Conversion failed");
                    failed_count += 1;
                    // TODO Phase 1.3: error_reporter.report_batch_error(batch_id, ...)
                }
            }
        }

        // Step 4: Retry locked files
        if self.config.batch.retry_locked_files && !locked_files.is_empty() {
            info!(locked_count = locked_files.len(), "Retrying locked files");

            // Wait a bit before retry
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            for dbf_file in &locked_files {
                match self.convert_and_compress(dbf_file).await {
                    Ok(compressed_path) => {
                        info!(file = %dbf_file.path.display(), "Locked file processed on retry");
                        converted_files.push(compressed_path);
                    }
                    Err(e) => {
                        warn!(file = %dbf_file.path.display(), error = %e, "Still locked or failed");
                        failed_count += 1;
                    }
                }
            }
        }

        if converted_files.is_empty() {
            warn!("No files to upload, failing batch");
            let batch_mgr = self.batch_manager.lock().await;
            batch_mgr
                .fail_batch("No files could be converted")
                .await?;
            return Ok(BatchSummary {
                batch_id,
                processed_count: 0,
                failed_count,
                total_size: 0,
                duration_secs: 0,
            });
        }

        // Step 5: Upload files in chunks
        let max_per_batch = self.config.batch.max_files_per_batch;
        let mut upload_retries = 0;
        let max_retries = self.config.batch.max_retries;

        for chunk in converted_files.chunks(max_per_batch) {
            let mut retry_count = 0;

            loop {
                let batch_mgr = self.batch_manager.lock().await;

                match batch_mgr.upload_files(chunk.to_vec()).await {
                    Ok(summary) => {
                        info!(
                            uploaded = summary.files.len(),
                            total_size = summary.files.iter().map(|f| f.file_size).sum::<u64>(),
                            "Uploaded files chunk"
                        );
                        break;
                    }
                    Err(e) => {
                        retry_count += 1;
                        upload_retries += 1;

                        if retry_count >= max_retries {
                            error!(error = %e, "Batch upload failed after {} retries", max_retries);
                            // TODO Phase 1.3: error_reporter.report_batch_error(batch_id, ...)

                            batch_mgr
                                .fail_batch(&format!("Upload failed: {}", e))
                                .await?;

                            return Err(ProcessingError::UploadError(format!(
                                "Upload failed after {} retries: {}",
                                max_retries, e
                            )));
                        }

                        warn!(error = %e, retry = retry_count, "Upload failed, retrying");
                        drop(batch_mgr); // Release lock before sleep
                        tokio::time::sleep(std::time::Duration::from_secs(2_u64.pow(retry_count)))
                            .await;
                    }
                }
            }
        }

        // Step 6: Complete batch
        let start_time = std::time::Instant::now();
        let complete_response = {
            let batch_mgr = self.batch_manager.lock().await;
            match batch_mgr.complete_batch().await {
                Ok(response) => {
                    info!(
                        batch_id = %batch_id,
                        uploaded_count = response.uploaded_files_count,
                        total_size = response.total_size,
                        "Batch completed successfully"
                    );
                    response
                }
                Err(e) => {
                    error!(error = %e, "Failed to complete batch");
                    // TODO Phase 1.3: error_reporter.report_batch_error(batch_id, ...)
                    return Err(ProcessingError::BatchError(format!(
                        "Failed to complete batch: {}",
                        e
                    )));
                }
            }
        };

        // Step 7: Cleanup converted files
        for file in &converted_files {
            if let Err(e) = tokio::fs::remove_file(file).await {
                warn!(file = %file.display(), error = %e, "Failed to delete compressed file");
            }
        }

        let duration_secs = start_time.elapsed().as_secs() as i64;

        info!(
            upload_retries = upload_retries,
            "Batch upload completed successfully"
        );

        Ok(BatchSummary {
            batch_id,
            processed_count: complete_response.uploaded_files_count as usize,
            failed_count,
            total_size: complete_response.total_size as u64,
            duration_secs,
        })
    }

    /// Convert DBF to CSV and compress
    async fn convert_and_compress(
        &self,
        dbf_file: &crate::models::DbfFile,
    ) -> Result<PathBuf> {
        // Step 1: Convert DBF to CSV
        let csv_path = convert_dbf_to_csv(dbf_file, &self.config_v1_compat())?;

        // Step 2: Compress CSV to gzip
        let compressed_filename = dbf_file.generate_compressed_filename();
        let gzip_path = compress_csv(csv_path.clone(), compressed_filename)?;

        // Step 3: Cleanup CSV file
        if let Err(e) = std::fs::remove_file(&csv_path) {
            warn!(file = %csv_path.display(), error = %e, "Failed to delete CSV file");
        }

        Ok(gzip_path)
    }

    /// Create v1 config compatibility layer
    /// TODO: Remove this when convert_dbf_to_csv is updated to use ConfigV2
    fn config_v1_compat(&self) -> crate::models::config::Config {
        crate::models::config::Config {
            scheduler: crate::models::config::SchedulerConfig {
                crontab: self.config.schedule.cron.clone(),
            },
            src: crate::models::config::SourceConfig {
                source_dir: self.config.source.directory.clone(),
            },
            credential: crate::models::config::CredentialConfig {
                username: self.config.auth.domain.clone(),
                password: self.config.auth.client_secret.clone(),
            },
            api: crate::models::config::ApiConfig {
                base_url: self.config.api.base_url.clone(),
            },
            encoding: crate::models::config::EncodingConfig {
                dbf_encoding: self.config.encoding.fallback.clone(),
            },
        }
    }
}

/// Batch summary result
#[derive(Debug, Clone)]
pub struct BatchSummary {
    pub batch_id: uuid::Uuid,
    pub processed_count: usize,
    pub failed_count: usize,
    pub total_size: u64,
    pub duration_secs: i64,
}

/// Check if an error is due to a locked file
fn is_file_locked(error: &ProcessingError) -> bool {
    match error {
        ProcessingError::FileReadError(io_error) => matches!(
            io_error.kind(),
            ErrorKind::PermissionDenied | ErrorKind::WouldBlock
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_file_locked_permission_denied() {
        let io_err = std::io::Error::new(ErrorKind::PermissionDenied, "access denied");
        let proc_err = ProcessingError::FileReadError(io_err);
        assert!(is_file_locked(&proc_err));
    }

    #[test]
    fn test_is_file_locked_would_block() {
        let io_err = std::io::Error::new(ErrorKind::WouldBlock, "would block");
        let proc_err = ProcessingError::FileReadError(io_err);
        assert!(is_file_locked(&proc_err));
    }

    #[test]
    fn test_is_file_locked_not_found() {
        let io_err = std::io::Error::new(ErrorKind::NotFound, "not found");
        let proc_err = ProcessingError::FileReadError(io_err);
        assert!(!is_file_locked(&proc_err));
    }

    #[test]
    fn test_is_file_locked_other_error() {
        let proc_err = ProcessingError::ConfigurationError("test".to_string());
        assert!(!is_file_locked(&proc_err));
    }

    #[test]
    fn test_batch_summary_creation() {
        let summary = BatchSummary {
            batch_id: uuid::Uuid::new_v4(),
            processed_count: 10,
            failed_count: 2,
            total_size: 1024,
            duration_secs: 60,
        };

        assert_eq!(summary.processed_count, 10);
        assert_eq!(summary.failed_count, 2);
        assert_eq!(summary.total_size, 1024);
        assert_eq!(summary.duration_secs, 60);
    }
}
