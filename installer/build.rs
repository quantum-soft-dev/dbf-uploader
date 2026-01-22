// Build script for Windows installer

fn main() {
    // Embed icon for Windows executable
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("res/icon.ico");
        res.set("ProductName", "Data Exporter");
        res.set("FileDescription", "Data Exporter Installer");
        res.set("CompanyName", "");
        res.compile().unwrap_or_default();
    }
}
