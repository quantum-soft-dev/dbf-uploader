# API Contract: Authentication

**Endpoint**: `POST /api/auth/token`
**Purpose**: Obtain JWT token for API authentication
**Used By**: Auth Client (initial installation, token renewal)

## Request

### Method
`POST`

### Headers
```
Authorization: Basic <base64(username:password)>
Content-Type: application/json
```

**Authorization Header Format**:
- Scheme: `Basic`
- Credentials: Base64 encoding of `username:password`
- Example: `Basic dXNlcm5hbWU6cGFzc3dvcmQ=` (username:password)

### Request Body
None (credentials in Authorization header)

### Example Request
```http
POST /api/auth/token HTTP/1.1
Host: api.example.com
Authorization: Basic dXNlcm5hbWU6cGFzc3dvcmQ=
Content-Type: application/json
```

## Response

### Success Response (200 OK)

#### Headers
```
Content-Type: application/json
```

#### Body Schema
```json
{
  "token": "string (JWT format: header.payload.signature)",
  "expires_in": "integer (seconds)"
}
```

#### Field Descriptions
- `token`: JWT token string to be used for subsequent API requests
  - Format: Standard JWT with three parts separated by dots
  - Example: `eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VybmFtZSIsImV4cCI6MTcwMDAwMDAwMH0.signature`
- `expires_in`: Token lifetime in seconds from issuance
  - Typical value: `86400` (24 hours)
  - Client should track expiration and renew before expiry

#### Example Success Response
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VybmFtZSIsImV4cCI6MTcwMDAwMDAwMH0.abc123xyz",
  "expires_in": 86400
}
```

### Error Responses

#### 401 Unauthorized - Invalid Credentials
```json
{
  "error": "unauthorized",
  "message": "Invalid username or password"
}
```

**When**: Username or password incorrect

#### 403 Forbidden - Subscription Inactive
```json
{
  "error": "forbidden",
  "message": "Subscription is not active"
}
```

**When**: Credentials valid but subscription expired/inactive
**Client Action**: Abort installation with error message

#### 400 Bad Request - Missing Authorization
```json
{
  "error": "bad_request",
  "message": "Missing Authorization header"
}
```

**When**: Authorization header not provided

#### 500 Internal Server Error
```json
{
  "error": "internal_error",
  "message": "An internal error occurred"
}
```

**When**: Server-side error
**Client Action**: Log error, retry with exponential backoff (max 3 attempts)

## Client Behavior Requirements

### On Success (200)
1. Store `token` in memory (never persist to disk)
2. Calculate expiration time: `now + expires_in`
3. Use token for all subsequent API requests with Bearer authentication

### On 401 Unauthorized
1. During installation: Terminate with clear error message to user
2. During token renewal: Skip scheduled operation, report error to server (if possible)

### On 403 Forbidden
1. During installation: Terminate with "Subscription inactive" message
2. During operation: Skip scheduled operation, log error locally

### On Network Error
1. During installation: Terminate with network error message
2. During operation: Skip scheduled operation, write to local error log

### Token Expiration Handling
1. Check token expiration before each scheduled run
2. If expired or within 5 minutes of expiry: Request new token
3. If renewal fails: Skip scheduled operation, report error

## Security Requirements

- **HTTPS Only**: All requests must use HTTPS (TLS/SSL)
- **Credential Transmission**: Basic Auth credentials sent over HTTPS only
- **Token Storage**: JWT token stored in memory only, never written to disk
- **Token Transmission**: JWT token sent in Authorization header (Bearer scheme)

## Sequence Diagram

```
Client                          Server
  │                               │
  │  POST /api/auth/token         │
  │  Authorization: Basic ...     │
  ├──────────────────────────────►│
  │                               │
  │                               │ Validate credentials
  │                               │ Check subscription
  │                               │ Generate JWT
  │                               │
  │  200 OK                       │
  │  { token, expires_in }        │
  │◄──────────────────────────────┤
  │                               │
  │  Store token in memory        │
  │  Calculate expiration         │
  │                               │
```

## Contract Tests

The following test scenarios must be implemented:

1. **Successful Authentication**
   - Given: Valid username and password
   - When: POST to /api/auth/token
   - Then: Receive 200 with valid JWT token and expires_in

2. **Invalid Credentials**
   - Given: Invalid username or password
   - When: POST to /api/auth/token
   - Then: Receive 401 Unauthorized

3. **Inactive Subscription**
   - Given: Valid credentials but inactive subscription
   - When: POST to /api/auth/token
   - Then: Receive 403 Forbidden

4. **Missing Authorization Header**
   - Given: No Authorization header
   - When: POST to /api/auth/token
   - Then: Receive 400 Bad Request

5. **Token Expiration**
   - Given: Token obtained with expires_in value
   - When: Time passes beyond expires_in
   - Then: Client recognizes token as expired

6. **HTTPS Enforcement**
   - Given: HTTP URL (not HTTPS)
   - When: Attempt to make request
   - Then: Client rejects request (HTTPS-only enforcement)

## Notes

- Token format and expiration are controlled by server
- Client does not parse or validate JWT payload
- Client only checks expiration based on `expires_in` value
- No refresh token mechanism - full re-authentication required on expiration
