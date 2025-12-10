// File processing module
pub mod batch_client;
pub mod compressor;
pub mod converter;
pub mod data;
pub mod scanner;
pub mod uploader;

pub use batch_client::BatchClient;
pub use compressor::compress_csv;
pub use converter::convert_dbf_to_csv;
pub use data::ProcessingData;
pub use scanner::scan_directory;
pub use uploader::upload_file;

use crate::auth::TokenManager;
use crate::error::{log_error_locally, ErrorReporter, ProcessingError, Result};
use crate::models::{Batch, BatchStatus, Config, ErrorReport};
use crate::vss;
use std::io::ErrorKind;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Run a complete batch processing cycle
/// This orchestrates: scan → convert → compress → upload for all DBF files
pub async fn run_batch(config: Config, token_manager: Arc<TokenManager>) -> Result<Batch> {
    let mut batch = Batch::new(config.clone());
    let error_reporter = ErrorReporter::new(config.api.https_only)?;
    let batch_client = BatchClient::new(config.api.https_only)?;

    // Start batch on server and get batch ID
    let token = token_manager.get_valid_token().await?;
    let server_batch_id = match batch_client.start_batch(&token, &config).await {
        Ok(id) => id,
        Err(e) => {
            error!(error = %e, "Failed to start batch on server");
            batch.status = BatchStatus::Aborted;
            return Err(e);
        }
    };

    info!(
        batch_id = %batch.batch_id,
        server_batch_id = %server_batch_id,
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
        match process_single_file(
            &dbf_file,
            &server_batch_id,
            &config,
            &token_manager,
            &error_reporter,
        )
        .await
        {
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
                    Some(&server_batch_id),
                    &token_manager,
                    &error_reporter,
                    &config,
                )
                .await;
            }
        }
    }

    // Step 3: Retry locked files using VSS (rawcopy)
    if !batch.locked_files.is_empty() {
        info!(
            batch_id = %batch.batch_id,
            locked_count = batch.locked_files.len(),
            "Retrying locked files using VSS (rawcopy)"
        );
        batch.status = BatchStatus::RetryingLocked;

        // Create temp directory for VSS copies
        let temp_dir = std::env::temp_dir().join(format!("dbf_vss_{}", batch.batch_id));

        let locked_files_clone = batch.locked_files.clone();
        for file_path in locked_files_clone {
            debug!(
                batch_id = %batch.batch_id,
                file = %file_path.display(),
                "Processing locked file via VSS copy"
            );

            // Try to copy locked file via VSS
            match vss::copy_locked_file(&file_path, &temp_dir) {
                Ok(vss_copy_path) => {
                    // Create DbfFile pointing to the VSS copy
                    let vss_dbf_file =
                        crate::models::DbfFile::new(vss_copy_path.clone(), &temp_dir);

                    // Process the VSS copy
                    match process_single_file(
                        &vss_dbf_file,
                        &server_batch_id,
                        &config,
                        &token_manager,
                        &error_reporter,
                    )
                    .await
                    {
                        Ok(_) => {
                            batch.mark_completed();
                            info!(
                                batch_id = %batch.batch_id,
                                file = %file_path.display(),
                                "Locked file processed successfully via VSS copy"
                            );

                            // Clean up VSS copy
                            if let Err(e) = std::fs::remove_file(&vss_copy_path) {
                                warn!(
                                    "Failed to delete VSS copy {}: {}",
                                    vss_copy_path.display(),
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            error!(
                                batch_id = %batch.batch_id,
                                file = %file_path.display(),
                                error = %e,
                                "Failed to process locked file via VSS copy"
                            );
                            batch.mark_failed();

                            // Clean up VSS copy
                            let _ = std::fs::remove_file(&vss_copy_path);

                            report_processing_error(
                                &file_path,
                                &e,
                                Some(&server_batch_id),
                                &token_manager,
                                &error_reporter,
                                &config,
                            )
                            .await;
                        }
                    }
                }
                Err(e) => {
                    error!(
                        batch_id = %batch.batch_id,
                        file = %file_path.display(),
                        error = %e,
                        "Failed to copy locked file via VSS"
                    );
                    batch.mark_failed();

                    // Report VSS copy error
                    let vss_error = ProcessingError::VssError(e.to_string());
                    report_processing_error(
                        &file_path,
                        &vss_error,
                        Some(&server_batch_id),
                        &token_manager,
                        &error_reporter,
                        &config,
                    )
                    .await;
                }
            }
        }

        // Clean up temp directory
        if temp_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
                warn!(
                    "Failed to remove VSS temp directory {}: {}",
                    temp_dir.display(),
                    e
                );
            }
        }
    }

    // Step 4: Complete or fail batch on server
    let token = token_manager.get_valid_token().await?;
    if batch.failed_count > 0 {
        // Mark batch as failed if any files failed
        if let Err(e) = batch_client
            .fail_batch(&server_batch_id, &token, &config)
            .await
        {
            warn!(error = %e, "Failed to mark batch as failed on server");
        }
    } else {
        // Mark batch as completed
        if let Err(e) = batch_client
            .complete_batch(&server_batch_id, &token, &config)
            .await
        {
            warn!(error = %e, "Failed to mark batch as completed on server");
        }
    }

    batch.status = BatchStatus::Completed;
    info!(
        batch_id = %batch.batch_id,
        server_batch_id = %server_batch_id,
        processed = batch.processed_count,
        failed = batch.failed_count,
        "Batch processing complete"
    );

    Ok(batch)
}

/// Process a single DBF file through the complete pipeline (in-memory)
async fn process_single_file(
    dbf_file: &crate::models::DbfFile,
    server_batch_id: &str,
    config: &Config,
    token_manager: &Arc<TokenManager>,
    _error_reporter: &ErrorReporter,
) -> Result<()> {
    use crate::processor::compressor::compress_csv_memory;
    use crate::processor::converter::convert_dbf_to_csv_memory;
    use crate::processor::uploader::upload_data;

    // Step 1: Convert DBF to CSV (in memory or temp file if too large)
    let csv_data = convert_dbf_to_csv_memory(dbf_file, config)?;

    // Step 2: Compress CSV to gzip (in memory or temp file)
    let gzip_data = compress_csv_memory(csv_data)?;

    // Step 3: Upload compressed data with batch ID
    let compressed_filename = dbf_file.generate_compressed_filename();
    let token = token_manager.get_valid_token().await?;
    upload_data(
        gzip_data,
        compressed_filename,
        server_batch_id,
        &token,
        config,
    )
    .await?;

    // Step 4: Cleanup happens automatically when ProcessingData drops
    // Temp files (if any) are deleted automatically

    Ok(())
}

// Note: Cleanup is now automatic via Drop trait in ProcessingData
// No manual cleanup needed - temp files are deleted when ProcessingData goes out of scope

/// Check if an error is due to a locked file
fn is_file_locked(error: &ProcessingError) -> bool {
    match error {
        ProcessingError::FileReadError(io_error) => {
            // Check for Windows-specific error codes for file locking
            // Error 32: The process cannot access the file because it is being used by another process
            // Error 33: The process cannot access the file because another process has locked a portion of the file
            if let Some(os_error) = io_error.raw_os_error() {
                if os_error == 32 || os_error == 33 {
                    return true;
                }
            }

            // Also check standard error kinds
            matches!(
                io_error.kind(),
                ErrorKind::PermissionDenied | ErrorKind::WouldBlock
            )
        }
        ProcessingError::ConversionError(msg) => {
            // Check if conversion error is due to file access issues
            msg.contains("Failed to open DBF file")
                && (msg.contains("os error 32") || msg.contains("os error 33"))
        }
        _ => false,
    }
}

/// Report a processing error to the remote API or log locally
async fn report_processing_error(
    file_path: &std::path::Path,
    error: &ProcessingError,
    batch_id: Option<&str>,
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
        .send_error_report(&error_report, batch_id, token.as_ref(), config)
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
