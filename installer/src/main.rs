//! Data Exporter Installer
//!
//! This crate exists solely to provide metadata for cargo-wix.
//! The actual installer is built using WiX Toolset from the wix/main.wxs manifest.
//!
//! Build the MSI installer with:
//! ```powershell
//! cargo wix -p data-exporter-installer
//! ```

fn main() {
    println!("This binary is not meant to be run directly.");
    println!("Use cargo-wix to build the MSI installer:");
    println!("  cargo wix -p data-exporter-installer");
}
