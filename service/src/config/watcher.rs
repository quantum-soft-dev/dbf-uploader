//! Config file watcher module for hot-reload functionality
use common::error::{ProcessingError, Result};
use common::models::Config;
use notify_debouncer_mini::{new_debouncer, notify, DebounceEventResult, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::{
    mpsc::{channel, Receiver},
    Arc, Mutex,
};
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub struct ConfigWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher>,
    config_path: PathBuf,
    change_receiver: Arc<Mutex<Receiver<()>>>,
}

impl ConfigWatcher {
    /// Create a new config file watcher
    /// Watches the config file for changes and signals when it's modified
    pub fn new<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let config_path = config_path.as_ref().to_path_buf();

        // Create channels for change notifications
        let (tx, rx) = channel();

        // Create debouncer with 2-second delay
        let mut debouncer = new_debouncer(
            Duration::from_secs(2),
            move |res: DebounceEventResult| match res {
                Ok(events) => {
                    for event in events {
                        debug!("Config file event: {:?}", event);
                        // Signal that config has changed
                        if tx.send(()).is_err() {
                            error!("Failed to send config change notification");
                        }
                    }
                }
                Err(error) => {
                    warn!("Config watch error: {:?}", error);
                }
            },
        )
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to create file watcher: {}", e))
        })?;

        // Watch the config file's parent directory
        // (watching individual files can be problematic with some editors)
        let watch_path = config_path
            .parent()
            .ok_or_else(|| {
                ProcessingError::ConfigurationError(
                    "Config path has no parent directory".to_string(),
                )
            })?
            .to_path_buf();

        // Start watching the directory for changes
        debouncer
            .watcher()
            .watch(&watch_path, notify::RecursiveMode::NonRecursive)
            .map_err(|e| {
                ProcessingError::ConfigurationError(format!(
                    "Failed to watch config directory: {}",
                    e
                ))
            })?;

        info!(
            path = %watch_path.display(),
            "Starting config file watcher"
        );

        Ok(Self {
            _debouncer: debouncer,
            config_path,
            change_receiver: Arc::new(Mutex::new(rx)),
        })
    }

    /// Check if the config file has changed
    /// Returns true if a change was detected, false otherwise
    pub fn has_changed(&self) -> bool {
        // Handle poisoned mutex gracefully - if poisoned, assume no change
        let receiver = match self.change_receiver.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Config watcher mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };
        receiver.try_recv().is_ok()
    }

    /// Wait for config file change (blocking)
    /// Returns the new config if successfully loaded
    pub fn wait_for_change(&self) -> Result<Config> {
        // Handle poisoned mutex gracefully
        let receiver = match self.change_receiver.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Config watcher mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };

        // Wait for change notification
        receiver.recv().map_err(|e| {
            ProcessingError::ConfigurationError(format!("Watcher channel error: {}", e))
        })?;

        info!("Config file changed, reloading");

        // Load and validate new config
        self.load_config()
    }

    /// Load the config file
    fn load_config(&self) -> Result<Config> {
        Config::from_file(&self.config_path).map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to load config: {}", e))
        })
    }

    /// Get the path being watched
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config_file(path: &Path) {
        let config_content = r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "/tmp"

[credential]
username = "test"
password = "test"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP866"
"#;
        fs::write(path, config_content).unwrap();
    }

    #[test]
    fn test_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        create_test_config_file(&config_path);

        let watcher = ConfigWatcher::new(&config_path);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watcher_has_changed_initially_false() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        create_test_config_file(&config_path);

        let watcher = ConfigWatcher::new(&config_path).unwrap();
        // Should not have changes immediately
        assert!(!watcher.has_changed());
    }

    #[test]
    fn test_watcher_config_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        create_test_config_file(&config_path);

        let watcher = ConfigWatcher::new(&config_path).unwrap();
        assert_eq!(watcher.config_path(), config_path.as_path());
    }
}
