// Cron scheduler module
// NOTE: This module is commented out as it uses v1 batch protocol
// It will be completely replaced in Phase 3 with new BatchManager-based implementation

/*
use crate::auth::TokenManager;
use crate::config::ConfigWatcher;
use crate::error::{ProcessingError, Result};
use crate::models::Config;
use crate::processor::run_batch;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info};

pub struct BatchScheduler {
    scheduler: JobScheduler,
    config: Arc<RwLock<Config>>,
    token_manager: Arc<TokenManager>,
    config_watcher: Option<ConfigWatcher>,
}

impl BatchScheduler {
    /// Create a new scheduler with the given configuration
    pub async fn new(config: Config) -> Result<Self> {
        let scheduler = JobScheduler::new().await.map_err(|e| {
            ProcessingError::ConfigurationError(format!("Failed to create scheduler: {}", e))
        })?;

        let token_manager = Arc::new(TokenManager::new(&config)?);
        let config_arc = Arc::new(RwLock::new(config));

        Ok(Self {
            scheduler,
            config: config_arc,
            token_manager,
            config_watcher: None,
        })
    }

    /// Start the scheduler with cron expression from config
    pub async fn start(&mut self) -> Result<()> {
        let config = self.config.read().await.clone();
        let crontab = config.scheduler.crontab.clone();

        info!(crontab = %crontab, "Starting scheduler");

        // Create the batch processing job
        let config_arc = Arc::clone(&self.config);
        let token_manager = Arc::clone(&self.token_manager);

        let job = Job::new_async(crontab.as_str(), move |_uuid, _l| {
            let config_arc = Arc::clone(&config_arc);
            let token_manager = Arc::clone(&token_manager);

            Box::pin(async move {
                // Reload config at start of each batch (respects config file changes)
                let current_config = {
                    let config_guard = config_arc.read().await;
                    config_guard.clone()
                };

                info!("Scheduled batch starting");

                match run_batch(current_config, token_manager).await {
                    Ok(batch) => {
                        info!(
                            batch_id = %batch.batch_id,
                            processed = batch.processed_count,
                            failed = batch.failed_count,
                            "Scheduled batch completed successfully"
                        );
                    }
                    Err(e) => {
                        error!(error = %e, "Scheduled batch failed");
                    }
                }
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

    fn create_test_config() -> Config {
        Config {
            scheduler: crate::models::config::SchedulerConfig {
                crontab: "0 0 * * * *".to_string(), // Every hour
            },
            src: crate::models::config::SourceConfig {
                source_dir: PathBuf::from("/tmp"),
            },
            credential: crate::models::config::CredentialConfig {
                username: "test".to_string(),
                password: "test".to_string(),
            },
            api: crate::models::config::ApiConfig {
                base_url: "https://api.example.com".to_string(),
            },
            encoding: crate::models::config::EncodingConfig {
                dbf_encoding: "CP866".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = create_test_config();
        let scheduler = BatchScheduler::new(config).await;
        assert!(scheduler.is_ok());
    }

    #[tokio::test]
    async fn test_scheduler_invalid_crontab() {
        let mut config = create_test_config();
        config.scheduler.crontab = "invalid cron".to_string();

        let scheduler = BatchScheduler::new(config).await;
        // Scheduler creation should succeed, but starting will fail
        assert!(scheduler.is_ok());

        let mut scheduler = scheduler.unwrap();
        let start_result = scheduler.start().await;
        assert!(start_result.is_err());
    }

    #[tokio::test]
    async fn test_config_reload() {
        let config = create_test_config();
        let scheduler = BatchScheduler::new(config).await.unwrap();

        let mut new_config = create_test_config();
        new_config.scheduler.crontab = "0 30 * * * *".to_string();

        let result = scheduler.reload_config(new_config.clone()).await;
        assert!(result.is_ok());

        // Verify config was updated
        let current_config = scheduler.config.read().await;
        assert_eq!(current_config.scheduler.crontab, "0 30 * * * *");
    }
}
*/
