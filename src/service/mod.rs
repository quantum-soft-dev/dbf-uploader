// Windows service implementation module
pub mod scheduler;
pub mod uploader;

// NOTE: BatchScheduler commented out - will be replaced in Phase 3
// pub use scheduler::BatchScheduler;
pub use uploader::UploaderService;
