// Data abstraction for in-memory or file-based processing
use std::path::PathBuf;

/// Represents data that can be either in memory or in a temp file
#[derive(Debug)]
pub enum ProcessingData {
    /// Data stored in memory
    InMemory(Vec<u8>),
    /// Data stored in a temporary file (for large files)
    TempFile(PathBuf),
}

impl ProcessingData {
    /// Get the size of the data in bytes
    pub fn size(&self) -> usize {
        match self {
            ProcessingData::InMemory(data) => data.len(),
            ProcessingData::TempFile(path) => std::fs::metadata(path)
                .map(|m| m.len() as usize)
                .unwrap_or(0),
        }
    }

    /// Check if data is in memory
    pub fn is_in_memory(&self) -> bool {
        matches!(self, ProcessingData::InMemory(_))
    }

    /// Get reference to in-memory data (if available)
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            ProcessingData::InMemory(data) => Some(data),
            ProcessingData::TempFile(_) => None,
        }
    }

    /// Get reference to temp file path (if available)
    pub fn as_path(&self) -> Option<&PathBuf> {
        match self {
            ProcessingData::InMemory(_) => None,
            ProcessingData::TempFile(path) => Some(path),
        }
    }
}

impl Drop for ProcessingData {
    fn drop(&mut self) {
        // Automatically clean up temp files when ProcessingData is dropped
        if let ProcessingData::TempFile(ref path) = self {
            if path.exists() {
                if let Err(e) = std::fs::remove_file(path) {
                    tracing::warn!(
                        file = %path.display(),
                        error = %e,
                        "Failed to delete temp file during cleanup"
                    );
                }
            }
        }
    }
}
