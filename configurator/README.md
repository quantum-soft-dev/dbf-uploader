# Data Exporter Configurator

GUI application for configuring and managing the Data Exporter Windows service.

## Features

### Authentication
- **Device Authorization Flow**: Secure authentication using OAuth 2.0 Device Authorization Grant (RFC 8628)
- **Improved UX**: Automatically opens browser for authorization, displays code with copy button
- Site name and description configuration
- Automatic credential storage in `config.toml`

### Settings
- **Server URL**: Configure the API endpoint (validated for HTTPS when https_only enabled)
- **Source Directory**: Select the directory containing DBF files to process (with existence warning)
- **Include/Exclude Patterns**: Filter DBF files with glob patterns (validated on save)
- Browse button for easy directory selection

### Schedule
- **Cron Expression**: Configure when the service runs
- **Real-time Validation**: Instant feedback on cron expression validity
- Built-in examples and help text
- Supports 6-field cron syntax (tokio-cron-scheduler format)

### Service Management
- **Install**: Register the Windows service
- **Start**: Start the service
- **Stop**: Stop the service
- **Uninstall**: Remove the service (with confirmation)
- **Smart Button States**: Buttons are enabled/disabled based on service status
- Real-time service status display with auto-refresh
- Detailed installation info (executable, config, logs paths)

### UI Features
- Application icon in title bar
- Consistent font styling across all inputs
- Error messages with visual highlighting

## Usage

### First Time Setup

1. **Run the Configurator**
   ```cmd
   configurator.exe
   ```

2. **Configure Authentication** (Authentication tab)
   - Enter a site name (e.g., "warehouse-01")
   - Optionally add a description
   - Click "Start Device Authorization"
   - Follow the instructions to authorize in your browser

3. **Configure Settings** (Settings tab)
   - Server URL is automatically set from device authorization
   - Select the source directory containing DBF files

4. **Configure Schedule** (Schedule tab)
   - Set the cron expression for when the service should run
   - Default: `0 0 8,12,16,18 * * *` (8am, 12pm, 4pm, 6pm daily)

5. **Save Configuration**
   - Click the "Save" button at the bottom
   - Configuration is saved to `config.toml` in the current directory

6. **Install and Start Service** (Service tab)
   - Click "Install Service" to register the Windows service
   - Click "Start Service" to start it

### Managing the Service

Use the **Service** tab to:
- Check current service status
- Start/stop the service
- Uninstall the service when needed

### Configuration File Location

The configurator saves configuration to `config.toml` in the current working directory.

For the Windows service to find the configuration, it should be located at:
```
C:\Program Files\data-exporter\config.toml
```

## Requirements

- Windows 10 or later / Windows Server 2016 or later
- Administrator privileges (for service management operations)
- Internet connection (for device authorization)

## Building

```bash
cargo build --release --bin configurator
```

The compiled binary will be at: `target\release\configurator.exe`

## Testing

Run the validation unit tests:

```bash
cargo test -p data-exporter-configurator --lib
```

The tests cover:
- Cron expression validation (5-field and 6-field formats)
- HTTPS URL validation
- Glob pattern validation
- Source directory existence checks

## Architecture

### Modules

- **config_manager.rs**: Handles loading and saving `config.toml`
- **service_manager.rs**: Windows service control using `sc.exe`
- **validation.rs**: Testable validation functions (cron, patterns, HTTPS URL, source directory)
- **ui/app.rs**: Main GUI application window with native-windows-gui
- **ui/mod.rs**: UI module exports

### Dependencies

- `native-windows-gui`: Native Windows GUI framework
- `tokio`: Async runtime for device flow
- `anyhow`: Error handling
- `toml`: Configuration serialization
- `globset`: Glob pattern validation
- `cron`: Cron expression parsing and validation
- `open`: Open URLs in default browser
- `common`: Shared models and utilities

## Troubleshooting

### Service Installation Fails

**Error**: "Service executable not found"
- Ensure `data_exporter_service.exe` is copied to `C:\Program Files\data-exporter\`

**Error**: "Configuration file not found"
- Save the configuration first using the Save button
- Verify `config.toml` exists in the current directory

### Service Won't Start

- Check that the source directory exists and is accessible
- Verify the configuration is valid (Schedule tab uses valid cron syntax)
- Check Windows Event Log for detailed error messages

### Device Authorization Fails

**Error**: "Failed to create client"
- Verify the Server URL is correct and starts with `https://`
- Check internet connection

**Error**: "Authorization timeout"
- The authorization code expires after 15 minutes
- Start the process again and complete authorization promptly

## License

See the main project LICENSE file.
