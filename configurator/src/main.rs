// Data Exporter Configurator
// GUI application for configuring Data Exporter service

// #![windows_subsystem = "windows"]

mod ui;

use native_windows_gui as nwg;
use nwg::NativeUi;

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    // Set font
    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .family("Segoe UI")
        .size(17)
        .build(&mut font)
        .expect("Failed to create font");
    nwg::Font::set_global_default(Some(font));

    let _ui = ui::ConfiguratorApp::build_ui(Default::default())
        .expect("Failed to build UI");

    nwg::dispatch_thread_events();
}
