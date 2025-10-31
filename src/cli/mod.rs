// CLI module for data_exporter
pub mod install;
pub mod uninstall;

pub use install::install;
pub use uninstall::uninstall;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "data_exporter")]
#[command(about = "Data Exporter Service - DBF to CSV/gzip uploader")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install the service
    Install {
        /// Account identifier for uniqueness
        #[arg(long)]
        account: String,

        /// Username for API authentication
        #[arg(long)]
        username: String,

        /// Password for API authentication
        #[arg(long)]
        password: String,

        /// Source directory containing DBF files
        #[arg(long)]
        source_dir: String,

        /// Cron schedule (e.g., "*/5 * * * *" for every 5 minutes)
        #[arg(long, default_value = "*/5 * * * *")]
        crontab: String,

        /// API base URL
        #[arg(long, default_value = "https://api.example.com")]
        api_url: String,

        /// DBF encoding (CP866, Windows1251, UTF8)
        #[arg(long, default_value = "CP866")]
        encoding: String,

        /// Disable HTTPS-only connections (insecure, for local testing only)
        /// By default, HTTPS is enforced. Use this flag to allow HTTP connections.
        #[arg(long, default_value = "false")]
        no_https: bool,
    },

    /// Uninstall the service
    Uninstall,
}
