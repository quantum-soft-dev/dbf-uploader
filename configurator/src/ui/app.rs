// Main Configurator Application Window
use crate::config_manager::ConfigManager;
use common::models::Config;
use native_windows_gui as nwg;
use nwg::NativeUi;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Section {
    Auth,
    Settings,
    Schedule,
    Service,
    Status,
}

impl Default for Section {
    fn default() -> Self {
        Section::Auth
    }
}

pub struct ConfiguratorApp {
    window: nwg::Window,

    // Navigation panel
    nav_frame: nwg::Frame,
    nav_title: nwg::Label,
    nav_auth_button: nwg::Button,
    nav_settings_button: nwg::Button,
    nav_schedule_button: nwg::Button,
    nav_service_button: nwg::Button,
    nav_status_button: nwg::Button,

    // Content area
    content_frame: nwg::Frame,
    section_title: nwg::Label,

    // Section: Authentication
    auth_desc_label: nwg::Label,
    auth_status_label: nwg::Label,
    auth_button: nwg::Button,
    auth_code_label: nwg::Label,

    // Section: Settings
    settings_server_label: nwg::Label,
    settings_server_input: nwg::TextInput,
    settings_source_label: nwg::Label,
    settings_source_input: nwg::TextInput,
    settings_source_browse: nwg::Button,

    // Section: Schedule
    schedule_cron_label: nwg::Label,
    schedule_cron_input: nwg::TextInput,
    schedule_help_label: nwg::Label,

    // Section: Service
    service_status_label: nwg::Label,
    service_install_button: nwg::Button,
    service_start_button: nwg::Button,
    service_stop_button: nwg::Button,
    service_uninstall_button: nwg::Button,

    // Section: Status
    status_info_label: nwg::Label,
    status_refresh_button: nwg::Button,

    // Bottom buttons
    save_button: nwg::Button,
    cancel_button: nwg::Button,

    // State
    current_section: RefCell<Section>,
    config_path: RefCell<std::path::PathBuf>,
    config: RefCell<Config>,
}

impl Default for ConfiguratorApp {
    fn default() -> Self {
        Self {
            window: Default::default(),
            nav_frame: Default::default(),
            nav_title: Default::default(),
            nav_auth_button: Default::default(),
            nav_settings_button: Default::default(),
            nav_schedule_button: Default::default(),
            nav_service_button: Default::default(),
            nav_status_button: Default::default(),
            content_frame: Default::default(),
            section_title: Default::default(),
            auth_desc_label: Default::default(),
            auth_status_label: Default::default(),
            auth_button: Default::default(),
            auth_code_label: Default::default(),
            settings_server_label: Default::default(),
            settings_server_input: Default::default(),
            settings_source_label: Default::default(),
            settings_source_input: Default::default(),
            settings_source_browse: Default::default(),
            schedule_cron_label: Default::default(),
            schedule_cron_input: Default::default(),
            schedule_help_label: Default::default(),
            service_status_label: Default::default(),
            service_install_button: Default::default(),
            service_start_button: Default::default(),
            service_stop_button: Default::default(),
            service_uninstall_button: Default::default(),
            status_info_label: Default::default(),
            status_refresh_button: Default::default(),
            save_button: Default::default(),
            cancel_button: Default::default(),
            current_section: RefCell::new(Section::Auth),
            config_path: RefCell::new(std::path::PathBuf::new()),
            config: RefCell::new(ConfigManager::default_config()),
        }
    }
}

impl ConfiguratorApp {
    fn init(&self) {
        let config_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        let config_path = config_dir.join("config.toml");
        *self.config_path.borrow_mut() = config_path.clone();

        // Load configuration
        let config_manager = ConfigManager::new(&config_path);
        let config = config_manager.load().unwrap_or_else(|_| ConfigManager::default_config());

        // Update UI with loaded config
        self.load_config_to_ui(&config);
        *self.config.borrow_mut() = config;

        self.show_section(Section::Auth);
    }

    fn load_config_to_ui(&self, config: &Config) {
        // Update UI fields from config
        self.settings_server_input.set_text(&config.api.base_url);
        self.settings_source_input.set_text(&config.src.source_dir.to_string_lossy());
        self.schedule_cron_input.set_text(&config.scheduler.crontab);

        // Update auth status
        if config.credential.is_device_flow() {
            self.auth_status_label.set_text("Status: Authenticated (Device Flow)");
        } else {
            self.auth_status_label.set_text("Status: Not authenticated");
        }
    }

    fn save_config_from_ui(&self) -> Config {
        let mut config = self.config.borrow().clone();

        // Update config from UI fields
        config.api.base_url = self.settings_server_input.text();
        config.src.source_dir = std::path::PathBuf::from(self.settings_source_input.text());
        config.scheduler.crontab = self.schedule_cron_input.text();

        config
    }

    fn show_section(&self, section: Section) {
        *self.current_section.borrow_mut() = section;

        // Update section title
        let title = match section {
            Section::Auth => "Authentication",
            Section::Settings => "Settings",
            Section::Schedule => "Schedule",
            Section::Service => "Service Management",
            Section::Status => "Status",
        };
        self.section_title.set_text(title);

        // Hide all sections
        self.auth_desc_label.set_visible(false);
        self.auth_status_label.set_visible(false);
        self.auth_button.set_visible(false);
        self.auth_code_label.set_visible(false);

        self.settings_server_label.set_visible(false);
        self.settings_server_input.set_visible(false);
        self.settings_source_label.set_visible(false);
        self.settings_source_input.set_visible(false);
        self.settings_source_browse.set_visible(false);

        self.schedule_cron_label.set_visible(false);
        self.schedule_cron_input.set_visible(false);
        self.schedule_help_label.set_visible(false);

        self.service_status_label.set_visible(false);
        self.service_install_button.set_visible(false);
        self.service_start_button.set_visible(false);
        self.service_stop_button.set_visible(false);
        self.service_uninstall_button.set_visible(false);

        self.status_info_label.set_visible(false);
        self.status_refresh_button.set_visible(false);

        // Show current section
        match section {
            Section::Auth => {
                self.auth_desc_label.set_visible(true);
                self.auth_status_label.set_visible(true);
                self.auth_button.set_visible(true);
                self.auth_code_label.set_visible(true);
            }
            Section::Settings => {
                self.settings_server_label.set_visible(true);
                self.settings_server_input.set_visible(true);
                self.settings_source_label.set_visible(true);
                self.settings_source_input.set_visible(true);
                self.settings_source_browse.set_visible(true);
            }
            Section::Schedule => {
                self.schedule_cron_label.set_visible(true);
                self.schedule_cron_input.set_visible(true);
                self.schedule_help_label.set_visible(true);
            }
            Section::Service => {
                self.service_status_label.set_visible(true);
                self.service_install_button.set_visible(true);
                self.service_start_button.set_visible(true);
                self.service_stop_button.set_visible(true);
                self.service_uninstall_button.set_visible(true);
            }
            Section::Status => {
                self.status_info_label.set_visible(true);
                self.status_refresh_button.set_visible(true);
            }
        }
    }

    fn on_auth_button(&self) {
        nwg::modal_info_message(
            &self.window,
            "Authentication",
            "Device Authorization Flow will be implemented here",
        );
    }

    fn on_browse_source(&self) {
        let mut dialog = Default::default();
        if nwg::FileDialog::builder()
            .title("Select Source Directory")
            .action(nwg::FileDialogAction::OpenDirectory)
            .build(&mut dialog)
            .is_ok()
        {
            if dialog.run(Some(&self.window)) {
                if let Ok(directory) = dialog.get_selected_item() {
                    self.settings_source_input
                        .set_text(&directory.to_string_lossy());
                }
            }
        }
    }

    fn on_service_install(&self) {
        nwg::modal_info_message(&self.window, "Service", "Install service");
    }

    fn on_service_start(&self) {
        nwg::modal_info_message(&self.window, "Service", "Start service");
    }

    fn on_service_stop(&self) {
        nwg::modal_info_message(&self.window, "Service", "Stop service");
    }

    fn on_service_uninstall(&self) {
        nwg::modal_info_message(&self.window, "Service", "Uninstall service");
    }

    fn on_status_refresh(&self) {
        nwg::modal_info_message(&self.window, "Status", "Refresh status");
    }

    fn on_save(&self) {
        // Save config from UI
        let config = self.save_config_from_ui();
        let config_path = self.config_path.borrow().clone();
        let config_manager = ConfigManager::new(&config_path);

        match config_manager.save(&config) {
            Ok(_) => {
                *self.config.borrow_mut() = config;
                nwg::modal_info_message(
                    &self.window,
                    "Success",
                    &format!("Configuration saved to:\n{}", config_path.display()),
                );
            }
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to save configuration:\n{}", e),
                );
            }
        }
    }

    fn on_cancel(&self) {
        nwg::stop_thread_dispatch();
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

pub struct ConfiguratorUi {
    inner: Rc<ConfiguratorApp>,
    default_handler: RefCell<Option<nwg::EventHandler>>,
}

impl NativeUi<ConfiguratorUi> for ConfiguratorApp {
    fn build_ui(mut data: ConfiguratorApp) -> Result<ConfiguratorUi, nwg::NwgError> {
        // Main window
        nwg::Window::builder()
            .size((800, 550))
            .position((250, 150))
            .title("Data Exporter Configurator")
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
            .build(&mut data.window)?;

        // === Navigation panel (left side) ===
        nwg::Frame::builder()
            .position((0, 0))
            .size((180, 550))
            .parent(&data.window)
            .build(&mut data.nav_frame)?;

        nwg::Label::builder()
            .text("Data Exporter")
            .position((15, 15))
            .size((150, 30))
            .parent(&data.nav_frame)
            .build(&mut data.nav_title)?;

        nwg::Button::builder()
            .text("Authentication")
            .position((15, 60))
            .size((150, 45))
            .parent(&data.nav_frame)
            .build(&mut data.nav_auth_button)?;

        nwg::Button::builder()
            .text("Settings")
            .position((15, 115))
            .size((150, 45))
            .parent(&data.nav_frame)
            .build(&mut data.nav_settings_button)?;

        nwg::Button::builder()
            .text("Schedule")
            .position((15, 170))
            .size((150, 45))
            .parent(&data.nav_frame)
            .build(&mut data.nav_schedule_button)?;

        nwg::Button::builder()
            .text("Service")
            .position((15, 225))
            .size((150, 45))
            .parent(&data.nav_frame)
            .build(&mut data.nav_service_button)?;

        nwg::Button::builder()
            .text("Status")
            .position((15, 280))
            .size((150, 45))
            .parent(&data.nav_frame)
            .build(&mut data.nav_status_button)?;

        // === Content area (right side) ===
        nwg::Frame::builder()
            .position((180, 0))
            .size((620, 500))
            .parent(&data.window)
            .build(&mut data.content_frame)?;

        nwg::Label::builder()
            .text("Authentication")
            .position((200, 20))
            .size((580, 35))
            .parent(&data.window)
            .build(&mut data.section_title)?;

        // === Section: Authentication ===
        nwg::Label::builder()
            .text("Configure authentication using Device Authorization Flow")
            .position((200, 70))
            .size((580, 25))
            .parent(&data.window)
            .build(&mut data.auth_desc_label)?;

        nwg::Label::builder()
            .text("Status: Not authenticated")
            .position((200, 110))
            .size((580, 25))
            .parent(&data.window)
            .build(&mut data.auth_status_label)?;

        nwg::Button::builder()
            .text("Start Device Authorization")
            .position((200, 150))
            .size((220, 40))
            .parent(&data.window)
            .build(&mut data.auth_button)?;

        nwg::Label::builder()
            .text("")
            .position((200, 210))
            .size((580, 100))
            .parent(&data.window)
            .build(&mut data.auth_code_label)?;

        // === Section: Settings ===
        nwg::Label::builder()
            .text("Server URL:")
            .position((200, 75))
            .size((150, 25))
            .parent(&data.window)
            .build(&mut data.settings_server_label)?;

        nwg::TextInput::builder()
            .text("https://")
            .position((360, 75))
            .size((410, 28))
            .parent(&data.window)
            .build(&mut data.settings_server_input)?;

        nwg::Label::builder()
            .text("Source Directory:")
            .position((200, 120))
            .size((150, 25))
            .parent(&data.window)
            .build(&mut data.settings_source_label)?;

        nwg::TextInput::builder()
            .position((360, 120))
            .size((320, 28))
            .parent(&data.window)
            .build(&mut data.settings_source_input)?;

        nwg::Button::builder()
            .text("Browse...")
            .position((690, 120))
            .size((80, 28))
            .parent(&data.window)
            .build(&mut data.settings_source_browse)?;

        // === Section: Schedule ===
        nwg::Label::builder()
            .text("Cron Expression:")
            .position((200, 75))
            .size((150, 25))
            .parent(&data.window)
            .build(&mut data.schedule_cron_label)?;

        nwg::TextInput::builder()
            .text("0 0 8,12,16,18 * * *")
            .position((360, 75))
            .size((410, 28))
            .parent(&data.window)
            .build(&mut data.schedule_cron_input)?;

        nwg::Label::builder()
            .text("Examples:\n\n  0 0 8,12,16,18 * * *    Run at 8am, 12pm, 4pm, 6pm daily\n\n  0 0 */4 * * *           Run every 4 hours\n\n  0 30 9 * * *            Run at 9:30am daily")
            .position((200, 120))
            .size((580, 200))
            .parent(&data.window)
            .build(&mut data.schedule_help_label)?;

        // === Section: Service ===
        nwg::Label::builder()
            .text("Service Status: Unknown")
            .position((200, 75))
            .size((580, 25))
            .parent(&data.window)
            .build(&mut data.service_status_label)?;

        nwg::Button::builder()
            .text("Install Service")
            .position((200, 120))
            .size((150, 40))
            .parent(&data.window)
            .build(&mut data.service_install_button)?;

        nwg::Button::builder()
            .text("Start Service")
            .position((365, 120))
            .size((150, 40))
            .parent(&data.window)
            .build(&mut data.service_start_button)?;

        nwg::Button::builder()
            .text("Stop Service")
            .position((530, 120))
            .size((150, 40))
            .parent(&data.window)
            .build(&mut data.service_stop_button)?;

        nwg::Button::builder()
            .text("Uninstall Service")
            .position((200, 175))
            .size((150, 40))
            .parent(&data.window)
            .build(&mut data.service_uninstall_button)?;

        // === Section: Status ===
        nwg::Label::builder()
            .text("Service information will appear here")
            .position((200, 75))
            .size((580, 250))
            .parent(&data.window)
            .build(&mut data.status_info_label)?;

        nwg::Button::builder()
            .text("Refresh")
            .position((200, 340))
            .size((120, 40))
            .parent(&data.window)
            .build(&mut data.status_refresh_button)?;

        // === Bottom buttons ===
        nwg::Button::builder()
            .text("Save")
            .position((590, 505))
            .size((100, 40))
            .parent(&data.window)
            .build(&mut data.save_button)?;

        nwg::Button::builder()
            .text("Cancel")
            .position((700, 505))
            .size((100, 40))
            .parent(&data.window)
            .build(&mut data.cancel_button)?;

        // Wrap-up
        let ui = ConfiguratorUi {
            inner: Rc::new(data),
            default_handler: Default::default(),
        };

        // Event handlers
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
                        // Navigation
                        if &handle == &ui.nav_auth_button {
                            ui.show_section(Section::Auth);
                        } else if &handle == &ui.nav_settings_button {
                            ui.show_section(Section::Settings);
                        } else if &handle == &ui.nav_schedule_button {
                            ui.show_section(Section::Schedule);
                        } else if &handle == &ui.nav_service_button {
                            ui.show_section(Section::Service);
                        } else if &handle == &ui.nav_status_button {
                            ui.show_section(Section::Status);
                        }
                        // Actions
                        else if &handle == &ui.auth_button {
                            ui.on_auth_button();
                        } else if &handle == &ui.settings_source_browse {
                            ui.on_browse_source();
                        } else if &handle == &ui.service_install_button {
                            ui.on_service_install();
                        } else if &handle == &ui.service_start_button {
                            ui.on_service_start();
                        } else if &handle == &ui.service_stop_button {
                            ui.on_service_stop();
                        } else if &handle == &ui.service_uninstall_button {
                            ui.on_service_uninstall();
                        } else if &handle == &ui.status_refresh_button {
                            ui.on_status_refresh();
                        } else if &handle == &ui.save_button {
                            ui.on_save();
                        } else if &handle == &ui.cancel_button {
                            ui.on_cancel();
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

impl Drop for ConfiguratorUi {
    fn drop(&mut self) {
        let handler = self.default_handler.borrow();
        if let Some(h) = handler.as_ref() {
            nwg::unbind_event_handler(h);
        }
    }
}

impl Deref for ConfiguratorUi {
    type Target = ConfiguratorApp;

    fn deref(&self) -> &ConfiguratorApp {
        &self.inner
    }
}
