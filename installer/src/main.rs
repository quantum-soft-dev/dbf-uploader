// Data Exporter Installer
// GUI installer using native-windows-gui

#![windows_subsystem = "windows"]

mod installer_logic;
mod shortcuts;

use native_windows_gui as nwg;
use nwg::NativeUi;
use std::path::PathBuf;

use installer_logic::{install, launch_configurator, InstallConfig};

#[derive(Default)]
pub struct InstallerApp {
    window: nwg::Window,

    // Layout frames
    header_frame: nwg::Frame,
    content_frame: nwg::Frame,
    button_separator: nwg::Frame,

    // Header
    header_title: nwg::Label,
    header_subtitle: nwg::Label,

    // Welcome screen
    welcome_text: nwg::Label,

    // Directory selection screen
    dir_label: nwg::Label,
    dir_input: nwg::TextInput,
    browse_button: nwg::Button,

    // Progress screen
    progress_label: nwg::Label,
    progress_bar: nwg::ProgressBar,

    // Completion screen
    complete_text: nwg::Label,

    // Buttons
    cancel_button: nwg::Button,
    back_button: nwg::Button,
    next_button: nwg::Button,
    finish_button: nwg::Button,

    // State
    current_screen: std::cell::RefCell<Screen>,
    install_dir: std::cell::RefCell<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Screen {
    Welcome,
    Directory,
    Progress,
    Complete,
}

impl Default for Screen {
    fn default() -> Self {
        Screen::Welcome
    }
}

impl InstallerApp {
    fn init(&self) {
        // Set default installation directory
        let default_dir = InstallConfig::default().install_dir;
        *self.install_dir.borrow_mut() = default_dir.clone();
        self.dir_input.set_text(&default_dir.to_string_lossy());

        // Show welcome screen
        self.show_screen(Screen::Welcome);
    }

    fn show_screen(&self, screen: Screen) {
        *self.current_screen.borrow_mut() = screen;

        // Hide all content controls
        self.welcome_text.set_visible(false);
        self.dir_label.set_visible(false);
        self.dir_input.set_visible(false);
        self.browse_button.set_visible(false);
        self.progress_label.set_visible(false);
        self.progress_bar.set_visible(false);
        self.complete_text.set_visible(false);

        // Hide all buttons first
        self.back_button.set_visible(false);
        self.next_button.set_visible(false);
        self.finish_button.set_visible(false);

        // Show controls for current screen
        match screen {
            Screen::Welcome => {
                self.header_title.set_text("Welcome to Data Exporter Setup");
                self.header_subtitle.set_text("This wizard will install Data Exporter on your computer.");
                self.welcome_text.set_visible(true);
                self.cancel_button.set_visible(true);
                self.next_button.set_visible(true);
            }
            Screen::Directory => {
                self.header_title.set_text("Choose Install Location");
                self.header_subtitle.set_text("Select the folder where Data Exporter will be installed.");
                self.dir_label.set_visible(true);
                self.dir_input.set_visible(true);
                self.browse_button.set_visible(true);
                self.cancel_button.set_visible(true);
                self.back_button.set_visible(true);
                self.next_button.set_visible(true);
                self.next_button.set_text("Install");
            }
            Screen::Progress => {
                self.header_title.set_text("Installing");
                self.header_subtitle.set_text("Please wait while Data Exporter is being installed...");
                self.progress_label.set_visible(true);
                self.progress_bar.set_visible(true);
                self.progress_bar.set_pos(0);
                self.cancel_button.set_enabled(false);
            }
            Screen::Complete => {
                self.header_title.set_text("Completing Data Exporter Setup");
                self.header_subtitle.set_text("Setup has finished installing Data Exporter.");
                self.complete_text.set_visible(true);
                self.finish_button.set_visible(true);
            }
        }
    }

    fn on_browse(&self) {
        let mut dialog = Default::default();
        if nwg::FileDialog::builder()
            .title("Select Installation Directory")
            .action(nwg::FileDialogAction::OpenDirectory)
            .build(&mut dialog)
            .is_ok()
        {
            if dialog.run(Some(&self.window)) {
                if let Ok(directory) = dialog.get_selected_item() {
                    let dir_path = directory.to_string_lossy().to_string();
                    self.dir_input.set_text(&dir_path);
                    *self.install_dir.borrow_mut() = PathBuf::from(dir_path);
                }
            }
        }
    }

    fn on_back(&self) {
        let current = *self.current_screen.borrow();
        match current {
            Screen::Directory => self.show_screen(Screen::Welcome),
            _ => {}
        }
    }

    fn on_next(&self) {
        let current = *self.current_screen.borrow();

        match current {
            Screen::Welcome => {
                self.show_screen(Screen::Directory);
            }
            Screen::Directory => {
                // Update install dir from input
                let dir_text = self.dir_input.text();
                *self.install_dir.borrow_mut() = PathBuf::from(dir_text);

                // Start installation
                self.show_screen(Screen::Progress);
                self.perform_installation();
            }
            _ => {}
        }
    }

    fn perform_installation(&self) {
        self.progress_label.set_text("Installing...");

        let install_dir = self.install_dir.borrow().clone();
        let config = InstallConfig { install_dir };

        // Simulate progress steps
        self.progress_bar.set_pos(10);
        self.progress_label.set_text("Creating installation directory...");
        std::thread::sleep(std::time::Duration::from_millis(300));

        self.progress_bar.set_pos(30);
        self.progress_label.set_text("Copying files...");
        std::thread::sleep(std::time::Duration::from_millis(500));

        self.progress_bar.set_pos(60);
        self.progress_label.set_text("Creating shortcuts...");
        std::thread::sleep(std::time::Duration::from_millis(300));

        self.progress_bar.set_pos(90);
        self.progress_label.set_text("Finalizing installation...");
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Perform actual installation
        match install(&config) {
            Ok(_) => {
                self.progress_bar.set_pos(100);
                self.show_screen(Screen::Complete);
            }
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Installation Error",
                    &format!("Failed to install: {}", e),
                );
                nwg::stop_thread_dispatch();
            }
        }
    }

    fn on_finish(&self) {
        let install_dir = self.install_dir.borrow();

        // Launch configurator
        if let Err(e) = launch_configurator(&install_dir) {
            nwg::modal_error_message(
                &self.window,
                "Launch Error",
                &format!("Failed to launch configurator: {}", e),
            );
        }

        // Close installer
        nwg::stop_thread_dispatch();
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

mod installer_ui {
    use super::*;
    use native_windows_gui as nwg;
    use std::cell::RefCell;
    use std::ops::Deref;
    use std::rc::Rc;

    pub struct InstallerUi {
        inner: Rc<InstallerApp>,
        default_handler: RefCell<Option<nwg::EventHandler>>,
    }

    impl nwg::NativeUi<InstallerUi> for InstallerApp {
        fn build_ui(mut data: InstallerApp) -> Result<InstallerUi, nwg::NwgError> {
            // Window
            nwg::Window::builder()
                .size((600, 400))
                .position((300, 300))
                .title("Data Exporter Installer")
                .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
                .build(&mut data.window)?;

            // Welcome screen
            nwg::Label::builder()
                .text("Welcome to Data Exporter Installer\n\nThis wizard will guide you through the installation process.\n\nClick Next to continue.")
                .position((40, 60))
                .size((520, 200))
                .parent(&data.window)
                .build(&mut data.welcome_label)?;

            // Directory selection screen
            nwg::Label::builder()
                .text("Select Installation Directory:")
                .position((40, 60))
                .size((520, 25))
                .parent(&data.window)
                .build(&mut data.dir_label)?;

            nwg::TextInput::builder()
                .position((40, 90))
                .size((420, 25))
                .parent(&data.window)
                .build(&mut data.dir_input)?;

            nwg::Button::builder()
                .text("Browse...")
                .position((470, 90))
                .size((90, 25))
                .parent(&data.window)
                .build(&mut data.browse_button)?;

            // Progress screen
            nwg::Label::builder()
                .text("Installing...")
                .position((40, 120))
                .size((520, 25))
                .parent(&data.window)
                .build(&mut data.progress_label)?;

            nwg::ProgressBar::builder()
                .position((40, 150))
                .size((520, 25))
                .range(0..100)
                .parent(&data.window)
                .build(&mut data.progress_bar)?;

            // Completion screen
            nwg::Label::builder()
                .text("Installation Complete!\n\nData Exporter has been successfully installed.\n\nThe Configurator will now launch to complete the setup.")
                .position((40, 80))
                .size((520, 200))
                .parent(&data.window)
                .build(&mut data.complete_label)?;

            // Buttons
            nwg::Button::builder()
                .text("Next >")
                .position((460, 330))
                .size((100, 30))
                .parent(&data.window)
                .build(&mut data.next_button)?;

            nwg::Button::builder()
                .text("Finish")
                .position((460, 330))
                .size((100, 30))
                .parent(&data.window)
                .build(&mut data.finish_button)?;

            // Wrap-up
            let ui = InstallerUi {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };

            // Events
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data, handle| {
                if let Some(ui) = evt_ui.upgrade() {
                    match evt {
                        nwg::Event::OnWindowClose => {
                            if &handle == &ui.window {
                                ui.exit();
                            }
                        }
                        nwg::Event::OnButtonClick => {
                            if &handle == &ui.next_button {
                                ui.on_next();
                            } else if &handle == &ui.finish_button {
                                ui.on_finish();
                            } else if &handle == &ui.browse_button {
                                ui.on_browse();
                            }
                        }
                        _ => {}
                    }
                }
            };

            *ui.default_handler.borrow_mut() = Some(nwg::full_bind_event_handler(
                &ui.window.handle,
                handle_events,
            ));

            // Initialize
            ui.init();

            Ok(ui)
        }
    }

    impl Drop for InstallerUi {
        fn drop(&mut self) {
            let handler = self.default_handler.borrow();
            if let Some(h) = handler.as_ref() {
                nwg::unbind_event_handler(h);
            }
        }
    }

    impl Deref for InstallerUi {
        type Target = InstallerApp;

        fn deref(&self) -> &InstallerApp {
            &self.inner
        }
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let _ui = InstallerApp::build_ui(Default::default()).expect("Failed to build UI");

    nwg::dispatch_thread_events();
}
