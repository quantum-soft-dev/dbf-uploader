# Research: Configuration Module

**Feature**: 002-configuration-module
**Date**: 2026-01-24
**Status**: Complete

## Overview

This document consolidates research findings for the Configuration Module implementation, resolving all technical unknowns identified during planning.

---

## 1. TOML Configuration Parsing in Rust

### Question
What is the best approach for parsing TOML configuration files with validation in Rust?

### Decision
Use the `toml` crate (v0.8) with `serde` derive macros for deserialization.

### Rationale
- **Type Safety**: Compile-time checking of configuration structure
- **Default Values**: `#[serde(default)]` and `#[serde(default = "function")]` for optional fields
- **Nested Structures**: Native support for TOML sections as nested structs
- **Error Messages**: Clear parsing errors with line/column information
- **Pretty Print**: `toml::to_string_pretty()` for human-readable output

### Implementation Pattern

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub scheduler: SchedulerConfig,
    pub src: SourceConfig,
    // ... other sections
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub base_url: String,
    #[serde(default = "default_https_only")]
    pub https_only: bool,
}

fn default_https_only() -> bool { true }

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }
}
```

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| Manual parsing | Error-prone, verbose, no type safety |
| JSON | Less human-readable for configuration |
| YAML | More complex syntax, indentation-sensitive |
| `config-rs` crate | Over-engineered for simple TOML files |

### Source
- Context7: `/websites/rs_toml` documentation
- Existing implementation: `common/src/models/config.rs`

---

## 2. File Watching with Debouncing

### Question
How to monitor configuration file changes with debouncing to prevent multiple reloads?

### Decision
Use `notify-debouncer-mini` crate with 2-second debounce period.

### Rationale
- **Debouncing**: Prevents multiple events from editor save operations (temp file creation, rename, etc.)
- **Cross-platform**: Works on Windows, macOS, Linux
- **Lightweight**: Minimal dependencies compared to full `notify` debouncer
- **2-second delay**: Balances responsiveness with preventing duplicate events

### Implementation Pattern

```rust
use notify_debouncer_mini::{new_debouncer, notify, DebounceEventResult, Debouncer};
use notify::RecursiveMode;
use std::time::Duration;

pub struct ConfigWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher>,
    config_path: PathBuf,
    change_receiver: Receiver<()>,
}

impl ConfigWatcher {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let (tx, rx) = channel();

        let mut debouncer = new_debouncer(
            Duration::from_secs(2),  // 2-second debounce
            move |res: DebounceEventResult| {
                if res.is_ok() {
                    let _ = tx.send(());
                }
            }
        )?;

        // CRITICAL: Must call watch() to start monitoring
        let watch_path = config_path.parent().unwrap();
        debouncer.watcher().watch(watch_path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            _debouncer: debouncer,
            config_path,
            change_receiver: rx,
        })
    }
}
```

### Issue Found
The current implementation in `service/src/config/watcher.rs` creates the debouncer but **does not call `watch()` to start monitoring**. This is a critical bug (GAP-001).

### Fix Required
Add after line 66 in `watcher.rs`:
```rust
debouncer.watcher().watch(&watch_path, notify::RecursiveMode::NonRecursive)
    .map_err(|e| ProcessingError::ConfigurationError(
        format!("Failed to watch config directory: {}", e)
    ))?;
```

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| `notify` crate directly | No built-in debouncing |
| `notify-debouncer-full` | Heavier, unnecessary features |
| Polling | Inefficient, higher CPU usage |
| inotify/ReadDirectoryChangesW | Platform-specific, complex |

---

## 3. Glob Pattern Matching

### Question
How to implement case-insensitive glob pattern matching for file filtering?

### Decision
Use `globset` crate with `GlobBuilder::case_insensitive(true)`.

### Rationale
- **Case Insensitive**: Windows file systems are case-insensitive by default
- **Standard Syntax**: Supports `*`, `?`, `[...]` patterns
- **GlobSet**: Compile multiple patterns for efficient matching
- **Error Reporting**: Clear error messages for invalid patterns

### Implementation Pattern

```rust
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

pub struct FileFilter {
    include_set: Option<GlobSet>,
    exclude_set: Option<GlobSet>,
}

impl FileFilter {
    fn build_globset(patterns: &Option<Vec<String>>) -> Result<Option<GlobSet>, String> {
        if let Some(pattern_list) = patterns {
            let mut builder = GlobSetBuilder::new();
            for pattern in pattern_list {
                let glob = GlobBuilder::new(pattern)
                    .case_insensitive(true)  // Windows compatibility
                    .build()
                    .map_err(|e| format!("Invalid pattern '{}': {}", pattern, e))?;
                builder.add(glob);
            }
            Ok(Some(builder.build()?))
        } else {
            Ok(None)
        }
    }

    pub fn should_process(&self, filename: &str) -> bool {
        // 1. Check include (whitelist) - must match if present
        if let Some(ref include) = self.include_set {
            if !include.is_match(filename) {
                return false;
            }
        }

        // 2. Check exclude (blacklist) - must not match if present
        if let Some(ref exclude) = self.exclude_set {
            if exclude.is_match(filename) {
                return false;
            }
        }

        true
    }
}
```

### Filter Logic
1. Include patterns act as whitelist - file MUST match at least one
2. Exclude patterns act as blacklist - file MUST NOT match any
3. Include is evaluated first, then exclude
4. If no patterns configured, all files are processed

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| `glob` crate | No GlobSet for multiple patterns |
| Regex | Over-complicated for file matching |
| Manual wildcard parsing | Error-prone, incomplete |

---

## 4. Cron Expression Scheduling

### Question
How to handle cron expressions for scheduling batch operations?

### Decision
Use `tokio-cron-scheduler` with automatic 5-to-6 field conversion.

### Rationale
- **Async Native**: Built for tokio runtime
- **6-Field Format**: Includes seconds for precise scheduling
- **5-Field Compatibility**: Common cron format converted automatically
- **Timezone Support**: Optional with `chrono-tz` feature

### Cron Format

```
Standard 5-field:  "min hour day month weekday"
6-field (tokio):   "sec min hour day month weekday"

Examples:
  "0 8,12,16,18 * * *"    -> 5-field: 8am, 12pm, 4pm, 6pm
  "0 0 8,12,16,18 * * *"  -> 6-field equivalent
```

### Implementation Pattern

```rust
// Convert 5-field to 6-field (add seconds at beginning)
let crontab_6field = if crontab.split_whitespace().count() == 5 {
    format!("0 {}", crontab)  // Prepend "0" for seconds
} else {
    crontab.clone()
};

let job = Job::new_async(&crontab_6field, |uuid, _| {
    Box::pin(async move {
        // Batch processing
    })
})?;

scheduler.add(job).await?;
scheduler.start().await?;
```

### Documentation Reference
Context7: `/mvniekerk/tokio-cron-scheduler` - shows 6-field format examples

---

## 5. Windows Service Retry Strategy

### Question
How to handle network drive unavailability at service startup?

### Decision
Exponential backoff with eventual hourly retry.

### Rationale
- **Initial Fast Retry**: 1, 2, 4, 8, 16 minutes for quick recovery
- **Hourly After 5 Attempts**: Prevents resource waste during extended outages
- **Specific Error Matching**: Only retry for "Source directory does not exist"
- **Graceful Shutdown**: Check stop signal every 5 seconds during wait

### Retry Schedule

| Attempt | Wait Time | Cumulative |
|---------|-----------|------------|
| 1 | 1 min | 1 min |
| 2 | 2 min | 3 min |
| 3 | 4 min | 7 min |
| 4 | 8 min | 15 min |
| 5 | 16 min | 31 min |
| 6+ | 60 min | hourly |

### Implementation Pattern

```rust
const INITIAL_RETRY_MINUTES: &[u64] = &[1, 2, 4, 8, 16];
const HOURLY_INTERVAL_MINUTES: u64 = 60;

async fn load_config_with_retry(
    config_path: &Path,
    stop_signal: Arc<AtomicBool>,
) -> Result<Config> {
    let mut attempt = 0;

    loop {
        attempt += 1;

        // Check stop signal
        if stop_signal.load(Ordering::Relaxed) {
            return Err("Service stop requested".into());
        }

        match Config::from_file(config_path) {
            Ok(cfg) => return Ok(cfg),
            Err(e) if e.to_string().contains("Source directory does not exist") => {
                let wait_minutes = if attempt <= INITIAL_RETRY_MINUTES.len() {
                    INITIAL_RETRY_MINUTES[attempt - 1]
                } else {
                    HOURLY_INTERVAL_MINUTES
                };

                // Sleep with periodic stop signal checks (every 5 seconds)
                interruptible_sleep(Duration::from_secs(wait_minutes * 60), &stop_signal).await;
            }
            Err(e) => return Err(e),  // Fail immediately for other errors
        }
    }
}
```

---

## 6. Hot-Reload Architecture

### Question
How should configuration changes be applied at runtime?

### Decision
Apply changes at the start of the next batch cycle, not mid-execution.

### Rationale
- **Consistency**: Batch runs with single configuration snapshot
- **Safety**: No race conditions during file processing
- **Simplicity**: No complex mid-batch configuration swapping
- **Failure Handling**: Invalid config keeps previous valid config

### Implementation Pattern

```rust
// In scheduler job execution
let current_config = match Config::from_file(&config_path) {
    Ok(new_config) => {
        // Update shared config for other components
        *config_arc.write().await = new_config.clone();
        debug!("Config reloaded from disk before batch");
        new_config
    }
    Err(error_msg) => {
        // Keep cached config on failure
        warn!("Failed to reload config: {}, using cached", error_msg);
        config_arc.read().await.clone()
    }
};

// Run batch with current_config snapshot
run_batch(current_config, token_manager).await?;
```

---

## 7. Credential Security

### Question
How to ensure credentials are never logged?

### Decision
Never log credential fields; use structured logging carefully.

### Implementation Guidelines

1. **Never log raw password/client_secret**
2. **Use `#[serde(skip_serializing)]` for sensitive fields in debug output**
3. **Log only non-sensitive identifiers** (account, domain - not password)
4. **Audit log statements** in config-related code

### Verification
Review all `tracing::info!`, `debug!`, `warn!`, `error!` calls in:
- `common/src/models/config.rs`
- `service/src/config/watcher.rs`
- `service/src/service/scheduler.rs`

---

## Summary

All technical unknowns have been resolved. Key findings:

1. **TOML + Serde**: Best practice, already implemented correctly
2. **File Watching**: `notify-debouncer-mini` correct choice, but **bug found** (GAP-001)
3. **Glob Patterns**: `globset` with case-insensitive matching, implemented correctly
4. **Cron Scheduling**: `tokio-cron-scheduler` with 5-to-6 field conversion, implemented
5. **Retry Strategy**: Exponential backoff + hourly, implemented correctly
6. **Hot-Reload**: Apply at batch start, preserve on failure, implemented
7. **Security**: Manual review required for credential logging
