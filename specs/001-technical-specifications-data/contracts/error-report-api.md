# API Contract: Error Reporting v2.0

**Endpoints**:
- `POST /api/v1/error` - Report standalone errors (not associated with batch)
- `POST /api/v1/error/{batchId}` - Report batch-specific errors

**Purpose**: Send error reports to middleware for monitoring and diagnostics
**Used By**: ErrorReporter (when processing errors occur)
**Version**: 2.0

## Request

### Method
`POST`

### Headers
```
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

**Header Descriptions**:
- `Authorization`: JWT token obtained from `/api/auth/token`
  - Format: `Bearer <token>`
  - Optional: If JWT not available (auth failure), attempt without (server may accept or reject)
- `Content-Type`: JSON payload
  - Value: `application/json`

### Request Body Schema (camelCase)

```json
{
  "type": "string",
  "message": "string",
  "clientVersion": "string (optional)"
}
```

#### Field Descriptions

- `type`: Classification of error
  - Values: `"FileReadError"`, `"EncodingError"`, `"ConversionError"`, `"CompressionError"`, `"UploadError"`, `"BatchError"`, `"DiskFullError"`, `"DirectoryInaccessible"`, `"AuthenticationError"`, `"ConfigurationError"`, `"NetworkError"`
  - Required: Yes
  - Purpose: Allows server to categorize and aggregate errors

- `message`: Human-readable error description
  - Format: Detailed error message with context
  - Example: `"Failed to read DBF file: Permission denied (OS Error 5)"`
  - Required: Yes
  - Max Length: 2000 characters (recommended)
  - Should Include: Error details, operation context, error chain

- `clientVersion`: Version of data exporter service
  - Format: Semantic versioning (MAJOR.MINOR.PATCH)
  - Example: `"2.0.0"`
  - Required: Optional (auto-populated by client)
  - Purpose: Server-side diagnostics, compatibility tracking

### Example Requests

#### Standalone Error (No Batch Context)
```http
POST /api/v1/error HTTP/1.1
Host: middleware.example.com
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "type": "ConfigurationError",
  "message": "Invalid configuration: Missing required field 'auth.domain'",
  "clientVersion": "2.0.0"
}
```

#### Batch Error (With Batch Context)
```http
POST /api/v1/error/abcdef12-3456-7890-abcd-ef1234567890 HTTP/1.1
Host: middleware.example.com
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "type": "BatchError",
  "message": "Failed to complete batch: Upload timeout exceeded",
  "clientVersion": "2.0.0"
}
```

### Additional Example Requests

**File Processing Error (Standalone)**:
```json
{
  "type": "EncodingError",
  "message": "Failed to detect encoding from DBF header, fallback encoding CP866 also failed",
  "clientVersion": "2.0.0"
}
```

**Disk Space Error (Standalone)**:
```json
{
  "type": "DiskFullError",
  "message": "Failed to write CSV during conversion: No space left on device (OS Error 28)",
  "clientVersion": "2.0.0"
}
```

**Directory Error (Standalone)**:
```json
{
  "type": "DirectoryInaccessible",
  "message": "Cannot access source directory: Network path not found (OS Error 53)",
  "clientVersion": "2.0.0"
}
```

**Authentication Error (Standalone)**:
```json
{
  "type": "AuthenticationError",
  "message": "Failed to obtain JWT token: Invalid credentials",
  "clientVersion": "2.0.0"
}
```

**Batch Upload Error (With Batch ID)**:
```json
{
  "type": "UploadError",
  "message": "Failed to upload file data.csv.gz: Server returned 503 Service Unavailable",
  "clientVersion": "2.0.0"
}
```

## Response

### Success Response (200 OK or 204 No Content)

#### Option 1: 200 OK with Body
```json
{
  "status": "received",
  "error_id": "string (optional)"
}
```

**Field Descriptions**:
- `status`: Acknowledgment indicator
  - Value: `"received"`, `"logged"`, or `"acknowledged"`
- `error_id`: Server-assigned error identifier (optional)
  - Used for server-side tracking and correlation

#### Option 2: 204 No Content
No response body. Status code indicates successful receipt.

### Error Responses

#### 401 Unauthorized - Invalid or Expired Token
```json
{
  "error": "unauthorized",
  "message": "Invalid or expired token"
}
```

**When**: JWT token missing, invalid, or expired
**Client Action**: Write to local error log (fallback)

#### 400 Bad Request - Invalid Request Body
```json
{
  "error": "bad_request",
  "message": "Missing required field: type"
}
```

**OR**

```json
{
  "error": "bad_request",
  "message": "Invalid error type value"
}
```

**When**: Required field missing or invalid format
**Client Action**: Log validation error locally, fix bug in error reporter

#### 500 Internal Server Error
```json
{
  "error": "internal_error",
  "message": "Failed to log error report"
}
```

**When**: Server-side error processing error report
**Client Action**: Write to local error log (fallback)

#### 503 Service Unavailable
```json
{
  "error": "service_unavailable",
  "message": "Error logging service temporarily unavailable"
}
```

**When**: Server error logging unavailable
**Client Action**: Write to local error log (fallback)

## Client Behavior Requirements

### When to Send Error Reports

**Standalone Errors** (`POST /api/v1/error`):
1. Configuration errors (invalid config.toml)
2. Authentication failures
3. Source directory inaccessible
4. Disk space errors
5. Service startup errors

**Batch Errors** (`POST /api/v1/error/{batchId}`):
1. DBF file is corrupted or unreadable
2. Encoding detection and conversion fails
3. CSV conversion fails
4. Compression fails
5. File upload fails
6. Batch operation failures

### Do NOT Send Error Reports For

1. Network errors when trying to send error reports (avoid infinite loop)
2. Config file syntax errors (log locally only)
3. Expected conditions (locked files that will retry)

### Pre-Send Checks
1. Verify error report has all required fields (type, message)
2. Truncate message if > 2000 characters
3. Auto-populate clientVersion field
4. If JWT token available and valid, include in Authorization header
5. If JWT token unavailable, attempt without (server may accept unauthenticated error reports)
6. Choose correct endpoint: /api/v1/error vs /api/v1/error/{batchId}

### On Success (200/204)
1. Log successful error report transmission
2. Continue processing

### On 401 Unauthorized
1. Write error to local error log (fallback)
2. Do NOT attempt to renew token and retry (avoid error reporting loops)

### On 4xx Client Errors (400)
1. Write error to local error log with validation failure details
2. Log bug in error reporter for investigation
3. Continue processing

### On 5xx Server Errors (500, 503)
1. Write error to local error log (fallback)
2. Do NOT retry (avoid overwhelming server)

### On Network Error
1. Write error to local error log (primary use case for fallback logging)
2. Continue processing

## Local Error Log Fallback

When error report cannot be sent to server:

**File Path**: `C:\Program Files\data-exporter\error.log`

**Format** (plain text):
```
[2025-10-05T14:30:00Z] FileReadError - Failed to read DBF file: Permission denied (OS Error 5)
[2025-10-05T14:35:12Z] BatchError - Failed to complete batch abc-123: Upload timeout
[2025-10-05T14:40:00Z] ConfigurationError - Invalid config: Missing auth.domain
```

**Behavior**:
- Append to file (don't overwrite)
- Create file if it doesn't exist
- Format: `[timestamp] type - message`
- Include timestamp (ISO 8601), error type, message
- No automatic rotation or cleanup (manual admin task)

## Security Requirements

- **HTTPS Only**: All error reports must be sent over HTTPS
- **Authentication**: JWT token recommended but may be optional for error reports (server decides)
- **Sensitive Data**: Do not include passwords or credentials in error messages
- **File Paths**: Use relative paths from source directory (don't leak full system paths unnecessarily)

## Sequence Diagram

```
Client                          Server
  │                               │
  │  Error occurs during          │
  │  batch processing             │
  │                               │
  │  Build error report JSON      │
  │  (type, message, clientVersion)
  │                               │
  │  POST /api/v1/error/{batchId} │
  │  Authorization: Bearer ...    │
  │  { type, message }            │
  ├──────────────────────────────►│
  │                               │
  │                               │ Validate token (optional)
  │                               │ Validate JSON schema
  │                               │ Log error
  │                               │ Store for monitoring
  │                               │
  │  200 OK                       │
  │  { status: "received" }       │
  │◄──────────────────────────────┤
  │                               │
  │  Continue processing          │
  │                               │

Alternative: Server Unavailable
  │                               X
  │  POST /api/v1/error/{batchId} │
  │  [Network timeout]            │
  ├─────────────────────────────► X
  │                               │
  │  Write to local error.log     │
  │  [timestamp] type - message   │
  │  Continue processing          │
  │                               │
```

## Contract Tests

The following test scenarios must be implemented:

1. **Successful Standalone Error Report**
   - Given: Valid JWT token and valid error report JSON
   - When: POST to /api/v1/error
   - Then: Receive 200/204 success response

2. **Successful Batch Error Report**
   - Given: Valid JWT token, batch ID, and valid error report JSON
   - When: POST to /api/v1/error/{batchId}
   - Then: Receive 200/204 success response

3. **Missing Authorization (Optional)**
   - Given: No Authorization header
   - When: POST to /api/v1/error
   - Then: Receive 401 Unauthorized OR 200 (if server allows unauthenticated error reports)

4. **Missing Required Field**
   - Given: Error report missing "type" field
   - When: POST to /api/v1/error
   - Then: Receive 400 Bad Request

5. **Invalid Error Type**
   - Given: Unknown error type value
   - When: POST to /api/v1/error
   - Then: Receive 400 Bad Request

6. **All Error Types**
   - Given: Error reports for each type value
   - When: POST to /api/v1/error
   - Then: All accepted (200/204)

7. **Local Fallback on Network Error**
   - Given: Server unreachable
   - When: Attempt to send error report
   - Then: Error written to local error.log with correct format

8. **HTTPS Enforcement**
   - Given: Production HTTPS URL
   - When: Attempt to send error report
   - Then: Client uses HTTPS-only

9. **No Infinite Loop**
   - Given: Error reporting endpoint is down
   - When: Error occurs during processing
   - Then: Write to local log, do NOT retry error report

10. **camelCase Field Serialization**
    - Given: Error report with clientVersion field
    - When: Serialize to JSON
    - Then: JSON contains "clientVersion" (camelCase), not "client_version"

## Error Type Catalog

| Error Type              | When to Use | Endpoint |
|------------------------|-------------|----------|
| `FileReadError`         | Cannot read DBF file (permissions, not found, corrupted) | Batch |
| `EncodingError`         | Failed to detect or convert encoding | Batch |
| `ConversionError`       | DBF to CSV conversion failed | Batch |
| `CompressionError`      | Gzip compression failed | Batch |
| `UploadError`           | File upload failed (server rejected, validation) | Batch |
| `BatchError`            | Batch operation failed (start, complete, timeout) | Batch |
| `DiskFullError`         | No space left on device | Standalone |
| `DirectoryInaccessible` | Cannot access source directory | Standalone |
| `AuthenticationError`   | Failed to obtain JWT token | Standalone |
| `ConfigurationError`    | Invalid configuration detected | Standalone |
| `NetworkError`          | Network communication failed | Both |

## Notes

- Error reports are fire-and-forget (no retry on failure)
- Duplicate error reports for same error acceptable (server deduplicates)
- Error reporting should never block main processing flow
- Client version auto-populated from binary version
- Local error log is append-only, requires manual administrator cleanup
- Batch errors use `/api/v1/error/{batchId}` endpoint
- Standalone errors use `/api/v1/error` endpoint
- All request/response fields use camelCase naming convention
- Server may accept unauthenticated error reports for reliability
