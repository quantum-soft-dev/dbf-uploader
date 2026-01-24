// DBF File model for tracking file processing
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Encoding {
    CP866,
    Windows1251,
    Windows1255, // Hebrew (Israel)
    ISO8859_8,   // Hebrew (ISO)
    UTF8,
}

impl Encoding {
    /// Convert encoding enum to string for use with encoding_rs
    pub fn as_str(&self) -> &'static str {
        match self {
            Encoding::CP866 => "CP866",
            Encoding::Windows1251 => "WINDOWS-1251",
            Encoding::Windows1255 => "WINDOWS-1255",
            Encoding::ISO8859_8 => "ISO-8859-8",
            Encoding::UTF8 => "UTF-8",
        }
    }

    /// Parse encoding from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "CP866" | "IBM866" => Some(Encoding::CP866),
            "WINDOWS-1251" | "WINDOWS1251" | "CP1251" => Some(Encoding::Windows1251),
            "WINDOWS-1255" | "WINDOWS1255" | "CP1255" => Some(Encoding::Windows1255),
            "ISO-8859-8" | "ISO88598" | "ISO8859-8" | "ISO8859_8" => Some(Encoding::ISO8859_8),
            "UTF-8" | "UTF8" => Some(Encoding::UTF8),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileProcessingStatus {
    Pending,
    Locked,
    Converting,
    Compressing,
    Uploading,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct DbfFile {
    /// Full filesystem path
    pub path: PathBuf,

    /// Path relative to source directory
    pub relative_path: PathBuf,

    /// Detected or fallback encoding
    pub encoding: Option<Encoding>,

    /// Current processing status
    pub status: FileProcessingStatus,
}

impl DbfFile {
    /// Create a new DBF file instance
    ///
    /// # Arguments
    /// * `path` - Full filesystem path to the DBF file
    /// * `source_dir` - Source directory to calculate relative path
    pub fn new(path: PathBuf, source_dir: &Path) -> Self {
        let relative_path = path.strip_prefix(source_dir).unwrap_or(&path).to_path_buf();

        Self {
            path,
            relative_path,
            encoding: None,
            status: FileProcessingStatus::Pending,
        }
    }

    /// Generate compressed filename from relative path
    /// Converts path separators to underscores and adds .csv.gz extension
    ///
    /// Example: `subdir\data.dbf` → `subdir_data.csv.gz`
    pub fn generate_compressed_filename(&self) -> String {
        let path_str = self.relative_path.to_string_lossy();

        // Replace path separators with underscores
        let encoded = path_str.replace(['\\', '/'], "_");

        // Replace .dbf extension with .csv.gz
        if let Some(stem) = encoded.strip_suffix(".dbf") {
            format!("{}.csv.gz", stem)
        } else if let Some(stem) = encoded.strip_suffix(".DBF") {
            format!("{}.csv.gz", stem)
        } else {
            format!("{}.csv.gz", encoded)
        }
    }

    /// Set the encoding for this file
    pub fn set_encoding(&mut self, encoding: Encoding) {
        self.encoding = Some(encoding);
    }

    /// Set the processing status
    pub fn set_status(&mut self, status: FileProcessingStatus) {
        self.status = status;
    }

    /// Detect encoding from DBF file header (byte 29 - Language Driver ID)
    /// Returns None if file cannot be read or encoding is unknown
    pub fn detect_encoding_from_header(&mut self) -> Option<Encoding> {
        use crate::file_utils::open_shared_read;
        use std::io::Read;

        let mut file = open_shared_read(&self.path).ok()?;
        let mut header = [0u8; 32];
        file.read_exact(&mut header).ok()?;

        // Byte 29 contains the Language Driver ID (LDID)
        let ldid = header[29];

        let encoding = match ldid {
            0x57 => Some(Encoding::Windows1255), // Hebrew (Windows-1255)
            0x69 | 0xCA => Some(Encoding::ISO8859_8), // Hebrew (ISO-8859-8)
            0xC8 | 0xC9 | 0xCB => Some(Encoding::Windows1251), // Windows-1251 (Cyrillic)
            0x65 | 0x66 => Some(Encoding::CP866), // CP866 (DOS Cyrillic)
            0x4D | 0x7C => Some(Encoding::UTF8), // UTF-8
            _ => None,                           // Unknown encoding - will use config fallback
        };

        if let Some(ref enc) = encoding {
            self.encoding = Some(enc.clone());
        }

        encoding
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_dbf_file_creation() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/reports/sales.dbf");

        let dbf_file = DbfFile::new(file_path.clone(), &source_dir);

        assert_eq!(dbf_file.path, file_path);
        assert_eq!(dbf_file.relative_path, PathBuf::from("reports/sales.dbf"));
        assert_eq!(dbf_file.status, FileProcessingStatus::Pending);
        assert!(dbf_file.encoding.is_none());
    }

    #[test]
    fn test_generate_compressed_filename_unix() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/reports/sales.dbf");
        let dbf_file = DbfFile::new(file_path, &source_dir);

        let filename = dbf_file.generate_compressed_filename();
        assert_eq!(filename, "reports_sales.csv.gz");
    }

    #[test]
    fn test_generate_compressed_filename_nested() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/archive/2024/monthly/report.dbf");
        let dbf_file = DbfFile::new(file_path, &source_dir);

        let filename = dbf_file.generate_compressed_filename();
        assert_eq!(filename, "archive_2024_monthly_report.csv.gz");
    }

    #[test]
    fn test_generate_compressed_filename_uppercase() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/DATA.DBF");
        let dbf_file = DbfFile::new(file_path, &source_dir);

        let filename = dbf_file.generate_compressed_filename();
        assert_eq!(filename, "DATA.csv.gz");
    }

    #[test]
    fn test_encoding_as_str() {
        assert_eq!(Encoding::CP866.as_str(), "CP866");
        assert_eq!(Encoding::Windows1251.as_str(), "WINDOWS-1251");
        assert_eq!(Encoding::UTF8.as_str(), "UTF-8");
    }

    #[test]
    fn test_encoding_from_str() {
        assert_eq!(Encoding::parse("CP866"), Some(Encoding::CP866));
        assert_eq!(Encoding::parse("cp866"), Some(Encoding::CP866));
        assert_eq!(Encoding::parse("WINDOWS-1251"), Some(Encoding::Windows1251));
        assert_eq!(Encoding::parse("windows1251"), Some(Encoding::Windows1251));
        assert_eq!(Encoding::parse("UTF-8"), Some(Encoding::UTF8));
        assert_eq!(Encoding::parse("utf8"), Some(Encoding::UTF8));
        assert_eq!(Encoding::parse("invalid"), None);
    }

    #[test]
    fn test_set_encoding() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        dbf_file.set_encoding(Encoding::CP866);
        assert_eq!(dbf_file.encoding, Some(Encoding::CP866));
    }

    #[test]
    fn test_set_status() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        dbf_file.set_status(FileProcessingStatus::Converting);
        assert_eq!(dbf_file.status, FileProcessingStatus::Converting);
    }

    // Test 1: FileProcessingStatus transitions: Pending -> Converting -> Compressing -> Uploading -> Completed
    #[test]
    fn test_status_transition_happy_path() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        // Initial status should be Pending
        assert_eq!(dbf_file.status, FileProcessingStatus::Pending);

        // Transition: Pending -> Converting
        dbf_file.set_status(FileProcessingStatus::Converting);
        assert_eq!(dbf_file.status, FileProcessingStatus::Converting);

        // Transition: Converting -> Compressing
        dbf_file.set_status(FileProcessingStatus::Compressing);
        assert_eq!(dbf_file.status, FileProcessingStatus::Compressing);

        // Transition: Compressing -> Uploading
        dbf_file.set_status(FileProcessingStatus::Uploading);
        assert_eq!(dbf_file.status, FileProcessingStatus::Uploading);

        // Transition: Uploading -> Completed
        dbf_file.set_status(FileProcessingStatus::Completed);
        assert_eq!(dbf_file.status, FileProcessingStatus::Completed);
    }

    // Test 2: FileProcessingStatus transitions: Pending -> Locked
    #[test]
    fn test_status_transition_pending_to_locked() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/locked_file.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        // Initial status should be Pending
        assert_eq!(dbf_file.status, FileProcessingStatus::Pending);

        // Transition: Pending -> Locked (file is locked by another process)
        dbf_file.set_status(FileProcessingStatus::Locked);
        assert_eq!(dbf_file.status, FileProcessingStatus::Locked);
    }

    // Test 3: FileProcessingStatus transitions: Any state -> Failed
    #[test]
    fn test_status_transition_to_failed_from_pending() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        assert_eq!(dbf_file.status, FileProcessingStatus::Pending);
        dbf_file.set_status(FileProcessingStatus::Failed);
        assert_eq!(dbf_file.status, FileProcessingStatus::Failed);
    }

    #[test]
    fn test_status_transition_to_failed_from_converting() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        dbf_file.set_status(FileProcessingStatus::Converting);
        assert_eq!(dbf_file.status, FileProcessingStatus::Converting);

        // Transition: Converting -> Failed
        dbf_file.set_status(FileProcessingStatus::Failed);
        assert_eq!(dbf_file.status, FileProcessingStatus::Failed);
    }

    #[test]
    fn test_status_transition_to_failed_from_compressing() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        dbf_file.set_status(FileProcessingStatus::Compressing);
        assert_eq!(dbf_file.status, FileProcessingStatus::Compressing);

        // Transition: Compressing -> Failed
        dbf_file.set_status(FileProcessingStatus::Failed);
        assert_eq!(dbf_file.status, FileProcessingStatus::Failed);
    }

    #[test]
    fn test_status_transition_to_failed_from_uploading() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        dbf_file.set_status(FileProcessingStatus::Uploading);
        assert_eq!(dbf_file.status, FileProcessingStatus::Uploading);

        // Transition: Uploading -> Failed
        dbf_file.set_status(FileProcessingStatus::Failed);
        assert_eq!(dbf_file.status, FileProcessingStatus::Failed);
    }

    #[test]
    fn test_status_transition_to_failed_from_locked() {
        let source_dir = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        dbf_file.set_status(FileProcessingStatus::Locked);
        assert_eq!(dbf_file.status, FileProcessingStatus::Locked);

        // Transition: Locked -> Failed
        dbf_file.set_status(FileProcessingStatus::Failed);
        assert_eq!(dbf_file.status, FileProcessingStatus::Failed);
    }

    // Test 4: LDID to Encoding mapping - verify specific LDID values map to correct encodings
    // These tests verify the LDID byte mappings documented in detect_encoding_from_header
    #[test]
    fn test_ldid_encoding_mappings() {
        // Windows-1255 (Hebrew)
        assert_eq!(ldid_to_encoding(0x57), Some(Encoding::Windows1255));

        // ISO-8859-8 (Hebrew ISO)
        assert_eq!(ldid_to_encoding(0x69), Some(Encoding::ISO8859_8));
        assert_eq!(ldid_to_encoding(0xCA), Some(Encoding::ISO8859_8));

        // Windows-1251 (Cyrillic)
        assert_eq!(ldid_to_encoding(0xC8), Some(Encoding::Windows1251));
        assert_eq!(ldid_to_encoding(0xC9), Some(Encoding::Windows1251));
        assert_eq!(ldid_to_encoding(0xCB), Some(Encoding::Windows1251));

        // CP866 (DOS Cyrillic)
        assert_eq!(ldid_to_encoding(0x65), Some(Encoding::CP866));
        assert_eq!(ldid_to_encoding(0x66), Some(Encoding::CP866));

        // UTF-8
        assert_eq!(ldid_to_encoding(0x4D), Some(Encoding::UTF8));
        assert_eq!(ldid_to_encoding(0x7C), Some(Encoding::UTF8));

        // Unknown LDID values should return None
        assert_eq!(ldid_to_encoding(0x00), None);
        assert_eq!(ldid_to_encoding(0x01), None);
        assert_eq!(ldid_to_encoding(0xFF), None);
    }

    /// Helper function that mirrors the LDID to encoding logic from detect_encoding_from_header
    fn ldid_to_encoding(ldid: u8) -> Option<Encoding> {
        match ldid {
            0x57 => Some(Encoding::Windows1255),
            0x69 | 0xCA => Some(Encoding::ISO8859_8),
            0xC8 | 0xC9 | 0xCB => Some(Encoding::Windows1251),
            0x65 | 0x66 => Some(Encoding::CP866),
            0x4D | 0x7C => Some(Encoding::UTF8),
            _ => None,
        }
    }

    // Test 5: detect_encoding_from_header returns None for non-existent file
    #[test]
    fn test_detect_encoding_from_header_nonexistent_file() {
        let source_dir = PathBuf::from("/nonexistent/path");
        let file_path = PathBuf::from("/nonexistent/path/missing_file.dbf");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        // Should return None for non-existent file
        let result = dbf_file.detect_encoding_from_header();
        assert!(result.is_none());

        // encoding should remain None since file couldn't be read
        assert!(dbf_file.encoding.is_none());
    }

    #[test]
    fn test_detect_encoding_from_header_invalid_path() {
        let source_dir = PathBuf::from("");
        let file_path = PathBuf::from("");
        let mut dbf_file = DbfFile::new(file_path, &source_dir);

        // Should return None for empty/invalid path
        let result = dbf_file.detect_encoding_from_header();
        assert!(result.is_none());
        assert!(dbf_file.encoding.is_none());
    }
}
