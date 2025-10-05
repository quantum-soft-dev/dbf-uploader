// Local error logging module - fallback when remote error reporting fails
use crate::error::{ProcessingError, Result};
use crate::models::ErrorReport;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Log error report to local file when remote reporting fails
/// This is a fallback mechanism to ensure errors are never lost
pub fn log_error_locally(
    error_report: &ErrorReport,
    fallback_reason: &str,
    log_path: Option<PathBuf>,
) -> Result<()> {
    // Determine log file path
    // On macOS (development), use current directory
    // On Windows (production), use C:\Program Files\data-exporter\error.log
    let log_file = log_path.unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        {
            PathBuf::from(r"C:\Program Files\data-exporter\error.log")
        }
        #[cfg(not(target_os = "windows"))]
        {
            PathBuf::from("error.log")
        }
    });

    // Create parent directory if it doesn't exist
    if let Some(parent) = log_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ProcessingError::FileReadError(std::io::Error::other(
                format!("Failed to create error log directory: {}", e),
            ))
        })?;
    }

    // Open file in append mode, create if doesn't exist
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| {
            ProcessingError::FileReadError(std::io::Error::other(
                format!("Failed to open error log file: {}", e),
            ))
        })?;

    // Format error report entry
    let log_entry = format!(
        "[{}] ERROR: {}\n  Filename: {}\n  Error Type: {}\n  Message: {}\n  Client Version: {}\n\n",
        error_report.timestamp,
        fallback_reason,
        error_report.filename,
        error_report.error_type,
        error_report.message,
        error_report.client_version
    );

    // Write to file
    file.write_all(log_entry.as_bytes()).map_err(|e| {
        ProcessingError::FileReadError(std::io::Error::other(
            format!("Failed to write to error log: {}", e),
        ))
    })?;

    // Flush to ensure data is written
    file.flush().map_err(|e| {
        ProcessingError::FileReadError(std::io::Error::other(
            format!("Failed to flush error log: {}", e),
        ))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_log_error_locally() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test_error.log");

        let error_report = ErrorReport::new(
            "test.dbf".to_string(),
            "FileReadError".to_string(),
            "Test error message".to_string(),
            "1.0.0".to_string(),
        );

        let result = log_error_locally(
            &error_report,
            "Network error during remote reporting",
            Some(log_path.clone()),
        );

        assert!(result.is_ok());
        assert!(log_path.exists());

        // Read the log file and verify contents
        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("Filename: test.dbf"));
        assert!(contents.contains("Error Type: FileReadError"));
        assert!(contents.contains("Test error message"));
        assert!(contents.contains("Network error during remote reporting"));
    }

    #[test]
    fn test_log_error_locally_append() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test_error.log");

        let error_report1 = ErrorReport::new(
            "test1.dbf".to_string(),
            "FileReadError".to_string(),
            "First error".to_string(),
            "1.0.0".to_string(),
        );

        let error_report2 = ErrorReport::new(
            "test2.dbf".to_string(),
            "ConversionError".to_string(),
            "Second error".to_string(),
            "1.0.0".to_string(),
        );

        // Log first error
        log_error_locally(&error_report1, "Test reason 1", Some(log_path.clone())).unwrap();

        // Log second error (should append)
        log_error_locally(&error_report2, "Test reason 2", Some(log_path.clone())).unwrap();

        // Verify both errors are in the file
        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("test1.dbf"));
        assert!(contents.contains("test2.dbf"));
        assert!(contents.contains("First error"));
        assert!(contents.contains("Second error"));
    }

    #[test]
    fn test_log_error_locally_creates_directory() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("subdir").join("test_error.log");

        let error_report = ErrorReport::new(
            "test.dbf".to_string(),
            "FileReadError".to_string(),
            "Test error".to_string(),
            "1.0.0".to_string(),
        );

        let result = log_error_locally(&error_report, "Test", Some(log_path.clone()));

        assert!(result.is_ok());
        assert!(log_path.parent().unwrap().exists());
        assert!(log_path.exists());
    }
}
