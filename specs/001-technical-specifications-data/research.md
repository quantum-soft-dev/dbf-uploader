# Research: Data Exporter Service Implementation

**Date**: 2025-10-05
**Purpose**: Technical research for implementing a Windows service in Rust for DBF file export

## Technology Decisions

### 1. Windows Service Management

**Decision**: Use `windows-service` crate (v0.7+)

**Rationale**:
- Most mature and widely-used Windows service implementation in Rust
- Production-proven (used by Mullvad VPN)
- Provides `define_windows_service!` macro for boilerplate generation
- Handles service lifecycle, control events, and system integration
- Well-documented with recent tutorials

**Alternatives Considered**:
- `windows-services` (Microsoft official): Lower adoption, newer offering. Not chosen due to less community validation.

**Implementation Notes**:
- Must handle service control events properly to avoid timeout failures
- Requires service lifecycle management (start, stop, pause, continue)
- Testing locally requires service installation or test harnesses

---

### 2. Cron Scheduling

**Decision**: Use `tokio-cron-scheduler` crate

**Rationale**:
- Full-featured async scheduler built on Tokio
- Uses `croner` for cron parsing (supports standard Unix format)
- Supports timezone-aware scheduling
- Job notifications (start, stop, remove events)
- Optional persistence using PostgreSQL or Nats

**Alternatives Considered**:
- `croner` (parser only): Would require custom scheduler implementation
- `job_scheduler`: Synchronous only, less suitable for async HTTP operations

**Implementation Notes**:
- Requires multi-threaded Tokio runtime (single-threaded will hang on `scheduler.add()`)
- Notification ordering not guaranteed for rapidly-finishing tasks
- Consider persistence layer if service restarts mid-batch

---

### 3. DBF File Parsing with Encoding

**Decision**: Use `dbase` crate with `yore` feature for encoding support

**Rationale**:
- Most maintained DBF parser in Rust
- Supports dBase III, IV, and VisualFoxPro formats
- `yore` feature provides CP866 and Windows-1251 encoding support
- Can read field definitions from headers

**Supplementary**: `encoding_rs` for Windows-1251 and UTF-8 conversion

**Alternatives Considered**:
- `dbase` with `encoding_rs` feature: Doesn't support CP866, only UTF-8 and GBK

**Implementation Notes**:
- DBF header code page marker not always reliable - may need manual encoding specification
- Memo fields are read-only
- If both `yore` and `encoding_rs` features enabled, `yore` takes priority
- Use `encoding_rs` for converting from detected encoding to UTF-8 for CSV

**Dependency Configuration**:
```toml
dbase = { version = "0.6", features = ["yore"] }
encoding_rs = "0.8"
```

---

### 4. HTTP Client

**Decision**: Use `reqwest` crate with `rustls-tls` backend

**Rationale**:
- De facto standard HTTP client in Rust
- Built-in support for HTTPS-only, JWT Bearer auth, Basic auth, multipart uploads
- `rustls-tls`: Pure Rust TLS, no OpenSSL dependency, easier cross-compilation

**Alternatives Considered**:
- `hyper` directly: Lower-level, more complex API
- `reqwest` with `native-tls`: Platform-dependent, requires system TLS libraries

**Implementation Notes**:
- Requires Tokio async runtime
- Must set `default-features = false` to avoid TLS backend conflicts
- For large file uploads, use streaming with `Part::stream()` instead of `.file()` to avoid loading entire file in memory
- Token refresh logic must be implemented manually

**Dependency Configuration**:
```toml
reqwest = { version = "0.11", features = ["multipart", "rustls-tls"], default-features = false }
```

**Key Capabilities**:
- HTTPS-only: `Client::builder().https_only(true).use_rustls_tls()`
- JWT: `.bearer_auth(token)`
- Basic Auth: `.basic_auth(username, Some(password))`
- Multipart: `multipart::Form::new().file("field", path)`

---

### 5. Configuration Management

**Decision**: Use `toml` + `serde` for parsing, `notify-debouncer-mini` for file watching

**Rationale**:
- `toml`: Official TOML parser, used by Cargo itself, seamless serde integration
- `notify-debouncer-mini`: Prevents multiple events per file change, lightweight, cross-platform

**Alternatives Considered**:
- `notify-debouncer-full`: More features but heavier, not needed for simple config watching
- Manual polling: Less efficient, higher latency

**Implementation Notes**:
- Debouncer must be kept alive (don't drop it)
- Recommended debounce duration: 1-2 seconds for config files
- Network filesystems may not emit events - use PollWatcher backend if needed
- Validate configuration before applying changes

**Dependency Configuration**:
```toml
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
notify-debouncer-mini = "0.4"
```

---

### 6. Error Handling Pattern

**Decision**: Hybrid approach with `thiserror` for domain errors and `anyhow` for application errors

**Rationale**:
- `thiserror`: Creates custom error types with derive macros, enables different error variant handling
- `anyhow`: Trait object-based errors with context, perfect for applications and logging
- Hybrid provides best of both: type safety where needed, ergonomics at application layer

**Alternatives Considered**:
- `eyre`: Similar to anyhow with more customization, but standard anyhow is sufficient
- Pure `thiserror`: More boilerplate at application level
- Pure `anyhow`: Loses type information for library-like code

**Implementation Pattern**:
```rust
// Domain errors (thiserror)
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("DBF parsing failed: {0}")]
    DbfParse(#[from] dbase::Error),

    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

// Application error handling (anyhow)
fn process() -> anyhow::Result<()> {
    let data = read_dbf()
        .context("Failed to read DBF file")?;
    Ok(())
}
```

**Fallback Logging Strategy**:
- Try server error reporting first
- On network failure, write to local error.log with full context
- Include timestamp, error chain, and operation context

**Dependency Configuration**:
```toml
thiserror = "1.0"
anyhow = "1.0"
```

---

### 7. CSV Writing

**Decision**: Use `csv` crate with serde serialization

**Rationale**:
- Fast and flexible, maintained by BurntSushi
- RFC 4180 compliant by default
- Automatic quote escaping (doubles quotes: `""`)
- Native UTF-8 handling (Rust strings are UTF-8)
- Serde integration for structured data

**Alternatives Considered**:
- Manual CSV writing: Error-prone, doesn't handle edge cases properly

**Implementation Notes**:
- Always call `.flush()` when done (internal buffering)
- Use `encoding_rs` to convert from source encoding (CP866/Windows-1251) to UTF-8 before writing
- Fields with commas, newlines, or quotes are automatically quoted
- Quote style can be configured if needed

**Dependency Configuration**:
```toml
csv = "1.3"
```

**Encoding Conversion Pattern**:
```rust
use encoding_rs::WINDOWS_1251;
let (decoded, _, _) = WINDOWS_1251.decode(&bytes);
let mut wtr = csv::Writer::from_path("output.csv")?;
wtr.write_record(&["col1", "col2"])?;
wtr.write_record(&[&decoded, "value"])?;
wtr.flush()?;
```

---

### 8. Gzip Compression

**Decision**: Use `flate2` crate with default `miniz_oxide` backend

**Rationale**:
- Standard compression library maintained by Rust team
- Pure Rust implementation (no system dependencies)
- Safe Rust only
- Good performance for most use cases
- Supports streaming compression

**Alternatives Considered**:
- `zlib-rs` backend: Faster but not needed for this use case
- `zlib-ng` backend: Requires C compiler, more complex build

**Implementation Notes**:
- Always call `.finish()` to flush and finalize gzip stream
- Use `Compression::default()` (level 6) for good balance
- For large files, write directly to file instead of in-memory Vec
- Compression can fail (disk full) - handle errors appropriately

**Dependency Configuration**:
```toml
flate2 = "1.0"
```

**Usage Pattern**:
```rust
use flate2::write::GzEncoder;
use flate2::Compression;

let file = std::fs::File::create("output.csv.gz")?;
let mut encoder = GzEncoder::new(file, Compression::default());
encoder.write_all(csv_data.as_bytes())?;
encoder.finish()?; // Must call!
```

---

### 9. Logging and Observability

**Decision**: Use `tracing` ecosystem for structured logging

**Rationale**:
- Designed for long-running services and async applications
- Structured logging with spans and events
- Environment-based filtering
- Industry standard for Rust services

**Implementation Notes**:
- Configure log levels via environment variables
- Use spans to track operation context (batch ID, file name)
- Log to both stdout and local file (fallback when server unavailable)

**Dependency Configuration**:
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

### 10. Async Runtime

**Decision**: Use Tokio with full features

**Rationale**:
- Required by `reqwest` and `tokio-cron-scheduler`
- Production-proven for long-running services
- Multi-threaded runtime required for cron scheduler

**Dependency Configuration**:
```toml
tokio = { version = "1.0", features = ["full"] }
```

**Implementation Notes**:
- Service must initialize Tokio runtime in service main function
- Use `#[tokio::main]` for async entry point or build runtime manually
- Multi-threaded runtime required (single-threaded causes scheduler hangs)

---

## Summary of Key Dependencies

```toml
[dependencies]
# Windows Service
windows-service = "0.7"

# Scheduling & Async Runtime
tokio-cron-scheduler = "0.13"
tokio = { version = "1.0", features = ["full"] }

# DBF Parsing & Encoding
dbase = { version = "0.6", features = ["yore"] }
encoding_rs = "0.8"

# HTTP Client
reqwest = { version = "0.11", features = ["multipart", "rustls-tls"], default-features = false }

# Configuration
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
notify-debouncer-mini = "0.4"

# Error Handling
thiserror = "1.0"
anyhow = "1.0"

# CSV & Compression
csv = "1.3"
flate2 = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

## Architecture Considerations

### Service Lifecycle
1. Install command: Register service, copy executable, create config, validate credentials
2. Service start: Initialize Tokio runtime, load config, set up scheduler, start config watcher
3. Scheduled execution: Scan directory, process files (convert → compress → upload), handle errors
4. Config reload: Detect file change, validate, apply at next scheduled run
5. Service stop: Cancel in-flight operations gracefully, flush logs
6. Uninstall: Stop service, unregister, clean up installation directory

### Error Recovery Strategy
1. Individual file errors: Log to server, continue with batch
2. Network errors (upload): Write to local error.log, continue with batch
3. Directory inaccessible: Report to server, skip iteration
4. Auth failure: Skip scheduled operation, report to server
5. Disk full: Report to server, continue with remaining files

### Concurrency Model
- Single scheduler instance
- Sequential file processing per batch (no parallel processing specified in requirements)
- Async I/O for network operations
- Config watcher runs in background

### Security Considerations
- HTTPS-only communication (enforced in reqwest client)
- JWT tokens stored in memory only (never persisted)
- Config file restricted to administrators (Windows ACL)
- No credential caching beyond in-memory JWT

---

## Testing Strategy

### Unit Tests
- DBF encoding detection
- CSV conversion logic
- Filename path encoding (subdirectory handling)
- Cron expression parsing
- Config validation

### Integration Tests
- Service lifecycle (install, start, stop, uninstall)
- Full batch processing flow
- Config reload during operation
- Locked file retry logic

### Contract Tests
- JWT token endpoint (Basic auth → JWT)
- File upload endpoint (multipart with gzip)
- Error report endpoint (JSON structure)

All contract tests should be written first (TDD) and fail until implementation complete.

---

## Risk Mitigation

### Known Risks
1. **DBF encoding detection unreliable**: Mitigation - fallback encoding in config
2. **Large file memory usage**: Mitigation - streaming compression and upload
3. **Network instability**: Mitigation - local error logging, retry on next scheduled run
4. **Service timeout on shutdown**: Mitigation - graceful cancellation of in-flight operations
5. **Config change mid-batch**: Mitigation - defer config application to next run

### Performance Concerns
- Thousands of files: Use efficient directory traversal, streaming I/O
- Large individual files: Stream through conversion/compression/upload pipeline
- Idle resource usage: Use event-driven architecture, sleep scheduler thread

---

**Research Complete**: All NEEDS CLARIFICATION items resolved. Ready to proceed to Phase 1 (Design & Contracts).
