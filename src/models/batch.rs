// Batch model for tracking processing batches
use crate::models::Config;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BatchStatus {
    Scanning,
    Processing,
    RetryingLocked,
    Completed,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingStatus {
    Pending,
    Locked,
    Converting,
    Compressing,
    Uploading,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Batch {
    pub batch_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub config_snapshot: Config,
    pub files: Vec<PathBuf>,
    pub locked_files: Vec<PathBuf>,
    pub processed_count: usize,
    pub failed_count: usize,
    pub status: BatchStatus,
}

impl Batch {
    /// Create a new batch with given configuration
    pub fn new(config: Config) -> Self {
        Self {
            batch_id: Uuid::new_v4(),
            started_at: Utc::now(),
            config_snapshot: config,
            files: Vec::new(),
            locked_files: Vec::new(),
            processed_count: 0,
            failed_count: 0,
            status: BatchStatus::Scanning,
        }
    }

    /// Add a file to the batch
    pub fn add_file(&mut self, path: PathBuf) {
        self.files.push(path);
    }

    /// Defer a locked file to be retried later
    pub fn defer_locked_file(&mut self, path: PathBuf) {
        self.locked_files.push(path);
    }

    /// Mark a file as successfully processed
    pub fn mark_completed(&mut self) {
        self.processed_count += 1;
    }

    /// Mark a file as failed
    pub fn mark_failed(&mut self) {
        self.failed_count += 1;
    }

    /// Get total number of files in batch
    pub fn total_files(&self) -> usize {
        self.files.len() + self.locked_files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_config() -> Config {
        Config {
            scheduler: crate::models::config::SchedulerConfig {
                crontab: "*/5 * * * *".to_string(),
            },
            src: crate::models::config::SourceConfig {
                source_dir: PathBuf::from("/tmp"),
                include_patterns: None,
                exclude_patterns: None,
            },
            credential: crate::models::config::CredentialConfig {
                account: "testaccount".to_string(),
                username: "test".to_string(),
                password: "test".to_string(),
            },
            api: crate::models::config::ApiConfig {
                base_url: "https://api.example.com".to_string(),
                https_only: true,
            },
            encoding: crate::models::config::EncodingConfig {
                dbf_encoding: "CP866".to_string(),
            },
        }
    }

    #[test]
    fn test_batch_creation() {
        let config = create_test_config();
        let batch = Batch::new(config.clone());

        assert_eq!(batch.status, BatchStatus::Scanning);
        assert_eq!(batch.processed_count, 0);
        assert_eq!(batch.failed_count, 0);
        assert!(batch.files.is_empty());
        assert!(batch.locked_files.is_empty());
    }

    #[test]
    fn test_batch_add_file() {
        let config = create_test_config();
        let mut batch = Batch::new(config);

        batch.add_file(PathBuf::from("/tmp/test.dbf"));
        assert_eq!(batch.files.len(), 1);
        assert_eq!(batch.total_files(), 1);
    }

    #[test]
    fn test_batch_defer_locked_file() {
        let config = create_test_config();
        let mut batch = Batch::new(config);

        batch.defer_locked_file(PathBuf::from("/tmp/locked.dbf"));
        assert_eq!(batch.locked_files.len(), 1);
        assert_eq!(batch.total_files(), 1);
    }

    #[test]
    fn test_batch_mark_completed() {
        let config = create_test_config();
        let mut batch = Batch::new(config);

        batch.mark_completed();
        assert_eq!(batch.processed_count, 1);
        assert_eq!(batch.failed_count, 0);
    }

    #[test]
    fn test_batch_mark_failed() {
        let config = create_test_config();
        let mut batch = Batch::new(config);

        batch.mark_failed();
        assert_eq!(batch.processed_count, 0);
        assert_eq!(batch.failed_count, 1);
    }
}
