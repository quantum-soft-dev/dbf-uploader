// Contract test for Upload API
// Tests POST /api/files/upload endpoint

use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_upload_file_success() {
    // Arrange
    let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.token";
    let auth_header = format!("Bearer {}", jwt_token);

    // Create a temporary gzip file
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(b"test gzip content").expect("Failed to write to temp file");
    let file_path = temp_file.path();

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/files/upload";

    // Create multipart form
    let form = reqwest::multipart::Form::new()
        .file("file", file_path)
        .await
        .expect("Failed to create multipart form");

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .multipart(form)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 201, "Expected 201 Created response");

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    assert_eq!(body["status"], "uploaded", "Status should be 'uploaded'");
    assert!(body.get("file_id").is_some(), "Response should contain 'file_id'");
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_upload_file_invalid_token() {
    // Arrange
    let auth_header = "Bearer invalid.token.here";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(b"test content").expect("Failed to write to temp file");
    let file_path = temp_file.path();

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/files/upload";

    let form = reqwest::multipart::Form::new()
        .file("file", file_path)
        .await
        .expect("Failed to create multipart form");

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .multipart(form)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 401, "Expected 401 Unauthorized for invalid token");
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_upload_file_missing_authorization() {
    // Arrange
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(b"test content").expect("Failed to write to temp file");
    let file_path = temp_file.path();

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/files/upload";

    let form = reqwest::multipart::Form::new()
        .file("file", file_path)
        .await
        .expect("Failed to create multipart form");

    // Act
    let response = client
        .post(api_url)
        .multipart(form)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 401, "Expected 401 Unauthorized when Authorization header is missing");
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_upload_file_missing_file_field() {
    // Arrange
    let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.token";
    let auth_header = format!("Bearer {}", jwt_token);

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/files/upload";

    // Create empty multipart form (missing 'file' field)
    let form = reqwest::multipart::Form::new();

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .multipart(form)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 400, "Expected 400 Bad Request when 'file' field is missing");

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    assert_eq!(body["error"], "bad_request", "Error should be 'bad_request'");
}

#[tokio::test]
#[ignore] // Ignored until we have test server or mock
async fn test_upload_file_not_gzip() {
    // Arrange
    let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.token";
    let auth_header = format!("Bearer {}", jwt_token);

    // Create a temp file without .gz extension
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(b"not gzipped content").expect("Failed to write to temp file");
    let file_path = temp_file.path();

    let client = reqwest::Client::new();
    let api_url = "https://api.example.com/api/files/upload";

    let form = reqwest::multipart::Form::new()
        .file("file", file_path)
        .await
        .expect("Failed to create multipart form");

    // Act
    let response = client
        .post(api_url)
        .header("Authorization", auth_header)
        .multipart(form)
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_eq!(response.status(), 400, "Expected 400 Bad Request for non-gzip file");

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    assert_eq!(body["error"], "invalid_file_type", "Error should be 'invalid_file_type'");
}
