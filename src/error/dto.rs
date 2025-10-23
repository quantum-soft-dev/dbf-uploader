use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error log request matching middleware schema
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorLogRequest {
    /// Error type/category
    #[serde(rename = "type")]
    pub error_type: String,

    /// Error message
    pub message: String,

    /// Optional metadata as key-value pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Client version for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_version: Option<String>,
}

impl ErrorLogRequest {
    /// Create a new error log request
    pub fn new(error_type: String, message: String) -> Self {
        Self {
            error_type,
            message,
            metadata: None,
            client_version: None,
        }
    }

    /// Add metadata to the request
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add client version
    pub fn with_client_version(mut self, version: String) -> Self {
        self.client_version = Some(version);
        self
    }

    /// Validate the request
    pub fn validate(&self) -> Result<(), String> {
        if self.error_type.is_empty() {
            return Err("Error type cannot be empty".to_string());
        }
        if self.error_type.len() > 100 {
            return Err("Error type too long (max 100 characters)".to_string());
        }
        if self.message.is_empty() {
            return Err("Message cannot be empty".to_string());
        }
        if self.message.len() > 1000 {
            return Err("Message too long (max 1000 characters)".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_log_request_new() {
        let request = ErrorLogRequest::new(
            "FileReadError".to_string(),
            "Failed to read file".to_string(),
        );

        assert_eq!(request.error_type, "FileReadError");
        assert_eq!(request.message, "Failed to read file");
        assert!(request.metadata.is_none());
        assert!(request.client_version.is_none());
    }

    #[test]
    fn test_error_log_request_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("file".to_string(), "test.dbf".to_string());

        let request = ErrorLogRequest::new(
            "FileReadError".to_string(),
            "Failed to read file".to_string(),
        )
        .with_metadata(metadata.clone());

        assert!(request.metadata.is_some());
        assert_eq!(request.metadata.unwrap().get("file").unwrap(), "test.dbf");
    }

    #[test]
    fn test_error_log_request_with_version() {
        let request = ErrorLogRequest::new(
            "FileReadError".to_string(),
            "Failed to read file".to_string(),
        )
        .with_client_version("2.0.0".to_string());

        assert_eq!(request.client_version.unwrap(), "2.0.0");
    }

    #[test]
    fn test_validate_success() {
        let request = ErrorLogRequest::new(
            "FileReadError".to_string(),
            "Failed to read file".to_string(),
        );

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_type() {
        let request = ErrorLogRequest::new("".to_string(), "Failed to read file".to_string());

        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_type_too_long() {
        let request = ErrorLogRequest::new("x".repeat(101), "Failed to read file".to_string());

        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too long"));
    }

    #[test]
    fn test_validate_empty_message() {
        let request = ErrorLogRequest::new("FileReadError".to_string(), "".to_string());

        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_message_too_long() {
        let request = ErrorLogRequest::new("FileReadError".to_string(), "x".repeat(1001));

        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too long"));
    }

    #[test]
    fn test_serialization() {
        let request = ErrorLogRequest::new(
            "FileReadError".to_string(),
            "Failed to read file".to_string(),
        );

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""type":"FileReadError""#));
        assert!(json.contains(r#""message":"Failed to read file""#));
    }

    #[test]
    fn test_serialization_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("file".to_string(), "test.dbf".to_string());

        let request = ErrorLogRequest::new(
            "FileReadError".to_string(),
            "Failed to read file".to_string(),
        )
        .with_metadata(metadata)
        .with_client_version("2.0.0".to_string());

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""metadata""#));
        assert!(json.contains(r#""clientVersion":"2.0.0""#));
    }
}
