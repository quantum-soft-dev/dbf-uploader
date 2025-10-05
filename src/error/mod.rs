// Error types for data_exporter
use thiserror::Error;

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
        }
    }
}

/// Result type alias for operations that can fail with ProcessingError
pub type Result<T> = std::result::Result<T, ProcessingError>;
