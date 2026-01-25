# Tasks: Configuration Module

**Input**: Design documents from `/specs/002-configuration-module/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Status**: Feature is **substantially implemented**. Tasks focus on bug fixes, missing tests, and optional enhancements.

**Tests**: Yes - test tasks are included to address identified gaps (SC-001, SC-002, SC-005)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US3, US7)
- Include exact file paths in descriptions

## Path Conventions

Based on existing workspace structure:
- **common/**: Shared library (models, auth, error handling)
- **service/**: Windows service implementation
- **configurator/**: Native Windows GUI
- **tests/**: Integration tests

---

## Phase 1: Setup (No Tasks Required)

**Purpose**: Project initialization and basic structure

**Status**: ✅ Complete - workspace structure already exists

---

## Phase 2: Foundational (No Tasks Required)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**Status**: ✅ Complete - all foundational components are implemented

- Config struct with serde derive macros ✅
- Validation framework ✅
- Error types with thiserror ✅
- Logging infrastructure with tracing ✅

**Checkpoint**: Foundation ready - proceeding to bug fixes and tests

---

## Phase 3: User Story 3 - Hot-Reload Configuration Changes (Priority: P1-Critical) 🔧 BUG FIX

**Goal**: Fix critical bug where ConfigWatcher doesn't actually start monitoring the config file

**Independent Test**: Modify config.toml while service is running; verify change is detected within 2-second debounce window

### Implementation for User Story 3

- [X] T001 [US3] Fix ConfigWatcher to call `watch()` to start file monitoring in service/src/config/watcher.rs
- [X] T002 [US3] Add unit test for ConfigWatcher initialization verifying watcher is active in service/src/config/watcher.rs

**Details for T001**:
Add after debouncer creation (around line 66):
```rust
debouncer.watcher().watch(&watch_path, notify::RecursiveMode::NonRecursive)
    .map_err(|e| ProcessingError::ConfigurationError(
        format!("Failed to watch config directory: {}", e)
    ))?;
```

**Checkpoint**: ConfigWatcher now properly monitors config file changes

---

## Phase 4: User Story 3 - Hot-Reload Performance Test (Priority: P2)

**Goal**: Verify SC-002 success criterion - hot-reload detects and applies changes within 5 seconds

**Independent Test**: Run test that modifies config file and measures time to detection

### Tests for User Story 3

- [X] T003 [P] [US3] Add integration test for SC-002 (5s hot-reload detection) in service/tests/config_reload_sc002_test.rs

**Test Requirements (T003)**:
1. Start ConfigWatcher
2. Modify config file
3. Assert change is detected within 5 seconds (including 2-second debounce)
4. Assert new config values are loaded correctly

**Checkpoint**: Hot-reload performance verified against SC-002

---

## Phase 5: User Story 1 - Load Configuration Performance Test (Priority: P2)

**Goal**: Verify SC-001 success criterion - configuration loads and validates within 100ms

**Independent Test**: Run benchmark test measuring config load time

### Tests for User Story 1

- [X] T004 [P] [US1] Add benchmark test for SC-001 (100ms config load) in service/tests/config_load_test.rs

**Test Requirements (T004)**:
1. Create valid config file with all sections populated
2. Measure time to load and validate
3. Assert completes within 100ms
4. Run multiple iterations for statistical significance

**Checkpoint**: Config load performance verified against SC-001

---

## Phase 6: User Story 9 - Graceful Shutdown Test (Priority: P2)

**Goal**: Verify SC-005 success criterion - service responds to stop commands within 5 seconds during retry wait

**Independent Test**: Start service with unavailable source_dir, issue stop command, verify prompt termination

### Tests for User Story 9

- [X] T005 [P] [US9] Add integration test for SC-005 (5s stop response during retry) in service/tests/graceful_shutdown_test.rs

**Test Requirements (T005)**:
1. Configure service with non-existent source_dir to trigger retry loop
2. Start service in retry mode
3. Issue stop signal
4. Assert service terminates within 5 seconds
5. Verify no cleanup errors

**Checkpoint**: Graceful shutdown behavior verified against SC-005

---

## Phase 7: User Story 7 - GUI Pattern Configuration (Priority: P3) - OPTIONAL

**Goal**: Allow users to configure include/exclude patterns through the GUI configurator

**Independent Test**: Open configurator GUI, add patterns, save, verify patterns appear in config.toml

### Implementation for User Story 7

- [X] T006 [US7] Add include_patterns and exclude_patterns fields to GUI form in configurator/src/ui/app.rs
- [X] T007 [US7] Update config_manager.rs to load/save pattern arrays in configurator/src/config_manager.rs (config_manager already handles this via common::models::Config)
- [X] T008 [US7] Add pattern validation in GUI before save in configurator/src/ui/app.rs

**Checkpoint**: GUI now supports full file filtering configuration

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation and validation

- [X] T009 Verify all credential fields never appear in log output (SC-004 audit) across common/, service/, configurator/
- [X] T010 Run quickstart.md validation scenarios to ensure documentation accuracy
- [X] T011 Update CLAUDE.md if any new technologies or patterns were added

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1-2**: ✅ Already complete
- **Phase 3 (Bug Fix)**: No dependencies - **CRITICAL, do first**
- **Phase 4-6 (Tests)**: Depend on Phase 3 completion for US3 tests; US1 and US9 tests can run in parallel
- **Phase 7 (GUI Enhancement)**: Optional, can be done any time after Phase 3
- **Phase 8 (Polish)**: Depends on all other phases

### User Story Dependencies

- **User Story 3 (Bug Fix)**: No dependencies - fixes critical defect
- **User Story 1 (Test)**: No dependencies - tests already-working functionality
- **User Story 9 (Test)**: No dependencies - tests already-working functionality
- **User Story 7 (Enhancement)**: No dependencies on other stories

### Parallel Opportunities

Tasks marked [P] can run in parallel:
- T003, T004, T005 can all run in parallel (different test files)

---

## Parallel Example: Test Tasks

```bash
# After T001-T002 complete, launch all test tasks together:
Task: T003 - "Integration test for SC-002 in tests/integration/config_reload_test.rs"
Task: T004 - "Benchmark test for SC-001 in tests/integration/config_load_test.rs"
Task: T005 - "Integration test for SC-005 in tests/integration/graceful_shutdown_test.rs"
```

---

## Implementation Strategy

### Priority Order

1. **Critical Bug Fix (T001-T002)**: Fix ConfigWatcher immediately
2. **Performance Tests (T003-T005)**: Verify success criteria SC-001, SC-002, SC-005
3. **GUI Enhancement (T006-T008)**: Optional quality-of-life improvement
4. **Polish (T009-T011)**: Security audit and documentation validation

### Minimum Viable Fix

1. Complete T001 (fix watch() call)
2. Complete T002 (unit test for watcher)
3. **STOP and VALIDATE**: Test hot-reload manually
4. Deploy fix

### Full Implementation

1. T001-T002 → Bug fixed
2. T003-T005 → All success criteria verified
3. T006-T008 → GUI fully featured (optional)
4. T009-T011 → Documentation and security validated

---

## Implementation Status Summary

| User Story | Priority | Status | Remaining Tasks |
|------------|----------|--------|-----------------|
| US1: Load Configuration | P1 | ✅ Implemented | T004 (performance test) |
| US2: Validate Configuration | P1 | ✅ Implemented | None |
| US3: Hot-Reload Changes | P2 | ⚠️ Bug | T001, T002, T003 |
| US4: Traditional Auth | P1 | ✅ Implemented | None |
| US5: Device Flow Auth | P2 | ✅ Implemented | None |
| US6: Glob Pattern Filter | P2 | ✅ Implemented | None |
| US7: GUI Configuration | P3 | ⚠️ Partial | T006, T007, T008 |
| US8: Network Drive Retry | P2 | ✅ Implemented | None |
| US9: Graceful Shutdown | P2 | ✅ Implemented | T005 (test) |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Most implementation is complete; focus on bug fix and test coverage
- T001 is the critical path - fixes a blocking defect in hot-reload
- GUI enhancement (T006-T008) is optional based on stakeholder priority
- Verify tests fail before implementing (for new tests)
- Commit after each task or logical group
