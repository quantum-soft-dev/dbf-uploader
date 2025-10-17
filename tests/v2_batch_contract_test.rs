// Contract tests for Batch v2 API
// Tests the middleware v2 batch lifecycle with state transitions

mod common;

use common::{test_data, MockMiddleware};
use data_exporter::batch::{BatchManager, BatchState};
use data_exporter::auth::{AuthClient, SiteCredentials, TokenManager};
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create BatchManager with mock server
async fn create_batch_manager_with_mock(mock: &MockMiddleware) -> BatchManager {
    let base_url = mock.url();
    let credentials = SiteCredentials {
        domain: test_data::TEST_DOMAIN.to_string(),
        client_secret: test_data::TEST_CLIENT_SECRET.to_string(),
    };

    let auth_client = Arc::new(
        AuthClient::from_credentials(base_url.clone(), credentials)
            .expect("Failed to create AuthClient"),
    );
    let token_manager = Arc::new(TokenManager::from_auth_client(auth_client));

    BatchManager::new(base_url, token_manager, 300).expect("Failed to create BatchManager")
}

#[tokio::test]
async fn test_batch_start_success() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;

    // Create test JWT
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    // Setup mocks: auth + batch start
    let _auth_mock = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);
    let batch_id = test_data::test_batch_id();
    let _batch_mock = mock.mock_batch_start(&token, batch_id, test_data::test_site_id());

    // Create BatchManager
    let mut batch_manager = create_batch_manager_with_mock(&mock).await;

    // Act
    let batch = batch_manager.start_batch().await.expect("Failed to start batch");

    // Assert
    assert_eq!(batch.id, batch_id);
    assert_eq!(batch.state, BatchState::InProgress);
    assert_eq!(batch.uploaded_files_count, 0);
    assert_eq!(batch.error_count, 0);
}

#[tokio::test]
async fn test_batch_upload_success() {
    // Setup mock server and batch
    let mut mock = MockMiddleware::new().await;
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();
    let _auth_mock = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);
    let _start_mock = mock.mock_batch_start(&token, batch_id, test_data::test_site_id());

    let mut batch_manager = create_batch_manager_with_mock(&mock).await;
    batch_manager.start_batch().await.expect("Failed to start batch");

    // Create test file to upload
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.csv.gz");
    std::fs::write(&test_file, b"test data").expect("Failed to write test file");

    // Setup upload mock
    let _upload_mock = mock.mock_batch_upload(&token, batch_id, 1, 9);

    // Act
    let result = batch_manager.upload_files(vec![test_file]).await;

    // Assert
    assert!(result.is_ok());
    let summary = result.unwrap();
    assert_eq!(summary.status, "OK");
    assert_eq!(summary.uploaded_files, 1); // Count of uploaded files
    assert_eq!(summary.files.len(), 1); // Actual file array
    assert_eq!(summary.files[0].file_size, 9);
}

#[tokio::test]
async fn test_batch_complete_success() {
    // Setup mock server and batch
    let mut mock = MockMiddleware::new().await;
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();
    let _auth_mock = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);
    let _start_mock = mock.mock_batch_start(&token, batch_id, test_data::test_site_id());
    let _complete_mock = mock.mock_batch_complete(&token, batch_id, test_data::test_site_id(), 5, 1024);

    let mut batch_manager = create_batch_manager_with_mock(&mock).await;
    batch_manager.start_batch().await.expect("Failed to start batch");

    // Act
    let result = batch_manager.complete_batch().await;

    // Assert
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.batch_id, batch_id);
    assert_eq!(response.uploaded_files_count, 5);
    assert_eq!(response.total_size, 1024);
    assert!(!response.has_errors);
}

#[tokio::test]
async fn test_batch_fail_success() {
    // Setup mock server and batch
    let mut mock = MockMiddleware::new().await;
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();
    let _auth_mock = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);
    let _start_mock = mock.mock_batch_start(&token, batch_id, test_data::test_site_id());
    let _fail_mock = mock.mock_batch_fail(&token, batch_id, test_data::test_site_id());

    let mut batch_manager = create_batch_manager_with_mock(&mock).await;
    batch_manager.start_batch().await.expect("Failed to start batch");

    // Act
    let result = batch_manager.fail_batch("Test failure reason").await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_batch_cancel_success() {
    // Setup mock server and batch
    let mut mock = MockMiddleware::new().await;
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();
    let _auth_mock = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);
    let _start_mock = mock.mock_batch_start(&token, batch_id, test_data::test_site_id());
    let _cancel_mock = mock.mock_batch_cancel(&token, batch_id, test_data::test_site_id());

    let mut batch_manager = create_batch_manager_with_mock(&mock).await;
    batch_manager.start_batch().await.expect("Failed to start batch");

    // Act
    let result = batch_manager.cancel_batch().await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_batch_state_transitions() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();
    let _auth_mock = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);
    let _start_mock = mock.mock_batch_start(&token, batch_id, test_data::test_site_id());

    let mut batch_manager = create_batch_manager_with_mock(&mock).await;

    // Initial state
    assert!(batch_manager.get_current_batch().expect("Failed to get current batch").is_none());

    // After start
    batch_manager.start_batch().await.expect("Failed to start batch");
    assert!(batch_manager.get_current_batch().expect("Failed to get current batch").is_some());
    assert_eq!(
        batch_manager.get_current_batch().expect("Failed to get current batch").unwrap().state,
        BatchState::InProgress
    );

    // Setup complete mock
    let _complete_mock = mock.mock_batch_complete(&token, batch_id, test_data::test_site_id(), 0, 0);

    // After complete
    batch_manager.complete_batch().await.expect("Failed to complete batch");

    // After completion, current batch is cleared
    assert!(batch_manager.get_current_batch().expect("Failed to get current batch").is_none());
}

#[tokio::test]
async fn test_batch_endpoints_use_bearer_token() {
    // This test verifies that batch endpoints use Bearer token auth
    let mut mock = MockMiddleware::new().await;
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();

    // Setup mocks that expect Bearer token
    let _auth_mock = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);
    let _batch_mock = mock.mock_batch_start(&token, batch_id, test_data::test_site_id());

    let mut batch_manager = create_batch_manager_with_mock(&mock).await;

    // Act - if the token isn't passed correctly, the mock won't match
    let result = batch_manager.start_batch().await;

    // Assert - successful call means Bearer token was used correctly
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_batch_complete_with_errors() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();
    let _auth_mock = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);
    let _start_mock = mock.mock_batch_start(&token, batch_id, test_data::test_site_id());

    // Mock complete response with errors (full BatchResponseDto)
    let _complete_mock = mock.get_server()
        .mock("POST", format!("/api/dfc/batch/{}/complete", batch_id).as_str())
        .match_header("authorization", format!("Bearer {}", token).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"id": "{}", "batchId": "{}", "siteId": "{}", "status": "COMPLETED", "s3Path": "account123/site456/2025-10-17/batch123", "uploadedFilesCount": 3, "totalSize": 512, "hasErrors": true, "startedAt": "2025-10-17T10:00:00Z", "completedAt": "2025-10-17T11:30:00Z"}}"#,
            batch_id, batch_id, test_data::test_site_id()
        ))
        .create();

    let mut batch_manager = create_batch_manager_with_mock(&mock).await;
    batch_manager.start_batch().await.expect("Failed to start batch");

    // Act
    let result = batch_manager.complete_batch().await;

    // Assert
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.has_errors);
    assert_eq!(response.uploaded_files_count, 3);
}

#[tokio::test]
async fn test_multiple_batch_operations_sequential() {
    // Test sequential batch operations (complete one before starting another)
    let mut mock = MockMiddleware::new().await;
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id_1 = test_data::test_batch_id();
    let batch_id_2 = uuid::Uuid::new_v4();

    let _auth_mock = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);

    // First batch
    let _start_mock_1 = mock.mock_batch_start(&token, batch_id_1, test_data::test_site_id());
    let _complete_mock_1 = mock.mock_batch_complete(&token, batch_id_1, test_data::test_site_id(), 1, 100);

    let mut batch_manager = create_batch_manager_with_mock(&mock).await;

    // Start and complete first batch
    batch_manager.start_batch().await.expect("Failed to start batch 1");
    assert_eq!(batch_manager.get_current_batch().expect("Failed to get current batch").unwrap().id, batch_id_1);

    batch_manager.complete_batch().await.expect("Failed to complete batch 1");

    // Second batch
    let _start_mock_2 = mock.mock_batch_start(&token, batch_id_2, test_data::test_site_id());
    let _complete_mock_2 = mock.mock_batch_complete(&token, batch_id_2, test_data::test_site_id(), 2, 200);

    // Start and complete second batch
    batch_manager.start_batch().await.expect("Failed to start batch 2");
    assert_eq!(batch_manager.get_current_batch().expect("Failed to get current batch").unwrap().id, batch_id_2);

    batch_manager.complete_batch().await.expect("Failed to complete batch 2");
}
