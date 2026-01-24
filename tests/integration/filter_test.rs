// Integration tests for file filtering with pattern-based directory scanning
// T102 [US7]: Write integration test for pattern-based directory scanning

use common::models::config::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test config
fn create_test_config(
    source_dir: PathBuf,
    include_patterns: Option<Vec<String>>,
    exclude_patterns: Option<Vec<String>>,
) -> Config {
    Config {
        scheduler: SchedulerConfig {
            crontab: "0 * * * *".to_string(),
        },
        src: SourceConfig {
            source_dir,
            include_patterns,
            exclude_patterns,
        },
        credential: CredentialConfig {
            account: "test".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            device: None,
        },
        api: ApiConfig {
            base_url: "https://test.com".to_string(),
            https_only: true,
        },
        encoding: EncodingConfig {
            dbf_encoding: "CP866".to_string(),
        },
    }
}

/// Helper to create dummy DBF files for testing
fn create_dummy_dbf_files(dir: &std::path::Path, filenames: &[&str]) {
    for filename in filenames {
        let file_path = dir.join(filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file_path, b"test dbf content").unwrap();
    }
}

/// T102: Integration test for scanning with no filters (all DBF files)
#[test]
fn test_scan_no_filters_includes_all_dbf() {
    let temp_dir = TempDir::new().unwrap();

    // Create various DBF files
    create_dummy_dbf_files(
        temp_dir.path(),
        &["data1.dbf", "data2.DBF", "sales.dbf", "inventory.dbf"],
    );

    let config = create_test_config(temp_dir.path().to_path_buf(), None, None);

    // Use the scanner module
    let result = data_exporter_service::processor::scan_directory(&config);
    assert!(result.is_ok());

    let files = result.unwrap();
    assert_eq!(files.len(), 4, "Should find all 4 DBF files");
}

/// T102: Integration test for scanning with include patterns (whitelist)
#[test]
fn test_scan_include_patterns_whitelist() {
    let temp_dir = TempDir::new().unwrap();

    // Create various DBF files
    create_dummy_dbf_files(
        temp_dir.path(),
        &[
            "nsf_data.dbf",
            "nsf_sales.dbf",
            "other_data.dbf",
            "sales.dbf",
        ],
    );

    let config = create_test_config(
        temp_dir.path().to_path_buf(),
        Some(vec!["nsf_*.dbf".to_string()]),
        None,
    );

    let result = data_exporter_service::processor::scan_directory(&config);
    assert!(result.is_ok());

    let files = result.unwrap();
    assert_eq!(files.len(), 2, "Should only find 2 nsf_ files");

    // Verify the correct files were found
    let filenames: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(filenames.iter().any(|n| n.eq_ignore_ascii_case("nsf_data.dbf")));
    assert!(filenames.iter().any(|n| n.eq_ignore_ascii_case("nsf_sales.dbf")));
}

/// T102: Integration test for scanning with exclude patterns (blacklist)
#[test]
fn test_scan_exclude_patterns_blacklist() {
    let temp_dir = TempDir::new().unwrap();

    // Create various DBF files
    create_dummy_dbf_files(
        temp_dir.path(),
        &[
            "data1.dbf",
            "data2.dbf",
            "temp_backup.dbf",
            "temp_cache.dbf",
        ],
    );

    let config = create_test_config(
        temp_dir.path().to_path_buf(),
        None,
        Some(vec!["temp_*.dbf".to_string()]),
    );

    let result = data_exporter_service::processor::scan_directory(&config);
    assert!(result.is_ok());

    let files = result.unwrap();
    assert_eq!(files.len(), 2, "Should find 2 non-temp files");

    // Verify temp files were excluded
    let filenames: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(!filenames.iter().any(|n| n.starts_with("temp_")));
}

/// T102: Integration test for combined include and exclude patterns
#[test]
fn test_scan_combined_include_exclude() {
    let temp_dir = TempDir::new().unwrap();

    // Create various DBF files
    create_dummy_dbf_files(
        temp_dir.path(),
        &[
            "nsf_data.dbf",
            "nsf_sales.dbf",
            "nsf_temp.dbf",   // Should be excluded even though it matches include
            "other_data.dbf", // Doesn't match include, should be excluded
        ],
    );

    let config = create_test_config(
        temp_dir.path().to_path_buf(),
        Some(vec!["nsf_*.dbf".to_string()]),
        Some(vec!["*_temp.dbf".to_string()]),
    );

    let result = data_exporter_service::processor::scan_directory(&config);
    assert!(result.is_ok());

    let files = result.unwrap();
    assert_eq!(files.len(), 2, "Should find 2 files (nsf_* minus *_temp)");

    // Verify the correct files were found
    let filenames: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(filenames.iter().any(|n| n.eq_ignore_ascii_case("nsf_data.dbf")));
    assert!(filenames.iter().any(|n| n.eq_ignore_ascii_case("nsf_sales.dbf")));
    assert!(!filenames.iter().any(|n| n.eq_ignore_ascii_case("nsf_temp.dbf")));
}

/// T102: Integration test for case-insensitive pattern matching
#[test]
fn test_scan_case_insensitive_patterns() {
    let temp_dir = TempDir::new().unwrap();

    // Create DBF files with mixed case
    create_dummy_dbf_files(
        temp_dir.path(),
        &["DATA.DBF", "data.dbf", "Data.Dbf", "OTHER.txt"],
    );

    let config = create_test_config(
        temp_dir.path().to_path_buf(),
        Some(vec!["data.dbf".to_string()]), // lowercase pattern
        None,
    );

    let result = data_exporter_service::processor::scan_directory(&config);
    assert!(result.is_ok());

    let files = result.unwrap();
    assert_eq!(files.len(), 3, "Should find all 3 data.dbf files (case-insensitive)");
}

/// T102: Integration test for recursive directory scanning with patterns
#[test]
fn test_scan_recursive_with_patterns() {
    let temp_dir = TempDir::new().unwrap();

    // Create DBF files in nested directories
    create_dummy_dbf_files(
        temp_dir.path(),
        &[
            "nsf_root.dbf",
            "subdir1/nsf_sub1.dbf",
            "subdir1/subdir2/nsf_sub2.dbf",
            "subdir1/other.dbf",
        ],
    );

    let config = create_test_config(
        temp_dir.path().to_path_buf(),
        Some(vec!["nsf_*.dbf".to_string()]),
        None,
    );

    let result = data_exporter_service::processor::scan_directory(&config);
    assert!(result.is_ok());

    let files = result.unwrap();
    assert_eq!(files.len(), 3, "Should find 3 nsf_* files across directories");
}

/// T102: Integration test for empty include pattern list (should process all)
#[test]
fn test_scan_empty_include_patterns() {
    let temp_dir = TempDir::new().unwrap();

    create_dummy_dbf_files(temp_dir.path(), &["data1.dbf", "data2.dbf"]);

    let config = create_test_config(
        temp_dir.path().to_path_buf(),
        Some(vec![]), // Empty include patterns
        None,
    );

    let result = data_exporter_service::processor::scan_directory(&config);
    assert!(result.is_ok());

    let files = result.unwrap();
    // Empty include patterns should process all files (no whitelist = all allowed)
    assert_eq!(files.len(), 2);
}

/// T102: Integration test for multiple include patterns (OR logic)
#[test]
fn test_scan_multiple_include_patterns() {
    let temp_dir = TempDir::new().unwrap();

    create_dummy_dbf_files(
        temp_dir.path(),
        &["sales.dbf", "orders.dbf", "inventory.dbf", "temp.dbf"],
    );

    let config = create_test_config(
        temp_dir.path().to_path_buf(),
        Some(vec!["sales.dbf".to_string(), "orders.dbf".to_string()]),
        None,
    );

    let result = data_exporter_service::processor::scan_directory(&config);
    assert!(result.is_ok());

    let files = result.unwrap();
    assert_eq!(files.len(), 2, "Should find files matching any include pattern");
}

/// T102: Integration test for multiple exclude patterns (OR logic)
#[test]
fn test_scan_multiple_exclude_patterns() {
    let temp_dir = TempDir::new().unwrap();

    create_dummy_dbf_files(
        temp_dir.path(),
        &["data.dbf", "temp.dbf", "backup.dbf", "cache.dbf"],
    );

    let config = create_test_config(
        temp_dir.path().to_path_buf(),
        None,
        Some(vec!["temp.dbf".to_string(), "backup.dbf".to_string()]),
    );

    let result = data_exporter_service::processor::scan_directory(&config);
    assert!(result.is_ok());

    let files = result.unwrap();
    assert_eq!(files.len(), 2, "Should exclude files matching any exclude pattern");
}
