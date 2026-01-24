// Error reporting module - sends error reports to the remote API
use crate::auth::JwtToken;
use crate::error::{ProcessingError, Result};
use crate::models::{Config, ErrorReport, GlobalErrorReport};
use reqwest::Client;
use std::time::Duration;

pub struct ErrorReporter {
    client: Client,
}

impl ErrorReporter {
    /// Create a new error reporter with https_only setting from config
    pub fn new(https_only: bool) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(https_only)
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client })
    }

    /// Send batch error report to the remote API
    /// This is a fire-and-forget operation - we never retry to avoid infinite loops
    /// If sending fails, the error will be logged locally instead
    pub async fn send_error_report(
        &self,
        error_report: &ErrorReport,
        batch_id: Option<&str>,
        token: Option<&JwtToken>,
        config: &Config,
    ) -> Result<()> {
        // Use batch-specific endpoint if batch_id provided, otherwise standalone
        let url = if let Some(bid) = batch_id {
            format!(
                "{}/api/v1/device/errors/batches/{}",
                config.api.base_url, bid
            )
        } else {
            format!("{}/api/v1/device/errors", config.api.base_url)
        };

        tracing::debug!(
            url = %url,
            error_type = %error_report.error_type,
            has_metadata = error_report.metadata.is_some(),
            "Sending error report to server"
        );

        // Build the request
        let mut request = self.client.post(&url).json(error_report);

        // Add JWT token if available (server may allow unauthenticated error reports)
        if let Some(jwt) = token {
            request = request.header("Authorization", format!("Bearer {}", jwt.token));
        }

        // Send the request (fire-and-forget)
        let response = request.send().await.map_err(|e| {
            ProcessingError::NetworkError(format!("Failed to send error report: {}", e))
        })?;

        // Check response status
        let status = response.status().as_u16();

        match status {
            200 | 201 | 204 => Ok(()),
            401 => Err(ProcessingError::AuthenticationError(
                "Unauthorized - token may be invalid".to_string(),
            )),
            _ => {
                // Try to get response body for better error diagnostics
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unable to read response body".to_string());

                tracing::error!(
                    status = status,
                    url = %url,
                    response_body = %error_body,
                    "Error report API returned error"
                );

                Err(ProcessingError::NetworkError(format!(
                    "Error report API returned status: {} - {}",
                    status, error_body
                )))
            }
        }
    }

    /// Send global error report to the remote API (POST /api/dfc/error)
    /// This is for errors outside batch processing context (system-level errors)
    /// This is a fire-and-forget operation - we never retry to avoid infinite loops
    /// If sending fails, the error will be logged locally instead
    pub async fn send_global_error(
        &self,
        mut error_report: GlobalErrorReport,
        token: &JwtToken,
        config: &Config,
    ) -> Result<()> {
        let url = format!("{}/api/dfc/error", config.api.base_url);

        // Validate and truncate fields according to API limits
        error_report.validate_and_truncate();

        tracing::debug!(
            url = %url,
            error_type = %error_report.error_type,
            severity = ?error_report.severity,
            has_metadata = error_report.metadata.is_some(),
            "Sending global error report to server"
        );

        // Send the request with JWT token
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .json(&error_report)
            .send()
            .await
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to send global error report: {}", e))
            })?;

        // Check response status
        let status = response.status().as_u16();

        match status {
            204 => {
                tracing::info!(
                    error_type = %error_report.error_type,
                    severity = ?error_report.severity,
                    "Global error report sent successfully"
                );
                Ok(())
            }
            401 => Err(ProcessingError::AuthenticationError(
                "Unauthorized - token may be invalid".to_string(),
            )),
            400 => {
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unable to read response body".to_string());

                tracing::error!(
                    status = status,
                    url = %url,
                    response_body = %error_body,
                    "Global error report validation failed"
                );

                Err(ProcessingError::NetworkError(format!(
                    "Global error report validation failed: {}",
                    error_body
                )))
            }
            _ => {
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unable to read response body".to_string());

                tracing::error!(
                    status = status,
                    url = %url,
                    response_body = %error_body,
                    "Global error report API returned error"
                );

                Err(ProcessingError::NetworkError(format!(
                    "Global error report API returned status: {} - {}",
                    status, error_body
                )))
            }
        }
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new(true).expect("Failed to create error reporter")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_reporter_creation() {
        let reporter = ErrorReporter::new(true);
        assert!(reporter.is_ok());
    }

    #[test]
    fn test_error_reporter_default() {
        let _reporter = ErrorReporter::default();
        // Should not panic
    }
}
