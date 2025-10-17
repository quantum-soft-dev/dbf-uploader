// Error reporting module - sends error reports to the middleware API
use crate::auth::TokenManager;
use crate::error::dto::ErrorLogRequest;
use crate::error::{ProcessingError, Result};
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, warn};
use uuid::Uuid;

const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ErrorReporter {
    /// HTTP client for API requests
    client: Client,
    /// Authentication token manager
    token_manager: Option<Arc<TokenManager>>,
    /// Base URL for API endpoints
    base_url: String,
    /// Local log file path for fallback
    local_log_path: PathBuf,
}

impl ErrorReporter {
    /// Create a new error reporter with authentication
    pub fn new(
        base_url: String,
        token_manager: Arc<TokenManager>,
        local_log_path: PathBuf,
    ) -> Result<Self> {
        // Allow HTTP for localhost (for testing), otherwise require HTTPS
        let is_localhost = base_url.starts_with("http://localhost")
            || base_url.starts_with("http://127.0.0.1")
            || base_url.starts_with("http://[::1]");

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(!is_localhost)
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            token_manager: Some(token_manager),
            base_url,
            local_log_path,
        })
    }

    /// Create error reporter without authentication (for standalone errors)
    pub fn without_auth(base_url: String, local_log_path: PathBuf) -> Result<Self> {
        // Allow HTTP for localhost (for testing), otherwise require HTTPS
        let is_localhost = base_url.starts_with("http://localhost")
            || base_url.starts_with("http://127.0.0.1")
            || base_url.starts_with("http://[::1]");

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(!is_localhost)
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            token_manager: None,
            base_url,
            local_log_path,
        })
    }

    /// Report error associated with a batch
    pub async fn report_batch_error(
        &self,
        batch_id: Uuid,
        error: &ProcessingError,
    ) -> Result<()> {
        let error_request = ErrorLogRequest::new(
            error.error_type().to_string(),
            error.to_string(),
        )
        .with_client_version(CLIENT_VERSION.to_string());

        // Validate request
        if let Err(e) = error_request.validate() {
            warn!("Invalid error request: {}", e);
            return self.log_to_local_file(&error_request).await;
        }

        // Try to send to middleware
        let url = format!("{}/api/v1/error/{}", self.base_url, batch_id);

        debug!("Reporting batch error to {}", url);

        let result = self.send_with_auth(&url, &error_request).await;

        match result {
            Ok(_) => {
                debug!("Error reported successfully for batch {}", batch_id);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to report error to middleware: {}", e);
                // Fallback to local log
                self.log_to_local_file(&error_request).await
            }
        }
    }

    /// Report standalone error (not associated with a batch)
    pub async fn report_standalone_error(&self, error: &ProcessingError) -> Result<()> {
        let error_request = ErrorLogRequest::new(
            error.error_type().to_string(),
            error.to_string(),
        )
        .with_client_version(CLIENT_VERSION.to_string());

        // Validate request
        if let Err(e) = error_request.validate() {
            warn!("Invalid error request: {}", e);
            return self.log_to_local_file(&error_request).await;
        }

        // Try to send to middleware
        let url = format!("{}/api/v1/error", self.base_url);

        debug!("Reporting standalone error to {}", url);

        let result = self.send_with_auth(&url, &error_request).await;

        match result {
            Ok(_) => {
                debug!("Standalone error reported successfully");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to report error to middleware: {}", e);
                // Fallback to local log
                self.log_to_local_file(&error_request).await
            }
        }
    }

    /// Send error request with authentication (if available)
    async fn send_with_auth(&self, url: &str, request: &ErrorLogRequest) -> Result<()> {
        let mut req = self.client.post(url).json(request);

        // Add authentication if available
        if let Some(ref token_manager) = self.token_manager {
            match token_manager.get_valid_token().await {
                Ok(token) => {
                    req = req.header("Authorization", format!("Bearer {}", token.token));
                }
                Err(e) => {
                    warn!("Failed to get auth token for error reporting: {}", e);
                    // Continue without auth - middleware may allow unauthenticated error reports
                }
            }
        }

        // Send request
        let response = req.send().await.map_err(|e| {
            ProcessingError::NetworkError(format!("Failed to send error report: {}", e))
        })?;

        // Check response status
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let error_body = response.text().await.unwrap_or_default();
            Err(ProcessingError::NetworkError(format!(
                "Error reporting failed with status {}: {}",
                status, error_body
            )))
        }
    }

    /// Log error to local file as fallback
    async fn log_to_local_file(&self, request: &ErrorLogRequest) -> Result<()> {
        use chrono::Utc;
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;

        let timestamp = Utc::now().to_rfc3339();
        let log_entry = format!(
            "[{}] {} - {}\n",
            timestamp, request.error_type, request.message
        );

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.local_log_path)
            .await
        {
            Ok(mut file) => {
                if let Err(e) = file.write_all(log_entry.as_bytes()).await {
                    error!("Failed to write to local error log: {}", e);
                    return Err(ProcessingError::FileReadError(e));
                }
                debug!("Error logged locally to {:?}", self.local_log_path);
                Ok(())
            }
            Err(e) => {
                error!("Failed to open local error log file: {}", e);
                Err(ProcessingError::FileReadError(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_error_reporter_without_auth() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("error.log");

        let reporter = ErrorReporter::without_auth(
            "https://api.example.com".to_string(),
            log_path,
        );

        assert!(reporter.is_ok());
    }

    #[tokio::test]
    async fn test_log_to_local_file() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("error.log");

        let reporter = ErrorReporter::without_auth(
            "https://api.example.com".to_string(),
            log_path.clone(),
        )
        .unwrap();

        let request = ErrorLogRequest::new(
            "TestError".to_string(),
            "Test message".to_string(),
        );

        let result = reporter.log_to_local_file(&request).await;
        assert!(result.is_ok());

        // Verify file was created and contains the log entry
        let contents = tokio::fs::read_to_string(&log_path).await.unwrap();
        assert!(contents.contains("TestError"));
        assert!(contents.contains("Test message"));
    }

    #[test]
    fn test_client_version() {
        assert!(!CLIENT_VERSION.is_empty());
    }
}
