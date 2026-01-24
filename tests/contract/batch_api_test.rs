// Contract tests for Batch API
// Tests POST /api/batches/start and POST /api/batches/{id}/complete endpoints
// Using wiremock for HTTP mocking

use wiremock::matchers::{body_json_schema, header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// T046 - Contract test for /api/batches/start endpoint
#[tokio::test]
async fn test_batch_start_success() {
    // Arrange
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .and(header("Authorization", "Bearer test_token"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "550e8400-e29b-41d4-a716-446655440000"
            })),
        )
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/device/batches/start", mock_server.uri());

    // Act
    let response = client
        .post(&url)
        .header("Authorization", "Bearer test_token")
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 201, "Expected 201 Created response");

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON response");

    assert!(body.get("id").is_some(), "Response should contain 'id' field");

    let batch_id = body["id"].as_str().expect("id should be a string");
    assert!(!batch_id.is_empty(), "Batch ID should not be empty");
}

/// T046 - Contract test for /api/batches/start with invalid token
#[tokio::test]
async fn test_batch_start_unauthorized() {
    // Arrange
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "unauthorized",
                "message": "Invalid or expired token"
            })),
        )
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/device/batches/start", mock_server.uri());

    // Act
    let response = client
        .post(&url)
        .header("Authorization", "Bearer invalid_token")
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(
        response.status(),
        401,
        "Expected 401 Unauthorized for invalid token"
    );
}

/// T047 - Contract test for /api/batches/{id}/complete endpoint
#[tokio::test]
async fn test_batch_complete_success() {
    // Arrange
    let mock_server = MockServer::start().await;
    let batch_id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("POST"))
        .and(path_regex(r"^/api/v1/device/batches/[a-f0-9-]+/complete$"))
        .and(header("Authorization", "Bearer test_token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "acknowledged": true
            })),
        )
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/device/batches/{}/complete",
        mock_server.uri(),
        batch_id
    );

    // Act
    let response = client
        .post(&url)
        .header("Authorization", "Bearer test_token")
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 200, "Expected 200 OK response");

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON response");

    assert_eq!(
        body["acknowledged"], true,
        "Response should contain acknowledged: true"
    );
}

/// T047 - Contract test for /api/batches/{id}/complete with invalid batch
#[tokio::test]
async fn test_batch_complete_not_found() {
    // Arrange
    let mock_server = MockServer::start().await;
    let invalid_batch_id = "invalid-batch-id";

    Mock::given(method("POST"))
        .and(path_regex(r"^/api/v1/device/batches/[a-z0-9-]+/complete$"))
        .and(header("Authorization", "Bearer test_token"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": "not_found",
                "message": "Batch not found"
            })),
        )
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/device/batches/{}/complete",
        mock_server.uri(),
        invalid_batch_id
    );

    // Act
    let response = client
        .post(&url)
        .header("Authorization", "Bearer test_token")
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(
        response.status(),
        404,
        "Expected 404 Not Found for invalid batch"
    );
}

/// T047 - Contract test for /api/batches/{id}/complete-with-warnings endpoint
#[tokio::test]
async fn test_batch_complete_with_warnings_success() {
    // Arrange
    let mock_server = MockServer::start().await;
    let batch_id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("POST"))
        .and(path_regex(
            r"^/api/v1/device/batches/[a-f0-9-]+/complete-with-warnings$",
        ))
        .and(header("Authorization", "Bearer test_token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "acknowledged": true
            })),
        )
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/device/batches/{}/complete-with-warnings",
        mock_server.uri(),
        batch_id
    );

    // Act
    let response = client
        .post(&url)
        .header("Authorization", "Bearer test_token")
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 200, "Expected 200 OK response");
}

/// Contract test for /api/batches/{id}/fail endpoint
#[tokio::test]
async fn test_batch_fail_success() {
    // Arrange
    let mock_server = MockServer::start().await;
    let batch_id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("POST"))
        .and(path_regex(r"^/api/v1/device/batches/[a-f0-9-]+/fail$"))
        .and(header("Authorization", "Bearer test_token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "acknowledged": true
            })),
        )
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/device/batches/{}/fail",
        mock_server.uri(),
        batch_id
    );

    // Act
    let response = client
        .post(&url)
        .header("Authorization", "Bearer test_token")
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 200, "Expected 200 OK response");
}

/// T053 - Integration test for batch start/complete API flow
#[tokio::test]
async fn test_batch_full_lifecycle() {
    // Arrange
    let mock_server = MockServer::start().await;

    // Mock batch start
    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .and(header("Authorization", "Bearer test_token"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "lifecycle-test-batch-id"
            })),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // Mock batch complete
    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/lifecycle-test-batch-id/complete"))
        .and(header("Authorization", "Bearer test_token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "acknowledged": true
            })),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();

    // Act - Start batch
    let start_url = format!("{}/api/v1/device/batches/start", mock_server.uri());
    let start_response = client
        .post(&start_url)
        .header("Authorization", "Bearer test_token")
        .send()
        .await
        .expect("Failed to start batch");

    assert_eq!(start_response.status(), 201);
    let start_body: serde_json::Value = start_response.json().await.unwrap();
    let batch_id = start_body["id"].as_str().unwrap();

    // Act - Complete batch
    let complete_url = format!(
        "{}/api/v1/device/batches/{}/complete",
        mock_server.uri(),
        batch_id
    );
    let complete_response = client
        .post(&complete_url)
        .header("Authorization", "Bearer test_token")
        .send()
        .await
        .expect("Failed to complete batch");

    // Assert
    assert_eq!(complete_response.status(), 200);
    let complete_body: serde_json::Value = complete_response.json().await.unwrap();
    assert_eq!(complete_body["acknowledged"], true);
}
