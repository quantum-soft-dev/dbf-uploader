use super::BatchState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Batch represents a file upload session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    /// Batch ID from middleware
    pub id: Uuid,
    /// Current state of the batch
    pub state: BatchState,
    /// Timestamp when batch was started
    pub started_at: DateTime<Utc>,
    /// Timestamp when batch completed (if terminal state)
    pub completed_at: Option<DateTime<Utc>>,
    /// Number of files uploaded
    pub uploaded_files_count: u32,
    /// Total size of uploaded files in bytes
    pub total_size_bytes: u64,
    /// Number of errors encountered
    pub error_count: u32,
}

impl Batch {
    /// Create a new batch in Created state
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            state: BatchState::Created,
            started_at: Utc::now(),
            completed_at: None,
            uploaded_files_count: 0,
            total_size_bytes: 0,
            error_count: 0,
        }
    }

    /// Transition to a new state with validation
    pub fn transition_to(&mut self, new_state: BatchState) -> Result<(), String> {
        if !self.state.can_transition_to(new_state) {
            return Err(format!(
                "Invalid state transition from {} to {}",
                self.state, new_state
            ));
        }

        self.state = new_state;

        // Set completed_at timestamp for terminal states
        if new_state.is_terminal() && self.completed_at.is_none() {
            self.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Mark batch as started (Created → InProgress)
    pub fn mark_started(&mut self) -> Result<(), String> {
        self.transition_to(BatchState::InProgress)
    }

    /// Mark batch as completed
    pub fn mark_completed(&mut self) -> Result<(), String> {
        self.transition_to(BatchState::Completed)
    }

    /// Mark batch as failed
    pub fn mark_failed(&mut self) -> Result<(), String> {
        self.transition_to(BatchState::Failed)
    }

    /// Mark batch as cancelled
    pub fn mark_cancelled(&mut self) -> Result<(), String> {
        self.transition_to(BatchState::Cancelled)
    }

    /// Add uploaded file statistics
    pub fn add_uploaded_file(&mut self, file_size: u64) {
        self.uploaded_files_count += 1;
        self.total_size_bytes += file_size;
    }

    /// Increment error counter
    pub fn increment_error_count(&mut self) {
        self.error_count += 1;
    }

    /// Get batch duration if completed
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at.map(|completed| completed - self.started_at)
    }

    /// Check if batch has errors
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if batch is in progress
    pub fn is_in_progress(&self) -> bool {
        self.state == BatchState::InProgress
    }

    /// Check if batch is completed successfully
    pub fn is_completed(&self) -> bool {
        self.state == BatchState::Completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_batch() {
        let batch_id = Uuid::new_v4();
        let batch = Batch::new(batch_id);

        assert_eq!(batch.id, batch_id);
        assert_eq!(batch.state, BatchState::Created);
        assert_eq!(batch.uploaded_files_count, 0);
        assert_eq!(batch.total_size_bytes, 0);
        assert_eq!(batch.error_count, 0);
        assert!(batch.completed_at.is_none());
    }

    #[test]
    fn test_mark_started() {
        let mut batch = Batch::new(Uuid::new_v4());
        assert!(batch.mark_started().is_ok());
        assert_eq!(batch.state, BatchState::InProgress);
        assert!(batch.completed_at.is_none());
    }

    #[test]
    fn test_mark_completed() {
        let mut batch = Batch::new(Uuid::new_v4());
        batch.mark_started().unwrap();
        assert!(batch.mark_completed().is_ok());
        assert_eq!(batch.state, BatchState::Completed);
        assert!(batch.completed_at.is_some());
    }

    #[test]
    fn test_mark_failed() {
        let mut batch = Batch::new(Uuid::new_v4());
        batch.mark_started().unwrap();
        assert!(batch.mark_failed().is_ok());
        assert_eq!(batch.state, BatchState::Failed);
        assert!(batch.completed_at.is_some());
    }

    #[test]
    fn test_mark_cancelled() {
        let mut batch = Batch::new(Uuid::new_v4());
        batch.mark_started().unwrap();
        assert!(batch.mark_cancelled().is_ok());
        assert_eq!(batch.state, BatchState::Cancelled);
        assert!(batch.completed_at.is_some());
    }

    #[test]
    fn test_invalid_transition() {
        let mut batch = Batch::new(Uuid::new_v4());
        // Cannot go from Created directly to Completed
        let result = batch.mark_completed();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid state transition"));
    }

    #[test]
    fn test_add_uploaded_file() {
        let mut batch = Batch::new(Uuid::new_v4());
        batch.add_uploaded_file(1024);
        batch.add_uploaded_file(2048);

        assert_eq!(batch.uploaded_files_count, 2);
        assert_eq!(batch.total_size_bytes, 3072);
    }

    #[test]
    fn test_increment_error_count() {
        let mut batch = Batch::new(Uuid::new_v4());
        assert_eq!(batch.error_count, 0);
        assert!(!batch.has_errors());

        batch.increment_error_count();
        assert_eq!(batch.error_count, 1);
        assert!(batch.has_errors());
    }

    #[test]
    fn test_duration() {
        let mut batch = Batch::new(Uuid::new_v4());
        assert!(batch.duration().is_none());

        batch.mark_started().unwrap();
        batch.mark_completed().unwrap();

        let duration = batch.duration();
        assert!(duration.is_some());
        assert!(duration.unwrap().num_milliseconds() >= 0);
    }

    #[test]
    fn test_helper_methods() {
        let mut batch = Batch::new(Uuid::new_v4());
        assert!(!batch.is_in_progress());
        assert!(!batch.is_completed());

        batch.mark_started().unwrap();
        assert!(batch.is_in_progress());
        assert!(!batch.is_completed());

        batch.mark_completed().unwrap();
        assert!(!batch.is_in_progress());
        assert!(batch.is_completed());
    }
}
