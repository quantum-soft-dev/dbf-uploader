// Authentication module
mod credentials;

pub use credentials::SiteCredentials;

use crate::error::{ProcessingError, Result};
use crate::models::Config;
use base64::Engine;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtToken {
    pub token: String,
    pub expires_at: u64, // Unix timestamp
    pub site_id: uuid::Uuid,
    pub account_id: uuid::Uuid,
    pub domain: String,
}

impl JwtToken {
    /// Check if token is expired or will expire soon (within 5 minutes)
    /// This threshold ensures we renew the token before batch operations.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        // Consider expired if less than 5 minutes (300 seconds) remaining
        self.expires_at <= now + 300
    }

    /// Parse JWT token payload to extract claims
    /// JWT format: header.payload.signature (all base64url encoded)
    pub fn parse_payload(token: &str) -> Result<JwtPayload> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(ProcessingError::AuthenticationError(
                "Invalid JWT format: expected 3 parts".to_string(),
            ));
        }

        // Decode the payload (second part)
        let payload_b64 = parts[1];
        let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|e| {
                ProcessingError::AuthenticationError(format!("Failed to decode JWT payload: {}", e))
            })?;

        // Parse JSON payload
        let payload: JwtPayload = serde_json::from_slice(&payload_bytes).map_err(|e| {
            ProcessingError::AuthenticationError(format!("Failed to parse JWT payload JSON: {}", e))
        })?;

        Ok(payload)
    }

    /// Create JwtToken from raw token string by parsing its payload
    pub fn from_token_string(token: String, expires_at: u64) -> Result<Self> {
        let payload = Self::parse_payload(&token)?;

        Ok(Self {
            token,
            expires_at,
            site_id: payload.site_id,
            account_id: payload.account_id,
            domain: payload.domain,
        })
    }
}

/// JWT payload structure matching middleware token format
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JwtPayload {
    pub site_id: uuid::Uuid,
    pub account_id: uuid::Uuid,
    pub domain: String,
    pub exp: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponse {
    token: String,
    #[serde(default)]
    expires_in: Option<u64>, // Seconds until expiration (legacy format)
    #[serde(default)]
    expires_at: Option<String>, // RFC3339 timestamp (current format)
}

pub struct AuthClient {
    client: Client,
    base_url: String,
    credentials: SiteCredentials,
}

impl AuthClient {
    /// Validate URL is secure (HTTPS or HTTP localhost for testing)
    fn validate_base_url(base_url: &str) -> Result<()> {
        if base_url.starts_with("https://") {
            return Ok(());
        }

        // Allow HTTP only for localhost/127.0.0.1 (for testing)
        if base_url.starts_with("http://localhost")
            || base_url.starts_with("http://127.0.0.1")
            || base_url.starts_with("http://[::1]")
        {
            return Ok(());
        }

        Err(ProcessingError::ConfigurationError(
            "API base URL must use HTTPS (HTTP only allowed for localhost)".to_string(),
        ))
    }

    /// Create a new authentication client with HTTPS-only enforcement
    ///
    /// Note: This currently accepts v1 Config with username/password.
    /// In v2, this will be replaced with SiteCredentials (domain:client_secret).
    /// For now, we treat username as domain and password as client_secret.
    pub fn new(config: &Config) -> Result<Self> {
        // Validate URL is secure
        Self::validate_base_url(&config.api.base_url)?;

        let is_localhost = config.api.base_url.starts_with("http://localhost")
            || config.api.base_url.starts_with("http://127.0.0.1")
            || config.api.base_url.starts_with("http://[::1]");

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(!is_localhost) // Allow HTTP for localhost testing
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        // Create SiteCredentials from config
        // V1 compatibility: username=domain, password=client_secret
        let credentials = SiteCredentials {
            domain: config.credential.username.clone(),
            client_secret: config.credential.password.clone(),
        };

        Ok(Self {
            client,
            base_url: config.api.base_url.clone(),
            credentials,
        })
    }

    /// Create a new authentication client directly from SiteCredentials (v2)
    pub fn from_credentials(base_url: String, credentials: SiteCredentials) -> Result<Self> {
        // Validate URL is secure
        Self::validate_base_url(&base_url)?;

        let is_localhost = base_url.starts_with("http://localhost")
            || base_url.starts_with("http://127.0.0.1")
            || base_url.starts_with("http://[::1]");

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(!is_localhost) // Allow HTTP for localhost testing
            .build()
            .map_err(|e| {
                ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            base_url,
            credentials,
        })
    }

    /// Retrieve a JWT token from the server using middleware v2 protocol
    pub async fn get_token(&self) -> Result<JwtToken> {
        let url = format!("{}/api/v1/auth/token", self.base_url);

        // Create Basic auth header with domain:clientSecret format
        let credentials_str = format!(
            "{}:{}",
            self.credentials.domain, self.credentials.client_secret
        );
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials_str.as_bytes());
        let auth_header = format!("Basic {}", encoded);

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

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("System time before UNIX epoch")
                    .as_secs();

                let expires_at = if let Some(expires_in) = token_response.expires_in {
                    now + expires_in
                } else if let Some(ref expires_at_str) = token_response.expires_at {
                    let parsed = DateTime::parse_from_rfc3339(expires_at_str)
                        .map_err(|e| {
                            ProcessingError::AuthenticationError(format!(
                                "Failed to parse expiresAt timestamp: {}",
                                e
                            ))
                        })?
                        .with_timezone(&Utc)
                        .timestamp();
                    if parsed < 0 {
                        return Err(ProcessingError::AuthenticationError(
                            "Parsed expiresAt timestamp is negative".to_string(),
                        ));
                    }
                    parsed as u64
                } else {
                    return Err(ProcessingError::AuthenticationError(
                        "Token response missing expiresIn/expiresAt".to_string(),
                    ));
                };

                JwtToken::from_token_string(token_response.token, expires_at)
            }
            401 => Err(ProcessingError::AuthenticationError(
                "Invalid credentials".to_string(),
            )),
            403 => {
                let body = response.text().await.unwrap_or_default();
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

    /// Create a TokenManager from an existing AuthClient (v2)
    pub fn from_auth_client(auth_client: Arc<AuthClient>) -> Self {
        Self {
            auth_client,
            current_token: Arc::new(RwLock::new(None)),
        }
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

    /// Helper to create a test JWT token with valid payload
    fn create_test_jwt() -> String {
        let site_id = uuid::Uuid::new_v4();
        let account_id = uuid::Uuid::new_v4();

        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let payload = format!(
            r#"{{"siteId":"{}","accountId":"{}","domain":"test.example.com","exp":9999999999}}"#,
            site_id, account_id
        );

        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header.as_bytes());
        let payload_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.as_bytes());

        format!("{}.{}.fake_signature", header_b64, payload_b64)
    }

    #[test]
    fn test_jwt_token_expiration() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let site_id = uuid::Uuid::new_v4();
        let account_id = uuid::Uuid::new_v4();

        // Expired token
        let expired_token = JwtToken {
            token: "expired".to_string(),
            expires_at: now - 100,
            site_id,
            account_id,
            domain: "test.example.com".to_string(),
        };
        assert!(expired_token.is_expired());

        // Valid token with plenty of time
        let valid_token = JwtToken {
            token: "valid".to_string(),
            expires_at: now + 3600,
            site_id,
            account_id,
            domain: "test.example.com".to_string(),
        };
        assert!(!valid_token.is_expired());

        // Token expiring soon (within 5 minutes)
        let expiring_soon = JwtToken {
            token: "expiring".to_string(),
            expires_at: now + 60, // 1 minute remaining
            site_id,
            account_id,
            domain: "test.example.com".to_string(),
        };
        assert!(expiring_soon.is_expired());

        // Token with more than 5 minutes remaining is not expired
        let valid_token_5min = JwtToken {
            token: "valid_5min".to_string(),
            expires_at: now + 301, // 5 minutes + 1 second
            site_id,
            account_id,
            domain: "test.example.com".to_string(),
        };
        assert!(!valid_token_5min.is_expired());
    }

    #[test]
    fn test_jwt_parse_payload_valid() {
        let token = create_test_jwt();
        let result = JwtToken::parse_payload(&token);
        assert!(result.is_ok());

        let payload = result.unwrap();
        assert_eq!(payload.domain, "test.example.com");
        assert_eq!(payload.exp, 9999999999);
    }

    #[test]
    fn test_jwt_parse_payload_invalid_format() {
        let result = JwtToken::parse_payload("invalid.token");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected 3 parts"));
    }

    #[test]
    fn test_jwt_parse_payload_invalid_base64() {
        let result = JwtToken::parse_payload("header.!!!invalid!!!.signature");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to decode"));
    }

    #[test]
    fn test_jwt_from_token_string() {
        let token_str = create_test_jwt();
        let expires_at = 9999999999;

        let result = JwtToken::from_token_string(token_str.clone(), expires_at);
        assert!(result.is_ok());

        let jwt_token = result.unwrap();
        assert_eq!(jwt_token.token, token_str);
        assert_eq!(jwt_token.expires_at, expires_at);
        assert_eq!(jwt_token.domain, "test.example.com");
    }
}
