// Entry point for data_exporter service
// Handles both Windows Service mode and CLI commands

use clap::Parser;
use data_exporter::cli::{authorize_device, install, uninstall, Cli, Commands, InstallParams};
use tracing_subscriber::{fmt, EnvFilter};

#[cfg(windows)]
use data_exporter::service::run_service;

// When running as a Windows service, use this entry point
#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                eprintln!(
                    "Error: This executable must be run with arguments or as a Windows Service."
                );
                eprintln!("Windows Service Error: {}", e);
                eprintln!();
                eprintln!("Usage:");
                eprintln!();
                eprintln!("  Traditional auth:");
                eprintln!("    data_exporter.exe install --account <ACCOUNT> --username <USER> --password <PASS> \\");
                eprintln!("      --source-dir <DIR> [--crontab <CRON>] [--api-url <URL>] [--encoding <ENC>]");
                eprintln!();
                eprintln!("  Device flow:");
                eprintln!("    data_exporter.exe install --use-device-flow --site-name <SITE> \\");
                eprintln!("      --source-dir <DIR> [--site-description <DESC>] [--crontab <CRON>] [--api-url <URL>]");
                eprintln!();
                eprintln!("  Uninstall:");
                eprintln!("    data_exporter.exe uninstall");
                eprintln!();
                eprintln!("  Re-authorize:");
                eprintln!("    data_exporter.exe authorize-device --site-name <SITE> [--site-description <DESC>]");
                eprintln!();
                eprintln!("  Run as service:");
                eprintln!("    Start-Service -Name \"data-exporter\"");
                eprintln!();
                eprintln!("Optional parameters:");
                eprintln!("  --crontab <CRON>      Schedule (default: \"*/5 * * * *\" - every 5 minutes)");
                eprintln!("  --api-url <URL>       API base URL (default: https://dev.dfm.bitbi.io)");
                eprintln!("  --encoding <ENC>      DBF encoding (default: CP866)");
                eprintln!("  --no-https            Allow HTTP connections (insecure, testing only)");
                eprintln!();
                eprintln!("For detailed help, run: data_exporter.exe --help");
                std::process::exit(1);
            }
        }
    } else {
        // Has arguments - run as CLI
        tokio::runtime::Runtime::new()?.block_on(async { run_cli().await })
    }
}

#[cfg(not(windows))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_cli().await
}

async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize simple logging for CLI (no technical details)
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(env_filter)
        .with_target(false) // Hide module paths
        .with_thread_ids(false) // Hide thread IDs
        .with_file(false) // Hide file names
        .with_line_number(false) // Hide line numbers
        .with_level(true) // Keep log level (INFO, ERROR, etc.)
        .with_ansi(true) // Keep colors for better readability
        .without_time() // Hide timestamps
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    let result = match cli.command {
        Commands::Install {
            use_device_flow,
            site_name,
            site_description,
            account,
            username,
            password,
            source_dir,
            crontab,
            api_url,
            encoding,
            no_https,
        } => {
            let https_only = !no_https; // Invert the flag
            install(InstallParams {
                use_device_flow,
                site_name,
                site_description,
                account,
                username,
                password,
                source_dir,
                crontab,
                api_url,
                encoding,
                https_only,
            })
            .await
        }
        Commands::Uninstall => uninstall().await,
        Commands::AuthorizeDevice {
            site_name,
            site_description,
            api_url,
        } => {
            use data_exporter::cli::authorize_device::AuthorizeDeviceParams;
            authorize_device(AuthorizeDeviceParams {
                site_name,
                site_description,
                api_url,
            })
            .await
        }
    };

    // Handle result
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
