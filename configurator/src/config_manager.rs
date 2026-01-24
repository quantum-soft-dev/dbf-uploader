// Configuration management for Configurator
use anyhow::{Context, Result};
use common::models::Config;
use std::fs;
use std::path::Path;

pub struct ConfigManager {
    config_path: std::path::PathBuf,
}

impl ConfigManager {
    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {
        Self {
            config_path: config_path.as_ref().to_path_buf(),
        }
    }

    /// Load configuration from file, or return default if file doesn't exist
    pub fn load(&self) -> Result<Config> {
        if !self.config_path.exists() {
            return Ok(Self::default_config());
        }

        let contents =
            fs::read_to_string(&self.config_path).context("Failed to read config file")?;

        let config: Config = toml::from_str(&contents).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, config: &Config) -> Result<()> {
        let toml_string = toml::to_string_pretty(config).context("Failed to serialize config")?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        fs::write(&self.config_path, toml_string).context("Failed to write config file")?;

        Ok(())
    }

    /// Check if config file exists
    pub fn exists(&self) -> bool {
        self.config_path.exists()
    }

    /// Get default configuration
    pub fn default_config() -> Config {
        use common::models::{
            ApiConfig, CredentialConfig, EncodingConfig, SchedulerConfig, SourceConfig,
        };

        Config {
            scheduler: SchedulerConfig {
                crontab: "0 0 8,12,16,18 * * *".to_string(),
            },
            src: SourceConfig {
                source_dir: std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("C:\\data")),
                include_patterns: None,
                exclude_patterns: None,
            },
            credential: CredentialConfig {
                account: String::new(),
                username: String::new(),
                password: String::new(),
                device: None,
            },
            api: ApiConfig {
                base_url: "https://".to_string(),
                https_only: true,
            },
            encoding: EncodingConfig {
                dbf_encoding: "CP866".to_string(),
            },
        }
    }
}
