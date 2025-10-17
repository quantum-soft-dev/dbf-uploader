# API Contract: Authentication v2.0

**Endpoint**: `POST /api/v1/auth/token`
**Purpose**: Obtain JWT token for API authentication via site credentials
**Used By**: Auth Client (initial installation, token renewal)
**Version**: 2.0

## Request

### Method
`POST`

### Headers
```
Authorization: Basic <base64(domain:client_secret)>
Content-Type: application/json
```

**Authorization Header Format**:
- Scheme: `Basic`
- Credentials: Base64 encoding of `domain:client_secret`
- Example: `Basic c3RvcmUtMDEuZXhhbXBsZS5jb206YTFiMmMzZDQtZTVmNi03ODkwLWFiY2QtZWYxMjM0NTY3ODkw`
  - Decoded: `store-01.example.com:a1b2c3d4-e5f6-7890-abcd-ef1234567890`

### Request Body
None (credentials in Authorization header)

### Example Request
```http
POST /api/v1/auth/token HTTP/1.1
Host: middleware.example.com
Authorization: Basic c3RvcmUtMDEuZXhhbXBsZS5jb206YTFiMmMzZDQtZTVmNi03ODkwLWFiY2QtZWYxMjM0NTY3ODkw
Content-Type: application/json
```

## Response

### Success Response (200 OK)

#### Headers
```
Content-Type: application/json
```

#### Body Schema (camelCase)
```json
{
  "token": "string (JWT format: header.payload.signature)",
  "expiresIn": "integer (seconds)",
  "tokenType": "string"
}
```

#### Field Descriptions
- `token`: JWT token string to be used for subsequent API requests
  - Format: Standard JWT with three parts separated by dots
  - Contains claims: `siteId`, `accountId`, `domain`, `exp`
  - Example: `eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzaXRlSWQiOiIxMjM0NTY3OC0xMjM0LTEyMzQtMTIzNC0xMjM0NTY3ODkwMTIiLCJhY2NvdW50SWQiOiI4NzY1NDMyMS00MzIxLTQzMjEtNDMyMS0yMTA5ODc2NTQzMjEiLCJkb21haW4iOiJzdG9yZS0wMS5leGFtcGxlLmNvbSIsImV4cCI6OTk5OTk5OTk5OX0.signature`
- `expiresIn`: Token lifetime in seconds from issuance
  - Typical value: `3600` (1 hour)
  - Client should track expiration and renew before expiry
- `tokenType`: Token type identifier
  - Value: `"Bearer"`
  - Used to construct Authorization header

#### Example Success Response
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzaXRlSWQiOiIxMjM0NTY3OC0xMjM0LTEyMzQtMTIzNC0xMjM0NTY3ODkwMTIiLCJhY2NvdW50SWQiOiI4NzY1NDMyMS00MzIxLTQzMjEtNDMyMS0yMTA5ODc2NTQzMjEiLCJkb21haW4iOiJzdG9yZS0wMS5leGFtcGxlLmNvbSIsImV4cCI6OTk5OTk5OTk5OX0.abc123xyz",
  "expiresIn": 3600,
  "tokenType": "Bearer"
}
```

### Error Responses

#### 401 Unauthorized - Invalid Credentials
```json
{
  "error": "Invalid credentials"
}
```

**When**: Domain or client_secret incorrect

#### 403 Forbidden - Subscription Inactive
```json
{
  "error": "subscription_inactive"
}
```

**When**: Site credentials valid but subscription expired/inactive
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
1. Parse JWT payload to extract `siteId`, `accountId`, `domain`, `exp`
2. Store `token` in memory cache (never persist to disk)
3. Calculate expiration time: `now + expiresIn`
4. Use token for all subsequent API requests with Bearer authentication
5. Automatically renew token before expiration

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
1. TokenManager checks token expiration before each operation
2. If expired or within 5 minutes of expiry: Automatically request new token
3. If renewal fails: Skip scheduled operation, report error
4. Token cache maintained in memory with automatic cleanup

## Security Requirements

- **HTTPS Only**: All requests must use HTTPS (TLS/SSL) in production
  - HTTP allowed only for localhost testing (127.0.0.1, localhost, [::1])
- **Site Credentials**: Domain and client_secret sent via Basic Auth over HTTPS
- **Client Secret Format**: UUID format (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
- **Token Storage**: JWT token stored in memory cache only, never written to disk
- **Token Transmission**: JWT token sent in Authorization header (Bearer scheme)
- **Automatic Token Renewal**: Client automatically renews tokens before expiration

## Sequence Diagram

```
Client                          Server
  │                               │
  │  POST /api/v1/auth/token      │
  │  Authorization: Basic ...     │
  │  (domain:client_secret)       │
  ├──────────────────────────────►│
  │                               │
  │                               │ Validate site credentials
  │                               │ Check subscription
  │                               │ Generate JWT with claims:
  │                               │   siteId, accountId, domain, exp
  │                               │
  │  200 OK                       │
  │  { token, expiresIn,          │
  │    tokenType: "Bearer" }      │
  │◄──────────────────────────────┤
  │                               │
  │  Parse JWT payload            │
  │  Store in TokenManager cache  │
  │  Calculate expiration         │
  │  Set auto-renewal timer       │
  │                               │
```

## Contract Tests

The following test scenarios must be implemented:

1. **Successful Authentication with Site Credentials**
   - Given: Valid domain and client_secret
   - When: POST to /api/v1/auth/token
   - Then: Receive 200 with valid JWT token, expiresIn, and tokenType
   - And: JWT payload contains siteId, accountId, domain, exp

2. **Invalid Site Credentials**
   - Given: Invalid domain or client_secret
   - When: POST to /api/v1/auth/token
   - Then: Receive 401 Unauthorized

3. **Inactive Subscription**
   - Given: Valid site credentials but inactive subscription
   - When: POST to /api/v1/auth/token
   - Then: Receive 403 Forbidden with "subscription_inactive" error

4. **Missing Authorization Header**
   - Given: No Authorization header
   - When: POST to /api/v1/auth/token
   - Then: Receive 401 or 400 error

5. **JWT Payload Parsing**
   - Given: Successful authentication response
   - When: Parse JWT payload
   - Then: Extract siteId, accountId, domain, exp claims successfully

6. **Token Expiration and Renewal**
   - Given: Token obtained with expiresIn value
   - When: TokenManager checks expiration
   - Then: Automatically renews token before expiry

7. **HTTPS Enforcement**
   - Given: Production HTTPS URL
   - When: Attempt to make request
   - Then: Request uses HTTPS-only client

8. **Localhost HTTP Exception**
   - Given: Localhost HTTP URL (testing)
   - When: Attempt to make request
   - Then: Client allows HTTP for localhost

## Notes

- Token format and expiration are controlled by server
- Client parses JWT payload to extract siteId, accountId, domain, exp
- Client checks expiration based on both `expiresIn` value and JWT `exp` claim
- No refresh token mechanism - full re-authentication required on expiration
- TokenManager handles automatic token renewal and caching
- Site credentials (domain + client_secret) replace v1.0 username/password authentication
