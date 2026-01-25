# Data Model: Configuration Module

**Feature**: 002-configuration-module
**Date**: 2026-01-24
**Location**: `common/src/models/config.rs`

## Overview

The Configuration Module defines the data structures for the dbf-uploader Windows service configuration. All configuration is stored in a single TOML file and loaded at startup with hot-reload support.

---

## Entity Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                           Config                                 │
│  (Root configuration container)                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │ SchedulerConfig │  │  SourceConfig   │  │ CredentialConfig│  │
│  ├─────────────────┤  ├─────────────────┤  ├─────────────────┤  │
│  │ crontab: String │  │ source_dir      │  │ account         │  │
│  │                 │  │ include_patterns│  │ username        │  │
│  │                 │  │ exclude_patterns│  │ password        │  │
│  └─────────────────┘  └─────────────────┘  │ device?         │──┼──┐
│                                            └─────────────────┘  │  │
│  ┌─────────────────┐  ┌─────────────────┐                       │  │
│  │   ApiConfig     │  │ EncodingConfig  │  ┌─────────────────┐  │  │
│  ├─────────────────┤  ├─────────────────┤  │DeviceCredentials│◄─┼──┘
│  │ base_url        │  │ dbf_encoding    │  ├─────────────────┤  │
│  │ https_only      │  │                 │  │ site_id         │  │
│  └─────────────────┘  └─────────────────┘  │ domain          │  │
│                                            │ client_secret   │  │
│                                            └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Entities

### Config (Root)

The root configuration container holding all service settings.

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub scheduler: SchedulerConfig,
    pub src: SourceConfig,
    pub credential: CredentialConfig,
    pub api: ApiConfig,
    pub encoding: EncodingConfig,
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| scheduler | SchedulerConfig | Yes | Cron scheduling settings |
| src | SourceConfig | Yes | Data source settings |
| credential | CredentialConfig | Yes | Authentication settings |
| api | ApiConfig | Yes | API connection settings |
| encoding | EncodingConfig | Yes | Character encoding settings |

**Methods**:
- `from_file(path: &Path) -> Result<Self>` - Load and validate from TOML file
- `to_file(path: &Path) -> Result<()>` - Save to TOML file (pretty-printed)
- `validate() -> Result<()>` - Validate all configuration values

---

### SchedulerConfig

Controls when batch processing jobs are executed.

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchedulerConfig {
    pub crontab: String,
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| crontab | String | Yes | - | Cron expression (5 or 6 field format) |

**Validation Rules**:
- Must not be empty
- Parsed by `tokio-cron-scheduler` at scheduler start

**Cron Format Examples**:
```
"0 8,12,16,18 * * *"      # 8am, 12pm, 4pm, 6pm daily (5-field)
"0 0 8,12,16,18 * * *"    # Same in 6-field format
"*/5 * * * *"             # Every 5 minutes
"0 0 * * 1-5"             # Midnight on weekdays
```

---

### SourceConfig

Defines the data source location and file filtering.

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SourceConfig {
    pub source_dir: PathBuf,
    #[serde(default)]
    pub include_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub exclude_patterns: Option<Vec<String>>,
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| source_dir | PathBuf | Yes | - | Directory containing DBF files |
| include_patterns | Option<Vec<String>> | No | None | Whitelist glob patterns |
| exclude_patterns | Option<Vec<String>> | No | None | Blacklist glob patterns |

**Validation Rules**:
- `source_dir` must exist (triggers retry if unavailable at startup)
- All patterns must be valid glob syntax

**Filter Logic**:
1. If `include_patterns` present: file MUST match at least one
2. If `exclude_patterns` present: file MUST NOT match any
3. Matching is **case-insensitive** (Windows compatibility)

**Pattern Examples**:
```toml
include_patterns = ["*.dbf", "data_*.DBF"]
exclude_patterns = ["temp_*.dbf", "*.bak", "nsfcli.DBF"]
```

---

### CredentialConfig

Authentication settings supporting two modes: traditional and device flow.

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialConfig {
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<DeviceCredentials>,
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| account | String | Conditional* | "" | Account identifier |
| username | String | Conditional* | "" | User name |
| password | String | Conditional* | "" | User password |
| device | Option<DeviceCredentials> | No | None | Device flow credentials |

*Required if `device` is not configured.

**Methods**:
- `full_username() -> String` - Returns `"{account}_{username}"` or device domain
- `password() -> &str` - Returns password or device client_secret
- `is_device_flow() -> bool` - Returns true if device credentials configured

**Authentication Priority**:
1. If `device` is Some → use device flow credentials
2. Otherwise → use traditional account/username/password

**Validation Rules**:
- If `device` is None: account, username, password must all be non-empty
- If `device` is Some: device validation applies instead

---

### DeviceCredentials

OAuth 2.0 Device Authorization Flow credentials (RFC 8628).

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceCredentials {
    pub site_id: String,
    pub domain: String,
    pub client_secret: String,
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| site_id | String | Yes | UUID of the registered site |
| domain | String | Yes | Site domain (used as username in Basic Auth) |
| client_secret | String | Yes | Site secret (used as password in Basic Auth) |

**Validation Rules**:
- `domain` must not be empty
- `client_secret` must not be empty

**Security Note**: `client_secret` must NEVER be logged.

---

### ApiConfig

API connection settings.

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub base_url: String,
    #[serde(default = "default_https_only")]
    pub https_only: bool,
}

fn default_https_only() -> bool { true }
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| base_url | String | Yes | - | API server base URL |
| https_only | bool | No | true | Enforce HTTPS connections |

**Validation Rules**:
- `base_url` must start with `http://` or `https://`
- If `https_only` is true: `base_url` must start with `https://`

**Security Warning**: Setting `https_only = false` is insecure and should only be used for local development/testing.

---

### EncodingConfig

Character encoding settings for DBF file processing.

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EncodingConfig {
    #[serde(default = "default_dbf_encoding")]
    pub dbf_encoding: String,
}

fn default_dbf_encoding() -> String { "CP866".to_string() }
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| dbf_encoding | String | No | "CP866" | Fallback encoding for DBF files |

**Supported Encodings**:
- `CP866` - DOS Cyrillic (default)
- `CP1251` - Windows Cyrillic
- `CP1255` - Windows Hebrew
- `UTF-8` - Unicode

---

## Validation Summary

| Entity | Field | Rule | Error Message |
|--------|-------|------|---------------|
| SchedulerConfig | crontab | Non-empty | "Crontab expression cannot be empty" |
| SourceConfig | source_dir | Exists on filesystem | "Source directory does not exist: {path}" |
| SourceConfig | include_patterns | Valid glob syntax | "Invalid include pattern '{pattern}': {error}" |
| SourceConfig | exclude_patterns | Valid glob syntax | "Invalid exclude pattern '{pattern}': {error}" |
| CredentialConfig | account | Non-empty (if no device) | "Account cannot be empty when not using device flow" |
| CredentialConfig | username | Non-empty (if no device) | "Username cannot be empty when not using device flow" |
| CredentialConfig | password | Non-empty (if no device) | "Password cannot be empty when not using device flow" |
| DeviceCredentials | domain | Non-empty | "Device domain cannot be empty" |
| DeviceCredentials | client_secret | Non-empty | "Device client_secret cannot be empty" |
| ApiConfig | base_url | Valid protocol | "API base URL must start with http:// or https://" |
| ApiConfig | base_url | HTTPS if https_only | "API base URL must start with https:// when https_only is enabled" |

---

## TOML Configuration Example

```toml
[scheduler]
crontab = "0 8,12,16,18 * * *"

[src]
source_dir = "C:\\data\\dbf"
include_patterns = ["*.dbf", "nsf*.DBF"]
exclude_patterns = ["temp_*.dbf", "nsfcli.DBF"]

[credential]
account = "mycompany"
username = "serviceuser"
password = "secretpassword"

# OR use device flow:
# [credential.device]
# site_id = "550e8400-e29b-41d4-a716-446655440000"
# domain = "mysite.example.com"
# client_secret = "device_secret_key"

[api]
base_url = "https://api.example.com"
https_only = true

[encoding]
dbf_encoding = "CP866"
```

---

## State Transitions

### Service Startup Flow

```
                    ┌───────────────┐
                    │    Initial    │
                    └───────┬───────┘
                            │
                            ▼
                    ┌───────────────┐
                    │ LoadingConfig │
                    └───────┬───────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
              ▼                           ▼
    ┌─────────────────┐         ┌─────────────────┐
    │   Validating    │         │ Source Dir Not  │
    └────────┬────────┘         │     Found       │
             │                  └────────┬────────┘
    ┌────────┴────────┐                  │
    │                 │                  ▼
    ▼                 ▼         ┌─────────────────┐
┌───────┐    ┌───────────────┐  │   RetryWait     │◄───┐
│Running│    │ OtherError    │  │ (1,2,4,8,16,60m)│    │
└───────┘    │   Stopped     │  └────────┬────────┘    │
             └───────────────┘           │             │
                                         │ retry       │
                                         └─────────────┘
```

### Hot-Reload Flow

```
┌─────────┐     File Changed      ┌───────────┐
│ Running │ ─────────────────────▶│ Reloading │
└─────────┘                       └─────┬─────┘
     ▲                                  │
     │           ┌──────────────────────┴──────────────────────┐
     │           │                                             │
     │           ▼                                             ▼
     │   ┌───────────────┐                            ┌───────────────┐
     └───│ Valid Config  │                            │ Invalid Config│
         │ (Apply New)   │                            │ (Keep Cached) │
         └───────────────┘                            └───────┬───────┘
                                                              │
                                                              └────────▶ Log Error
```

---

## Related Files

| File | Purpose |
|------|---------|
| `common/src/models/config.rs` | Entity definitions and validation |
| `common/src/models/mod.rs` | Module exports |
| `service/src/config/watcher.rs` | File watching for hot-reload |
| `service/src/processor/filter.rs` | Glob pattern matching |
| `configurator/src/config_manager.rs` | GUI load/save operations |
