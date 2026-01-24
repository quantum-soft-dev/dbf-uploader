# Quickstart: Configuration Module

**Feature**: 002-configuration-module
**Date**: 2026-01-24

## Overview

This guide helps developers work with the Configuration Module of dbf-uploader, covering configuration file setup, validation, hot-reload, and testing.

---

## Configuration File Location

**Production**: `C:\Program Files\data-exporter\config.toml`
**Development**: `./config.toml` (current directory)

---

## Minimal Configuration

Create `config.toml` with the following required settings:

```toml
[scheduler]
crontab = "0 8,12,16,18 * * *"  # Run at 8am, 12pm, 4pm, 6pm

[src]
source_dir = "C:\\data\\dbf"    # Path to DBF files

[credential]
account = "company"
username = "user"
password = "secret"

[api]
base_url = "https://api.example.com"

[encoding]
# Uses default CP866
```

---

## Using Device Flow Authentication

For OAuth 2.0 Device Authorization Flow:

```toml
[credential]
# Traditional credentials are ignored when device flow is configured

[credential.device]
site_id = "550e8400-e29b-41d4-a716-446655440000"
domain = "mysite.example.com"
client_secret = "your_device_secret"
```

---

## File Filtering

Filter which DBF files are processed:

```toml
[src]
source_dir = "C:\\data\\dbf"

# Whitelist: only process files matching these patterns
include_patterns = ["nsf*.dbf", "data_*.DBF"]

# Blacklist: exclude files matching these (applied after include)
exclude_patterns = ["nsfcli.DBF", "temp_*.dbf", "*.bak"]
```

**Matching Rules**:
1. Patterns are case-insensitive (Windows compatibility)
2. Include patterns act as whitelist - file must match at least one
3. Exclude patterns act as blacklist - file must not match any
4. Order: include filters first, then exclude

---

## Loading Configuration in Rust

### Basic Usage

```rust
use common::models::Config;
use std::path::Path;

// Load and validate
let config = Config::from_file(Path::new("config.toml"))?;

// Access values
println!("Crontab: {}", config.scheduler.crontab);
println!("Source dir: {}", config.src.source_dir.display());
println!("API URL: {}", config.api.base_url);

// Check authentication type
if config.credential.is_device_flow() {
    println!("Using device flow authentication");
} else {
    println!("Using traditional authentication");
}

// Get composed credentials
let username = config.credential.full_username(); // "account_user" or device domain
let password = config.credential.password();       // password or client_secret
```

### Saving Configuration

```rust
use common::models::Config;

let config = Config { /* ... */ };

// Save with pretty-print formatting
config.to_file(Path::new("config.toml"))?;
```

---

## Hot-Reload Support

Configuration changes are detected and applied automatically:

### Service Behavior

1. File watcher monitors `config.toml` with 2-second debounce
2. Changes are applied at the **start of the next batch cycle**
3. Invalid configuration keeps previous valid config (error logged)
4. Batch in progress is not interrupted

### Using ConfigWatcher

```rust
use service::config::ConfigWatcher;

// Create watcher
let watcher = ConfigWatcher::new("config.toml")?;

// Check for changes (non-blocking)
if watcher.has_changed() {
    let new_config = watcher.load_config()?;
    // Apply new configuration
}

// Or wait for changes (blocking)
let new_config = watcher.wait_for_change()?;
```

---

## Validation Errors

The configuration system provides specific, actionable error messages:

| Error | Cause | Fix |
|-------|-------|-----|
| "Crontab expression cannot be empty" | Empty crontab field | Add valid cron expression |
| "Source directory does not exist: {path}" | Invalid path | Create directory or fix path |
| "API base URL must start with https://" | http:// with https_only=true | Use https:// or set https_only=false |
| "Account cannot be empty when not using device flow" | Missing credential | Add account or configure device flow |
| "Invalid include pattern '{pattern}': {error}" | Invalid glob syntax | Fix pattern syntax |

---

## Testing Configuration

### Unit Test Example

```rust
use common::models::Config;
use tempfile::{TempDir, NamedTempFile};
use std::io::Write;

#[test]
fn test_config_loads_correctly() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().to_str().unwrap();

    let config_content = format!(r#"
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "{}"

[credential]
account = "test"
username = "user"
password = "pass"

[api]
base_url = "https://api.example.com"

[encoding]
"#, source_path.replace('\\', "\\\\"));

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(config_content.as_bytes()).unwrap();

    let config = Config::from_file(temp_file.path()).unwrap();
    assert_eq!(config.scheduler.crontab, "*/5 * * * *");
}
```

### Testing Validation Errors

```rust
#[test]
fn test_empty_crontab_fails() {
    let config_content = r#"
[scheduler]
crontab = ""
# ... rest of config
"#;
    // ... write to temp file
    let result = Config::from_file(temp_file.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Crontab expression cannot be empty"));
}
```

---

## GUI Configurator

The GUI configurator (`configurator.exe`) provides a visual interface:

### Sections

1. **Authentication** - Configure device flow or view current auth status
2. **Settings** - Set API URL and source directory
3. **Schedule** - Configure cron expression
4. **Service** - Install/start/stop/uninstall Windows service
5. **Status** - View service status

### Default Configuration

When no config file exists, defaults are:

```rust
Config {
    scheduler: { crontab: "0 0 8,12,16,18 * * *" },
    src: { source_dir: current_dir() or "C:\\data" },
    credential: { account: "", username: "", password: "", device: None },
    api: { base_url: "https://", https_only: true },
    encoding: { dbf_encoding: "CP866" },
}
```

---

## Cron Expression Reference

The service uses 6-field cron format (5-field automatically converted):

```
┌──────────── second (0-59) [added automatically if 5-field]
│ ┌────────── minute (0-59)
│ │ ┌──────── hour (0-23)
│ │ │ ┌────── day of month (1-31)
│ │ │ │ ┌──── month (1-12)
│ │ │ │ │ ┌── day of week (0-7, 0 and 7 are Sunday)
│ │ │ │ │ │
* * * * * *
```

**Examples**:

| Expression | Description |
|------------|-------------|
| `*/5 * * * *` | Every 5 minutes |
| `0 */2 * * *` | Every 2 hours at minute 0 |
| `0 8,12,16,18 * * *` | At 8am, 12pm, 4pm, 6pm |
| `0 9-17 * * 1-5` | Hourly 9am-5pm on weekdays |
| `30 4 * * *` | Daily at 4:30am |

---

## Network Drive Retry

When source directory is on a network share that's unavailable at startup:

1. Service retries with exponential backoff: 1, 2, 4, 8, 16 minutes
2. After 5 attempts, retries hourly
3. Service responds to stop commands during retry wait (within 5 seconds)
4. Other configuration errors fail immediately (no retry)

---

## Common Issues

### "Source directory does not exist" at startup

- Network share not mounted yet → service will retry automatically
- Path typo → fix path in config.toml

### Hot-reload not working

- Ensure file is saved (not just modified in memory)
- Wait for 2-second debounce period
- Check service logs for errors

### Credentials not working

- Device flow configured but missing fields → check domain and client_secret
- Traditional auth: ensure account, username, AND password are all set

### Glob patterns not matching

- Patterns are case-insensitive
- Use `*` for any characters, `?` for single character
- Character classes: `[a-z]`, `[0-9]`
- Invalid patterns cause validation error at load time

---

## Related Documentation

- [Data Model](./data-model.md) - Entity definitions and validation rules
- [Research](./research.md) - Technical decisions and alternatives
- [Feature Spec](./spec.md) - Requirements and acceptance criteria
