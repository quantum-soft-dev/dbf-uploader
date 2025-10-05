// Directory scanner for DBF files
use crate::error::{ProcessingError, Result};
use crate::models::DbfFile;
use std::path::Path;
use tracing::{debug, warn};

/// Recursively scan a directory for DBF files
/// Returns a list of DBF files found, with relative paths calculated from source_dir
pub fn scan_directory<P: AsRef<Path>>(source_dir: P) -> Result<Vec<DbfFile>> {
    let source_dir = source_dir.as_ref();

    // Verify source directory exists and is accessible
    if !source_dir.exists() {
        return Err(ProcessingError::DirectoryInaccessible(
            format!("Source directory does not exist: {}", source_dir.display())
        ));
    }

    if !source_dir.is_dir() {
        return Err(ProcessingError::DirectoryInaccessible(
            format!("Path is not a directory: {}", source_dir.display())
        ));
    }

    let mut dbf_files = Vec::new();

    // Recursively walk the directory tree
    match walk_directory(source_dir, source_dir, &mut dbf_files) {
        Ok(_) => {
            debug!("Scan complete: found {} DBF files", dbf_files.len());
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
fn walk_directory(current_dir: &Path, source_dir: &Path, dbf_files: &mut Vec<DbfFile>) -> Result<()> {
    let entries = std::fs::read_dir(current_dir)
        .map_err(|e| ProcessingError::DirectoryInaccessible(
            format!("Cannot read directory {}: {}", current_dir.display(), e)
        ))?;

    for entry in entries {
        let entry = entry.map_err(ProcessingError::FileReadError)?;
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories
            walk_directory(&path, source_dir, dbf_files)?;
        } else if path.is_file() {
            // Check if file has .dbf extension (case-insensitive)
            if let Some(extension) = path.extension() {
                if extension.eq_ignore_ascii_case("dbf") {
                    let dbf_file = DbfFile::new(path.clone(), source_dir);
                    debug!("Found DBF file: {}", dbf_file.relative_path.display());
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
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = scan_directory(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_scan_with_dbf_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test DBF files
        fs::write(temp_dir.path().join("file1.dbf"), b"test").unwrap();
        fs::write(temp_dir.path().join("file2.DBF"), b"test").unwrap(); // Test case-insensitivity
        fs::write(temp_dir.path().join("not_dbf.txt"), b"test").unwrap();

        let result = scan_directory(temp_dir.path()).unwrap();
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

        let result = scan_directory(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 3);

        // Verify relative paths are calculated correctly
        let paths: Vec<_> = result.iter().map(|f| f.relative_path.to_string_lossy().to_string()).collect();
        assert!(paths.iter().any(|p| p == "root.dbf"));
        assert!(paths.iter().any(|p| p.contains("subdir1") && p.contains("level1.dbf")));
        assert!(paths.iter().any(|p| p.contains("subdir2") && p.contains("level2.dbf")));
    }

    #[test]
    fn test_scan_nonexistent_directory() {
        let result = scan_directory("/nonexistent/path/12345");
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

        let result = scan_directory(&file_path);
        assert!(result.is_err());
        match result {
            Err(ProcessingError::DirectoryInaccessible(_)) => (),
            _ => panic!("Expected DirectoryInaccessible error"),
        }
    }
}
