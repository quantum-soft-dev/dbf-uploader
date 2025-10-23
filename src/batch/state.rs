use serde::{Deserialize, Serialize};
use std::fmt;

/// Batch state representing the lifecycle of a batch upload session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchState {
    /// Batch created but not yet started
    Created,
    /// Batch is actively receiving file uploads
    InProgress,
    /// Batch completed successfully
    Completed,
    /// Batch failed due to errors
    Failed,
    /// Batch was cancelled by user or timeout
    Cancelled,
}

impl BatchState {
    /// Check if transition to another state is valid
    pub fn can_transition_to(&self, new_state: BatchState) -> bool {
        use BatchState::*;

        match (self, new_state) {
            // From Created
            (Created, InProgress) => true,
            (Created, Cancelled) => true,

            // From InProgress
            (InProgress, Completed) => true,
            (InProgress, Failed) => true,
            (InProgress, Cancelled) => true,

            // Terminal states cannot transition
            (Completed, _) => false,
            (Failed, _) => false,
            (Cancelled, _) => false,

            // All other transitions invalid
            _ => false,
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            BatchState::Completed | BatchState::Failed | BatchState::Cancelled
        )
    }

    /// Check if batch is active (can accept uploads)
    pub fn is_active(&self) -> bool {
        matches!(self, BatchState::Created | BatchState::InProgress)
    }
}

impl fmt::Display for BatchState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchState::Created => write!(f, "Created"),
            BatchState::InProgress => write!(f, "InProgress"),
            BatchState::Completed => write!(f, "Completed"),
            BatchState::Failed => write!(f, "Failed"),
            BatchState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        use BatchState::*;

        // Created → InProgress
        assert!(Created.can_transition_to(InProgress));

        // Created → Cancelled
        assert!(Created.can_transition_to(Cancelled));

        // InProgress → Completed
        assert!(InProgress.can_transition_to(Completed));

        // InProgress → Failed
        assert!(InProgress.can_transition_to(Failed));

        // InProgress → Cancelled
        assert!(InProgress.can_transition_to(Cancelled));
    }

    #[test]
    fn test_invalid_transitions() {
        use BatchState::*;

        // Created cannot go directly to Completed/Failed
        assert!(!Created.can_transition_to(Completed));
        assert!(!Created.can_transition_to(Failed));

        // Terminal states cannot transition
        assert!(!Completed.can_transition_to(InProgress));
        assert!(!Completed.can_transition_to(Failed));
        assert!(!Failed.can_transition_to(Completed));
        assert!(!Cancelled.can_transition_to(InProgress));
    }

    #[test]
    fn test_is_terminal() {
        use BatchState::*;

        assert!(!Created.is_terminal());
        assert!(!InProgress.is_terminal());
        assert!(Completed.is_terminal());
        assert!(Failed.is_terminal());
        assert!(Cancelled.is_terminal());
    }

    #[test]
    fn test_is_active() {
        use BatchState::*;

        assert!(Created.is_active());
        assert!(InProgress.is_active());
        assert!(!Completed.is_active());
        assert!(!Failed.is_active());
        assert!(!Cancelled.is_active());
    }

    #[test]
    fn test_display() {
        use BatchState::*;

        assert_eq!(format!("{}", Created), "Created");
        assert_eq!(format!("{}", InProgress), "InProgress");
        assert_eq!(format!("{}", Completed), "Completed");
        assert_eq!(format!("{}", Failed), "Failed");
        assert_eq!(format!("{}", Cancelled), "Cancelled");
    }

    #[test]
    fn test_serialization() {
        use BatchState::*;

        let json = serde_json::to_string(&Created).unwrap();
        assert_eq!(json, r#""Created""#);

        let deserialized: BatchState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Created);
    }
}
