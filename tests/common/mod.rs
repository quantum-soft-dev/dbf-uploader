// Common test utilities for contract and integration tests

use base64::Engine;
use mockito::{Mock, Server, ServerGuard};
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

/// Mock middleware server for testing
pub struct MockMiddleware {
    server: ServerGuard,
}

impl MockMiddleware {
    /// Create a new mock middleware server
    pub async fn new() -> Self {
        let server = Server::new_async().await;
        Self { server }
    }

    /// Get the base URL of the mock server
    pub fn url(&self) -> String {
        self.server.url()
    }

    /// Get a reference to the mock server for custom mocks
    pub fn get_server(&mut self) -> &mut mockito::ServerGuard {
        &mut self.server
    }

    /// Create a mock for successful authentication (v2 protocol)
    pub fn mock_auth_success(
        &mut self,
        domain: &str,
        client_secret: &str,
        token: &str,
        expires_in: u64,
    ) -> Mock {
        let credentials = format!("{}:{}", domain, client_secret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        self.server
            .mock("POST", "/api/v1/auth/token")
            .match_header("authorization", auth_header.as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"token": "{}", "expiresIn": {}, "tokenType": "Bearer"}}"#,
                token, expires_in
            ))
            .create()
    }

    /// Create a mock for failed authentication (401)
    pub fn mock_auth_invalid_credentials(&mut self) -> Mock {
        self.server
            .mock("POST", "/api/v1/auth/token")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid credentials"}"#)
            .create()
    }

    /// Create a mock for inactive subscription (403)
    pub fn mock_auth_inactive_subscription(&mut self, domain: &str, client_secret: &str) -> Mock {
        let credentials = format!("{}:{}", domain, client_secret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        self.server
            .mock("POST", "/api/v1/auth/token")
            .match_header("authorization", auth_header.as_str())
            .with_status(403)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "subscription_inactive"}"#)
            .create()
    }

    /// Create a mock for batch start
    /// Returns full BatchResponseDto matching server's BatchController.java
    pub fn mock_batch_start(&mut self, token: &str, batch_id: Uuid, site_id: Uuid) -> Mock {
        self.server
            .mock("POST", "/api/dfc/batch/start")
            .match_header("authorization", format!("Bearer {}", token).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"id": "{}", "batchId": "{}", "siteId": "{}", "status": "IN_PROGRESS", "s3Path": "account123/site456/2025-10-17/batch123", "uploadedFilesCount": 0, "totalSize": 0, "hasErrors": false, "startedAt": "2025-10-17T10:00:00Z", "completedAt": null}}"#,
                batch_id, batch_id, site_id
            ))
            .create()
    }

    /// Create a mock for batch upload
    /// Matches server response from FileUploadController.java
    pub fn mock_batch_upload(
        &mut self,
        token: &str,
        batch_id: Uuid,
        uploaded_count: u32,
        total_size: u64,
    ) -> Mock {
        self.server
            .mock("POST", format!("/api/dfc/batch/{}/upload", batch_id).as_str())
            .match_header("authorization", format!("Bearer {}", token).as_str())
            .match_header("content-type", mockito::Matcher::Regex("multipart/form-data.*".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"status": "OK", "uploadedFiles": {}, "files": [{{"fileName": "test.csv.gz", "fileSize": {}, "uploadedAt": "2025-10-06T10:35:00Z"}}]}}"#,
                uploaded_count, total_size
            ))
            .create()
    }

    /// Create a mock for batch complete
    /// Returns full BatchResponseDto matching server's BatchController.java
    pub fn mock_batch_complete(
        &mut self,
        token: &str,
        batch_id: Uuid,
        site_id: Uuid,
        uploaded_count: i32,
        total_size: i64,
    ) -> Mock {
        self.server
            .mock("POST", format!("/api/dfc/batch/{}/complete", batch_id).as_str())
            .match_header("authorization", format!("Bearer {}", token).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"id": "{}", "batchId": "{}", "siteId": "{}", "status": "COMPLETED", "s3Path": "account123/site456/2025-10-17/batch123", "uploadedFilesCount": {}, "totalSize": {}, "hasErrors": false, "startedAt": "2025-10-17T10:00:00Z", "completedAt": "2025-10-17T11:30:00Z"}}"#,
                batch_id, batch_id, site_id, uploaded_count, total_size
            ))
            .create()
    }

    /// Create a mock for batch fail
    /// Returns full BatchResponseDto matching server's BatchController.java
    /// Note: Server does NOT accept request body
    pub fn mock_batch_fail(&mut self, token: &str, batch_id: Uuid, site_id: Uuid) -> Mock {
        self.server
            .mock("POST", format!("/api/dfc/batch/{}/fail", batch_id).as_str())
            .match_header("authorization", format!("Bearer {}", token).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"id": "{}", "batchId": "{}", "siteId": "{}", "status": "FAILED", "s3Path": "account123/site456/2025-10-17/batch123", "uploadedFilesCount": 0, "totalSize": 0, "hasErrors": true, "startedAt": "2025-10-17T10:00:00Z", "completedAt": "2025-10-17T10:15:00Z"}}"#,
                batch_id, batch_id, site_id
            ))
            .create()
    }

    /// Create a mock for batch cancel
    /// Returns full BatchResponseDto matching server's BatchController.java
    pub fn mock_batch_cancel(&mut self, token: &str, batch_id: Uuid, site_id: Uuid) -> Mock {
        self.server
            .mock("POST", format!("/api/dfc/batch/{}/cancel", batch_id).as_str())
            .match_header("authorization", format!("Bearer {}", token).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"id": "{}", "batchId": "{}", "siteId": "{}", "status": "CANCELLED", "s3Path": "account123/site456/2025-10-17/batch123", "uploadedFilesCount": 0, "totalSize": 0, "hasErrors": false, "startedAt": "2025-10-17T10:00:00Z", "completedAt": "2025-10-17T10:10:00Z"}}"#,
                batch_id, batch_id, site_id
            ))
            .create()
    }

    /// Create a mock for error reporting
    pub fn mock_error_report(&mut self, token: &str, batch_id: Option<Uuid>) -> Mock {
        let path = match batch_id {
            Some(id) => format!("/api/v1/error/{}", id),
            None => "/api/v1/error".to_string(),
        };

        self.server
            .mock("POST", path.as_str())
            .match_header("authorization", format!("Bearer {}", token).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": true}"#)
            .create()
    }
}

/// Test fixture for creating temporary DBF files
pub struct DbfTestFixture {
    pub temp_dir: TempDir,
}

impl DbfTestFixture {
    /// Create a new DBF test fixture with temp directory
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        Self { temp_dir }
    }

    /// Get the path to the temp directory
    pub fn dir_path(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }

    /// Create a simple test DBF file using dbase library
    pub fn create_test_dbf(&self, filename: &str) -> PathBuf {
        use dbase::{FieldName, FieldValue, TableWriterBuilder};
        use std::fs::File;

        let dbf_path = self.temp_dir.path().join(filename);

        // Create DBF file using dbase library
        let file = File::create(&dbf_path).expect("Failed to create DBF file");

        let mut table_writer = TableWriterBuilder::new()
            .add_character_field(FieldName::try_from("NAME").unwrap(), 10)
            .add_character_field(FieldName::try_from("CODE").unwrap(), 5)
            .add_numeric_field(FieldName::try_from("VALUE").unwrap(), 8, 2)
            .build_with_dest(file);

        // Write a single test record
        let mut record = dbase::Record::default();
        record.insert(
            "NAME".to_string(),
            FieldValue::Character(Some("TestName".to_string())),
        );
        record.insert(
            "CODE".to_string(),
            FieldValue::Character(Some("TEST".to_string())),
        );
        record.insert("VALUE".to_string(), FieldValue::Numeric(Some(123.45)));

        table_writer
            .write_record(&record)
            .expect("Failed to write record");

        // Finalize the file
        drop(table_writer);

        dbf_path
    }

    /// Create multiple test DBF files
    pub fn create_multiple_dbf(&self, count: usize) -> Vec<PathBuf> {
        (0..count)
            .map(|i| self.create_test_dbf(&format!("test_{}.dbf", i)))
            .collect()
    }
}

/// Helper to create a test JWT token
pub fn create_test_jwt(site_id: Uuid, account_id: Uuid, domain: &str, exp: u64) -> String {
    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let payload = format!(
        r#"{{"siteId":"{}","accountId":"{}","domain":"{}","exp":{}}}"#,
        site_id, account_id, domain, exp
    );

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header.as_bytes());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.as_bytes());

    format!("{}.{}.fake_signature", header_b64, payload_b64)
}

/// Helper to encode Basic Auth header
pub fn encode_basic_auth(domain: &str, client_secret: &str) -> String {
    let credentials = format!("{}:{}", domain, client_secret);
    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
    format!("Basic {}", encoded)
}

/// Test data constants
pub mod test_data {
    use uuid::Uuid;

    pub const TEST_DOMAIN: &str = "store-01.example.com";
    pub const TEST_CLIENT_SECRET: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
    pub const TEST_BASE_URL: &str = "http://127.0.0.1";

    /// Generate a test site ID
    pub fn test_site_id() -> Uuid {
        Uuid::parse_str("12345678-1234-1234-1234-123456789012").unwrap()
    }

    /// Generate a test account ID
    pub fn test_account_id() -> Uuid {
        Uuid::parse_str("87654321-4321-4321-4321-210987654321").unwrap()
    }

    /// Generate a test batch ID
    pub fn test_batch_id() -> Uuid {
        Uuid::parse_str("abcdef12-3456-7890-abcd-ef1234567890").unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbf_fixture_creation() {
        let fixture = DbfTestFixture::new();
        assert!(fixture.dir_path().exists());
    }

    #[test]
    fn test_create_test_dbf() {
        let fixture = DbfTestFixture::new();
        let dbf_path = fixture.create_test_dbf("test.dbf");

        assert!(dbf_path.exists());
        assert_eq!(dbf_path.extension().unwrap(), "dbf");

        // Verify file has content
        let content = std::fs::read(&dbf_path).unwrap();
        assert!(!content.is_empty());
        assert_eq!(content[0], 0x03); // dBASE III version
    }

    #[test]
    fn test_create_multiple_dbf() {
        let fixture = DbfTestFixture::new();
        let files = fixture.create_multiple_dbf(5);

        assert_eq!(files.len(), 5);
        for file in files {
            assert!(file.exists());
        }
    }

    #[test]
    fn test_create_test_jwt() {
        let site_id = test_data::test_site_id();
        let account_id = test_data::test_account_id();
        let token = create_test_jwt(site_id, account_id, "test.com", 9999999999);

        // JWT should have 3 parts separated by dots
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_encode_basic_auth() {
        let auth = encode_basic_auth("test.com", "secret123");
        assert!(auth.starts_with("Basic "));

        // Decode and verify
        let encoded = auth.strip_prefix("Basic ").unwrap();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let decoded_str = String::from_utf8(decoded).unwrap();
        assert_eq!(decoded_str, "test.com:secret123");
    }
}
