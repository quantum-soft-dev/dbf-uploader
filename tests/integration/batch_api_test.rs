// T053 - Integration test for batch start/complete API flow
// This test verifies the full batch lifecycle using mock server

use common::auth::JwtToken;
use common::models::config::*;
use common::models::Config;
use std::path::PathBuf;
use wiremock::matchers::{header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn create_test_config(base_url: &str) -> Config {
    Config {
        scheduler: SchedulerConfig {
            crontab: "*/5 * * * *".to_string(),
        },
        src: SourceConfig {
            source_dir: PathBuf::from("/tmp"),
            include_patterns: None,
            exclude_patterns: None,
        },
        credential: CredentialConfig {
            account: "test".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            device: None,
        },
        api: ApiConfig {
            base_url: base_url.to_string(),
            https_only: false,
        },
        encoding: EncodingConfig {
            dbf_encoding: "CP866".to_string(),
        },
    }
}

/// T053 - Integration test for full batch API flow
#[tokio::test]
async fn test_batch_client_start_and_complete_flow() {
    // Arrange
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let token = JwtToken {
        token: "test_token".to_string(),
        expires_at: 9999999999,
    };

    // Mock batch start endpoint
    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .and(header("Authorization", "Bearer test_token"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "integration-test-batch-123"
            })),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // Mock batch complete endpoint
    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/integration-test-batch-123/complete"))
        .and(header("Authorization", "Bearer test_token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "acknowledged": true
            })),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // Act - Create batch client and run lifecycle
    let batch_client =
        data_exporter_service::processor::BatchClient::new(false).expect("Failed to create client");

    // Start batch
    let batch_id = batch_client
        .start_batch(&token, &config)
        .await
        .expect("Failed to start batch");

    assert_eq!(batch_id, "integration-test-batch-123");

    // Complete batch
    let complete_result = batch_client
        .complete_batch(&batch_id, &token, &config)
        .await;

    assert!(complete_result.is_ok(), "Batch complete should succeed");
}

/// T053 - Integration test for batch complete with warnings
#[tokio::test]
async fn test_batch_client_complete_with_warnings() {
    // Arrange
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let token = JwtToken {
        token: "test_token".to_string(),
        expires_at: 9999999999,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "warnings-test-batch"
            })),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/api/v1/device/batches/warnings-test-batch/complete-with-warnings",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "acknowledged": true
            })),
        )
        .mount(&mock_server)
        .await;

    // Act
    let batch_client = data_exporter_service::processor::BatchClient::new(false).unwrap();

    let batch_id = batch_client
        .start_batch(&token, &config)
        .await
        .expect("Failed to start batch");

    let result = batch_client
        .complete_with_warnings(&batch_id, &token, &config)
        .await;

    // Assert
    assert!(result.is_ok());
}

/// T053 - Integration test for batch fail
#[tokio::test]
async fn test_batch_client_fail_batch() {
    // Arrange
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let token = JwtToken {
        token: "test_token".to_string(),
        expires_at: 9999999999,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "fail-test-batch"
            })),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/fail-test-batch/fail"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "acknowledged": true
            })),
        )
        .mount(&mock_server)
        .await;

    // Act
    let batch_client = data_exporter_service::processor::BatchClient::new(false).unwrap();

    let batch_id = batch_client
        .start_batch(&token, &config)
        .await
        .expect("Failed to start batch");

    let result = batch_client.fail_batch(&batch_id, &token, &config).await;

    // Assert
    assert!(result.is_ok());
}

/// T053 - Integration test for batch cancel
#[tokio::test]
async fn test_batch_client_cancel_batch() {
    // Arrange
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let token = JwtToken {
        token: "test_token".to_string(),
        expires_at: 9999999999,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "cancel-test-batch"
            })),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/cancel-test-batch/cancel"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "acknowledged": true
            })),
        )
        .mount(&mock_server)
        .await;

    // Act
    let batch_client = data_exporter_service::processor::BatchClient::new(false).unwrap();

    let batch_id = batch_client
        .start_batch(&token, &config)
        .await
        .expect("Failed to start batch");

    let result = batch_client.cancel_batch(&batch_id, &token, &config).await;

    // Assert
    assert!(result.is_ok());
}

/// Test batch client error handling for server errors
#[tokio::test]
async fn test_batch_client_handles_server_error() {
    // Arrange
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let token = JwtToken {
        token: "test_token".to_string(),
        expires_at: 9999999999,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "error": "Internal server error"
            })),
        )
        .mount(&mock_server)
        .await;

    // Act
    let batch_client = data_exporter_service::processor::BatchClient::new(false).unwrap();
    let result = batch_client.start_batch(&token, &config).await;

    // Assert
    assert!(result.is_err(), "Should fail on 500 error");
}

/// Test batch client error handling for unauthorized
#[tokio::test]
async fn test_batch_client_handles_unauthorized() {
    // Arrange
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let token = JwtToken {
        token: "invalid_token".to_string(),
        expires_at: 9999999999,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Unauthorized"
            })),
        )
        .mount(&mock_server)
        .await;

    // Act
    let batch_client = data_exporter_service::processor::BatchClient::new(false).unwrap();
    let result = batch_client.start_batch(&token, &config).await;

    // Assert
    assert!(result.is_err(), "Should fail on 401 error");
}
