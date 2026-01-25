// Authentication module
use crate::error::{ProcessingError, Result};
use crate::models::Config;
use base64::Engine;
use chrono::DateTime;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod device_flow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtToken {
    pub token: String,
    pub expires_at: u64, // Unix timestamp
}

impl JwtToken {
    /// Check if token is expired or will expire soon (within 60 seconds)
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        // Consider expired if less than 60 seconds remaining
        self.expires_at <= now + 60
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponse {
    token: String,
    #[serde(rename = "expiresAt")]
    expires_at: String, // ISO 8601 timestamp
}

pub struct AuthClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

impl AuthClient {
    /// Create a new authentication client with optional HTTPS-only enforcement
    pub fn new(config: &Config) -> Result<Self> {
        // Validate HTTPS-only URL if https_only is enabled
        if config.api.https_only && !config.api.base_url.starts_with("https://") {
            return Err(ProcessingError::ConfigurationError(
                "API base URL must use HTTPS when https_only is enabled".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(config.api.https_only) // Enforce HTTPS-only connections if configured
            .user_agent("DataExporter/1.0")
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            base_url: config.api.base_url.clone(),
            username: config.credential.full_username(),
            password: config.credential.password().to_string(),
        })
    }

    /// Retrieve a JWT token from the server
    pub async fn get_token(&self) -> Result<JwtToken> {
        let url = format!("{}/api/v1/device/auth/token", self.base_url);

        // Create Basic auth header
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        tracing::debug!(
            url = %url,
            "Sending auth request"
        );

        // Send request
        let response = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .send()
            .await
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to send auth request: {}", e))
            })?;

        // Handle response
        match response.status().as_u16() {
            200 => {
                let token_response: TokenResponse = response.json().await.map_err(|e| {
                    ProcessingError::AuthenticationError(format!(
                        "Failed to parse token response: {}",
                        e
                    ))
                })?;

                // Parse ISO 8601 timestamp
                let expires_at_dt = DateTime::parse_from_rfc3339(&token_response.expires_at)
                    .map_err(|e| {
                        ProcessingError::AuthenticationError(format!(
                            "Failed to parse expiration timestamp: {}",
                            e
                        ))
                    })?;

                let expires_at = expires_at_dt.timestamp() as u64;

                Ok(JwtToken {
                    token: token_response.token,
                    expires_at,
                })
            }
            401 => Err(ProcessingError::AuthenticationError(
                "Invalid credentials".to_string(),
            )),
            403 => {
                let body = response.text().await.unwrap_or_default();
                tracing::debug!(
                    response_body = %body,
                    "Received 403 Forbidden response"
                );
                if body.contains("subscription_inactive") {
                    Err(ProcessingError::AuthenticationError(
                        "Subscription inactive".to_string(),
                    ))
                } else {
                    Err(ProcessingError::AuthenticationError(
                        "Forbidden".to_string(),
                    ))
                }
            }
            status => Err(ProcessingError::AuthenticationError(format!(
                "Unexpected status code: {}",
                status
            ))),
        }
    }
}

/// TokenManager manages JWT token lifecycle with automatic renewal
pub struct TokenManager {
    auth_client: Arc<AuthClient>,
    current_token: Arc<RwLock<Option<JwtToken>>>,
}

impl TokenManager {
    /// Create a new TokenManager
    pub fn new(config: &Config) -> Result<Self> {
        let auth_client = Arc::new(AuthClient::new(config)?);
        Ok(Self {
            auth_client,
            current_token: Arc::new(RwLock::new(None)),
        })
    }

    /// Get a valid token, renewing if necessary
    /// Returns the current token if it's valid and not expiring soon,
    /// otherwise requests a new token and updates the stored token.
    pub async fn get_valid_token(&self) -> Result<JwtToken> {
        // First, check if we have a valid token
        {
            let token_lock = self.current_token.read().map_err(|e| {
                ProcessingError::AuthenticationError(format!(
                    "Failed to acquire read lock on token: {}",
                    e
                ))
            })?;

            if let Some(ref token) = *token_lock {
                if !token.is_expired() {
                    return Ok(token.clone());
                }
            }
        }

        // Token is expired or doesn't exist, need to get a new one
        // Acquire write lock to check one more time (double-check pattern)
        {
            let token_lock = self.current_token.write().map_err(|e| {
                ProcessingError::AuthenticationError(format!(
                    "Failed to acquire write lock on token: {}",
                    e
                ))
            })?;

            // Double-check: another thread might have already refreshed the token
            if let Some(ref token) = *token_lock {
                if !token.is_expired() {
                    return Ok(token.clone());
                }
            }
        } // Release write lock before await

        // Request a new token (without holding any locks)
        let new_token = self.auth_client.get_token().await?;

        // Acquire write lock again to store the new token
        {
            let mut token_lock = self.current_token.write().map_err(|e| {
                ProcessingError::AuthenticationError(format!(
                    "Failed to acquire write lock on token: {}",
                    e
                ))
            })?;
            *token_lock = Some(new_token.clone());
        }

        Ok(new_token)
    }

    /// Clear the stored token (useful for testing or manual refresh)
    pub fn clear_token(&self) -> Result<()> {
        let mut token_lock = self.current_token.write().map_err(|e| {
            ProcessingError::AuthenticationError(format!(
                "Failed to acquire write lock on token: {}",
                e
            ))
        })?;
        *token_lock = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_token_expiration() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Expired token
        let expired_token = JwtToken {
            token: "expired".to_string(),
            expires_at: now - 100,
        };
        assert!(expired_token.is_expired());

        // Valid token with plenty of time
        let valid_token = JwtToken {
            token: "valid".to_string(),
            expires_at: now + 3600,
        };
        assert!(!valid_token.is_expired());

        // Token expiring soon (within 60 seconds)
        let expiring_soon = JwtToken {
            token: "expiring".to_string(),
            expires_at: now + 30,
        };
        assert!(expiring_soon.is_expired());
    }
}
