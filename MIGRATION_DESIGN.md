# DBF Uploader v2.0 Migration Design

**Status**: Design Phase
**Version**: 2.0.0 (Breaking Changes)
**Target**: Protocol compatibility with data-forge-middleware
**Date**: 2025-10-17

## Executive Summary

DBF Uploader v1.0 uses a legacy protocol incompatible with the current data-forge-middleware batch API. This document details the architectural changes required for v2.0 migration.

### Critical Incompatibilities
1. ❌ **No batch lifecycle** - Direct file upload vs. batch/start → upload → complete
2. ❌ **Wrong authentication** - username:password vs. domain:clientSecret (site credentials)
3. ❌ **Different endpoints** - `/api/files/upload` vs. `/api/v1/batch/{id}/upload`
4. ❌ **Incompatible error reporting** - Different schemas and endpoints

---

## Architecture Overview

### V1.0 (Current - Incompatible)
```
┌─────────────┐
│   Uploader  │
│             │
│  Auth       │──► POST /api/auth/token (username:password)
│  ↓          │
│  Upload     │──► POST /api/files/upload (direct, single file)
│  ↓          │
│  Error Log  │──► POST /api/errors/report
└─────────────┘
```

### V2.0 (Target - Middleware Compatible)
```
┌─────────────┐
│   Uploader  │
│             │
│  Auth       │──► POST /api/v1/auth/token (domain:clientSecret)
│  ↓          │
│  Batch      │──┬► POST /api/v1/batch/start
│  Lifecycle  │  │
│             │  ├► POST /api/v1/batch/{id}/upload (multiple files)
│             │  │
│             │  └► POST /api/v1/batch/{id}/complete
│  ↓          │
│  Error Log  │──► POST /api/v1/error[/{batchId}]
└─────────────┘
```

---

## Component Design

### 1. Authentication Module Redesign

#### V1.0 (Current)
```rust
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub struct AuthClient {
    base_url: String,
    credentials: Credentials,
    token: Option<JwtToken>,
}

impl AuthClient {
    pub async fn authenticate(&mut self) -> Result<String> {
        let basic_auth = format!("{}:{}",
            self.credentials.username,
            self.credentials.password
        );

        let response = self.http_client
            .post(&format!("{}/api/auth/token", self.base_url))
            .header("Authorization", format!("Basic {}",
                base64::encode(basic_auth)))
            .send()
            .await?;

        let token_response: TokenResponse = response.json().await?;
        self.token = Some(JwtToken {
            value: token_response.token,
            expires_at: Utc::now() + Duration::seconds(token_response.expires_in),
        });

        Ok(token_response.token)
    }
}
```

#### V2.0 (Target)
```rust
/// Site credentials for middleware authentication
pub struct SiteCredentials {
    /// Site domain (e.g., "store-01.example.com")
    pub domain: String,
    /// Client secret UUID from middleware admin API
    pub client_secret: String,
}

/// JWT token with site context
pub struct JwtToken {
    pub value: String,
    pub expires_at: DateTime<Utc>,
    pub site_id: Uuid,      // NEW: from JWT payload
    pub account_id: Uuid,   // NEW: from JWT payload
}

pub struct AuthClient {
    base_url: String,
    credentials: SiteCredentials,
    token: Option<JwtToken>,
}

impl AuthClient {
    /// Authenticate with middleware using site credentials
    pub async fn authenticate(&mut self) -> Result<JwtToken> {
        let basic_auth = format!("{}:{}",
            self.credentials.domain,
            self.credentials.client_secret
        );

        let response = self.http_client
            .post(&format!("{}/api/v1/auth/token", self.base_url))
            .header("Authorization", format!("Basic {}",
                base64::encode(basic_auth)))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuthError::InvalidCredentials(
                response.text().await?
            ));
        }

        let token_response: TokenResponse = response.json().await?;

        // Parse JWT payload to extract site context
        let payload = self.parse_jwt_payload(&token_response.token)?;

        let token = JwtToken {
            value: token_response.token.clone(),
            expires_at: Utc::now() + Duration::seconds(token_response.expires_in),
            site_id: payload.site_id,
            account_id: payload.account_id,
        };

        self.token = Some(token.clone());
        Ok(token)
    }

    /// Check if token needs renewal (within 5 minutes of expiry)
    pub fn needs_renewal(&self) -> bool {
        match &self.token {
            None => true,
            Some(token) => {
                let threshold = Utc::now() + Duration::minutes(5);
                token.expires_at < threshold
            }
        }
    }

    /// Parse JWT payload without validation (extract claims)
    fn parse_jwt_payload(&self, token: &str) -> Result<JwtPayload> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken);
        }

        let payload = base64::decode(parts[1])?;
        let payload: JwtPayload = serde_json::from_slice(&payload)?;
        Ok(payload)
    }
}

#[derive(Deserialize)]
struct JwtPayload {
    #[serde(rename = "siteId")]
    site_id: Uuid,
    #[serde(rename = "accountId")]
    account_id: Uuid,
    domain: String,
    exp: i64,
}

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
    #[serde(rename = "expiresIn")]
    expires_in: i64,
    #[serde(rename = "tokenType")]
    token_type: String,
}
```

**Key Changes:**
- ✅ Replace username:password with domain:clientSecret
- ✅ Update endpoint from `/api/auth/token` to `/api/v1/auth/token`
- ✅ Parse JWT payload to extract siteId and accountId
- ✅ Add token renewal check with 5-minute threshold
- ✅ Enhanced error handling for auth failures

---

### 2. Batch Lifecycle Manager (NEW)

```rust
/// Batch lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchState {
    NotStarted,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Batch metadata
pub struct Batch {
    pub id: Uuid,
    pub site_id: Uuid,
    pub state: BatchState,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub uploaded_files: Vec<String>,  // Filenames
    pub total_size: u64,
    pub has_errors: bool,
}

/// Manages batch lifecycle with middleware
pub struct BatchManager {
    http_client: reqwest::Client,
    base_url: String,
    auth_client: Arc<Mutex<AuthClient>>,
    current_batch: Option<Batch>,
}

impl BatchManager {
    pub fn new(
        base_url: String,
        auth_client: Arc<Mutex<AuthClient>>,
    ) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url,
            auth_client,
            current_batch: None,
        }
    }

    /// Start a new batch session
    pub async fn start_batch(&mut self) -> Result<Uuid> {
        // Ensure token is valid
        let mut auth = self.auth_client.lock().await;
        if auth.needs_renewal() {
            auth.authenticate().await?;
        }
        let token = auth.token.as_ref()
            .ok_or(BatchError::NoAuthToken)?;
        drop(auth);

        // Call middleware batch/start
        let response = self.http_client
            .post(&format!("{}/api/v1/batch/start", self.base_url))
            .bearer_auth(&token.value)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(BatchError::StartFailed(
                response.text().await?
            ));
        }

        let batch_response: BatchStartResponse = response.json().await?;

        self.current_batch = Some(Batch {
            id: batch_response.batch_id,
            site_id: token.site_id,
            state: BatchState::InProgress,
            started_at: Utc::now(),
            completed_at: None,
            uploaded_files: Vec::new(),
            total_size: 0,
            has_errors: false,
        });

        info!("Batch started: {}", batch_response.batch_id);
        Ok(batch_response.batch_id)
    }

    /// Upload files to current batch
    pub async fn upload_files(
        &mut self,
        files: Vec<PathBuf>,
    ) -> Result<UploadSummary> {
        let batch = self.current_batch.as_mut()
            .ok_or(BatchError::NoBatchActive)?;

        if batch.state != BatchState::InProgress {
            return Err(BatchError::BatchNotInProgress);
        }

        let auth = self.auth_client.lock().await;
        let token = auth.token.as_ref()
            .ok_or(BatchError::NoAuthToken)?;
        drop(auth);

        let mut form = multipart::Form::new();
        let mut total_uploaded = 0;
        let mut failed_files = Vec::new();

        // Add all files to multipart form
        for file_path in &files {
            match tokio::fs::read(file_path).await {
                Ok(data) => {
                    let filename = file_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");

                    form = form.part(
                        "files",
                        multipart::Part::bytes(data)
                            .file_name(filename.to_string())
                            .mime_str("application/gzip")?
                    );

                    total_uploaded += 1;
                    batch.uploaded_files.push(filename.to_string());
                }
                Err(e) => {
                    warn!("Failed to read file {:?}: {}", file_path, e);
                    failed_files.push(file_path.clone());
                }
            }
        }

        if total_uploaded == 0 {
            return Err(BatchError::NoFilesToUpload);
        }

        // Upload to middleware
        let response = self.http_client
            .post(&format!("{}/api/v1/batch/{}/upload",
                self.base_url, batch.id))
            .bearer_auth(&token.value)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            batch.has_errors = true;
            return Err(BatchError::UploadFailed(
                response.text().await?
            ));
        }

        let upload_response: UploadResponse = response.json().await?;
        batch.total_size += upload_response.files.iter()
            .map(|f| f.file_size)
            .sum::<u64>();

        info!("Uploaded {} files to batch {}", total_uploaded, batch.id);

        Ok(UploadSummary {
            uploaded: total_uploaded,
            failed: failed_files.len(),
            failed_files,
        })
    }

    /// Complete current batch
    pub async fn complete_batch(&mut self) -> Result<BatchSummary> {
        let batch = self.current_batch.as_mut()
            .ok_or(BatchError::NoBatchActive)?;

        if batch.state != BatchState::InProgress {
            return Err(BatchError::BatchNotInProgress);
        }

        let auth = self.auth_client.lock().await;
        let token = auth.token.as_ref()
            .ok_or(BatchError::NoAuthToken)?;
        drop(auth);

        let response = self.http_client
            .post(&format!("{}/api/v1/batch/{}/complete",
                self.base_url, batch.id))
            .bearer_auth(&token.value)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(BatchError::CompleteFailed(
                response.text().await?
            ));
        }

        let complete_response: BatchCompleteResponse = response.json().await?;

        batch.state = BatchState::Completed;
        batch.completed_at = Some(Utc::now());

        info!("Batch {} completed: {} files, {} bytes",
            batch.id,
            complete_response.uploaded_files_count,
            complete_response.total_size
        );

        let summary = BatchSummary {
            batch_id: batch.id,
            uploaded_count: complete_response.uploaded_files_count,
            total_size: complete_response.total_size,
            duration: (Utc::now() - batch.started_at).num_seconds(),
            has_errors: batch.has_errors,
        };

        self.current_batch = None;
        Ok(summary)
    }

    /// Fail current batch (on critical error)
    pub async fn fail_batch(&mut self, reason: &str) -> Result<()> {
        let batch = self.current_batch.as_mut()
            .ok_or(BatchError::NoBatchActive)?;

        let auth = self.auth_client.lock().await;
        let token = auth.token.as_ref()
            .ok_or(BatchError::NoAuthToken)?;
        drop(auth);

        let response = self.http_client
            .post(&format!("{}/api/v1/batch/{}/fail",
                self.base_url, batch.id))
            .bearer_auth(&token.value)
            .json(&serde_json::json!({
                "reason": reason
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            warn!("Failed to mark batch as failed: {}",
                response.text().await?);
        }

        batch.state = BatchState::Failed;
        self.current_batch = None;

        Ok(())
    }

    /// Cancel current batch
    pub async fn cancel_batch(&mut self) -> Result<()> {
        let batch = self.current_batch.as_mut()
            .ok_or(BatchError::NoBatchActive)?;

        let auth = self.auth_client.lock().await;
        let token = auth.token.as_ref()
            .ok_or(BatchError::NoAuthToken)?;
        drop(auth);

        let response = self.http_client
            .post(&format!("{}/api/v1/batch/{}/cancel",
                self.base_url, batch.id))
            .bearer_auth(&token.value)
            .send()
            .await?;

        if !response.status().is_success() {
            warn!("Failed to cancel batch: {}",
                response.text().await?);
        }

        batch.state = BatchState::Cancelled;
        self.current_batch = None;

        Ok(())
    }
}

// Response DTOs
#[derive(Deserialize)]
struct BatchStartResponse {
    #[serde(rename = "batchId")]
    batch_id: Uuid,
}

#[derive(Deserialize)]
struct UploadResponse {
    status: String,
    #[serde(rename = "uploadedFiles")]
    uploaded_files: usize,
    files: Vec<UploadedFileInfo>,
}

#[derive(Deserialize)]
struct UploadedFileInfo {
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "fileSize")]
    file_size: u64,
    #[serde(rename = "uploadedAt")]
    uploaded_at: String,
}

#[derive(Deserialize)]
struct BatchCompleteResponse {
    #[serde(rename = "batchId")]
    batch_id: Uuid,
    status: String,
    #[serde(rename = "completedAt")]
    completed_at: String,
    #[serde(rename = "uploadedFilesCount")]
    uploaded_files_count: usize,
    #[serde(rename = "totalSize")]
    total_size: u64,
}

// Result types
pub struct UploadSummary {
    pub uploaded: usize,
    pub failed: usize,
    pub failed_files: Vec<PathBuf>,
}

pub struct BatchSummary {
    pub batch_id: Uuid,
    pub uploaded_count: usize,
    pub total_size: u64,
    pub duration: i64,
    pub has_errors: bool,
}
```

**Key Features:**
- ✅ Full batch lifecycle management (start → upload → complete/fail/cancel)
- ✅ State tracking and validation
- ✅ Multiple file upload support
- ✅ Error handling and recovery
- ✅ Automatic token renewal
- ✅ Detailed logging and metrics

---

### 3. Error Reporting Module Update

#### V2.0 Design
```rust
/// Error reporting client compatible with middleware
pub struct ErrorReporter {
    http_client: reqwest::Client,
    base_url: String,
    auth_client: Arc<Mutex<AuthClient>>,
    local_log_path: PathBuf,
    client_version: String,
}

impl ErrorReporter {
    /// Report error associated with batch
    pub async fn report_batch_error(
        &self,
        batch_id: Uuid,
        error_type: &str,
        message: String,
        metadata: HashMap<String, Value>,
    ) -> Result<()> {
        let auth = self.auth_client.lock().await;
        let token = auth.token.as_ref();
        drop(auth);

        let error_log = ErrorLogRequest {
            type_: error_type.to_string(),
            message,
            metadata,
            client_version: self.client_version.clone(),
        };

        let result = match token {
            Some(t) => {
                self.http_client
                    .post(&format!("{}/api/v1/error/{}",
                        self.base_url, batch_id))
                    .bearer_auth(&t.value)
                    .json(&error_log)
                    .send()
                    .await
            }
            None => {
                // Try without auth for error reports
                self.http_client
                    .post(&format!("{}/api/v1/error/{}",
                        self.base_url, batch_id))
                    .json(&error_log)
                    .send()
                    .await
            }
        };

        match result {
            Ok(response) if response.status().is_success() => {
                info!("Error reported to server");
                Ok(())
            }
            Ok(response) => {
                let error_text = response.text().await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                self.log_to_local_file(&format!(
                    "Failed to report error to server: {}", error_text
                )).await;
                Ok(())
            }
            Err(e) => {
                self.log_to_local_file(&format!(
                    "Network error reporting to server: {}", e
                )).await;
                Ok(())
            }
        }
    }

    /// Report standalone error (not associated with batch)
    pub async fn report_standalone_error(
        &self,
        error_type: &str,
        message: String,
        metadata: HashMap<String, Value>,
    ) -> Result<()> {
        let auth = self.auth_client.lock().await;
        let token = auth.token.as_ref();
        drop(auth);

        let error_log = ErrorLogRequest {
            type_: error_type.to_string(),
            message,
            metadata,
            client_version: self.client_version.clone(),
        };

        let result = match token {
            Some(t) => {
                self.http_client
                    .post(&format!("{}/api/v1/error", self.base_url))
                    .bearer_auth(&t.value)
                    .json(&error_log)
                    .send()
                    .await
            }
            None => {
                self.http_client
                    .post(&format!("{}/api/v1/error", self.base_url))
                    .json(&error_log)
                    .send()
                    .await
            }
        };

        match result {
            Ok(response) if response.status().is_success() => {
                info!("Error reported to server");
                Ok(())
            }
            _ => {
                self.log_to_local_file(&format!(
                    "Failed to report standalone error: {}", error_type
                )).await;
                Ok(())
            }
        }
    }

    /// Fallback to local error log
    async fn log_to_local_file(&self, message: &str) {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        let log_entry = format!("[{}] {}\n", timestamp, message);

        if let Err(e) = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.local_log_path)
            .await
            .and_then(|mut file| async move {
                use tokio::io::AsyncWriteExt;
                file.write_all(log_entry.as_bytes()).await
            }.boxed())
            .await
        {
            eprintln!("Failed to write to local error log: {}", e);
        }
    }
}

#[derive(Serialize)]
struct ErrorLogRequest {
    #[serde(rename = "type")]
    type_: String,
    message: String,
    metadata: HashMap<String, Value>,
    #[serde(rename = "clientVersion")]
    client_version: String,
}
```

**Key Changes:**
- ✅ Update endpoint from `/api/errors/report` to `/api/v1/error[/{batchId}]`
- ✅ Change schema to match middleware (type, message, metadata, clientVersion)
- ✅ Support batch-associated and standalone errors
- ✅ Automatic fallback to local log on failure
- ✅ No retry logic (fire-and-forget)

---

### 4. Configuration Migration

#### V1.0 Config (config.toml)
```toml
[auth]
username = "user123"
password = "password123"

[api]
base_url = "https://api.example.com"

[source]
directory = "C:\\Program Files\\abc\\data"

[schedule]
cron = "0 8,12,16,18 * * *"

[encoding]
fallback = "CP866"
```

#### V2.0 Config (config.toml)
```toml
# DBF Uploader v2.0 Configuration
# Migration from v1.0 requires new site credentials

[auth]
# Site domain from middleware admin API
domain = "store-01.example.com"
# Client secret UUID from middleware admin API
client_secret = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"

[api]
base_url = "https://api.example.com"
# V2 endpoints
auth_endpoint = "/api/v1/auth/token"
batch_start = "/api/v1/batch/start"
batch_upload = "/api/v1/batch/{batchId}/upload"
batch_complete = "/api/v1/batch/{batchId}/complete"
batch_fail = "/api/v1/batch/{batchId}/fail"
error_log = "/api/v1/error"

[source]
directory = "C:\\Program Files\\abc\\data"

[schedule]
cron = "0 8,12,16,18 * * *"

[encoding]
fallback = "CP866"

[batch]
# Maximum files per batch (middleware limit: 1000)
max_files_per_batch = 500
# Retry failed files at end of batch
retry_locked_files = true

[logging]
# Local error log fallback
error_log_path = "C:\\Program Files\\dbf-uploader\\error.log"
```

#### Migration Tool Design
```rust
/// Configuration migration from v1 to v2
pub struct ConfigMigration;

impl ConfigMigration {
    /// Detect config version
    pub fn detect_version(config_path: &Path) -> Result<ConfigVersion> {
        let content = std::fs::read_to_string(config_path)?;

        if content.contains("[auth]\nusername") {
            Ok(ConfigVersion::V1)
        } else if content.contains("[auth]\ndomain") {
            Ok(ConfigVersion::V2)
        } else {
            Err(ConfigError::UnknownVersion)
        }
    }

    /// Migrate v1 config to v2 format
    pub fn migrate_v1_to_v2(
        v1_config: ConfigV1,
    ) -> Result<MigrationGuide> {
        println!("╔═══════════════════════════════════════════════════╗");
        println!("║   DBF Uploader Configuration Migration v1 → v2   ║");
        println!("╚═══════════════════════════════════════════════════╝");
        println!();
        println!("⚠️  BREAKING CHANGES:");
        println!("  • Username/password authentication REMOVED");
        println!("  • Site credentials (domain + clientSecret) REQUIRED");
        println!("  • Batch protocol lifecycle REQUIRED");
        println!();
        println!("📋 Migration Steps:");
        println!("  1. Log in to middleware admin UI");
        println!("  2. Navigate to Accounts → Your Account");
        println!("  3. Create a new Site for this uploader");
        println!("  4. Copy the domain and clientSecret");
        println!("  5. Update config.toml with new credentials");
        println!();

        // Generate template config
        let template = ConfigV2 {
            auth: AuthConfigV2 {
                domain: "REPLACE_WITH_YOUR_SITE_DOMAIN".to_string(),
                client_secret: "REPLACE_WITH_CLIENT_SECRET_UUID".to_string(),
            },
            api: ApiConfigV2 {
                base_url: v1_config.api.base_url,
                auth_endpoint: "/api/v1/auth/token".to_string(),
                batch_start: "/api/v1/batch/start".to_string(),
                batch_upload: "/api/v1/batch/{batchId}/upload".to_string(),
                batch_complete: "/api/v1/batch/{batchId}/complete".to_string(),
                batch_fail: "/api/v1/batch/{batchId}/fail".to_string(),
                error_log: "/api/v1/error".to_string(),
            },
            source: v1_config.source,
            schedule: v1_config.schedule,
            encoding: v1_config.encoding,
            batch: BatchConfig {
                max_files_per_batch: 500,
                retry_locked_files: true,
            },
            logging: LoggingConfig {
                error_log_path: PathBuf::from(
                    "C:\\Program Files\\dbf-uploader\\error.log"
                ),
            },
        };

        let template_toml = toml::to_string_pretty(&template)?;

        Ok(MigrationGuide {
            old_config: v1_config,
            template_config: template_toml,
            instructions: vec![
                "Backup your current config.toml".to_string(),
                "Get site credentials from middleware admin UI".to_string(),
                "Replace config.toml with new template".to_string(),
                "Update domain and client_secret fields".to_string(),
                "Test authentication with: dbf-uploader test-auth".to_string(),
                "Restart the Windows service".to_string(),
            ],
        })
    }

    /// Interactive migration wizard
    pub async fn run_wizard() -> Result<ConfigV2> {
        println!("🧙 Configuration Migration Wizard");
        println!();

        let domain = prompt("Enter site domain: ")?;
        let client_secret = prompt("Enter client secret: ")?;
        let base_url = prompt("Enter API base URL: ")?;
        let source_dir = prompt("Enter source directory: ")?;

        // Test authentication
        println!("\n🔍 Testing authentication...");
        let test_result = Self::test_auth(
            &base_url, &domain, &client_secret
        ).await;

        match test_result {
            Ok(_) => {
                println!("✅ Authentication successful!");
            }
            Err(e) => {
                println!("❌ Authentication failed: {}", e);
                println!("Please verify your credentials and try again.");
                return Err(ConfigError::AuthTestFailed);
            }
        }

        // Generate config
        let config = ConfigV2 {
            auth: AuthConfigV2 { domain, client_secret },
            api: ApiConfigV2 {
                base_url,
                auth_endpoint: "/api/v1/auth/token".to_string(),
                batch_start: "/api/v1/batch/start".to_string(),
                batch_upload: "/api/v1/batch/{batchId}/upload".to_string(),
                batch_complete: "/api/v1/batch/{batchId}/complete".to_string(),
                batch_fail: "/api/v1/batch/{batchId}/fail".to_string(),
                error_log: "/api/v1/error".to_string(),
            },
            source: SourceConfig {
                directory: PathBuf::from(source_dir),
            },
            schedule: ScheduleConfig {
                cron: "0 8,12,16,18 * * *".to_string(),
            },
            encoding: EncodingConfig {
                fallback: "CP866".to_string(),
            },
            batch: BatchConfig {
                max_files_per_batch: 500,
                retry_locked_files: true,
            },
            logging: LoggingConfig {
                error_log_path: PathBuf::from(
                    "C:\\Program Files\\dbf-uploader\\error.log"
                ),
            },
        };

        Ok(config)
    }

    async fn test_auth(
        base_url: &str,
        domain: &str,
        client_secret: &str,
    ) -> Result<()> {
        let client = reqwest::Client::new();
        let basic_auth = format!("{}:{}", domain, client_secret);

        let response = client
            .post(&format!("{}/api/v1/auth/token", base_url))
            .header("Authorization", format!("Basic {}",
                base64::encode(basic_auth)))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ConfigError::AuthFailed(response.text().await?))
        }
    }
}

pub struct MigrationGuide {
    pub old_config: ConfigV1,
    pub template_config: String,
    pub instructions: Vec<String>,
}
```

**Migration Process:**
1. Detect existing v1.0 config
2. Display migration guide with instructions
3. Run interactive wizard to gather new credentials
4. Test authentication before finalizing
5. Generate v2.0 config template
6. Backup old config and install new one

---

### 5. Main Processing Loop Update

#### V2.0 Design
```rust
pub struct UploaderService {
    auth_client: Arc<Mutex<AuthClient>>,
    batch_manager: Arc<Mutex<BatchManager>>,
    error_reporter: Arc<ErrorReporter>,
    config: Config,
}

impl UploaderService {
    /// Execute scheduled batch upload
    pub async fn run_scheduled_batch(&self) -> Result<()> {
        info!("Starting scheduled batch upload");

        // 1. Authenticate if needed
        {
            let mut auth = self.auth_client.lock().await;
            if auth.needs_renewal() {
                match auth.authenticate().await {
                    Ok(_) => info!("Authentication successful"),
                    Err(e) => {
                        error!("Authentication failed: {}", e);
                        self.error_reporter.report_standalone_error(
                            "AuthenticationError",
                            format!("Failed to authenticate: {}", e),
                            HashMap::new(),
                        ).await?;
                        return Err(e.into());
                    }
                }
            }
        }

        // 2. Start new batch
        let batch_id = {
            let mut batch_mgr = self.batch_manager.lock().await;
            match batch_mgr.start_batch().await {
                Ok(id) => id,
                Err(e) => {
                    error!("Failed to start batch: {}", e);
                    self.error_reporter.report_standalone_error(
                        "BatchStartError",
                        format!("Failed to start batch: {}", e),
                        HashMap::new(),
                    ).await?;
                    return Err(e.into());
                }
            }
        };

        info!("Batch started: {}", batch_id);

        // 3. Scan for DBF files
        let dbf_files = self.scan_dbf_files(&self.config.source.directory)?;
        info!("Found {} DBF files", dbf_files.len());

        // 4. Convert DBF → CSV → gzip
        let mut converted_files = Vec::new();
        let mut locked_files = Vec::new();

        for dbf_path in &dbf_files {
            match self.convert_and_compress(dbf_path).await {
                Ok(compressed_path) => {
                    converted_files.push(compressed_path);
                }
                Err(ConversionError::FileLocked) => {
                    warn!("File locked, will retry: {:?}", dbf_path);
                    locked_files.push(dbf_path.clone());
                }
                Err(e) => {
                    error!("Conversion failed for {:?}: {}", dbf_path, e);
                    self.error_reporter.report_batch_error(
                        batch_id,
                        &e.error_type(),
                        format!("Failed to convert {}: {}",
                            dbf_path.display(), e),
                        HashMap::new(),
                    ).await?;
                }
            }
        }

        // 5. Retry locked files
        if self.config.batch.retry_locked_files && !locked_files.is_empty() {
            info!("Retrying {} locked files", locked_files.len());
            tokio::time::sleep(Duration::from_secs(5)).await;

            for dbf_path in &locked_files {
                if let Ok(compressed_path) = self.convert_and_compress(dbf_path).await {
                    converted_files.push(compressed_path);
                }
            }
        }

        if converted_files.is_empty() {
            warn!("No files to upload");
            let mut batch_mgr = self.batch_manager.lock().await;
            batch_mgr.cancel_batch().await?;
            return Ok(());
        }

        // 6. Upload files in batches
        let max_per_batch = self.config.batch.max_files_per_batch;
        for chunk in converted_files.chunks(max_per_batch) {
            let mut batch_mgr = self.batch_manager.lock().await;

            match batch_mgr.upload_files(chunk.to_vec()).await {
                Ok(summary) => {
                    info!("Uploaded {} files, {} failed",
                        summary.uploaded, summary.failed);

                    for failed_path in &summary.failed_files {
                        self.error_reporter.report_batch_error(
                            batch_id,
                            "UploadError",
                            format!("Failed to upload: {}", failed_path.display()),
                            HashMap::new(),
                        ).await?;
                    }
                }
                Err(e) => {
                    error!("Batch upload failed: {}", e);
                    self.error_reporter.report_batch_error(
                        batch_id,
                        "UploadError",
                        format!("Batch upload failed: {}", e),
                        HashMap::new(),
                    ).await?;

                    let mut batch_mgr = self.batch_manager.lock().await;
                    batch_mgr.fail_batch(&format!("Upload failed: {}", e)).await?;
                    return Err(e.into());
                }
            }
        }

        // 7. Complete batch
        {
            let mut batch_mgr = self.batch_manager.lock().await;
            match batch_mgr.complete_batch().await {
                Ok(summary) => {
                    info!("Batch completed: {} files, {} bytes, {} seconds",
                        summary.uploaded_count,
                        summary.total_size,
                        summary.duration
                    );
                }
                Err(e) => {
                    error!("Failed to complete batch: {}", e);
                    self.error_reporter.report_batch_error(
                        batch_id,
                        "BatchCompleteError",
                        format!("Failed to complete batch: {}", e),
                        HashMap::new(),
                    ).await?;
                    return Err(e.into());
                }
            }
        }

        // 8. Cleanup converted files
        for file in &converted_files {
            if let Err(e) = tokio::fs::remove_file(file).await {
                warn!("Failed to delete {}: {}", file.display(), e);
            }
        }

        info!("Batch upload completed successfully");
        Ok(())
    }
}
```

**Key Changes:**
- ✅ Batch lifecycle integration (start → upload → complete)
- ✅ Automatic authentication renewal
- ✅ Batch-aware error reporting
- ✅ Locked file retry logic
- ✅ Chunked uploads respecting max_files_per_batch
- ✅ Proper cleanup and error handling

---

## Implementation Plan

### Phase 1: Core Protocol Migration (Week 1-2)
1. **Update authentication module**
   - Implement SiteCredentials
   - Update JWT parsing
   - Add token renewal logic

2. **Implement BatchManager**
   - Batch lifecycle methods
   - State tracking
   - Multipart upload support

3. **Update error reporting**
   - New endpoints
   - New schema
   - Batch association

### Phase 2: Configuration & Migration (Week 2)
1. **Create ConfigV2 structures**
2. **Implement migration tool**
3. **Build interactive wizard**
4. **Add config validation**

### Phase 3: Integration & Testing (Week 3)
1. **Update main processing loop**
2. **Contract tests for all endpoints**
3. **Integration tests with middleware**
4. **End-to-end testing**

### Phase 4: Deployment & Documentation (Week 4)
1. **Update user documentation**
2. **Create migration guide**
3. **Windows service installer update**
4. **Release v2.0.0**

---

## Testing Strategy

### Contract Tests
```rust
#[tokio::test]
async fn test_auth_with_site_credentials() {
    // Test new authentication flow
}

#[tokio::test]
async fn test_batch_lifecycle() {
    // Test start → upload → complete
}

#[tokio::test]
async fn test_batch_error_reporting() {
    // Test error reporting with batch association
}

#[tokio::test]
async fn test_multipart_upload() {
    // Test multiple files in single request
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_full_upload_workflow() {
    // End-to-end test with real middleware
}

#[tokio::test]
async fn test_authentication_renewal() {
    // Test token expiration and renewal
}

#[tokio::test]
async fn test_batch_timeout_handling() {
    // Test middleware batch timeout
}
```

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking changes for existing users | High | Migration tool, documentation, versioning |
| Authentication failures during migration | High | Test wizard, validation, rollback plan |
| Batch timeout issues | Medium | Configurable timeouts, automatic retry |
| File upload failures | Medium | Chunked uploads, retry logic, error reporting |
| Configuration errors | Medium | Interactive wizard, validation, clear errors |

---

## Success Criteria

- ✅ All endpoints compatible with middleware batch protocol
- ✅ Authentication using site credentials works
- ✅ Batch lifecycle (start → upload → complete) functional
- ✅ Error reporting integrated with middleware
- ✅ Migration tool successfully converts v1 to v2 configs
- ✅ Contract tests pass against middleware API
- ✅ End-to-end testing with real DBF files successful
- ✅ Documentation complete and clear

---

## Version 2.0.0 Release Notes

### Breaking Changes
- ❌ Removed username:password authentication
- ❌ Removed direct file upload endpoint
- ❌ Changed error reporting schema

### New Features
- ✅ Batch lifecycle management
- ✅ Site credentials authentication
- ✅ Multiple file upload support
- ✅ Automatic token renewal
- ✅ Configurable batch limits
- ✅ Enhanced error reporting with batch context

### Migration Required
- All users must migrate to v2.0 configuration
- New site credentials required from middleware admin
- Config file format updated

---

**Document Status**: Design Complete
**Next Step**: Review and approve design → Begin Phase 1 implementation
