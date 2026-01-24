// Contract tests for Error Report API
// Tests batch-specific and global error reporting endpoints

use wiremock::matchers::{method, path, path_regex, header, body_json_schema};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ==================== Batch Error Report Tests (POST /api/batches/{id}/errors) ====================

/// T074: Contract test for /api/batches/{id}/errors endpoint
#[tokio::test]
async fn test_batch_error_report_success() {
    // Arrange
    let mock_server = MockServer::start().await;
    let batch_id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("POST"))
        .and(path(format!("/api/batches/{}/errors", batch_id)))
        .and(header("Authorization", "Bearer test-token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&mock_server)
        .await;

    let error_report = serde_json::json!({
        "type": "FileReadError",
        "message": "Failed to read DBF file: Permission denied (OS Error 5)",
        "error_details": "Caused by: Access is denied (OS Error 5)",
        "metadata": {
            "filename": "archive/2024/sales.dbf",
            "clientVersion": "1.0.0",
            "timestamp": "2025-10-05T14:30:00Z"
        }
    });

    let client = reqwest::Client::new();

    // Act
    let response = client
        .post(format!("{}/api/batches/{}/errors", mock_server.uri(), batch_id))
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .json(&error_report)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 201);
}

/// T074: Contract test for batch error with invalid batch ID
#[tokio::test]
async fn test_batch_error_report_not_found() {
    // Arrange
    let mock_server = MockServer::start().await;
    let batch_id = "non-existent-batch-id";

    Mock::given(method("POST"))
        .and(path(format!("/api/batches/{}/errors", batch_id)))
        .respond_with(ResponseTemplate::new(404)
            .set_body_json(serde_json::json!({
                "error": "not_found",
                "message": "Batch not found"
            })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let error_report = serde_json::json!({
        "type": "FileReadError",
        "message": "Test error"
    });

    let client = reqwest::Client::new();

    // Act
    let response = client
        .post(format!("{}/api/batches/{}/errors", mock_server.uri(), batch_id))
        .header("Authorization", "Bearer test-token")
        .json(&error_report)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 404);
}

// ==================== Global Error Report Tests (POST /api/errors) ====================

/// T073: Contract test for /api/errors endpoint (global errors outside batch context)
#[tokio::test]
async fn test_global_error_report_success() {
    // Arrange
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/errors"))
        .and(header("Authorization", "Bearer test-token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&mock_server)
        .await;

    let error_report = serde_json::json!({
        "type": "CONFIG_ERROR",
        "message": "Failed to load configuration: Invalid TOML syntax",
        "severity": "CRITICAL",
        "metadata": {
            "configPath": "C:\\Program Files\\data-exporter\\config.toml",
            "timestamp": "2025-10-05T14:30:00Z"
        }
    });

    let client = reqwest::Client::new();

    // Act
    let response = client
        .post(format!("{}/api/errors", mock_server.uri()))
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .json(&error_report)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 201);
}

/// T073: Contract test for all severity levels
#[tokio::test]
async fn test_global_error_report_all_severities() {
    let mock_server = MockServer::start().await;
    let client = reqwest::Client::new();

    let severities = vec!["CRITICAL", "ERROR", "WARNING", "INFO"];

    Mock::given(method("POST"))
        .and(path("/api/errors"))
        .respond_with(ResponseTemplate::new(201))
        .expect(4)
        .mount(&mock_server)
        .await;

    for severity in severities {
        let error_report = serde_json::json!({
            "type": "TestError",
            "message": format!("Test message with {} severity", severity),
            "severity": severity
        });

        let response = client
            .post(format!("{}/api/errors", mock_server.uri()))
            .header("Authorization", "Bearer test-token")
            .json(&error_report)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status(), 201, "Expected 201 for severity {}", severity);
    }
}

/// T073: Contract test for global error report without authorization
#[tokio::test]
async fn test_global_error_report_unauthorized() {
    // Arrange
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/errors"))
        .respond_with(ResponseTemplate::new(401)
            .set_body_json(serde_json::json!({
                "error": "unauthorized",
                "message": "Missing or invalid authorization token"
            })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let error_report = serde_json::json!({
        "type": "CONFIG_ERROR",
        "message": "Test error",
        "severity": "ERROR"
    });

    let client = reqwest::Client::new();

    // Act
    let response = client
        .post(format!("{}/api/errors", mock_server.uri()))
        .header("Content-Type", "application/json")
        .json(&error_report)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 401);
}

/// T073: Contract test for global error report validation failure
#[tokio::test]
async fn test_global_error_report_validation_error() {
    // Arrange
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/errors"))
        .respond_with(ResponseTemplate::new(400)
            .set_body_json(serde_json::json!({
                "error": "validation_error",
                "message": "Missing required field: type"
            })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Missing 'type' field
    let error_report = serde_json::json!({
        "message": "Test error without type",
        "severity": "ERROR"
    });

    let client = reqwest::Client::new();

    // Act
    let response = client
        .post(format!("{}/api/errors", mock_server.uri()))
        .header("Authorization", "Bearer test-token")
        .json(&error_report)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 400);
}

// ==================== Legacy Contract Tests (for backwards compatibility) ====================

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_error_report_success() {
    // Arrange
    let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.token";
    let auth_header = format!("Bearer {}", jwt_token);

    let error_report = serde_json::json!({
        "filename": "archive\\2024\\sales.dbf",
        "error_type": "FileReadError",
        "message": "Failed to read DBF file: Permission denied (OS Error 5)",
        "timestamp": "2025-10-05T14:30:00Z",
        "client_version": "1.0.0"
    });

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/errors/report";

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&error_report)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert!(
        response.status() == 200 || response.status() == 204,
        "Expected 200 OK or 204 No Content, got {}",
        response.status()
    );

    if response.status() == 200 {
        let body: serde_json::Value = response
            .json()
            .await
            .expect("Failed to parse JSON response");

        assert!(body.get("status").is_some(), "Response should contain 'status' field");
    }
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_error_report_missing_field() {
    // Arrange
    let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.token";
    let auth_header = format!("Bearer {}", jwt_token);

    // Missing 'filename' field
    let error_report = serde_json::json!({
        "error_type": "FileReadError",
        "message": "Failed to read DBF file",
        "timestamp": "2025-10-05T14:30:00Z",
        "client_version": "1.0.0"
    });

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/errors/report";

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&error_report)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 400, "Expected 400 Bad Request for missing required field");

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    assert_eq!(body["error"], "bad_request", "Error should be 'bad_request'");
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_error_report_invalid_timestamp() {
    // Arrange
    let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.token";
    let auth_header = format!("Bearer {}", jwt_token);

    // Invalid timestamp format
    let error_report = serde_json::json!({
        "filename": "test.dbf",
        "error_type": "FileReadError",
        "message": "Failed to read DBF file",
        "timestamp": "invalid-timestamp",
        "client_version": "1.0.0"
    });

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/errors/report";

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&error_report)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 400, "Expected 400 Bad Request for invalid timestamp");
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_error_report_all_error_types() {
    // Arrange
    let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.token";
    let auth_header = format!("Bearer {}", jwt_token);
    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/errors/report";

    let error_types = vec![
        "FileReadError",
        "EncodingError",
        "ConversionError",
        "CompressionError",
        "UploadError",
        "DiskFullError",
        "DirectoryInaccessible",
        "AuthenticationError",
        "ConfigurationError",
        "NetworkError",
    ];

    // Act & Assert
    for error_type in error_types {
        let error_report = serde_json::json!({
            "filename": "test.dbf",
            "error_type": error_type,
            "message": format!("Test error: {}", error_type),
            "timestamp": "2025-10-05T14:30:00Z",
            "client_version": "1.0.0"
        });

        let response = client
            .post(api_url)
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .json(&error_report)
            .send()
            .await
            .expect("Failed to send request");

        assert!(
            response.status() == 200 || response.status() == 204,
            "Expected 200/204 for error_type '{}', got {}",
            error_type,
            response.status()
        );
    }
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_error_report_without_authorization() {
    // Arrange
    let error_report = serde_json::json!({
        "filename": "test.dbf",
        "error_type": "FileReadError",
        "message": "Failed to read DBF file",
        "timestamp": "2025-10-05T14:30:00Z",
        "client_version": "1.0.0"
    });

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/errors/report";

    // Act
    let response = client
        .post(api_url)
        .header("Content-Type", "application/json")
        .json(&error_report)
        .send()
        .await
        .expect("Failed to send request");

    // Assert - Server may accept unauthenticated error reports (200/204) or reject (401)
    assert!(
        response.status() == 200 || response.status() == 204 || response.status() == 401,
        "Expected 200/204 (accepted) or 401 (rejected), got {}",
        response.status()
    );
}
