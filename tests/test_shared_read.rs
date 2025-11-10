// Integration test for shared file reading
#[cfg(windows)]
#[test]
fn test_read_file_opened_by_another_process() {
    use data_exporter::file_utils::open_shared_read;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Write};
    use std::os::windows::fs::OpenOptionsExt;
    use tempfile::tempdir;

    // Create a temporary directory and file
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.dbf");

    // Create and write test content
    {
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Test content for shared read").unwrap();
        file.flush().unwrap();
    }

    // Keep the file open with write access (simulating another process like FoxPro)
    let _write_handle = OpenOptions::new()
        .write(true)
        .read(true)
        .share_mode(0x00000001 | 0x00000002) // FILE_SHARE_READ | FILE_SHARE_WRITE
        .open(&file_path)
        .expect("Failed to open file with write access");

    // Now try to open the same file for reading using our shared read function
    let mut read_handle = open_shared_read(&file_path)
        .expect("Failed to open file with shared read while it's opened by another process");

    // Read the content
    let mut content = String::new();
    read_handle.read_to_string(&mut content).unwrap();

    assert_eq!(content, "Test content for shared read");
}

#[cfg(not(windows))]
#[test]
fn test_read_file_unix() {
    use data_exporter::file_utils::open_shared_read;
    use std::io::{Read, Write};
    use tempfile::NamedTempFile;

    // Create a temporary file
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"Test content").unwrap();
    temp_file.flush().unwrap();

    // Open it with shared read
    let mut file = open_shared_read(temp_file.path()).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    assert_eq!(content, "Test content");
}
