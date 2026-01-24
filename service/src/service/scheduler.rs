// Cron scheduler module
use crate::config::ConfigWatcher;
use crate::processor::run_batch;
use common::auth::TokenManager;
use common::error::{ProcessingError, Result};
use common::models::Config;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, warn};

pub struct BatchScheduler {
    scheduler: JobScheduler,
    config: Arc<RwLock<Config>>,
    config_path: Arc<PathBuf>,
    token_manager: Arc<TokenManager>,
    config_watcher: Option<ConfigWatcher>,
    batch_lock: Arc<Mutex<bool>>, // Lock to prevent concurrent batch execution
}

impl BatchScheduler {
    /// Create a new scheduler with the given configuration
    /// The config_path is used to reload configuration before each batch
    pub async fn new(config: Config, config_path: PathBuf) -> Result<Self> {
        let scheduler = JobScheduler::new().await.map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to create scheduler: {}", e))
        })?;

        let token_manager = Arc::new(TokenManager::new(&config)?);
        let config_arc = Arc::new(RwLock::new(config));
        let config_path_arc = Arc::new(config_path);
        let batch_lock = Arc::new(Mutex::new(false)); // false = not running

        Ok(Self {
            scheduler,
            config: config_arc,
            config_path: config_path_arc,
            token_manager,
            config_watcher: None,
            batch_lock,
        })
    }

    /// Start the scheduler with cron expression from config
    pub async fn start(&mut self) -> Result<()> {
        let config = self.config.read().await.clone();
        let crontab = config.scheduler.crontab.clone();

        // Convert 5-field cron to 6-field (add seconds at the beginning)
        // tokio-cron-scheduler expects: "sec min hour day month weekday"
        // Standard cron is: "min hour day month weekday"
        let crontab_6field = if crontab.split_whitespace().count() == 5 {
            format!("0 {}", crontab) // Add "0" for seconds at the beginning
        } else {
            crontab.clone()
        };

        info!(crontab = %crontab_6field, "Starting scheduler");

        // Create the batch processing job
        let config_arc = Arc::clone(&self.config);
        let config_path = Arc::clone(&self.config_path);
        let token_manager = Arc::clone(&self.token_manager);
        let batch_lock = Arc::clone(&self.batch_lock);

        let job = Job::new_async(crontab_6field.as_str(), move |_uuid, _l| {
            let config_arc = Arc::clone(&config_arc);
            let config_path = Arc::clone(&config_path);
            let token_manager = Arc::clone(&token_manager);
            let batch_lock = Arc::clone(&batch_lock);

            Box::pin(async move {
                // Try to acquire lock - if already running, skip this execution
                let mut is_running = batch_lock.lock().await;

                if *is_running {
                    warn!("Batch is already running, skipping this scheduled execution. Waiting for previous batch to complete.");
                    return;
                }

                // Mark as running
                *is_running = true;
                drop(is_running); // Release lock while batch runs

                // Reload config from disk at start of each batch (hot-reload)
                let reload_result: std::result::Result<Config, String> = Config::from_file(&config_path)
                    .map_err(|e| e.to_string());

                let current_config = match reload_result {
                    Ok(new_config) => {
                        // Update the shared config for other components
                        {
                            let mut config_guard = config_arc.write().await;
                            *config_guard = new_config.clone();
                        }
                        debug!("Config reloaded from disk before batch");
                        new_config
                    }
                    Err(error_msg) => {
                        // On reload failure, use cached config
                        warn!(
                            error = %error_msg,
                            "Failed to reload config from disk, using cached config"
                        );
                        let config_guard = config_arc.read().await;
                        config_guard.clone()
                    }
                };

                info!("========================================");
                info!("Scheduler woke up - starting batch execution");
                info!("Crontab: {}", current_config.scheduler.crontab);
                info!("Source: {}", current_config.src.source_dir.display());
                info!("========================================");

                let start_time = std::time::Instant::now();

                match run_batch(current_config, token_manager).await {
                    Ok(batch) => {
                        let duration = start_time.elapsed();
                        info!("========================================");
                        info!("BATCH COMPLETED SUCCESSFULLY");
                        info!("Batch ID: {}", batch.batch_id);
                        info!("Files processed: {}", batch.processed_count);
                        info!("Files failed: {}", batch.failed_count);
                        info!("Files deferred (locked): {}", batch.locked_files.len());
                        info!("Total files scanned: {}", batch.total_files());
                        info!("Duration: {:.2}s", duration.as_secs_f64());
                        info!("========================================");
                    }
                    Err(e) => {
                        let duration = start_time.elapsed();
                        error!("========================================");
                        error!("BATCH FAILED");
                        error!("Error: {}", e);
                        error!("Duration: {:.2}s", duration.as_secs_f64());
                        error!("========================================");
                    }
                }

                // Mark as not running
                let mut is_running = batch_lock.lock().await;
                *is_running = false;
            })
        })
        .map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to create cron job: {}", e))
        })?;

        self.scheduler.add(job).await.map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to add job to scheduler: {}", e))
        })?;

        self.scheduler.start().await.map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to start scheduler: {}", e))
        })?;

        info!("Scheduler started successfully");
        Ok(())
    }

    /// Attach a config watcher to the scheduler
    /// The config will be reloaded at the start of the next scheduled batch
    pub fn attach_config_watcher(&mut self, watcher: ConfigWatcher) {
        self.config_watcher = Some(watcher);
    }

    /// Reload configuration (called when config file changes)
    pub async fn reload_config(&self, new_config: Config) -> Result<()> {
        info!("Reloading configuration");

        let mut config_guard = self.config.write().await;
        *config_guard = new_config;

        debug!("Configuration reloaded, will take effect on next scheduled run");
        Ok(())
    }

    /// Gracefully shutdown the scheduler
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down scheduler");

        self.scheduler.shutdown().await.map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to shutdown scheduler: {}", e))
        })?;

        info!("Scheduler shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn create_test_config() -> Config {
        Config {
            scheduler: common::models::config::SchedulerConfig {
                crontab: "0 0 * * * *".to_string(), // Every hour
            },
            src: common::models::config::SourceConfig {
                source_dir: PathBuf::from("/tmp"),
                include_patterns: None,
                exclude_patterns: None,
            },
            credential: common::models::config::CredentialConfig {
                account: "testaccount".to_string(),
                username: "test".to_string(),
                password: "test".to_string(),
                device: None,
            },
            api: common::models::config::ApiConfig {
                base_url: "https://api.example.com".to_string(),
                https_only: true,
            },
            encoding: common::models::config::EncodingConfig {
                dbf_encoding: "CP866".to_string(),
            },
        }
    }

    fn create_test_config_path() -> PathBuf {
        // Use a dummy path for tests - config reload will fail but fallback to cached
        PathBuf::from("/tmp/test_config.toml")
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = create_test_config();
        let config_path = create_test_config_path();
        let scheduler = BatchScheduler::new(config, config_path).await;
        assert!(scheduler.is_ok());
    }

    #[tokio::test]
    async fn test_scheduler_invalid_crontab() {
        let mut config = create_test_config();
        config.scheduler.crontab = "invalid cron".to_string();
        let config_path = create_test_config_path();

        let scheduler = BatchScheduler::new(config, config_path).await;
        // Scheduler creation should succeed, but starting will fail
        assert!(scheduler.is_ok());

        let mut scheduler = scheduler.unwrap();
        let start_result = scheduler.start().await;
        assert!(start_result.is_err());
    }

    #[tokio::test]
    async fn test_config_reload() {
        let config = create_test_config();
        let config_path = create_test_config_path();
        let scheduler = BatchScheduler::new(config, config_path).await.unwrap();

        let mut new_config = create_test_config();
        new_config.scheduler.crontab = "0 30 * * * *".to_string();

        let result = scheduler.reload_config(new_config.clone()).await;
        assert!(result.is_ok());

        // Verify config was updated
        let current_config = scheduler.config.read().await;
        assert_eq!(current_config.scheduler.crontab, "0 30 * * * *");
    }

    // T016 [US1] Additional tests for cron expression parsing
    #[tokio::test]
    async fn test_valid_cron_expressions() {
        // Test various valid cron expressions
        let valid_crons = vec![
            "*/5 * * * *",    // Every 5 minutes
            "0 */2 * * *",    // Every 2 hours
            "30 4 * * *",     // 4:30 AM daily
            "0 0 * * 0",      // Midnight on Sundays
            "0 9-17 * * 1-5", // 9am-5pm weekdays
        ];

        for cron in valid_crons {
            let mut config = create_test_config();
            config.scheduler.crontab = cron.to_string();
            let config_path = create_test_config_path();

            let scheduler = BatchScheduler::new(config, config_path).await;
            assert!(
                scheduler.is_ok(),
                "Failed to create scheduler with cron: {}",
                cron
            );

            let mut scheduler = scheduler.unwrap();
            let start_result = scheduler.start().await;
            assert!(
                start_result.is_ok(),
                "Failed to start scheduler with cron: {}",
                cron
            );

            // Clean up
            let _ = scheduler.shutdown().await;
        }
    }

    #[tokio::test]
    async fn test_invalid_cron_expressions() {
        // Test various invalid cron expressions
        let invalid_crons = vec![
            "invalid",
            "* * *",      // Too few fields
            "60 * * * *", // Invalid minute (>59)
            "* 25 * * *", // Invalid hour (>23)
            "* * 32 * *", // Invalid day (>31)
            "* * * 13 *", // Invalid month (>12)
            "* * * * 8",  // Invalid weekday (>7)
        ];

        for cron in invalid_crons {
            let mut config = create_test_config();
            config.scheduler.crontab = cron.to_string();
            let config_path = create_test_config_path();

            let scheduler = BatchScheduler::new(config, config_path).await;
            assert!(
                scheduler.is_ok(),
                "Scheduler creation should succeed for: {}",
                cron
            );

            let mut scheduler = scheduler.unwrap();
            let start_result = scheduler.start().await;
            assert!(
                start_result.is_err(),
                "Start should fail for invalid cron: {}",
                cron
            );
        }
    }

    // T018 [US1] Test batch lock initialization
    #[tokio::test]
    async fn test_batch_lock_initialized_false() {
        let config = create_test_config();
        let config_path = create_test_config_path();
        let scheduler = BatchScheduler::new(config, config_path).await.unwrap();

        // The batch lock should be initialized to false (not running)
        let is_running = scheduler.batch_lock.lock().await;
        assert!(
            !*is_running,
            "Batch lock should be initialized to false (not running)"
        );
    }

    #[tokio::test]
    async fn test_cron_5field_to_6field_conversion() {
        // Verify the 5-field to 6-field conversion logic
        let cron_5field = "*/5 * * * *";
        let cron_6field = if cron_5field.split_whitespace().count() == 5 {
            format!("0 {}", cron_5field)
        } else {
            cron_5field.to_string()
        };

        assert_eq!(cron_6field, "0 */5 * * * *");
        assert_eq!(cron_6field.split_whitespace().count(), 6);
    }
}
