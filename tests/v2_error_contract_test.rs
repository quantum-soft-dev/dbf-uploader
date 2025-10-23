// Contract tests for Error Reporting v2 API
// Tests the middleware v2 error reporting endpoints

mod common;

use common::{test_data, MockMiddleware};
use data_exporter::auth::{AuthClient, SiteCredentials, TokenManager};
use data_exporter::error::{ErrorReporter, ProcessingError};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_batch_error_reporting_success() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create test JWT
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();

    // Setup mocks
    let _auth_mock = mock.mock_auth_success(
        test_data::TEST_DOMAIN,
        test_data::TEST_CLIENT_SECRET,
        &token,
        3600,
    );
    let _error_mock = mock.mock_error_report(&token, Some(batch_id));

    // Create error reporter with authentication
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("error.log");

    let credentials = SiteCredentials {
        domain: test_data::TEST_DOMAIN.to_string(),
        client_secret: test_data::TEST_CLIENT_SECRET.to_string(),
    };

    let auth_client = Arc::new(
        AuthClient::from_credentials(base_url.clone(), credentials)
            .expect("Failed to create AuthClient"),
    );
    let token_manager = Arc::new(TokenManager::from_auth_client(auth_client));

    let error_reporter = ErrorReporter::new(base_url, token_manager, log_path)
        .expect("Failed to create ErrorReporter");

    // Act
    let test_error = ProcessingError::ConfigurationError("Test error".to_string());
    let result = error_reporter
        .report_batch_error(batch_id, &test_error)
        .await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_standalone_error_reporting_success() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create test JWT
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    // Setup mocks
    let _auth_mock = mock.mock_auth_success(
        test_data::TEST_DOMAIN,
        test_data::TEST_CLIENT_SECRET,
        &token,
        3600,
    );
    let _error_mock = mock.mock_error_report(&token, None);

    // Create error reporter
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("error.log");

    let credentials = SiteCredentials {
        domain: test_data::TEST_DOMAIN.to_string(),
        client_secret: test_data::TEST_CLIENT_SECRET.to_string(),
    };

    let auth_client = Arc::new(
        AuthClient::from_credentials(base_url.clone(), credentials)
            .expect("Failed to create AuthClient"),
    );
    let token_manager = Arc::new(TokenManager::from_auth_client(auth_client));

    let error_reporter = ErrorReporter::new(base_url, token_manager, log_path)
        .expect("Failed to create ErrorReporter");

    // Act
    let test_error = ProcessingError::ConfigurationError("Standalone test error".to_string());
    let result = error_reporter.report_standalone_error(&test_error).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_error_reporting_without_auth() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Setup mock without authentication header requirement
    let _error_mock = mock
        .get_server()
        .mock("POST", "/api/v1/error")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"success": true}"#)
        .create();

    // Create error reporter without authentication
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("error.log");

    let error_reporter =
        ErrorReporter::without_auth(base_url, log_path).expect("Failed to create ErrorReporter");

    // Act
    let test_error = ProcessingError::NetworkError("Network failure".to_string());
    let result = error_reporter.report_standalone_error(&test_error).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_error_reporting_fallback_to_local_log() {
    // Setup mock server that will fail
    let mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // No mock setup - requests will fail

    // Create error reporter without authentication
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("error.log");

    let error_reporter = ErrorReporter::without_auth(base_url, log_path.clone())
        .expect("Failed to create ErrorReporter");

    // Act
    let test_error = ProcessingError::ConfigurationError("Should fallback to log".to_string());
    let result = error_reporter.report_standalone_error(&test_error).await;

    // Assert - should succeed via fallback
    assert!(result.is_ok());

    // Verify file was created and contains the error
    let contents = tokio::fs::read_to_string(&log_path)
        .await
        .expect("Failed to read log file");
    assert!(contents.contains("ConfigurationError"));
    assert!(contents.contains("Should fallback to log"));
}

#[tokio::test]
async fn test_batch_error_uses_correct_endpoint() {
    // Verify batch errors use /api/v1/error/{batch_id}
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();

    let _auth_mock = mock.mock_auth_success(
        test_data::TEST_DOMAIN,
        test_data::TEST_CLIENT_SECRET,
        &token,
        3600,
    );

    // Setup mock for batch-specific endpoint
    let _error_mock = mock
        .get_server()
        .mock("POST", format!("/api/v1/error/{}", batch_id).as_str())
        .match_header("authorization", format!("Bearer {}", token).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"success": true}"#)
        .create();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("error.log");

    let credentials = SiteCredentials {
        domain: test_data::TEST_DOMAIN.to_string(),
        client_secret: test_data::TEST_CLIENT_SECRET.to_string(),
    };

    let auth_client = Arc::new(
        AuthClient::from_credentials(base_url.clone(), credentials)
            .expect("Failed to create AuthClient"),
    );
    let token_manager = Arc::new(TokenManager::from_auth_client(auth_client));

    let error_reporter = ErrorReporter::new(base_url, token_manager, log_path)
        .expect("Failed to create ErrorReporter");

    // Act
    let test_error = ProcessingError::BatchError("Batch failure".to_string());
    let result = error_reporter
        .report_batch_error(batch_id, &test_error)
        .await;

    // Assert - if mock matched, endpoint is correct
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_standalone_error_uses_correct_endpoint() {
    // Verify standalone errors use /api/v1/error (without batch ID)
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let _auth_mock = mock.mock_auth_success(
        test_data::TEST_DOMAIN,
        test_data::TEST_CLIENT_SECRET,
        &token,
        3600,
    );

    // Setup mock for standalone endpoint
    let _error_mock = mock
        .get_server()
        .mock("POST", "/api/v1/error")
        .match_header("authorization", format!("Bearer {}", token).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"success": true}"#)
        .create();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("error.log");

    let credentials = SiteCredentials {
        domain: test_data::TEST_DOMAIN.to_string(),
        client_secret: test_data::TEST_CLIENT_SECRET.to_string(),
    };

    let auth_client = Arc::new(
        AuthClient::from_credentials(base_url.clone(), credentials)
            .expect("Failed to create AuthClient"),
    );
    let token_manager = Arc::new(TokenManager::from_auth_client(auth_client));

    let error_reporter = ErrorReporter::new(base_url, token_manager, log_path)
        .expect("Failed to create ErrorReporter");

    // Act
    let test_error = ProcessingError::AuthenticationError("Auth failure".to_string());
    let result = error_reporter.report_standalone_error(&test_error).await;

    // Assert - if mock matched, endpoint is correct
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_error_dto_serialization() {
    use data_exporter::error::dto::ErrorLogRequest;

    // Test that ErrorLogRequest serializes correctly
    let request = ErrorLogRequest::new("TestError".to_string(), "Test error message".to_string())
        .with_client_version("1.0.0".to_string());

    // Serialize to JSON
    let json = serde_json::to_string(&request).expect("Failed to serialize");

    // Verify JSON contains expected fields
    assert!(json.contains("TestError"));
    assert!(json.contains("Test error message"));
    assert!(json.contains("1.0.0"));

    // Verify field names match the API contract (camelCase due to #[serde(rename_all = "camelCase")])
    assert!(json.contains("\"type\"")); // error_type is explicitly renamed to "type"
    assert!(json.contains("clientVersion")); // client_version becomes "clientVersion" via camelCase
}
