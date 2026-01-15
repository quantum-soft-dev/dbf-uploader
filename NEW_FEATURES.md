# New Features - Device Authorization Flow & Global Error Reporting

This document describes the newly implemented features for the Data Exporter application.

## Table of Contents

1. [Device Authorization Flow (RFC 8628)](#device-authorization-flow-rfc-8628)
2. [Global Error Reporting](#global-error-reporting)
3. [API Integration](#api-integration)

---

## Device Authorization Flow (RFC 8628)

The Device Authorization Grant allows headless devices (IoT, CLI, embedded systems) to obtain site credentials through browser-based user authorization.

### When to Use

- New device without credentials
- Device lost/rotated credentials
- Headless environments (no interactive browser)

### Usage

#### Command Line

```bash
data_exporter.exe authorize-device \
  --device-name "Warehouse Scanner #1" \
  --device-type "raspberry-pi" \
  --firmware-version "2.1.0" \
  --source-dir "C:\Data\DBF" \
  --crontab "*/5 * * * *" \
  --api-url "https://dev.dfm.bitbi.io"
```

#### Example Output

```
Starting device authorization flow...

Requesting device authorization codes...

═══════════════════════════════════════════════════════════
  DEVICE AUTHORIZATION REQUIRED
═══════════════════════════════════════════════════════════

  1. Open: https://app.dataforge.com/device-verify

  2. Enter code: ABCD-1234

  3. Select your site and click 'Authorize'

  Code expires in 15 minutes.
═══════════════════════════════════════════════════════════

Waiting for user to authorize device...

✓ Device authorized successfully!
  Domain: warehouse.example.com

Configuration saved to: C:\ProgramData\data_exporter\config.toml

Device authorization complete!
You can now install the service using the saved configuration.
```

### Configuration File

After successful authorization, a configuration file is created with device credentials:

```toml
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "C:\\Data\\DBF"

[credential.device]
domain = "warehouse.example.com"
client_secret = "cs_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6"

[api]
base_url = "https://dev.dfm.bitbi.io"
https_only = true

[encoding]
dbf_encoding = "CP866"
```

### Implementation Details

The device flow is implemented in [src/auth/device_flow.rs](src/auth/device_flow.rs):

- **DeviceFlowClient**: Main client for device authorization
- **ClientMetadata**: Device information (name, type, firmware version)
- **DeviceCredentials**: Domain and client secret obtained after authorization
- **Polling Logic**: Automatic retry with backoff according to RFC 8628

Key methods:

- `authorize()`: Request device and user codes (Step 1)
- `poll_for_token()`: Poll for credentials after user confirmation (Step 3)
- `run_device_flow()`: Complete flow with automatic polling

### Error Handling

The device flow handles all RFC 8628 error codes:

- `authorization_pending`: User hasn't authorized yet (continue polling)
- `slow_down`: Polling too fast (increase interval by 5s)
- `access_denied`: User denied authorization (stop polling)
- `expired_token`: Code expired (restart flow)
- `invalid_grant`: Code not found (restart flow)

---

## Global Error Reporting

Global error reporting allows the application to report system-level errors to the server outside of batch processing context.

### Use Cases

- **System-level errors**: Database connection failures, disk space issues
- **Configuration errors**: Invalid settings, missing environment variables
- **Startup/shutdown errors**: Application lifecycle issues
- **Background process errors**: Scheduled task failures, monitoring alerts

### Error Severity Levels

| Severity | Description | Use Case |
|----------|-------------|----------|
| `CRITICAL` | System failure, data loss risk | Database down, file corruption, security breach |
| `ERROR` | Operation failed (default) | Connection failed, service unavailable |
| `WARNING` | Degraded performance, recoverable | Slow network, disk space low, retry succeeded |
| `INFO` | Informational, expected condition | Service started, config reloaded, health check |

### Implementation

The global error reporting system consists of:

1. **GlobalErrorReport** ([src/models/error_report.rs](src/models/error_report.rs)): Error model with severity levels
2. **ErrorReporter::send_global_error()** ([src/error/reporter.rs](src/error/reporter.rs)): HTTP client for sending errors
3. **GlobalErrorLogger** ([src/error/global_logger.rs](src/error/global_logger.rs)): High-level API for error reporting

### Usage Example

```rust
use data_exporter::error::GlobalErrorLogger;
use data_exporter::models::{ErrorSeverity, GlobalErrorReport};
use std::collections::HashMap;

// Create global error logger
let logger = GlobalErrorLogger::new(reporter, token_manager, config);

// Report a critical error
let mut metadata = HashMap::new();
metadata.insert("host".to_string(), serde_json::Value::String("db.example.com".to_string()));
metadata.insert("port".to_string(), serde_json::Value::Number(5432.into()));

logger.critical(
    "CONNECTION_FAILED".to_string(),
    "Database connection lost: Connection refused after 3 retries".to_string(),
    Some(metadata),
).await?;

// Convenience methods for different severity levels
logger.error("SERVICE_UNAVAILABLE".to_string(), "Payment gateway unreachable".to_string(), None).await?;
logger.warning("DISK_FULL".to_string(), "Disk space below 10% threshold".to_string(), None).await?;
logger.info("STARTUP_SUCCESS".to_string(), "Service started successfully".to_string(), None).await?;
```

### API Endpoint

Global errors are sent to:

```
POST /api/dfc/error
Authorization: Bearer {jwt_token}
Content-Type: application/json
```

Request body:

```json
{
  "type": "CONNECTION_FAILED",
  "message": "Failed to connect to database server: timeout after 30s",
  "severity": "CRITICAL",
  "metadata": {
    "host": "db.example.com",
    "port": 5432,
    "retryCount": 3,
    "lastError": "ETIMEDOUT"
  }
}
```

Response: `204 No Content` on success

### Error Type Guidelines

Use consistent error types for easier filtering and analysis:

| Type | Description |
|------|-------------|
| `CONNECTION_FAILED` | Network/database connection errors |
| `AUTHENTICATION_FAILED` | Auth failures (credentials, token) |
| `SERVICE_UNAVAILABLE` | External service unreachable |
| `TIMEOUT_ERROR` | Operation timeouts |
| `DISK_FULL` | Storage capacity issues |
| `MEMORY_ERROR` | Out of memory conditions |
| `CONFIG_ERROR` | Configuration problems |
| `STARTUP_ERROR` | Application startup failures |
| `SHUTDOWN_ERROR` | Graceful shutdown failures |
| `HEALTH_CHECK_FAILED` | Health monitoring failures |
| `UNKNOWN_ERROR` | Unclassified errors |

### Validation & Limits

The system enforces API limits automatically:

- Error type: max 100 characters
- Message: max 10,000 characters
- Metadata: max 20 keys, 10KB total size

The `GlobalErrorReport::validate_and_truncate()` method ensures compliance with these limits.

---

## API Integration

### Authentication Flow

Both traditional and device flow credentials use the same authentication endpoint to obtain JWT tokens:

```
POST /api/dfc/auth/token
Authorization: Basic {base64(domain:client_secret)}
```

**Example:**
```bash
# For traditional auth: domain = account_username
# For device flow: domain = warehouse.example.com
curl -X POST https://dev.dfm.bitbi.io/api/dfc/auth/token \
  -H "Authorization: Basic $(echo -n 'warehouse.example.com:cs_secret123' | base64)"
```

**Response:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expiresIn": 86400,
  "tokenType": "Bearer"
}
```

The `AuthClient` in [src/auth/mod.rs](src/auth/mod.rs) automatically uses the correct credentials (traditional or device flow) from the configuration.

### Device Flow Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/device/authorize` | POST | Request device and user codes |
| `/api/v1/device/token` | POST | Poll for credentials |

### Global Error Endpoint

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/dfc/error` | POST | Report global error (requires JWT) |

---

## Testing

All new features include comprehensive unit tests:

```bash
# Run all tests
cargo test

# Run specific test modules
cargo test auth::device_flow
cargo test error::global_logger
cargo test models::error_report

# Run tests with output
cargo test -- --nocapture
```

Test coverage includes:

- Device flow client creation and validation
- Error response handling (authorization_pending, slow_down, etc.)
- Global error report creation with all severity levels
- Metadata validation and truncation
- Configuration parsing with device credentials

---

## Migration Guide

### From Traditional Auth to Device Flow

If you have an existing installation with traditional credentials:

1. Run the `authorize-device` command to obtain device credentials
2. The new configuration will overwrite the existing one
3. Restart the service to use the new credentials

### Backward Compatibility

The application maintains backward compatibility:

- Traditional credentials (account/username/password) still work
- Existing installations continue to function without changes
- Device credentials are optional (`credential.device` field in config)

---

## Security Considerations

### Device Flow

- Device code: 64 chars, UUID-based, cryptographically secure
- User code: XXXX-1234 format, excludes confusing chars (I, O, 0, 1)
- Code TTL: 15 minutes (auto-cleanup every 5 minutes)
- Credential rotation: On authorization, old clientSecret is invalidated
- One device per site: New authorization revokes previous device

### Global Error Reporting

- Requires valid JWT token (obtained via Basic Auth)
- Errors are fire-and-forget (never retry to avoid infinite loops)
- Sensitive data should not be included in error messages or metadata
- All communication uses HTTPS (enforced by default)

---

## Troubleshooting

### Device Authorization Issues

| Problem | Cause | Solution |
|---------|-------|----------|
| `invalid_grant` | Code not found | Restart authorization flow |
| `expired_token` | 15 min TTL exceeded | Restart flow, act faster |
| `access_denied` | User clicked "Deny" | Contact account owner |
| `slow_down` responses | Polling too fast | Wait longer between requests |

### Global Error Reporting Issues

| Problem | Cause | Solution |
|---------|-------|----------|
| `403 Forbidden` | Invalid JWT | Refresh JWT token |
| `400 Bad Request` | Validation failed | Check type/message length limits |
| `401 Unauthorized` | Missing/expired token | Re-authenticate to get new JWT |

---

## Examples

### Complete Device Authorization Example

```rust
use data_exporter::auth::device_flow::{ClientMetadata, DeviceFlowClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = DeviceFlowClient::new(
        "https://dev.dfm.bitbi.io".to_string(),
        true, // https_only
    )?;

    // Create metadata
    let metadata = ClientMetadata {
        device_name: "Warehouse Scanner #1".to_string(),
        device_type: Some("raspberry-pi".to_string()),
        firmware_version: Some("2.1.0".to_string()),
    };

    // Run complete flow
    let (credentials, auth_response) = client.run_device_flow(metadata).await?;

    println!("Authorized! Domain: {}", credentials.domain);
    println!("Client Secret: {}", credentials.client_secret);

    Ok(())
}
```

### Global Error Reporting Example

```rust
use data_exporter::error::{GlobalErrorLogger, ErrorReporter};
use data_exporter::auth::TokenManager;
use data_exporter::models::{Config, ErrorSeverity, GlobalErrorReport};
use std::sync::Arc;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize components
    let config = Arc::new(Config::from_file(Path::new("config.toml"))?);
    let reporter = Arc::new(ErrorReporter::new(config.api.https_only)?);
    let token_manager = Arc::new(TokenManager::new(&config)?);

    // Create logger
    let logger = GlobalErrorLogger::new(reporter, token_manager, config);

    // Report startup
    logger.info(
        "STARTUP_SUCCESS".to_string(),
        "Data Exporter started successfully".to_string(),
        None,
    ).await?;

    // Report error with metadata
    let mut metadata = HashMap::new();
    metadata.insert("version".to_string(), serde_json::Value::String("1.0.0".to_string()));

    logger.critical(
        "STARTUP_ERROR".to_string(),
        "Failed to initialize database connection pool".to_string(),
        Some(metadata),
    ).await?;

    Ok(())
}
```

---

## References

- [RFC 8628: OAuth 2.0 Device Authorization Grant](https://datatracker.ietf.org/doc/html/rfc8628)
- [Client Integration Guide](client-integration.md) - Full API documentation
- [Source Code](src/) - Implementation details

---

## Version

**Version**: 1.0.0
**Date**: 2026-01-12
**Status**: Production Ready
