# Changelog

All notable changes to the Data Exporter Service project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.1] - 2025-10-17

### ⚠️ Breaking Changes

#### Batch API URL Update (middleware v3.0.0 compatibility)

- **Batch endpoints moved** from `/api/v1/batch` to `/api/dfc/batch`:
  - `POST /api/dfc/batch/start` (was `/api/v1/batch/start`)
  - `POST /api/dfc/batch/{batchId}/upload` (was `/api/v1/batch/{batchId}/upload`)
  - `POST /api/dfc/batch/{batchId}/complete` (was `/api/v1/batch/{batchId}/complete`)
  - `POST /api/dfc/batch/{batchId}/fail` (was `/api/v1/batch/{batchId}/fail`)
  - `POST /api/dfc/batch/{batchId}/cancel` (was `/api/v1/batch/{batchId}/cancel`)

### 🔄 Changed

#### Upload Response DTO Structure

- **UploadResponse fields updated** to match server (`FileUploadController.java`):
  ```json
  {
    "status": "OK",             // New field
    "uploadedFiles": 2,         // Now integer count (was array)
    "files": [                  // New field name (was uploadedFiles)
      {
        "fileName": "test.csv.gz",    // camelCase (was filename)
        "fileSize": 1024,             // camelCase (was size)
        "uploadedAt": "2025-10-06..."  // New field
      }
    ]
  }
  ```

### 🐛 Fixed

- Updated all internal URLs to match data-forge-middleware v3.0.0
- Fixed DTO deserialization to match server response format
- Updated 209 tests to use correct API endpoints

### 📊 Test Coverage

- **All 209 tests passing**:
  - 156 unit tests
  - 40 contract tests (API validation)
  - 13 integration tests

## [2.0.0] - 2025-10-17

### ⚠️ Breaking Changes

This is a major version release with significant breaking changes. **Migration from v1.0 is required.**

#### Authentication Changes
- **Replaced username/password with site credentials**: Authentication now uses `domain` + `client_secret` instead of `username` + `password`
- **New authentication endpoint**: `/api/auth/token` → `/api/v1/auth/token`
- **Enhanced JWT payload**: Tokens now include `siteId`, `accountId`, `domain`, and `exp` claims

#### Batch Protocol (Replaces Single-File Upload)
- **New batch-based upload workflow**: Files are now uploaded in batches with lifecycle management
- **Batch lifecycle states**: `InProgress` → `Completed` / `Failed` / `Cancelled`
- **Five new batch endpoints** (updated to `/api/dfc/batch` in v2.0.1):
  - `POST /api/dfc/batch/start` - Create batch and get batch ID
  - `POST /api/dfc/batch/{batchId}/upload` - Upload files to batch
  - `POST /api/dfc/batch/{batchId}/complete` - Finalize successful batch
  - `POST /api/dfc/batch/{batchId}/fail` - Mark batch as failed
  - `POST /api/dfc/batch/{batchId}/cancel` - Cancel batch
- **Removed single-file upload endpoint**: `/api/upload` is deprecated (see `upload-api-v1-deprecated.md`)

#### Configuration Format Changes
- **New TOML structure** with organized sections:
  - `[auth]` - Site credentials (domain, client_secret)
  - `[api]` - API settings (base_url)
  - `[source]` - Source directory settings (directory)
  - `[schedule]` - Cron schedule (crontab)
  - `[encoding]` - Encoding settings (dbf_encoding)
  - `[batch]` - Batch-specific settings (max_files_per_batch, chunk_size_mb, max_retries, retry_delay_seconds)
  - `[logging]` - Logging configuration (level, error_log_path)
- **Field renames**:
  - `username` → `domain`
  - `password` → `client_secret`
  - `source_directory` → `directory`
  - `encoding` → `dbf_encoding`
- **Removed fields**: `api_url` (split into `base_url` and batch endpoints)

#### API Response Format Changes
- **All JSON fields use camelCase**: `expires_in` → `expiresIn`, `client_version` → `clientVersion`, etc.
- **Token response format**:
  ```json
  {
    "token": "string (JWT)",
    "expiresIn": "integer (seconds)",
    "tokenType": "Bearer"
  }
  ```

#### Error Reporting Changes
- **Split into two endpoints**:
  - `/api/v1/error` - For standalone errors (not associated with batch)
  - `/api/v1/error/{batchId}` - For batch-specific errors
- **Simplified error schema**: Removed `filename` and `timestamp` fields (server-assigned)
- **New error types**: Added `BatchError` for batch-specific failures

### ✨ Added

#### Batch Protocol Features
- **Batch lifecycle management** with state tracking (InProgress, Completed, Failed, Cancelled)
- **Configurable batch settings**:
  - `max_files_per_batch` - Maximum files per batch (default: 100)
  - `chunk_size_mb` - File chunk size for uploads (default: 10 MB)
  - `max_retries` - Maximum retry attempts (default: 3)
  - `retry_delay_seconds` - Delay between retries (default: 30)
- **Automatic batch retry logic** with exponential backoff
- **Batch cancellation support** for graceful error handling
- **Multipart file uploads** with chunking support

#### Authentication & Security
- **Automatic JWT token renewal** - Tokens are automatically refreshed when expired
- **Token caching** - JWT tokens cached to reduce authentication overhead
- **Enhanced JWT validation** - Validates `exp`, `siteId`, `accountId`, and `domain` claims
- **Localhost HTTP exception** - Allow HTTP for localhost/127.0.0.1 testing, enforce HTTPS for production
- **Site credentials validation** - UUID format validation for `client_secret`

#### Installation & Migration
- **Interactive installation wizard** (`data_exporter.exe install`):
  - Guided setup for fresh installations
  - Input validation for all configuration fields
  - Real-time credential verification
  - Automatic service registration
- **Migration wizard** (`data_exporter.exe migrate`):
  - Automatic detection of v1.0 configuration
  - Interactive migration with new credential prompts
  - Automatic configuration backup (config.toml.v1.backup)
  - Service restart with new configuration
- **Detection of existing installations** - Prevents accidental overwrites

#### Configuration Features
- **Hot reload support** - Configuration changes apply between scheduled runs (no service restart required)
- **Enhanced validation** - All configuration fields validated on load with detailed error messages
- **Multiple example configurations** - Production, performance-optimized, development, and low-resource examples
- **Environment-specific settings** - Support for development vs. production configurations

#### Logging & Monitoring
- **Configurable log levels** - `trace`, `debug`, `info`, `warn`, `error` (default: `info`)
- **Custom error log path** - Configurable fallback error log location (default: `error.log`)
- **Structured logging** - JSON-formatted logs with context and metadata
- **Enhanced error context** - Errors include batch ID, file name, and operation context

#### Documentation
- **Comprehensive README.md** - Updated for v2.0 with breaking changes warning, migration guide link, and new architecture
- **Migration Guide** (`MIGRATION_GUIDE.md`):
  - Automatic vs. manual migration instructions
  - Configuration field mapping table
  - Rollback instructions
  - Troubleshooting section
- **Configuration Reference** (`CONFIGURATION.md`):
  - Complete field-by-field documentation
  - Validation rules and default values
  - Multiple example configurations
  - Hot reload behavior
  - Troubleshooting guide
- **Updated API contracts**:
  - `auth-api.md` - v2.0 authentication with site credentials
  - `batch-api.md` - Complete batch protocol documentation
  - `error-report-api.md` - Batch vs. standalone error reporting
  - `upload-api-v1-deprecated.md` - Deprecated v1.0 single-file upload

### 🔄 Changed

#### Service Behavior
- **Service lifecycle** - Now manages batch operations instead of individual file uploads
- **Scheduled execution** - Each cron trigger starts a new batch operation
- **Error handling** - Improved error recovery with batch-level retry logic
- **File processing** - Files are grouped into batches before upload (batch size configurable)

#### CLI Commands
- **Install command** - No longer accepts CLI arguments, uses interactive wizard instead
  - Before: `data_exporter.exe install <username> <password> <source_dir> <crontab> <api_url> <encoding>`
  - After: `data_exporter.exe install` (interactive wizard)
- **New Migrate command** - `data_exporter.exe migrate` for v1.0 → v2.0 upgrades
- **Uninstall command** - Unchanged

#### API Client Behavior
- **Authentication flow** - Obtains JWT token before batch operations, automatically renews when expired
- **Upload workflow**:
  - Before: Single file → `/api/upload`
  - After: Start batch → Upload files → Complete batch
- **Error reporting** - Batch errors reported to batch-specific endpoint with batch ID

#### Performance Improvements
- **Reduced authentication overhead** - JWT token caching reduces API calls
- **Batch uploads** - Grouping files reduces HTTP overhead and improves throughput
- **Chunked uploads** - Large files uploaded in configurable chunks (default: 10 MB)
- **Connection pooling** - Reuses HTTP connections for batch uploads

### 🗑️ Deprecated

- **Single-file upload endpoint** (`POST /api/upload`) - Use batch protocol instead (see `batch-api.md`)
- **Username/password authentication** - Use site credentials (domain + client_secret) instead

### 🔒 Security

- **Enhanced authentication** - Site credentials provide better security than username/password
- **JWT token expiration** - Tokens expire and are automatically renewed
- **HTTPS enforcement** - Production deployments must use HTTPS (localhost HTTP exception for testing)
- **Configuration file permissions** - Restricted to administrators only (Windows ACL)
- **Sensitive data protection** - Credentials not included in error messages or logs

### 🐛 Fixed

- **Encoding detection** - Improved DBF encoding detection with fallback to CP866
- **Error recovery** - Better handling of network failures and server errors
- **Configuration validation** - Comprehensive validation prevents invalid configurations
- **Service installation** - Improved Windows service registration reliability

### 📊 Test Coverage

- **208+ tests passing**:
  - 155 unit tests
  - 39 contract tests (API validation)
  - 14 integration tests
- **Test categories**:
  - Configuration parsing and validation
  - Authentication and JWT token handling
  - Batch protocol lifecycle
  - DBF to CSV conversion
  - Compression (gzip)
  - Error reporting
  - Service installation and migration

### 📝 Migration Guide

**For v1.0 users**: See [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) for detailed migration instructions.

**Quick migration**:
```powershell
# Automatic migration with wizard
.\data_exporter.exe migrate
```

**Manual migration**:
1. Backup your existing configuration: `copy config.toml config.toml.v1.backup`
2. Obtain site credentials from middleware panel (domain + client_secret)
3. Update configuration file to v2.0 format (see [CONFIGURATION.md](CONFIGURATION.md))
4. Restart the service: `sc stop data-exporter && sc start data-exporter`

### 🔗 Resources

- [README.md](README.md) - Overview and quick start guide
- [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) - Detailed v1.0 → v2.0 migration instructions
- [CONFIGURATION.md](CONFIGURATION.md) - Complete configuration reference
- [specs/001-technical-specifications-data/contracts/](specs/001-technical-specifications-data/contracts/) - API contract documentation
  - `auth-api.md` - Authentication API v2.0
  - `batch-api.md` - Batch protocol API
  - `error-report-api.md` - Error reporting API

---

## [1.0.0] - 2025-10-01

### Added
- Initial release with single-file upload protocol
- Username/password authentication
- DBF to CSV conversion
- Gzip compression
- Windows service installation
- Cron-based scheduling
- Basic error reporting

### Security
- Basic authentication with username/password
- HTTPS support for API communication

---

[2.0.0]: https://github.com/yourusername/dbf-uploader/compare/v1.0.0...v2.0.0
[1.0.0]: https://github.com/yourusername/dbf-uploader/releases/tag/v1.0.0
