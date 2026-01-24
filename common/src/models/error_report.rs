// Error Report model for data_exporter
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error severity levels for global error reporting
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorSeverity {
    /// System failure, data loss risk (database down, file corruption, security breach)
    Critical,
    /// Operation failed (connection failed, service unavailable) - default
    #[default]
    Error,
    /// Degraded performance, recoverable (slow network, disk space low, retry succeeded)
    Warning,
    /// Informational, expected condition (service started, config reloaded, health check)
    Info,
}

/// Batch-specific error report (for errors during batch processing)
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

/// Global error report (for errors outside batch context)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalErrorReport {
    /// Error type/category (max 100 chars)
    #[serde(rename = "type")]
    pub error_type: String,

    /// Detailed error message (max 10000 chars)
    pub message: String,

    /// Error severity level
    #[serde(default)]
    pub severity: ErrorSeverity,

    /// Additional error context (max 20 keys, 10KB total)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl GlobalErrorReport {
    /// Create a new global error report
    pub fn new(error_type: String, message: String, severity: ErrorSeverity) -> Self {
        Self {
            error_type,
            message,
            severity,
            metadata: None,
        }
    }

    /// Create a new global error report with metadata
    pub fn with_metadata(
        error_type: String,
        message: String,
        severity: ErrorSeverity,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            error_type,
            message,
            severity,
            metadata: Some(metadata),
        }
    }

    /// Add metadata to the error report
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        if let Some(ref mut map) = self.metadata {
            map.insert(key, value);
        } else {
            let mut map = HashMap::new();
            map.insert(key, value);
            self.metadata = Some(map);
        }
    }

    /// Truncate message to maximum length (10000 characters per API spec)
    pub fn truncate_message(&mut self, max_len: usize) {
        if self.message.len() > max_len {
            self.message.truncate(max_len);
            self.message.push_str("... (truncated)");
        }
    }

    /// Truncate error type to maximum length (100 characters per API spec)
    pub fn truncate_error_type(&mut self, max_len: usize) {
        if self.error_type.len() > max_len {
            self.error_type.truncate(max_len);
        }
    }

    /// Validate and truncate all fields according to API limits
    pub fn validate_and_truncate(&mut self) {
        self.truncate_error_type(100);
        self.truncate_message(10000);

        // Limit metadata to 20 keys
        if let Some(ref mut metadata) = self.metadata {
            if metadata.len() > 20 {
                tracing::warn!("Metadata has {} keys, truncating to 20", metadata.len());
                // Keep only first 20 keys
                let keys_to_remove: Vec<String> = metadata.keys().skip(20).cloned().collect();
                for key in keys_to_remove {
                    metadata.remove(&key);
                }
            }
        }
    }

    /// Convenience constructors for different severity levels
    pub fn critical(error_type: String, message: String) -> Self {
        Self::new(error_type, message, ErrorSeverity::Critical)
    }

    pub fn error(error_type: String, message: String) -> Self {
        Self::new(error_type, message, ErrorSeverity::Error)
    }

    pub fn warning(error_type: String, message: String) -> Self {
        Self::new(error_type, message, ErrorSeverity::Warning)
    }

    pub fn info(error_type: String, message: String) -> Self {
        Self::new(error_type, message, ErrorSeverity::Info)
    }
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

    // ==================== GlobalErrorReport Tests ====================

    #[test]
    fn test_validate_and_truncate_error_type_truncated_to_100_chars() {
        let long_error_type = "x".repeat(150);
        let mut report = GlobalErrorReport::new(
            long_error_type,
            "Test message".to_string(),
            ErrorSeverity::Error,
        );

        assert_eq!(report.error_type.len(), 150);
        report.validate_and_truncate();
        assert_eq!(report.error_type.len(), 100);
        assert_eq!(report.error_type, "x".repeat(100));
    }

    #[test]
    fn test_validate_and_truncate_error_type_not_truncated_when_under_limit() {
        let short_error_type = "x".repeat(50);
        let mut report = GlobalErrorReport::new(
            short_error_type.clone(),
            "Test message".to_string(),
            ErrorSeverity::Error,
        );

        report.validate_and_truncate();
        assert_eq!(report.error_type.len(), 50);
        assert_eq!(report.error_type, short_error_type);
    }

    #[test]
    fn test_validate_and_truncate_message_truncated_to_10000_chars() {
        let long_message = "y".repeat(15000);
        let mut report =
            GlobalErrorReport::new("TestError".to_string(), long_message, ErrorSeverity::Error);

        assert_eq!(report.message.len(), 15000);
        report.validate_and_truncate();
        // After truncation: 10000 chars + "... (truncated)" = 10015 chars
        assert!(report.message.len() <= 10015);
        assert!(report.message.ends_with("... (truncated)"));
        assert!(report.message.starts_with(&"y".repeat(10000)));
    }

    #[test]
    fn test_validate_and_truncate_message_not_truncated_when_under_limit() {
        let short_message = "y".repeat(5000);
        let mut report = GlobalErrorReport::new(
            "TestError".to_string(),
            short_message.clone(),
            ErrorSeverity::Error,
        );

        report.validate_and_truncate();
        assert_eq!(report.message.len(), 5000);
        assert_eq!(report.message, short_message);
    }

    #[test]
    fn test_validate_and_truncate_metadata_limited_to_20_keys() {
        let mut metadata = HashMap::new();
        for i in 0..30 {
            metadata.insert(format!("key{:02}", i), serde_json::json!(i));
        }

        let mut report = GlobalErrorReport::with_metadata(
            "TestError".to_string(),
            "Test message".to_string(),
            ErrorSeverity::Error,
            metadata,
        );

        assert_eq!(report.metadata.as_ref().unwrap().len(), 30);
        report.validate_and_truncate();
        assert_eq!(report.metadata.as_ref().unwrap().len(), 20);
    }

    #[test]
    fn test_validate_and_truncate_metadata_not_truncated_when_under_limit() {
        let mut metadata = HashMap::new();
        for i in 0..15 {
            metadata.insert(format!("key{}", i), serde_json::json!(i));
        }

        let mut report = GlobalErrorReport::with_metadata(
            "TestError".to_string(),
            "Test message".to_string(),
            ErrorSeverity::Error,
            metadata,
        );

        assert_eq!(report.metadata.as_ref().unwrap().len(), 15);
        report.validate_and_truncate();
        assert_eq!(report.metadata.as_ref().unwrap().len(), 15);
    }

    #[test]
    fn test_validate_and_truncate_no_metadata() {
        let mut report = GlobalErrorReport::new(
            "TestError".to_string(),
            "Test message".to_string(),
            ErrorSeverity::Error,
        );

        assert!(report.metadata.is_none());
        report.validate_and_truncate();
        assert!(report.metadata.is_none());
    }

    // ==================== ErrorSeverity Serialization Tests ====================

    #[test]
    fn test_error_severity_serialization_critical() {
        let severity = ErrorSeverity::Critical;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, "\"CRITICAL\"");
    }

    #[test]
    fn test_error_severity_serialization_error() {
        let severity = ErrorSeverity::Error;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, "\"ERROR\"");
    }

    #[test]
    fn test_error_severity_serialization_warning() {
        let severity = ErrorSeverity::Warning;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, "\"WARNING\"");
    }

    #[test]
    fn test_error_severity_serialization_info() {
        let severity = ErrorSeverity::Info;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, "\"INFO\"");
    }

    #[test]
    fn test_error_severity_deserialization() {
        let critical: ErrorSeverity = serde_json::from_str("\"CRITICAL\"").unwrap();
        assert_eq!(critical, ErrorSeverity::Critical);

        let error: ErrorSeverity = serde_json::from_str("\"ERROR\"").unwrap();
        assert_eq!(error, ErrorSeverity::Error);

        let warning: ErrorSeverity = serde_json::from_str("\"WARNING\"").unwrap();
        assert_eq!(warning, ErrorSeverity::Warning);

        let info: ErrorSeverity = serde_json::from_str("\"INFO\"").unwrap();
        assert_eq!(info, ErrorSeverity::Info);
    }

    #[test]
    fn test_error_severity_default() {
        let default_severity = ErrorSeverity::default();
        assert_eq!(default_severity, ErrorSeverity::Error);
    }

    // ==================== Convenience Constructor Tests ====================

    #[test]
    fn test_global_error_report_critical_constructor() {
        let report = GlobalErrorReport::critical(
            "CriticalError".to_string(),
            "Critical failure occurred".to_string(),
        );

        assert_eq!(report.error_type, "CriticalError");
        assert_eq!(report.message, "Critical failure occurred");
        assert_eq!(report.severity, ErrorSeverity::Critical);
        assert!(report.metadata.is_none());
    }

    #[test]
    fn test_global_error_report_error_constructor() {
        let report =
            GlobalErrorReport::error("OperationError".to_string(), "Operation failed".to_string());

        assert_eq!(report.error_type, "OperationError");
        assert_eq!(report.message, "Operation failed");
        assert_eq!(report.severity, ErrorSeverity::Error);
        assert!(report.metadata.is_none());
    }

    #[test]
    fn test_global_error_report_warning_constructor() {
        let report = GlobalErrorReport::warning(
            "PerformanceWarning".to_string(),
            "Slow network detected".to_string(),
        );

        assert_eq!(report.error_type, "PerformanceWarning");
        assert_eq!(report.message, "Slow network detected");
        assert_eq!(report.severity, ErrorSeverity::Warning);
        assert!(report.metadata.is_none());
    }

    #[test]
    fn test_global_error_report_info_constructor() {
        let report = GlobalErrorReport::info(
            "ServiceStatus".to_string(),
            "Service started successfully".to_string(),
        );

        assert_eq!(report.error_type, "ServiceStatus");
        assert_eq!(report.message, "Service started successfully");
        assert_eq!(report.severity, ErrorSeverity::Info);
        assert!(report.metadata.is_none());
    }

    // ==================== add_metadata() Method Tests ====================

    #[test]
    fn test_add_metadata_to_empty_metadata() {
        let mut report = GlobalErrorReport::new(
            "TestError".to_string(),
            "Test message".to_string(),
            ErrorSeverity::Error,
        );

        assert!(report.metadata.is_none());

        report.add_metadata("key1".to_string(), serde_json::json!("value1"));

        assert!(report.metadata.is_some());
        let metadata = report.metadata.as_ref().unwrap();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata.get("key1").unwrap(), &serde_json::json!("value1"));
    }

    #[test]
    fn test_add_metadata_to_existing_metadata() {
        let mut initial_metadata = HashMap::new();
        initial_metadata.insert(
            "existing_key".to_string(),
            serde_json::json!("existing_value"),
        );

        let mut report = GlobalErrorReport::with_metadata(
            "TestError".to_string(),
            "Test message".to_string(),
            ErrorSeverity::Error,
            initial_metadata,
        );

        report.add_metadata("new_key".to_string(), serde_json::json!(42));

        let metadata = report.metadata.as_ref().unwrap();
        assert_eq!(metadata.len(), 2);
        assert_eq!(
            metadata.get("existing_key").unwrap(),
            &serde_json::json!("existing_value")
        );
        assert_eq!(metadata.get("new_key").unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn test_add_metadata_overwrites_existing_key() {
        let mut initial_metadata = HashMap::new();
        initial_metadata.insert("key".to_string(), serde_json::json!("old_value"));

        let mut report = GlobalErrorReport::with_metadata(
            "TestError".to_string(),
            "Test message".to_string(),
            ErrorSeverity::Error,
            initial_metadata,
        );

        report.add_metadata("key".to_string(), serde_json::json!("new_value"));

        let metadata = report.metadata.as_ref().unwrap();
        assert_eq!(metadata.len(), 1);
        assert_eq!(
            metadata.get("key").unwrap(),
            &serde_json::json!("new_value")
        );
    }

    #[test]
    fn test_add_metadata_with_various_value_types() {
        let mut report = GlobalErrorReport::new(
            "TestError".to_string(),
            "Test message".to_string(),
            ErrorSeverity::Error,
        );

        report.add_metadata("string_val".to_string(), serde_json::json!("hello"));
        report.add_metadata("int_val".to_string(), serde_json::json!(123));
        report.add_metadata("float_val".to_string(), serde_json::json!(3.14));
        report.add_metadata("bool_val".to_string(), serde_json::json!(true));
        report.add_metadata("null_val".to_string(), serde_json::json!(null));
        report.add_metadata("array_val".to_string(), serde_json::json!([1, 2, 3]));
        report.add_metadata(
            "object_val".to_string(),
            serde_json::json!({"nested": "value"}),
        );

        let metadata = report.metadata.as_ref().unwrap();
        assert_eq!(metadata.len(), 7);
        assert_eq!(
            metadata.get("string_val").unwrap(),
            &serde_json::json!("hello")
        );
        assert_eq!(metadata.get("int_val").unwrap(), &serde_json::json!(123));
        assert_eq!(metadata.get("float_val").unwrap(), &serde_json::json!(3.14));
        assert_eq!(metadata.get("bool_val").unwrap(), &serde_json::json!(true));
        assert_eq!(metadata.get("null_val").unwrap(), &serde_json::json!(null));
        assert_eq!(
            metadata.get("array_val").unwrap(),
            &serde_json::json!([1, 2, 3])
        );
        assert_eq!(
            metadata.get("object_val").unwrap(),
            &serde_json::json!({"nested": "value"})
        );
    }

    // ==================== GlobalErrorReport Serialization Tests ====================

    #[test]
    fn test_global_error_report_serialization() {
        let report = GlobalErrorReport::critical(
            "DatabaseError".to_string(),
            "Database connection failed".to_string(),
        );

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"type\":\"DatabaseError\""));
        assert!(json.contains("\"message\":\"Database connection failed\""));
        assert!(json.contains("\"severity\":\"CRITICAL\""));
        // metadata should not appear when None
        assert!(!json.contains("\"metadata\""));
    }

    #[test]
    fn test_global_error_report_serialization_with_metadata() {
        let mut report = GlobalErrorReport::error(
            "ConnectionError".to_string(),
            "Failed to connect".to_string(),
        );
        report.add_metadata("retry_count".to_string(), serde_json::json!(3));

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"type\":\"ConnectionError\""));
        assert!(json.contains("\"severity\":\"ERROR\""));
        assert!(json.contains("\"metadata\""));
        assert!(json.contains("\"retry_count\":3"));
    }
}
