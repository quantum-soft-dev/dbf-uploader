// CLI entry point for data_exporter service v2.0
// Handles install, migrate, and uninstall commands

use clap::Parser;
use data_exporter::cli::{install, migrate, uninstall, Cli, Commands};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() {
    // Initialize structured logging with tracing
    // Set log level from RUST_LOG environment variable, default to INFO
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Data Exporter Service v2.0 starting");

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    let result = match cli.command {
        Commands::Install => {
            // Interactive installation wizard for fresh installs
            install().await
        }
        Commands::Migrate => {
            // Migration wizard for v1.0 → v2.0 upgrades
            migrate().await
        }
        Commands::Uninstall => {
            // Uninstall service and cleanup
            uninstall().await
        }
    };

    // Handle result
    match result {
        Ok(_) => {
            tracing::info!("Command completed successfully");
            std::process::exit(0);
        }
        Err(e) => {
            tracing::error!(error = %e, "Command failed");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
