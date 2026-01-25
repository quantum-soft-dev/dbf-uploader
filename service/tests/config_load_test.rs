// Integration tests for config load performance (SC-001)
// T004 [US1]: Benchmark test verifying configuration loads and validates within 100ms

use common::models::Config;
use std::fs;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Helper function to create a complete TOML config file with all sections
fn create_complete_config_file(path: &std::path::Path, source_dir: &str) {
    let config_content = format!(
        r#"
[scheduler]
crontab = "0 8,12,16,18 * * *"

[src]
source_dir = "{}"
include_patterns = ["*.dbf", "nsf*.DBF"]
exclude_patterns = ["temp_*.dbf", "nsfcli.DBF"]

[credential]
account = "testaccount"
username = "testuser"
password = "testpassword"

[api]
base_url = "https://api.example.com"
https_only = true

[encoding]
dbf_encoding = "CP866"
"#,
        source_dir.replace('\\', "\\\\")
    );
    fs::write(path, config_content).expect("Failed to write config file");
}

/// SC-001: Test that config loads and validates within 100ms
#[test]
fn test_config_load_performance_under_100ms_sc001() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().to_string();

    // Create config file with all sections
    create_complete_config_file(&config_path, &source_dir);

    const ITERATIONS: u32 = 10;
    const MAX_LOAD_TIME_MS: u128 = 100;

    // Warmup iteration to prime filesystem cache (not counted in measurement)
    let _ = Config::from_file(&config_path);

    // Collect results without assertions inside the timing loop
    let mut results = Vec::with_capacity(ITERATIONS as usize);
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        results.push(Config::from_file(&config_path));
    }
    let total_duration = start.elapsed();

    // Calculate max duration by measuring individual loads (separate from main timing)
    let mut max_duration = Duration::ZERO;
    for _ in 0..ITERATIONS {
        let iter_start = Instant::now();
        let _ = Config::from_file(&config_path);
        let elapsed = iter_start.elapsed();
        if elapsed > max_duration {
            max_duration = elapsed;
        }
    }

    // Assertions after measurement
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.is_ok(),
            "Iteration {}: Config load failed: {:?}",
            i,
            result.as_ref().err()
        );
    }

    let average_ms = total_duration.as_millis() / ITERATIONS as u128;

    println!(
        "Config load performance: avg={}ms, max={}ms (over {} iterations)",
        average_ms,
        max_duration.as_millis(),
        ITERATIONS
    );

    assert!(
        average_ms < MAX_LOAD_TIME_MS,
        "Average config load time ({}ms) exceeds {}ms requirement (SC-001)",
        average_ms,
        MAX_LOAD_TIME_MS
    );
}

/// Test that config with all sections loads correctly
#[test]
fn test_config_load_with_all_sections() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().to_string();

    create_complete_config_file(&config_path, &source_dir);

    let config = Config::from_file(&config_path).expect("Config should load successfully");

    // Verify scheduler section
    assert_eq!(config.scheduler.crontab, "0 8,12,16,18 * * *");

    // Verify src section
    assert_eq!(config.src.source_dir.to_string_lossy(), source_dir);
    assert!(config.src.include_patterns.is_some());
    assert!(config.src.exclude_patterns.is_some());
    assert_eq!(
        config.src.include_patterns.as_ref().unwrap().len(),
        2,
        "Should have 2 include patterns"
    );
    assert_eq!(
        config.src.exclude_patterns.as_ref().unwrap().len(),
        2,
        "Should have 2 exclude patterns"
    );

    // Verify credential section (traditional auth)
    assert_eq!(config.credential.account, "testaccount");
    assert_eq!(config.credential.username, "testuser");
    assert_eq!(config.credential.password, "testpassword");
    assert!(config.credential.device.is_none());

    // Verify api section
    assert_eq!(config.api.base_url, "https://api.example.com");
    assert!(config.api.https_only);

    // Verify encoding section
    assert_eq!(config.encoding.dbf_encoding, "CP866");
}

/// Test config load with device flow credentials
#[test]
fn test_config_load_with_device_flow() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().to_string();

    let config_content = format!(
        r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "{}"

[credential]
account = ""
username = ""
password = ""

[credential.device]
site_id = "550e8400-e29b-41d4-a716-446655440000"
domain = "mysite.example.com"
client_secret = "device_secret_key"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP1251"
"#,
        source_dir.replace('\\', "\\\\")
    );
    fs::write(&config_path, config_content).unwrap();

    let config = Config::from_file(&config_path).expect("Config should load successfully");

    // Verify device flow is configured
    assert!(config.credential.device.is_some());
    assert!(config.credential.is_device_flow());

    let device = config.credential.device.as_ref().unwrap();
    assert_eq!(device.site_id, "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(device.domain, "mysite.example.com");
    assert_eq!(device.client_secret, "device_secret_key");

    // Verify full_username returns device domain
    assert_eq!(config.credential.full_username(), "mysite.example.com");

    // Verify password returns client_secret
    assert_eq!(config.credential.password(), "device_secret_key");
}

/// Test config with minimal required fields
#[test]
fn test_config_load_minimal() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_dir = temp_dir.path().to_string_lossy().to_string();

    let config_content = format!(
        r#"
[scheduler]
crontab = "0 * * * *"

[src]
source_dir = "{}"

[credential]
account = "acc"
username = "user"
password = "pass"

[api]
base_url = "https://api.example.com"

[encoding]
"#,
        source_dir.replace('\\', "\\\\")
    );
    fs::write(&config_path, config_content).unwrap();

    let config = Config::from_file(&config_path).expect("Minimal config should load");

    // Verify defaults are applied
    assert_eq!(config.encoding.dbf_encoding, "CP866"); // default value
    assert!(config.api.https_only); // default value
    assert!(config.src.include_patterns.is_none()); // not specified
    assert!(config.src.exclude_patterns.is_none()); // not specified
}
