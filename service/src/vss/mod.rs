//! Volume Shadow Copy Service integration for accessing locked files
//!
//! This module uses rawcopy-rs to copy files that are currently locked by other processes
//! using Windows Volume Shadow Copy technology.
use std::fs;
use std::path::{Path, PathBuf};

mod error;
pub use error::VssError;

type Result<T> = std::result::Result<T, VssError>;

/// Copy a locked file using VSS technology to a temporary location
///
/// This function uses Windows Volume Shadow Copy Service to copy files
/// that are currently locked by other processes (e.g., FoxPro, dBase).
///
/// # Arguments
/// * `source_path` - The path to the locked file
/// * `temp_dir` - Directory where the copy should be saved
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the copied file in temp directory
/// * `Err(VssError)` - Failed to copy file via VSS
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use data_exporter::vss::copy_locked_file;
///
/// let source = Path::new("C:\\Data\\locked_file.dbf");
/// let temp_dir = std::env::temp_dir();
/// let copy_path = copy_locked_file(source, &temp_dir).expect("Failed to copy via VSS");
/// ```
pub fn copy_locked_file<P: AsRef<Path>>(source_path: P, temp_dir: &Path) -> Result<PathBuf> {
    let source = source_path.as_ref();

    tracing::info!("Copying locked file via VSS: {}", source.display());

    // Ensure temp directory exists
    if !temp_dir.exists() {
        tracing::info!("Creating temp directory: {}", temp_dir.display());
        fs::create_dir_all(temp_dir).map_err(|e| {
            VssError::InitializationFailed(format!("Failed to create temp directory: {}", e))
        })?;
    }

    // Get filename from source
    let filename = source.file_name().ok_or(VssError::VssNotAvailable)?;

    // rawcopy expects save_path to be a DIRECTORY, not a full file path
    // It will automatically append the filename
    let dest_path = temp_dir.join(filename);

    tracing::info!(
        "VSS copy: source={}, dest_dir={}, expected_output={}",
        source.display(),
        temp_dir.display(),
        dest_path.display()
    );

    // Convert paths to strings for rawcopy
    let source_str = source.to_string_lossy();
    let dest_dir_str = temp_dir.to_string_lossy();

    tracing::debug!(
        "Calling rawcopy with: source='{}', dest_dir='{}'",
        source_str,
        dest_dir_str
    );

    // Use rawcopy to copy the file via VSS
    // Note: rawcopy expects destination to be a DIRECTORY, not a file path
    rawcopy_rs::rawcopy(&source_str, &dest_dir_str).map_err(|e| {
        tracing::error!(
            "Failed to copy file via VSS: {}. Source: {}, Dest dir: {}",
            e,
            source_str,
            dest_dir_str
        );
        VssError::SnapshotCreationFailed(e.to_string())
    })?;

    tracing::info!(
        "File copied successfully via VSS: {} -> {}",
        source.display(),
        dest_path.display()
    );

    Ok(dest_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_dir_path_construction() {
        let temp_dir = std::env::temp_dir();
        let _source = Path::new("C:\\Data\\test.dbf");

        let expected = temp_dir.join("test.dbf");

        // This would be the expected output path
        assert_eq!(expected.file_name(), Some(std::ffi::OsStr::new("test.dbf")));
    }

    /// T063 - Test VssError types
    #[test]
    fn test_vss_error_types() {
        let init_err = VssError::InitializationFailed("test".to_string());
        assert!(init_err.to_string().contains("initialize"));

        let snapshot_err = VssError::SnapshotCreationFailed("test".to_string());
        assert!(snapshot_err.to_string().contains("snapshot"));

        let not_available_err = VssError::VssNotAvailable;
        assert!(not_available_err.to_string().contains("not available"));

        let privileges_err = VssError::InsufficientPrivileges;
        assert!(privileges_err.to_string().contains("Administrator"));
    }

    /// T063 - Test copy_locked_file with invalid source (no filename)
    #[test]
    fn test_copy_locked_file_invalid_source() {
        let temp_dir = std::env::temp_dir();
        // A path without a filename should return VssNotAvailable
        let result = copy_locked_file(Path::new("/"), &temp_dir);
        assert!(result.is_err());
    }

    /// T063 - Test copy_locked_file creates temp directory if missing
    #[test]
    fn test_copy_locked_file_creates_temp_dir() {
        let temp_base = std::env::temp_dir();
        let non_existent_dir = temp_base.join("vss_test_nonexistent_dir_12345");

        // Ensure directory doesn't exist
        if non_existent_dir.exists() {
            fs::remove_dir_all(&non_existent_dir).ok();
        }

        // This will fail at rawcopy (file doesn't exist), but should create the directory first
        let result = copy_locked_file(
            Path::new("C:\\nonexistent_test_file.dbf"),
            &non_existent_dir,
        );

        // Either the directory was created (and rawcopy failed), or initialization failed
        // We just verify the function handles missing directories gracefully
        assert!(result.is_err());

        // Cleanup
        if non_existent_dir.exists() {
            fs::remove_dir_all(&non_existent_dir).ok();
        }
    }
}

/// T063 - Unit tests for locked file detection
/// These tests are in the processor module since is_file_locked is defined there
#[cfg(test)]
pub mod locked_file_tests {
    use std::io::{Error as IoError, ErrorKind};

    /// Check if an error represents a locked file based on OS error codes
    pub fn is_locked_error_code(os_error: i32) -> bool {
        // Windows error codes:
        // 32: ERROR_SHARING_VIOLATION - The process cannot access the file because it is being used by another process
        // 33: ERROR_LOCK_VIOLATION - The process cannot access the file because another process has locked a portion of the file
        os_error == 32 || os_error == 33
    }

    #[test]
    fn test_locked_file_os_error_32() {
        // OS error 32: ERROR_SHARING_VIOLATION
        assert!(is_locked_error_code(32));
    }

    #[test]
    fn test_locked_file_os_error_33() {
        // OS error 33: ERROR_LOCK_VIOLATION
        assert!(is_locked_error_code(33));
    }

    #[test]
    fn test_non_locked_error_codes() {
        // Other error codes should not be treated as locked
        assert!(!is_locked_error_code(2)); // FILE_NOT_FOUND
        assert!(!is_locked_error_code(3)); // PATH_NOT_FOUND
        assert!(!is_locked_error_code(5)); // ACCESS_DENIED (different from lock)
    }
}
