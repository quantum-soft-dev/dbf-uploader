// File uploader for gzip files
use common::auth::JwtToken;
use common::error::{ProcessingError, Result};
use common::models::Config;
use crate::processor::ProcessingData;
use reqwest::{multipart, Client};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{debug, warn};

/// Upload gzip data (from memory or temp file) to the server
///
/// # Arguments
/// * `gzip_data` - Gzip data (in memory or temp file)
/// * `filename` - Filename to use in the upload (should include subdirectory encoding)
/// * `batch_id` - Batch ID from server
/// * `token` - JWT token for authentication
/// * `config` - Configuration containing API base URL
///
/// # Returns
/// Ok(()) if upload succeeds
pub async fn upload_data(
    gzip_data: ProcessingData,
    filename: String,
    batch_id: &str,
    token: &JwtToken,
    config: &Config,
) -> Result<()> {
    // Create HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(300)) // 5 minute timeout for large files
        .https_only(config.api.https_only)
        .build()
        .map_err(|e| {
            ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
        })?;

    match &gzip_data {
        ProcessingData::InMemory(gzip_bytes) => {
            debug!(
                "Uploading file from memory: {} ({} KB) for batch {}",
                filename,
                gzip_bytes.len() / 1024,
                batch_id
            );

            // Upload directly from memory
            upload_with_retry(&client, gzip_bytes, &filename, batch_id, token, config, 3).await
        }
        ProcessingData::TempFile(gzip_path) => {
            debug!(
                "Uploading file from temp: {} -> {} for batch {}",
                gzip_path.display(),
                filename,
                batch_id
            );

            // Read temp file into memory for upload
            let mut file = File::open(gzip_path)
                .await
                .map_err(ProcessingError::FileReadError)?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .await
                .map_err(ProcessingError::FileReadError)?;

            // Upload from memory
            upload_with_retry(&client, &buffer, &filename, batch_id, token, config, 3).await

            // Temp file will be deleted automatically by Drop when gzip_data goes out of scope
        }
    }
}

/// Upload a gzip file to the server (legacy function - reads from disk)
///
/// # Arguments
/// * `gzip_path` - Path to the gzip file to upload
/// * `filename` - Filename to use in the upload (should include subdirectory encoding)
/// * `batch_id` - Batch ID from server
/// * `token` - JWT token for authentication
/// * `config` - Configuration containing API base URL
///
/// # Returns
/// Ok(()) if upload succeeds
pub async fn upload_file(
    gzip_path: PathBuf,
    filename: String,
    batch_id: &str,
    token: &JwtToken,
    config: &Config,
) -> Result<()> {
    debug!(
        "Uploading file: {} as {} for batch {}",
        gzip_path.display(),
        filename,
        batch_id
    );

    // Create HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(300)) // 5 minute timeout for large files
        .https_only(config.api.https_only)
        .build()
        .map_err(|e| {
            ProcessingError::NetworkError(format!("Failed to create HTTP client: {}", e))
        })?;

    // Read file contents
    let mut file = File::open(&gzip_path)
        .await
        .map_err(ProcessingError::FileReadError)?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .await
        .map_err(ProcessingError::FileReadError)?;

    // Try upload with retries
    upload_with_retry(&client, &buffer, &filename, batch_id, token, config, 3).await
}

/// Upload with retry logic for transient failures
async fn upload_with_retry(
    client: &Client,
    file_data: &[u8],
    filename: &str,
    batch_id: &str,
    token: &JwtToken,
    config: &Config,
    max_retries: u32,
) -> Result<()> {
    let url = format!(
        "{}/api/v1/device/files/batches/{}/upload",
        config.api.base_url, batch_id
    );

    for attempt in 1..=max_retries {
        match try_upload(client, file_data, filename, token, &url).await {
            Ok(_) => {
                debug!("Upload successful: {}", filename);
                return Ok(());
            }
            Err(e) => {
                match &e {
                    ProcessingError::UploadError(msg) if msg.contains("401") => {
                        // Token expired - caller should renew and retry
                        return Err(e);
                    }
                    ProcessingError::UploadError(msg) if msg.starts_with("4") => {
                        // Client error (4xx) - don't retry, report and skip
                        warn!("Client error uploading {}: {}", filename, msg);
                        return Err(e);
                    }
                    ProcessingError::UploadError(msg) if msg.starts_with("5") => {
                        // Server error (5xx) - retry with backoff
                        if attempt < max_retries {
                            let backoff = Duration::from_secs(2u64.pow(attempt - 1));
                            warn!(
                                "Server error uploading {} (attempt {}/{}), retrying in {:?}: {}",
                                filename, attempt, max_retries, backoff, msg
                            );
                            tokio::time::sleep(backoff).await;
                            continue;
                        } else {
                            warn!("Upload failed after {} attempts: {}", max_retries, msg);
                            return Err(e);
                        }
                    }
                    ProcessingError::NetworkError(_) => {
                        // Network error - retry with backoff
                        if attempt < max_retries {
                            let backoff = Duration::from_secs(2u64.pow(attempt - 1));
                            warn!(
                                "Network error uploading {} (attempt {}/{}), retrying in {:?}",
                                filename, attempt, max_retries, backoff
                            );
                            tokio::time::sleep(backoff).await;
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                    _ => return Err(e),
                }
            }
        }
    }

    Err(ProcessingError::UploadError(format!(
        "Upload failed after {} retries",
        max_retries
    )))
}

/// Single upload attempt
async fn try_upload(
    client: &Client,
    file_data: &[u8],
    filename: &str,
    token: &JwtToken,
    url: &str,
) -> Result<()> {
    // Create multipart form
    let part = multipart::Part::bytes(file_data.to_vec())
        .file_name(filename.to_string())
        .mime_str("application/gzip")
        .map_err(|e| ProcessingError::UploadError(format!("Failed to create multipart: {}", e)))?;

    let form = multipart::Form::new().part("files", part);

    // Send request
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", token.token))
        .header("Content-Encoding", "gzip")
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            ProcessingError::NetworkError(format!("Failed to send upload request: {}", e))
        })?;

    // Check response status
    let status = response.status();

    if status.is_success() {
        Ok(())
    } else {
        let status_code = status.as_u16();
        let error_body = response.text().await.unwrap_or_default();

        Err(ProcessingError::UploadError(format!(
            "{} - {}",
            status_code, error_body
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::models::config::*;
    use wiremock::matchers::{header, method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_config_with_url(base_url: &str) -> Config {
        Config {
            scheduler: SchedulerConfig {
                crontab: "*/5 * * * *".to_string(),
            },
            src: SourceConfig {
                source_dir: std::path::PathBuf::from("/tmp"),
                include_patterns: None,
                exclude_patterns: None,
            },
            credential: CredentialConfig {
                account: "testaccount".to_string(),
                username: "test".to_string(),
                password: "test".to_string(),
                device: None,
            },
            api: ApiConfig {
                base_url: base_url.to_string(),
                https_only: false, // Allow HTTP for tests
            },
            encoding: EncodingConfig {
                dbf_encoding: "CP866".to_string(),
            },
        }
    }

    fn create_test_config() -> Config {
        create_test_config_with_url("https://api.test.com")
    }

    #[test]
    fn test_upload_nonexistent_file() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        runtime.block_on(async {
            let config = create_test_config();
            let token = JwtToken {
                token: "test_token".to_string(),
                expires_at: 9999999999,
            };

            let result = upload_file(
                PathBuf::from("/nonexistent/file.csv.gz"),
                "test.csv.gz".to_string(),
                "test-batch-id",
                &token,
                &config,
            )
            .await;

            assert!(result.is_err());
            match result {
                Err(ProcessingError::FileReadError(_)) => (),
                _ => panic!("Expected FileReadError"),
            }
        });
    }

    /// T048 - Test multipart form construction
    #[tokio::test]
    async fn test_multipart_form_construction() {
        // Arrange
        let mock_server = MockServer::start().await;
        let config = create_test_config_with_url(&mock_server.uri());
        let token = JwtToken {
            token: "test_token".to_string(),
            expires_at: 9999999999,
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/api/v1/device/files/batches/.*/upload"))
            .and(header("Authorization", "Bearer test_token"))
            .and(header("Content-Encoding", "gzip"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock_server)
            .await;

        let test_data = ProcessingData::InMemory(b"test gzip data".to_vec());

        // Act
        let result = upload_data(
            test_data,
            "test.csv.gz".to_string(),
            "test-batch-id",
            &token,
            &config,
        )
        .await;

        // Assert
        assert!(result.is_ok(), "Upload should succeed: {:?}", result);
    }

    /// T049 - Test retry logic (1s, 2s, 4s backoff) on 5xx errors
    #[tokio::test]
    async fn test_retry_on_5xx_errors() {
        // Arrange
        let mock_server = MockServer::start().await;
        let config = create_test_config_with_url(&mock_server.uri());
        let token = JwtToken {
            token: "test_token".to_string(),
            expires_at: 9999999999,
        };

        // First two requests return 500, third succeeds
        Mock::given(method("POST"))
            .and(path_regex(r"/api/v1/device/files/batches/.*/upload"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
            .expect(2)
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path_regex(r"/api/v1/device/files/batches/.*/upload"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock_server)
            .await;

        let test_data = ProcessingData::InMemory(b"test gzip data".to_vec());

        // Act
        let start = std::time::Instant::now();
        let result = upload_data(
            test_data,
            "test.csv.gz".to_string(),
            "test-batch-id",
            &token,
            &config,
        )
        .await;
        let elapsed = start.elapsed();

        // Assert
        assert!(result.is_ok(), "Upload should eventually succeed after retries");
        // Should have waited at least 1s + 2s = 3s for backoff
        assert!(
            elapsed.as_secs() >= 3,
            "Should have waited for exponential backoff: {:?}",
            elapsed
        );
    }

    /// T050 - Test no retry on 4xx client errors (except 401)
    #[tokio::test]
    async fn test_no_retry_on_4xx_errors() {
        // Arrange
        let mock_server = MockServer::start().await;
        let config = create_test_config_with_url(&mock_server.uri());
        let token = JwtToken {
            token: "test_token".to_string(),
            expires_at: 9999999999,
        };

        // Return 400 Bad Request - should not retry
        Mock::given(method("POST"))
            .and(path_regex(r"/api/v1/device/files/batches/.*/upload"))
            .respond_with(ResponseTemplate::new(400).set_body_string("Bad request"))
            .expect(1) // Should only be called once (no retry)
            .mount(&mock_server)
            .await;

        let test_data = ProcessingData::InMemory(b"test gzip data".to_vec());

        // Act
        let result = upload_data(
            test_data,
            "test.csv.gz".to_string(),
            "test-batch-id",
            &token,
            &config,
        )
        .await;

        // Assert
        assert!(result.is_err(), "Upload should fail on 400");
        match result {
            Err(ProcessingError::UploadError(msg)) => {
                assert!(msg.contains("400"), "Error should indicate 400 status");
            }
            _ => panic!("Expected UploadError"),
        }
    }

    /// T051 - Test 401 token expired returns error (for caller to refresh)
    #[tokio::test]
    async fn test_401_returns_error_for_token_refresh() {
        // Arrange
        let mock_server = MockServer::start().await;
        let config = create_test_config_with_url(&mock_server.uri());
        let token = JwtToken {
            token: "expired_token".to_string(),
            expires_at: 9999999999,
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/api/v1/device/files/batches/.*/upload"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .expect(1) // Should only be called once (no retry on 401)
            .mount(&mock_server)
            .await;

        let test_data = ProcessingData::InMemory(b"test gzip data".to_vec());

        // Act
        let result = upload_data(
            test_data,
            "test.csv.gz".to_string(),
            "test-batch-id",
            &token,
            &config,
        )
        .await;

        // Assert
        assert!(result.is_err(), "Upload should fail on 401");
        match result {
            Err(ProcessingError::UploadError(msg)) => {
                assert!(msg.contains("401"), "Error should indicate 401 status");
            }
            _ => panic!("Expected UploadError with 401"),
        }
    }

    /// T052 - Test 5-minute upload timeout
    #[test]
    fn test_upload_timeout_configuration() {
        // Verify that the HTTP client is configured with a 300-second timeout
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to build client");

        // We can't directly inspect the timeout, but we can verify the builder doesn't panic
        // The actual timeout behavior would need integration testing with a slow server
        drop(client);
    }

    /// Test upload with in-memory data
    #[tokio::test]
    async fn test_upload_in_memory_data_success() {
        // Arrange
        let mock_server = MockServer::start().await;
        let config = create_test_config_with_url(&mock_server.uri());
        let token = JwtToken {
            token: "test_token".to_string(),
            expires_at: 9999999999,
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/api/v1/device/files/batches/.*/upload"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&mock_server)
            .await;

        let test_data = ProcessingData::InMemory(vec![0x1f, 0x8b, 0x08, 0x00]); // gzip magic bytes

        // Act
        let result = upload_data(
            test_data,
            "subdir/test.csv.gz".to_string(),
            "batch-123",
            &token,
            &config,
        )
        .await;

        // Assert
        assert!(result.is_ok());
    }

    /// Test upload with temp file data
    #[tokio::test]
    async fn test_upload_temp_file_data_success() {
        use tokio::io::AsyncWriteExt;

        // Arrange
        let mock_server = MockServer::start().await;
        let config = create_test_config_with_url(&mock_server.uri());
        let token = JwtToken {
            token: "test_token".to_string(),
            expires_at: 9999999999,
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/api/v1/device/files/batches/.*/upload"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&mock_server)
            .await;

        // Create a temp file
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().join("test.csv.gz");
        let mut file = tokio::fs::File::create(&temp_path).await.unwrap();
        file.write_all(&[0x1f, 0x8b, 0x08, 0x00]).await.unwrap();
        drop(file);

        let test_data = ProcessingData::TempFile(temp_path);

        // Act
        let result = upload_data(
            test_data,
            "subdir/test.csv.gz".to_string(),
            "batch-123",
            &token,
            &config,
        )
        .await;

        // Assert
        assert!(result.is_ok());
    }

    /// Test all retries exhausted on persistent 5xx errors
    #[tokio::test]
    async fn test_retry_exhausted_on_persistent_5xx() {
        // Arrange
        let mock_server = MockServer::start().await;
        let config = create_test_config_with_url(&mock_server.uri());
        let token = JwtToken {
            token: "test_token".to_string(),
            expires_at: 9999999999,
        };

        // Always return 500
        Mock::given(method("POST"))
            .and(path_regex(r"/api/v1/device/files/batches/.*/upload"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
            .expect(3) // Should try 3 times (initial + 2 retries)
            .mount(&mock_server)
            .await;

        let test_data = ProcessingData::InMemory(b"test gzip data".to_vec());

        // Act
        let result = upload_data(
            test_data,
            "test.csv.gz".to_string(),
            "test-batch-id",
            &token,
            &config,
        )
        .await;

        // Assert
        assert!(result.is_err(), "Upload should fail after retries exhausted");
        match result {
            Err(ProcessingError::UploadError(msg)) => {
                assert!(
                    msg.contains("500"),
                    "Error should indicate server error: {}",
                    msg
                );
            }
            _ => panic!("Expected UploadError"),
        }
    }
}
