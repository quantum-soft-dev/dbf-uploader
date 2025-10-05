// Error Report model for data_exporter
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReport {
    /// Name/path of file that caused error
    pub filename: String,

    /// Classification of error
    pub error_type: String,

    /// Human-readable error description
    pub message: String,

    /// When error occurred (ISO 8601 format)
    pub timestamp: String,

    /// Version of data exporter service
    pub client_version: String,
}

impl ErrorReport {
    /// Create a new error report
    pub fn new(
        filename: String,
        error_type: String,
        message: String,
        client_version: String,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        Self {
            filename,
            error_type,
            message,
            timestamp,
            client_version,
        }
    }

    /// Truncate message to maximum length (2000 characters as recommended)
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
            "1.0.0".to_string(),
        );

        assert_eq!(report.filename, "test.dbf");
        assert_eq!(report.error_type, "FileReadError");
        assert_eq!(report.message, "Failed to read file");
        assert_eq!(report.client_version, "1.0.0");
        assert!(!report.timestamp.is_empty());
    }

    #[test]
    fn test_error_report_truncate_message() {
        let mut report = ErrorReport::new(
            "test.dbf".to_string(),
            "FileReadError".to_string(),
            "x".repeat(2500),
            "1.0.0".to_string(),
        );

        report.truncate_message(2000);
        assert!(report.message.len() <= 2020); // 2000 + "... (truncated)"
        assert!(report.message.ends_with("... (truncated)"));
    }

    #[test]
    fn test_error_report_serialization() {
        let report = ErrorReport {
            filename: "test.dbf".to_string(),
            error_type: "FileReadError".to_string(),
            message: "Failed to read file".to_string(),
            timestamp: "2025-10-05T14:30:00Z".to_string(),
            client_version: "1.0.0".to_string(),
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"filename\":\"test.dbf\""));
        assert!(json.contains("\"error_type\":\"FileReadError\""));
    }
}
