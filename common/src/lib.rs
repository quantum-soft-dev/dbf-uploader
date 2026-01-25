// Common library for Data Exporter
// Shared between installer, configurator, uninstaller, and service

pub mod auth;
pub mod error;
pub mod file_utils;
pub mod models;
pub mod paths;
pub mod version;

// Re-export commonly used types for convenience
pub use auth::{device_flow, AuthClient, JwtToken, TokenManager};
pub use error::{ErrorReporter, ProcessingError};
pub use models::{Config, CredentialConfig, DeviceCredentials};
pub use version::Version;
