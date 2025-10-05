// Contract test for Error Report API
// Tests POST /api/errors/report endpoint

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
