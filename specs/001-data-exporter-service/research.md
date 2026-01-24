# Research: Data Exporter Windows Service

**Branch**: `001-data-exporter-service` | **Date**: 2026-01-24

## Overview

This document consolidates research findings for implementing the Data Exporter Windows Service. The service exports DBF files to CSV format and uploads them to a remote server.

---

## Technology Decisions

### 1. Windows Service Framework

**Decision**: Use `windows-service` crate (v0.7)

**Rationale**:
- Native Rust implementation for Windows Service Control Manager integration
- Well-documented API for service lifecycle management (Start, Stop, Interrogate)
- Supports graceful shutdown via channel-based event handling
- Used successfully in existing codebase

**Alternatives Considered**:
- `service-manager` crate - Cross-platform but adds unnecessary abstraction for Windows-only service
- Manual WinAPI bindings - Too low-level, error-prone

**TDD Approach**:
- Service control handler logic can be unit tested with mock channels
- Integration tests require actual Windows service registration (CI/CD consideration)
- Use `#[ignore]` attribute for tests requiring admin privileges

### 2. Cron Scheduling

**Decision**: Use `tokio-cron-scheduler` crate (v0.13)

**Rationale**:
- Native async/Tokio support
- Standard 5-field cron expression parsing
- Job lifecycle notifications (on_start, on_stop, on_removed)
- Supports querying next execution time

**Alternatives Considered**:
- `cron` crate - Parsing only, no scheduler
- `job_scheduler` - Sync only, not Tokio-native

**TDD Approach**:
- Test cron expression parsing separately from scheduler
- Use `#[tokio::test(flavor = "multi_thread")]` for scheduler tests
- Mock time for deterministic testing where possible

### 3. DBF File Parsing

**Decision**: Use `dbase` crate (v0.5) with `yore` feature for encoding

**Rationale**:
- Supports dBase III, IV, and FoxPro formats
- Built-in encoding support via `yore` code pages (CP866, CP1251, CP1255)
- Iterative record reading (memory efficient for large files)
- Already integrated in codebase

**Alternatives Considered**:
- Manual parsing - Complex format, error-prone
- `shapefile` crate - Includes DBF but focused on GIS

**TDD Approach**:
- Create test fixture DBF files with known content
- Test encoding detection and conversion
- Test corrupted record handling (skip and continue)

### 4. HTTP Client

**Decision**: Use `reqwest` crate (v0.12) with `native-tls`, `multipart`, `json` features

**Rationale**:
- Industry-standard Rust HTTP client
- Native multipart form support for file uploads
- Built-in retry and timeout support
- Already integrated in codebase

**Alternatives Considered**:
- `hyper` - Lower-level, more boilerplate
- `surf` - Less mature ecosystem

**TDD Approach**:
- Use `wiremock` crate for HTTP mock server in tests
- Test retry logic with simulated failures
- Test multipart form construction separately from actual upload

### 5. Testing Framework

**Decision**: Standard `cargo test` with `wiremock` for HTTP mocking

**Rationale**:
- Native Rust testing infrastructure
- `wiremock` provides flexible HTTP mock server
- `tempfile` crate for filesystem fixtures
- Supports async tests via `#[tokio::test]`

**Dependencies for Testing**:
```toml
[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
tokio-test = "0.4"
```

---

## TDD Implementation Strategy

### Test Categories

1. **Unit Tests** (`tests/unit/`)
   - Pure function testing (encoding conversion, CSV formatting)
   - Model validation (Config, Batch, ErrorReport)
   - No external dependencies

2. **Integration Tests** (`tests/integration/`)
   - Component interaction (Scanner + Converter + Compressor)
   - File system operations with `tempfile`
   - Database fixtures with known DBF files

3. **Contract Tests** (`tests/contract/`)
   - API contract verification against server specification
   - HTTP mock server with expected request/response pairs
   - Existing structure: `auth_test.rs`, `error_report_test.rs`, `upload_test.rs`

### Red-Green-Refactor Workflow

For each feature (mapped to User Stories):

1. **RED**: Write failing test based on acceptance criteria
2. **GREEN**: Implement minimum code to pass test
3. **REFACTOR**: Improve code quality while maintaining green tests

### Test File Naming Convention

```
tests/
├── unit/
│   ├── converter_test.rs      # DBF to CSV conversion
│   ├── compressor_test.rs     # GZIP compression
│   ├── filter_test.rs         # Include/exclude patterns
│   └── scheduler_test.rs      # Cron expression parsing
├── integration/
│   ├── batch_processing_test.rs
│   ├── vss_copy_test.rs
│   └── config_reload_test.rs
└── contract/
    ├── auth_test.rs           # (existing)
    ├── upload_test.rs         # (existing)
    ├── error_report_test.rs   # (existing)
    └── batch_api_test.rs      # (new)
```

---

## Critical Implementation Notes

### Windows Service Testing Limitations

1. **Admin Privileges**: Service installation/start/stop requires elevated permissions
2. **CI/CD Strategy**: Use `#[ignore]` for admin-requiring tests, run manually or in privileged CI runner
3. **Workaround**: Test service logic in isolation from Windows SCM integration

### VSS (Volume Shadow Copy) Testing

1. **Availability**: VSS may not be available on all test systems
2. **Strategy**: Skip VSS tests with `#[cfg_attr(not(feature = "vss-tests"), ignore)]`
3. **Mocking**: Use file system abstraction to mock locked files

### Encoding Test Fixtures

Create test DBF files with various encodings:
- `fixtures/cp866_russian.dbf` - Cyrillic CP866
- `fixtures/cp1251_russian.dbf` - Windows-1251 Cyrillic
- `fixtures/cp1255_hebrew.dbf` - Windows-1255 Hebrew
- `fixtures/utf8_unicode.dbf` - UTF-8 (if supported by DBF format)
- `fixtures/corrupted_records.dbf` - File with known bad records

---

## API Contract Summary

Based on existing contract tests, the API endpoints are:

| Endpoint | Method | Auth | Purpose |
|----------|--------|------|---------|
| `/api/auth/login` | POST | Basic | Obtain JWT token |
| `/api/files/upload` | POST | Bearer | Upload gzipped CSV file |
| `/api/errors` | POST | Bearer | Report errors |
| `/api/batches/start` | POST | Bearer | Register batch start |
| `/api/batches/{id}/complete` | POST | Bearer | Report batch completion |

---

## Next Steps

1. Create data model documentation (`data-model.md`)
2. Generate API contracts in OpenAPI format (`contracts/`)
3. Create quickstart guide (`quickstart.md`)
4. Generate task breakdown for TDD implementation (`tasks.md`)
