# DBF Uploader v2.0 - Implementation Task List

**Project**: DBF Uploader Migration to Middleware Batch Protocol
**Version**: 2.0.0
**Timeline**: 4 weeks (28 days)
**Status**: Ready for Implementation

---

## 📋 Task Hierarchy

### 🎯 Epic: DBF Uploader v2.0 Migration
**Objective**: Migrate uploader to middleware-compatible batch protocol
**Duration**: 4 weeks
**Priority**: Critical

---

## Phase 1: Core Protocol Migration (Week 1-2, 10 days)

### 1.1 Authentication Module Redesign
**Priority**: Critical | **Estimate**: 3 days | **Phase**: 1

#### Task 1.1.1: Create SiteCredentials struct ✅
- [x] Define `SiteCredentials` struct with domain and client_secret fields
- [x] Add serde Deserialize/Serialize derives
- [x] Add validation methods for domain format
- [x] Add validation for UUID format in client_secret
- [x] Write unit tests for validation logic
- **Files**: `src/auth/credentials.rs` (new)
- **Dependencies**: None
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.1.2: Update JwtToken struct with site context ✅
- [x] Add `site_id: Uuid` field to JwtToken
- [x] Add `account_id: Uuid` field to JwtToken
- [x] Add `domain: String` field to JwtToken
- [x] Update test constructors with new fields
- **Files**: `src/auth/mod.rs`
- **Dependencies**: 1.1.1
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.1.3: Implement JWT payload parsing ✅
- [x] Create `JwtPayload` struct matching middleware claims
- [x] Implement base64 decoding for JWT payload
- [x] Parse siteId, accountId, domain, exp claims
- [x] Handle camelCase to snake_case conversion
- [x] Add error handling for malformed tokens
- [x] Write unit tests with sample JWTs
- [x] Add JwtToken::from_token_string() method
- [x] Update AuthClient::get_token() to use parsing
- **Files**: `src/auth/mod.rs`
- **Dependencies**: 1.1.2
- **Estimate**: 1 day
- **Status**: COMPLETED

#### Task 1.1.4: Update AuthClient for site credentials ✅
- [x] Replace username/password with SiteCredentials
- [x] Update get_token() to use domain:clientSecret format
- [x] Change endpoint from `/api/auth/token` to `/api/v1/auth/token`
- [x] Update Basic Auth header construction
- [x] Add AuthClient::from_credentials() for v2 config
- [x] Add v1 compatibility layer in AuthClient::new()
- **Files**: `src/auth/mod.rs`
- **Dependencies**: 1.1.1, 1.1.2, 1.1.3
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.1.5: Implement token renewal logic ✅
- [x] Update `is_expired()` method to use 5-minute threshold (300s)
- [x] TokenManager automatic renewal already implemented
- [x] Add test cases for 5-minute threshold
- [x] Renewal logic validates token before batch operations
- **Files**: `src/auth/mod.rs`
- **Dependencies**: 1.1.4
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.1.6: Authentication error handling ✅
- [x] Authentication errors already handled via ProcessingError enum
- [x] HTTP error codes (401, 403, 5xx) properly mapped
- [x] Error context includes detailed messages
- [x] TokenManager handles renewal failures
- **Files**: `src/auth/mod.rs`, `src/error/mod.rs`
- **Dependencies**: 1.1.5
- **Estimate**: 0.5 day
- **Status**: COMPLETED (existing error handling sufficient)

---

### 1.2 Batch Lifecycle Manager (NEW Module)
**Priority**: Critical | **Estimate**: 5 days | **Phase**: 1

#### Task 1.2.1: Create BatchState enum ✅
- [x] Define BatchState enum with all states (Created, InProgress, Completed, Failed, Cancelled)
- [x] Implement state transition validation
- [x] Add Display trait for logging
- [x] Add serialization support
- [x] Add helper methods: is_terminal(), is_active(), can_transition_to()
- [x] Write state machine tests (6 tests passing)
- **Files**: `src/batch/state.rs` (new)
- **Dependencies**: None
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.2.2: Create Batch struct ✅
- [x] Define Batch struct with all metadata fields
- [x] Add builder pattern via new() constructor
- [x] Implement transition_to() validation method
- [x] Add helper methods: mark_started(), mark_completed(), mark_failed(), mark_cancelled()
- [x] Add statistics methods: add_uploaded_file(), increment_error_count(), duration()
- [x] Write unit tests (9 tests passing)
- **Files**: `src/batch/batch.rs` (new)
- **Dependencies**: 1.2.1
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.2.3: Create BatchManager struct ✅
- [x] Define BatchManager with http_client and token_manager
- [x] Implement constructor and initialization
- [x] Add current_batch tracking with Arc<Mutex<>>
- [x] Add Arc<Mutex<>> for thread safety
- **Files**: `src/batch/manager.rs` (new)
- **Dependencies**: 1.2.2, 1.1
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.2.4: Implement start_batch() ✅
- [x] Call middleware `/api/v1/batch/start` endpoint
- [x] Handle authentication token renewal via TokenManager
- [x] Parse BatchStartResponse DTO
- [x] Create and store Batch instance
- [x] Add error handling and logging (debug, info)
- [x] Check for existing active batch before starting
- **Files**: `src/batch/manager.rs`
- **Dependencies**: 1.2.3
- **Estimate**: 1 day
- **Status**: COMPLETED

#### Task 1.2.5: Implement upload_files() ✅
- [x] Build multipart form with multiple files
- [x] Call `/api/v1/batch/{id}/upload` endpoint
- [x] Handle file read errors gracefully
- [x] Parse UploadResponse DTO
- [x] Update batch metadata (file count, size)
- [x] Add comprehensive error handling
- **Files**: `src/batch/manager.rs`
- **Dependencies**: 1.2.4
- **Estimate**: 1.5 days
- **Status**: COMPLETED

#### Task 1.2.6: Implement complete_batch() ✅
- [x] Call `/api/v1/batch/{id}/complete` endpoint
- [x] Parse BatchCompleteResponse DTO
- [x] Update batch state to Completed
- [x] Calculate and log duration
- [x] Clear current_batch
- **Files**: `src/batch/manager.rs`
- **Dependencies**: 1.2.5
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.2.7: Implement fail_batch() ✅
- [x] Call `/api/v1/batch/{id}/fail` endpoint with reason
- [x] Handle failure response with error logging
- [x] Update batch state to Failed
- [x] Clear current_batch
- [x] Log failure details (warn level)
- **Files**: `src/batch/manager.rs`
- **Dependencies**: 1.2.6
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.2.8: Implement cancel_batch() ✅
- [x] Call `/api/v1/batch/{id}/cancel` endpoint
- [x] Handle cancellation response
- [x] Update batch state to Cancelled
- [x] Clear current_batch
- [x] Add info logging for cancellation
- **Files**: `src/batch/manager.rs`
- **Dependencies**: 1.2.7
- **Estimate**: 0.5 day
- **Status**: COMPLETED

#### Task 1.2.9: Create response DTOs ✅
- [x] Define BatchStartResponse struct
- [x] Define UploadResponse struct
- [x] Define UploadedFileInfo struct
- [x] Define BatchCompleteResponse struct
- [x] Add serde derives and camelCase field renaming
- [x] Write DTO serialization tests (3 tests passing)
- **Files**: `src/batch/dto.rs` (new)
- **Dependencies**: None
- **Estimate**: 0.5 day
- **Status**: COMPLETED

---

### 1.3 Error Reporting Module Update
**Priority**: High | **Estimate**: 2 days | **Phase**: 1

#### Task 1.3.1: Create ErrorLogRequest DTO
- [ ] Define ErrorLogRequest struct matching middleware schema
- [ ] Add type, message, metadata, clientVersion fields
- [ ] Implement serde with camelCase renaming
- [ ] Add validation methods
- [ ] Write DTO tests
- **Files**: `src/error/dto.rs` (new)
- **Dependencies**: None
- **Estimate**: 0.5 day

#### Task 1.3.2: Update ErrorReporter struct
- [ ] Add auth_client reference
- [ ] Add client_version field
- [ ] Add local_log_path configuration
- [ ] Update constructor
- **Files**: `src/error/reporter.rs`
- **Dependencies**: 1.3.1, 1.1
- **Estimate**: 0.5 day

#### Task 1.3.3: Implement report_batch_error()
- [ ] Call `/api/v1/error/{batchId}` endpoint
- [ ] Try with auth token, fallback without
- [ ] Use ErrorLogRequest DTO
- [ ] Implement fallback to local log on failure
- [ ] Add fire-and-forget behavior (no retry)
- [ ] Write error reporting tests
- **Files**: `src/error/reporter.rs`
- **Dependencies**: 1.3.2, 1.2
- **Estimate**: 0.5 day

#### Task 1.3.4: Implement report_standalone_error()
- [ ] Call `/api/v1/error` endpoint (no batch ID)
- [ ] Handle authentication scenarios
- [ ] Implement fallback to local log
- [ ] Add logging for reporting failures
- [ ] Write standalone error tests
- **Files**: `src/error/reporter.rs`
- **Dependencies**: 1.3.3
- **Estimate**: 0.5 day

#### Task 1.3.5: Implement local log fallback
- [ ] Create log_to_local_file() method
- [ ] Use tokio async file operations
- [ ] Format log entries with timestamp
- [ ] Implement append-only behavior
- [ ] Handle file creation and permissions
- [ ] Write local logging tests
- **Files**: `src/error/reporter.rs`
- **Dependencies**: 1.3.4
- **Estimate**: 0.5 day

---

## Phase 2: Configuration & Migration (Week 2, 5 days)

### 2.1 Configuration V2 Structures
**Priority**: High | **Estimate**: 1.5 days | **Phase**: 2

#### Task 2.1.1: Define ConfigV2 struct
- [ ] Create main ConfigV2 struct
- [ ] Define AuthConfigV2 with domain and client_secret
- [ ] Define ApiConfigV2 with v2 endpoints
- [ ] Define BatchConfig with new options
- [ ] Define LoggingConfig
- [ ] Add serde derives for TOML
- **Files**: `src/config/v2.rs` (new)
- **Dependencies**: None
- **Estimate**: 0.5 day

#### Task 2.1.2: Add config validation
- [ ] Validate domain format (DNS-compliant)
- [ ] Validate client_secret as UUID
- [ ] Validate directory paths exist
- [ ] Validate cron format
- [ ] Validate max_files_per_batch range
- [ ] Write validation tests
- **Files**: `src/config/validation.rs` (new)
- **Dependencies**: 2.1.1
- **Estimate**: 0.5 day

#### Task 2.1.3: Implement config loading
- [ ] Read config file from standard location
- [ ] Parse TOML into ConfigV2
- [ ] Apply validation
- [ ] Handle parsing errors gracefully
- [ ] Write config loading tests
- **Files**: `src/config/loader.rs`
- **Dependencies**: 2.1.1, 2.1.2
- **Estimate**: 0.5 day

---

### 2.2 Migration Tool
**Priority**: High | **Estimate**: 2.5 days | **Phase**: 2

#### Task 2.2.1: Create ConfigMigration module
- [ ] Define ConfigMigration struct
- [ ] Create ConfigVersion enum (V1, V2)
- [ ] Add migration result types
- **Files**: `src/migration/mod.rs` (new)
- **Dependencies**: None
- **Estimate**: 0.5 day

#### Task 2.2.2: Implement version detection
- [ ] Read existing config file
- [ ] Detect version from content patterns
- [ ] Handle missing config scenarios
- [ ] Write detection tests
- **Files**: `src/migration/detector.rs` (new)
- **Dependencies**: 2.2.1
- **Estimate**: 0.5 day

#### Task 2.2.3: Implement migrate_v1_to_v2()
- [ ] Read v1 config
- [ ] Generate v2 template config
- [ ] Preserve source, schedule, encoding settings
- [ ] Display migration instructions
- [ ] Generate MigrationGuide struct
- [ ] Write migration tests
- **Files**: `src/migration/migrator.rs` (new)
- **Dependencies**: 2.2.2, 2.1
- **Estimate**: 1 day

#### Task 2.2.4: Create authentication test utility
- [ ] Implement test_auth() helper
- [ ] Call middleware auth endpoint
- [ ] Validate credentials
- [ ] Return clear success/failure
- [ ] Write test utility tests
- **Files**: `src/migration/auth_test.rs` (new)
- **Dependencies**: 1.1
- **Estimate**: 0.5 day

---

### 2.3 Interactive Migration Wizard
**Priority**: Medium | **Estimate**: 1 day | **Phase**: 2

#### Task 2.3.1: Create interactive prompt system
- [ ] Implement prompt() function for user input
- [ ] Add input validation
- [ ] Handle Ctrl+C gracefully
- [ ] Add colored terminal output
- [ ] Write prompt tests
- **Files**: `src/migration/wizard.rs` (new)
- **Dependencies**: None
- **Estimate**: 0.5 day

#### Task 2.3.2: Implement run_wizard()
- [ ] Display wizard introduction
- [ ] Prompt for domain, client_secret, base_url, source_dir
- [ ] Call test_auth() to validate
- [ ] Generate ConfigV2 from input
- [ ] Write wizard to config file
- [ ] Add wizard integration tests
- **Files**: `src/migration/wizard.rs`
- **Dependencies**: 2.3.1, 2.2.4, 2.1
- **Estimate**: 0.5 day

---

## Phase 3: Integration & Testing (Week 3, 7 days)

### 3.1 Main Processing Loop Integration
**Priority**: Critical | **Estimate**: 2 days | **Phase**: 3

#### Task 3.1.1: Update UploaderService struct
- [ ] Add auth_client: Arc<Mutex<AuthClient>>
- [ ] Add batch_manager: Arc<Mutex<BatchManager>>
- [ ] Add error_reporter: Arc<ErrorReporter>
- [ ] Update constructor with new dependencies
- **Files**: `src/service/uploader.rs`
- **Dependencies**: 1.1, 1.2, 1.3
- **Estimate**: 0.5 day

#### Task 3.1.2: Implement run_scheduled_batch() v2
- [ ] Add authentication check with renewal
- [ ] Call batch_manager.start_batch()
- [ ] Scan DBF files
- [ ] Convert and compress files
- [ ] Implement locked file retry logic
- [ ] Call batch_manager.upload_files() in chunks
- [ ] Handle upload failures with error reporting
- [ ] Call batch_manager.complete_batch()
- [ ] Cleanup temporary files
- [ ] Add comprehensive logging
- **Files**: `src/service/uploader.rs`
- **Dependencies**: 3.1.1
- **Estimate**: 1.5 days

---

### 3.2 Contract Tests
**Priority**: Critical | **Estimate**: 2 days | **Phase**: 3

#### Task 3.2.1: Setup test infrastructure
- [ ] Create test utilities module
- [ ] Setup mock middleware server
- [ ] Add test fixtures for DBF files
- [ ] Configure test environment
- **Files**: `tests/common/mod.rs` (new)
- **Dependencies**: None
- **Estimate**: 0.5 day

#### Task 3.2.2: Authentication contract tests
- [ ] Test auth with valid site credentials
- [ ] Test auth with invalid credentials
- [ ] Test auth endpoint URL
- [ ] Test JWT payload parsing
- [ ] Test token renewal logic
- **Files**: `tests/auth_contract_test.rs` (new)
- **Dependencies**: 3.2.1, 1.1
- **Estimate**: 0.5 day

#### Task 3.2.3: Batch lifecycle contract tests
- [ ] Test batch start endpoint
- [ ] Test batch upload with multipart
- [ ] Test batch complete endpoint
- [ ] Test batch fail endpoint
- [ ] Test batch cancel endpoint
- [ ] Test state transitions
- **Files**: `tests/batch_contract_test.rs` (new)
- **Dependencies**: 3.2.1, 1.2
- **Estimate**: 0.5 day

#### Task 3.2.4: Error reporting contract tests
- [ ] Test batch error reporting endpoint
- [ ] Test standalone error endpoint
- [ ] Test error DTO serialization
- [ ] Test local log fallback
- **Files**: `tests/error_contract_test.rs` (new)
- **Dependencies**: 3.2.1, 1.3
- **Estimate**: 0.5 day

---

### 3.3 Integration Tests
**Priority**: High | **Estimate**: 2 days | **Phase**: 3

#### Task 3.3.1: Full upload workflow test
- [ ] Setup test middleware instance
- [ ] Create test DBF files
- [ ] Run complete upload workflow
- [ ] Verify files uploaded to middleware
- [ ] Verify batch completed successfully
- [ ] Verify error reporting works
- **Files**: `tests/integration_test.rs` (new)
- **Dependencies**: 3.2, 3.1
- **Estimate**: 1 day

#### Task 3.3.2: Token renewal integration test
- [ ] Create scenario with expired token
- [ ] Trigger automatic renewal
- [ ] Verify workflow continues
- [ ] Test renewal failure handling
- **Files**: `tests/integration_test.rs`
- **Dependencies**: 3.3.1
- **Estimate**: 0.5 day

#### Task 3.3.3: Batch timeout handling test
- [ ] Simulate middleware batch timeout
- [ ] Verify uploader handles gracefully
- [ ] Test new batch creation after timeout
- **Files**: `tests/integration_test.rs`
- **Dependencies**: 3.3.1
- **Estimate**: 0.5 day

---

### 3.4 End-to-End Testing
**Priority**: High | **Estimate**: 1 day | **Phase**: 3

#### Task 3.4.1: Windows service deployment test
- [ ] Install service on test Windows machine
- [ ] Run migration wizard
- [ ] Configure with test credentials
- [ ] Start service and verify startup
- [ ] Trigger scheduled batch
- [ ] Verify files uploaded
- [ ] Check logs for errors
- **Files**: Manual test plan
- **Dependencies**: All previous tasks
- **Estimate**: 0.5 day

#### Task 3.4.2: Production scenario simulation
- [ ] Test with large DBF files (>10MB)
- [ ] Test with 500+ files in batch
- [ ] Test locked file handling
- [ ] Test network interruption recovery
- [ ] Test authentication expiration
- [ ] Verify all error scenarios
- **Files**: Manual test plan
- **Dependencies**: 3.4.1
- **Estimate**: 0.5 day

---

## Phase 4: Deployment & Documentation (Week 4, 6 days)

### 4.1 Documentation Updates
**Priority**: High | **Estimate**: 2 days | **Phase**: 4

#### Task 4.1.1: Update README.md
- [ ] Add v2.0 overview and changes
- [ ] Document new configuration format
- [ ] Add migration instructions
- [ ] Update installation steps
- [ ] Add troubleshooting section
- **Files**: `README.md`
- **Dependencies**: None
- **Estimate**: 0.5 day

#### Task 4.1.2: Create MIGRATION_GUIDE.md
- [ ] Document v1 → v2 changes
- [ ] Step-by-step migration instructions
- [ ] Common issues and solutions
- [ ] Rollback procedures
- [ ] Add screenshots of wizard
- **Files**: `MIGRATION_GUIDE.md` (new)
- **Dependencies**: 4.1.1
- **Estimate**: 0.5 day

#### Task 4.1.3: Create CONFIGURATION.md
- [ ] Document all config options
- [ ] Provide config examples
- [ ] Explain batch settings
- [ ] Document endpoint configuration
- [ ] Add troubleshooting tips
- **Files**: `CONFIGURATION.md` (new)
- **Dependencies**: 4.1.1
- **Estimate**: 0.5 day

#### Task 4.1.4: Update API documentation
- [ ] Document middleware endpoints used
- [ ] Add request/response examples
- [ ] Document error codes
- [ ] Add sequence diagrams
- **Files**: `docs/API.md` (new)
- **Dependencies**: 4.1.1
- **Estimate**: 0.5 day

---

### 4.2 Windows Service Installer Update
**Priority**: High | **Estimate**: 2 days | **Phase**: 4

#### Task 4.2.1: Update installer script
- [ ] Modify installer to run migration check
- [ ] Add wizard invocation for new installs
- [ ] Update service registration
- [ ] Add config validation before install
- [ ] Update uninstall script
- **Files**: `installer/install.ps1`
- **Dependencies**: 2.2, 2.3
- **Estimate**: 1 day

#### Task 4.2.2: Test installer on clean Windows
- [ ] Test fresh install on Windows 10
- [ ] Test fresh install on Windows Server 2019
- [ ] Test upgrade from v1.0
- [ ] Test uninstall and cleanup
- [ ] Verify service starts correctly
- **Files**: Manual test plan
- **Dependencies**: 4.2.1
- **Estimate**: 1 day

---

### 4.3 Release Preparation
**Priority**: Critical | **Estimate**: 2 days | **Phase**: 4

#### Task 4.3.1: Version bump and changelog
- [ ] Update Cargo.toml to 2.0.0
- [ ] Create CHANGELOG.md with all changes
- [ ] Document breaking changes
- [ ] Document new features
- [ ] Add migration notes
- **Files**: `Cargo.toml`, `CHANGELOG.md` (new)
- **Dependencies**: None
- **Estimate**: 0.5 day

#### Task 4.3.2: Build release binaries
- [ ] Build Windows x64 release binary
- [ ] Build Windows x86 release binary (if needed)
- [ ] Test binaries on clean systems
- [ ] Generate installer packages
- [ ] Sign binaries (if required)
- **Files**: Build artifacts
- **Dependencies**: 4.3.1
- **Estimate**: 0.5 day

#### Task 4.3.3: Create GitHub release
- [ ] Tag v2.0.0 in git
- [ ] Create GitHub release
- [ ] Upload binaries and installers
- [ ] Write release notes
- [ ] Mark breaking changes clearly
- **Files**: GitHub release
- **Dependencies**: 4.3.2
- **Estimate**: 0.5 day

#### Task 4.3.4: User communication
- [ ] Send migration notification to users
- [ ] Provide migration timeline
- [ ] Offer migration support
- [ ] Schedule migration assistance calls
- **Files**: Email/notification templates
- **Dependencies**: 4.3.3
- **Estimate**: 0.5 day

---

## Task Dependencies Graph

```
Phase 1: Core Protocol Migration
├─ 1.1 Authentication Module
│  ├─ 1.1.1 SiteCredentials
│  ├─ 1.1.2 JwtToken (→ 1.1.1)
│  ├─ 1.1.3 JWT parsing (→ 1.1.2)
│  ├─ 1.1.4 AuthClient (→ 1.1.1, 1.1.2, 1.1.3)
│  ├─ 1.1.5 Token renewal (→ 1.1.4)
│  └─ 1.1.6 Error handling (→ 1.1.5)
│
├─ 1.2 Batch Lifecycle Manager
│  ├─ 1.2.1 BatchState
│  ├─ 1.2.2 Batch (→ 1.2.1)
│  ├─ 1.2.3 BatchManager (→ 1.2.2, 1.1)
│  ├─ 1.2.4 start_batch (→ 1.2.3)
│  ├─ 1.2.5 upload_files (→ 1.2.4)
│  ├─ 1.2.6 complete_batch (→ 1.2.5)
│  ├─ 1.2.7 fail_batch (→ 1.2.6)
│  ├─ 1.2.8 cancel_batch (→ 1.2.7)
│  └─ 1.2.9 Response DTOs
│
└─ 1.3 Error Reporting
   ├─ 1.3.1 ErrorLogRequest DTO
   ├─ 1.3.2 ErrorReporter (→ 1.3.1, 1.1)
   ├─ 1.3.3 report_batch_error (→ 1.3.2, 1.2)
   ├─ 1.3.4 report_standalone_error (→ 1.3.3)
   └─ 1.3.5 Local log fallback (→ 1.3.4)

Phase 2: Configuration & Migration
├─ 2.1 Configuration V2
│  ├─ 2.1.1 ConfigV2 struct
│  ├─ 2.1.2 Validation (→ 2.1.1)
│  └─ 2.1.3 Config loading (→ 2.1.1, 2.1.2)
│
├─ 2.2 Migration Tool
│  ├─ 2.2.1 ConfigMigration module
│  ├─ 2.2.2 Version detection (→ 2.2.1)
│  ├─ 2.2.3 migrate_v1_to_v2 (→ 2.2.2, 2.1)
│  └─ 2.2.4 Auth test utility (→ 1.1)
│
└─ 2.3 Migration Wizard
   ├─ 2.3.1 Interactive prompt
   └─ 2.3.2 run_wizard (→ 2.3.1, 2.2.4, 2.1)

Phase 3: Integration & Testing
├─ 3.1 Main Loop Integration
│  ├─ 3.1.1 Update UploaderService (→ 1.1, 1.2, 1.3)
│  └─ 3.1.2 run_scheduled_batch (→ 3.1.1)
│
├─ 3.2 Contract Tests
│  ├─ 3.2.1 Test infrastructure
│  ├─ 3.2.2 Auth tests (→ 3.2.1, 1.1)
│  ├─ 3.2.3 Batch tests (→ 3.2.1, 1.2)
│  └─ 3.2.4 Error tests (→ 3.2.1, 1.3)
│
├─ 3.3 Integration Tests
│  ├─ 3.3.1 Full workflow (→ 3.2, 3.1)
│  ├─ 3.3.2 Token renewal (→ 3.3.1)
│  └─ 3.3.3 Batch timeout (→ 3.3.1)
│
└─ 3.4 E2E Testing
   ├─ 3.4.1 Service deployment (→ All)
   └─ 3.4.2 Production simulation (→ 3.4.1)

Phase 4: Deployment & Documentation
├─ 4.1 Documentation
│  ├─ 4.1.1 README.md
│  ├─ 4.1.2 MIGRATION_GUIDE.md (→ 4.1.1)
│  ├─ 4.1.3 CONFIGURATION.md (→ 4.1.1)
│  └─ 4.1.4 API.md (→ 4.1.1)
│
├─ 4.2 Installer Update
│  ├─ 4.2.1 Installer script (→ 2.2, 2.3)
│  └─ 4.2.2 Installer testing (→ 4.2.1)
│
└─ 4.3 Release
   ├─ 4.3.1 Version bump
   ├─ 4.3.2 Build binaries (→ 4.3.1)
   ├─ 4.3.3 GitHub release (→ 4.3.2)
   └─ 4.3.4 User communication (→ 4.3.3)
```

---

## Quick Reference: Task Counts

**Total Tasks**: 66

### By Phase
- Phase 1 (Core Protocol): 21 tasks (32%)
- Phase 2 (Configuration): 9 tasks (14%)
- Phase 3 (Testing): 16 tasks (24%)
- Phase 4 (Deployment): 20 tasks (30%)

### By Priority
- Critical: 26 tasks (39%)
- High: 28 tasks (42%)
- Medium: 12 tasks (18%)

### By Module
- Authentication: 6 tasks
- Batch Manager: 9 tasks
- Error Reporting: 5 tasks
- Configuration: 3 tasks
- Migration Tool: 6 tasks
- Integration: 2 tasks
- Testing: 13 tasks
- Documentation: 8 tasks
- Release: 14 tasks

---

## Suggested Team Allocation

**Option 1: Single Developer (Full-time)**
- Duration: 4 weeks (28 days)
- Workload: ~3-4 tasks per day
- Focus: Sequential implementation following phases

**Option 2: Two Developers**
- Developer 1: Auth + Batch Manager (Phase 1)
- Developer 2: Error Reporting + Config + Migration (Phase 1-2)
- Both: Integration + Testing (Phase 3)
- Both: Documentation + Release (Phase 4)
- Duration: 2-3 weeks

**Option 3: Three Developers**
- Developer 1: Authentication Module
- Developer 2: Batch Manager
- Developer 3: Error Reporting + Configuration + Migration
- Duration: 2 weeks for core implementation
- Week 3: Integration + Testing (all)
- Week 4: Documentation + Release (all)

---

## Daily Standup Template

**Yesterday:**
- Task IDs completed
- Blockers resolved

**Today:**
- Task IDs in progress
- Expected completions

**Blockers:**
- Technical issues
- Dependency waits
- Questions needed

---

## Definition of Done

Each task is considered done when:
- [ ] Code implemented and compiles
- [ ] Unit tests written and passing
- [ ] Integration tests passing (where applicable)
- [ ] Code reviewed (if team)
- [ ] Documentation updated
- [ ] No compiler warnings
- [ ] Clippy lints passing
- [ ] Manual testing completed (for UI/service tasks)

---

**Document Status**: Task List Complete
**Ready For**: Implementation Sprint Planning
