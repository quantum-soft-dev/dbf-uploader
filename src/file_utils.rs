// File utilities for cross-platform file operations
use std::fs::File;
use std::io;
use std::path::Path;

/// Open a file for reading with shared read access
/// On Windows, this allows reading files that are opened by other processes
/// On Unix, this is equivalent to File::open()
#[cfg(windows)]
pub fn open_shared_read<P: AsRef<Path>>(path: P) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::thread;
    use std::time::Duration;

    // Windows file sharing flags:
    // FILE_SHARE_READ (0x00000001) - Allow other processes to read
    // FILE_SHARE_WRITE (0x00000002) - Allow other processes to write
    // FILE_SHARE_DELETE (0x00000004) - Allow other processes to delete
    //
    // This combination allows reading files that are currently:
    // - Opened by other processes for reading
    // - Opened by other processes for writing
    // - Locked by database applications (FoxPro, dBase, etc.)

    // Try multiple times with exponential backoff for files that are temporarily locked
    const MAX_RETRIES: u32 = 3;
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .share_mode(0x00000001 | 0x00000002 | 0x00000004)
            .open(path.as_ref())
        {
            Ok(file) => return Ok(file),
            Err(e) => {
                // Check if it's a sharing violation (error code 32)
                if e.raw_os_error() == Some(32) && attempt < MAX_RETRIES - 1 {
                    // Wait with exponential backoff (50ms, 100ms, 200ms)
                    let wait_ms = 50 * 2u64.pow(attempt);
                    thread::sleep(Duration::from_millis(wait_ms));
                    last_error = Some(e);
                } else {
                    return Err(e);
                }
            }
        }
    }

    // Return the last error if all retries failed
    Err(last_error.unwrap())
}

#[cfg(not(windows))]
pub fn open_shared_read<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_open_shared_read() {
        // Create a temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        temp_file.flush().unwrap();

        // Open it with shared read
        let file = open_shared_read(temp_file.path()).unwrap();
        assert!(file.metadata().unwrap().is_file());
    }
}
