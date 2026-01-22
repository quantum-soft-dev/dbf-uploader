// Installation logic module

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::shortcuts;

/// Installation configuration
pub struct InstallConfig {
    pub install_dir: PathBuf,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            install_dir: PathBuf::from(r"C:\Program Files\data-exporter"),
        }
    }
}

/// Copy embedded executables to installation directory
///
/// In production, executables would be embedded in the installer.
/// For now, we'll copy from the build directory (for testing).
fn copy_executables(install_dir: &Path) -> Result<()> {
    // Get the path to the current executable (installer.exe)
    let installer_path = std::env::current_exe()
        .context("Failed to get installer path")?;

    let build_dir = installer_path
        .parent()
        .context("Failed to get installer directory")?;

    // List of files to copy
    let files = [
        "data_exporter_service.exe",
        "configurator.exe",
        "uninstaller.exe",
    ];

    for file in &files {
        let source = build_dir.join(file);
        let dest = install_dir.join(file);

        if source.exists() {
            std::fs::copy(&source, &dest)
                .with_context(|| format!("Failed to copy {}", file))?;
        } else {
            anyhow::bail!("Required file not found: {}", file);
        }
    }

    Ok(())
}

/// Create example config.toml file
fn create_example_config(install_dir: &Path) -> Result<()> {
    let config_example = r#"# Data Exporter Configuration Template
#
# This is an example configuration file.
# Run the Configurator to set up your actual configuration.

[scheduler]
# Cron expression for scheduling (default: every 5 minutes)
crontab = "*/5 * * * *"

[src]
# Source directory containing DBF files
dir = "C:\\path\\to\\dbf\\files"

# Optional: File name patterns to include (glob patterns)
# include_patterns = ["*.DBF", "data_*.dbf"]

# Optional: File name patterns to exclude
# exclude_patterns = ["temp_*.dbf"]

[credential]
# Authentication credentials
# This will be configured by the Configurator

[api]
# API base URL (default for development)
base_url = "https://dev.dfm.bitbi.io"
https_only = true

[encoding]
# DBF file encoding (CP866, Windows1251, Windows1255, ISO8859-8, UTF8)
encoding = "CP866"
"#;

    let config_path = install_dir.join("config.toml.example");
    std::fs::write(&config_path, config_example)
        .context("Failed to create config.toml.example")?;

    Ok(())
}

/// Perform the installation
pub fn install(config: &InstallConfig) -> Result<()> {
    // 1. Create installation directory
    std::fs::create_dir_all(&config.install_dir)
        .context("Failed to create installation directory")?;

    // 2. Copy executables
    copy_executables(&config.install_dir)
        .context("Failed to copy executables")?;

    // 3. Create example config
    create_example_config(&config.install_dir)
        .context("Failed to create example config")?;

    // 4. Create Start Menu shortcuts
    shortcuts::create_start_menu_shortcuts(&config.install_dir)
        .context("Failed to create Start Menu shortcuts")?;

    Ok(())
}

/// Launch the configurator after installation
pub fn launch_configurator(install_dir: &Path) -> Result<()> {
    let configurator_exe = install_dir.join("configurator.exe");

    Command::new(&configurator_exe)
        .spawn()
        .context("Failed to launch configurator")?;

    Ok(())
}
