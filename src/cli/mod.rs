// CLI module for data_exporter v2.0
pub mod install;
pub mod migrate;
pub mod run;
pub mod uninstall;
pub mod wizard;

pub use install::install;
pub use migrate::migrate;
pub use run::run_once;
pub use uninstall::uninstall;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "data_exporter")]
#[command(about = "Data Exporter Service v2.0 - DBF to CSV/gzip uploader with batch protocol")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install the service (interactive wizard for v2.0)
    ///
    /// Runs an interactive installation wizard that guides you through:
    /// - Site credentials setup (domain + client_secret)
    /// - Source directory configuration
    /// - Middleware API URL
    /// - Schedule (cron format)
    /// - Encoding settings
    ///
    /// If an existing v1.0 installation is detected, you will be prompted
    /// to use the 'migrate' command instead.
    Install,

    /// Migrate from v1.0 to v2.0
    ///
    /// Detects existing v1.0 configuration and guides you through upgrading to v2.0:
    /// - Reads existing config.toml (v1.0)
    /// - Prompts for new site credentials (domain + client_secret)
    /// - Migrates all other settings automatically
    /// - Creates backup of old configuration
    /// - Restarts Windows service with new configuration
    Migrate,

    /// Uninstall the service
    ///
    /// Removes the Windows service and cleans up installation files.
    /// Configuration files are preserved for potential reinstallation.
    Uninstall,

    /// Run a single batch upload using the current configuration.
    ///
    /// Loads `config.toml`, validates settings, and executes one batch cycle
    /// against the middleware API.
    Run,
}
