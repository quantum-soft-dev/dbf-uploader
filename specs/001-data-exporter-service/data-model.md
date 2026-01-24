# Data Model: Data Exporter Windows Service

**Branch**: `001-data-exporter-service` | **Date**: 2026-01-24

## Overview

This document defines the core data entities, their relationships, validation rules, and state transitions for the Data Exporter Windows Service.

---

## Entity Diagram

```
┌─────────────────┐      1:N      ┌─────────────────┐
│     Config      │───────────────│      Batch      │
└─────────────────┘               └─────────────────┘
        │                                 │
        │                                 │ 1:N
        │                           ┌─────┴─────┐
        │                           │           │
        │                     ┌─────┴───┐ ┌─────┴───────┐
        │                     │ DbfFile │ │ ErrorReport │
        │                     └─────────┘ └─────────────┘
        │
        └── SchedulerConfig
        └── SourceConfig
        └── CredentialConfig
        └── ApiConfig
        └── EncodingConfig
```

---

## Entities

### 1. Config

**Location**: `common/src/models/config.rs`

**Purpose**: Root configuration loaded from TOML file at service startup.

```rust
pub struct Config {
    pub scheduler: SchedulerConfig,
    pub src: SourceConfig,
    pub credential: CredentialConfig,
    pub api: ApiConfig,
    pub encoding: EncodingConfig,
}
```

#### Sub-entities

| Entity | Fields | Validation |
|--------|--------|------------|
| `SchedulerConfig` | `crontab: String` | Valid 5-field cron expression |
| `SourceConfig` | `source_dir: PathBuf`, `include_patterns: Option<Vec<String>>`, `exclude_patterns: Option<Vec<String>>` | Directory must exist (at batch start) |
| `CredentialConfig` | `account: String`, `username: String`, `password: String`, `device: Option<DeviceCredentials>` | Non-empty strings |
| `ApiConfig` | `base_url: String`, `https_only: bool` | Valid URL, HTTPS required if `https_only=true` |
| `EncodingConfig` | `dbf_encoding: String` | One of: CP866, Windows-1251, Windows-1255, ISO-8859-8, UTF-8 |

#### Validation Rules

- **FR-SVC-005**: Config loaded from TOML at startup
- **FR-SCH-001**: Cron expression must be valid 5-field format
- **Edge Case**: Invalid cron expression causes immediate startup failure

---

### 2. Batch

**Location**: `common/src/models/batch.rs`

**Purpose**: Represents a single scheduled execution cycle.

```rust
pub struct Batch {
    pub batch_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub config_snapshot: Config,
    pub files: Vec<PathBuf>,
    pub locked_files: Vec<PathBuf>,
    pub processed_count: usize,
    pub failed_count: usize,
    pub status: BatchStatus,
}
```

#### State Machine: BatchStatus

```
                 ┌──────────────┐
                 │   Scanning   │ ◄── Initial state
                 └──────┬───────┘
                        │
              found files/empty
                        │
                        ▼
                 ┌──────────────┐
                 │  Processing  │
                 └──────┬───────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
   locked files    all done      critical error
        │               │               │
        ▼               ▼               ▼
┌───────────────┐ ┌───────────┐ ┌───────────┐
│RetryingLocked │ │ Completed │ │  Aborted  │
└───────┬───────┘ └───────────┘ └───────────┘
        │
   VSS retry
        │
        ├──────► Completed (all succeed)
        └──────► Aborted (critical error)
```

#### Transitions

| From | To | Trigger |
|------|----|---------|
| Scanning | Processing | Files found (or empty) |
| Processing | RetryingLocked | Locked files deferred |
| Processing | Completed | All files processed |
| Processing | Aborted | Critical error |
| RetryingLocked | Completed | All locked files resolved |
| RetryingLocked | Aborted | Critical error |

#### Methods

| Method | Description |
|--------|-------------|
| `new(config)` | Create batch in Scanning state |
| `add_file(path)` | Add file to processing queue |
| `defer_locked_file(path)` | Move file to locked queue |
| `mark_completed()` | Increment processed count |
| `mark_failed()` | Increment failed count |
| `total_files()` | Return total file count |

---

### 3. DbfFile

**Location**: `common/src/models/dbf_file.rs`

**Purpose**: Represents a source DBF file for processing.

```rust
pub struct DbfFile {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub encoding: Option<Encoding>,
    pub status: FileProcessingStatus,
}
```

#### State Machine: FileProcessingStatus

```
┌─────────┐    ┌───────────┐    ┌────────────┐    ┌───────────┐    ┌───────────┐
│ Pending │───►│Converting │───►│Compressing │───►│ Uploading │───►│ Completed │
└─────────┘    └───────────┘    └────────────┘    └───────────┘    └───────────┘
     │              │                 │                 │
     │              │                 │                 │
     ▼              ▼                 ▼                 ▼
┌─────────┐   ┌─────────┐       ┌─────────┐       ┌─────────┐
│ Locked  │   │ Failed  │       │ Failed  │       │ Failed  │
└─────────┘   └─────────┘       └─────────┘       └─────────┘
```

#### Encoding Enum

```rust
pub enum Encoding {
    CP866,
    Windows1251,
    Windows1255,
    ISO8859_8,
    UTF8,
}
```

**Detection Logic**:
1. Check DBF header Language Driver ID (LDID)
2. If LDID present, map to Encoding
3. If absent, use `EncodingConfig.dbf_encoding` fallback

---

### 4. ErrorReport

**Location**: `common/src/models/error_report.rs`

**Purpose**: Represents a processing error for server reporting.

#### Batch Error Report

```rust
pub struct ErrorReport {
    pub error_type: String,      // Classification (max 100 chars)
    pub message: String,         // Human-readable (max 1000 chars)
    pub error_details: Option<String>,  // Stack trace / cause chain
    pub metadata: Option<HashMap<String, Value>>,
}
```

#### Global Error Report

```rust
pub struct GlobalErrorReport {
    pub error_type: String,      // Classification (max 100 chars)
    pub message: String,         // Human-readable (max 10000 chars)
    pub severity: ErrorSeverity,
    pub metadata: Option<HashMap<String, Value>>,  // max 20 keys, 10KB total
}
```

#### Error Severity Levels

| Level | Description | Action |
|-------|-------------|--------|
| `Critical` | System failure, data loss risk | Abort batch, immediate alert |
| `Error` | Operation failed | Continue batch, report |
| `Warning` | Degraded performance, recoverable | Continue batch, log |
| `Info` | Informational | Log only |

#### Validation Methods

- `truncate_message(max_len)` - Ensure message fits API limits
- `truncate_error_type(max_len)` - Ensure type fits API limits
- `validate_and_truncate()` - Apply all API constraints

---

## Processing Data Types

### ProcessingData

**Location**: `service/src/processor/mod.rs`

```rust
pub enum ProcessingData {
    InMemory(Vec<u8>),      // CSV data <= 10 MB
    TempFile(PathBuf),      // CSV data > 10 MB
}
```

**Thresholds**:
- `MAX_IN_MEMORY_SIZE = 10 * 1024 * 1024` (10 MB)

---

## Relationships

| Parent | Child | Cardinality | Description |
|--------|-------|-------------|-------------|
| Config | Batch | 1:N | Config snapshot taken per batch |
| Batch | DbfFile | 1:N | Batch contains files to process |
| Batch | ErrorReport | 1:N | Errors reported per batch |

---

## API Data Transfer Objects

### BatchStartRequest

```json
{
  "client_version": "string",
  "source_dir": "string",
  "file_count": 0
}
```

### BatchCompleteRequest

```json
{
  "status": "success|with-warnings|failed",
  "processed_count": 0,
  "failed_count": 0,
  "duration_ms": 0
}
```

### FileUploadRequest

- Multipart form with `file` field
- File must be gzipped CSV
- Filename format: `{relative_path}.csv.gz`

### ErrorReportRequest

```json
{
  "type": "string",
  "message": "string",
  "error_details": "string|null",
  "metadata": {}
}
```

---

## Next Steps

1. Generate OpenAPI contracts in `contracts/` directory
2. Create quickstart guide with TDD examples
3. Generate implementation tasks aligned with TDD workflow
