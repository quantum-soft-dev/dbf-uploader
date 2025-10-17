use super::Batch;
use super::dto::{BatchResponseDto, UploadResponse};
use crate::auth::TokenManager;
use crate::error::{ProcessingError, Result};
use reqwest::{multipart, Client};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info, warn};

/// BatchManager manages the lifecycle of batch upload sessions
pub struct BatchManager {
    /// HTTP client for API requests
    http_client: Client,
    /// Authentication token manager
    token_manager: Arc<TokenManager>,
    /// Base URL for API endpoints
    base_url: String,
    /// Currently active batch
    current_batch: Arc<Mutex<Option<Batch>>>,
}

impl BatchManager {
    /// Create a new BatchManager with configurable timeout
    pub fn new(base_url: String, token_manager: Arc<TokenManager>, http_timeout_secs: u64) -> Result<Self> {
        // Allow HTTP for localhost (for testing), otherwise require HTTPS
        let is_localhost = base_url.starts_with("http://localhost")
            || base_url.starts_with("http://127.0.0.1")
            || base_url.starts_with("http://[::1]");

        let http_client = Client::builder()
            .timeout(Duration::from_secs(http_timeout_secs))
            .https_only(!is_localhost) // Allow HTTP for localhost testing
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            http_client,
            token_manager,
            base_url,
            current_batch: Arc::new(Mutex::new(None)),
        })
    }

    /// Start a new batch upload session
    pub async fn start_batch(&self) -> Result<Batch> {
        // Check if there's already an active batch
        {
            let current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;

            if let Some(ref batch) = *current {
                if batch.state.is_active() {
                    return Err(ProcessingError::BatchError(format!(
                        "Batch {} is already active",
                        batch.id
                    )));
                }
            }
        }

        // Get valid authentication token
        let token = self.token_manager.get_valid_token().await?;

        // Call middleware batch start endpoint
        let url = format!("{}/api/dfc/batch/start", self.base_url);

        debug!("Starting new batch at {}", url);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to send batch start request: {}", e))
            })?;

        // Handle response
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProcessingError::BatchError(format!(
                "Batch start failed with status {}: {}",
                status, error_body
            )));
        }

        let start_response: BatchResponseDto = response.json().await.map_err(|e| {
            ProcessingError::BatchError(format!("Failed to parse batch start response: {}", e))
        })?;

        // Create Batch instance
        let mut batch = Batch::new(start_response.batch_id);
        batch.mark_started().map_err(|e| {
            ProcessingError::BatchError(format!("Failed to mark batch as started: {}", e))
        })?;

        info!("Batch {} started successfully", batch.id);

        // Store current batch
        {
            let mut current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;
            *current = Some(batch.clone());
        }

        Ok(batch)
    }

    /// Upload files to the current batch
    pub async fn upload_files(&self, file_paths: Vec<PathBuf>) -> Result<UploadResponse> {
        // Get current batch
        let batch_id = {
            let current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;

            match *current {
                Some(ref batch) if batch.is_in_progress() => batch.id,
                Some(ref batch) => {
                    return Err(ProcessingError::BatchError(format!(
                        "Batch {} is not in progress (state: {})",
                        batch.id, batch.state
                    )));
                }
                None => {
                    return Err(ProcessingError::BatchError(
                        "No active batch".to_string()
                    ));
                }
            }
        };

        // Get valid token
        let token = self.token_manager.get_valid_token().await?;

        // Build multipart form
        let mut form = multipart::Form::new();

        for file_path in file_paths.iter() {
            // Read file
            let mut file = File::open(file_path).await.map_err(|e| {
                ProcessingError::FileReadError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Failed to open file {:?}: {}", file_path, e),
                ))
            })?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await.map_err(|e| {
                ProcessingError::FileReadError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to read file {:?}: {}", file_path, e),
                ))
            })?;

            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    ProcessingError::BatchError(format!(
                        "Invalid filename: {:?}",
                        file_path
                    ))
                })?
                .to_string();

            let part = multipart::Part::bytes(buffer)
                .file_name(filename)
                .mime_str("application/gzip")
                .map_err(|e| {
                    ProcessingError::BatchError(format!("Failed to create multipart: {}", e))
                })?;

            form = form.part("files", part);
        }

        // Call middleware upload endpoint
        let url = format!("{}/api/dfc/batch/{}/upload", self.base_url, batch_id);

        debug!("Uploading {} files to batch {}", file_paths.len(), batch_id);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to send upload request: {}", e))
            })?;

        // Handle response
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProcessingError::UploadError(format!(
                "Upload failed with status {}: {}",
                status, error_body
            )));
        }

        let upload_response: UploadResponse = response.json().await.map_err(|e| {
            ProcessingError::BatchError(format!("Failed to parse upload response: {}", e))
        })?;

        // Update batch statistics
        {
            let mut current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;

            if let Some(ref mut batch) = *current {
                for file_info in &upload_response.files {
                    batch.add_uploaded_file(file_info.file_size);
                }
            }
        }

        // Calculate total size from files
        let total_size: u64 = upload_response.files.iter().map(|f| f.file_size).sum();

        info!(
            "Uploaded {} files to batch {}, total size: {} bytes",
            upload_response.files.len(),
            batch_id,
            total_size
        );

        Ok(upload_response)
    }

    /// Complete the current batch
    pub async fn complete_batch(&self) -> Result<BatchResponseDto> {
        // Get current batch
        let batch_id = {
            let current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;

            match *current {
                Some(ref batch) if batch.is_in_progress() => batch.id,
                Some(ref batch) => {
                    return Err(ProcessingError::BatchError(format!(
                        "Batch {} is not in progress (state: {})",
                        batch.id, batch.state
                    )));
                }
                None => {
                    return Err(ProcessingError::BatchError(
                        "No active batch".to_string()
                    ));
                }
            }
        };

        // Get valid token
        let token = self.token_manager.get_valid_token().await?;

        // Call middleware complete endpoint
        let url = format!("{}/api/dfc/batch/{}/complete", self.base_url, batch_id);

        debug!("Completing batch {}", batch_id);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to send complete request: {}", e))
            })?;

        // Handle response
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProcessingError::BatchError(format!(
                "Batch complete failed with status {}: {}",
                status, error_body
            )));
        }

        let complete_response: BatchResponseDto = response.json().await.map_err(|e| {
            ProcessingError::BatchError(format!("Failed to parse complete response: {}", e))
        })?;

        // Update batch state
        {
            let mut current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;

            if let Some(ref mut batch) = *current {
                batch.mark_completed().map_err(|e| {
                    ProcessingError::BatchError(format!("Failed to mark batch as completed: {}", e))
                })?;

                let duration = batch.duration().map(|d| d.num_seconds()).unwrap_or(0);
                info!(
                    "Batch {} completed successfully in {}s, {} files, {} bytes",
                    batch.id,
                    duration,
                    batch.uploaded_files_count,
                    batch.total_size_bytes
                );
            }

            // Clear current batch
            *current = None;
        }

        Ok(complete_response)
    }

    /// Mark the current batch as failed
    pub async fn fail_batch(&self, reason: &str) -> Result<()> {
        // Get current batch
        let batch_id = {
            let current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;

            match *current {
                Some(ref batch) if batch.is_in_progress() => batch.id,
                Some(ref batch) => {
                    warn!("Batch {} is not in progress (state: {}), cannot fail", batch.id, batch.state);
                    return Ok(());
                }
                None => {
                    warn!("No active batch to fail");
                    return Ok(());
                }
            }
        };

        // Get valid token
        let token = self.token_manager.get_valid_token().await?;

        // Call middleware fail endpoint
        let url = format!("{}/api/dfc/batch/{}/fail", self.base_url, batch_id);

        warn!("Failing batch {}: {}", batch_id, reason);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send fail request for reason '{}': {}", reason, e);
                ProcessingError::NetworkError(format!("Failed to send fail request: {}", e))
            })?;

        // Handle response
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!("Batch fail request failed with status {}: {}", status, error_body);
        }

        // Update batch state
        {
            let mut current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;

            if let Some(ref mut batch) = *current {
                let _ = batch.mark_failed();
            }

            // Clear current batch
            *current = None;
        }

        Ok(())
    }

    /// Cancel the current batch
    pub async fn cancel_batch(&self) -> Result<()> {
        // Get current batch
        let batch_id = {
            let current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;

            match *current {
                Some(ref batch) if batch.is_in_progress() => batch.id,
                Some(ref batch) => {
                    warn!("Batch {} is not in progress (state: {}), cannot cancel", batch.id, batch.state);
                    return Ok(());
                }
                None => {
                    warn!("No active batch to cancel");
                    return Ok(());
                }
            }
        };

        // Get valid token
        let token = self.token_manager.get_valid_token().await?;

        // Call middleware cancel endpoint
        let url = format!("{}/api/dfc/batch/{}/cancel", self.base_url, batch_id);

        info!("Cancelling batch {}", batch_id);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send cancel request: {}", e);
                ProcessingError::NetworkError(format!("Failed to send cancel request: {}", e))
            })?;

        // Handle response
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!("Batch cancel request failed with status {}: {}", status, error_body);
        }

        // Update batch state
        {
            let mut current = self.current_batch.lock().map_err(|e| {
                ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
            })?;

            if let Some(ref mut batch) = *current {
                let _ = batch.mark_cancelled();
            }

            // Clear current batch
            *current = None;
        }

        Ok(())
    }

    /// Get the current batch if one exists
    pub fn get_current_batch(&self) -> Result<Option<Batch>> {
        let current = self.current_batch.lock().map_err(|e| {
            ProcessingError::BatchError(format!("Failed to acquire batch lock: {}", e))
        })?;

        Ok(current.clone())
    }
}
