// DBF File model for tracking file processing
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Encoding {
    CP866,
    Windows1251,
    UTF8,
}

impl Encoding {
    /// Convert encoding enum to string for use with encoding_rs
    pub fn as_str(&self) -> &'static str {
        match self {
            Encoding::CP866 => "CP866",
            Encoding::Windows1251 => "WINDOWS-1251",
            Encoding::UTF8 => "UTF-8",
        }
    }

    /// Parse encoding from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "CP866" => Some(Encoding::CP866),
            "WINDOWS-1251" | "WINDOWS1251" => Some(Encoding::Windows1251),
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
        assert_eq!(
            Encoding::parse("WINDOWS-1251"),
            Some(Encoding::Windows1251)
        );
        assert_eq!(
            Encoding::parse("windows1251"),
            Some(Encoding::Windows1251)
        );
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
}
