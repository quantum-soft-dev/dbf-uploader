
# Implementation Plan: Data Exporter Service

**Branch**: `001-technical-specifications-data` | **Date**: 2025-10-05 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-technical-specifications-data/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → If not found: ERROR "No feature spec at {path}"
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → Detect Project Type from file system structure or context (web=frontend+backend, mobile=app+api)
   → Set Structure Decision based on project type
3. Fill the Constitution Check section based on the content of the constitution document.
4. Evaluate Constitution Check section below
   → If violations exist: Document in Complexity Tracking
   → If no justification possible: ERROR "Simplify approach first"
   → Update Progress Tracking: Initial Constitution Check
5. Execute Phase 0 → research.md
   → If NEEDS CLARIFICATION remain: ERROR "Resolve unknowns"
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, agent-specific template file (e.g., `CLAUDE.md` for Claude Code, `.github/copilot-instructions.md` for GitHub Copilot, `GEMINI.md` for Gemini CLI, `QWEN.md` for Qwen Code, or `AGENTS.md` for all other agents).
7. Re-evaluate Constitution Check section
   → If new violations: Refactor design, return to Phase 1
   → Update Progress Tracking: Post-Design Constitution Check
8. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
9. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
A Windows service application that automatically exports DBF database files to CSV format, compresses them to gzip, and uploads them to a cloud server on a configurable schedule. The service runs unattended, handles encoding detection, manages authentication via JWT tokens, implements robust error handling with server-side reporting, and supports dynamic configuration reloading.

## Technical Context
**Language/Version**: Rust (latest stable, targeting Windows 10+ / Windows Server 2016+)
**Primary Dependencies**: windows-service crate for service management, cron parser for scheduling, reqwest for HTTP/HTTPS client, serde for TOML config, DBF parsing library, CSV writer, gzip compression, JWT validation library
**Storage**: File system (DBF files in configured directory, local error.log fallback, TOML config file)
**Testing**: cargo test with unit tests, integration tests for service lifecycle, contract tests for API endpoints
**Target Platform**: Windows 10+ and Windows Server 2016+ (x86_64 architecture)
**Project Type**: single (command-line service application)
**Performance Goals**: Process thousands of DBF files recursively with minimal idle resource consumption, handle large files efficiently
**Constraints**: Must run as Windows Service, administrator rights required for installation, HTTPS-only communication, minimal resource usage when idle
**Scale/Scope**: Single-binary executable, CLI install/uninstall interface, scheduled batch processing of arbitrary file counts

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Status**: PASS - No constitution file exists (placeholder template), proceeding with standard software engineering best practices:
- Single-purpose application (Windows service for DBF export)
- Clear separation of concerns (CLI, service, processing, networking)
- Testable components with unit and integration tests
- Well-defined error handling and observability

## Project Structure

### Documentation (this feature)
```
specs/[###-feature]/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
src/
├── main.rs                 # CLI entry point (install, uninstall commands)
├── service/
│   ├── mod.rs              # Windows service implementation
│   ├── scheduler.rs        # Cron-based scheduling logic
│   └── lifecycle.rs        # Service start/stop/restart handlers
├── config/
│   ├── mod.rs              # Configuration loading and validation
│   └── watcher.rs          # Config file change detection
├── auth/
│   ├── mod.rs              # JWT token management
│   └── client.rs           # HTTP Basic auth for token retrieval
├── processor/
│   ├── mod.rs              # Core file processing orchestration
│   ├── scanner.rs          # Recursive DBF file discovery
│   ├── converter.rs        # DBF to CSV conversion with encoding
│   ├── compressor.rs       # Gzip compression
│   └── uploader.rs         # HTTP file upload with multipart
├── models/
│   ├── mod.rs              # Data structures
│   ├── config.rs           # Config TOML structure
│   ├── error_report.rs     # Error report JSON structure
│   └── batch.rs            # Batch processing state
├── error/
│   ├── mod.rs              # Error types and handling
│   └── logger.rs           # Local fallback logging
└── lib.rs                  # Library exports

tests/
├── contract/
│   ├── auth_test.rs        # JWT token endpoint contract
│   ├── upload_test.rs      # File upload endpoint contract
│   └── error_test.rs       # Error report endpoint contract
├── integration/
│   ├── service_lifecycle_test.rs
│   ├── batch_processing_test.rs
│   └── config_reload_test.rs
└── unit/
    ├── converter_test.rs
    ├── scanner_test.rs
    └── scheduler_test.rs

Cargo.toml                  # Rust dependencies and metadata
```

**Structure Decision**: Single Rust application using standard cargo project layout. Modular design with clear separation: service lifecycle, scheduling, authentication, file processing, and error handling as distinct modules.

## Phase 0: Outline & Research
1. **Extract unknowns from Technical Context** above:
   - For each NEEDS CLARIFICATION → research task
   - For each dependency → best practices task
   - For each integration → patterns task

2. **Generate and dispatch research agents**:
   ```
   For each unknown in Technical Context:
     Task: "Research {unknown} for {feature context}"
   For each technology choice:
     Task: "Find best practices for {tech} in {domain}"
   ```

3. **Consolidate findings** in `research.md` using format:
   - Decision: [what was chosen]
   - Rationale: [why chosen]
   - Alternatives considered: [what else evaluated]

**Output**: research.md with all NEEDS CLARIFICATION resolved

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

1. **Extract entities from feature spec** → `data-model.md`:
   - Entity name, fields, relationships
   - Validation rules from requirements
   - State transitions if applicable

2. **Generate API contracts** from functional requirements:
   - For each user action → endpoint
   - Use standard REST/GraphQL patterns
   - Output OpenAPI/GraphQL schema to `/contracts/`

3. **Generate contract tests** from contracts:
   - One test file per endpoint
   - Assert request/response schemas
   - Tests must fail (no implementation yet)

4. **Extract test scenarios** from user stories:
   - Each story → integration test scenario
   - Quickstart test = story validation steps

5. **Update agent file incrementally** (O(1) operation):
   - Run `.specify/scripts/bash/update-agent-context.sh claude`
     **IMPORTANT**: Execute it exactly as specified above. Do not add or remove any arguments.
   - If exists: Add only NEW tech from current plan
   - Preserve manual additions between markers
   - Update recent changes (keep last 3)
   - Keep under 150 lines for token efficiency
   - Output to repository root

**Output**: data-model.md, /contracts/*, failing tests, quickstart.md, agent-specific file

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:

1. **Foundation Setup** (5-7 tasks)
   - Cargo project initialization and dependencies
   - Project structure creation (modules, directories)
   - Configuration models and TOML parsing
   - Error type definitions (thiserror)

2. **Contract Tests** (3 tasks) [P]
   - Auth API contract test (auth_test.rs)
   - Upload API contract test (upload_test.rs)
   - Error Report API contract test (error_test.rs)
   - All tests should FAIL initially (TDD)

3. **Core Models** (4-5 tasks) [P]
   - Config struct (config.rs)
   - Error Report struct (error_report.rs)
   - Batch struct (batch.rs)
   - JWT Token management
   - DBF File processing status

4. **Authentication & HTTP Client** (3-4 tasks)
   - HTTP client setup with reqwest (HTTPS-only, rustls)
   - Basic auth for token retrieval
   - JWT token storage and renewal logic
   - Make auth contract tests pass

5. **File Processing Pipeline** (6-8 tasks)
   - Directory scanner (recursive DBF discovery)
   - DBF to CSV converter with encoding detection
   - CSV to gzip compressor
   - Filename path encoding (subdirs → underscores)
   - Make upload contract tests pass
   - Locked file deferral and retry logic

6. **Scheduler & Service Lifecycle** (4-5 tasks)
   - Cron parser and scheduler setup (tokio-cron-scheduler)
   - Windows service registration and lifecycle
   - Config file watcher (notify-debouncer-mini)
   - Config reload at batch start (not mid-batch)
   - Service control event handlers

7. **Error Handling & Reporting** (3-4 tasks)
   - Error reporter (send to API)
   - Fallback local logger (error.log)
   - Error handling throughout processing pipeline
   - Make error report contract tests pass

8. **CLI Interface** (3-4 tasks)
   - Install command (copy exe, register service, create config, validate credentials)
   - Uninstall command (stop service, unregister, cleanup)
   - Command-line argument parsing
   - Initial credential validation

9. **Integration Tests** (4-5 tasks)
   - Service lifecycle test (install, start, stop, uninstall)
   - Full batch processing test (scan → convert → compress → upload)
   - Config reload test (change file during operation)
   - Locked file retry test
   - Error handling test (corrupted file, network failure)

10. **Cleanup & Polish** (2-3 tasks)
    - CSV file deletion after upload (success or failure)
    - Logging and observability (tracing)
    - Documentation review
    - Quickstart validation

**Ordering Strategy**:
- **TDD Order**: Contract tests → Models → Implementation → Make tests pass
- **Dependency Order**:
  1. Foundation (Cargo, project structure, error types)
  2. Models (Config, Error Report, Batch)
  3. Contract tests (all failing initially)
  4. Authentication (make auth tests pass)
  5. File processing (make upload tests pass)
  6. Error reporting (make error tests pass)
  7. Scheduler & service (integration)
  8. CLI interface
  9. Integration tests
  10. Cleanup

- **Parallel Execution** [P]:
  - Contract tests can be written in parallel
  - Model structs can be created in parallel
  - Individual module implementations (after contracts) can be parallel

**Estimated Output**: 35-45 numbered, ordered tasks in tasks.md

**Task Grouping**:
- Each task is a discrete unit of work (1-3 hours estimated)
- Tasks marked [P] can be done in parallel (no blocking dependencies)
- Integration tests come after all unit functionality complete
- Quickstart validation is the final acceptance gate

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)  
**Phase 4**: Implementation (execute tasks.md following constitutional principles)  
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |


## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command)
- [x] Phase 1: Design complete (/plan command)
- [x] Phase 2: Task planning complete (/plan command - describe approach only)
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
- [x] All NEEDS CLARIFICATION resolved
- [x] Complexity deviations documented (N/A - no deviations)

**Artifacts Generated**:
- [x] research.md - Technology decisions and library selections
- [x] data-model.md - Entities, relationships, validation rules, state machines
- [x] contracts/auth-api.md - Authentication endpoint specification
- [x] contracts/upload-api.md - File upload endpoint specification
- [x] contracts/error-report-api.md - Error reporting endpoint specification
- [x] quickstart.md - Installation and testing guide
- [x] CLAUDE.md - Agent context file updated

**Ready for /tasks Command**: Yes - All design artifacts complete, approach described

---
*Based on Constitution (placeholder template) - See `/.specify/memory/constitution.md`*
