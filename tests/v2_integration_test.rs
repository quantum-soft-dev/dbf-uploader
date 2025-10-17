// Integration tests for full upload workflow with v2 batch protocol
// Tests the complete end-to-end flow through UploaderService

mod common;

use common::{test_data, MockMiddleware};
use data_exporter::config::v2::{
    ApiConfigV2, AuthConfigV2, BatchConfig, ConfigV2, EncodingConfig, LoggingConfig,
    ScheduleConfig, SourceConfig,
};
use data_exporter::service::uploader::UploaderService;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create test ConfigV2 with mock server URL
fn create_test_config(base_url: String, source_dir: PathBuf, temp_dir: &TempDir) -> ConfigV2 {
    ConfigV2 {
        auth: AuthConfigV2 {
            domain: test_data::TEST_DOMAIN.to_string(),
            client_secret: test_data::TEST_CLIENT_SECRET.to_string(),
        },
        api: ApiConfigV2 {
            base_url,
            auth_endpoint: "/api/v1/auth/token".to_string(),
            batch_start: "/api/dfc/batch/start".to_string(),
            batch_upload: "/api/dfc/batch/{batchId}/upload".to_string(),
            batch_complete: "/api/dfc/batch/{batchId}/complete".to_string(),
            batch_fail: "/api/dfc/batch/{batchId}/fail".to_string(),
            batch_cancel: "/api/dfc/batch/{batchId}/cancel".to_string(),
            error_log: "/api/v1/error".to_string(),
        },
        source: SourceConfig {
            directory: source_dir,
        },
        schedule: ScheduleConfig {
            cron: "*/5 * * * *".to_string(),
        },
        encoding: EncodingConfig {
            fallback: "CP866".to_string(),
        },
        batch: BatchConfig {
            max_files_per_batch: 500,
            retry_locked_files: true,
            batch_timeout: 3600,
            max_retries: 3,
        },
        logging: LoggingConfig {
            error_log_path: temp_dir.path().join("error.log"),
        },
    }
}

#[tokio::test]
async fn test_full_upload_workflow_success() {
    // Setup test environment
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");

    // Create test DBF files in source directory
    let dbf_fixture = common::DbfTestFixture::new();
    let dbf_files = dbf_fixture.create_multiple_dbf(3);

    // Copy DBF files to source directory
    for dbf_file in &dbf_files {
        let filename = dbf_file.file_name().unwrap();
        let dest = source_dir.join(filename);
        std::fs::copy(dbf_file, &dest).expect("Failed to copy DBF file");
    }

    // Setup mock middleware server
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

    // Setup mocks for full workflow
    let _auth_mock = mock.mock_auth_success(
        test_data::TEST_DOMAIN,
        test_data::TEST_CLIENT_SECRET,
        &token,
        3600,
    );
    let _start_mock = mock.mock_batch_start(&token, batch_id);
    let _upload_mock = mock.mock_batch_upload(&token, batch_id, 3, 1024);
    let _complete_mock = mock.mock_batch_complete(&token, batch_id, 3, 1024);

    // Create configuration
    let config = create_test_config(base_url, source_dir.clone(), &temp_dir);

    // Create UploaderService
    let uploader = UploaderService::from_config(config)
        .await
        .expect("Failed to create UploaderService");

    // Act: Run scheduled batch
    let result = uploader.run_scheduled_batch().await;

    // Assert
    assert!(result.is_ok(), "Batch upload should succeed");
    let summary = result.unwrap();

    assert_eq!(summary.batch_id, batch_id);
    assert_eq!(summary.processed_count, 3, "Should process 3 files");
    assert_eq!(summary.failed_count, 0, "Should have no failures");
    assert!(summary.total_size > 0, "Should have uploaded data");
    assert!(summary.duration_secs >= 0, "Should have valid duration");
}

#[tokio::test]
async fn test_full_workflow_with_empty_directory() {
    // Setup test environment with empty source directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");

    // Setup mock middleware server
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
    let _start_mock = mock.mock_batch_start(&token, batch_id);
    let _cancel_mock = mock.mock_batch_cancel(&token, batch_id);

    // Create configuration
    let config = create_test_config(base_url, source_dir, &temp_dir);

    // Create UploaderService
    let uploader = UploaderService::from_config(config)
        .await
        .expect("Failed to create UploaderService");

    // Act: Run scheduled batch
    let result = uploader.run_scheduled_batch().await;

    // Assert: Batch should be cancelled when no files found
    assert!(result.is_ok(), "Empty directory should not error");
    let summary = result.unwrap();

    assert_eq!(summary.batch_id, batch_id);
    assert_eq!(summary.processed_count, 0, "Should process 0 files");
    assert_eq!(summary.failed_count, 0, "Should have no failures");
    assert_eq!(summary.total_size, 0, "Should have no data");
}

#[tokio::test]
async fn test_full_workflow_with_upload_failure() {
    // Setup test environment
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");

    // Create test DBF file
    let dbf_fixture = common::DbfTestFixture::new();
    let dbf_files = dbf_fixture.create_multiple_dbf(1);

    for dbf_file in &dbf_files {
        let filename = dbf_file.file_name().unwrap();
        let dest = source_dir.join(filename);
        std::fs::copy(dbf_file, &dest).expect("Failed to copy DBF file");
    }

    // Setup mock middleware server
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

    // Setup mocks: auth and batch start succeed, but upload fails
    let _auth_mock = mock.mock_auth_success(
        test_data::TEST_DOMAIN,
        test_data::TEST_CLIENT_SECRET,
        &token,
        3600,
    );
    let _start_mock = mock.mock_batch_start(&token, batch_id);

    // Mock upload failure (all retries will fail because mock returns 500)
    let _upload_mock = mock
        .get_server()
        .mock("POST", format!("/api/dfc/batch/{}/upload", batch_id).as_str())
        .match_header("authorization", format!("Bearer {}", token).as_str())
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Internal server error"}"#)
        .expect_at_least(3) // Should retry 3 times
        .create();

    let _fail_mock = mock.mock_batch_fail(&token, batch_id);

    // Create configuration with max_retries = 3
    let mut config = create_test_config(base_url, source_dir, &temp_dir);
    config.batch.max_retries = 3;

    // Create UploaderService
    let uploader = UploaderService::from_config(config)
        .await
        .expect("Failed to create UploaderService");

    // Act: Run scheduled batch
    let result = uploader.run_scheduled_batch().await;

    // Assert: Should fail after retries
    assert!(result.is_err(), "Upload should fail after max retries");

    let error = result.unwrap_err();
    assert!(
        format!("{}", error).contains("Upload failed"),
        "Error should indicate upload failure"
    );
}

#[tokio::test]
async fn test_full_workflow_with_chunked_uploads() {
    // Setup test environment with many files to trigger chunking
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");

    // Create 5 test DBF files
    let dbf_fixture = common::DbfTestFixture::new();
    let dbf_files = dbf_fixture.create_multiple_dbf(5);

    for dbf_file in &dbf_files {
        let filename = dbf_file.file_name().unwrap();
        let dest = source_dir.join(filename);
        std::fs::copy(dbf_file, &dest).expect("Failed to copy DBF file");
    }

    // Setup mock middleware server
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
    let _start_mock = mock.mock_batch_start(&token, batch_id);

    // Mock upload endpoint to accept any number of files
    let _upload_mock = mock
        .get_server()
        .mock("POST", format!("/api/dfc/batch/{}/upload", batch_id).as_str())
        .match_header("authorization", format!("Bearer {}", token).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status": "OK", "uploadedFiles": 2, "files": [{"fileName": "test.csv.gz", "fileSize": 1024, "uploadedAt": "2025-10-06T10:35:00Z"}]}"#)
        .expect_at_least(1)
        .create();

    let _complete_mock = mock.mock_batch_complete(&token, batch_id, 5, 2048);

    // Create configuration with small chunk size (2 files per batch)
    let mut config = create_test_config(base_url, source_dir, &temp_dir);
    config.batch.max_files_per_batch = 2;

    // Create UploaderService
    let uploader = UploaderService::from_config(config)
        .await
        .expect("Failed to create UploaderService");

    // Act: Run scheduled batch
    let result = uploader.run_scheduled_batch().await;

    // Assert
    assert!(result.is_ok(), "Chunked upload should succeed");
    let summary = result.unwrap();

    assert_eq!(summary.batch_id, batch_id);
    assert_eq!(summary.processed_count, 5, "Should process all 5 files");
    assert_eq!(summary.failed_count, 0, "Should have no failures");
    assert!(summary.total_size > 0, "Should have uploaded data");
}

// ============================================================================
// Token Renewal Integration Tests (Task 3.3.2)
// ============================================================================

// Note: Testing actual token expiry and renewal is complex with mock servers
// because TokenManager caches tokens and mockito doesn't support sequential
// responses easily. The token renewal logic is tested in unit tests for
// TokenManager itself (src/auth/mod.rs tests).

#[tokio::test]
async fn test_token_renewal_failure_handling() {
    // Setup test environment
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");

    // Create test DBF file
    let dbf_fixture = common::DbfTestFixture::new();
    let dbf_files = dbf_fixture.create_multiple_dbf(1);

    for dbf_file in &dbf_files {
        let filename = dbf_file.file_name().unwrap();
        let dest = source_dir.join(filename);
        std::fs::copy(dbf_file, &dest).expect("Failed to copy DBF file");
    }

    // Setup mock middleware server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Setup mock: all auth requests fail
    let _auth_mock = mock
        .get_server()
        .mock("POST", "/api/v1/auth/token")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Invalid credentials"}"#)
        .expect_at_least(1)
        .create();

    // Create configuration
    let config = create_test_config(base_url, source_dir, &temp_dir);

    // Create UploaderService (this succeeds, auth happens during operations)
    let uploader = UploaderService::from_config(config)
        .await
        .expect("Failed to create UploaderService");

    // Act: Try to run batch (should fail during auth)
    let result = uploader.run_scheduled_batch().await;

    // Assert: Should fail with authentication error
    assert!(result.is_err(), "Should fail when authentication fails");
}

#[tokio::test]
async fn test_multiple_operations_with_same_token() {
    // Test that TokenManager reuses valid tokens across multiple operations
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");

    // Create 2 test DBF files
    let dbf_fixture = common::DbfTestFixture::new();
    let dbf_files = dbf_fixture.create_multiple_dbf(2);

    for dbf_file in &dbf_files {
        let filename = dbf_file.file_name().unwrap();
        let dest = source_dir.join(filename);
        std::fs::copy(dbf_file, &dest).expect("Failed to copy DBF file");
    }

    // Setup mock middleware server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create valid token
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();

    // Setup mocks: auth should only be called ONCE
    let _auth_mock = mock
        .get_server()
        .mock("POST", "/api/v1/auth/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"token": "{}", "expiresIn": 3600, "tokenType": "Bearer"}}"#,
            token
        ))
        .expect(1) // Should be called exactly once
        .create();

    let _start_mock = mock.mock_batch_start(&token, batch_id);
    let _upload_mock = mock.mock_batch_upload(&token, batch_id, 2, 1024);
    let _complete_mock = mock.mock_batch_complete(&token, batch_id, 2, 1024);

    // Create configuration
    let config = create_test_config(base_url, source_dir, &temp_dir);

    // Create UploaderService
    let uploader = UploaderService::from_config(config)
        .await
        .expect("Failed to create UploaderService");

    // Act: Run scheduled batch
    let result = uploader.run_scheduled_batch().await;

    // Assert: Should succeed with token reuse
    assert!(result.is_ok(), "Batch should succeed with token reuse");
    let summary = result.unwrap();

    assert_eq!(summary.processed_count, 2, "Should process 2 files");
    assert_eq!(summary.failed_count, 0, "Should have no failures");

    // Auth mock's expect(1) will fail if called more than once
}

// ============================================================================
// Batch Timeout Handling Integration Tests (Task 3.3.3)
// ============================================================================

#[tokio::test]
async fn test_batch_start_failure_handling() {
    // Test that uploader gracefully handles batch start failure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");

    // Create test DBF file
    let dbf_fixture = common::DbfTestFixture::new();
    let dbf_files = dbf_fixture.create_multiple_dbf(1);

    for dbf_file in &dbf_files {
        let filename = dbf_file.file_name().unwrap();
        let dest = source_dir.join(filename);
        std::fs::copy(dbf_file, &dest).expect("Failed to copy DBF file");
    }

    // Setup mock middleware server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create valid token
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    // Setup mocks: auth succeeds but batch start fails
    let _auth_mock = mock.mock_auth_success(
        test_data::TEST_DOMAIN,
        test_data::TEST_CLIENT_SECRET,
        &token,
        3600,
    );

    let _start_mock = mock
        .get_server()
        .mock("POST", "/api/dfc/batch/start")
        .match_header("authorization", format!("Bearer {}", token).as_str())
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Internal server error"}"#)
        .create();

    // Create configuration
    let config = create_test_config(base_url, source_dir, &temp_dir);

    // Create UploaderService
    let uploader = UploaderService::from_config(config)
        .await
        .expect("Failed to create UploaderService");

    // Act: Try to run batch
    let result = uploader.run_scheduled_batch().await;

    // Assert: Should fail gracefully
    assert!(result.is_err(), "Should fail when batch start fails");
}

#[tokio::test]
async fn test_upload_retry_on_transient_failure() {
    // Test that uploader retries uploads on transient failures
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");

    // Create test DBF file
    let dbf_fixture = common::DbfTestFixture::new();
    let dbf_files = dbf_fixture.create_multiple_dbf(1);

    for dbf_file in &dbf_files {
        let filename = dbf_file.file_name().unwrap();
        let dest = source_dir.join(filename);
        std::fs::copy(dbf_file, &dest).expect("Failed to copy DBF file");
    }

    // Setup mock middleware server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create valid token
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
    let _start_mock = mock.mock_batch_start(&token, batch_id);

    // First 2 upload attempts fail, 3rd succeeds
    let _upload_mock_fail = mock
        .get_server()
        .mock("POST", format!("/api/dfc/batch/{}/upload", batch_id).as_str())
        .match_header("authorization", format!("Bearer {}", token).as_str())
        .with_status(503)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Service temporarily unavailable"}"#)
        .expect(2) // First 2 attempts fail
        .create();

    let _upload_mock_success = mock.mock_batch_upload(&token, batch_id, 1, 512);
    let _complete_mock = mock.mock_batch_complete(&token, batch_id, 1, 512);

    // Create configuration with 3 retries
    let mut config = create_test_config(base_url, source_dir, &temp_dir);
    config.batch.max_retries = 3;

    // Create UploaderService
    let uploader = UploaderService::from_config(config)
        .await
        .expect("Failed to create UploaderService");

    // Act: Run batch (should retry and succeed)
    let result = uploader.run_scheduled_batch().await;

    // Assert: Should eventually succeed after retries
    assert!(result.is_ok(), "Should succeed after retrying");
    let summary = result.unwrap();

    assert_eq!(summary.batch_id, batch_id);
    assert_eq!(summary.processed_count, 1, "Should process 1 file");
}

#[tokio::test]
async fn test_batch_complete_failure_handling() {
    // Test that uploader handles batch complete failure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");

    // Create test DBF file
    let dbf_fixture = common::DbfTestFixture::new();
    let dbf_files = dbf_fixture.create_multiple_dbf(1);

    for dbf_file in &dbf_files {
        let filename = dbf_file.file_name().unwrap();
        let dest = source_dir.join(filename);
        std::fs::copy(dbf_file, &dest).expect("Failed to copy DBF file");
    }

    // Setup mock middleware server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create valid token
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    let batch_id = test_data::test_batch_id();

    // Setup mocks: everything succeeds except batch complete
    let _auth_mock = mock.mock_auth_success(
        test_data::TEST_DOMAIN,
        test_data::TEST_CLIENT_SECRET,
        &token,
        3600,
    );
    let _start_mock = mock.mock_batch_start(&token, batch_id);
    let _upload_mock = mock.mock_batch_upload(&token, batch_id, 1, 512);

    let _complete_mock = mock
        .get_server()
        .mock("POST", format!("/api/dfc/batch/{}/complete", batch_id).as_str())
        .match_header("authorization", format!("Bearer {}", token).as_str())
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Failed to complete batch"}"#)
        .create();

    // Create configuration
    let config = create_test_config(base_url, source_dir, &temp_dir);

    // Create UploaderService
    let uploader = UploaderService::from_config(config)
        .await
        .expect("Failed to create UploaderService");

    // Act: Run batch
    let result = uploader.run_scheduled_batch().await;

    // Assert: Should fail when batch complete fails
    assert!(result.is_err(), "Should fail when batch complete fails");
}
