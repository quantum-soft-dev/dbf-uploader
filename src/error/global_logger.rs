// Global error logger - utility for reporting system-level errors
use crate::auth::TokenManager;
use crate::error::reporter::ErrorReporter;
use crate::error::{ProcessingError, Result};
use crate::models::{Config, ErrorSeverity, GlobalErrorReport};
use std::collections::HashMap;
use std::sync::Arc;

/// Global error logger for system-level errors
/// This provides a high-level API for reporting errors outside batch context
pub struct GlobalErrorLogger {
    reporter: Arc<ErrorReporter>,
    token_manager: Arc<TokenManager>,
    config: Arc<Config>,
}

impl GlobalErrorLogger {
    /// Create a new global error logger
    pub fn new(
        reporter: Arc<ErrorReporter>,
        token_manager: Arc<TokenManager>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            reporter,
            token_manager,
            config,
        }
    }

    /// Report a global error with custom severity and metadata
    pub async fn report(
        &self,
        error_type: String,
        message: String,
        severity: ErrorSeverity,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        let error_report = if let Some(meta) = metadata {
            GlobalErrorReport::with_metadata(error_type.clone(), message.clone(), severity, meta)
        } else {
            GlobalErrorReport::new(error_type.clone(), message.clone(), severity)
        };

        // Get a valid JWT token
        let token = match self.token_manager.get_valid_token().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to get JWT token for global error reporting, logging locally only"
                );
                return Err(e);
            }
        };

        // Send the error report
        match self
            .reporter
            .send_global_error(error_report, &token, &self.config)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    error_type = %error_type,
                    "Failed to send global error report, logged locally"
                );
                Err(e)
            }
        }
    }

    /// Report a CRITICAL error
    pub async fn critical(
        &self,
        error_type: String,
        message: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        tracing::error!(
            severity = "CRITICAL",
            error_type = %error_type,
            message = %message,
            "Critical error occurred"
        );
        self.report(error_type, message, ErrorSeverity::Critical, metadata)
            .await
    }

    /// Report an ERROR
    pub async fn error(
        &self,
        error_type: String,
        message: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        tracing::error!(
            severity = "ERROR",
            error_type = %error_type,
            message = %message,
            "Error occurred"
        );
        self.report(error_type, message, ErrorSeverity::Error, metadata)
            .await
    }

    /// Report a WARNING
    pub async fn warning(
        &self,
        error_type: String,
        message: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        tracing::warn!(
            severity = "WARNING",
            error_type = %error_type,
            message = %message,
            "Warning occurred"
        );
        self.report(error_type, message, ErrorSeverity::Warning, metadata)
            .await
    }

    /// Report an INFO message
    pub async fn info(
        &self,
        error_type: String,
        message: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        tracing::info!(
            severity = "INFO",
            error_type = %error_type,
            message = %message,
            "Info message"
        );
        self.report(error_type, message, ErrorSeverity::Info, metadata)
            .await
    }

    /// Report a ProcessingError as a global error
    pub async fn report_processing_error(
        &self,
        error: &ProcessingError,
        context: Option<&str>,
    ) -> Result<()> {
        let (error_type, severity) = match error {
            ProcessingError::AuthenticationError(_) => {
                ("AUTHENTICATION_FAILED".to_string(), ErrorSeverity::Critical)
            }
            ProcessingError::NetworkError(_) => {
                ("CONNECTION_FAILED".to_string(), ErrorSeverity::Error)
            }
            ProcessingError::ConfigurationError(_) => {
                ("CONFIG_ERROR".to_string(), ErrorSeverity::Critical)
            }
            ProcessingError::FileReadError(_) => ("FILE_ERROR".to_string(), ErrorSeverity::Error),
            _ => ("UNKNOWN_ERROR".to_string(), ErrorSeverity::Error),
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "errorSource".to_string(),
            serde_json::Value::String("data_exporter".to_string()),
        );

        if let Some(ctx) = context {
            metadata.insert(
                "context".to_string(),
                serde_json::Value::String(ctx.to_string()),
            );
        }

        self.report(
            error_type,
            error.to_string(),
            severity,
            Some(metadata),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_global_error_logger_creation() {
        // This test just ensures the types compile correctly
        // Actual functionality testing requires integration tests with real endpoints
    }
}
