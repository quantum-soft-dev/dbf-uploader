// CLI module for data_exporter
pub mod authorize_device;
pub mod install;
pub mod uninstall;

pub use authorize_device::authorize_device;
pub use install::{install, InstallParams};
pub use uninstall::uninstall;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "data_exporter")]
#[command(about = "Data Exporter Service - DBF to CSV/gzip uploader")]
#[command(long_version = crate::version::Version::detailed())]
#[command(version = crate::version::Version::get())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install the service
    Install {
        /// Use Device Authorization Flow (RFC 8628) instead of traditional credentials
        #[arg(long)]
        use_device_flow: bool,

        /// Site name (required if use_device_flow, 1-100 chars, alphanumeric + hyphens)
        #[arg(long, required_if_eq("use_device_flow", "true"))]
        site_name: Option<String>,

        /// Site description (optional, max 500 chars)
        #[arg(long)]
        site_description: Option<String>,

        /// Account identifier for uniqueness (required if NOT using device flow)
        #[arg(long, required_unless_present = "use_device_flow")]
        account: Option<String>,

        /// Username for API authentication (required if NOT using device flow)
        #[arg(long, required_unless_present = "use_device_flow")]
        username: Option<String>,

        /// Password for API authentication (required if NOT using device flow)
        #[arg(long, required_unless_present = "use_device_flow")]
        password: Option<String>,

        /// Source directory containing DBF files
        #[arg(long)]
        source_dir: String,

        /// Cron schedule (e.g., "*/5 * * * *" for every 5 minutes)
        #[arg(long, default_value = "*/5 * * * *")]
        crontab: String,

        /// API base URL
        #[arg(long, default_value = "https://dev.dfm.bitbi.io")]
        api_url: String,

        /// DBF encoding (CP866, Windows1251, Windows1255, ISO8859-8, UTF8)
        #[arg(long, default_value = "CP866")]
        encoding: String,

        /// Disable HTTPS-only connections (insecure, for local testing only)
        /// By default, HTTPS is enforced. Use this flag to allow HTTP connections.
        #[arg(long, default_value = "false")]
        no_https: bool,
    },

    /// Uninstall the service
    Uninstall,

    /// Re-authorize device to connect to a different site (Device Authorization Grant RFC 8628)
    AuthorizeDevice {
        /// Site name (1-100 chars, alphanumeric + hyphens)
        #[arg(long)]
        site_name: String,

        /// Site description (optional, max 500 chars)
        #[arg(long)]
        site_description: Option<String>,

        /// API base URL (optional, reads from existing config if not provided)
        #[arg(long)]
        api_url: Option<String>,
    },
}
