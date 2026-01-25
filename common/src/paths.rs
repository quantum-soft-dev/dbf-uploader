//! Centralized path resolution for Data Exporter
//!
//! Reads the installation directory from the Windows registry.
//! Falls back to the default path if the registry key is not found.

use std::path::PathBuf;

#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

/// Registry key where the install path is stored
pub const REGISTRY_KEY: &str = r"SOFTWARE\DataExporter";

/// Registry value name for the install path
pub const REGISTRY_VALUE: &str = "InstallPath";

/// Default installation path (used when registry key is not found)
pub const DEFAULT_PATH: &str = r"C:\Program Files\data-exporter";

/// Get the installation directory from registry, or return the default path
///
/// Reads from `HKEY_LOCAL_MACHINE\SOFTWARE\DataExporter\InstallPath`.
/// If the registry key or value doesn't exist, returns the default path.
#[cfg(windows)]
pub fn get_install_dir() -> PathBuf {
    if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(REGISTRY_KEY) {
        if let Ok(path) = hklm.get_value::<String, _>(REGISTRY_VALUE) {
            return PathBuf::from(path);
        }
    }
    PathBuf::from(DEFAULT_PATH)
}

/// Get the installation directory (non-Windows fallback)
#[cfg(not(windows))]
pub fn get_install_dir() -> PathBuf {
    PathBuf::from(DEFAULT_PATH)
}

/// Get the path to the configuration file
///
/// Returns `{install_dir}/config.toml`
pub fn get_config_path() -> PathBuf {
    get_install_dir().join("config.toml")
}

/// Get the path to the log directory
///
/// Returns `{install_dir}/logs`
pub fn get_log_dir() -> PathBuf {
    get_install_dir().join("logs")
}

/// Get the path to the service executable
///
/// Returns `{install_dir}/data_exporter_service.exe`
pub fn get_service_exe_path() -> PathBuf {
    get_install_dir().join("data_exporter_service.exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_path_is_valid() {
        let path = PathBuf::from(DEFAULT_PATH);
        assert!(path.is_absolute());
    }

    #[test]
    fn test_config_path_joins_correctly() {
        // This test checks the logic, not actual registry
        let install_dir = PathBuf::from(r"C:\Test\Dir");
        let config = install_dir.join("config.toml");
        assert_eq!(config, PathBuf::from(r"C:\Test\Dir\config.toml"));
    }

    #[test]
    fn test_log_dir_joins_correctly() {
        let install_dir = PathBuf::from(r"C:\Test\Dir");
        let logs = install_dir.join("logs");
        assert_eq!(logs, PathBuf::from(r"C:\Test\Dir\logs"));
    }

    #[test]
    fn test_get_service_exe_path_has_correct_filename() {
        let path = get_service_exe_path();
        let filename = path.file_name().unwrap().to_string_lossy();
        assert_eq!(filename, "data_exporter_service.exe");
    }
}
