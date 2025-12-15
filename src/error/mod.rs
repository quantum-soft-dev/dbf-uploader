// Error types for data_exporter
use thiserror::Error;

pub mod logger;
pub mod reporter;

pub use logger::log_error_locally;
pub use reporter::ErrorReporter;

/// Domain-specific errors for file processing operations
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("Failed to read DBF file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to detect or convert encoding: {0}")]
    EncodingError(String),

    #[error("Failed to convert DBF to CSV: {0}")]
    ConversionError(String),

    #[error("Failed to compress file: {0}")]
    CompressionError(String),

    #[error("Failed to upload file: {0}")]
    UploadError(String),

    #[error("Disk full: {0}")]
    DiskFullError(String),

    #[error("Directory inaccessible: {0}")]
    DirectoryInaccessible(String),

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("VSS error: {0}")]
    VssError(String),
}

impl ProcessingError {
    /// Get the error type as a string for error reporting API
    pub fn error_type(&self) -> &'static str {
        match self {
            ProcessingError::FileReadError(_) => "FileReadError",
            ProcessingError::EncodingError(_) => "EncodingError",
            ProcessingError::ConversionError(_) => "ConversionError",
            ProcessingError::CompressionError(_) => "CompressionError",
            ProcessingError::UploadError(_) => "UploadError",
            ProcessingError::DiskFullError(_) => "DiskFullError",
            ProcessingError::DirectoryInaccessible(_) => "DirectoryInaccessible",
            ProcessingError::AuthenticationError(_) => "AuthenticationError",
            ProcessingError::ConfigurationError(_) => "ConfigurationError",
            ProcessingError::NetworkError(_) => "NetworkError",
            ProcessingError::VssError(_) => "VssError",
        }
    }

    /// Get detailed error information including source chain
    ///
    /// This method walks the error source chain and formats it as a multi-line string
    /// with each level of the error chain indented appropriately.
    ///
    /// # Returns
    /// A formatted string containing the error message and all its sources
    pub fn detailed_message(&self) -> String {
        use std::error::Error;

        let mut details = vec![self.to_string()];

        // Walk the error source chain
        if let Some(source) = (self as &dyn Error).source() {
            details.push(format!("Caused by: {}", source));

            let mut current = source.source();
            while let Some(src) = current {
                details.push(format!("  Caused by: {}", src));
                current = src.source();
            }
        }

        details.join("\n")
    }

    /// Check if this error is critical (should fail the entire batch)
    ///
    /// Critical errors are those that prevent the entire batch from being processed,
    /// such as authentication failures or inaccessible directories.
    ///
    /// Non-critical errors (warnings) are per-file issues like corrupted DBF files
    /// or encoding problems that don't affect other files.
    ///
    /// # Returns
    /// `true` if the error is critical, `false` if it's a warning
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            ProcessingError::DirectoryInaccessible(_)
                | ProcessingError::AuthenticationError(_)
                | ProcessingError::ConfigurationError(_)
        )
    }
}

/// Result type alias for operations that can fail with ProcessingError
pub type Result<T> = std::result::Result<T, ProcessingError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_detailed_message_with_source() {
        // FileReadError has a source (std::io::Error)
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error = ProcessingError::FileReadError(io_error);

        let detailed = error.detailed_message();

        // Should include both the main error and the source
        assert!(detailed.contains("Failed to read DBF file"));
        assert!(detailed.contains("Caused by:"));
        assert!(detailed.contains("File not found"));
    }

    #[test]
    fn test_detailed_message_without_source() {
        // ConversionError has no source (just a String)
        let error = ProcessingError::ConversionError("Invalid DBF format".to_string());

        let detailed = error.detailed_message();

        // Should only include the main error
        assert!(detailed.contains("Failed to convert DBF to CSV"));
        assert!(detailed.contains("Invalid DBF format"));
        // Should not have "Caused by" since there's no source
        assert_eq!(detailed.matches("Caused by:").count(), 0);
    }

    #[test]
    fn test_is_critical_for_critical_errors() {
        assert!(ProcessingError::DirectoryInaccessible("test".to_string()).is_critical());
        assert!(ProcessingError::AuthenticationError("test".to_string()).is_critical());
        assert!(ProcessingError::ConfigurationError("test".to_string()).is_critical());
    }

    #[test]
    fn test_is_critical_for_warnings() {
        // These are non-critical (warnings)
        let io_error = io::Error::new(io::ErrorKind::NotFound, "test");
        assert!(!ProcessingError::FileReadError(io_error).is_critical());
        assert!(!ProcessingError::ConversionError("test".to_string()).is_critical());
        assert!(!ProcessingError::EncodingError("test".to_string()).is_critical());
        assert!(!ProcessingError::CompressionError("test".to_string()).is_critical());
        assert!(!ProcessingError::UploadError("test".to_string()).is_critical());
        assert!(!ProcessingError::NetworkError("test".to_string()).is_critical());
        assert!(!ProcessingError::VssError("test".to_string()).is_critical());
        assert!(!ProcessingError::DiskFullError("test".to_string()).is_critical());
    }

    #[test]
    fn test_error_type_returns_correct_strings() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "test");
        assert_eq!(
            ProcessingError::FileReadError(io_error).error_type(),
            "FileReadError"
        );
        assert_eq!(
            ProcessingError::ConversionError("test".to_string()).error_type(),
            "ConversionError"
        );
        assert_eq!(
            ProcessingError::DirectoryInaccessible("test".to_string()).error_type(),
            "DirectoryInaccessible"
        );
    }
}
