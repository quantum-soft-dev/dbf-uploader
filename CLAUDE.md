# dbf-uploader Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-10-05

## Active Technologies
- Rust (latest stable, targeting Windows 10+ / Windows Server 2016+) + windows-service crate for service management, cron parser for scheduling, reqwest for HTTP/HTTPS client, serde for TOML config, DBF parsing library, CSV writer, gzip compression, JWT validation library (001-technical-specifications-data)
- Rust 1.82.0 (stable, as per `rust-version` in Cargo.toml) (001-data-exporter-service)
- File-based (DBF source, temp CSV, GZIP output) (001-data-exporter-service)
- File-based (TOML configuration at `C:\Program Files\data-exporter\config.toml`) (002-configuration-module)

## Project Structure
```
common/          # Shared library (auth, models, error handling, paths)
service/         # Windows service implementation
configurator/    # Native Windows GUI for configuration
installer/       # WiX MSI installer
tests/           # Integration tests
```

## Commands
```bash
cargo test       # Run all tests
cargo clippy     # Lint code
cargo fmt        # Format code
cargo build --release  # Build release binaries
```

## Code Style
Rust (latest stable, targeting Windows 10+ / Windows Server 2016+): Follow standard conventions

## Recent Changes
- 002-configuration-module: Added Rust 1.82.0 (stable, as per `rust-version` in Cargo.toml)
- 001-data-exporter-service: Added Rust 1.82.0 (stable, as per `rust-version` in Cargo.toml)
- 001-technical-specifications-data: Added Rust (latest stable, targeting Windows 10+ / Windows Server 2016+) + windows-service crate for service management, cron parser for scheduling, reqwest for HTTP/HTTPS client, serde for TOML config, DBF parsing library, CSV writer, gzip compression, JWT validation library

<!-- MANUAL ADDITIONS START -->
## Integration Tests

Service integration tests are in `service/tests/`:
- `config_load_test.rs` - SC-001 performance benchmark (100ms load requirement)
- `config_reload_sc002_test.rs` - SC-002 hot-reload timing test (5s detection requirement)
- `graceful_shutdown_test.rs` - SC-005 stop signal responsiveness (5s response requirement)
- `vss_copy_test.rs` - VSS shadow copy tests (require admin privileges)

Run specific test:
```bash
cargo test -p data-exporter-service --test config_load_test -- --nocapture
```

## Configuration Hot-Reload

ConfigWatcher in `service/src/config/watcher.rs` monitors config.toml for changes:
- Uses notify-debouncer-mini with 2-second debounce window
- Changes detected via `has_changed()` method
- Reload triggered automatically before each batch run

## Configurator Tests

Configurator validation unit tests are in `configurator/src/validation.rs`:
- Cron expression validation (5-field and 6-field formats)
- HTTPS URL validation
- Glob pattern validation
- Source directory existence checks

Run configurator tests:
```bash
cargo test -p data-exporter-configurator --lib
```
<!-- MANUAL ADDITIONS END -->
