// Authentication testing utility for migration
use super::{MigrationError, MigrationResult};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Deserialize;

/// Test authentication with middleware using site credentials
pub async fn test_auth(
    base_url: &str,
    domain: &str,
    client_secret: &str,
) -> MigrationResult<AuthTestResult> {
    // Validate inputs
    if base_url.is_empty() {
        return Err(MigrationError::InvalidConfig(
            "Base URL cannot be empty".to_string(),
        ));
    }

    if domain.is_empty() {
        return Err(MigrationError::InvalidConfig(
            "Domain cannot be empty".to_string(),
        ));
    }

    if client_secret.is_empty() {
        return Err(MigrationError::InvalidConfig(
            "Client secret cannot be empty".to_string(),
        ));
    }

    // Ensure base_url starts with https://
    if !base_url.starts_with("https://") && !base_url.starts_with("http://") {
        return Err(MigrationError::InvalidConfig(
            "Base URL must start with https:// or http://".to_string(),
        ));
    }

    // Create HTTP client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| {
            MigrationError::AuthTestFailed(format!("Failed to create HTTP client: {}", e))
        })?;

    // Construct Basic Auth header
    let basic_auth = format!("{}:{}", domain, client_secret);
    let basic_auth_encoded = BASE64.encode(basic_auth.as_bytes());

    // Call authentication endpoint
    let auth_url = format!("{}/api/v1/auth/token", base_url);
    let response = client
        .post(&auth_url)
        .header("Authorization", format!("Basic {}", basic_auth_encoded))
        .send()
        .await
        .map_err(|e| MigrationError::AuthTestFailed(format!("Network error: {}", e)))?;

    // Check response status
    let status = response.status();
    if status.is_success() {
        // Try to parse response
        match response.json::<TokenResponse>().await {
            Ok(token_response) => Ok(AuthTestResult {
                success: true,
                message: "Authentication successful".to_string(),
                token_type: Some(token_response.token_type),
                expires_in: Some(token_response.expires_in),
            }),
            Err(e) => {
                // Authentication succeeded but couldn't parse response
                Ok(AuthTestResult {
                    success: true,
                    message: format!("Authentication successful (parse warning: {})", e),
                    token_type: None,
                    expires_in: None,
                })
            }
        }
    } else {
        // Authentication failed
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| format!("HTTP {}: Authentication failed", status.as_u16()));

        let message = match status.as_u16() {
            401 => format!(
                "Authentication failed: Invalid credentials ({})",
                error_text
            ),
            403 => format!("Authentication failed: Access forbidden ({})", error_text),
            404 => "Authentication failed: Endpoint not found. Check your base_url.".to_string(),
            _ => format!("Authentication failed: {} ({})", status, error_text),
        };

        Ok(AuthTestResult {
            success: false,
            message,
            token_type: None,
            expires_in: None,
        })
    }
}

/// Authentication test result
#[derive(Debug, Clone)]
pub struct AuthTestResult {
    /// Whether authentication succeeded
    pub success: bool,
    /// Result message
    pub message: String,
    /// Token type (if successful)
    pub token_type: Option<String>,
    /// Token expiration in seconds (if successful)
    pub expires_in: Option<i64>,
}

impl AuthTestResult {
    /// Display the test result
    pub fn display(&self) {
        if self.success {
            println!("✅ {}", self.message);
            if let Some(token_type) = &self.token_type {
                println!("   Token type: {}", token_type);
            }
            if let Some(expires_in) = self.expires_in {
                println!("   Expires in: {} seconds", expires_in);
            }
        } else {
            println!("❌ {}", self.message);
        }
    }
}

/// Token response from middleware
#[derive(Debug, Deserialize)]
struct TokenResponse {
    #[allow(dead_code)]
    token: String,
    #[serde(rename = "expiresIn")]
    expires_in: i64,
    #[serde(rename = "tokenType")]
    token_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_test_result_display() {
        let result = AuthTestResult {
            success: true,
            message: "Test successful".to_string(),
            token_type: Some("Bearer".to_string()),
            expires_in: Some(3600),
        };

        assert!(result.success);
        assert_eq!(result.message, "Test successful");
        assert_eq!(result.token_type, Some("Bearer".to_string()));
        assert_eq!(result.expires_in, Some(3600));
    }

    #[test]
    fn test_auth_test_result_failure() {
        let result = AuthTestResult {
            success: false,
            message: "Invalid credentials".to_string(),
            token_type: None,
            expires_in: None,
        };

        assert!(!result.success);
        assert_eq!(result.message, "Invalid credentials");
        assert!(result.token_type.is_none());
        assert!(result.expires_in.is_none());
    }

    #[tokio::test]
    async fn test_test_auth_empty_base_url() {
        let result = test_auth("", "test.com", "test-secret").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MigrationError::InvalidConfig(_)
        ));
    }

    #[tokio::test]
    async fn test_test_auth_empty_domain() {
        let result = test_auth("https://api.example.com", "", "test-secret").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MigrationError::InvalidConfig(_)
        ));
    }

    #[tokio::test]
    async fn test_test_auth_empty_client_secret() {
        let result = test_auth("https://api.example.com", "test.com", "").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MigrationError::InvalidConfig(_)
        ));
    }

    #[tokio::test]
    async fn test_test_auth_invalid_url_format() {
        let result = test_auth("not-a-url", "test.com", "test-secret").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MigrationError::InvalidConfig(_)
        ));
    }

    #[tokio::test]
    async fn test_test_auth_network_error() {
        // Using a non-existent domain should cause network error
        let result = test_auth(
            "https://nonexistent.invalid.domain.example",
            "test.com",
            "test-secret",
        )
        .await;

        // Should return error (network error or timeout)
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MigrationError::AuthTestFailed(_)
        ));
    }
}
