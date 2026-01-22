# Data Exporter - Architecture Documentation

## Overview

Data Exporter is a Windows service application designed to automatically export DBF files to CSV format, compress them, and upload them to a cloud server. The project uses a **Cargo workspace architecture** with separate crates for different responsibilities.

## Workspace Structure

```
dbf-uploader/
├── common/              # Shared library (models, auth, error handling)
├── service/             # Windows service implementation
├── configurator/        # GUI configuration tool
├── src/                 # Legacy monolithic code (being phased out)
└── Cargo.toml          # Workspace configuration
```

### Crates

#### 1. **common** - Shared Library
Location: `common/`

**Purpose**: Provides shared functionality used by both the service and configurator.

**Key Modules**:
- `models/` - Data structures (Config, CredentialConfig, ApiConfig, etc.)
- `auth/` - Authentication clients
  - `device_flow.rs` - OAuth 2.0 Device Authorization Grant (RFC 8628)
  - `auth_client.rs` - HTTP client with Basic Auth
- `error/` - Error types and Result aliases

**Dependencies**:
- serde, serde_json, toml (serialization)
- reqwest (HTTP client)
- thiserror, anyhow (error handling)
- chrono, uuid (utilities)

#### 2. **service** - Windows Service
Location: `service/`

**Purpose**: Main Windows service that runs scheduled batch operations.

**Key Modules**:
- `service/windows_service.rs` - Windows service implementation
- `service/batch_scheduler.rs` - Cron-based scheduling
- `main.rs` - Service entry point

**Features**:
- Runs as a Windows service
- Cron-based scheduling (tokio-cron-scheduler)
- DBF file processing and upload
- Configuration retry logic for network shares
- Graceful shutdown

**Dependencies**:
- windows-service (Windows service API)
- tokio-cron-scheduler (scheduling)
- dbase, csv, flate2 (file processing)
- common (shared library)

#### 3. **configurator** - GUI Configuration Tool
Location: `configurator/`

**Purpose**: User-friendly GUI for configuring the service and managing it.

**Key Modules**:
- `config_manager.rs` - Config file I/O
- `service_manager.rs` - Windows service control (sc.exe wrapper)
- `ui/app.rs` - Main GUI window

**Features**:
- **Authentication Tab**: Device Authorization Flow setup
- **Settings Tab**: Server URL and source directory configuration
- **Schedule Tab**: Cron expression configuration
- **Service Tab**: Install/start/stop/uninstall service
- **Status Tab**: Service status display

**Dependencies**:
- native-windows-gui (GUI framework)
- tokio (async runtime for device flow)
- anyhow (error handling)
- common (shared library)

## Authentication Flow

### Device Authorization Grant (RFC 8628)

```
┌──────────────┐                                ┌──────────────┐
│              │  1. POST /api/v1/device/       │              │
│              │     authorize                  │              │
│ Configurator │─────────────────────────────▶ │ API Server   │
│              │  {siteName, siteDescription}   │              │
│              │◀─────────────────────────────  │              │
│              │  {deviceCode, userCode,        │              │
│              │   verificationUri, ...}        │              │
└──────────────┘                                └──────────────┘
       │                                               ▲
       │                                               │
       │  2. Display instructions:                    │
       │     "Go to <verificationUri>"                │
       │     "Enter code: <userCode>"                 │
       │                                               │
       │                                               │
       │  ┌───────────────────────────────────────┐   │
       │  │ User opens browser and authorizes     │   │
       │  └───────────────────────────────────────┘   │
       │                                               │
       │  3. Poll: POST /api/v1/device/token          │
       │     {deviceCode}                             │
       └──────────────────────────────────────────────┘
       │
       │  4. Response (when authorized):
       │     {siteId, domain, clientSecret, apiBaseUrl}
       │
       ▼
   Save to config.toml
```

## Service Management

### Installation Flow

```
┌──────────────┐
│ User runs    │
│ Configurator │
└──────┬───────┘
       │
       │ 1. Configure authentication (Device Flow)
       │    → Saves credentials to config.toml
       │
       │ 2. Configure settings
       │    → Updates config.toml (source_dir, etc.)
       │
       │ 3. Click "Install Service"
       │    ↓
       ├─▶ Check: data_exporter.exe exists in C:\Program Files\data-exporter\
       ├─▶ Check: config.toml exists
       ├─▶ Run: sc.exe create data-exporter binPath=... start=auto
       └─▶ Service registered in Windows Service Manager
```

### Service Operations

```
┌──────────────┐
│ Service      │                    ┌─────────────┐
│ Manager      │───── sc.exe ─────▶ │   Windows   │
│ (GUI)        │      commands      │   Service   │
└──────────────┘                    │   Control   │
                                    │   Manager   │
Commands:                           └─────────────┘
- sc.exe create <name> ...    (Install)
- sc.exe start <name>         (Start)
- sc.exe stop <name>          (Stop)
- sc.exe delete <name>        (Uninstall)
- sc.exe query <name>         (Status)
```

## Configuration Management

### Config File: `config.toml`

```toml
[scheduler]
crontab = "0 0 8,12,16,18 * * *"  # 6-field cron expression

[src]
source_dir = "C:\\data"
include_patterns = ["*.dbf"]      # Optional whitelist
exclude_patterns = ["temp_*.dbf"] # Optional blacklist

[credential]
# Traditional auth (deprecated):
account = ""
username = ""
password = ""

# Device flow auth (recommended):
[credential.device]
site_id = "550e8400-e29b-41d4-a716-446655440000"
domain = "c823d8b8-0e6f-4242-a350-d6ef335ab4e8_warehouse-01"
client_secret = "cs_secret123"

[api]
base_url = "https://api.dataforge.com"
https_only = true

[encoding]
dbf_encoding = "CP866"  # Fallback encoding
```

### Configuration Locations

- **Development**: Current working directory
- **Production (Service)**: `C:\Program Files\data-exporter\config.toml`

## Scheduling

### Cron Expression Format

Uses 6-field format (tokio-cron-scheduler):
```
┌─────── second (0-59)
│ ┌───── minute (0-59)
│ │ ┌─── hour (0-23)
│ │ │ ┌─ day of month (1-31)
│ │ │ │ ┌ month (1-12)
│ │ │ │ │ ┌ day of week (0-7, 0 and 7 = Sunday)
│ │ │ │ │ │
* * * * * *
```

**Examples**:
- `0 0 8,12,16,18 * * *` - Run at 8am, 12pm, 4pm, 6pm daily
- `0 0 */4 * * *` - Run every 4 hours
- `0 30 9 * * *` - Run at 9:30am daily

## Data Flow

```
┌─────────────┐
│ DBF Files   │
│ (source_dir)│
└──────┬──────┘
       │
       │ Service reads on schedule
       ▼
┌─────────────────────────┐
│ Batch Processing        │
│ 1. Read DBF             │
│ 2. Convert to CSV       │
│ 3. Compress (gzip)      │
│ 4. Upload via HTTP      │
└──────┬──────────────────┘
       │
       │ POST /api/v1/batches/{batchId}/records
       ▼
┌─────────────┐
│ API Server  │
│ (cloud)     │
└─────────────┘
```

## Error Handling

### Configuration Loading with Retry

The service implements exponential backoff for config loading:

```
Retry Schedule (for "Source directory does not exist"):
- Attempt 1: Wait 1 minute
- Attempt 2: Wait 2 minutes
- Attempt 3: Wait 4 minutes
- Attempt 4: Wait 8 minutes
- Attempt 5: Wait 16 minutes
- Attempt 6+: Wait 60 minutes (hourly)

Other config errors: Fail immediately
```

This handles cases where network shares aren't immediately available at service startup.

## Security

### Credential Protection

1. **Device Flow Credentials**:
   - domain (UUID-based)
   - client_secret (randomly generated)
   - Stored in config.toml with restricted ACL

2. **File Permissions**:
   - config.toml: SYSTEM and Administrators only
   - Applied via icacls during installation

3. **HTTPS Enforcement**:
   - Default: `https_only = true`
   - Can be disabled for local testing only

## Build System

### Workspace Dependencies

All dependencies are managed at the workspace level in the root `Cargo.toml`:

```toml
[workspace.dependencies]
common = { path = "common" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
# ... etc
```

Each crate references them:

```toml
[dependencies]
common = { workspace = true }
serde = { workspace = true }
```

### Build Targets

```bash
# Build service
cargo build --release --bin data_exporter

# Build configurator
cargo build --release --bin configurator

# Build all
cargo build --release
```

## Deployment

### Manual Deployment

1. Build release binaries
2. Copy `data_exporter.exe` to `C:\Program Files\data-exporter\`
3. Run `configurator.exe`
4. Configure via GUI
5. Install service via GUI

### Future: WiX Toolset MSI

Planned for future releases:
- MSI installer using WiX Toolset
- Automated file placement
- Automatic service registration
- Start Menu shortcuts

## Logging

- **Service**: Uses `tracing` with file appender
- **Configurator**: GUI dialogs for user feedback
- **Location**: Service logs to Windows Event Log and files

## Future Improvements

1. **Installer**: Implement WiX-based MSI installer
2. **GUI**: Add log viewer to configurator
3. **Monitoring**: Add metrics and health checks
4. **Updates**: Auto-update mechanism
5. **Multi-tenancy**: Support multiple sites per service instance
