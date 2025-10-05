// Error reporting module - sends error reports to the remote API
use crate::auth::JwtToken;
use crate::error::{ProcessingError, Result};
use crate::models::{Config, ErrorReport};
use reqwest::Client;
use std::time::Duration;

pub struct ErrorReporter {
    client: Client,
}

impl ErrorReporter {
    /// Create a new error reporter
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(true)
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client })
    }

    /// Send error report to the remote API
    /// This is a fire-and-forget operation - we never retry to avoid infinite loops
    /// If sending fails, the error will be logged locally instead
    pub async fn send_error_report(
        &self,
        error_report: &ErrorReport,
        token: Option<&JwtToken>,
        config: &Config,
    ) -> Result<()> {
        let url = format!("{}/api/errors/report", config.api.base_url);

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
        match response.status().as_u16() {
            200 | 201 | 204 => Ok(()),
            401 => Err(ProcessingError::AuthenticationError(
                "Unauthorized - token may be invalid".to_string(),
            )),
            status => Err(ProcessingError::NetworkError(format!(
                "Error report API returned status: {}",
                status
            ))),
        }
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new().expect("Failed to create error reporter")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_reporter_creation() {
        let reporter = ErrorReporter::new();
        assert!(reporter.is_ok());
    }

    #[test]
    fn test_error_reporter_default() {
        let _reporter = ErrorReporter::default();
        // Should not panic
    }
}
