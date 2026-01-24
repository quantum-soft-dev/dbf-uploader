// Integration tests for scheduler triggering batch processing
// T017 [US1] Write integration test for scheduler triggering batch

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Test helper: Create a test configuration for scheduler tests
fn create_test_scheduler_config() -> common::models::Config {
    use common::models::config::*;

    common::models::Config {
        scheduler: SchedulerConfig {
            // Schedule to run every second for testing
            crontab: "* * * * * *".to_string(),
        },
        src: SourceConfig {
            source_dir: PathBuf::from(std::env::temp_dir()),
            include_patterns: None,
            exclude_patterns: None,
        },
        credential: CredentialConfig {
            account: "testaccount".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            device: None,
        },
        api: ApiConfig {
            base_url: "https://api.test.com".to_string(),
            https_only: true,
        },
        encoding: EncodingConfig {
            dbf_encoding: "CP866".to_string(),
        },
    }
}

/// T017: Test that scheduler triggers batch processing on schedule
/// This is a unit-style integration test that verifies the cron job fires
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_scheduler_triggers_on_cron_schedule() {
    // Counter to track how many times the job was triggered
    let trigger_count = Arc::new(AtomicUsize::new(0));
    let trigger_count_clone = Arc::clone(&trigger_count);

    // Create scheduler
    let scheduler = JobScheduler::new().await.expect("Failed to create scheduler");

    // Create a job that runs every second (for test purposes)
    // tokio-cron-scheduler uses 6-field cron: "sec min hour day month weekday"
    let job = Job::new_async("* * * * * *", move |_uuid, _lock| {
        let counter = Arc::clone(&trigger_count_clone);
        Box::pin(async move {
            counter.fetch_add(1, Ordering::SeqCst);
        })
    })
    .expect("Failed to create job");

    scheduler.add(job).await.expect("Failed to add job");
    scheduler.start().await.expect("Failed to start scheduler");

    // Wait for at least 2 triggers (with some margin)
    sleep(Duration::from_secs(3)).await;

    // Verify the job was triggered at least twice
    let count = trigger_count.load(Ordering::SeqCst);
    assert!(count >= 2, "Expected at least 2 triggers, got {}", count);

    scheduler.shutdown().await.expect("Failed to shutdown scheduler");
}

/// T018: Test that batch lock prevents concurrent execution
/// When a batch is running, subsequent scheduled triggers should be skipped
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_batch_lock_prevents_concurrent_execution() {
    let batch_lock = Arc::new(Mutex::new(false));
    let execution_count = Arc::new(AtomicUsize::new(0));
    let skip_count = Arc::new(AtomicUsize::new(0));

    let batch_lock_clone = Arc::clone(&batch_lock);
    let execution_count_clone = Arc::clone(&execution_count);
    let skip_count_clone = Arc::clone(&skip_count);

    // Create scheduler
    let scheduler = JobScheduler::new().await.expect("Failed to create scheduler");

    // Create a job that simulates batch processing with locking
    let job = Job::new_async("* * * * * *", move |_uuid, _lock| {
        let batch_lock = Arc::clone(&batch_lock_clone);
        let execution_count = Arc::clone(&execution_count_clone);
        let skip_count = Arc::clone(&skip_count_clone);

        Box::pin(async move {
            // Try to acquire lock
            let mut is_running = batch_lock.lock().await;

            if *is_running {
                // Skip this execution - batch already running
                skip_count.fetch_add(1, Ordering::SeqCst);
                return;
            }

            // Mark as running
            *is_running = true;
            drop(is_running);

            // Count execution
            execution_count.fetch_add(1, Ordering::SeqCst);

            // Simulate long-running batch (2 seconds)
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Mark as not running
            let mut is_running = batch_lock.lock().await;
            *is_running = false;
        })
    })
    .expect("Failed to create job");

    scheduler.add(job).await.expect("Failed to add job");
    scheduler.start().await.expect("Failed to start scheduler");

    // Wait for several trigger attempts
    sleep(Duration::from_secs(5)).await;

    let executions = execution_count.load(Ordering::SeqCst);
    let skips = skip_count.load(Ordering::SeqCst);

    // With a 2-second batch and 5-second wait:
    // - First trigger starts batch (1 execution)
    // - Triggers during batch get skipped
    // - After batch completes, more executions may occur
    // We expect at least 1 execution and at least 1 skip
    assert!(executions >= 1, "Expected at least 1 execution, got {}", executions);
    assert!(skips >= 1, "Expected at least 1 skip due to lock, got {}", skips);

    scheduler.shutdown().await.expect("Failed to shutdown scheduler");
}

/// Test that scheduler handles invalid cron expressions gracefully
#[tokio::test]
async fn test_scheduler_rejects_invalid_cron() {
    let scheduler = JobScheduler::new().await.expect("Failed to create scheduler");

    // Try to create a job with invalid cron expression
    let result = Job::new_async("invalid cron expression", |_uuid, _lock| {
        Box::pin(async move {})
    });

    // Should fail to create job with invalid cron
    assert!(result.is_err(), "Expected error for invalid cron expression");
}

/// Test that scheduler can be gracefully shutdown
#[tokio::test]
async fn test_scheduler_graceful_shutdown() {
    let shutdown_complete = Arc::new(AtomicBool::new(false));
    let shutdown_complete_clone = Arc::clone(&shutdown_complete);

    let scheduler = JobScheduler::new().await.expect("Failed to create scheduler");

    let job = Job::new_async("* * * * * *", move |_uuid, _lock| {
        Box::pin(async move {
            // Short task
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
    })
    .expect("Failed to create job");

    scheduler.add(job).await.expect("Failed to add job");
    scheduler.start().await.expect("Failed to start scheduler");

    // Let it run briefly
    sleep(Duration::from_secs(1)).await;

    // Shutdown should complete without error
    let shutdown_result = scheduler.shutdown().await;
    assert!(shutdown_result.is_ok(), "Scheduler shutdown should succeed");

    shutdown_complete_clone.store(true, Ordering::SeqCst);
    assert!(shutdown_complete.load(Ordering::SeqCst));
}

/// Test 5-field to 6-field cron conversion
#[test]
fn test_cron_expression_conversion() {
    // Standard 5-field cron: "min hour day month weekday"
    let cron_5field = "*/5 * * * *";

    // Convert to 6-field: "sec min hour day month weekday"
    let cron_6field = if cron_5field.split_whitespace().count() == 5 {
        format!("0 {}", cron_5field)
    } else {
        cron_5field.to_string()
    };

    assert_eq!(cron_6field, "0 */5 * * * *");

    // Already 6-field should remain unchanged
    let cron_already_6 = "0 */5 * * * *";
    let result = if cron_already_6.split_whitespace().count() == 5 {
        format!("0 {}", cron_already_6)
    } else {
        cron_already_6.to_string()
    };

    assert_eq!(result, "0 */5 * * * *");
}
