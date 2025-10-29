// Entry point for data_exporter service
// Handles both Windows Service mode and CLI commands

use clap::Parser;
use data_exporter::cli::{install, uninstall, Cli, Commands};
use tracing_subscriber::{fmt, EnvFilter};

#[cfg(windows)]
use data_exporter::service::run_service;

// When running as a Windows service, use this entry point
#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::OsString;
    use windows_service::service_dispatcher;

    // VERY FIRST THING: Write to log to prove we even start
    let _ = std::fs::write(
        r"C:\Windows\Temp\data_exporter_main_entry.txt",
        format!("main() called at {}\n", chrono::Utc::now()),
    );

    // Check if running as a service or CLI
    let args: Vec<String> = std::env::args().collect();

    let _ = std::fs::write(
        r"C:\Windows\Temp\data_exporter_args.txt",
        format!("Args: {:?}\n", args),
    );

    // Try to dispatch as a service first
    // This will only succeed if actually launched by SCM
    if args.len() == 1 {
        // No arguments - try running as Windows service
        // If this fails (not launched by SCM), we'll fall back to showing help
        match run_service() {
            Ok(_) => {
                // Successfully ran as service
                Ok(())
            }
            Err(e) => {
                // Not running as service - show helpful message
                eprintln!("Error: This executable must be run with arguments or as a Windows Service.");
                eprintln!("Windows Service Error: {}", e);
                eprintln!();
                eprintln!("Usage:");
                eprintln!("  Install service:   data_exporter.exe install --username <USER> --password <PASS> ...");
                eprintln!("  Uninstall service: data_exporter.exe uninstall");
                eprintln!("  Run as service:    Start-Service -Name \"data-exporter\"");
                eprintln!();
                eprintln!("For more options, run: data_exporter.exe --help");
                std::process::exit(1);
            }
        }
    } else {
        // Has arguments - run as CLI
        tokio::runtime::Runtime::new()?.block_on(async {
            run_cli().await
        })
    }
}

#[cfg(not(windows))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_cli().await
}

async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging with tracing
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Data Exporter Service starting");

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    let result = match cli.command {
        Commands::Install {
            username,
            password,
            source_dir,
            crontab,
            api_url,
            encoding,
            no_https,
        } => {
            let https_only = !no_https; // Invert the flag
            install(username, password, source_dir, crontab, api_url, encoding, https_only).await
        }
        Commands::Uninstall => uninstall().await,
    };

    // Handle result
    match result {
        Ok(_) => {
            tracing::info!("Command completed successfully");
            Ok(())
        }
        Err(e) => {
            tracing::error!(error = %e, "Command failed");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
