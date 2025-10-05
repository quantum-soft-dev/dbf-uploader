# Data Model: Data Exporter Service

**Date**: 2025-10-05
**Source**: Derived from feature spec requirements and clarifications

## Core Entities

### Configuration

Represents the service configuration stored in TOML format at `C:\Program Files\data-exporter\config.toml`.

**Fields**:
- `scheduler.crontab`: String - Cron expression (minute hour day month day_of_week)
  - Default: `"0 8,12,16,18 * * *"`
  - Validation: Must parse as valid Unix cron expression
- `src.source_dir`: String - Path to directory containing DBF files
  - Default: `"C:\\Program Files\\abc\\def"`
  - Validation: Must be valid Windows path
- `credential.username`: String - Username for API authentication
  - Required: Yes
  - Validation: Non-empty
- `credential.password`: String - Password for API authentication
  - Required: Yes
  - Validation: Non-empty
  - Storage: Plain text (restricted file permissions)
- `api.base_url`: String - Base URL for API server
  - Default: Built into code (e.g., `"https://api.example.com"`)
  - Validation: Must be valid HTTPS URL
- `encoding.dbf_encoding`: String - Fallback DBF encoding
  - Default: `"CP866"`
  - Allowed values: `"CP866"`, `"WINDOWS-1251"`, `"UTF-8"`
  - Usage: Applied when auto-detection from DBF header fails

**Lifecycle**:
- Created: During service installation
- Modified: Manually by administrators
- Loaded: At service start and at the beginning of each scheduled run
- Validated: On load (before applying changes)

**Relationships**:
- Used by: Scheduler, Auth Client, File Processor

---

### JWT Token

Represents authentication token for API access. Stored in memory only.

**Fields**:
- `token`: String - JWT token value
  - Format: Standard JWT (header.payload.signature)
  - Validation: Must be non-empty
- `expires_in`: u64 - Token lifetime in seconds
  - Typical value: 86400 (24 hours)
  - Usage: For automatic renewal
- `obtained_at`: Timestamp - When token was retrieved
  - Usage: Calculate expiration time

**Lifecycle**:
- Created: During installation (initial validation) and at scheduled run start (if expired)
- Stored: In memory only (never persisted to disk)
- Validated: Before each scheduled run
- Renewed: Automatically when expired

**State Transitions**:
1. None → Obtaining (request to `/api/auth/token`)
2. Obtaining → Valid (successful response)
3. Obtaining → Failed (authentication error)
4. Valid → Expired (time-based expiration)
5. Expired → Obtaining (renewal request)

**Relationships**:
- Obtained from: Auth API endpoint
- Used by: File Uploader, Error Reporter

---

### DBF File

Represents a dBase database file to be processed.

**Fields**:
- `path`: PathBuf - Full file system path
  - Example: `C:\Program Files\abc\def\subdir\data.dbf`
  - Validation: Must exist, have `.dbf` extension
- `relative_path`: String - Path relative to source_dir
  - Example: `subdir\data.dbf`
  - Usage: For generating compressed filename
- `encoding`: Option<Encoding> - Detected or fallback encoding
  - Detected from: DBF header code page marker
  - Fallback: From config.encoding.dbf_encoding
  - Values: CP866, Windows-1251, UTF-8
- `status`: ProcessingStatus - Current processing state
  - Values: Pending, Locked, Converting, Compressing, Uploading, Completed, Failed

**Processing Status States**:
- `Pending`: File discovered, not yet processed
- `Locked`: File locked/in-use, deferred to end of batch
- `Converting`: DBF → CSV conversion in progress
- `Compressing`: CSV → gzip compression in progress
- `Uploading`: Upload to server in progress
- `Completed`: Successfully processed
- `Failed`: Error occurred (reported to server)

**Lifecycle**:
1. Discovered during directory scan
2. Converted to CSV (with encoding conversion to UTF-8)
3. Compressed to gzip
4. Uploaded to server
5. CSV file deleted (source DBF preserved)

**Relationships**:
- Scanned by: File Scanner
- Converted by: DBF Converter
- Compressed by: Gzip Compressor
- Uploaded by: File Uploader

---

### CSV File (Intermediate)

Temporary file created during processing, deleted after upload.

**Fields**:
- `path`: PathBuf - Temporary file path
  - Location: Same directory as DBF or temp directory
  - Naming: Generated from DBF relative path
- `encoding`: UTF-8 (always)
- `separator`: Comma (`,`)
- `quote_char`: Double quote (`"`)
- `escape_style`: RFC 4180 (double quotes: `""`)
- `has_headers`: bool - Always `true` (field names from DBF)

**Lifecycle**:
- Created: After DBF conversion
- Compressed: To gzip format
- Deleted: After upload attempt (success or failure)

**Relationships**:
- Created from: DBF File
- Compressed to: Compressed Archive
- Deleted by: Cleanup Handler

---

### Compressed Archive

Gzip-compressed CSV file ready for upload.

**Fields**:
- `path`: PathBuf - Path to gzip file
- `original_name`: String - Name derived from DBF relative path
  - Format: Path separators (`\`) converted to underscores (`_`)
  - Example: `subdir\data.dbf` → `subdir_data.csv.gz`
  - Validation: Must be unique within batch
- `size`: u64 - File size in bytes
  - Usage: For upload progress, logging
- `compression_level`: u8 - Gzip compression level
  - Default: 6 (Compression::default())

**Lifecycle**:
- Created: After CSV compression
- Uploaded: To API server
- Deleted: After CSV cleanup (happens after upload regardless of success)

**Relationships**:
- Created from: CSV File
- Uploaded by: File Uploader

---

### Batch

Represents a single scheduled execution processing multiple files.

**Fields**:
- `batch_id`: UUID - Unique identifier for this batch
  - Usage: Correlation in logs, error reports
- `started_at`: Timestamp - When batch processing began
- `config_snapshot`: Configuration - Config loaded at batch start
  - Immutable: Config changes don't affect in-progress batch
- `files`: Vec<DBF File> - List of files to process
  - Discovered: Via recursive directory scan
- `locked_files`: Vec<DBF File> - Files deferred due to locks
  - Retry: At end of batch, once
- `processed_count`: usize - Successfully processed files
- `failed_count`: usize - Files that encountered errors
- `status`: BatchStatus - Current batch state

**Batch Status States**:
- `Scanning`: Discovering files in source directory
- `Processing`: Converting/compressing/uploading files
- `RetryingLocked`: Retrying previously locked files
- `Completed`: All files processed (or skipped)
- `Aborted`: Batch cancelled (e.g., service stop)

**Lifecycle**:
1. Created at scheduled run time
2. Scans source directory for DBF files
3. Processes each file sequentially
4. Defers locked files to end
5. Retries locked files once
6. Completes (regardless of individual file success/failure)

**Relationships**:
- Triggered by: Scheduler
- Uses: Configuration snapshot
- Processes: Multiple DBF Files
- Reports: Errors to server

---

### Error Report

Structured error information sent to API server.

**Fields**:
- `filename`: String - Name of file that caused error
  - Example: `"subdir\data.dbf"`
- `error_type`: String - Classification of error
  - Examples: `"FileReadError"`, `"EncodingError"`, `"ConversionError"`, `"CompressionError"`, `"UploadError"`, `"DiskFullError"`, `"DirectoryInaccessible"`, `"AuthenticationError"`
- `message`: String - Human-readable error description
  - Include: Error details, context, operation that failed
- `timestamp`: String (ISO 8601) - When error occurred
  - Format: `"2025-10-05T14:30:00Z"`
- `client_version`: String - Version of data exporter service
  - Example: `"1.0.0"`
  - Usage: Server-side diagnostics, compatibility

**JSON Structure**:
```json
{
  "filename": "subdir\\data.dbf",
  "error_type": "FileReadError",
  "message": "Failed to read DBF file: Permission denied",
  "timestamp": "2025-10-05T14:30:00Z",
  "client_version": "1.0.0"
}
```

**Lifecycle**:
- Created: When error occurs during processing
- Sent: To `/api/errors/report` endpoint with JWT authentication
- Fallback: Written to local `error.log` if server unavailable

**Relationships**:
- Created by: Error Handler
- Sent by: Error Reporter
- Associated with: Batch, DBF File

---

### Local Error Log (Fallback)

Plain text log file used when server is unavailable.

**Fields** (per log entry):
- `timestamp`: ISO 8601 timestamp
- `level`: String - Log level (`"ERROR"`, `"WARN"`)
- `message`: String - Error message with full context
- `error_details`: String - Full error chain

**Format**:
```
[2025-10-05T14:30:00Z] ERROR: Failed to send error report to server: Connection refused
  Context: Processing file subdir\data.dbf in batch abc-123
  Original error: Failed to read DBF file: Permission denied
```

**Lifecycle**:
- Created: On first fallback error
- Appended: When server communication fails
- Managed: By system administrator (manual cleanup)

**Location**: `C:\Program Files\data-exporter\error.log`

**Relationships**:
- Written by: Fallback Logger
- Triggered when: Error Reporter cannot reach server

---

## Entity Relationships Diagram

```
┌─────────────┐
│Configuration│◄──────┐
└─────────────┘       │
       │              │
       │ loads        │ reads
       │              │
       ▼              │
┌─────────────┐   ┌────────┐
│  Scheduler  │──►│ Batch  │
└─────────────┘   └────────┘
                      │
                      │ scans for
                      ▼
                  ┌─────────┐
                  │DBF File │
                  └─────────┘
                      │
                      │ converts to
                      ▼
                  ┌─────────┐
                  │CSV File │
                  └─────────┘
                      │
                      │ compresses to
                      ▼
              ┌────────────────┐
              │Compressed      │
              │Archive         │
              └────────────────┘
                      │
                      │ uploads using
                      ▼
              ┌────────────────┐
              │   JWT Token    │
              └────────────────┘

    Error Flow:
    ┌─────────┐ creates ┌──────────────┐
    │DBF File │────────►│ Error Report │
    │ (failed)│         └──────────────┘
    └─────────┘                │
                               │ sends to server
                               │ OR
                               │ writes to
                               ▼
                      ┌──────────────────┐
                      │Local Error Log   │
                      │(fallback)        │
                      └──────────────────┘
```

---

## Validation Rules

### Configuration Validation
- `crontab`: Must parse as valid Unix cron expression (5 fields)
- `source_dir`: Must be absolute Windows path, directory must exist
- `username`, `password`: Must be non-empty strings
- `base_url`: Must start with `https://`
- `dbf_encoding`: Must be one of `CP866`, `WINDOWS-1251`, `UTF-8`

### File Processing Validation
- DBF files: Must have `.dbf` extension (case-insensitive)
- Relative path encoding: Must not exceed MAX_PATH on Windows (260 characters after encoding)
- CSV output: Must be valid UTF-8
- Gzip output: Must have valid gzip header

### API Request Validation
- JWT token: Must be present and non-expired before API calls
- Error report: All required fields must be present
- File upload: Must include gzip file data

---

## State Machine: Batch Processing

```
        [Scheduled Run Triggered]
                  │
                  ▼
         ┌────────────────┐
         │   Scanning     │
         └────────────────┘
                  │
                  │ files discovered
                  ▼
         ┌────────────────┐
    ┌───►│  Processing    │───┐ file locked
    │    └────────────────┘   │
    │            │             ▼
    │            │     [Defer to locked_files]
    │            │
    │   all files processed
    │            │
    │            ▼
    │   ┌────────────────┐
    │   │ RetryingLocked │
    │   └────────────────┘
    │            │
    │   locked files still locked
    │            │
    │            ▼
    │    [Skip locked files]
    │            │
    │            │ all retries complete
    │            ▼
    └───  ┌────────────────┐
          │   Completed    │
          └────────────────┘
```

---

## Data Persistence

### In-Memory Only
- JWT Token (security requirement)
- Batch state (ephemeral per run)
- DBF File processing status (ephemeral per batch)

### File System
- Configuration (TOML at `C:\Program Files\data-exporter\config.toml`)
- Source DBF files (read-only, never deleted)
- Temporary CSV files (deleted after upload)
- Temporary gzip files (deleted after upload)
- Local error log (fallback, append-only)

### No Database
This service does not require a database. All state is either transient (in-memory) or file-based.

---

## Encoding Handling Flow

```
┌────────────┐
│  DBF File  │
│ (CP866 or  │
│ Win-1251)  │
└────────────┘
      │
      │ 1. Read with detected/fallback encoding
      ▼
┌────────────┐
│ Rust String│
│  (UTF-8)   │ ← Automatic conversion by dbase crate
└────────────┘
      │
      │ 2. Write to CSV
      ▼
┌────────────┐
│  CSV File  │
│  (UTF-8)   │
└────────────┘
      │
      │ 3. Compress
      ▼
┌────────────┐
│ Gzip File  │
│ (binary)   │
└────────────┘
```

**Key Points**:
- DBF reader handles encoding conversion automatically
- Rust strings are UTF-8, so CSV writer outputs UTF-8 automatically
- No manual encoding conversion needed if using correct dbase features

---

**Data Model Complete**: All entities, relationships, validation rules, and state machines defined. Ready for contract generation.
