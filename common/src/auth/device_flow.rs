// Device Authorization Grant (RFC 8628) implementation
use crate::error::{ProcessingError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Response from device authorization request (Step 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthorizationResponse {
    /// Device code for polling (64 chars, keep secure)
    pub device_code: String,
    /// User code for display (format: XXXX-1234)
    pub user_code: String,
    /// URL user should open
    pub verification_uri: String,
    /// URL with code pre-filled
    pub verification_uri_complete: String,
    /// Seconds until codes expire (900 = 15 minutes)
    pub expires_in: u64,
    /// Minimum seconds between polling requests
    pub interval: u64,
}

/// Response from device token request (Step 3 - success)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCredentials {
    /// UUID of the created site
    pub site_id: String,
    /// Domain for the site (username for Basic Auth)
    pub domain: String,
    /// Client secret for authentication (password for Basic Auth)
    pub client_secret: String,
    /// Base URL for API requests
    pub api_base_url: String,
}

/// Error response from device token request (Step 3 - pending/error)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTokenError {
    /// RFC 8628 error code
    pub error: String,
    /// Human-readable error description
    #[serde(default)]
    pub error_description: Option<String>,
}

/// Site information for device authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteInfo {
    /// Site name (1-100 chars, alphanumeric + hyphens)
    pub site_name: String,
    /// Optional site description (max 500 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_description: Option<String>,
}

/// Device token request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceTokenRequest {
    device_code: String,
}

/// Device Authorization Flow client
#[derive(Debug)]
pub struct DeviceFlowClient {
    client: Client,
    base_url: String,
}

impl DeviceFlowClient {
    /// Create a new Device Flow client with optional HTTPS-only enforcement
    pub fn new(base_url: String, https_only: bool) -> Result<Self> {
        // Validate HTTPS-only URL if https_only is enabled
        if https_only && !base_url.starts_with("https://") {
            return Err(ProcessingError::ConfigurationError(
                "API base URL must use HTTPS when https_only is enabled".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(https_only)
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client, base_url })
    }

    /// Step 1: Request device and user codes
    pub async fn authorize(&self, site_info: SiteInfo) -> Result<DeviceAuthorizationResponse> {
        let url = format!("{}/api/v1/device/authorize", self.base_url);

        tracing::info!(
            url = %url,
            site_name = %site_info.site_name,
            "Requesting device authorization codes"
        );

        let response = self
            .client
            .post(&url)
            .json(&site_info)
            .send()
            .await
            .map_err(|e| {
                ProcessingError::NetworkError(format!(
                    "Failed to send authorization request: {}",
                    e
                ))
            })?;

        match response.status().as_u16() {
            200 => {
                let auth_response: DeviceAuthorizationResponse =
                    response.json().await.map_err(|e| {
                        ProcessingError::NetworkError(format!(
                            "Failed to parse authorization response: {}",
                            e
                        ))
                    })?;

                tracing::info!(
                    user_code = %auth_response.user_code,
                    expires_in = auth_response.expires_in,
                    "Device authorization codes received"
                );

                Ok(auth_response)
            }
            status => {
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unable to read response body".to_string());

                Err(ProcessingError::NetworkError(format!(
                    "Device authorization failed with status {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Step 3: Poll for credentials after user confirmation
    ///
    /// Returns Ok(Some(credentials)) when authorized
    /// Returns Ok(None) when still pending or should slow down
    /// Returns Err(_) when authorization denied, expired, or invalid
    pub async fn poll_for_token(&self, device_code: &str) -> Result<Option<DeviceCredentials>> {
        let url = format!("{}/api/v1/device/token", self.base_url);

        let request_body = DeviceTokenRequest {
            device_code: device_code.to_string(),
        };

        tracing::debug!(
            url = %url,
            "Polling for device credentials"
        );

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to send token request: {}", e))
            })?;

        match response.status().as_u16() {
            200 => {
                // Success - credentials received
                let credentials: DeviceCredentials = response.json().await.map_err(|e| {
                    ProcessingError::NetworkError(format!(
                        "Failed to parse credentials response: {}",
                        e
                    ))
                })?;

                tracing::info!(
                    site_id = %credentials.site_id,
                    domain = %credentials.domain,
                    "Device authorization successful"
                );

                Ok(Some(credentials))
            }
            400 => {
                // Error response - check error code
                let error_response: DeviceTokenError = response.json().await.map_err(|e| {
                    ProcessingError::NetworkError(format!("Failed to parse error response: {}", e))
                })?;

                match error_response.error.as_str() {
                    "authorization_pending" => {
                        tracing::debug!("Authorization pending, continue polling");
                        Ok(None)
                    }
                    "slow_down" => {
                        tracing::warn!("Polling too fast, should slow down");
                        // Return as error so run_device_flow can increase polling interval
                        Err(ProcessingError::AuthenticationError(
                            "slow_down".to_string(),
                        ))
                    }
                    "expired_token" => Err(ProcessingError::AuthenticationError(
                        "Device code expired (15 minute TTL exceeded)".to_string(),
                    )),
                    "invalid_grant" => Err(ProcessingError::AuthenticationError(
                        "Invalid device code".to_string(),
                    )),
                    other => Err(ProcessingError::AuthenticationError(format!(
                        "Device authorization error: {} - {}",
                        other,
                        error_response.error_description.unwrap_or_default()
                    ))),
                }
            }
            403 => {
                // Access denied
                Err(ProcessingError::AuthenticationError(
                    "User denied authorization".to_string(),
                ))
            }
            status => {
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unable to read response body".to_string());

                Err(ProcessingError::NetworkError(format!(
                    "Device token request failed with status {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Complete device authorization flow with automatic polling
    ///
    /// This is a convenience method that combines authorize() and poll_for_token()
    /// with automatic retry logic according to RFC 8628.
    ///
    /// Returns credentials when user completes authorization
    pub async fn run_device_flow(
        &self,
        site_info: SiteInfo,
    ) -> Result<(DeviceCredentials, DeviceAuthorizationResponse)> {
        // Step 1: Get codes
        let auth_response = self.authorize(site_info).await?;

        // Step 2: User instructions are displayed by the caller

        // Step 3: Poll for credentials
        let mut interval = Duration::from_secs(auth_response.interval);
        let timeout = Duration::from_secs(auth_response.expires_in);
        let start_time = std::time::Instant::now();

        loop {
            // Check timeout
            if start_time.elapsed() >= timeout {
                return Err(ProcessingError::AuthenticationError(
                    "Device authorization timeout (15 minutes expired)".to_string(),
                ));
            }

            // Wait for interval
            tokio::time::sleep(interval).await;

            // Poll for credentials
            match self.poll_for_token(&auth_response.device_code).await {
                Ok(Some(credentials)) => {
                    // Success!
                    return Ok((credentials, auth_response));
                }
                Ok(None) => {
                    // Still pending or slow_down - continue polling
                    // Note: slow_down should increase interval, but we handle that in the error
                    continue;
                }
                Err(ProcessingError::AuthenticationError(msg)) if msg.contains("slow_down") => {
                    // Increase interval by 5 seconds as per RFC 8628
                    interval += Duration::from_secs(5);
                    tracing::warn!(
                        new_interval_secs = interval.as_secs(),
                        "Polling too fast, increasing interval"
                    );
                    continue;
                }
                Err(e) => {
                    // Other errors are terminal
                    return Err(e);
                }
            }
        }
    }

    /// Display user instructions for device authorization
    pub fn display_instructions(auth_response: &DeviceAuthorizationResponse) {
        println!("\n{}", "=".repeat(63));
        println!("  DEVICE AUTHORIZATION REQUIRED");
        println!("{}", "=".repeat(63));
        println!();
        println!("  1. Open: {}", auth_response.verification_uri);
        println!();
        println!("  2. Enter code: {}", auth_response.user_code);
        println!();
        println!("  3. Select your site and click 'Authorize'");
        println!();
        println!(
            "  Code expires in {} minutes.",
            auth_response.expires_in / 60
        );
        println!("{}", "=".repeat(63));
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_flow_client_creation() {
        let client = DeviceFlowClient::new("https://api.example.com".to_string(), true);
        assert!(client.is_ok());
    }

    #[test]
    fn test_device_flow_client_https_only_validation() {
        let result = DeviceFlowClient::new("http://api.example.com".to_string(), true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must use HTTPS"));
    }

    #[test]
    fn test_site_info_serialization() {
        let site_info = SiteInfo {
            site_name: "warehouse-01".to_string(),
            site_description: Some("Main warehouse".to_string()),
        };

        let json = serde_json::to_string(&site_info).unwrap();
        assert!(json.contains("siteName"));
        assert!(json.contains("siteDescription"));
    }

    #[test]
    fn test_device_credentials_deserialization() {
        let json = r#"{"siteId":"550e8400-e29b-41d4-a716-446655440000","domain":"c823d8b8-0e6f-4242-a350-d6ef335ab4e8_warehouse-01","clientSecret":"cs_secret123","apiBaseUrl":"https://api.dataforge.com"}"#;
        let credentials: DeviceCredentials = serde_json::from_str(json).unwrap();
        assert_eq!(credentials.site_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(
            credentials.domain,
            "c823d8b8-0e6f-4242-a350-d6ef335ab4e8_warehouse-01"
        );
        assert_eq!(credentials.client_secret, "cs_secret123");
        assert_eq!(credentials.api_base_url, "https://api.dataforge.com");
    }
}
