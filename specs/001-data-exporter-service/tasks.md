# Tasks: Data Exporter Windows Service

**Input**: Design documents from `/specs/001-data-exporter-service/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/api.yaml

**Tests**: TDD approach specified - tests are written first, verified failing, then implementation to pass tests.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md workspace structure:
- **common/src/**: Shared library (models, auth, error)
- **service/src/**: Windows service binary (service, processor, config, vss)
- **configurator/src/**: GUI configurator
- **tests/**: Integration & contract tests

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure verification

- [X] T001 Verify workspace structure matches plan.md (common/, service/, configurator/, tests/)
- [X] T002 [P] Verify Cargo.toml dependencies: windows-service 0.7, tokio-cron-scheduler 0.13, dbase 0.5, reqwest 0.12, flate2, globset, rawcopy-rs
- [X] T003 [P] Add dev-dependencies: wiremock 0.6, tempfile 3, tokio-test 0.4
- [X] T004 [P] Create test fixtures directory at tests/fixtures/ with placeholder README

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**CRITICAL**: No user story work can begin until this phase is complete

### Tests for Foundational Phase

- [X] T005 [P] Write unit tests for Config validation (cron expression, URL, encoding) in common/src/models/config.rs
- [X] T006 [P] Write unit tests for Batch state machine transitions in common/src/models/batch.rs
- [X] T007 [P] Write unit tests for DbfFile status tracking in common/src/models/dbf_file.rs
- [X] T008 [P] Write unit tests for ErrorReport validation and truncation in common/src/models/error_report.rs

### Implementation for Foundational Phase

- [X] T009 [P] Implement Config struct with SchedulerConfig, SourceConfig, CredentialConfig, ApiConfig, EncodingConfig in common/src/models/config.rs
- [X] T010 [P] Implement Batch struct with BatchStatus state machine in common/src/models/batch.rs
- [X] T011 [P] Implement DbfFile struct with FileProcessingStatus state machine in common/src/models/dbf_file.rs
- [X] T012 [P] Implement ErrorReport and GlobalErrorReport structs with validation in common/src/models/error_report.rs
- [X] T013 [P] Implement Encoding enum with LDID detection mapping in common/src/models/dbf_file.rs
- [X] T014 Create common/src/models/mod.rs exporting all model types
- [X] T015 [P] Create ProcessingData enum (InMemory/TempFile) in service/src/processor/mod.rs

**Checkpoint**: Foundation ready - all models have unit tests and pass. User story implementation can now begin.

---

## Phase 3: User Story 1 - Automated Scheduled Data Export (Priority: P1) - MVP

**Goal**: Service triggers batch processing automatically on a configurable cron schedule

**Independent Test**: Configure a cron schedule and verify batch processing triggers at expected times, processes files, and uploads to server.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T016 [P] [US1] Write unit test for cron expression parsing in service/src/service/scheduler.rs
- [ ] T017 [P] [US1] Write integration test for scheduler triggering batch in tests/integration/scheduler_test.rs
- [ ] T018 [P] [US1] Write unit test for batch lock preventing concurrent execution in service/src/service/scheduler.rs

### Implementation for User Story 1

- [ ] T019 [US1] Implement cron expression parsing using tokio-cron-scheduler in service/src/service/scheduler.rs
- [ ] T020 [US1] Implement JobScheduler with batch trigger callback in service/src/service/scheduler.rs
- [ ] T021 [US1] Implement batch lock mechanism (AtomicBool or Mutex) in service/src/service/scheduler.rs
- [ ] T022 [US1] Implement skip-and-log behavior when batch already running in service/src/service/scheduler.rs
- [ ] T023 [US1] Integrate scheduler with main service loop in service/src/main.rs
- [ ] T024 [US1] Add logging for scheduled triggers, skipped executions in service/src/service/scheduler.rs

**Checkpoint**: Service can be configured with cron schedule and triggers batch processing automatically. Concurrent batch execution is prevented.

---

## Phase 4: User Story 2 - DBF to CSV Conversion with Encoding Support (Priority: P1)

**Goal**: DBF files are converted to UTF-8 CSV with correct encoding detection from header or fallback config

**Independent Test**: Place DBF files with various encodings (CP866, Windows-1251, Windows-1255) in source directory and verify output CSV contains correctly decoded UTF-8 text.

### Tests for User Story 2

- [ ] T025 [P] [US2] Create test fixture DBF file with CP866 encoding at tests/fixtures/cp866_russian.dbf
- [ ] T026 [P] [US2] Create test fixture DBF file with Windows-1251 encoding at tests/fixtures/cp1251_russian.dbf
- [ ] T027 [P] [US2] Create test fixture DBF file with Windows-1255 encoding at tests/fixtures/cp1255_hebrew.dbf
- [ ] T028 [P] [US2] Create test fixture DBF file with corrupted records at tests/fixtures/corrupted_records.dbf
- [ ] T029 [P] [US2] Write unit test for LDID to Encoding mapping in service/src/processor/converter.rs
- [ ] T030 [P] [US2] Write unit test for CP866 DBF to UTF-8 CSV conversion in service/src/processor/converter.rs
- [ ] T031 [P] [US2] Write unit test for Windows-1251 DBF to UTF-8 CSV conversion in service/src/processor/converter.rs
- [ ] T032 [P] [US2] Write unit test for fallback encoding when LDID absent in service/src/processor/converter.rs
- [ ] T033 [P] [US2] Write unit test for corrupted record skipping in service/src/processor/converter.rs
- [ ] T034 [P] [US2] Write unit test for GZIP compression output in service/src/processor/compressor.rs
- [ ] T035 [P] [US2] Write integration test for full conversion pipeline in tests/integration/conversion_test.rs

### Implementation for User Story 2

- [ ] T036 [US2] Implement LDID to Encoding detection using dbase crate in service/src/processor/converter.rs
- [ ] T037 [US2] Implement DBF to CSV conversion with yore encoding support in service/src/processor/converter.rs
- [ ] T038 [US2] Implement convert_dbf_to_csv_memory for files <= 10MB in service/src/processor/converter.rs
- [ ] T039 [US2] Implement convert_dbf_to_csv_tempfile for files > 10MB in service/src/processor/converter.rs
- [ ] T040 [US2] Implement corrupted record handling (skip and log warning) in service/src/processor/converter.rs
- [ ] T041 [US2] Implement GZIP compression using flate2 in service/src/processor/compressor.rs
- [ ] T042 [US2] Implement temporary file cleanup after processing in service/src/processor/mod.rs
- [ ] T043 [US2] Create service/src/processor/mod.rs exporting converter and compressor modules

**Checkpoint**: DBF files with various encodings convert correctly to UTF-8 CSV. Corrupted records are skipped with warnings. Output is GZIP compressed.

---

## Phase 5: User Story 3 - Reliable File Upload with Retry (Priority: P1)

**Goal**: Files upload to server with automatic retry on transient failures using exponential backoff

**Independent Test**: Simulate network failures during upload and verify system retries and eventually succeeds when connectivity is restored.

### Tests for User Story 3

- [ ] T044 [P] [US3] Write contract test for /api/auth/login endpoint in tests/contract/auth_test.rs
- [ ] T045 [P] [US3] Write contract test for /api/files/upload endpoint in tests/contract/upload_test.rs
- [ ] T046 [P] [US3] Write contract test for /api/batches/start endpoint in tests/contract/batch_api_test.rs
- [ ] T047 [P] [US3] Write contract test for /api/batches/{id}/complete endpoint in tests/contract/batch_api_test.rs
- [ ] T048 [P] [US3] Write unit test for multipart form construction in service/src/processor/uploader.rs
- [ ] T049 [P] [US3] Write unit test for retry logic (1s, 2s, 4s backoff) on 5xx errors in service/src/processor/uploader.rs
- [ ] T050 [P] [US3] Write unit test for no retry on 4xx client errors (except 401) in service/src/processor/uploader.rs
- [ ] T051 [P] [US3] Write unit test for 401 token refresh and retry in service/src/processor/uploader.rs
- [ ] T052 [P] [US3] Write unit test for 5-minute upload timeout in service/src/processor/uploader.rs
- [ ] T053 [P] [US3] Write integration test for batch start/complete API flow in tests/integration/batch_api_test.rs

### Implementation for User Story 3

- [ ] T054 [US3] Implement authentication client (login, token storage) in common/src/auth/client.rs
- [ ] T055 [US3] Implement token refresh on 401 response in common/src/auth/client.rs
- [ ] T056 [US3] Implement multipart file upload with reqwest in service/src/processor/uploader.rs
- [ ] T057 [US3] Implement retry logic with exponential backoff (1s, 2s, 4s) in service/src/processor/uploader.rs
- [ ] T058 [US3] Implement 5-minute timeout for large file uploads in service/src/processor/uploader.rs
- [ ] T059 [US3] Implement batch start API client in service/src/processor/batch_client.rs
- [ ] T060 [US3] Implement batch complete API client in service/src/processor/batch_client.rs
- [ ] T061 [US3] Implement continue-on-failure behavior for individual file uploads in service/src/processor/uploader.rs
- [ ] T062 [US3] Add logging for upload success, retry, and final failure in service/src/processor/uploader.rs

**Checkpoint**: Files upload successfully with bearer token auth. Transient failures retry automatically. Batch lifecycle (start/complete) is reported to server.

---

## Phase 6: User Story 4 - Locked File Handling via VSS (Priority: P2)

**Goal**: Files locked by other applications are accessed via Volume Shadow Copy

**Independent Test**: Open a DBF file exclusively in another application and verify service successfully copies and processes it via shadow copy.

### Tests for User Story 4

- [ ] T063 [P] [US4] Write unit test for locked file detection (OS errors 32, 33) in service/src/vss/mod.rs
- [ ] T064 [P] [US4] Write unit test for deferred file queue management in service/src/processor/scanner.rs
- [ ] T065 [P] [US4] Write integration test for VSS copy with feature flag in tests/integration/vss_copy_test.rs

### Implementation for User Story 4

- [ ] T066 [US4] Implement locked file detection by OS error codes 32, 33 in service/src/vss/mod.rs
- [ ] T067 [US4] Implement deferred file queue (move locked files to end of batch) in service/src/processor/scanner.rs
- [ ] T068 [US4] Implement VSS shadow copy using rawcopy-rs in service/src/vss/mod.rs
- [ ] T069 [US4] Implement VSS error handling in service/src/vss/error.rs
- [ ] T070 [US4] Implement VSS temp file cleanup after processing in service/src/vss/mod.rs
- [ ] T071 [US4] Add feature flag for VSS tests (#[cfg(feature = "vss-tests")]) in tests/integration/vss_copy_test.rs
- [ ] T072 [US4] Add logging for locked file detection, VSS copy success/failure in service/src/vss/mod.rs

**Checkpoint**: Locked files are detected and deferred. VSS shadow copy creates accessible copies. Cleanup removes VSS temp files.

---

## Phase 7: User Story 5 - Comprehensive Error Logging and Reporting (Priority: P2)

**Goal**: All errors are logged locally and reported to server for visibility and alerting

**Independent Test**: Induce various error conditions and verify logs are created locally and error reports are sent to server.

### Tests for User Story 5

- [ ] T073 [P] [US5] Write contract test for /api/errors endpoint in tests/contract/error_report_test.rs
- [ ] T074 [P] [US5] Write contract test for /api/batches/{id}/errors endpoint in tests/contract/error_report_test.rs
- [ ] T075 [P] [US5] Write unit test for error severity classification in common/src/error/reporter.rs
- [ ] T076 [P] [US5] Write unit test for local file fallback when server unreachable in common/src/error/logger.rs
- [ ] T077 [P] [US5] Write unit test for panic handler logging in common/src/error/global_logger.rs
- [ ] T078 [P] [US5] Write unit test for log rotation (daily) in common/src/error/logger.rs

### Implementation for User Story 5

- [ ] T079 [US5] Implement error severity classification (Critical, Error, Warning, Info) in common/src/error/reporter.rs
- [ ] T080 [US5] Implement server error reporting client in common/src/error/reporter.rs
- [ ] T081 [US5] Implement batch error reporting client (POST /api/batches/{id}/errors) in common/src/error/reporter.rs
- [ ] T082 [US5] Implement local file logging with daily rotation in common/src/error/logger.rs
- [ ] T083 [US5] Implement fallback to local logging when server unreachable in common/src/error/reporter.rs
- [ ] T084 [US5] Implement panic handler with fallback log location in common/src/error/global_logger.rs
- [ ] T085 [US5] Implement error metadata collection (version, paths, details) in common/src/error/reporter.rs
- [ ] T086 [US5] Create common/src/error/mod.rs exporting all error modules

**Checkpoint**: Errors are classified by severity. Reports sent to server with fallback to local logs. Panics are captured before exit.

---

## Phase 8: User Story 6 - Graceful Service Lifecycle Management (Priority: P2)

**Goal**: Service starts, stops, and recovers gracefully without data corruption

**Independent Test**: Issue start/stop commands and verify service responds correctly and completes or cleanly aborts in-progress work.

### Tests for User Story 6

- [ ] T087 [P] [US6] Write unit test for graceful shutdown signal handling in service/src/service/windows_service.rs
- [ ] T088 [P] [US6] Write unit test for config retry with exponential backoff in service/src/config/watcher.rs
- [ ] T089 [P] [US6] Write integration test for config hot-reload in tests/integration/config_reload_test.rs

### Implementation for User Story 6

- [ ] T090 [US6] Implement Windows SCM integration using windows-service in service/src/service/windows_service.rs
- [ ] T091 [US6] Implement Start, Stop, Interrogate command handlers in service/src/service/windows_service.rs
- [ ] T092 [US6] Implement graceful shutdown (complete current file, then stop) in service/src/service/windows_service.rs
- [ ] T093 [US6] Implement config loading from TOML in service/src/config/loader.rs
- [ ] T094 [US6] Implement config retry with exponential backoff (1, 2, 4, 8, 16 min, then hourly) in service/src/config/loader.rs
- [ ] T095 [US6] Implement config hot-reload before each batch in service/src/config/watcher.rs
- [ ] T096 [US6] Implement service status reporting (Stopped on fatal error) in service/src/service/windows_service.rs
- [ ] T097 [US6] Mark admin-required tests with #[ignore] attribute in service/src/service/windows_service.rs

**Checkpoint**: Service integrates with Windows SCM. Stop signal completes current work. Config reloads automatically.

---

## Phase 9: User Story 7 - File Filtering with Include/Exclude Patterns (Priority: P3)

**Goal**: Files are filtered using configurable glob patterns for include/exclude

**Independent Test**: Configure include/exclude patterns and verify only matching files are processed.

### Tests for User Story 7

- [ ] T098 [P] [US7] Write unit test for include pattern matching in service/src/processor/filter.rs
- [ ] T099 [P] [US7] Write unit test for exclude pattern matching in service/src/processor/filter.rs
- [ ] T100 [P] [US7] Write unit test for exclude taking precedence over include in service/src/processor/filter.rs
- [ ] T101 [P] [US7] Write unit test for default behavior (all DBF files) when no patterns in service/src/processor/filter.rs
- [ ] T102 [P] [US7] Write integration test for pattern-based directory scanning in tests/integration/filter_test.rs

### Implementation for User Story 7

- [ ] T103 [US7] Implement glob pattern matching using globset in service/src/processor/filter.rs
- [ ] T104 [US7] Implement include pattern filter (match at least one) in service/src/processor/filter.rs
- [ ] T105 [US7] Implement exclude pattern filter (skip if matched) in service/src/processor/filter.rs
- [ ] T106 [US7] Implement directory scanner with pattern filtering in service/src/processor/scanner.rs
- [ ] T107 [US7] Implement recursive .dbf file discovery (case-insensitive) in service/src/processor/scanner.rs
- [ ] T108 [US7] Integrate filter with scanner in batch processing pipeline in service/src/processor/mod.rs

**Checkpoint**: Files are filtered by include/exclude patterns. Default behavior processes all DBF files.

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T109 [P] Run cargo clippy and fix all warnings
- [ ] T110 [P] Run cargo fmt to ensure consistent formatting
- [ ] T111 Verify all tests pass with cargo test
- [ ] T112 [P] Run ignored tests (admin required) with cargo test -- --ignored
- [ ] T113 Run quickstart.md validation scenarios
- [ ] T114 [P] Build release binary with cargo build --release
- [ ] T115 Create service installer script (if not exists)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-9)**: All depend on Foundational phase completion
  - US1, US2, US3 (P1 stories) should complete before P2 stories
  - User stories can proceed in parallel if staffed
  - Or sequentially in priority order (P1 -> P2 -> P3)
- **Polish (Phase 10)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - Provides scheduler infrastructure
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - Provides conversion pipeline
- **User Story 3 (P1)**: Can start after Foundational (Phase 2) - Provides upload infrastructure
- **User Story 4 (P2)**: Can start after Foundational - Integrates with scanner from US7
- **User Story 5 (P2)**: Can start after Foundational - Integrates with all stories for error handling
- **User Story 6 (P2)**: Can start after Foundational - Integrates with scheduler from US1
- **User Story 7 (P3)**: Can start after Foundational - Provides scanner used by other stories

### Within Each User Story (TDD Flow)

1. Tests MUST be written and verified FAILING before implementation
2. Models/utilities before services
3. Services before endpoints/integration
4. Core implementation before integration
5. Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tests (T005-T008) can run in parallel
- All Foundational implementations (T009-T015) can run in parallel after tests pass
- Test fixtures (T025-T028) can be created in parallel
- Contract tests for US3 (T044-T053) can run in parallel
- All P1 stories (US1, US2, US3) can be worked on in parallel after Foundational
- Different user stories can be worked on by different team members

---

## Parallel Example: User Story 2 (Conversion Pipeline)

```bash
# Launch all test fixtures in parallel:
Task: "Create test fixture DBF file with CP866 encoding at tests/fixtures/cp866_russian.dbf"
Task: "Create test fixture DBF file with Windows-1251 encoding at tests/fixtures/cp1251_russian.dbf"
Task: "Create test fixture DBF file with Windows-1255 encoding at tests/fixtures/cp1255_hebrew.dbf"
Task: "Create test fixture DBF file with corrupted records at tests/fixtures/corrupted_records.dbf"

# Launch all unit tests in parallel:
Task: "Write unit test for LDID to Encoding mapping in service/src/processor/converter.rs"
Task: "Write unit test for CP866 DBF to UTF-8 CSV conversion in service/src/processor/converter.rs"
Task: "Write unit test for Windows-1251 DBF to UTF-8 CSV conversion in service/src/processor/converter.rs"
Task: "Write unit test for GZIP compression output in service/src/processor/compressor.rs"
```

---

## Parallel Example: Foundational Phase

```bash
# Launch all model tests in parallel:
Task: "Write unit tests for Config validation in common/src/models/config.rs"
Task: "Write unit tests for Batch state machine in common/src/models/batch.rs"
Task: "Write unit tests for DbfFile status tracking in common/src/models/dbf_file.rs"
Task: "Write unit tests for ErrorReport validation in common/src/models/error_report.rs"

# After tests fail, launch all implementations in parallel:
Task: "Implement Config struct in common/src/models/config.rs"
Task: "Implement Batch struct in common/src/models/batch.rs"
Task: "Implement DbfFile struct in common/src/models/dbf_file.rs"
Task: "Implement ErrorReport structs in common/src/models/error_report.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1, 2, 3)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Scheduling)
4. Complete Phase 4: User Story 2 (Conversion)
5. Complete Phase 5: User Story 3 (Upload)
6. **STOP and VALIDATE**: Test full pipeline - cron trigger -> scan -> convert -> compress -> upload
7. Deploy/demo if ready - this is a working MVP!

### Incremental Delivery

1. Complete Setup + Foundational -> Foundation ready
2. Add User Story 1 -> Test scheduler independently -> Core scheduling works
3. Add User Story 2 -> Test conversion independently -> DBF to CSV works
4. Add User Story 3 -> Test upload independently -> Full pipeline works (MVP!)
5. Add User Story 4 -> VSS handling for locked files
6. Add User Story 5 -> Comprehensive error handling
7. Add User Story 6 -> Windows service lifecycle
8. Add User Story 7 -> File filtering
9. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Scheduling)
   - Developer B: User Story 2 (Conversion)
   - Developer C: User Story 3 (Upload)
3. After P1 stories complete:
   - Developer A: User Story 4 (VSS)
   - Developer B: User Story 5 (Error Reporting)
   - Developer C: User Story 6 (Service Lifecycle)
4. Then: User Story 7 (Filtering) and Polish

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing (TDD: RED -> GREEN -> REFACTOR)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- Admin-required tests use #[ignore] attribute
- VSS tests use #[cfg(feature = "vss-tests")] feature flag
