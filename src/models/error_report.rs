// Error Report model for data_exporter
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReport {
    /// Classification of error (maps to API 'type' field)
    #[serde(rename = "type")]
    pub error_type: String,

    /// Human-readable error description
    pub message: String,

    /// Detailed error information including source chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<String>,

    /// Optional additional error context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl ErrorReport {
    /// Create a new error report with metadata
    pub fn new(
        filename: String,
        error_type: String,
        message: String,
        error_details: Option<String>,
        client_version: String,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Build metadata with all context info
        let mut metadata = HashMap::new();
        metadata.insert("filename".to_string(), serde_json::Value::String(filename));
        metadata.insert(
            "clientVersion".to_string(),
            serde_json::Value::String(client_version),
        );
        metadata.insert(
            "timestamp".to_string(),
            serde_json::Value::String(timestamp),
        );

        Self {
            error_type,
            message,
            error_details,
            metadata: Some(metadata),
        }
    }

    /// Truncate message to maximum length (1000 characters per API spec)
    pub fn truncate_message(&mut self, max_len: usize) {
        if self.message.len() > max_len {
            self.message.truncate(max_len);
            self.message.push_str("... (truncated)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_report_creation() {
        let report = ErrorReport::new(
            "test.dbf".to_string(),
            "FileReadError".to_string(),
            "Failed to read file".to_string(),
            None,
            "1.0.0".to_string(),
        );

        assert_eq!(report.error_type, "FileReadError");
        assert_eq!(report.message, "Failed to read file");
        assert_eq!(report.error_details, None);
        assert!(report.metadata.is_some());

        let metadata = report.metadata.unwrap();
        assert_eq!(
            metadata.get("filename").unwrap().as_str().unwrap(),
            "test.dbf"
        );
        assert_eq!(
            metadata.get("clientVersion").unwrap().as_str().unwrap(),
            "1.0.0"
        );
        assert!(metadata.contains_key("timestamp"));
    }

    #[test]
    fn test_error_report_truncate_message() {
        let mut report = ErrorReport::new(
            "test.dbf".to_string(),
            "FileReadError".to_string(),
            "x".repeat(2500),
            None,
            "1.0.0".to_string(),
        );

        report.truncate_message(2000);
        assert!(report.message.len() <= 2020); // 2000 + "... (truncated)"
        assert!(report.message.ends_with("... (truncated)"));
    }

    #[test]
    fn test_error_report_serialization() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "filename".to_string(),
            serde_json::Value::String("test.dbf".to_string()),
        );

        let report = ErrorReport {
            error_type: "FileReadError".to_string(),
            message: "Failed to read file".to_string(),
            error_details: None,
            metadata: Some(metadata),
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"type\":\"FileReadError\""));
        assert!(json.contains("\"message\":\"Failed to read file\""));
        assert!(json.contains("\"metadata\""));
        // error_details should not appear when None
        assert!(!json.contains("error_details"));
    }

    #[test]
    fn test_error_report_with_details() {
        let report = ErrorReport::new(
            "test.dbf".to_string(),
            "ConversionError".to_string(),
            "Failed to convert file".to_string(),
            Some("Caused by: Invalid DBF header\n  Caused by: I/O error".to_string()),
            "1.0.0".to_string(),
        );

        assert_eq!(report.error_type, "ConversionError");
        assert_eq!(report.message, "Failed to convert file");
        assert!(report.error_details.is_some());
        assert!(report
            .error_details
            .as_ref()
            .unwrap()
            .contains("Caused by: Invalid DBF header"));

        // Test serialization includes error_details when present
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"error_details\""));
        assert!(json.contains("Invalid DBF header"));
    }
}
