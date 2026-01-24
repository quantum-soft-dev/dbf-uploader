// Directory scanner for DBF files
use crate::processor::filter;
use common::error::{ProcessingError, Result};
use common::models::{config::Config, DbfFile};
use std::path::Path;
use tracing::{debug, info, warn};

/// Recursively scan a directory for DBF files with filtering
/// Returns a list of DBF files found, with relative paths calculated from source_dir
/// Files are filtered based on include/exclude patterns in config
pub fn scan_directory(config: &Config) -> Result<Vec<DbfFile>> {
    let source_dir = &config.src.source_dir;

    // Verify source directory exists and is accessible
    if !source_dir.exists() {
        return Err(ProcessingError::DirectoryInaccessible(format!(
            "Source directory does not exist: {}",
            source_dir.display()
        )));
    }

    if !source_dir.is_dir() {
        return Err(ProcessingError::DirectoryInaccessible(format!(
            "Path is not a directory: {}",
            source_dir.display()
        )));
    }

    // Initialize filter from config
    filter::set_global_filter(&config.src)
        .map_err(|e| ProcessingError::ConfigurationError(format!("Filter error: {}", e)))?;

    let mut dbf_files = Vec::new();
    let mut filtered_count = 0;

    // Recursively walk the directory tree
    match walk_directory(source_dir, source_dir, &mut dbf_files, &mut filtered_count) {
        Ok(_) => {
            if filtered_count > 0 {
                info!(
                    found = dbf_files.len(),
                    filtered = filtered_count,
                    "Scan complete: {} files found, {} filtered out",
                    dbf_files.len(),
                    filtered_count
                );
            } else {
                debug!("Scan complete: found {} DBF files", dbf_files.len());
            }
            Ok(dbf_files)
        }
        Err(e) => {
            warn!("Error during directory scan: {}", e);
            // Return empty vec if directory is inaccessible, but don't fail
            // This allows batch processing to continue with other operations
            Ok(Vec::new())
        }
    }
}

/// Internal recursive function to walk directory tree
fn walk_directory(
    current_dir: &Path,
    source_dir: &Path,
    dbf_files: &mut Vec<DbfFile>,
    filtered_count: &mut usize,
) -> Result<()> {
    let entries = std::fs::read_dir(current_dir).map_err(|e| {
        ProcessingError::DirectoryInaccessible(format!(
            "Cannot read directory {}: {}",
            current_dir.display(),
            e
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(ProcessingError::FileReadError)?;
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories
            walk_directory(&path, source_dir, dbf_files, filtered_count)?;
        } else if path.is_file() {
            // Check if file has .dbf extension (case-insensitive)
            if let Some(extension) = path.extension() {
                if extension.eq_ignore_ascii_case("dbf") {
                    let mut dbf_file = DbfFile::new(path.clone(), source_dir);

                    // Get filename for filtering (just the filename, not full path)
                    let filename = dbf_file
                        .relative_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    // Apply filtering
                    if !filter::should_process_file(filename) {
                        debug!(
                            file = %filename,
                            "File filtered out by include/exclude patterns"
                        );
                        *filtered_count += 1;
                        continue; // Skip this file
                    }

                    // Try to detect encoding from DBF header
                    if let Some(detected_encoding) = dbf_file.detect_encoding_from_header() {
                        debug!(
                            "Found DBF file: {} (encoding: {})",
                            dbf_file.relative_path.display(),
                            detected_encoding.as_str()
                        );
                    } else {
                        debug!(
                            "Found DBF file: {} (encoding not detected, will use config fallback)",
                            dbf_file.relative_path.display()
                        );
                    }

                    dbf_files.push(dbf_file);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::models::config::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config(source_dir: &Path) -> Config {
        Config {
            scheduler: SchedulerConfig {
                crontab: "0 * * * *".to_string(),
            },
            src: SourceConfig {
                source_dir: source_dir.to_path_buf(),
                include_patterns: None,
                exclude_patterns: None,
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

    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let result = scan_directory(&config).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_scan_with_dbf_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test DBF files
        fs::write(temp_dir.path().join("file1.dbf"), b"test").unwrap();
        fs::write(temp_dir.path().join("file2.DBF"), b"test").unwrap(); // Test case-insensitivity
        fs::write(temp_dir.path().join("not_dbf.txt"), b"test").unwrap();

        let config = create_test_config(temp_dir.path());
        let result = scan_directory(&config).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_scan_nested_directories() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested directory structure
        let subdir1 = temp_dir.path().join("subdir1");
        let subdir2 = temp_dir.path().join("subdir1/subdir2");
        fs::create_dir(&subdir1).unwrap();
        fs::create_dir(&subdir2).unwrap();

        // Create DBF files at different levels
        fs::write(temp_dir.path().join("root.dbf"), b"test").unwrap();
        fs::write(subdir1.join("level1.dbf"), b"test").unwrap();
        fs::write(subdir2.join("level2.dbf"), b"test").unwrap();

        let config = create_test_config(temp_dir.path());
        let result = scan_directory(&config).unwrap();
        assert_eq!(result.len(), 3);

        // Verify relative paths are calculated correctly
        let paths: Vec<_> = result
            .iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();
        assert!(paths.iter().any(|p| p == "root.dbf"));
        assert!(paths
            .iter()
            .any(|p| p.contains("subdir1") && p.contains("level1.dbf")));
        assert!(paths
            .iter()
            .any(|p| p.contains("subdir2") && p.contains("level2.dbf")));
    }

    #[test]
    fn test_scan_nonexistent_directory() {
        let config = create_test_config(&std::path::PathBuf::from("/nonexistent/path/12345"));
        let result = scan_directory(&config);
        assert!(result.is_err());
        match result {
            Err(ProcessingError::DirectoryInaccessible(_)) => (),
            _ => panic!("Expected DirectoryInaccessible error"),
        }
    }

    #[test]
    fn test_scan_file_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, b"test").unwrap();

        let config = create_test_config(&file_path);
        let result = scan_directory(&config);
        assert!(result.is_err());
        match result {
            Err(ProcessingError::DirectoryInaccessible(_)) => (),
            _ => panic!("Expected DirectoryInaccessible error"),
        }
    }
}
