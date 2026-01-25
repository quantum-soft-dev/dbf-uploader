# Feature Specification: Configuration Module

**Feature Branch**: `002-configuration-module`
**Created**: 2026-01-24
**Status**: Draft
**Input**: PRD for Configuration Module - Type-safe configuration management system for Windows service dbf-uploader with TOML files, hot-reload, and dual authentication support

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Load Configuration at Service Startup (Priority: P1)

As a system administrator, I want the Windows service to load its configuration from a TOML file at startup, so that I can deploy and configure the service without recompiling.

**Why this priority**: This is the foundational capability - without configuration loading, the service cannot function. Every other feature depends on this.

**Independent Test**: Can be fully tested by creating a valid TOML configuration file, starting the service, and verifying it applies the correct settings (schedule, source directory, API endpoint, credentials).

**Acceptance Scenarios**:

1. **Given** a valid `config.toml` file exists at the expected location, **When** the service starts, **Then** the configuration is loaded and applied successfully
2. **Given** the configuration file has syntax errors, **When** the service starts, **Then** the service fails with a clear, actionable error message indicating the syntax problem
3. **Given** all 5 required configuration sections (scheduler, src, credential, api, encoding) are present, **When** the configuration is loaded, **Then** all values are accessible through the Config structure
4. **Given** optional fields are omitted, **When** the configuration is loaded, **Then** default values are applied (https_only=true, dbf_encoding="CP866")

---

### User Story 2 - Validate Configuration Before Use (Priority: P1)

As a system administrator, I want the service to validate all configuration values at startup, so that I am immediately notified of any misconfigurations before the service attempts to process data.

**Why this priority**: Critical for preventing runtime failures and data loss. Invalid configuration should fail fast with actionable feedback.

**Independent Test**: Can be fully tested by providing various invalid configurations and verifying appropriate error messages are returned for each violation.

**Acceptance Scenarios**:

1. **Given** the crontab expression is empty, **When** validation runs, **Then** an error "Crontab expression cannot be empty" is returned
2. **Given** the source_dir points to a non-existent directory, **When** validation runs, **Then** an error "Source directory does not exist: {path}" is returned
3. **Given** https_only=true and base_url starts with http://, **When** validation runs, **Then** an error about HTTPS requirement is returned
4. **Given** http:// URL with https_only=false, **When** validation runs, **Then** validation passes
5. **Given** traditional auth is used (no device flow) and account/username/password are empty, **When** validation runs, **Then** specific errors indicating which field is empty are returned
6. **Given** an invalid glob pattern in include_patterns or exclude_patterns, **When** validation runs, **Then** an error specifying which pattern is invalid is returned

---

### User Story 3 - Hot-Reload Configuration Changes (Priority: P2)

As a system administrator, I want to modify the service configuration while it is running and have changes applied without restarting the service, so that I can minimize downtime during configuration updates.

**Why this priority**: High operational value for production environments where service restarts are costly, but the service can function without this capability.

**Independent Test**: Can be fully tested by modifying the config file while service is running and observing configuration changes take effect at the next batch cycle.

**Acceptance Scenarios**:

1. **Given** the service is running and monitoring the config file, **When** the config file is modified, **Then** the change is detected within 2 seconds (debounce period)
2. **Given** multiple rapid file changes occur within 2 seconds, **When** debounce period elapses, **Then** only one reload is triggered
3. **Given** a valid config change is detected, **When** the next batch cycle begins, **Then** the new configuration is applied
4. **Given** an invalid config change is detected, **When** reload is attempted, **Then** the previous valid configuration continues to be used and an error is logged

---

### User Story 4 - Authenticate with Traditional Credentials (Priority: P1)

As a system administrator, I want to configure username/password credentials for API authentication, so that I can connect to the API using the traditional authentication method.

**Why this priority**: Essential for service operation - without authentication, the service cannot upload data.

**Independent Test**: Can be fully tested by configuring credentials in the TOML file and verifying the service can authenticate with the API.

**Acceptance Scenarios**:

1. **Given** account, username, and password are all provided, **When** credentials are composed, **Then** the username format is "{account}_{username}"
2. **Given** device flow is not configured, **When** any of account/username/password is empty, **Then** validation fails with a specific error for the empty field
3. **Given** credentials are configured correctly, **When** authentication is attempted, **Then** the credentials are used to obtain API access

---

### User Story 5 - Authenticate with Device Flow (Priority: P2)

As a system administrator of a headless Windows Server, I want to use OAuth 2.0 Device Authorization Flow for authentication, so that I can securely authenticate without entering credentials directly on the server.

**Why this priority**: Alternative authentication method for enhanced security scenarios. Service can function with traditional auth if unavailable.

**Independent Test**: Can be fully tested by configuring device flow credentials and verifying the device authorization flow is initiated correctly.

**Acceptance Scenarios**:

1. **Given** the [credential.device] section is configured with domain and client_secret, **When** authentication is initiated, **Then** Device Flow is used instead of traditional credentials
2. **Given** device flow is configured, **When** traditional credentials are also present, **Then** device flow takes precedence (traditional credentials are ignored)
3. **Given** device domain is empty, **When** validation runs, **Then** error "Device domain cannot be empty" is returned
4. **Given** device client_secret is empty, **When** validation runs, **Then** error "Device client_secret cannot be empty" is returned

---

### User Story 6 - Filter DBF Files with Glob Patterns (Priority: P2)

As a system administrator, I want to configure include and exclude patterns using glob syntax, so that I can precisely control which DBF files are processed.

**Why this priority**: Important for operational flexibility but service can function by processing all DBF files if not configured.

**Independent Test**: Can be fully tested by configuring patterns and verifying only matching files are processed.

**Acceptance Scenarios**:

1. **Given** include_patterns is configured with ["*.dbf"], **When** files are scanned, **Then** only files matching the pattern are included
2. **Given** both include and exclude patterns are configured, **When** files are scanned, **Then** exclude patterns are applied after include patterns (whitelist then blacklist)
3. **Given** patterns are configured, **When** matching is performed, **Then** matching is case-insensitive (*.DBF matches file.dbf)
4. **Given** an invalid glob pattern (e.g., "[invalid"), **When** validation runs, **Then** the specific invalid pattern is identified in the error message

---

### User Story 7 - Configure Service via GUI (Priority: P3)

As a Windows administrator, I want to use a graphical interface to configure the service, so that I don't need to manually edit TOML files.

**Why this priority**: Convenience feature for less technical administrators. Service is fully functional via TOML editing.

**Independent Test**: Can be fully tested by opening the configurator GUI, modifying settings, saving, and verifying the TOML file is correctly updated.

**Acceptance Scenarios**:

1. **Given** the GUI configurator is launched, **When** no config.toml exists, **Then** default values are loaded into the form
2. **Given** settings are modified in the GUI, **When** save is clicked, **Then** the config.toml is written with pretty-print formatting
3. **Given** the config directory doesn't exist, **When** save is clicked, **Then** the necessary directories are created automatically

---

### User Story 8 - Handle Network Drive Unavailability at Startup (Priority: P2)

As a system administrator, I want the service to wait for network drives to become available at system startup, so that the service can start automatically even before network shares are mounted.

**Why this priority**: Critical for reliable automated operation in enterprise environments with network storage.

**Independent Test**: Can be fully tested by starting service with unavailable source_dir and verifying retry behavior and eventual success.

**Acceptance Scenarios**:

1. **Given** source_dir doesn't exist at startup, **When** the service starts, **Then** retry attempts occur with exponential backoff (1, 2, 4, 8, 16 minutes)
2. **Given** 5 retry attempts have failed, **When** the 6th attempt begins, **Then** hourly retry intervals are used
3. **Given** the source_dir becomes available during retry, **When** the next retry occurs, **Then** the service starts successfully
4. **Given** a configuration error other than "Source directory does not exist", **When** startup fails, **Then** the service fails immediately without retry

---

### User Story 9 - Graceful Shutdown During Retry Wait (Priority: P2)

As a system administrator, I want the service to respond to stop commands promptly even while waiting for network drives, so that system restarts and service management are not blocked.

**Why this priority**: Essential for system manageability and clean shutdown behavior.

**Independent Test**: Can be fully tested by issuing a stop command while service is in retry wait and verifying prompt termination.

**Acceptance Scenarios**:

1. **Given** the service is in retry sleep waiting for source_dir, **When** a stop command is issued, **Then** the service terminates promptly (not waiting for sleep to complete)
2. **Given** the stop signal is received, **When** checked during any operation, **Then** the service exits cleanly with appropriate cleanup

---

### Edge Cases

- What happens when the config file is deleted while the service is running? Service continues with cached configuration.
- How does the system handle corrupted config files during hot-reload? Maintains previous valid configuration.
- What happens when credentials expire during operation? TokenManager handles refresh independently.
- How does the system handle mutex poisoning? Logs the error and continues with recovery.
- What happens when the config file is locked by another process? Retry read or use cached version.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST load configuration from a TOML file at a standard location
- **FR-002**: System MUST parse all 5 configuration sections: scheduler, src, credential, api, encoding
- **FR-003**: System MUST validate the crontab expression is not empty
- **FR-004**: System MUST validate the source directory exists on the filesystem
- **FR-005**: System MUST validate the API base URL starts with http:// or https://
- **FR-006**: System MUST enforce HTTPS-only when https_only=true is configured
- **FR-007**: System MUST validate traditional credentials (account, username, password) are non-empty when device flow is not configured
- **FR-008**: System MUST validate device flow credentials (domain, client_secret) when device flow is configured
- **FR-009**: System MUST validate all glob patterns are syntactically correct
- **FR-010**: System MUST apply default values for optional fields (https_only=true, dbf_encoding="CP866")
- **FR-011**: System MUST return specific, actionable error messages for all validation failures
- **FR-012**: System MUST monitor the configuration file for changes during service operation
- **FR-013**: System MUST debounce file change events for 2 seconds before reloading
- **FR-014**: System MUST apply configuration changes at the start of the next batch cycle
- **FR-015**: System MUST preserve the current configuration if hot-reload validation fails
- **FR-016**: System MUST compose traditional authentication username as "{account}_{username}"
- **FR-017**: System MUST prioritize device flow credentials over traditional credentials when both are present
- **FR-018**: System MUST apply include patterns as a whitelist filter
- **FR-019**: System MUST apply exclude patterns after include patterns as a blacklist
- **FR-020**: System MUST perform case-insensitive glob pattern matching
- **FR-021**: System MUST retry configuration loading with exponential backoff when source directory is unavailable
- **FR-022**: System MUST transition to hourly retry intervals after 5 failed attempts
- **FR-023**: System MUST respond to stop signals during retry wait periods
- **FR-024**: System MUST fail immediately for configuration errors other than missing source directory
- **FR-025**: GUI configurator MUST load defaults when no configuration file exists
- **FR-026**: GUI configurator MUST create parent directories when saving configuration
- **FR-027**: GUI configurator MUST format saved TOML with pretty-print formatting

### Key Entities

- **Config**: Root configuration container holding all service settings; contains 5 sub-configurations
- **SchedulerConfig**: Scheduling settings; contains crontab expression for job timing
- **SourceConfig**: Data source settings; contains source directory path and optional include/exclude glob patterns
- **CredentialConfig**: Authentication settings; contains traditional credentials (account, username, password) and optional device flow credentials
- **DeviceCredentials**: Device flow authentication settings; contains site_id, domain, and client_secret for RFC 8628 compliance
- **ApiConfig**: API connection settings; contains base_url and https_only flag
- **EncodingConfig**: Data encoding settings; contains dbf_encoding for character set specification

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Configuration loads and validates within 100ms under normal conditions
- **SC-002**: Hot-reload detects and applies changes within 5 seconds of file modification (including 2-second debounce)
- **SC-003**: Service starts successfully within 30 seconds when source directory is immediately available
- **SC-004**: 100% of credential values (passwords, client_secret) are never written to log files
- **SC-005**: Service responds to stop commands within 5 seconds during any retry wait period
- **SC-006**: All validation errors provide specific field names and actionable correction guidance
- **SC-007**: Configuration module test coverage exceeds 80%
- **SC-008**: GUI configurator successfully loads, edits, and saves configuration without data loss
- **SC-009**: System handles concurrent configuration access safely (no race conditions or data corruption)
- **SC-010**: Configuration changes via GUI are immediately usable by the service after save

## Assumptions

- The service runs with administrative privileges on Windows 10+ or Windows Server 2016+
- Network shares, if used for source_dir, follow standard Windows UNC or mapped drive conventions
- The configuration file location follows Windows Program Files conventions
- Character encodings are primarily CP866 (Cyrillic) with support for Windows-1255 (Hebrew) and other common encodings
- The 6-field cron format (seconds minutes hours day-of-month month day-of-week) is used for scheduling
- OAuth 2.0 Device Flow follows RFC 8628 with a 15-minute device code TTL
