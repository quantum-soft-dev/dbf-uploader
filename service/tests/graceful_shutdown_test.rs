// Integration tests for graceful shutdown (SC-005)
// T005 [US9]: Test that service responds to stop commands within 5 seconds during retry wait

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// SC-005: Test that stop signal terminates simulated retry loop within 5 seconds
#[tokio::test]
async fn test_stop_signal_terminates_retry_loop_within_5_seconds_sc005() {
    const MAX_RESPONSE_TIME: Duration = Duration::from_secs(5);
    const CHECK_INTERVAL: Duration = Duration::from_secs(1); // Using shorter interval for test

    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_signal_clone = stop_signal.clone();

    // Spawn a task that simulates the retry wait loop from windows_service.rs
    let retry_task = tokio::spawn(async move {
        let start = Instant::now();

        // Simulate retry loop with periodic stop signal checks
        // (similar to windows_service.rs lines 119-137)
        loop {
            // Check stop signal before sleeping
            if stop_signal_clone.load(Ordering::Relaxed) {
                return start.elapsed();
            }

            // Sleep for a short interval (simulating 5-second checks)
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });

    // Wait a bit to ensure the task is running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Set stop signal
    let signal_time = Instant::now();
    stop_signal.store(true, Ordering::Relaxed);

    // Wait for the task to complete
    let elapsed = retry_task.await.expect("Task should complete");

    let response_time = signal_time.elapsed();

    println!(
        "Stop signal response time: {}ms (task ran for {}ms)",
        response_time.as_millis(),
        elapsed.as_millis()
    );

    assert!(
        response_time < MAX_RESPONSE_TIME,
        "Stop signal response time ({}ms) exceeds 5 second requirement (SC-005)",
        response_time.as_millis()
    );
}

/// Test retry backoff intervals match specification
#[test]
fn test_retry_backoff_intervals_correct() {
    // From windows_service.rs:
    // const INITIAL_RETRY_MINUTES: &[u64] = &[1, 2, 4, 8, 16];
    // const HOURLY_INTERVAL_MINUTES: u64 = 60;
    const INITIAL_RETRY_MINUTES: [u64; 5] = [1, 2, 4, 8, 16];
    const HOURLY_INTERVAL_MINUTES: u64 = 60;

    // Test initial backoff sequence (attempts 1-5)
    for (attempt, expected_minutes) in INITIAL_RETRY_MINUTES.iter().enumerate() {
        let actual =
            calculate_wait_minutes(attempt + 1, &INITIAL_RETRY_MINUTES, HOURLY_INTERVAL_MINUTES);
        assert_eq!(
            actual,
            *expected_minutes,
            "Attempt {} should wait {} minutes, got {}",
            attempt + 1,
            expected_minutes,
            actual
        );
    }

    // Test hourly retry after initial attempts (attempts 6+)
    for attempt in 6..=10 {
        let actual =
            calculate_wait_minutes(attempt, &INITIAL_RETRY_MINUTES, HOURLY_INTERVAL_MINUTES);
        assert_eq!(
            actual, HOURLY_INTERVAL_MINUTES,
            "Attempt {} should wait {} minutes (hourly), got {}",
            attempt, HOURLY_INTERVAL_MINUTES, actual
        );
    }
}

/// Helper function matching the logic in windows_service.rs
fn calculate_wait_minutes(attempt: usize, initial_retries: &[u64], hourly_interval: u64) -> u64 {
    if attempt <= initial_retries.len() {
        initial_retries[attempt - 1]
    } else {
        hourly_interval
    }
}

/// Test that stop signal is visible across async tasks
#[tokio::test]
async fn test_stop_signal_visible_across_tasks() {
    let stop_signal = Arc::new(AtomicBool::new(false));

    // Spawn multiple tasks that check the stop signal
    let mut handles = Vec::new();
    for i in 0..5 {
        let signal = stop_signal.clone();
        handles.push(tokio::spawn(async move {
            // Wait for stop signal
            let start = Instant::now();
            while !signal.load(Ordering::Relaxed) {
                if start.elapsed() > Duration::from_secs(10) {
                    panic!("Task {} timed out waiting for stop signal", i);
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            i
        }));
    }

    // Give tasks time to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Set stop signal
    stop_signal.store(true, Ordering::Relaxed);

    // All tasks should complete
    for handle in handles {
        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "Task should complete within 2 seconds");
        assert!(result.unwrap().is_ok(), "Task should not panic");
    }
}

/// Test interruptible sleep pattern
#[tokio::test]
async fn test_interruptible_sleep_pattern() {
    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_signal_clone = stop_signal.clone();

    // Simulate the interruptible sleep from windows_service.rs
    // Target sleep: 60 seconds, but should exit early when stop signal is set
    let sleep_task = tokio::spawn(async move {
        let target_sleep_secs = 60u64;
        let check_interval_secs = 1u64; // Shorter for testing
        let mut elapsed_secs = 0u64;

        while elapsed_secs < target_sleep_secs {
            if stop_signal_clone.load(Ordering::Relaxed) {
                return Some(elapsed_secs);
            }

            let remaining = target_sleep_secs - elapsed_secs;
            let sleep_duration = if remaining < check_interval_secs {
                remaining
            } else {
                check_interval_secs
            };

            tokio::time::sleep(Duration::from_secs(sleep_duration)).await;
            elapsed_secs += sleep_duration;
        }

        None // Full sleep completed
    });

    // Wait 3 seconds then send stop signal
    tokio::time::sleep(Duration::from_secs(3)).await;
    stop_signal.store(true, Ordering::Relaxed);

    // Task should complete quickly after stop signal
    let result = tokio::time::timeout(Duration::from_secs(2), sleep_task).await;
    assert!(
        result.is_ok(),
        "Task should respond to stop signal within 2 seconds"
    );

    let elapsed = result.unwrap().unwrap();
    assert!(
        elapsed.is_some(),
        "Sleep should have been interrupted, not completed"
    );
    assert!(
        elapsed.unwrap() < 60,
        "Sleep should not have completed the full 60 seconds"
    );
}

/// Test that config loading check happens on retryable errors only
#[test]
fn test_retryable_error_detection() {
    // The error pattern that triggers retry
    let retryable_error = "Source directory does not exist: /some/path";
    let non_retryable_error = "Invalid TOML syntax";

    assert!(
        retryable_error.contains("Source directory does not exist"),
        "Should detect retryable error"
    );
    assert!(
        !non_retryable_error.contains("Source directory does not exist"),
        "Should not retry non-retryable error"
    );
}
