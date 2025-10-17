use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Response from batch start endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchStartResponse {
    pub batch_id: Uuid,
}

/// Information about a single uploaded file
/// Matches server response from FileUploadController.java
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFileInfo {
    pub file_name: String,
    pub file_size: u64,
    pub uploaded_at: String,
}

/// Response from batch upload endpoint
/// Matches server response from FileUploadController.java
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResponse {
    pub status: String,
    pub uploaded_files: usize, // Count of files, not array
    pub files: Vec<UploadedFileInfo>, // Actual file info array
}

/// Response from batch complete endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCompleteResponse {
    pub batch_id: Uuid,
    pub status: String,
    pub uploaded_files_count: u32,
    pub total_size: u64,
    pub has_errors: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_start_response_deserialization() {
        let json = r#"{"batchId":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let response: BatchStartResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.batch_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_upload_response_deserialization() {
        let json = r#"{
            "status": "OK",
            "uploadedFiles": 2,
            "files": [
                {
                    "fileName": "test.csv.gz",
                    "fileSize": 1024,
                    "uploadedAt": "2025-10-06T10:35:00Z"
                },
                {
                    "fileName": "data.csv.gz",
                    "fileSize": 2048,
                    "uploadedAt": "2025-10-06T10:35:01Z"
                }
            ]
        }"#;
        let response: UploadResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "OK");
        assert_eq!(response.uploaded_files, 2);
        assert_eq!(response.files.len(), 2);
        assert_eq!(response.files[0].file_name, "test.csv.gz");
        assert_eq!(response.files[0].file_size, 1024);
    }

    #[test]
    fn test_batch_complete_response_deserialization() {
        let json = r#"{
            "batchId": "550e8400-e29b-41d4-a716-446655440000",
            "status": "Completed",
            "uploadedFilesCount": 5,
            "totalSize": 5120,
            "hasErrors": false
        }"#;
        let response: BatchCompleteResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.uploaded_files_count, 5);
        assert_eq!(response.total_size, 5120);
        assert!(!response.has_errors);
    }
}
