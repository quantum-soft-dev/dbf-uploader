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
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
    ServiceType,
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
    use std::io::Write;

    // Try to write to a log file immediately
    // This tests if we even get to run code
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(r"C:\Program Files\data-exporter\pre_service.log")
    {
        Ok(mut file) => {
            let _ = writeln!(file, "[{}] run_service() called", chrono::Utc::now());
            let _ = writeln!(file, "[{}] About to call service_dispatcher::start()", chrono::Utc::now());
        }
        Err(e) => {
            // Can't even write log - try temp dir
            let _ = std::fs::write(
                r"C:\Windows\Temp\data_exporter_log.txt",
                format!("Failed to write to Program Files: {}\n", e),
            );
        }
    }

    let result = service_dispatcher::start(SERVICE_NAME, ffi_service_main);

    // Log the result
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(r"C:\Program Files\data-exporter\pre_service.log")
    {
        let _ = writeln!(file, "[{}] service_dispatcher::start() returned: {:?}", chrono::Utc::now(), result);
    }

    result
}

#[cfg(windows)]
fn service_main(_arguments: Vec<OsString>) {
    let _ = std::fs::write(
        r"C:\Program Files\data-exporter\service_main.txt",
        format!("Service main called at {}\n", chrono::Utc::now()),
    );

    if let Err(e) = run_service_impl() {
        let _ = std::fs::write(
            r"C:\Program Files\data-exporter\service_error.txt",
            format!("Service error at {}: {}\n", chrono::Utc::now(), e),
        );
        error!("Service error: {}", e);
    }
}

#[cfg(windows)]
fn run_service_impl() -> Result<()> {
    use std::fs::OpenOptions;

    // Initialize tracing with file logging
    let log_path = PathBuf::from(r"C:\Program Files\data-exporter\service.log");

    // Create a file appender for the log file
    let _file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .unwrap_or_else(|e| {
            let _ = std::fs::write(
                r"C:\Windows\Temp\data_exporter_log_error.txt",
                format!("Failed to open log file: {}\n", e),
            );
            std::process::exit(1);
        });

    // Setup tracing subscriber with file output
    use tracing_subscriber::fmt::writer::MakeWriter;

    // Create a writer that clones the file handle for each write
    struct FileWriter {
        path: PathBuf,
    }

    impl<'a> MakeWriter<'a> for FileWriter {
        type Writer = std::fs::File;

        fn make_writer(&'a self) -> Self::Writer {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .expect("Failed to open log file")
        }
    }

    let file_writer = FileWriter { path: log_path.clone() };

    tracing_subscriber::fmt()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .init();

    info!("Starting Data Exporter Service");

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
