# Tasks: Data Exporter Service

**Input**: Design documents from `/specs/001-technical-specifications-data/`
**Prerequisites**: plan.md, research.md, data-model.md, contracts/, quickstart.md

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → Extract: Rust, windows-service, tokio, reqwest, dbase, etc.
2. Load design documents:
   → data-model.md: 8 entities (Configuration, JWT Token, DBF File, Batch, Error Report, etc.)
   → contracts/: 3 API contracts (auth, upload, error-report)
   → research.md: Technology decisions and crate selections
   → quickstart.md: 8 test scenarios
3. Generate tasks by category:
   → Setup: Cargo init, dependencies, project structure
   → Tests: Contract tests (3), integration tests (5)
   → Core: Models (8), services (auth, processor, scheduler)
   → Integration: Windows service, CLI, error handling
   → Polish: Logging, documentation, quickstart validation
4. Apply TDD rules:
   → Contract tests MUST be written and failing before implementation
   → Unit tests before module implementation
   → Integration tests after core functionality complete
5. Number tasks sequentially (T001-T045)
6. Mark [P] for parallel-executable tasks (different files, no dependencies)
7. Validate completeness: All contracts tested, all entities modeled
8. Return: SUCCESS (45 tasks ready for execution)
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no blocking dependencies)
- All paths are absolute or relative to repository root

## Path Conventions
- Single Rust project structure (src/, tests/ at repository root)
- Tests mirror source structure

---

## Phase 3.1: Project Setup & Foundation

- [ ] **T001** Initialize Cargo project with Rust 1.79.0, edition 2021, targeting Windows x86_64
  - Create `Cargo.toml` with package metadata
  - Set `name = "data-exporter"`, `version = "0.1.0"`
  - Configure `[[bin]]` section with `name = "data_exporter"`, `path = "src/main.rs"`

- [ ] **T002** Add all required dependencies to `Cargo.toml`
  - Add `windows-service = "0.7"` for Windows service integration
  - Add `tokio = { version = "1", features = ["full"] }` for async runtime
  - Add `tokio-cron-scheduler = "0.13"` for cron scheduling
  - Add `reqwest = { version = "0.11", features = ["multipart", "rustls-tls"], default-features = false }` for HTTP client
  - Add `dbase = { version = "0.6", features = ["yore"] }` for DBF parsing
  - Add `encoding_rs = "0.8"` for encoding support
  - Add `serde = { version = "1", features = ["derive"] }` for serialization
  - Add `toml = "0.8"` for config parsing
  - Add `csv = "1.3"` for CSV writing
  - Add `flate2 = "1.0"` for gzip compression
  - Add `thiserror = "1.0"` and `anyhow = "1.0"` for error handling
  - Add `tracing = "0.1"` and `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` for logging
  - Add `notify-debouncer-mini = "0.4"` for config file watching
  - Add `uuid = { version = "1", features = ["v4", "serde"] }` for batch IDs
  - Add `chrono = { version = "0.4", features = ["serde"] }` for timestamps

- [ ] **T003** [P] Create project directory structure in `src/`
  - Create `src/main.rs` (CLI entry point placeholder)
  - Create `src/lib.rs` (library exports placeholder)
  - Create `src/service/mod.rs`
  - Create `src/config/mod.rs`
  - Create `src/auth/mod.rs`
  - Create `src/processor/mod.rs`
  - Create `src/models/mod.rs`
  - Create `src/error/mod.rs`

- [ ] **T004** [P] Create test directory structure in `tests/`
  - Create `tests/contract/mod.rs`
  - Create `tests/integration/mod.rs`
  - Create `tests/unit/mod.rs`

- [ ] **T005** [P] Define custom error types in `src/error/mod.rs`
  - Use `thiserror` to define `ServiceError` enum
  - Add variants: `DbfParse`, `HttpRequest`, `Config`, `Schedule`, `Io`, `Auth`, `Encoding`, `Compression`, `Upload`, `DiskFull`
  - Implement `From` conversions for underlying error types
  - Export error types from module

---

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Contract Tests (API Endpoints)

- [ ] **T006** [P] Write contract test for Auth API in `tests/contract/auth_test.rs`
  - Test POST `/api/auth/token` with Basic auth
  - Assert 200 response with `token` and `expires_in` fields
  - Test 401 response for invalid credentials
  - Test 403 response for inactive subscription
  - Test 400 response for missing Authorization header
  - Use `reqwest` client with HTTPS-only enforcement
  - **Test MUST fail** (no auth client implementation yet)

- [ ] **T007** [P] Write contract test for Upload API in `tests/contract/upload_test.rs`
  - Test POST `/api/files/upload` with multipart/form-data
  - Assert JWT Bearer token in Authorization header
  - Test successful upload returns 200/201
  - Test 401 for invalid/expired token
  - Test 400 for missing file field
  - Test filename encoding (subdirs → underscores)
  - Use mock gzip file for upload
  - **Test MUST fail** (no uploader implementation yet)

- [ ] **T008** [P] Write contract test for Error Report API in `tests/contract/error_test.rs`
  - Test POST `/api/errors/report` with JSON body
  - Assert required fields: filename, error_type, message, timestamp, client_version
  - Test 200/204 successful receipt
  - Test all error_type values accepted
  - Test with and without JWT token (server may allow unauthenticated error reports)
  - **Test MUST fail** (no error reporter implementation yet)

### Integration Tests

- [ ] **T009** [P] Write integration test for service lifecycle in `tests/integration/service_lifecycle_test.rs`
  - Test install command: registers service, creates config, validates credentials
  - Test service start: initializes runtime, loads config, starts scheduler
  - Test service stop: graceful shutdown, cancels in-flight operations
  - Test uninstall command: stops service, unregisters, cleans up directory
  - **Test MUST fail** (no service implementation yet)

- [ ] **T010** [P] Write integration test for batch processing in `tests/integration/batch_processing_test.rs`
  - Create test DBF files in temp directory
  - Start scheduled batch execution
  - Assert all DBF files discovered recursively
  - Assert files converted to CSV (UTF-8)
  - Assert files compressed to gzip
  - Assert files uploaded to mock server
  - Assert CSV files deleted after upload
  - Assert source DBF files preserved
  - **Test MUST fail** (no processor implementation yet)

- [ ] **T011** [P] Write integration test for config reload in `tests/integration/config_reload_test.rs`
  - Start service with initial config (crontab = "*/5 * * * *")
  - Modify config.toml during operation (change crontab to "*/10 * * * *")
  - Assert config change detected by file watcher
  - Assert new config applied at START of next scheduled run (not mid-batch)
  - **Test MUST fail** (no config watcher implementation yet)

- [ ] **T012** [P] Write integration test for locked file retry in `tests/integration/locked_file_retry_test.rs`
  - Create test DBF file and lock it (simulate file in use)
  - Start batch processing
  - Assert locked file deferred to end of batch
  - Assert other files processed successfully
  - Assert locked file retried once at end
  - Assert file skipped if still locked after retry
  - Release lock and verify file processed on next batch
  - **Test MUST fail** (no locked file handling yet)

- [ ] **T013** [P] Write integration test for error handling in `tests/integration/error_handling_test.rs`
  - Create corrupted DBF file (invalid format)
  - Start batch processing
  - Assert error report sent to mock server
  - Assert processing continues with remaining files
  - Simulate network failure (mock server unavailable)
  - Assert error written to local error.log
  - **Test MUST fail** (no error handling implementation yet)

---

## Phase 3.3: Core Models (ONLY after tests are failing)

- [ ] **T014** [P] Implement Configuration model in `src/models/config.rs`
  - Define `Config` struct with `serde::Deserialize`
  - Add fields: `scheduler.crontab`, `src.source_dir`, `credential.username`, `credential.password`, `api.base_url`, `encoding.dbf_encoding`
  - Implement validation: crontab parses, source_dir exists, base_url starts with https://
  - Implement `Config::from_file(path)` to load and parse TOML
  - Implement `Config::default()` with default values
  - Add unit tests for validation logic

- [ ] **T015** [P] Implement JWT Token model in `src/models/token.rs`
  - Define `JwtToken` struct with fields: `token`, `expires_in`, `obtained_at`
  - Implement `is_expired()` method (check if current time > obtained_at + expires_in)
  - Implement `should_renew()` method (expires in < 5 minutes)
  - Add unit tests for expiration logic

- [ ] **T016** [P] Implement Error Report model in `src/models/error_report.rs`
  - Define `ErrorReport` struct with `serde::Serialize`
  - Add fields: `filename`, `error_type`, `message`, `timestamp` (ISO 8601), `client_version`
  - Implement `ErrorReport::new(filename, error_type, message)` constructor
  - Auto-populate `timestamp` with current UTC time
  - Auto-populate `client_version` from Cargo.toml version
  - Add unit tests for JSON serialization

- [ ] **T017** [P] Implement Batch model in `src/models/batch.rs`
  - Define `Batch` struct with fields: `batch_id` (UUID), `started_at`, `config_snapshot`, `files`, `locked_files`, `processed_count`, `failed_count`, `status`
  - Define `BatchStatus` enum: `Scanning`, `Processing`, `RetryingLocked`, `Completed`, `Aborted`
  - Define `ProcessingStatus` enum: `Pending`, `Locked`, `Converting`, `Compressing`, `Uploading`, `Completed`, `Failed`
  - Implement `Batch::new(config)` constructor (generates UUID, sets started_at)
  - Add methods: `add_file()`, `defer_locked_file()`, `mark_completed()`, `mark_failed()`

- [ ] **T018** [P] Implement DBF File model in `src/models/dbf_file.rs`
  - Define `DbfFile` struct with fields: `path`, `relative_path`, `encoding`, `status`
  - Define `Encoding` enum: `CP866`, `Windows1251`, `UTF8`
  - Implement `DbfFile::new(path, source_dir)` to calculate relative path
  - Implement `generate_compressed_filename()` method (path separators → underscores, .dbf → .csv.gz)
  - Add unit tests for filename encoding logic

- [ ] **T019** Create HTTP client setup in `src/auth/client.rs`
  - Create `HttpClient` struct wrapping `reqwest::Client`
  - Configure client with HTTPS-only enforcement (`.https_only(true)`)
  - Use `rustls-tls` backend (`.use_rustls_tls()`)
  - Add timeout configuration (30 seconds)
  - Export client builder function

---

## Phase 3.4: Authentication & HTTP Client

- [ ] **T020** Implement JWT token retrieval in `src/auth/mod.rs`
  - Define `AuthClient` struct with `HttpClient` and `Config`
  - Implement `get_token(username, password) -> Result<JwtToken>`
  - Send POST to `/api/auth/token` with Basic auth
  - Parse response JSON to extract `token` and `expires_in`
  - Record `obtained_at` timestamp
  - Handle errors: 401 (invalid creds), 403 (subscription inactive), network errors
  - **Makes T006 contract test pass**

- [ ] **T021** Implement JWT token renewal logic in `src/auth/mod.rs`
  - Add `TokenManager` struct to manage token lifecycle
  - Store current token in memory (Arc<RwLock<Option<JwtToken>>>)
  - Implement `get_valid_token() -> Result<JwtToken>` that:
    - Returns current token if valid and not expiring soon
    - Requests new token if expired or expiring within 5 minutes
    - Updates stored token
  - Handle concurrent token refresh (only one refresh at a time)

---

## Phase 3.5: File Processing Pipeline

- [ ] **T022** Implement directory scanner in `src/processor/scanner.rs`
  - Implement `scan_directory(source_dir) -> Result<Vec<DbfFile>>`
  - Recursively walk directory tree using `std::fs::read_dir`
  - Filter for files with `.dbf` extension (case-insensitive)
  - Create `DbfFile` instances with relative paths
  - Handle errors: directory inaccessible (send error report, return empty vec)
  - Add unit tests with temp directories

- [ ] **T023** Implement DBF to CSV converter in `src/processor/converter.rs`
  - Implement `convert_dbf_to_csv(dbf_file: &DbfFile, config: &Config) -> Result<PathBuf>`
  - Use `dbase` crate with `yore` feature to read DBF
  - Attempt auto-detection of encoding from DBF header
  - Use fallback encoding from config if auto-detection fails
  - Write CSV with headers (field names from DBF)
  - Use `csv` crate for writing
  - Output encoding: UTF-8
  - Return path to created CSV file
  - Handle errors: corrupted DBF, encoding errors, disk full
  - Add unit tests with sample DBF files

- [ ] **T024** Implement CSV to gzip compressor in `src/processor/compressor.rs`
  - Implement `compress_csv(csv_path: PathBuf, output_name: String) -> Result<PathBuf>`
  - Use `flate2::write::GzEncoder` with `Compression::default()` (level 6)
  - Read CSV file and write to gzip stream
  - Use filename from `DbfFile::generate_compressed_filename()`
  - Close encoder with `.finish()` to flush stream
  - Return path to created gzip file
  - Handle errors: read errors, compression errors, disk full
  - Add unit tests with sample CSV files

- [ ] **T025** Implement file uploader in `src/processor/uploader.rs`
  - Implement `upload_file(gzip_path: PathBuf, filename: String, token: &JwtToken, config: &Config) -> Result<()>`
  - Use `reqwest::multipart::Form` to create multipart request
  - Add file field with gzip data
  - Send POST to `/api/files/upload` with JWT Bearer token
  - Set `Content-Encoding: gzip` header
  - Handle errors: 401 (renew token and retry once), 4xx (report and skip), 5xx (retry with backoff max 3 times)
  - **Makes T007 contract test pass**

- [ ] **T026** Implement locked file deferral in `src/processor/mod.rs`
  - In batch processing loop, wrap file operations in lock detection
  - Catch I/O errors with `ErrorKind::PermissionDenied` or lock-related codes
  - Move locked files to `batch.locked_files` list
  - Continue processing remaining files
  - After main processing, retry each locked file once
  - Skip files still locked after retry

- [ ] **T027** Implement CSV cleanup in `src/processor/mod.rs`
  - After upload attempt (success or failure), delete local CSV file
  - Use `std::fs::remove_file(csv_path)`
  - Log deletion with file path
  - Handle deletion errors (log but don't fail batch)
  - Ensure source DBF files are never deleted

---

## Phase 3.6: Scheduler & Windows Service

- [ ] **T028** Implement cron scheduler in `src/service/scheduler.rs`
  - Use `tokio-cron-scheduler` to create scheduler
  - Parse cron expression from config (`config.scheduler.crontab`)
  - Add job with cron schedule to trigger batch processing
  - Run on multi-threaded Tokio runtime (required by tokio-cron-scheduler)
  - Handle job notifications (started, completed, failed)
  - Implement graceful shutdown (stop scheduler, wait for in-flight jobs)
  - Add unit tests for schedule parsing

- [ ] **T029** Implement config file watcher in `src/config/watcher.rs`
  - Use `notify-debouncer-mini` to watch `config.toml`
  - Set debounce duration to 1-2 seconds
  - On file change event, set flag for config reload
  - Config reload happens at START of next scheduled run (not mid-batch)
  - Validate new config before applying
  - Keep debouncer alive for service lifetime
  - Handle watcher errors (log and continue)

- [ ] **T030** Implement Windows service lifecycle in `src/service/lifecycle.rs`
  - Use `windows-service` crate with `define_windows_service!` macro
  - Implement service_main function:
    - Initialize Tokio runtime
    - Load config from `C:\Program Files\data-exporter\config.toml`
    - Initialize TokenManager, Scheduler, ConfigWatcher
    - Start scheduler
    - Run until stop event
  - Implement service control handler:
    - Handle Stop: Gracefully shutdown scheduler, cancel in-flight batches
    - Handle Pause/Continue (optional)
  - Proper cleanup on exit
  - **Makes T009 integration test pass**

- [ ] **T031** Implement Windows service registration in `src/service/mod.rs`
  - Implement `register_service()` function
  - Use Windows API (via `windows-service` crate) to register service
  - Set service name: "data-exporter"
  - Set display name: "Data Exporter Service"
  - Set start type: Automatic
  - Set service binary path
  - Handle errors: access denied (needs admin), already exists

- [ ] **T032** Implement Windows service unregistration in `src/service/mod.rs`
  - Implement `unregister_service()` function
  - Stop service if running
  - Delete service from Service Control Manager
  - Handle errors: service not found, access denied

---

## Phase 3.7: Error Handling & Reporting

- [ ] **T033** Implement error reporter in `src/error/reporter.rs`
  - Implement `send_error_report(error_report: ErrorReport, token: Option<&JwtToken>, config: &Config) -> Result<()>`
  - Send POST to `/api/errors/report` with JSON body
  - Include JWT token if available (may be optional for error reports)
  - Handle network errors: fallback to local logging
  - Never retry error reports (fire-and-forget to avoid loops)
  - **Makes T008 contract test pass**

- [ ] **T034** Implement fallback local logger in `src/error/logger.rs`
  - Implement `log_error_locally(error_report: ErrorReport, fallback_reason: String) -> Result<()>`
  - Append to `C:\Program Files\data-exporter\error.log`
  - Format: `[timestamp] ERROR: {fallback_reason}\n  Filename: {filename}\n  Error Type: {error_type}\n  Message: {message}\n`
  - Create file if doesn't exist
  - Handle write errors (log to tracing but don't fail)

- [ ] **T035** Integrate error handling in batch processing (`src/processor/mod.rs`)
  - Wrap each file operation in error handling
  - On error: Create `ErrorReport`, send to server (or log locally)
  - Continue processing remaining files (don't abort batch)
  - Increment `batch.failed_count`
  - Log error with full context (batch ID, file path, error chain)
  - **Makes T013 integration test pass**

---

## Phase 3.8: CLI Interface

- [ ] **T036** Implement CLI argument parsing in `src/main.rs`
  - Use `clap` crate (add to Cargo.toml)
  - Define CLI structure with subcommands: `install`, `uninstall`
  - Install command args: `--username`, `--password`, `--source-dir`, `--crontab`, `--api-url`, `--encoding`
  - Uninstall command args: none
  - Parse args and route to appropriate function

- [ ] **T037** Implement install command in `src/cli/install.rs`
  - Create function `install(username, password, source_dir, crontab, api_url, encoding) -> Result<()>`
  - Step 1: Copy current executable to `C:\Program Files\data-exporter\data_exporter.exe`
  - Step 2: Create config.toml at `C:\Program Files\data-exporter\config.toml`
  - Step 3: Validate credentials by requesting JWT token
  - Step 4: If validation succeeds, register Windows service
  - Step 5: Start service
  - Handle errors at each step, rollback on failure
  - **Makes T009 integration test pass (install portion)**

- [ ] **T038** Implement uninstall command in `src/cli/uninstall.rs`
  - Create function `uninstall() -> Result<()>`
  - Step 1: Stop service if running
  - Step 2: Unregister service from SCM
  - Step 3: Delete `C:\Program Files\data-exporter\` directory
  - Handle errors: service not found (not an error), access denied
  - **Makes T009 integration test pass (uninstall portion)**

- [ ] **T039** Set file permissions on config.toml in install command
  - After creating config.toml, restrict access to Administrators only
  - Use Windows API to set ACL (Access Control List)
  - Deny read access to non-admin users
  - Log permission setting (success or failure)

---

## Phase 3.9: Logging & Observability

- [ ] **T040** Set up structured logging with tracing in `src/main.rs` and `src/service/lifecycle.rs`
  - Initialize `tracing-subscriber` with env filter
  - Configure log levels: ERROR, WARN, INFO, DEBUG
  - Log to stdout and stderr
  - Add spans for batch processing (with batch_id)
  - Add spans for file processing (with file path)
  - Include error chains in error logs

- [ ] **T041** Add logging throughout all modules
  - Log batch start/completion (INFO)
  - Log file processing stages (DEBUG)
  - Log errors with full context (ERROR)
  - Log config reload (INFO)
  - Log token renewal (DEBUG)
  - Log service lifecycle events (INFO)

---

## Phase 3.10: Integration & Polish

- [ ] **T042** Implement batch orchestration in `src/processor/mod.rs`
  - Create `run_batch(config: Config, token_manager: Arc<TokenManager>) -> Result<Batch>`
  - Step 1: Create new Batch with UUID
  - Step 2: Scan directory for DBF files
  - Step 3: For each file: convert → compress → upload (defer if locked)
  - Step 4: Retry locked files
  - Step 5: Complete batch, return stats
  - Integrate all processor components (scanner, converter, compressor, uploader)
  - **Makes T010 integration test pass**

- [ ] **T043** Wire scheduler to batch processing in `src/service/scheduler.rs`
  - In cron job handler, call `run_batch()`
  - Reload config at start of each batch (respect config file changes)
  - Pass TokenManager to batch processing
  - Log batch results
  - Handle batch errors (log but don't stop scheduler)
  - **Makes T011 integration test pass (config reload portion)**

- [ ] **T044** Add unit tests for all modules in `tests/unit/`
  - Create `tests/unit/converter_test.rs` for DBF conversion logic
  - Create `tests/unit/scanner_test.rs` for directory scanning
  - Create `tests/unit/scheduler_test.rs` for cron parsing
  - Create `tests/unit/filename_encoding_test.rs` for path → filename conversion
  - Use temp files and directories for test isolation

- [ ] **T045** Validate against quickstart guide and finalize documentation
  - Run through all quickstart.md scenarios manually or automated
  - Verify all 8 acceptance scenarios work as expected
  - Update documentation with any implementation notes
  - Verify all contract tests pass
  - Verify all integration tests pass
  - Verify all unit tests pass
  - Run `cargo clippy` and fix all warnings
  - Run `cargo fmt` to format all code
  - Update `CLAUDE.md` if needed

---

## Dependencies

**Setup Phase** (T001-T005):
- No dependencies (can run in any order)

**Tests First Phase** (T006-T013):
- T006-T008 [P] can run in parallel (contract tests, different files)
- T009-T013 [P] can run in parallel (integration tests, different files)
- All T006-T013 MUST complete and FAIL before starting T014

**Core Models Phase** (T014-T019):
- T014-T018 [P] can run in parallel (different model files)
- T019 depends on T002 (reqwest dependency)

**Authentication Phase** (T020-T021):
- T020 depends on T015 (JwtToken model), T019 (HttpClient)
- T021 depends on T020

**File Processing Phase** (T022-T027):
- T022 depends on T018 (DbfFile model)
- T023 depends on T018, T014 (Config model)
- T024 no dependencies
- T025 depends on T015 (JwtToken), T019 (HttpClient), T014 (Config)
- T026 depends on T022-T025
- T027 depends on T026

**Scheduler & Service Phase** (T028-T032):
- T028 depends on T014 (Config model)
- T029 depends on T014 (Config model)
- T030 depends on T028, T029, T021 (TokenManager)
- T031-T032 no dependencies (Windows API)

**Error Handling Phase** (T033-T035):
- T033 depends on T016 (ErrorReport model), T019 (HttpClient)
- T034 depends on T016 (ErrorReport model)
- T035 depends on T033, T034

**CLI Phase** (T036-T039):
- T036 no dependencies (clap library)
- T037 depends on T014 (Config model), T020 (token validation), T031 (service registration)
- T038 depends on T032 (service unregistration)
- T039 depends on T037

**Logging Phase** (T040-T041):
- T040 depends on T002 (tracing dependencies)
- T041 depends on T040

**Integration Phase** (T042-T045):
- T042 depends on T022-T027 (all processor components)
- T043 depends on T042, T028 (scheduler), T029 (config watcher)
- T044 can be done in parallel with T043
- T045 depends on ALL previous tasks

---

## Parallel Execution Examples

### Phase 3.2: All Contract Tests Together
```bash
# All contract tests are independent (different test files)
Task: Write contract test for Auth API in tests/contract/auth_test.rs
Task: Write contract test for Upload API in tests/contract/upload_test.rs
Task: Write contract test for Error Report API in tests/contract/error_test.rs
```

### Phase 3.2: All Integration Tests Together
```bash
# All integration tests are independent (different test files)
Task: Write integration test for service lifecycle in tests/integration/service_lifecycle_test.rs
Task: Write integration test for batch processing in tests/integration/batch_processing_test.rs
Task: Write integration test for config reload in tests/integration/config_reload_test.rs
Task: Write integration test for locked file retry in tests/integration/locked_file_retry_test.rs
Task: Write integration test for error handling in tests/integration/error_handling_test.rs
```

### Phase 3.3: All Model Structs Together
```bash
# All models are independent (different files)
Task: Implement Configuration model in src/models/config.rs
Task: Implement JWT Token model in src/models/token.rs
Task: Implement Error Report model in src/models/error_report.rs
Task: Implement Batch model in src/models/batch.rs
Task: Implement DBF File model in src/models/dbf_file.rs
```

---

## Notes

- **[P] tasks** = Different files, no blocking dependencies, can execute in parallel
- **TDD Discipline**: Verify all tests (T006-T013) are written and failing before implementing features
- **Constitution Compliance**: All code must pass `cargo clippy`, be formatted with `cargo fmt`, use `Result` types, avoid `unwrap()`
- **Error Handling**: Use `thiserror` for domain errors, `anyhow` for application errors, full error context in logs
- **Security**: HTTPS-only, JWT in memory only, config.toml restricted to admins, no unsafe code
- **Commit Strategy**: Commit after each task completion with descriptive message
- **Testing**: Run `cargo test` frequently, all tests must pass before moving to next phase
- **Windows-Specific**: Requires Windows environment for service lifecycle tests, can mock for unit tests
- **Performance**: No specific performance tests required, but monitor resource usage during integration tests

---

## Validation Checklist
*GATE: Verify before considering tasks complete*

- [x] All 3 contracts have corresponding contract tests (T006-T008)
- [x] All 8 entities have model implementation tasks (T014-T018 + implicit in other tasks)
- [x] All contract tests come before implementation (T006-T008 before T020, T025, T033)
- [x] All integration tests come before related implementations (T009-T013 before T030, T042, T043)
- [x] Parallel tasks are truly independent (verified: all [P] tasks use different files)
- [x] Each task specifies exact file path or module
- [x] No [P] task modifies same file as another [P] task (verified)
- [x] TDD order enforced: Tests → Models → Implementation → Make tests pass
- [x] Total task count matches plan estimate (45 tasks within 35-45 range)

---

**Tasks Ready for Execution**: All 45 tasks defined, ordered, and ready for TDD-first implementation following constitution principles.
