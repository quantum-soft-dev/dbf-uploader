# Quickstart: Data Exporter Service

**Purpose**: Step-by-step guide to install, configure, and verify the Data Exporter Service
**Audience**: System administrators, QA testers, developers
**Estimated Time**: 15 minutes

## Prerequisites

- Windows 10 or higher / Windows Server 2016 or higher
- Administrator rights
- Internet connectivity
- Valid API credentials (username and password)
- Source directory with DBF files (for testing)

## Test Environment Setup

### 1. Prepare Test Data

Create a test directory structure with sample DBF files:

```cmd
mkdir "C:\test\dbf-data"
mkdir "C:\test\dbf-data\reports"
mkdir "C:\test\dbf-data\archive\2024"
```

Copy or create sample DBF files:
- `C:\test\dbf-data\sample.dbf`
- `C:\test\dbf-data\reports\monthly.dbf`
- `C:\test\dbf-data\archive\2024\sales.dbf`

**Note**: DBF files should contain actual data for meaningful testing. Files with different encodings (CP866, Windows-1251) are ideal for testing encoding detection.

### 2. Obtain API Credentials

Get valid credentials from your API administrator:
- Username
- Password
- API server URL (if different from default)

## Installation

### Step 1: Install the Service

Run the installation command with your credentials:

```cmd
data_exporter.exe install --username YOUR_USERNAME --password YOUR_PASSWORD --source-dir "C:\test\dbf-data" --crontab "*/5 * * * *" --api-url https://api.example.com
```

**Parameters**:
- `--username`: Your API username
- `--password`: Your API password
- `--source-dir`: Path to directory containing DBF files
- `--crontab`: Schedule (every 5 minutes for testing)
- `--api-url`: API server URL (optional, uses default if omitted)

**Expected Output**:
```
Installing Data Exporter Service...
Copying executable to C:\Program Files\data-exporter\
Registering Windows Service...
Creating configuration file...
Validating credentials with API server...
✓ Credentials validated successfully
✓ Service installed and started
Installation complete!
```

**If Installation Fails**:
- **Error: "Invalid credentials"**: Check username and password
- **Error: "Subscription inactive"**: Contact API administrator
- **Error: "Access denied"**: Run as Administrator
- **Error: "Network error"**: Check internet connectivity and API URL

### Step 2: Verify Service Installation

Check that the service is registered and running:

```cmd
sc query data-exporter
```

**Expected Output**:
```
SERVICE_NAME: data-exporter
        TYPE               : 10  WIN32_OWN_PROCESS
        STATE              : 4  RUNNING
        WIN32_EXIT_CODE    : 0  (0x0)
```

**Alternative** (PowerShell):
```powershell
Get-Service -Name "data-exporter"
```

### Step 3: Verify Configuration File

Check that configuration file was created:

```cmd
type "C:\Program Files\data-exporter\config.toml"
```

**Expected Content**:
```toml
[scheduler]
crontab = "*/5 * * * *"

[src]
source_dir = "C:\\test\\dbf-data"

[credential]
username = "YOUR_USERNAME"
password = "YOUR_PASSWORD"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP866"
```

**Verify**:
- ✓ Schedule is correct (every 5 minutes for testing)
- ✓ Source directory matches your test directory
- ✓ Credentials are present
- ✓ API URL is correct

## Testing

### Test 1: Wait for Scheduled Execution

The service runs based on the cron schedule (every 5 minutes with test config).

**Wait Time**: Up to 5 minutes for first execution

**Monitor Service Logs** (if logging is configured):
- Check for scheduled execution start
- Check for file discovery and processing
- Check for successful uploads

**Alternative Monitoring**: Check API server for uploaded files

### Test 2: Verify File Upload

After scheduled execution, verify files were uploaded to the server.

**Check API** (if you have access):
- Look for uploaded files:
  - `sample.csv.gz`
  - `reports_monthly.csv.gz`
  - `archive_2024_sales.csv.gz`

**Note filenames**: Path separators (`\`) converted to underscores (`_`)

### Test 3: Test Configuration Reload

Modify the configuration file while service is running:

```cmd
notepad "C:\Program Files\data-exporter\config.toml"
```

**Change**:
```toml
[scheduler]
crontab = "*/10 * * * *"  # Change to every 10 minutes
```

Save and close.

**Expected Behavior**:
- Service detects file change
- New schedule applies at the START of next scheduled run (not immediately)
- No service restart required

**Verification**: Next run should occur 10 minutes after the current run (not 5 minutes)

### Test 4: Test Locked File Handling

**Create a locked file**:

1. Open a DBF file in an application that locks it (e.g., Excel if it can open DBF)
2. Wait for scheduled execution
3. Service should defer locked file to end of batch
4. After processing other files, service retries locked file once
5. If still locked, file is skipped until next scheduled run

**Expected Log** (if available):
```
Processing batch...
  sample.dbf: Locked, deferring to end
  reports_monthly.dbf: Success
  archive_2024_sales.csv.gz: Success
Retrying locked files...
  sample.dbf: Still locked, skipping
Batch complete: 2 processed, 1 skipped
```

### Test 5: Test Error Handling

**Create an unreadable file**:

1. Create a file with `.dbf` extension but invalid content:
   ```cmd
   echo invalid > "C:\test\dbf-data\corrupt.dbf"
   ```

2. Wait for scheduled execution

3. Expected behavior:
   - Service attempts to process `corrupt.dbf`
   - Conversion fails (invalid DBF format)
   - Error report sent to API server
   - Processing continues with remaining files
   - Local CSV not created

**Check API** for error report:
```json
{
  "filename": "corrupt.dbf",
  "error_type": "ConversionError",
  "message": "Failed to read DBF file: Invalid header",
  "timestamp": "2025-10-05T...",
  "client_version": "1.0.0"
}
```

### Test 6: Test Disk Full Scenario (Optional)

**Note**: This test requires creating a small virtual disk or using a disk quota.

1. Fill disk until very little space remains
2. Wait for scheduled execution with large DBF files
3. Expected behavior:
   - Service attempts CSV conversion
   - Disk full error occurs during write
   - Error report sent to API
   - Processing continues with remaining files

### Test 7: Test Network Failure (Optional)

**Simulate network failure**:

1. Temporarily disable network connection or block API server
2. Wait for scheduled execution
3. Expected behavior:
   - Files processed (converted and compressed)
   - Upload fails
   - Error written to local `error.log` file
   - CSV files deleted regardless

**Check fallback log**:
```cmd
type "C:\Program Files\data-exporter\error.log"
```

**Expected Content**:
```
[2025-10-05T14:30:00Z] ERROR: Failed to upload file sample.csv.gz
  Error: Network unreachable
  Context: Batch abc-123, file sample.dbf
```

## Acceptance Criteria Validation

Based on feature spec acceptance scenarios:

### ✓ Scenario 1: Scheduled Execution
- [ ] Service runs at scheduled times (every 5/10 minutes in test config)
- [ ] All DBF files discovered recursively
- [ ] Files converted to CSV with UTF-8 encoding
- [ ] Files compressed to gzip format
- [ ] Files uploaded to server successfully

### ✓ Scenario 2: Encoding Detection
- [ ] DBF files with encoding in header: Encoding auto-detected
- [ ] DBF files without encoding: Fallback encoding (CP866) used
- [ ] CSV output is valid UTF-8

### ✓ Scenario 3: Configuration Reload
- [ ] Config file modified while service running
- [ ] Change detected by service
- [ ] Change applied at start of next scheduled run (not mid-batch)

### ✓ Scenario 4: JWT Token Renewal
- [ ] Token expiration detected before scheduled run
- [ ] New token requested automatically
- [ ] Processing continues with new token

### ✓ Scenario 5: Error Handling (Corrupted File)
- [ ] Corrupted DBF file encountered
- [ ] Error report sent to server
- [ ] Processing continues with remaining files

### ✓ Scenario 6: Server Unavailable
- [ ] Server unreachable during upload
- [ ] Error written to local error.log
- [ ] CSV files deleted
- [ ] Files re-processed on next run (idempotent)

### ✓ Scenario 7: CSV Cleanup
- [ ] CSV files deleted after successful upload
- [ ] CSV files also deleted after failed upload
- [ ] Source DBF files preserved (never deleted)

### ✓ Scenario 8: Filename Collision Handling
- [ ] Multiple DBF files with same name in different subdirectories
- [ ] Compressed files have unique names (path encoded)
- [ ] Example: `subdir1\data.dbf` → `subdir1_data.csv.gz`, `subdir2\data.dbf` → `subdir2_data.csv.gz`

## Troubleshooting

### Service Won't Start

**Check**:
```cmd
sc query data-exporter
```

**If STATE = STOPPED**:
```cmd
sc start data-exporter
```

**Check Windows Event Log** for service errors:
```cmd
eventvwr.msc
```
Navigate to: Windows Logs → Application

### Files Not Uploading

**Verify**:
1. Service is running: `sc query data-exporter`
2. Configuration is correct: `type "C:\Program Files\data-exporter\config.toml"`
3. JWT token is valid (check logs or API)
4. Network connectivity: `ping api.example.com`
5. Check local error.log: `type "C:\Program Files\data-exporter\error.log"`

### Configuration Changes Not Applied

**Remember**:
- Config changes apply at START of next scheduled run
- Not mid-batch, not immediately
- Wait for next scheduled execution time

### Locked Files Always Skipped

**Check**:
- Close applications that may be locking DBF files
- Verify file permissions
- Check if antivirus is scanning files

## Cleanup / Uninstall

### Uninstall the Service

```cmd
data_exporter.exe uninstall
```

**Expected Output**:
```
Uninstalling Data Exporter Service...
Stopping service...
Unregistering Windows Service...
Cleaning up installation directory...
✓ Service uninstalled successfully
```

**Verify**:
```cmd
sc query data-exporter
```

Should return: `The specified service does not exist as an installed service.`

### Remove Test Data (Optional)

```cmd
rmdir /s "C:\test\dbf-data"
```

## Next Steps

After successful quickstart:

1. **Production Setup**:
   - Use actual production DBF directory
   - Set appropriate cron schedule (e.g., `0 8,12,16,18 * * *`)
   - Configure monitoring and alerting

2. **Monitoring**:
   - Set up server-side monitoring for uploaded files
   - Monitor error reports on API server
   - Periodically check local error.log

3. **Maintenance**:
   - Periodically review and clean error.log
   - Monitor disk space usage
   - Update service when new versions available

## Support

**For issues**:
- Check error.log: `C:\Program Files\data-exporter\error.log`
- Check Windows Event Log (Application)
- Contact API administrator for server-side issues
- Consult service documentation

---

**Quickstart Complete**: You have successfully installed, configured, and tested the Data Exporter Service!
