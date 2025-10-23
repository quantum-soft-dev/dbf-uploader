// Configuration migration module
pub mod auth_test;
pub mod detector;
pub mod migrator;
pub mod wizard;

use std::path::{Path, PathBuf};
use thiserror::Error;

pub use auth_test::{test_auth, AuthTestResult};
pub use detector::{detect_version, validate_config_file};
pub use migrator::{migrate_v1_to_v2, write_migration_guide};
pub use wizard::{run_wizard, write_config};

/// Configuration version identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigVersion {
    /// Version 1.0 - username/password authentication
    V1,
    /// Version 2.0 - site credentials authentication with batch protocol
    V2,
}

impl std::fmt::Display for ConfigVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigVersion::V1 => write!(f, "v1.0"),
            ConfigVersion::V2 => write!(f, "v2.0"),
        }
    }
}

/// Migration errors
#[derive(Debug, Error)]
pub enum MigrationError {
    /// Configuration file not found
    #[error("Configuration file not found: {0}")]
    ConfigNotFound(PathBuf),

    /// Failed to read configuration file
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    /// Configuration version could not be determined
    #[error("Unknown configuration version")]
    UnknownVersion,

    /// Failed to parse configuration
    #[error("Failed to parse config: {0}")]
    ParseError(String),

    /// Authentication test failed
    #[error("Authentication test failed: {0}")]
    AuthTestFailed(String),

    /// Migration was cancelled by user
    #[error("Migration cancelled")]
    Cancelled,

    /// Invalid configuration values
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Result type for migration operations
pub type MigrationResult<T> = Result<T, MigrationError>;

/// Migration guide containing v1 config and v2 template
#[derive(Debug, Clone)]
pub struct MigrationGuide {
    /// Path to old v1 configuration file
    pub old_config_path: PathBuf,
    /// Generated v2 configuration as TOML string
    pub template_config: String,
    /// Step-by-step migration instructions
    pub instructions: Vec<String>,
    /// Configuration items that were preserved
    pub preserved_settings: Vec<String>,
    /// Configuration items that require manual action
    pub manual_actions: Vec<String>,
}

impl MigrationGuide {
    /// Create a new migration guide
    pub fn new(old_config_path: PathBuf, template_config: String) -> Self {
        Self {
            old_config_path,
            template_config,
            instructions: Vec::new(),
            preserved_settings: Vec::new(),
            manual_actions: Vec::new(),
        }
    }

    /// Add an instruction step
    pub fn add_instruction(&mut self, instruction: String) {
        self.instructions.push(instruction);
    }

    /// Add a preserved setting
    pub fn add_preserved_setting(&mut self, setting: String) {
        self.preserved_settings.push(setting);
    }

    /// Add a manual action
    pub fn add_manual_action(&mut self, action: String) {
        self.manual_actions.push(action);
    }

    /// Display the migration guide
    pub fn display(&self) {
        println!("╔═══════════════════════════════════════════════════╗");
        println!("║   DBF Uploader Configuration Migration v1 → v2   ║");
        println!("╚═══════════════════════════════════════════════════╝");
        println!();
        println!("⚠️  BREAKING CHANGES:");
        println!("  • Username/password authentication REMOVED");
        println!("  • Site credentials (domain + clientSecret) REQUIRED");
        println!("  • Batch protocol lifecycle REQUIRED");
        println!();

        if !self.preserved_settings.is_empty() {
            println!("✅ Preserved Settings:");
            for setting in &self.preserved_settings {
                println!("  • {}", setting);
            }
            println!();
        }

        if !self.manual_actions.is_empty() {
            println!("⚠️  Manual Actions Required:");
            for action in &self.manual_actions {
                println!("  • {}", action);
            }
            println!();
        }

        println!("📋 Migration Steps:");
        for (i, instruction) in self.instructions.iter().enumerate() {
            println!("  {}. {}", i + 1, instruction);
        }
        println!();
    }
}

/// Configuration migration manager
#[derive(Debug)]
pub struct ConfigMigration {
    /// Path to configuration file
    config_path: PathBuf,
}

impl ConfigMigration {
    /// Create a new migration manager
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// Create migration manager with default config path
    pub fn with_default_path() -> Self {
        Self {
            config_path: PathBuf::from("config.toml"),
        }
    }

    /// Get the config path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Check if config file exists
    pub fn config_exists(&self) -> bool {
        self.config_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_version_display() {
        assert_eq!(ConfigVersion::V1.to_string(), "v1.0");
        assert_eq!(ConfigVersion::V2.to_string(), "v2.0");
    }

    #[test]
    fn test_config_version_equality() {
        assert_eq!(ConfigVersion::V1, ConfigVersion::V1);
        assert_eq!(ConfigVersion::V2, ConfigVersion::V2);
        assert_ne!(ConfigVersion::V1, ConfigVersion::V2);
    }

    #[test]
    fn test_migration_guide_creation() {
        let guide =
            MigrationGuide::new(PathBuf::from("old_config.toml"), "# v2 config".to_string());

        assert_eq!(guide.old_config_path, PathBuf::from("old_config.toml"));
        assert_eq!(guide.template_config, "# v2 config");
        assert!(guide.instructions.is_empty());
        assert!(guide.preserved_settings.is_empty());
        assert!(guide.manual_actions.is_empty());
    }

    #[test]
    fn test_migration_guide_add_instruction() {
        let mut guide = MigrationGuide::new(PathBuf::from("config.toml"), String::new());

        guide.add_instruction("Step 1".to_string());
        guide.add_instruction("Step 2".to_string());

        assert_eq!(guide.instructions.len(), 2);
        assert_eq!(guide.instructions[0], "Step 1");
        assert_eq!(guide.instructions[1], "Step 2");
    }

    #[test]
    fn test_migration_guide_add_preserved_setting() {
        let mut guide = MigrationGuide::new(PathBuf::from("config.toml"), String::new());

        guide.add_preserved_setting("source directory".to_string());
        guide.add_preserved_setting("cron schedule".to_string());

        assert_eq!(guide.preserved_settings.len(), 2);
    }

    #[test]
    fn test_migration_guide_add_manual_action() {
        let mut guide = MigrationGuide::new(PathBuf::from("config.toml"), String::new());

        guide.add_manual_action("Get site credentials".to_string());

        assert_eq!(guide.manual_actions.len(), 1);
    }

    #[test]
    fn test_config_migration_creation() {
        let migration = ConfigMigration::new(PathBuf::from("test_config.toml"));
        assert_eq!(migration.config_path(), Path::new("test_config.toml"));
    }

    #[test]
    fn test_config_migration_default_path() {
        let migration = ConfigMigration::with_default_path();
        assert_eq!(migration.config_path(), Path::new("config.toml"));
    }

    #[test]
    fn test_config_migration_exists() {
        let migration = ConfigMigration::new(PathBuf::from("Cargo.toml"));
        assert!(migration.config_exists());

        let migration = ConfigMigration::new(PathBuf::from("nonexistent.toml"));
        assert!(!migration.config_exists());
    }

    #[test]
    fn test_migration_error_display() {
        let err = MigrationError::UnknownVersion;
        assert_eq!(err.to_string(), "Unknown configuration version");

        let err = MigrationError::Cancelled;
        assert_eq!(err.to_string(), "Migration cancelled");

        let err = MigrationError::InvalidConfig("test error".to_string());
        assert_eq!(err.to_string(), "Invalid configuration: test error");
    }
}
