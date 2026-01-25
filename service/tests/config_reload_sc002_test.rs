// Integration tests for config hot-reload timing (SC-002)
// T003 [US3]: Test that hot-reload detects changes within 5 seconds

use data_exporter_service::config::ConfigWatcher;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
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

/// T003/SC-002: Test that hot-reload detects and applies changes within 5 seconds
/// This tests the timing requirement from SC-002 success criterion
#[test]
fn test_hot_reload_detection_within_5_seconds_sc002() {
    const MAX_DETECTION_TIME: Duration = Duration::from_secs(5);

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().replace("\\", "/");

    // Create initial config
    create_config_file(&config_path, "0 * * * *", &source_dir);

    // Create the watcher
    let watcher = ConfigWatcher::new(&config_path).expect("Failed to create ConfigWatcher");

    // Verify no initial changes
    assert!(
        !watcher.has_changed(),
        "Watcher should not detect changes immediately after creation"
    );

    // Modify the config file
    let start_time = Instant::now();
    create_config_file(&config_path, "30 * * * *", &source_dir);

    // Poll for changes with timeout
    // The debounce period is 2 seconds, so we expect detection within 5 seconds total
    let change_detected = loop {
        if watcher.has_changed() {
            break true;
        }

        if start_time.elapsed() > MAX_DETECTION_TIME {
            break false;
        }

        // Check every 100ms
        std::thread::sleep(Duration::from_millis(100));
    };

    let detection_time = start_time.elapsed();

    println!(
        "Hot-reload detection time: {}ms (requirement: <{}ms)",
        detection_time.as_millis(),
        MAX_DETECTION_TIME.as_millis()
    );

    assert!(
        change_detected,
        "Config change should be detected within {} seconds (SC-002)",
        MAX_DETECTION_TIME.as_secs()
    );

    assert!(
        detection_time < MAX_DETECTION_TIME,
        "Detection time ({}ms) exceeds 5 second requirement (SC-002)",
        detection_time.as_millis()
    );

    // Verify the new config can be loaded
    let config =
        common::models::Config::from_file(&config_path).expect("Should load updated config");
    assert_eq!(
        config.scheduler.crontab, "30 * * * *",
        "Updated crontab value should be loaded"
    );
}

/// Test that ConfigWatcher properly monitors the config file
#[test]
fn test_config_watcher_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().replace("\\", "/");

    create_config_file(&config_path, "0 * * * *", &source_dir);

    // Create watcher
    let watcher = ConfigWatcher::new(&config_path);
    assert!(
        watcher.is_ok(),
        "ConfigWatcher should be created successfully"
    );

    let watcher = watcher.unwrap();

    // Verify the config path is correct
    assert_eq!(
        watcher.config_path(),
        config_path.as_path(),
        "Watcher should monitor the correct config path"
    );

    // No changes initially
    assert!(
        !watcher.has_changed(),
        "No changes should be detected initially"
    );
}

/// Test that multiple rapid changes are debounced into a single notification
#[test]
fn test_config_watcher_debounce_behavior() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().replace("\\", "/");

    create_config_file(&config_path, "0 * * * *", &source_dir);

    let watcher = ConfigWatcher::new(&config_path).expect("Failed to create ConfigWatcher");

    // Make multiple rapid changes within the debounce window
    for i in 0..5 {
        create_config_file(&config_path, &format!("{} * * * *", i), &source_dir);
        std::thread::sleep(Duration::from_millis(100));
    }

    // Wait for debounce period (2 seconds) plus buffer
    std::thread::sleep(Duration::from_secs(3));

    // Count how many changes we receive (should be debounced to fewer than 5)
    let mut change_count = 0;
    while watcher.has_changed() {
        change_count += 1;
        // Safety break to prevent infinite loop
        if change_count > 10 {
            break;
        }
    }

    println!(
        "Detected {} change notifications for 5 rapid file writes",
        change_count
    );

    // All 5 writes occur within 500ms (100ms intervals), well inside the 2-second debounce window.
    // We expect exactly 1 notification (or at most 2 with timing jitter).
    assert!(
        change_count >= 1 && change_count <= 2,
        "Expected 1-2 notifications for 5 rapid writes within debounce window, got {}",
        change_count
    );
}
