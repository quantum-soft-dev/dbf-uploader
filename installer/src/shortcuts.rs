// Module for creating Windows shortcuts (.lnk files)

use anyhow::{Context, Result};
use std::path::Path;
use windows::{
    core::{Interface, PCWSTR},
    Win32::System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_INPROC_SERVER, IPersistFile},
    Win32::UI::Shell::{IShellLinkW, ShellLink},
};

/// Create a Windows shortcut (.lnk file)
///
/// # Arguments
/// * `shortcut_path` - Full path where the .lnk file will be created
/// * `target_path` - Path to the executable the shortcut points to
/// * `description` - Description of the shortcut
/// * `working_dir` - Working directory for the shortcut (optional)
pub fn create_shortcut<P: AsRef<Path>>(
    shortcut_path: P,
    target_path: P,
    description: &str,
    working_dir: Option<P>,
) -> Result<()> {
    unsafe {
        // Initialize COM
        CoInitialize(None).ok().context("Failed to initialize COM")?;

        // Create IShellLink instance
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .context("Failed to create ShellLink instance")?;

        // Set target path
        let target_wide: Vec<u16> = target_path
            .as_ref()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        shell_link
            .SetPath(PCWSTR(target_wide.as_ptr()))
            .context("Failed to set target path")?;

        // Set description
        let desc_wide: Vec<u16> = description.encode_utf16().chain(Some(0)).collect();
        shell_link
            .SetDescription(PCWSTR(desc_wide.as_ptr()))
            .context("Failed to set description")?;

        // Set working directory if provided
        if let Some(work_dir) = working_dir {
            let work_dir_wide: Vec<u16> = work_dir
                .as_ref()
                .to_string_lossy()
                .encode_utf16()
                .chain(Some(0))
                .collect();
            shell_link
                .SetWorkingDirectory(PCWSTR(work_dir_wide.as_ptr()))
                .context("Failed to set working directory")?;
        }

        // Save the shortcut
        let persist_file: IPersistFile = shell_link
            .cast()
            .context("Failed to cast to IPersistFile")?;

        let shortcut_wide: Vec<u16> = shortcut_path
            .as_ref()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();

        persist_file
            .Save(PCWSTR(shortcut_wide.as_ptr()), true)
            .context("Failed to save shortcut")?;

        // Cleanup COM
        CoUninitialize();

        Ok(())
    }
}

/// Get the Start Menu Programs folder path for all users
pub fn get_start_menu_programs_path() -> Result<String> {
    use std::env;

    // Get ProgramData path (e.g., C:\ProgramData)
    let program_data = env::var("ProgramData")
        .or_else(|_| env::var("ALLUSERSPROFILE"))
        .context("Failed to get ProgramData path")?;

    Ok(format!(
        r"{}\Microsoft\Windows\Start Menu\Programs",
        program_data
    ))
}

/// Create Start Menu folder with shortcuts for Data Exporter
///
/// Creates:
/// - Start Menu\Programs\Data Exporter\Configurator.lnk
/// - Start Menu\Programs\Data Exporter\Uninstall Data Exporter.lnk
pub fn create_start_menu_shortcuts(install_dir: &Path) -> Result<()> {
    let start_menu = get_start_menu_programs_path()?;
    let app_folder = Path::new(&start_menu).join("Data Exporter");

    // Create Data Exporter folder in Start Menu
    std::fs::create_dir_all(&app_folder)
        .context("Failed to create Start Menu folder")?;

    // Create Configurator shortcut
    let configurator_exe = install_dir.join("configurator.exe");
    let configurator_lnk = app_folder.join("Configurator.lnk");
    let install_dir_buf = install_dir.to_path_buf();
    create_shortcut(
        &configurator_lnk,
        &configurator_exe,
        "Data Exporter Configurator - Configure service settings",
        Some(&install_dir_buf),
    )
    .context("Failed to create Configurator shortcut")?;

    // Create Uninstaller shortcut
    let uninstaller_exe = install_dir.join("uninstaller.exe");
    let uninstaller_lnk = app_folder.join("Uninstall Data Exporter.lnk");
    create_shortcut(
        &uninstaller_lnk,
        &uninstaller_exe,
        "Uninstall Data Exporter",
        Some(&install_dir_buf),
    )
    .context("Failed to create Uninstaller shortcut")?;

    Ok(())
}
