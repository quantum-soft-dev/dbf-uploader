///! Error types for VSS operations

use thiserror::Error;

/// Errors that can occur during VSS operations
#[derive(Error, Debug)]
pub enum VssError {
    /// Failed to initialize VSS client (usually requires Administrator privileges)
    #[error("Failed to initialize VSS client: {0}")]
    InitializationFailed(String),

    /// Failed to create a VSS snapshot
    #[error("Failed to create VSS snapshot: {0}")]
    SnapshotCreationFailed(String),

    /// Failed to access VSS snapshot properties
    #[error("Failed to access VSS snapshot: {0}")]
    SnapshotAccessFailed(String),

    /// Failed to delete VSS snapshot
    #[error("Failed to delete VSS snapshot: {0}")]
    SnapshotDeletionFailed(String),

    /// VSS is not available on this system
    #[error("VSS is not available on this system")]
    VssNotAvailable,

    /// Insufficient privileges to perform VSS operations
    #[error("Administrator privileges required for VSS operations")]
    InsufficientPrivileges,
}
