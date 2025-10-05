// File processing module
pub mod scanner;
pub mod converter;
pub mod compressor;
pub mod uploader;

pub use scanner::scan_directory;
pub use converter::convert_dbf_to_csv;
pub use compressor::compress_csv;
pub use uploader::upload_file;

use crate::auth::TokenManager;
use crate::error::{log_error_locally, ErrorReporter, ProcessingError, Result};
use crate::models::{Batch, BatchStatus, Config, ErrorReport};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Run a complete batch processing cycle
/// This orchestrates: scan → convert → compress → upload for all DBF files
pub async fn run_batch(
    config: Config,
    token_manager: Arc<TokenManager>,
) -> Result<Batch> {
    let mut batch = Batch::new(config.clone());
    let error_reporter = ErrorReporter::new()?;

    info!(
        batch_id = %batch.batch_id,
        "Starting batch processing"
    );

    // Step 1: Scan directory for DBF files
    batch.status = BatchStatus::Scanning;
    let dbf_files = match scan_directory(&config.src.source_dir) {
        Ok(files) => files,
        Err(e) => {
            error!(
                batch_id = %batch.batch_id,
                error = %e,
                "Failed to scan directory"
            );
            batch.status = BatchStatus::Aborted;
            return Err(e);
        }
    };

    info!(
        batch_id = %batch.batch_id,
        file_count = dbf_files.len(),
        "Scan complete"
    );

    if dbf_files.is_empty() {
        info!(
            batch_id = %batch.batch_id,
            "No DBF files found, batch complete"
        );
        batch.status = BatchStatus::Completed;
        return Ok(batch);
    }

    // Step 2: Process each file (convert → compress → upload)
    batch.status = BatchStatus::Processing;
    for dbf_file in dbf_files {
        let file_path = dbf_file.path.clone();

        debug!(
            batch_id = %batch.batch_id,
            file = %file_path.display(),
            "Processing file"
        );

        // Process the file, handling locked files specially
        match process_single_file(&dbf_file, &config, &token_manager, &error_reporter).await {
            Ok(_) => {
                batch.mark_completed();
                info!(
                    batch_id = %batch.batch_id,
                    file = %file_path.display(),
                    "File processed successfully"
                );
            }
            Err(e) if is_file_locked(&e) => {
                warn!(
                    batch_id = %batch.batch_id,
                    file = %file_path.display(),
                    "File is locked, deferring to end of batch"
                );
                batch.defer_locked_file(file_path);
            }
            Err(e) => {
                error!(
                    batch_id = %batch.batch_id,
                    file = %file_path.display(),
                    error = %e,
                    "File processing failed"
                );
                batch.mark_failed();

                // Report error to server or log locally
                report_processing_error(
                    &dbf_file.path,
                    &e,
                    &token_manager,
                    &error_reporter,
                    &config,
                )
                .await;
            }
        }
    }

    // Step 3: Retry locked files
    if !batch.locked_files.is_empty() {
        info!(
            batch_id = %batch.batch_id,
            locked_count = batch.locked_files.len(),
            "Retrying locked files"
        );
        batch.status = BatchStatus::RetryingLocked;

        let locked_files_clone = batch.locked_files.clone();
        for file_path in locked_files_clone {
            debug!(
                batch_id = %batch.batch_id,
                file = %file_path.display(),
                "Retrying locked file"
            );

            // Recreate DbfFile for retry
            let dbf_file = crate::models::DbfFile::new(file_path.clone(), &config.src.source_dir);

            match process_single_file(&dbf_file, &config, &token_manager, &error_reporter).await {
                Ok(_) => {
                    batch.mark_completed();
                    info!(
                        batch_id = %batch.batch_id,
                        file = %file_path.display(),
                        "Locked file processed successfully on retry"
                    );
                }
                Err(e) if is_file_locked(&e) => {
                    warn!(
                        batch_id = %batch.batch_id,
                        file = %file_path.display(),
                        "File still locked after retry, skipping"
                    );
                    batch.mark_failed();
                }
                Err(e) => {
                    error!(
                        batch_id = %batch.batch_id,
                        file = %file_path.display(),
                        error = %e,
                        "Locked file processing failed on retry"
                    );
                    batch.mark_failed();

                    report_processing_error(
                        &file_path,
                        &e,
                        &token_manager,
                        &error_reporter,
                        &config,
                    )
                    .await;
                }
            }
        }
    }

    // Step 4: Complete batch
    batch.status = BatchStatus::Completed;
    info!(
        batch_id = %batch.batch_id,
        processed = batch.processed_count,
        failed = batch.failed_count,
        "Batch processing complete"
    );

    Ok(batch)
}

/// Process a single DBF file through the complete pipeline
async fn process_single_file(
    dbf_file: &crate::models::DbfFile,
    config: &Config,
    token_manager: &Arc<TokenManager>,
    _error_reporter: &ErrorReporter,
) -> Result<()> {
    // Step 1: Convert DBF to CSV
    let csv_path = convert_dbf_to_csv(dbf_file, config)?;

    // Step 2: Compress CSV to gzip
    let compressed_filename = dbf_file.generate_compressed_filename();
    let gzip_path = compress_csv(csv_path.clone(), compressed_filename.clone())?;

    // Step 3: Upload compressed file
    let token = token_manager.get_valid_token().await?;
    upload_file(gzip_path, compressed_filename, &token, config).await?;

    // Step 4: Cleanup - delete CSV file (keep source DBF, delete gzip after upload)
    cleanup_intermediate_files(&csv_path)?;

    Ok(())
}

/// Cleanup intermediate CSV files after processing
fn cleanup_intermediate_files(csv_path: &PathBuf) -> Result<()> {
    debug!(file = %csv_path.display(), "Cleaning up CSV file");

    match std::fs::remove_file(csv_path) {
        Ok(_) => {
            debug!(file = %csv_path.display(), "CSV file deleted");
            Ok(())
        }
        Err(e) => {
            warn!(
                file = %csv_path.display(),
                error = %e,
                "Failed to delete CSV file, continuing anyway"
            );
            // Don't fail the batch just because cleanup failed
            Ok(())
        }
    }
}

/// Check if an error is due to a locked file
fn is_file_locked(error: &ProcessingError) -> bool {
    match error {
        ProcessingError::FileReadError(io_error) => {
            matches!(
                io_error.kind(),
                ErrorKind::PermissionDenied | ErrorKind::WouldBlock
            )
        }
        _ => false,
    }
}

/// Report a processing error to the remote API or log locally
async fn report_processing_error(
    file_path: &std::path::Path,
    error: &ProcessingError,
    token_manager: &Arc<TokenManager>,
    error_reporter: &ErrorReporter,
    config: &Config,
) {
    let error_report = ErrorReport::new(
        file_path.to_string_lossy().to_string(),
        error.error_type().to_string(),
        error.to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );

    // Try to get a token, but continue even if we can't
    let token = token_manager.get_valid_token().await.ok();

    // Try to send error report to server
    match error_reporter
        .send_error_report(&error_report, token.as_ref(), config)
        .await
    {
        Ok(_) => {
            debug!(
                file = %file_path.display(),
                "Error report sent to server"
            );
        }
        Err(e) => {
            warn!(
                file = %file_path.display(),
                error = %e,
                "Failed to send error report to server, logging locally"
            );

            // Fallback to local logging
            if let Err(log_err) = log_error_locally(
                &error_report,
                &format!("Failed to send to server: {}", e),
                None,
            ) {
                error!(
                    file = %file_path.display(),
                    error = %log_err,
                    "Failed to log error locally"
                );
            }
        }
    }
}
