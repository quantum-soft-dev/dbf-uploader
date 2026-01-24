# Implementation Plan: Configuration Module

**Branch**: `002-configuration-module` | **Date**: 2026-01-24 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-configuration-module/spec.md`

## Summary

Type-safe configuration management system for Windows service dbf-uploader with TOML file parsing, validation, hot-reload capability, and dual authentication support (traditional credentials and OAuth 2.0 Device Flow). The system handles network drive unavailability at startup with exponential backoff retry and supports graceful shutdown during retry periods.

## Technical Context

**Language/Version**: Rust 1.82.0 (stable, as per `rust-version` in Cargo.toml)
**Primary Dependencies**:
- `toml = "0.8"` - TOML parsing with serde integration
- `serde = "1"` - Serialization/deserialization framework
- `notify-debouncer-mini = "0.4"` - File watching with 2-second debounce
- `globset = "0.4.15"` - Case-insensitive glob pattern matching
- `tokio-cron-scheduler = "0.13"` - 6-field cron expression scheduling
- `windows-service = "0.7"` - Windows service management
- `native-windows-gui = "1.0"` - GUI configurator framework

**Storage**: File-based (TOML configuration at `C:\Program Files\data-exporter\config.toml`)
**Testing**: `cargo test` with tempfile, wiremock for mocking
**Target Platform**: Windows 10+ / Windows Server 2016+
**Project Type**: Multi-crate workspace (common, service, configurator)
**Performance Goals**: Configuration loads and validates within 100ms (SC-001)
**Constraints**:
- Hot-reload applies changes at next batch cycle (not during batch execution)
- Service responds to stop commands within 5 seconds during retry wait (SC-005)
- Credentials never written to log files (SC-004)

**Scale/Scope**: Single Windows service instance per machine

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Note**: The constitution file contains only a template structure without defined principles. The following implicit principles are derived from the codebase:

| Principle | Status | Notes |
|-----------|--------|-------|
| Single workspace, multiple crates | PASS | Already follows `common/service/configurator` structure |
| Test coverage | PASS | >80% test coverage target (SC-007), tests exist for all modules |
| Type safety | PASS | Using Rust's type system with serde derive |
| Error handling | PASS | Uses `thiserror` for typed errors, `anyhow` for CLI |
| Logging/Observability | PASS | Uses `tracing` with daily log rotation |

## Project Structure

### Documentation (this feature)

```text
specs/002-configuration-module/
├── plan.md              # This file
├── research.md          # Phase 0 output - research findings
├── data-model.md        # Phase 1 output - entity documentation
├── quickstart.md        # Phase 1 output - developer guide
├── contracts/           # Phase 1 output - API contracts (N/A - no HTTP API)
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
# Existing Rust workspace structure
common/
├── src/
│   ├── models/
│   │   └── config.rs        # Config structs with validation ✓ IMPLEMENTED
│   ├── auth/
│   │   ├── mod.rs           # TokenManager ✓ IMPLEMENTED
│   │   └── device_flow.rs   # OAuth 2.0 Device Flow ✓ IMPLEMENTED
│   ├── error/
│   │   └── mod.rs           # ProcessingError types ✓ IMPLEMENTED
│   └── lib.rs
└── Cargo.toml

service/
├── src/
│   ├── config/
│   │   ├── mod.rs           # Config module exports ✓ IMPLEMENTED
│   │   └── watcher.rs       # ConfigWatcher with debounce ✓ IMPLEMENTED
│   ├── service/
│   │   ├── scheduler.rs     # BatchScheduler with hot-reload ✓ IMPLEMENTED
│   │   └── windows_service.rs # Windows service with retry ✓ IMPLEMENTED
│   ├── processor/
│   │   └── filter.rs        # Glob pattern filtering ✓ IMPLEMENTED
│   └── main.rs
└── Cargo.toml

configurator/
├── src/
│   ├── config_manager.rs    # Load/save config ✓ IMPLEMENTED
│   ├── service_manager.rs   # Windows service control ✓ IMPLEMENTED
│   ├── ui/
│   │   └── app.rs           # GUI application ✓ IMPLEMENTED
│   └── main.rs
└── Cargo.toml

tests/
├── integration/
│   ├── config_reload_test.rs # Hot-reload tests ✓ IMPLEMENTED
│   └── filter_test.rs        # Glob pattern tests ✓ IMPLEMENTED
└── contract/
    └── mod.rs
```

**Structure Decision**: The existing multi-crate workspace structure is maintained. The configuration module is distributed across `common` (models, validation), `service` (watcher, hot-reload), and `configurator` (GUI).

## Implementation Status Analysis

Based on code review, the configuration module is **substantially implemented**. The following table maps spec requirements to implementation status:

### Functional Requirements Coverage

| Requirement | Status | Implementation Location |
|-------------|--------|------------------------|
| FR-001: Load config from TOML | ✓ DONE | `common/src/models/config.rs:122-127` |
| FR-002: Parse 5 sections | ✓ DONE | `common/src/models/config.rs:5-118` |
| FR-003: Validate crontab non-empty | ✓ DONE | `common/src/models/config.rs:147-149` |
| FR-004: Validate source_dir exists | ✓ DONE | `common/src/models/config.rs:151-156` |
| FR-005: Validate URL protocol | ✓ DONE | `common/src/models/config.rs:164-166` |
| FR-006: Enforce HTTPS-only | ✓ DONE | `common/src/models/config.rs:159-162` |
| FR-007: Validate traditional creds | ✓ DONE | `common/src/models/config.rs:177-188` |
| FR-008: Validate device flow creds | ✓ DONE | `common/src/models/config.rs:169-177` |
| FR-009: Validate glob patterns | ✓ DONE | `common/src/models/config.rs:191-203` |
| FR-010: Default values | ✓ DONE | `common/src/models/config.rs:104-106, 116-118` |
| FR-011: Actionable error messages | ✓ DONE | All validation returns specific errors |
| FR-012: Monitor config file | ✓ DONE | `service/src/config/watcher.rs:22-72` |
| FR-013: Debounce 2 seconds | ✓ DONE | `service/src/config/watcher.rs:30` |
| FR-014: Apply at next batch cycle | ✓ DONE | `service/src/service/scheduler.rs:86-109` |
| FR-015: Preserve config on reload fail | ✓ DONE | `service/src/service/scheduler.rs:100-108` |
| FR-016: Compose username | ✓ DONE | `common/src/models/config.rs:71-77` |
| FR-017: Device flow priority | ✓ DONE | `common/src/models/config.rs:71-92` |
| FR-018: Include whitelist filter | ✓ DONE | `service/src/processor/filter.rs:72-82` |
| FR-019: Exclude blacklist filter | ✓ DONE | `service/src/processor/filter.rs:84-94` |
| FR-020: Case-insensitive matching | ✓ DONE | `service/src/processor/filter.rs:40-43` |
| FR-021: Retry with backoff | ✓ DONE | `service/src/service/windows_service.rs:62-150` |
| FR-022: Hourly retry after 5 attempts | ✓ DONE | `service/src/service/windows_service.rs:68-69, 99-103` |
| FR-023: Respond to stop during retry | ✓ DONE | `service/src/service/windows_service.rs:119-126` |
| FR-024: Fail immediately for other errors | ✓ DONE | `service/src/service/windows_service.rs:140-146` |
| FR-025: GUI load defaults | ✓ DONE | `configurator/src/config_manager.rs:19-30, 53-82` |
| FR-026: GUI create directories | ✓ DONE | `configurator/src/config_manager.rs:36-39` |
| FR-027: GUI pretty-print TOML | ✓ DONE | `configurator/src/config_manager.rs:34` |

### Identified Gaps

| Gap | Description | Priority |
|-----|-------------|----------|
| GAP-001 | ConfigWatcher doesn't start watching (missing `debouncer.watcher().watch()` call) | P1 |
| GAP-002 | GUI doesn't expose include/exclude pattern configuration | P3 |
| GAP-003 | No explicit test for SC-001 (100ms load time) | P2 |
| GAP-004 | No explicit test for SC-002 (5s hot-reload detection) | P2 |
| GAP-005 | No explicit test for SC-005 (5s stop response during retry) | P2 |

## Complexity Tracking

No constitution violations requiring justification.

## Research Findings (Phase 0)

### 1. TOML Configuration Best Practices (Rust)

**Decision**: Use `serde` derive macros with `#[serde(default)]` for optional fields
**Rationale**: Already implemented correctly; provides compile-time type safety
**Alternatives considered**:
- Manual parsing: Too verbose, error-prone
- JSON/YAML: TOML better suited for configuration files, human-readable

### 2. File Watching with Debouncing

**Decision**: Use `notify-debouncer-mini` with 2-second debounce
**Rationale**: Prevents multiple reloads from editor save operations
**Issue Found**: The current implementation creates the debouncer but doesn't call `watch()` to start monitoring
**Fix Required**: Add `debouncer.watcher().watch(watch_path, RecursiveMode::NonRecursive)?` in `ConfigWatcher::new()`

### 3. Glob Pattern Matching

**Decision**: Use `globset` with case-insensitive matching
**Rationale**: Already implemented correctly; supports standard glob syntax
**Pattern**: Include patterns act as whitelist (must match), exclude as blacklist (applied after)

### 4. Cron Expression Format

**Decision**: Support both 5-field (standard) and 6-field (with seconds) cron expressions
**Rationale**: Already implemented with automatic conversion in `scheduler.rs:50-57`
**Library**: `tokio-cron-scheduler` requires 6-field format; 5-field is converted by prepending "0 "

### 5. Windows Service Retry Strategy

**Decision**: Exponential backoff (1, 2, 4, 8, 16 min) then hourly
**Rationale**: Balances quick recovery with resource efficiency
**Implementation**: Already correct in `windows_service.rs:68-69`

## Data Model (Phase 1)

See [data-model.md](./data-model.md) for complete entity documentation.

### Core Entities

```rust
// Root configuration container
pub struct Config {
    pub scheduler: SchedulerConfig,    // Cron scheduling
    pub src: SourceConfig,             // Data source settings
    pub credential: CredentialConfig,  // Authentication
    pub api: ApiConfig,                // API connection
    pub encoding: EncodingConfig,      // Character encoding
}

// Sub-configurations
pub struct SchedulerConfig { pub crontab: String }
pub struct SourceConfig {
    pub source_dir: PathBuf,
    pub include_patterns: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
}
pub struct CredentialConfig {
    pub account: String,
    pub username: String,
    pub password: String,
    pub device: Option<DeviceCredentials>,
}
pub struct DeviceCredentials {
    pub site_id: String,
    pub domain: String,
    pub client_secret: String,
}
pub struct ApiConfig { pub base_url: String, pub https_only: bool }
pub struct EncodingConfig { pub dbf_encoding: String }
```

### State Transitions

```
Service Startup:
  [Initial] -> [LoadingConfig] -> [Validating] -> [Running]
                    |                   |
                    v                   v
              [RetryWait] <----  [ValidationFailed] -> [Stopped]
                    |
                    v
              [Running] (after source_dir becomes available)

Hot-Reload:
  [Running] -> [FileChanged] -> [Reloading] -> [Running] (new config)
                                    |
                                    v
                              [Running] (cached config, error logged)
```

## Contracts (Phase 1)

No HTTP API contracts - this is a file-based configuration system.

### TOML Configuration Contract

```toml
# config.toml schema
[scheduler]
crontab = "0 8,12,16,18 * * *"  # Required: 5 or 6 field cron expression

[src]
source_dir = "C:\\data\\dbf"    # Required: Path to DBF files
include_patterns = ["*.dbf"]    # Optional: Whitelist patterns
exclude_patterns = ["temp_*"]   # Optional: Blacklist patterns

[credential]
account = "company"             # Required if no device flow
username = "user"               # Required if no device flow
password = "secret"             # Required if no device flow

[credential.device]             # Optional: Takes precedence over traditional
site_id = "uuid"
domain = "site_domain"
client_secret = "secret"

[api]
base_url = "https://api.example.com"  # Required
https_only = true                      # Optional, default: true

[encoding]
dbf_encoding = "CP866"          # Optional, default: "CP866"
```

## Implementation Tasks Summary

### P0: Critical Bug Fix
1. Fix `ConfigWatcher` to actually start file watching

### P1: Testing Gaps
2. Add benchmark test for SC-001 (100ms config load)
3. Add integration test for SC-002 (5s hot-reload)
4. Add integration test for SC-005 (5s stop response)

### P2: Documentation
5. Create `research.md` with detailed findings
6. Create `data-model.md` with entity documentation
7. Create `quickstart.md` with developer guide

### P3: Optional Enhancements
8. Add include/exclude pattern fields to GUI configurator
