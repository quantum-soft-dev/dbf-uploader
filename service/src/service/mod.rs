// Windows service implementation module
pub mod scheduler;

#[cfg(windows)]
pub mod windows_service;

pub use scheduler::BatchScheduler;
