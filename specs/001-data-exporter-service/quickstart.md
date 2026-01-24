# Quickstart: Data Exporter Windows Service TDD Development

**Branch**: `001-data-exporter-service` | **Date**: 2026-01-24

## Prerequisites

- Rust 1.82.0+ (as specified in `Cargo.toml`)
- Windows 10+ or Windows Server 2016+
- Administrator privileges (for service installation tests)

## Project Structure

```
dbf-uploader/
├── common/                 # Shared library
│   └── src/
│       ├── models/         # Data models (Config, Batch, DbfFile, ErrorReport)
│       ├── auth/           # Authentication (device flow)
│       └── error/          # Error handling utilities
├── service/                # Windows service binary
│   └── src/
│       ├── service/        # Windows SCM integration
│       ├── processor/      # File processing pipeline
│       ├── config/         # Configuration loading
│       └── vss/            # Volume Shadow Copy
├── configurator/           # GUI configurator
└── tests/                  # Integration & contract tests
    ├── contract/           # API contract tests
    ├── integration/        # Component integration tests
    └── fixtures/           # Test DBF files
```

## Quick Commands

```bash
# Run all tests
cargo test

# Run specific test module
cargo test --package data-exporter-service converter

# Run tests with output
cargo test -- --nocapture

# Run ignored tests (admin required)
cargo test -- --ignored

# Check code quality
cargo clippy

# Build release
cargo build --release
```

---

## TDD Workflow

### Step 1: Write Failing Test (RED)

Example: Testing DBF to CSV conversion

```rust
// service/src/processor/converter.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_convert_dbf_with_cp866_encoding() {
        // Arrange
        let temp_dir = TempDir::new().unwrap();
        let dbf_path = temp_dir.path().join("test_cp866.dbf");

        // Create test DBF file with CP866 content
        create_test_dbf_cp866(&dbf_path);

        let dbf_file = DbfFile::new(dbf_path.clone(), temp_dir.path());
        let config = create_test_config_with_encoding("CP866");

        // Act
        let result = convert_dbf_to_csv_memory(&dbf_file, &config);

        // Assert
        assert!(result.is_ok());
        let data = result.unwrap();
        match data {
            ProcessingData::InMemory(csv_bytes) => {
                let csv_str = String::from_utf8(csv_bytes).unwrap();
                assert!(csv_str.contains("Тестовые данные")); // Cyrillic text
            }
            _ => panic!("Expected in-memory data"),
        }
    }
}
```

### Step 2: Implement Minimum Code (GREEN)

```rust
pub fn convert_dbf_to_csv_memory(dbf_file: &DbfFile, config: &Config) -> Result<ProcessingData> {
    match get_encoding_enum(dbf_file.encoding.as_ref(), &config.encoding.dbf_encoding) {
        Encoding::CP866 => convert_with_encoding(dbf_file, LossyCodePage(CP866)),
        // ... other encodings
    }
}
```

### Step 3: Refactor (REFACTOR)

- Extract common patterns
- Improve error messages
- Add documentation

---

## Test Categories

### Unit Tests

Located in module `#[cfg(test)]` blocks.

```rust
// common/src/models/batch.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_batch_creation() {
        let config = create_test_config();
        let batch = Batch::new(config);

        assert_eq!(batch.status, BatchStatus::Scanning);
        assert_eq!(batch.processed_count, 0);
    }
}
```

### Integration Tests

Located in `tests/` directory.

```rust
// tests/integration/batch_processing_test.rs
#[tokio::test]
async fn test_full_batch_processing_pipeline() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    // Configure mocks for batch API
    Mock::given(method("POST"))
        .and(path("/api/batches/start"))
        .respond_with(ResponseTemplate::new(201)
            .set_body_json(json!({"batch_id": "test-uuid"})))
        .mount(&mock_server)
        .await;

    // ... test full pipeline
}
```

### Contract Tests

Verify API contracts match server specification.

```rust
// tests/contract/upload_test.rs
#[tokio::test]
#[ignore] // Requires test server
async fn test_upload_file_success() {
    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .file("file", "/path/to/test.csv.gz")
        .await
        .unwrap();

    let response = client
        .post("https://api.example.com/api/files/upload")
        .header("Authorization", "Bearer test-token")
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);
}
```

---

## Mock Server Setup

Add to `Cargo.toml`:

```toml
[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
```

Usage:

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};

#[tokio::test]
async fn test_upload_with_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/files/upload"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(201)
            .set_body_json(serde_json::json!({
                "status": "uploaded",
                "file_id": "uuid-here"
            })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Use mock_server.uri() as base URL in tests
    let base_url = mock_server.uri();
    // ... test code
}
```

---

## Test Fixtures

Create test DBF files in `tests/fixtures/`:

```rust
// tests/fixtures/mod.rs
use dbase::{FieldValue, Record, TableWriterBuilder, FieldInfo};
use std::path::Path;

pub fn create_test_dbf_cp866(path: &Path) {
    let fields = vec![
        FieldInfo::new("NAME".to_string(), dbase::FieldType::Character, 50),
        FieldInfo::new("VALUE".to_string(), dbase::FieldType::Numeric, 10),
    ];

    let mut writer = TableWriterBuilder::new()
        .add_field(fields[0].clone())
        .add_field(fields[1].clone())
        .build_with_file_dest(path)
        .unwrap();

    // Add test records...
    writer.close().unwrap();
}
```

---

## Acceptance Test Template

Map each User Story acceptance scenario to a test:

```rust
// tests/integration/user_story_1_scheduled_export.rs

/// US1-AC1: Given service is running with valid cron schedule,
/// When scheduled time arrives, Then batch processing triggers
#[tokio::test]
async fn us1_ac1_scheduled_batch_triggers() {
    // Arrange: Configure service with test cron
    // Act: Advance time to trigger point
    // Assert: Batch processing initiated
}

/// US1-AC2: Given batch is running,
/// When next schedule arrives, Then execution is skipped
#[tokio::test]
async fn us1_ac2_concurrent_batch_skipped() {
    // ...
}
```

---

## Running Tests

### All Tests

```bash
cargo test
```

### Specific Package

```bash
cargo test --package common
cargo test --package data-exporter-service
```

### With Coverage (using cargo-tarpaulin)

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### Windows Service Tests (Admin Required)

```bash
# Run as Administrator
cargo test --package data-exporter-service -- --ignored
```

---

## CI/CD Considerations

1. **Standard Tests**: Run on every PR
2. **Ignored Tests**: Run in privileged CI runner or manually
3. **Contract Tests**: Run against staging server or mock

```yaml
# Example GitHub Actions
jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test
      - run: cargo clippy -- -D warnings
```

---

## Next Steps

1. Run `/speckit.tasks` to generate implementation tasks
2. Start with highest priority user stories (P1)
3. Follow TDD cycle: RED -> GREEN -> REFACTOR
