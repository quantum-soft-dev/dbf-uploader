// Batch API client for managing batch lifecycle
use common::auth::JwtToken;
use common::error::{ProcessingError, Result};
use common::models::Config;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, info};

#[derive(Debug, Deserialize)]
struct StartBatchResponse {
    id: String,
}

pub struct BatchClient {
    client: Client,
}

impl BatchClient {
    pub fn new(https_only: bool) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(https_only)
            .user_agent("DataExporter/1.0")
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client })
    }

    /// Start a new batch
    pub async fn start_batch(&self, token: &JwtToken, config: &Config) -> Result<String> {
        let url = format!("{}/api/v1/device/batches/start", config.api.base_url);

        debug!("Starting new batch");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| ProcessingError::NetworkError(format!("Failed to start batch: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProcessingError::NetworkError(format!(
                "Failed to start batch: {} - {}",
                status, error_body
            )));
        }

        let batch_response: StartBatchResponse = response.json().await.map_err(|e| {
            ProcessingError::NetworkError(format!("Failed to parse batch response: {}", e))
        })?;

        info!(batch_id = %batch_response.id, "Batch started");
        Ok(batch_response.id)
    }

    /// Complete a batch
    pub async fn complete_batch(
        &self,
        batch_id: &str,
        token: &JwtToken,
        config: &Config,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/device/batches/{}/complete",
            config.api.base_url, batch_id
        );

        debug!(batch_id = %batch_id, "Completing batch");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to complete batch: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProcessingError::NetworkError(format!(
                "Failed to complete batch: {} - {}",
                status, error_body
            )));
        }

        info!(batch_id = %batch_id, "Batch completed");
        Ok(())
    }

    /// Complete a batch with warnings (non-critical errors occurred)
    ///
    /// This marks the batch as COMPLETED_WITH_WARNINGS, indicating that some files
    /// failed to process but the batch was not completely unsuccessful.
    pub async fn complete_with_warnings(
        &self,
        batch_id: &str,
        token: &JwtToken,
        config: &Config,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/device/batches/{}/complete-with-warnings",
            config.api.base_url, batch_id
        );

        debug!(batch_id = %batch_id, "Completing batch with warnings");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| {
                ProcessingError::NetworkError(format!(
                    "Failed to complete batch with warnings: {}",
                    e
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProcessingError::NetworkError(format!(
                "Failed to complete batch with warnings: {} - {}",
                status, error_body
            )));
        }

        info!(batch_id = %batch_id, "Batch completed with warnings");
        Ok(())
    }

    /// Fail a batch
    pub async fn fail_batch(
        &self,
        batch_id: &str,
        token: &JwtToken,
        config: &Config,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/device/batches/{}/fail",
            config.api.base_url, batch_id
        );

        debug!(batch_id = %batch_id, "Failing batch");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| ProcessingError::NetworkError(format!("Failed to fail batch: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProcessingError::NetworkError(format!(
                "Failed to fail batch: {} - {}",
                status, error_body
            )));
        }

        info!(batch_id = %batch_id, "Batch marked as failed");
        Ok(())
    }

    /// Cancel a batch
    pub async fn cancel_batch(
        &self,
        batch_id: &str,
        token: &JwtToken,
        config: &Config,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/device/batches/{}/cancel",
            config.api.base_url, batch_id
        );

        debug!(batch_id = %batch_id, "Cancelling batch");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| ProcessingError::NetworkError(format!("Failed to cancel batch: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProcessingError::NetworkError(format!(
                "Failed to cancel batch: {} - {}",
                status, error_body
            )));
        }

        info!(batch_id = %batch_id, "Batch cancelled");
        Ok(())
    }
}
