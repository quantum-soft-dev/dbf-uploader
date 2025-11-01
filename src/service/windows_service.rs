// Windows Service implementation
#[cfg(windows)]
use crate::error::Result;
#[cfg(windows)]
use crate::models::Config;
#[cfg(windows)]
use crate::service::BatchScheduler;
#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::{Arc, Mutex};
#[cfg(windows)]
use std::time::Duration;
#[cfg(windows)]
use tracing::{error, info};
#[cfg(windows)]
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
#[cfg(windows)]
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
#[cfg(windows)]
use windows_service::{define_windows_service, service_dispatcher};

#[cfg(windows)]
const SERVICE_NAME: &str = "data-exporter";

#[cfg(windows)]
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

#[cfg(windows)]
define_windows_service!(ffi_service_main, service_main);

/// Run the Windows service
#[cfg(windows)]
pub fn run_service() -> windows_service::Result<()> {
    // Start the Windows service dispatcher
    // All logging is handled via tracing in service_main -> run_service_impl
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

#[cfg(windows)]
fn service_main(_arguments: Vec<OsString>) {
    // Service entry point - errors are logged via tracing in run_service_impl
    if let Err(e) = run_service_impl() {
        // Log to Windows Event Log or stderr (tracing may not be initialized if error is early)
        eprintln!("Service error: {}", e);
    }
}

#[cfg(windows)]
fn run_service_impl() -> Result<()> {
    use tracing_appender::rolling::{RollingFileAppender, Rotation};

    // Initialize tracing with daily log rotation
    // Logs will be in C:\Program Files\data-exporter\logs\
    // Old logs will be automatically renamed with date suffix
    let log_dir = PathBuf::from(r"C:\Program Files\data-exporter\logs");

    // Create logs directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        let _ = std::fs::write(
            r"C:\Windows\Temp\data_exporter_log_error.txt",
            format!("Failed to create log directory: {}\n", e),
        );
        std::process::exit(1);
    }

    // Create a daily rotating file appender
    // - Rotation::DAILY: Creates new log file at midnight
    // - Old files are named like: service.log.2025-10-31
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "service.log");

    // Setup tracing subscriber with rotating file output
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .init();

    info!("========================================");
    info!("Data Exporter Service Starting");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("========================================");

    // Load configuration
    let config_path = PathBuf::from(r"C:\Program Files\data-exporter\config.toml");
    info!("Loading config from: {}", config_path.display());
    let config = match Config::from_file(&config_path) {
        Ok(cfg) => {
            info!("Config loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load config: {}", e);
            return Err(crate::error::ProcessingError::ConfigurationError(format!(
                "Failed to load config: {}",
                e
            )));
        }
    };

    // Create scheduler
    let scheduler = Arc::new(Mutex::new(None::<BatchScheduler>));
    let scheduler_clone = scheduler.clone();

    // Define event handler
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                info!("Received stop signal");
                // Stop the scheduler
                if let Ok(mut sched) = scheduler_clone.lock() {
                    *sched = None;
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register service control handler
    info!("Registering service control handler");
    let status_handle = match service_control_handler::register(SERVICE_NAME, event_handler) {
        Ok(handle) => {
            info!("Service control handler registered");
            handle
        }
        Err(e) => {
            error!("Failed to register service control handler: {}", e);
            return Err(crate::error::ProcessingError::ConfigurationError(format!(
                "Failed to register service control handler: {}",
                e
            )));
        }
    };

    // Tell Windows we're running
    info!("Setting service status to RUNNING");
    if let Err(e) = status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    }) {
        error!("Failed to set service status to running: {}", e);
        return Err(crate::error::ProcessingError::ConfigurationError(format!(
            "Failed to set service status to running: {}",
            e
        )));
    }
    info!("Service status set to RUNNING");

    info!("Service is running");

    // Create and start the scheduler
    info!("Creating tokio runtime");
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => {
            info!("Tokio runtime created");
            rt
        }
        Err(e) => {
            error!("Failed to create tokio runtime: {}", e);
            return Err(crate::error::ProcessingError::ConfigurationError(format!(
                "Failed to create tokio runtime: {}",
                e
            )));
        }
    };

    info!("Creating BatchScheduler");

    runtime.block_on(async {
        match BatchScheduler::new(config).await {
            Ok(mut sched) => {
                info!("Scheduler created successfully");

                // Start the scheduler
                match sched.start().await {
                    Ok(_) => {
                        info!("Scheduler started successfully");
                        *scheduler.lock().unwrap() = Some(sched);

                        // Keep the service running
                        loop {
                            tokio::time::sleep(Duration::from_secs(1)).await;

                            // Check if we should stop
                            if scheduler.lock().unwrap().is_none() {
                                info!("Scheduler stopped, exiting service");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to start scheduler: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to create scheduler: {}", e);
            }
        }
    });

    // Tell Windows we're stopping
    status_handle
        .set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
        .map_err(|e| {
            crate::error::ProcessingError::ConfigurationError(format!(
                "Failed to set service status to stopped: {}",
                e
            ))
        })?;

    info!("Service stopped");
    Ok(())
}
