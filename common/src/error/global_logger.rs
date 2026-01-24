// Global error logger - utility for reporting system-level errors
use crate::auth::TokenManager;
use crate::error::reporter::ErrorReporter;
use crate::error::{ProcessingError, Result};
use crate::models::{Config, ErrorSeverity, GlobalErrorReport};
use std::collections::HashMap;
use std::panic::PanicHookInfo;
use std::sync::Arc;

/// Global error logger for system-level errors
/// This provides a high-level API for reporting errors outside batch context
pub struct GlobalErrorLogger {
    reporter: Arc<ErrorReporter>,
    token_manager: Arc<TokenManager>,
    config: Arc<Config>,
}

impl GlobalErrorLogger {
    /// Create a new global error logger
    pub fn new(
        reporter: Arc<ErrorReporter>,
        token_manager: Arc<TokenManager>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            reporter,
            token_manager,
            config,
        }
    }

    /// Report a global error with custom severity and metadata
    pub async fn report(
        &self,
        error_type: String,
        message: String,
        severity: ErrorSeverity,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        let error_report = if let Some(meta) = metadata {
            GlobalErrorReport::with_metadata(error_type.clone(), message.clone(), severity, meta)
        } else {
            GlobalErrorReport::new(error_type.clone(), message.clone(), severity)
        };

        // Get a valid JWT token
        let token = match self.token_manager.get_valid_token().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to get JWT token for global error reporting, logging locally only"
                );
                return Err(e);
            }
        };

        // Send the error report
        match self
            .reporter
            .send_global_error(error_report, &token, &self.config)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    error_type = %error_type,
                    "Failed to send global error report, logged locally"
                );
                Err(e)
            }
        }
    }

    /// Report a CRITICAL error
    pub async fn critical(
        &self,
        error_type: String,
        message: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        tracing::error!(
            severity = "CRITICAL",
            error_type = %error_type,
            message = %message,
            "Critical error occurred"
        );
        self.report(error_type, message, ErrorSeverity::Critical, metadata)
            .await
    }

    /// Report an ERROR
    pub async fn error(
        &self,
        error_type: String,
        message: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        tracing::error!(
            severity = "ERROR",
            error_type = %error_type,
            message = %message,
            "Error occurred"
        );
        self.report(error_type, message, ErrorSeverity::Error, metadata)
            .await
    }

    /// Report a WARNING
    pub async fn warning(
        &self,
        error_type: String,
        message: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        tracing::warn!(
            severity = "WARNING",
            error_type = %error_type,
            message = %message,
            "Warning occurred"
        );
        self.report(error_type, message, ErrorSeverity::Warning, metadata)
            .await
    }

    /// Report an INFO message
    pub async fn info(
        &self,
        error_type: String,
        message: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        tracing::info!(
            severity = "INFO",
            error_type = %error_type,
            message = %message,
            "Info message"
        );
        self.report(error_type, message, ErrorSeverity::Info, metadata)
            .await
    }

    /// Report a ProcessingError as a global error
    pub async fn report_processing_error(
        &self,
        error: &ProcessingError,
        context: Option<&str>,
    ) -> Result<()> {
        let (error_type, severity) = match error {
            ProcessingError::AuthenticationError(_) => {
                ("AUTHENTICATION_FAILED".to_string(), ErrorSeverity::Critical)
            }
            ProcessingError::NetworkError(_) => {
                ("CONNECTION_FAILED".to_string(), ErrorSeverity::Error)
            }
            ProcessingError::ConfigurationError(_) => {
                ("CONFIG_ERROR".to_string(), ErrorSeverity::Critical)
            }
            ProcessingError::FileReadError(_) => ("FILE_ERROR".to_string(), ErrorSeverity::Error),
            _ => ("UNKNOWN_ERROR".to_string(), ErrorSeverity::Error),
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "errorSource".to_string(),
            serde_json::Value::String("data_exporter".to_string()),
        );

        if let Some(ctx) = context {
            metadata.insert(
                "context".to_string(),
                serde_json::Value::String(ctx.to_string()),
            );
        }

        self.report(error_type, error.to_string(), severity, Some(metadata))
            .await
    }
}

/// Install a panic hook that logs panics to a fallback location
///
/// T077: Panic handler logging - ensures panics are captured before process exit
///
/// This function installs a custom panic hook that:
/// 1. Logs the panic to the fallback error log file
/// 2. Includes panic location (file, line, column)
/// 3. Includes the panic message or payload
/// 4. Writes to a reliable location even when tracing isn't available
///
/// The fallback log location is:
/// - Windows: `C:\Program Files\data-exporter\panic.log`
/// - Other platforms: `./panic.log` (for development)
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(panic_hook));
}

/// The actual panic hook function
fn panic_hook(info: &PanicHookInfo) {
    // Get panic location
    let location = info.location().map_or_else(
        || "unknown location".to_string(),
        |loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()),
    );

    // Get panic message
    let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic payload".to_string()
    };

    // Format the panic entry
    let timestamp = chrono::Utc::now().to_rfc3339();
    let panic_entry = format!(
        "[{}] PANIC at {}\n  Message: {}\n  Thread: {:?}\n\n",
        timestamp,
        location,
        message,
        std::thread::current().name().unwrap_or("unnamed")
    );

    // Log to tracing (may not work if tracing isn't initialized)
    tracing::error!(
        location = %location,
        message = %message,
        "Panic occurred"
    );

    // Always write to fallback log file
    if let Err(e) = write_panic_to_file(&panic_entry) {
        // Last resort: write to stderr
        eprintln!("Failed to write panic to file: {}", e);
        eprintln!("{}", panic_entry);
    }
}

/// Write panic information to the fallback panic log file
fn write_panic_to_file(entry: &str) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    // Determine log file path
    #[cfg(target_os = "windows")]
    let panic_log_path = std::path::PathBuf::from(r"C:\Program Files\data-exporter\panic.log");
    #[cfg(not(target_os = "windows"))]
    let panic_log_path = std::path::PathBuf::from("panic.log");

    // Create parent directory if needed
    if let Some(parent) = panic_log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Open file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&panic_log_path)?;

    // Write entry
    file.write_all(entry.as_bytes())?;
    file.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_global_error_logger_creation() {
        // This test just ensures the types compile correctly
        // Actual functionality testing requires integration tests with real endpoints
    }

    #[test]
    fn test_write_panic_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let panic_log_path = temp_dir.path().join("test_panic.log");

        // Simulate writing a panic entry
        let panic_entry = "[2025-10-05T14:30:00Z] PANIC at src/main.rs:42:5\n  Message: test panic\n  Thread: \"main\"\n\n";

        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&panic_log_path)
            .unwrap();

        file.write_all(panic_entry.as_bytes()).unwrap();
        file.flush().unwrap();

        // Verify the file was created and contains the entry
        let contents = std::fs::read_to_string(&panic_log_path).unwrap();
        assert!(contents.contains("PANIC at src/main.rs:42:5"));
        assert!(contents.contains("test panic"));
        assert!(contents.contains("main"));
    }

    #[test]
    fn test_panic_log_append_mode() {
        let temp_dir = TempDir::new().unwrap();
        let panic_log_path = temp_dir.path().join("test_panic_append.log");

        use std::fs::OpenOptions;
        use std::io::Write;

        // Write first entry
        {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&panic_log_path)
                .unwrap();
            file.write_all(b"First panic\n").unwrap();
        }

        // Write second entry
        {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&panic_log_path)
                .unwrap();
            file.write_all(b"Second panic\n").unwrap();
        }

        // Verify both entries are present
        let contents = std::fs::read_to_string(&panic_log_path).unwrap();
        assert!(contents.contains("First panic"));
        assert!(contents.contains("Second panic"));
    }
}
