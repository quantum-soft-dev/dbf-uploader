// Contract test for Auth API
// Tests POST /api/auth/token endpoint

use base64::Engine;

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_auth_token_success() {
    // Arrange
    let username = "test_user";
    let password = "test_password";
    let credentials = format!("{}:{}", username, password);
    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
    let auth_header = format!("Basic {}", encoded);

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/auth/token";

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 200, "Expected 200 OK response");

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    assert!(body.get("token").is_some(), "Response should contain 'token' field");
    assert!(body.get("expires_in").is_some(), "Response should contain 'expires_in' field");

    let token = body["token"].as_str().expect("Token should be a string");
    assert!(!token.is_empty(), "Token should not be empty");

    let expires_in = body["expires_in"].as_i64().expect("expires_in should be a number");
    assert!(expires_in > 0, "expires_in should be positive");
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_auth_token_invalid_credentials() {
    // Arrange
    let credentials = "invalid_user:wrong_password";
    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
    let auth_header = format!("Basic {}", encoded);

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/auth/token";

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 401, "Expected 401 Unauthorized for invalid credentials");

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    assert!(body.get("error").is_some(), "Response should contain 'error' field");
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_auth_token_missing_authorization() {
    // Arrange
    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/auth/token";

    // Act
    let response = client
        .post(api_url)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 401, "Expected 401 Unauthorized when Authorization header is missing");
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_auth_token_inactive_subscription() {
    // Arrange
    let credentials = "inactive_user:password";
    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
    let auth_header = format!("Basic {}", encoded);

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/auth/token";

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 403, "Expected 403 Forbidden for inactive subscription");

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    assert_eq!(body["error"], "subscription_inactive", "Error should be 'subscription_inactive'");
}
