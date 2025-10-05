# GitHub Actions Workflows

This directory contains automated workflows for building and releasing the Data Exporter Service.

## Workflows

### 1. Build Workflow (`build.yml`)

**Triggers:**
- Push to `main`, `develop`, or feature branches matching pattern `**-technical-specifications-**`
- Pull requests to `main` or `develop`

**Platform:** Windows only (runs on `windows-latest`)

**Steps:**
1. **Code Checkout** - Checks out the repository
2. **Rust Setup** - Installs Rust 1.79.0 with clippy and rustfmt
3. **Caching** - Caches cargo registry, git, and target directory for faster builds
4. **Code Quality Checks:**
   - `cargo fmt --check` - Ensures code is properly formatted
   - `cargo clippy` - Lints code for common mistakes and improvements
5. **Build & Test:**
   - Debug build
   - Run all unit tests
   - Release build
6. **Artifact Upload:**
   - Uploads `data_exporter.exe` (30 days retention)
   - Uploads debug symbols `.pdb` (7 days retention)

**Artifacts:**
- `data_exporter-windows-x86_64` - Release binary
- `data_exporter-windows-x86_64-debug` - Debug symbols

### 2. Release Workflow (`release.yml`)

**Triggers:**
- Push of version tags (format: `v*.*.*`, e.g., `v1.0.0`)

**Platform:** Windows only (runs on `windows-latest`)

**Steps:**
1. **Code Checkout** - Checks out the tagged version
2. **Rust Setup** - Installs Rust 1.79.0
3. **Build & Test:**
   - Release build with optimizations
   - Run all tests in release mode
4. **Packaging:**
   - Creates ZIP archive: `data_exporter-v1.0.0-windows-x86_64.zip`
   - Generates SHA256 checksum file
5. **GitHub Release:**
   - Creates a GitHub release with the tag name
   - Attaches ZIP archive and checksum
   - Includes installation instructions in release notes

**Release Assets:**
- ZIP archive with executable
- SHA256 checksum file

## Creating a Release

To create a new release:

```bash
# Tag the commit
git tag v1.0.0

# Push the tag
git push origin v1.0.0
```

The release workflow will automatically:
1. Build the Windows binary
2. Run tests
3. Create a GitHub release
4. Upload the binary and checksums

## Local Testing

To verify workflows locally before pushing:

```powershell
# Run the same checks as CI
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --verbose
cargo test --verbose
cargo build --release --verbose
```

## Requirements

- Rust 1.79.0 or higher
- Windows 10 or Windows Server 2016+ (for builds)
- GitHub Actions enabled in repository

## Troubleshooting

**Build fails on fmt check:**
```powershell
cargo fmt --all
git add .
git commit -m "Format code"
```

**Build fails on clippy:**
```powershell
cargo clippy --fix --allow-dirty
git add .
git commit -m "Fix clippy warnings"
```

**Tests fail:**
```powershell
cargo test -- --nocapture
```

## Windows-Specific Notes

- The workflows are configured for Windows-only builds since this is a Windows Service
- Uses `windows-latest` runner (currently Windows Server 2022)
- Produces `.exe` binary and `.pdb` debug symbols
- Archive format is ZIP (native Windows format)

## Future Enhancements

Potential improvements to the CI/CD pipeline:

- [ ] Add code coverage reporting
- [ ] Add security scanning (cargo audit)
- [ ] Add performance benchmarks
- [ ] Add integration tests with mock API server
- [ ] Add automatic version bumping
- [ ] Add changelog generation
