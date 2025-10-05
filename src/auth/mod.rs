// Authentication module
use crate::error::{ProcessingError, Result};
use crate::models::Config;
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
struct TokenResponse {
    token: String,
    expires_in: u64, // Seconds until expiration
}

pub struct AuthClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

impl AuthClient {
    /// Create a new authentication client
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: config.api.base_url.clone(),
            username: config.credential.username.clone(),
            password: config.credential.password.clone(),
        })
    }

    /// Retrieve a JWT token from the server
    pub async fn get_token(&self) -> Result<JwtToken> {
        let url = format!("{}/api/auth/token", self.base_url);

        // Create Basic auth header
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        // Send request
        let response = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .send()
            .await
            .map_err(|e| ProcessingError::NetworkError(format!("Failed to send auth request: {}", e)))?;

        // Handle response
        match response.status().as_u16() {
            200 => {
                let token_response: TokenResponse = response
                    .json()
                    .await
                    .map_err(|e| ProcessingError::AuthenticationError(format!("Failed to parse token response: {}", e)))?;

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("System time before UNIX epoch")
                    .as_secs();

                Ok(JwtToken {
                    token: token_response.token,
                    expires_at: now + token_response.expires_in,
                })
            }
            401 => Err(ProcessingError::AuthenticationError("Invalid credentials".to_string())),
            403 => {
                let body = response.text().await.unwrap_or_default();
                if body.contains("subscription_inactive") {
                    Err(ProcessingError::AuthenticationError("Subscription inactive".to_string()))
                } else {
                    Err(ProcessingError::AuthenticationError("Forbidden".to_string()))
                }
            }
            status => Err(ProcessingError::AuthenticationError(format!("Unexpected status code: {}", status))),
        }
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
