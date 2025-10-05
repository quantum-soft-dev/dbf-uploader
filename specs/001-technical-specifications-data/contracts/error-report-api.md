# API Contract: Error Reporting

**Endpoint**: `POST /api/errors/report`
**Purpose**: Send error reports to server for monitoring and diagnostics
**Used By**: Error Reporter (when processing errors occur)

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

### Request Body Schema

```json
{
  "filename": "string",
  "error_type": "string",
  "message": "string",
  "timestamp": "string (ISO 8601)",
  "client_version": "string"
}
```

#### Field Descriptions

- `filename`: Name/path of file that caused error
  - Format: Relative path from source directory
  - Example: `"subdir\\data.dbf"` (Windows path)
  - Required: Yes
  - Validation: Non-empty string

- `error_type`: Classification of error
  - Values: `"FileReadError"`, `"EncodingError"`, `"ConversionError"`, `"CompressionError"`, `"UploadError"`, `"DiskFullError"`, `"DirectoryInaccessible"`, `"AuthenticationError"`, `"ConfigurationError"`, `"NetworkError"`
  - Required: Yes
  - Purpose: Allows server to categorize and aggregate errors

- `message`: Human-readable error description
  - Format: Detailed error message with context
  - Example: `"Failed to read DBF file: Permission denied (OS Error 5)"`
  - Required: Yes
  - Max Length: 2000 characters (recommended)
  - Should Include: Error details, operation context, error chain

- `timestamp`: When error occurred
  - Format: ISO 8601 (UTC): `YYYY-MM-DDTHH:MM:SSZ`
  - Example: `"2025-10-05T14:30:00Z"`
  - Required: Yes
  - Validation: Valid ISO 8601 timestamp

- `client_version`: Version of data exporter service
  - Format: Semantic versioning (MAJOR.MINOR.PATCH)
  - Example: `"1.0.0"`
  - Required: Yes
  - Purpose: Server-side diagnostics, compatibility tracking

### Example Request

```http
POST /api/errors/report HTTP/1.1
Host: api.example.com
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "filename": "archive\\2024\\sales.dbf",
  "error_type": "FileReadError",
  "message": "Failed to read DBF file: Permission denied (OS Error 5)",
  "timestamp": "2025-10-05T14:30:00Z",
  "client_version": "1.0.0"
}
```

### Additional Example Requests

**Encoding Error**:
```json
{
  "filename": "reports\\monthly.dbf",
  "error_type": "EncodingError",
  "message": "Failed to detect encoding from DBF header, fallback encoding CP866 also failed",
  "timestamp": "2025-10-05T14:35:12Z",
  "client_version": "1.0.0"
}
```

**Disk Full Error**:
```json
{
  "filename": "data\\large_file.dbf",
  "error_type": "DiskFullError",
  "message": "Failed to write CSV during conversion: No space left on device (OS Error 28)",
  "timestamp": "2025-10-05T14:40:00Z",
  "client_version": "1.0.0"
}
```

**Directory Inaccessible**:
```json
{
  "filename": "N/A",
  "error_type": "DirectoryInaccessible",
  "message": "Cannot access source directory: Network path not found (OS Error 53)",
  "timestamp": "2025-10-05T14:45:00Z",
  "client_version": "1.0.0"
}
```

**Authentication Error**:
```json
{
  "filename": "N/A",
  "error_type": "AuthenticationError",
  "message": "Failed to obtain JWT token: Invalid credentials",
  "timestamp": "2025-10-05T14:50:00Z",
  "client_version": "1.0.0"
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
  "message": "Missing required field: filename"
}
```

**OR**

```json
{
  "error": "bad_request",
  "message": "Invalid timestamp format"
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

Send error reports for the following scenarios:
1. DBF file is corrupted or unreadable
2. Encoding detection and conversion fails
3. CSV conversion fails
4. Compression fails
5. File upload fails (non-network errors)
6. Disk full during processing
7. Source directory inaccessible
8. Authentication fails (when reporting is possible)

### Do NOT Send Error Reports For

1. Network errors when trying to send error reports (avoid infinite loop)
2. Config file syntax errors (log locally only)
3. Expected conditions (locked files that will retry)

### Pre-Send Checks
1. Verify error report has all required fields
2. Verify timestamp is in ISO 8601 format
3. Truncate message if > 2000 characters
4. If JWT token available and valid, include in Authorization header
5. If JWT token unavailable, attempt without (server may accept for error reports)

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
[2025-10-05T14:30:00Z] ERROR: Failed to send error report to server: Connection refused
  Filename: archive\2024\sales.dbf
  Error Type: FileReadError
  Message: Failed to read DBF file: Permission denied (OS Error 5)
  Context: Batch processing at 2025-10-05 14:30:00
```

**Behavior**:
- Append to file (don't overwrite)
- Create file if it doesn't exist
- Include timestamp, error type, filename, message
- Include reason why server report failed
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
  │  file processing              │
  │                               │
  │  Build error report JSON      │
  │                               │
  │  POST /api/errors/report      │
  │  Authorization: Bearer ...    │
  │  { error details }            │
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
  │  POST /api/errors/report      │
  │  [Network timeout]            │
  ├─────────────────────────────► X
  │                               │
  │  Write to local error.log     │
  │  Continue processing          │
  │                               │
```

## Contract Tests

The following test scenarios must be implemented:

1. **Successful Error Report**
   - Given: Valid JWT token and valid error report JSON
   - When: POST to /api/errors/report
   - Then: Receive 200/204 success response

2. **Missing Authorization**
   - Given: No Authorization header
   - When: POST to /api/errors/report
   - Then: Receive 401 Unauthorized OR 200 (if server allows unauthenticated error reports)

3. **Missing Required Field**
   - Given: Error report missing "filename" field
   - When: POST to /api/errors/report
   - Then: Receive 400 Bad Request

4. **Invalid Timestamp Format**
   - Given: Timestamp not in ISO 8601 format
   - When: POST to /api/errors/report
   - Then: Receive 400 Bad Request

5. **All Error Types**
   - Given: Error reports for each error_type value
   - When: POST to /api/errors/report
   - Then: All accepted (200/204)

6. **Local Fallback on Network Error**
   - Given: Server unreachable
   - When: Attempt to send error report
   - Then: Error written to local error.log

7. **HTTPS Enforcement**
   - Given: HTTP URL (not HTTPS)
   - When: Attempt to send error report
   - Then: Client rejects request

8. **No Infinite Loop**
   - Given: Error reporting endpoint is down
   - When: Error occurs during processing
   - Then: Write to local log, do NOT retry error report

## Error Type Catalog

| Error Type              | When to Use |
|------------------------|-------------|
| `FileReadError`         | Cannot read DBF file (permissions, not found, corrupted) |
| `EncodingError`         | Failed to detect or convert encoding |
| `ConversionError`       | DBF to CSV conversion failed |
| `CompressionError`      | Gzip compression failed |
| `UploadError`           | File upload failed (server rejected, validation) |
| `DiskFullError`         | No space left on device |
| `DirectoryInaccessible` | Cannot access source directory |
| `AuthenticationError`   | Failed to obtain JWT token |
| `ConfigurationError`    | Invalid configuration detected |
| `NetworkError`          | Network communication failed (for operations other than error reporting) |

## Notes

- Error reports are fire-and-forget (no retry on failure)
- Duplicate error reports for same file/error acceptable (server deduplicates)
- Error reporting should never block main processing flow
- Timestamp should be error occurrence time, not report send time
- Client version helps server track which client versions have issues
- Local error log is append-only, requires manual administrator cleanup
