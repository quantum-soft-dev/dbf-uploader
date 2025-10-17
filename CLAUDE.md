# dbf-uploader Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-10-17

## Active Technologies
- Rust (latest stable, targeting Windows 10+ / Windows Server 2016+) + windows-service crate for service management, cron parser for scheduling, reqwest for HTTP/HTTPS client, serde for TOML config, DBF parsing library, CSV writer, gzip compression, JWT validation library (001-technical-specifications-data)

## Project Structure
```
src/
├── auth/           # Authentication (SiteCredentials, TokenManager, JWT)
├── batch/          # Batch v2 protocol (BatchManager, DTOs, state machine)
├── config/         # Configuration (v1 legacy + v2)
├── error/          # Error handling and reporting
├── processor/      # DBF conversion, compression, scanning
├── service/        # UploaderService (orchestration)
└── models/         # Domain models

tests/
├── common/                      # Shared test utilities (MockMiddleware)
├── v2_auth_contract_test.rs    # Auth endpoint contracts
├── v2_batch_contract_test.rs   # Batch API contracts
├── v2_error_contract_test.rs   # Error reporting contracts
└── v2_integration_test.rs      # End-to-end workflows
```

## API Protocol (middleware v3.0.0)

### Batch v2 Endpoints
- **Auth**: `POST /api/v1/auth/token` (Basic Auth → JWT Bearer)
- **Batch Start**: `POST /api/dfc/batch/start` (Bearer token)
- **Upload**: `POST /api/dfc/batch/{batchId}/upload` (multipart/form-data)
- **Complete**: `POST /api/dfc/batch/{batchId}/complete`
- **Fail**: `POST /api/dfc/batch/{batchId}/fail`
- **Cancel**: `POST /api/dfc/batch/{batchId}/cancel`
- **Error Log**: `POST /api/v1/error` (standalone or batch errors)

### Response DTOs (camelCase JSON)
```rust
// Upload Response (matches server FileUploadController.java)
{
  "status": "OK",
  "uploadedFiles": 2,        // count (usize)
  "files": [                 // array of file info
    {
      "fileName": "test.csv.gz",
      "fileSize": 1024,
      "uploadedAt": "2025-10-06T10:35:00Z"
    }
  ]
}

// Batch Complete Response
{
  "batchId": "uuid",
  "status": "completed",
  "uploadedFilesCount": 5,
  "totalSize": 2048,
  "hasErrors": false
}
```

## Commands
- `cargo test` - Run all 209 tests (unit + contract + integration)
- `cargo clippy` - Linter checks
- `cargo build --release` - Production build

## Code Style
Rust (latest stable, targeting Windows 10+ / Windows Server 2016+): Follow standard conventions

## Recent Changes
- 2025-10-17: Updated Batch v2 protocol to match middleware v3.0.0 API (URL change: `/api/v1/batch` → `/api/dfc/batch`, DTO structure alignment)
- 2025-10-05: Added Rust (latest stable, targeting Windows 10+ / Windows Server 2016+) + windows-service crate for service management, cron parser for scheduling, reqwest for HTTP/HTTPS client, serde for TOML config, DBF parsing library, CSV writer, gzip compression, JWT validation library

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->