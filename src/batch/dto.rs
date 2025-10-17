use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unified batch response DTO matching server's BatchResponseDto.java
/// Used for all batch endpoints: start, complete, fail, cancel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResponseDto {
    pub id: Uuid,
    pub batch_id: Uuid, // Alias for id (backward compatibility)
    pub site_id: Uuid,
    pub status: String,
    pub s3_path: String,
    pub uploaded_files_count: i32,
    pub total_size: i64,
    pub has_errors: bool,
    pub started_at: String, // ISO 8601 timestamp
    pub completed_at: Option<String>, // Nullable - null for active batches
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_response_dto_deserialization() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "batchId": "550e8400-e29b-41d4-a716-446655440000",
            "siteId": "660e8400-e29b-41d4-a716-446655440000",
            "status": "IN_PROGRESS",
            "s3Path": "account123/site456/2025-10-17/batch123",
            "uploadedFilesCount": 0,
            "totalSize": 0,
            "hasErrors": false,
            "startedAt": "2025-10-17T10:00:00Z",
            "completedAt": null
        }"#;
        let response: BatchResponseDto = serde_json::from_str(json).unwrap();
        assert_eq!(response.id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(response.batch_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(response.status, "IN_PROGRESS");
        assert_eq!(response.uploaded_files_count, 0);
        assert_eq!(response.total_size, 0);
        assert!(!response.has_errors);
        assert!(response.completed_at.is_none());
    }

    #[test]
    fn test_batch_response_dto_completed() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "batchId": "550e8400-e29b-41d4-a716-446655440000",
            "siteId": "660e8400-e29b-41d4-a716-446655440000",
            "status": "COMPLETED",
            "s3Path": "account123/site456/2025-10-17/batch123",
            "uploadedFilesCount": 5,
            "totalSize": 5120,
            "hasErrors": false,
            "startedAt": "2025-10-17T10:00:00Z",
            "completedAt": "2025-10-17T11:30:00Z"
        }"#;
        let response: BatchResponseDto = serde_json::from_str(json).unwrap();
        assert_eq!(response.uploaded_files_count, 5);
        assert_eq!(response.total_size, 5120);
        assert!(!response.has_errors);
        assert_eq!(response.completed_at, Some("2025-10-17T11:30:00Z".to_string()));
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
}
