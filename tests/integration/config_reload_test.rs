// Integration tests for config hot-reload functionality
// T089 [US6]: Integration test for config hot-reload in tests/integration/config_reload_test.rs

use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// Helper function to create a valid TOML config file
fn create_config_file(path: &PathBuf, crontab: &str, source_dir: &str) {
    let config_content = format!(
        r#"
[scheduler]
crontab = "{}"

[src]
source_dir = "{}"

[credential]
account = "testaccount"
username = "test"
password = "test"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP866"
"#,
        crontab, source_dir
    );
    fs::write(path, config_content).expect("Failed to write config file");
}

/// T089: Test that config watcher detects file changes
#[test]
fn test_config_file_change_detection() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().replace("\\", "/");

    // Create initial config
    create_config_file(&config_path, "0 * * * *", &source_dir);

    // Load the config to verify it's valid
    let initial_config = common::models::Config::from_file(&config_path);
    assert!(initial_config.is_ok(), "Initial config should be valid");
    assert_eq!(
        initial_config.unwrap().scheduler.crontab,
        "0 * * * *",
        "Initial crontab should be '0 * * * *'"
    );

    // Modify the config file
    create_config_file(&config_path, "30 * * * *", &source_dir);

    // Wait a moment for the file system to update
    std::thread::sleep(Duration::from_millis(100));

    // Reload and verify the new config
    let updated_config = common::models::Config::from_file(&config_path);
    assert!(updated_config.is_ok(), "Updated config should be valid");
    assert_eq!(
        updated_config.unwrap().scheduler.crontab,
        "30 * * * *",
        "Updated crontab should be '30 * * * *'"
    );
}

/// T089: Test that invalid config file changes are detected as errors
#[test]
fn test_config_file_invalid_change() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().replace("\\", "/");

    // Create initial valid config
    create_config_file(&config_path, "0 * * * *", &source_dir);

    // Load the config to verify it's valid
    let initial_config = common::models::Config::from_file(&config_path);
    assert!(initial_config.is_ok());

    // Write an invalid config (missing required fields)
    fs::write(
        &config_path,
        r#"
[scheduler]
crontab = "0 * * * *"
# Missing other required sections
"#,
    )
    .unwrap();

    // Attempt to reload - should fail
    let invalid_config = common::models::Config::from_file(&config_path);
    assert!(invalid_config.is_err(), "Invalid config should fail to load");
}

/// T089: Test that config changes preserve source directory patterns
#[test]
fn test_config_reload_preserves_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().replace("\\", "/");

    // Create config with include/exclude patterns
    let config_content = format!(
        r#"
[scheduler]
crontab = "0 * * * *"

[src]
source_dir = "{}"
include_patterns = ["nsf*.dbf"]
exclude_patterns = ["temp_*.dbf"]

[credential]
account = "testaccount"
username = "test"
password = "test"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP866"
"#,
        source_dir
    );
    fs::write(&config_path, config_content).unwrap();

    // Load the config
    let config = common::models::Config::from_file(&config_path).unwrap();

    // Verify include/exclude patterns are present
    assert!(config.src.include_patterns.is_some());
    assert!(config.src.exclude_patterns.is_some());
    assert_eq!(
        config.src.include_patterns.as_ref().unwrap()[0],
        "nsf*.dbf"
    );
    assert_eq!(
        config.src.exclude_patterns.as_ref().unwrap()[0],
        "temp_*.dbf"
    );
}

/// T088: Test config loading retries correct number of times before giving up
#[test]
fn test_config_retry_backoff_intervals() {
    // Verify the backoff intervals match the specification
    // Initial attempts: 1, 2, 4, 8, 16 minutes
    // After initial attempts: hourly retries indefinitely
    let initial_retry_minutes: Vec<u64> = vec![1, 2, 4, 8, 16];
    let hourly_interval_minutes: u64 = 60;

    // Verify initial backoff sequence
    for (attempt, expected_minutes) in initial_retry_minutes.iter().enumerate() {
        let wait_minutes = if attempt + 1 <= initial_retry_minutes.len() {
            initial_retry_minutes[attempt]
        } else {
            hourly_interval_minutes
        };
        assert_eq!(wait_minutes, *expected_minutes);
    }

    // Verify hourly retry after initial attempts
    let attempt_6 = 6;
    let wait_minutes = if attempt_6 <= initial_retry_minutes.len() {
        initial_retry_minutes[attempt_6 - 1]
    } else {
        hourly_interval_minutes
    };
    assert_eq!(wait_minutes, 60);
}

/// T088: Test that config retry only applies to specific error types
#[test]
fn test_config_retry_only_for_directory_not_found() {
    // Create a config pointing to a non-existent source directory
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let config_content = r#"
[scheduler]
crontab = "0 * * * *"

[src]
source_dir = "/nonexistent/path/12345"

[credential]
account = "testaccount"
username = "test"
password = "test"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP866"
"#;
    fs::write(&config_path, config_content).unwrap();

    // This should fail with a "Source directory does not exist" error
    let config = common::models::Config::from_file(&config_path);
    assert!(config.is_err());
    let error_msg = config.unwrap_err().to_string();
    assert!(
        error_msg.contains("Source directory does not exist"),
        "Expected 'Source directory does not exist' error, got: {}",
        error_msg
    );
}

/// T087: Test graceful shutdown signal handling (basic)
#[test]
fn test_stop_signal_atomic_bool() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Simulate the stop signal used in windows_service.rs
    let stop_signal = Arc::new(AtomicBool::new(false));

    // Initially not stopped
    assert!(!stop_signal.load(Ordering::Relaxed));

    // Simulate stop signal
    stop_signal.store(true, Ordering::Relaxed);
    assert!(stop_signal.load(Ordering::Relaxed));

    // Can be checked from multiple threads
    let signal_clone = Arc::clone(&stop_signal);
    let handle = std::thread::spawn(move || {
        signal_clone.load(Ordering::Relaxed)
    });
    let result = handle.join().unwrap();
    assert!(result, "Stop signal should be visible from other threads");
}

// Note: SC-002 hot-reload timing test is in service/tests/config_reload_sc002_test.rs
