# API Contract: Batch Protocol v2.0

**Endpoints** (middleware v3.0.0):
- `POST /api/dfc/batch/start` - Start new batch
- `POST /api/dfc/batch/{batchId}/upload` - Upload files to batch
- `POST /api/dfc/batch/{batchId}/complete` - Complete batch
- `POST /api/dfc/batch/{batchId}/fail` - Mark batch as failed
- `POST /api/dfc/batch/{batchId}/cancel` - Cancel batch

**Purpose**: Batch upload protocol for grouped file operations with lifecycle management
**Used By**: UploaderService, BatchManager
**Version**: 2.0
**Server Version**: data-forge-middleware v3.0.0
**Breaking Change**: URL changed from `/api/v1/batch` to `/api/dfc/batch`

## Overview

The batch protocol provides grouped file upload with state tracking:

**Lifecycle States**:
1. **InProgress** - Batch created, accepting file uploads
2. **Completed** - All files uploaded successfully
3. **Failed** - Batch processing failed
4. **Cancelled** - Batch cancelled by client

**Workflow**:
```
Start Batch → Upload Files → Complete Batch
     ↓              ↓              ↓
 (InProgress)  (InProgress)  (Completed)

                    ↓ (on error)
                   Fail Batch → (Failed)

                   Cancel → (Cancelled)
```

---

## 1. Start Batch

**Endpoint**: `POST /api/dfc/batch/start`
**Purpose**: Create new batch and obtain batch ID for subsequent uploads

### Request

#### Method
`POST`

#### Headers
```
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

#### Request Body
None (empty JSON object or no body)

#### Example Request
```http
POST /api/dfc/batch/start HTTP/1.1
Host: middleware.example.com
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json
```

### Response

#### Success Response (200 OK)

##### Body Schema (camelCase)
```json
{
  "batchId": "string (UUID format)"
}
```

##### Field Descriptions
- `batchId`: Unique identifier for the created batch
  - Format: UUID (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
  - Used in all subsequent batch operations
  - Example: `"abcdef12-3456-7890-abcd-ef1234567890"`

##### Example Success Response
```json
{
  "batchId": "abcdef12-3456-7890-abcd-ef1234567890"
}
```

#### Error Responses

##### 401 Unauthorized
```json
{
  "error": "Invalid or expired token"
}
```

**When**: JWT token missing, invalid, or expired
**Client Action**: Renew token, retry start batch

##### 500 Internal Server Error
```json
{
  "error": "Failed to create batch"
}
```

**When**: Server-side error creating batch
**Client Action**: Retry with exponential backoff (max 3 attempts)

---

## 2. Upload Files

**Endpoint**: `POST /api/dfc/batch/{batchId}/upload`
**Purpose**: Upload one or more files to existing batch

### Request

#### Method
`POST`

#### Path Parameters
- `batchId`: UUID of batch created via `/api/dfc/batch/start`

#### Headers
```
Authorization: Bearer <jwt_token>
Content-Type: multipart/form-data; boundary=<boundary>
```

#### Request Body (Multipart Form Data)

**Form Fields**:
- `files`: One or more binary file attachments (gzip-compressed CSV)
  - Field name: `files` (multiple files allowed)
  - Filename: Generated from DBF relative path (e.g., `subdir_data.csv.gz`)
  - Content-Type: `application/gzip` or `application/octet-stream`

**Multipart Structure**:
```
--boundary
Content-Disposition: form-data; name="files"; filename="data.csv.gz"
Content-Type: application/gzip

<binary gzip data>
--boundary
Content-Disposition: form-data; name="files"; filename="reports_monthly.csv.gz"
Content-Type: application/gzip

<binary gzip data>
--boundary--
```

#### Example Request
```http
POST /api/dfc/batch/abcdef12-3456-7890-abcd-ef1234567890/upload HTTP/1.1
Host: middleware.example.com
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW

------WebKitFormBoundary7MA4YWxkTrZu0gW
Content-Disposition: form-data; name="files"; filename="data.csv.gz"
Content-Type: application/gzip

<binary gzip data>
------WebKitFormBoundary7MA4YWxkTrZu0gW--
```

### Response

#### Success Response (200 OK)

##### Body Schema (camelCase)
**Matches server**: `FileUploadController.java` in data-forge-middleware v3.0.0

```json
{
  "status": "string",
  "uploadedFiles": "integer",
  "files": [
    {
      "fileName": "string",
      "fileSize": "integer",
      "uploadedAt": "string (ISO 8601)"
    }
  ]
}
```

##### Field Descriptions
- `status`: Upload operation status ("OK", "ERROR")
- `uploadedFiles`: Count of successfully uploaded files (integer, not array!)
- `files`: Array of uploaded file metadata
  - `fileName`: Original filename from multipart (camelCase!)
  - `fileSize`: File size in bytes (camelCase!)
  - `uploadedAt`: Upload timestamp in ISO 8601 format (camelCase!)

##### Example Success Response
```json
{
  "status": "OK",
  "uploadedFiles": 2,
  "files": [
    {
      "fileName": "data.csv.gz",
      "fileSize": 1024,
      "uploadedAt": "2025-10-06T10:35:00Z"
    },
    {
      "fileName": "reports_monthly.csv.gz",
      "fileSize": 2048,
      "uploadedAt": "2025-10-06T10:35:01Z"
    }
  ]
}
```

#### Error Responses

##### 401 Unauthorized
```json
{
  "error": "Invalid or expired token"
}
```

**When**: JWT token missing, invalid, or expired
**Client Action**: Renew token, retry upload

##### 404 Not Found
```json
{
  "error": "Batch not found"
}
```

**When**: Batch ID doesn't exist or already completed/failed
**Client Action**: Report error, start new batch

##### 400 Bad Request
```json
{
  "error": "No files provided"
}
```

**When**: Multipart request has no file fields
**Client Action**: Log error, skip empty upload

##### 413 Payload Too Large
```json
{
  "error": "File size exceeds maximum allowed"
}
```

**When**: File exceeds server size limit
**Client Action**: Log error, report to error endpoint, skip file

##### 500 Internal Server Error
```json
{
  "error": "Failed to process upload"
}
```

**When**: Server-side processing error
**Client Action**: Retry with exponential backoff (max 3 attempts)

---

## 3. Complete Batch

**Endpoint**: `POST /api/dfc/batch/{batchId}/complete`
**Purpose**: Finalize batch after all files uploaded successfully

### Request

#### Method
`POST`

#### Path Parameters
- `batchId`: UUID of batch to complete

#### Headers
```
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

#### Request Body
None (empty JSON object or no body)

#### Example Request
```http
POST /api/dfc/batch/abcdef12-3456-7890-abcd-ef1234567890/complete HTTP/1.1
Host: middleware.example.com
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json
```

### Response

#### Success Response (200 OK)

##### Body Schema (camelCase)
```json
{
  "batchId": "string (UUID)",
  "status": "string",
  "uploadedFilesCount": "integer",
  "totalSize": "integer",
  "hasErrors": "boolean"
}
```

##### Field Descriptions
- `batchId`: UUID of completed batch
- `status`: Batch final status ("completed")
- `uploadedFilesCount`: Total number of files uploaded
- `totalSize`: Total bytes uploaded in entire batch
- `hasErrors`: Whether any files had errors (false for successful completion)

##### Example Success Response
```json
{
  "batchId": "abcdef12-3456-7890-abcd-ef1234567890",
  "status": "completed",
  "uploadedFilesCount": 15,
  "totalSize": 1048576,
  "hasErrors": false
}
```

#### Error Responses

##### 401 Unauthorized
```json
{
  "error": "Invalid or expired token"
}
```

**When**: JWT token missing, invalid, or expired
**Client Action**: Renew token, retry complete

##### 404 Not Found
```json
{
  "error": "Batch not found"
}
```

**When**: Batch ID doesn't exist
**Client Action**: Log error, report issue

##### 409 Conflict
```json
{
  "error": "Batch already completed or failed"
}
```

**When**: Batch already in terminal state
**Client Action**: Log warning, continue (idempotent)

##### 500 Internal Server Error
```json
{
  "error": "Failed to complete batch"
}
```

**When**: Server-side error finalizing batch
**Client Action**: Retry with exponential backoff (max 3 attempts)

---

## 4. Fail Batch

**Endpoint**: `POST /api/dfc/batch/{batchId}/fail`
**Purpose**: Mark batch as failed when unrecoverable error occurs

### Request

#### Method
`POST`

#### Path Parameters
- `batchId`: UUID of batch to mark as failed

#### Headers
```
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

#### Request Body (Optional)
```json
{
  "reason": "string (error description)"
}
```

#### Example Request
```http
POST /api/dfc/batch/abcdef12-3456-7890-abcd-ef1234567890/fail HTTP/1.1
Host: middleware.example.com
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "reason": "DBF file corrupted, cannot convert to CSV"
}
```

### Response

#### Success Response (200 OK)

##### Body Schema (camelCase)
```json
{
  "batchId": "string (UUID)",
  "status": "string"
}
```

##### Example Success Response
```json
{
  "batchId": "abcdef12-3456-7890-abcd-ef1234567890",
  "status": "failed"
}
```

#### Error Responses

Similar to Complete Batch endpoint

---

## 5. Cancel Batch

**Endpoint**: `POST /api/dfc/batch/{batchId}/cancel`
**Purpose**: Cancel batch operation (user-initiated or timeout)

### Request

#### Method
`POST`

#### Path Parameters
- `batchId`: UUID of batch to cancel

#### Headers
```
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

#### Request Body (Optional)
```json
{
  "reason": "string (cancellation reason)"
}
```

#### Example Request
```http
POST /api/dfc/batch/abcdef12-3456-7890-abcd-ef1234567890/cancel HTTP/1.1
Host: middleware.example.com
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "reason": "Batch timeout exceeded"
}
```

### Response

#### Success Response (200 OK)

##### Body Schema (camelCase)
```json
{
  "batchId": "string (UUID)",
  "status": "string"
}
```

##### Example Success Response
```json
{
  "batchId": "abcdef12-3456-7890-abcd-ef1234567890",
  "status": "cancelled"
}
```

#### Error Responses

Similar to Complete Batch endpoint

---

## Client Behavior Requirements

### Batch Lifecycle Management

1. **Start Batch**:
   - Create batch at beginning of scheduled run
   - Store `batchId` for all subsequent operations
   - Set batch timeout (e.g., 1 hour)

2. **Upload Files**:
   - Upload files in batches (configurable, default 100 files per request)
   - Support chunked uploads for large files
   - Track uploaded file count
   - Handle upload failures per file

3. **Complete Batch**:
   - Call after all files successfully uploaded
   - Verify server acknowledges completion
   - Clear batch state

4. **Fail Batch**:
   - Call on unrecoverable errors
   - Include error description
   - Report errors to error endpoint

5. **Cancel Batch**:
   - Call on timeout or user cancellation
   - Include cancellation reason

### Error Handling

- **401 Unauthorized**: Renew token, retry operation
- **404 Not Found**: Log error, start new batch
- **409 Conflict**: Log warning, treat as success (idempotent)
- **5xx Server Errors**: Retry with exponential backoff (max 3 attempts)
- **Network Errors**: Fail batch, write to local error log

### Retry Logic

- **Transient errors** (503, network timeout): Retry with backoff
- **Permanent errors** (400, 404, 409): Do not retry
- **Max retries**: 3 attempts
- **Backoff**: Exponential (30s, 60s, 120s)

### File Naming Convention

**Source**: `C:\Program Files\abc\def\subdir1\subdir2\data.dbf`
**Relative Path**: `subdir1\subdir2\data.dbf`
**Compressed Filename**: `subdir1_subdir2_data.csv.gz`

**Transformation Rules**:
1. Extract relative path from source_dir
2. Replace path separators (`\`) with underscores (`_`)
3. Replace `.dbf` extension with `.csv.gz`

## Security Requirements

- **HTTPS Only**: All requests must use HTTPS in production
  - HTTP allowed only for localhost testing
- **Authentication**: JWT Bearer token required in Authorization header
- **Token Validation**: Server validates token signature and expiration
- **Batch Ownership**: Server verifies batch belongs to authenticated site

## Sequence Diagram

```
Client                          Server
  │                               │
  │  POST /api/dfc/batch/start    │
  │  Authorization: Bearer ...    │
  ├──────────────────────────────►│
  │                               │ Create batch
  │                               │ Generate batch ID
  │  200 OK                       │
  │  { batchId: "..." }           │
  │◄──────────────────────────────┤
  │                               │
  │  Store batch ID               │
  │                               │
  │  POST /api/dfc/batch/{id}/upload
  │  multipart/form-data          │
  │  [files...]                   │
  ├──────────────────────────────►│
  │                               │ Store files
  │                               │ Track progress
  │  200 OK                       │
  │  { status: "OK",              │
  │    uploadedFiles: 2,          │
  │    files: [...] }             │
  │◄──────────────────────────────┤
  │                               │
  │  (repeat for all files)       │
  │                               │
  │  POST /api/dfc/batch/{id}/complete
  │  Authorization: Bearer ...    │
  ├──────────────────────────────►│
  │                               │ Finalize batch
  │                               │ Update status
  │  200 OK                       │
  │  { status: "completed" }      │
  │◄──────────────────────────────┤
  │                               │
  │  Clear batch state            │
  │  Delete local CSV files       │
  │                               │
```

## Contract Tests

The following test scenarios must be implemented:

1. **Successful Batch Workflow**
   - Start batch → Upload files → Complete batch
   - Verify batch ID returned and used correctly
   - Verify final status is "completed"

2. **Batch Start Failure**
   - Given: Invalid token
   - When: POST to /api/dfc/batch/start
   - Then: Receive 401 Unauthorized

3. **Upload to Non-Existent Batch**
   - Given: Invalid batch ID
   - When: POST to /api/dfc/batch/{invalid}/upload
   - Then: Receive 404 Not Found

4. **Complete Already Completed Batch**
   - Given: Batch already completed
   - When: POST to /api/dfc/batch/{id}/complete
   - Then: Receive 409 Conflict (idempotent)

5. **Fail Batch on Error**
   - Given: Processing error during upload
   - When: POST to /api/dfc/batch/{id}/fail
   - Then: Receive 200 OK, status "failed"

6. **Cancel Batch**
   - Given: Batch in progress
   - When: POST to /api/dfc/batch/{id}/cancel
   - Then: Receive 200 OK, status "cancelled"

7. **Multiple File Upload**
   - Given: Multiple files in multipart request
   - When: POST to /api/dfc/batch/{id}/upload
   - Then: All files uploaded, files array contains all entries

8. **Chunked Large File Upload**
   - Given: File larger than chunk size
   - When: Upload in multiple chunks
   - Then: All chunks accepted, file reconstructed server-side

## Notes

- Batch ID is UUID format, generated by server
- Batch lifecycle is linear: InProgress → (Completed | Failed | Cancelled)
- Server enforces batch timeout (default 1 hour)
- Client should cancel batch on timeout to cleanup server resources
- Multiple upload requests allowed per batch
- Upload endpoint accepts multiple files per request
- Server handles file deduplication and versioning
- Client deletes local CSV files after successful batch completion
- Batch protocol replaces v1.0 single-file upload endpoint
