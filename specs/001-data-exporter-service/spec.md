# Feature Specification: Data Exporter Windows Service

**Feature Branch**: `001-data-exporter-service`
**Created**: 2026-01-24
**Status**: Draft
**Input**: PRD for Data Exporter Service - Windows background service for automatic DBF file export to CSV with server upload

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Automated Scheduled Data Export (Priority: P1)

As an IT administrator, I want the service to automatically export DBF files on a configurable schedule so that data is regularly synchronized with the remote server without manual intervention.

**Why this priority**: Core functionality - without scheduled execution, the service provides no value. This is the fundamental capability that enables all other features.

**Independent Test**: Can be fully tested by configuring a cron schedule and verifying that batch processing triggers at the expected times, processes files, and uploads them to the server.

**Acceptance Scenarios**:

1. **Given** the service is installed and running with a valid cron schedule, **When** the scheduled time arrives, **Then** the service initiates batch processing automatically
2. **Given** a batch is already running, **When** the next scheduled time arrives, **Then** the service skips that execution and logs a warning
3. **Given** the service is stopped, **When** a scheduled time passes, **Then** no processing occurs and the service resumes normal scheduling when restarted

---

### User Story 2 - DBF to CSV Conversion with Encoding Support (Priority: P1)

As a data administrator, I want DBF files to be automatically converted to CSV format with proper encoding detection so that legacy database files can be integrated with modern systems.

**Why this priority**: Essential for data integrity - without correct encoding handling, exported data would be corrupted and unusable.

**Independent Test**: Can be tested by placing DBF files with various encodings (CP866, Windows-1251, Windows-1255) in the source directory and verifying the output CSV contains correctly decoded UTF-8 text.

**Acceptance Scenarios**:

1. **Given** a DBF file with embedded encoding information in the header, **When** the file is processed, **Then** the system detects and uses the correct encoding for conversion
2. **Given** a DBF file without encoding information, **When** the file is processed, **Then** the system uses the configured fallback encoding
3. **Given** a DBF file with corrupted records, **When** the file is processed, **Then** corrupted records are skipped and logged as warnings while valid records are exported

---

### User Story 3 - Reliable File Upload with Retry (Priority: P1)

As a system operator, I want files to be reliably uploaded to the server with automatic retry on failures so that transient network issues do not result in data loss.

**Why this priority**: Critical for data delivery - if uploads fail without retry, the entire export becomes unreliable and data synchronization fails.

**Independent Test**: Can be tested by simulating network failures during upload and verifying that the system retries and eventually succeeds when connectivity is restored.

**Acceptance Scenarios**:

1. **Given** a successful server connection, **When** a file is uploaded, **Then** the file is delivered and confirmed by the server
2. **Given** a temporary server error (5xx), **When** upload fails, **Then** the system retries up to 3 times with increasing delays (1s, 2s, 4s)
3. **Given** persistent network failure after all retries, **When** upload fails, **Then** the error is logged and reported, and batch continues with remaining files

---

### User Story 4 - Locked File Handling via VSS (Priority: P2)

As a data administrator, I want the service to access files that are locked by other applications so that active databases can be exported without interrupting business operations.

**Why this priority**: Important for real-world deployment where DBF files are often locked by legacy applications (FoxPro, dBase) during business hours.

**Independent Test**: Can be tested by opening a DBF file exclusively in another application and verifying the service successfully copies and processes it via shadow copy.

**Acceptance Scenarios**:

1. **Given** a DBF file locked by another process, **When** the service attempts to read it, **Then** the file is deferred to the end of the batch for VSS retry
2. **Given** a locked file and VSS is available, **When** VSS copy is performed, **Then** the file is successfully copied to a temp location and processed
3. **Given** VSS copy fails, **When** the file cannot be accessed, **Then** an error is logged as a warning and the batch continues

---

### User Story 5 - Comprehensive Error Logging and Reporting (Priority: P2)

As a system operator, I want all errors to be logged locally and reported to the server so that I can diagnose issues and receive alerts about service problems.

**Why this priority**: Essential for operations - without proper error visibility, problems go undetected and data synchronization silently fails.

**Independent Test**: Can be tested by inducing various error conditions and verifying logs are created locally and error reports are sent to the server.

**Acceptance Scenarios**:

1. **Given** a file processing error occurs, **When** the error is non-critical, **Then** the error is logged and the batch continues with other files
2. **Given** a critical error occurs (directory inaccessible, auth failure), **When** the batch cannot proceed, **Then** the batch is aborted and a critical error report is sent
3. **Given** the server is unreachable for error reporting, **When** an error occurs, **Then** the error is logged locally as a fallback
4. **Given** a panic occurs in the service, **When** the service crashes, **Then** a panic log is written to a fallback location before exit

---

### User Story 6 - Graceful Service Lifecycle Management (Priority: P2)

As an IT administrator, I want the service to start, stop, and recover gracefully so that system maintenance does not result in data corruption or lost work.

**Why this priority**: Important for reliability and maintainability in production environments.

**Independent Test**: Can be tested by issuing start/stop commands and verifying the service responds correctly and completes or cleanly aborts in-progress work.

**Acceptance Scenarios**:

1. **Given** the service receives a stop signal, **When** a batch is in progress, **Then** the service completes the current file and stops cleanly
2. **Given** the source directory is unavailable at startup, **When** the service starts, **Then** it retries loading configuration with exponential backoff
3. **Given** configuration becomes available after retries, **When** the next retry occurs, **Then** the service loads configuration and begins normal operation

---

### User Story 7 - File Filtering with Include/Exclude Patterns (Priority: P3)

As a data administrator, I want to configure which files are processed using patterns so that I can exclude temporary files or include only specific databases.

**Why this priority**: Nice-to-have for advanced use cases where selective file processing is needed.

**Independent Test**: Can be tested by configuring include/exclude patterns and verifying only matching files are processed.

**Acceptance Scenarios**:

1. **Given** include patterns are configured, **When** the directory is scanned, **Then** only files matching at least one include pattern are processed
2. **Given** exclude patterns are configured, **When** files match both include and exclude, **Then** excluded files are skipped
3. **Given** no patterns are configured, **When** the directory is scanned, **Then** all DBF files are processed

---

### Edge Cases

- What happens when the source directory contains no DBF files? (Empty batch is reported as complete)
- What happens when a file is too large to fit in memory? (System uses temporary file storage)
- What happens when disk space runs out during temp file creation? (Error is logged and file is skipped)
- What happens when the server returns 401 Unauthorized? (Token is refreshed and request is retried once)
- What happens when the configuration file is malformed? (Service fails immediately with descriptive error)
- What happens when cron expression is invalid? (Service fails at startup with configuration error)
- What happens when multiple batches are scheduled close together? (Batch lock prevents overlap, skipped runs are logged)

## Requirements *(mandatory)*

### Functional Requirements

**Service Lifecycle**
- **FR-SVC-001**: Service MUST register with Windows Service Control Manager under the name "data-exporter"
- **FR-SVC-002**: Service MUST support Start, Stop, and Interrogate commands
- **FR-SVC-003**: Service MUST perform graceful shutdown when receiving Stop signal
- **FR-SVC-004**: Service MUST write logs to a dedicated log directory with daily rotation
- **FR-SVC-005**: Service MUST load configuration from a TOML file at startup
- **FR-SVC-006**: Service MUST retry configuration loading with exponential backoff (1, 2, 4, 8, 16 min, then hourly) when source directory is unavailable

**Scheduling**
- **FR-SCH-001**: Service MUST support standard 5-field cron expressions for scheduling
- **FR-SCH-002**: Service MUST prevent concurrent batch execution (batch lock)
- **FR-SCH-003**: Service MUST reload configuration before each scheduled batch (hot-reload)

**Directory Scanning**
- **FR-SCN-001**: Service MUST recursively scan the configured source directory
- **FR-SCN-002**: Service MUST find all files with .dbf extension (case-insensitive)
- **FR-SCN-003**: Service MUST support include/exclude glob patterns for file filtering
- **FR-SCN-004**: Service MUST detect encoding from DBF file headers when available

**File Conversion**
- **FR-CNV-001**: Service MUST open files with shared read access to allow concurrent access
- **FR-CNV-002**: Service MUST support encoding conversion from CP866, Windows-1251, Windows-1255, ISO-8859-8, and UTF-8
- **FR-CNV-003**: Service MUST convert all output to UTF-8 encoded CSV
- **FR-CNV-004**: Service MUST skip corrupted records and log warnings while continuing processing

**Memory Management**
- **FR-MEM-001**: Service MUST process files in memory when data size is 10 MB or less
- **FR-MEM-002**: Service MUST use temporary files for data larger than 10 MB
- **FR-MEM-003**: Service MUST automatically clean up temporary files after processing

**Compression**
- **FR-CMP-001**: Service MUST compress CSV data using GZIP format before upload

**Upload**
- **FR-UPL-001**: Service MUST upload files using multipart form data with bearer token authentication
- **FR-UPL-002**: Service MUST retry failed uploads up to 3 times with exponential backoff (1s, 2s, 4s) for server errors and network failures
- **FR-UPL-003**: Service MUST support upload timeout of 5 minutes for large files
- **FR-UPL-004**: Service MUST NOT retry client errors (4xx except 401)

**VSS (Locked Files)**
- **FR-VSS-001**: Service MUST detect locked files by OS error codes 32 and 33
- **FR-VSS-002**: Service MUST defer locked files to end of batch for VSS retry
- **FR-VSS-003**: Service MUST create VSS copies in a temporary directory
- **FR-VSS-004**: Service MUST clean up VSS copies after processing

**Batch Management**
- **FR-BCH-001**: Service MUST register batch start with the server before processing
- **FR-BCH-002**: Service MUST report batch completion status (success, with-warnings, or failed)
- **FR-BCH-003**: Service MUST abort batch and report failure for critical errors

**Error Handling**
- **FR-ERR-001**: Service MUST classify errors as critical (abort batch) or warning (continue batch)
- **FR-ERR-002**: Service MUST send error reports to the server for all errors
- **FR-ERR-003**: Service MUST fall back to local file logging when server is unreachable
- **FR-ERR-004**: Service MUST include metadata (version, paths, details) in error reports

**Crash Recovery**
- **FR-LOG-001**: Service MUST log all initialization errors before exiting
- **FR-LOG-002**: Service MUST install a panic handler to capture and log crash information
- **FR-LOG-003**: Service MUST write panic logs to a fallback location when main log is unavailable
- **FR-LOG-004**: Service MUST set service status to Stopped on any fatal error

### Key Entities

- **Batch**: Represents a single scheduled execution cycle. Contains batch ID, start time, status (scanning/processing/retrying/completed/aborted), file counts (processed/failed/locked), and error summary.

- **DBF File**: Represents a source data file. Contains file path, relative path (for upload naming), detected encoding, and processing status.

- **Error Report**: Represents a processing error. Contains error type, message, severity (critical/error/warning/info), metadata, and optional batch association.

- **Configuration**: Represents service settings. Contains scheduler settings (cron), source settings (directory, patterns), API settings (URL, credentials), and encoding settings (fallback).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Service processes all accessible DBF files in the source directory within a single scheduled batch
- **SC-002**: 99% of scheduled batches complete without critical errors under normal operating conditions
- **SC-003**: Files locked by other applications are successfully processed via VSS in 95% of cases
- **SC-004**: Transient network failures are recovered automatically without operator intervention
- **SC-005**: All service crashes and errors are captured in logs (zero silent failures)
- **SC-006**: Service can be started, stopped, and restarted without data corruption
- **SC-007**: Configuration changes take effect without service restart (hot-reload on next batch)
- **SC-008**: Large files (over 10 MB) are processed without excessive memory consumption

## Assumptions

- Windows operating system (Windows 10+ or Windows Server 2016+) is the target platform
- Volume Shadow Copy Service (VSS) is available and enabled on the target system
- Network connectivity to the API server is generally available (transient failures expected)
- Source directory is on a local or network-mounted drive accessible by the service account
- DBF files follow standard dBase/FoxPro format with optional Language Driver ID in header
- API server endpoints follow the documented contract (auth, batch, upload, errors)
- Service runs with sufficient privileges to read source directory and write to log directory
- Clock synchronization is adequate for cron scheduling accuracy
