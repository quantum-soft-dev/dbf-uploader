// Main Configurator Application Window
use crate::config_manager::ConfigManager;
use crate::service_manager::{ServiceManager, ServiceStatus};
use common::auth::device_flow::{DeviceFlowClient, SiteInfo};
use common::models::{Config, DeviceCredentials};
use cron::Schedule;
use native_windows_gui as nwg;
use nwg::NativeUi;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Section {
    #[default]
    Auth,
    Settings,
    Schedule,
    Service,
    Status,
}

pub struct ConfiguratorApp {
    window: nwg::Window,

    // Icon
    app_icon: nwg::Icon,

    // Fonts
    title_font: nwg::Font,
    heading_font: nwg::Font,
    normal_font: nwg::Font,
    error_font: nwg::Font,

    // Navigation panel
    nav_frame: nwg::Frame,
    nav_title: nwg::Label,
    nav_auth_button: nwg::Button,
    nav_settings_button: nwg::Button,
    nav_schedule_button: nwg::Button,
    nav_service_button: nwg::Button,
    nav_status_button: nwg::Button,

    // Content area
    section_title: nwg::Label,

    // Section: Authentication
    auth_desc_label: nwg::Label,
    auth_site_name_label: nwg::Label,
    auth_site_name_input: nwg::TextInput,
    auth_site_desc_label: nwg::Label,
    auth_site_desc_input: nwg::TextInput,
    auth_status_label: nwg::Label,
    auth_button: nwg::Button,
    auth_code_label: nwg::Label,
    auth_code_input: nwg::TextInput, // TextInput for easy copy
    auth_copy_button: nwg::Button,
    auth_url_label: nwg::Label,

    // Section: Settings
    settings_server_label: nwg::Label,
    settings_server_input: nwg::TextInput,
    settings_source_label: nwg::Label,
    settings_source_input: nwg::TextInput,
    settings_source_browse: nwg::Button,
    settings_include_label: nwg::Label,
    settings_include_input: nwg::TextInput,
    settings_exclude_label: nwg::Label,
    settings_exclude_input: nwg::TextInput,
    settings_pattern_help_label: nwg::Label,

    // Section: Schedule
    schedule_cron_label: nwg::Label,
    schedule_cron_input: nwg::TextInput,
    schedule_cron_error: nwg::RichLabel, // RichLabel for proper background color support
    schedule_help_label: nwg::Label,

    // Section: Service
    service_status_label: nwg::Label,
    service_refresh_button: nwg::Button,
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
            app_icon: Default::default(),
            title_font: Default::default(),
            heading_font: Default::default(),
            normal_font: Default::default(),
            error_font: Default::default(),
            nav_frame: Default::default(),
            nav_title: Default::default(),
            nav_auth_button: Default::default(),
            nav_settings_button: Default::default(),
            nav_schedule_button: Default::default(),
            nav_service_button: Default::default(),
            nav_status_button: Default::default(),
            section_title: Default::default(),
            auth_desc_label: Default::default(),
            auth_site_name_label: Default::default(),
            auth_site_name_input: Default::default(),
            auth_site_desc_label: Default::default(),
            auth_site_desc_input: Default::default(),
            auth_status_label: Default::default(),
            auth_button: Default::default(),
            auth_code_label: Default::default(),
            auth_code_input: Default::default(),
            auth_copy_button: Default::default(),
            auth_url_label: Default::default(),
            settings_server_label: Default::default(),
            settings_server_input: Default::default(),
            settings_source_label: Default::default(),
            settings_source_input: Default::default(),
            settings_source_browse: Default::default(),
            settings_include_label: Default::default(),
            settings_include_input: Default::default(),
            settings_exclude_label: Default::default(),
            settings_exclude_input: Default::default(),
            settings_pattern_help_label: Default::default(),
            schedule_cron_label: Default::default(),
            schedule_cron_input: Default::default(),
            schedule_cron_error: Default::default(),
            schedule_help_label: Default::default(),
            service_status_label: Default::default(),
            service_refresh_button: Default::default(),
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
        let config_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let config_path = config_dir.join("config.toml");
        *self.config_path.borrow_mut() = config_path.clone();

        // Load configuration
        let config_manager = ConfigManager::new(&config_path);
        let config = config_manager
            .load()
            .unwrap_or_else(|_| ConfigManager::default_config());

        // Update UI with loaded config
        self.load_config_to_ui(&config);
        *self.config.borrow_mut() = config;

        // Show Auth section first (don't block on service status check)
        self.show_section(Section::Auth);
    }

    fn load_config_to_ui(&self, config: &Config) {
        // Update UI fields from config
        self.settings_server_input.set_text(&config.api.base_url);
        self.settings_source_input
            .set_text(&config.src.source_dir.to_string_lossy());
        self.schedule_cron_input.set_text(&config.scheduler.crontab);

        // Update include/exclude patterns
        // Empty string in UI corresponds to None in config (no filtering)
        if let Some(ref patterns) = config.src.include_patterns {
            self.settings_include_input.set_text(&patterns.join(", "));
        } else {
            self.settings_include_input.set_text(""); // None = no include filter
        }
        if let Some(ref patterns) = config.src.exclude_patterns {
            self.settings_exclude_input.set_text(&patterns.join(", "));
        } else {
            self.settings_exclude_input.set_text(""); // None = no exclude filter
        }

        // Update auth status
        if config.credential.is_device_flow() {
            self.auth_status_label
                .set_text("Status: Authenticated (Device Flow)");
        } else {
            self.auth_status_label.set_text("Status: Not authenticated");
        }
    }

    /// Parse comma-separated pattern string into Option<Vec<String>>
    fn parse_patterns(input: &str) -> Option<Vec<String>> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(
                trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            )
        }
    }

    /// Validate cron expression statically, returns Ok(()) or Err(error message)
    fn validate_cron_static(cron_text: &str) -> Result<(), String> {
        let cron_text = cron_text.trim();

        if cron_text.is_empty() {
            return Err("Cron expression is required".to_string());
        }

        // Convert 5-field cron to 6-field for validation (add seconds)
        let cron_6field = if cron_text.split_whitespace().count() == 5 {
            format!("0 {}", cron_text)
        } else {
            cron_text.to_string()
        };

        Schedule::from_str(&cron_6field)
            .map(|_| ())
            .map_err(|e| format!("Invalid cron expression: {}", e))
    }

    /// Validate glob patterns, returns Ok(()) or Err(error message)
    fn validate_patterns(patterns: &Option<Vec<String>>, field_name: &str) -> Result<(), String> {
        if let Some(ref pattern_list) = patterns {
            for pattern in pattern_list {
                if let Err(e) = globset::GlobBuilder::new(pattern).build() {
                    return Err(format!(
                        "Invalid {} pattern '{}': {}",
                        field_name, pattern, e
                    ));
                }
            }
        }
        Ok(())
    }

    /// Validate cron expression and update error label
    fn validate_cron_expression(&self) {
        let cron_text = self.schedule_cron_input.text();
        let cron_text = cron_text.trim();

        if cron_text.is_empty() {
            self.show_cron_error("Cron expression is required");
            return;
        }

        // Convert 5-field cron to 6-field for validation (add seconds)
        let cron_6field = if cron_text.split_whitespace().count() == 5 {
            format!("0 {}", cron_text)
        } else {
            cron_text.to_string()
        };

        match Schedule::from_str(&cron_6field) {
            Ok(_) => {
                // Valid cron expression - hide error
                self.schedule_cron_error.set_visible(false);
            }
            Err(_) => {
                // Invalid cron expression - show error
                self.show_cron_error("Invalid cron expression");
            }
        }
    }

    /// Show error message in the cron error RichLabel with red background and white text
    fn show_cron_error(&self, message: &str) {
        self.schedule_cron_error.set_text(message);
        // Set bright red background
        self.schedule_cron_error.set_background_color([220, 53, 69]);
        // Set white text color for contrast
        let text_len = message.len() as u32;
        let fmt = nwg::CharFormat {
            text_color: Some([255, 255, 255]), // White text
            effects: Some(nwg::CharEffects::BOLD),
            ..Default::default()
        };
        self.schedule_cron_error.set_char_format(0..text_len, &fmt);
        self.schedule_cron_error.set_visible(true);
    }

    fn save_config_from_ui(&self) -> Config {
        let mut config = self.config.borrow().clone();

        // Update config from UI fields
        config.api.base_url = self.settings_server_input.text();
        config.src.source_dir = std::path::PathBuf::from(self.settings_source_input.text());
        config.scheduler.crontab = self.schedule_cron_input.text();

        // Update include/exclude patterns
        config.src.include_patterns = Self::parse_patterns(&self.settings_include_input.text());
        config.src.exclude_patterns = Self::parse_patterns(&self.settings_exclude_input.text());

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
        self.auth_site_name_label.set_visible(false);
        self.auth_site_name_input.set_visible(false);
        self.auth_site_desc_label.set_visible(false);
        self.auth_site_desc_input.set_visible(false);
        self.auth_status_label.set_visible(false);
        self.auth_button.set_visible(false);
        self.auth_code_label.set_visible(false);
        self.auth_code_input.set_visible(false);
        self.auth_copy_button.set_visible(false);
        self.auth_url_label.set_visible(false);

        self.settings_server_label.set_visible(false);
        self.settings_server_input.set_visible(false);
        self.settings_source_label.set_visible(false);
        self.settings_source_input.set_visible(false);
        self.settings_source_browse.set_visible(false);
        self.settings_include_label.set_visible(false);
        self.settings_include_input.set_visible(false);
        self.settings_exclude_label.set_visible(false);
        self.settings_exclude_input.set_visible(false);
        self.settings_pattern_help_label.set_visible(false);

        self.schedule_cron_label.set_visible(false);
        self.schedule_cron_input.set_visible(false);
        self.schedule_cron_error.set_visible(false);
        self.schedule_help_label.set_visible(false);

        self.service_status_label.set_visible(false);
        self.service_refresh_button.set_visible(false);
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
                self.auth_site_name_label.set_visible(true);
                self.auth_site_name_input.set_visible(true);
                self.auth_site_desc_label.set_visible(true);
                self.auth_site_desc_input.set_visible(true);
                self.auth_status_label.set_visible(true);
                self.auth_button.set_visible(true);
                self.auth_code_label.set_visible(true);
                self.auth_code_input.set_visible(true);
                self.auth_copy_button.set_visible(true);
                self.auth_url_label.set_visible(true);
                // Set focus to first input field
                self.auth_site_name_input.set_focus();
            }
            Section::Settings => {
                self.settings_server_label.set_visible(true);
                self.settings_server_input.set_visible(true);
                self.settings_source_label.set_visible(true);
                self.settings_source_input.set_visible(true);
                self.settings_source_browse.set_visible(true);
                self.settings_include_label.set_visible(true);
                self.settings_include_input.set_visible(true);
                self.settings_exclude_label.set_visible(true);
                self.settings_exclude_input.set_visible(true);
                self.settings_pattern_help_label.set_visible(true);
                // Set focus to first input field
                self.settings_server_input.set_focus();
            }
            Section::Schedule => {
                self.schedule_cron_label.set_visible(true);
                self.schedule_cron_input.set_visible(true);
                self.schedule_help_label.set_visible(true);
                // Validate current cron expression (will show/hide error label)
                self.validate_cron_expression();
                // Set focus to input field
                self.schedule_cron_input.set_focus();
            }
            Section::Service => {
                self.service_status_label.set_visible(true);
                self.service_refresh_button.set_visible(true);
                self.service_install_button.set_visible(true);
                self.service_start_button.set_visible(true);
                self.service_stop_button.set_visible(true);
                self.service_uninstall_button.set_visible(true);
                // Refresh status and update button states
                self.refresh_service_status();
            }
            Section::Status => {
                self.status_info_label.set_visible(true);
                self.status_refresh_button.set_visible(true);
                // Auto-refresh status info when entering this section
                let info = ServiceManager::get_detailed_info();
                self.status_info_label.set_text(&info);
            }
        }
    }

    fn on_auth_button(&self) {
        // Get site info from UI
        let site_name = self.auth_site_name_input.text();
        if site_name.trim().is_empty() {
            nwg::modal_error_message(&self.window, "Error", "Please enter a site name");
            return;
        }

        let site_description = self.auth_site_desc_input.text();
        let site_description = if site_description.trim().is_empty() {
            None
        } else {
            Some(site_description)
        };

        // Get server URL from config
        let config = self.config.borrow();
        let base_url = config.api.base_url.clone();
        let https_only = config.api.https_only;
        drop(config);

        // Disable button during process
        self.auth_button.set_enabled(false);
        self.auth_status_label
            .set_text("Starting device authorization...");

        // Create DeviceFlowClient
        let client = match DeviceFlowClient::new(base_url.clone(), https_only) {
            Ok(c) => c,
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to create client: {}", e),
                );
                self.auth_button.set_enabled(true);
                self.auth_status_label.set_text("Status: Not authenticated");
                return;
            }
        };

        // Run device flow in background thread
        let site_info = SiteInfo {
            site_name,
            site_description,
        };

        // Create tokio runtime for async operations
        // Note: block_on() will block UI thread during authorization.
        // For a production app, consider using background threads with message passing.
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to create async runtime: {}", e),
                );
                self.auth_button.set_enabled(true);
                self.auth_status_label.set_text("Status: Not authenticated");
                return;
            }
        };
        let auth_result = rt.block_on(async { client.authorize(site_info).await });

        match auth_result {
            Ok(auth_response) => {
                // Open browser with verification URL
                if let Err(e) = open::that(&auth_response.verification_uri) {
                    nwg::modal_error_message(
                        &self.window,
                        "Error",
                        &format!(
                            "Failed to open browser: {}\n\nPlease open manually:\n{}",
                            e, auth_response.verification_uri
                        ),
                    );
                }

                // Display authorization code
                self.auth_status_label
                    .set_text("Browser opened. Enter the code below:");
                self.auth_code_label.set_text("Authorization Code:");
                self.auth_code_input.set_text(&auth_response.user_code);
                self.auth_code_input.set_readonly(true);
                self.auth_url_label.set_text(&format!(
                    "Code expires in {} minutes",
                    auth_response.expires_in / 60
                ));

                // Poll for credentials
                let device_code = auth_response.device_code.clone();
                let interval = auth_response.interval;
                let expires_in = auth_response.expires_in;

                let poll_result = rt.block_on(async {
                    let mut poll_interval =
                        tokio::time::interval(std::time::Duration::from_secs(interval));
                    let start_time = std::time::Instant::now();
                    let timeout = std::time::Duration::from_secs(expires_in);

                    loop {
                        poll_interval.tick().await;

                        if start_time.elapsed() >= timeout {
                            return Err("Authorization timeout".to_string());
                        }

                        match client.poll_for_token(&device_code).await {
                            Ok(Some(credentials)) => return Ok(credentials),
                            Ok(None) => continue,
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                });

                match poll_result {
                    Ok(credentials) => {
                        // Save credentials to config
                        let mut config = self.config.borrow_mut();
                        config.credential.device = Some(DeviceCredentials {
                            site_id: credentials.site_id,
                            domain: credentials.domain,
                            client_secret: credentials.client_secret,
                        });
                        // Update API base URL from credentials
                        config.api.base_url = credentials.api_base_url;
                        drop(config);

                        // Save config to file
                        let config_path = self.config_path.borrow().clone();
                        let config_manager = ConfigManager::new(&config_path);
                        let config = self.config.borrow().clone();
                        if let Err(e) = config_manager.save(&config) {
                            nwg::modal_error_message(
                                &self.window,
                                "Error",
                                &format!("Failed to save credentials: {}", e),
                            );
                        }

                        self.auth_status_label
                            .set_text("Status: Authenticated (Device Flow)");
                        self.auth_code_label.set_text("");
                        self.auth_code_input.set_text("");
                        self.auth_url_label.set_text("");
                        nwg::modal_info_message(
                            &self.window,
                            "Success",
                            "Device authorization completed successfully!",
                        );
                    }
                    Err(e) => {
                        self.auth_status_label
                            .set_text("Status: Authorization failed");
                        self.auth_code_label.set_text("");
                        self.auth_code_input.set_text("");
                        self.auth_url_label.set_text("");
                        nwg::modal_error_message(
                            &self.window,
                            "Error",
                            &format!("Authorization failed: {}", e),
                        );
                    }
                }
            }
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to start authorization: {}", e),
                );
                self.auth_status_label.set_text("Status: Not authenticated");
            }
        }

        self.auth_button.set_enabled(true);
    }

    fn on_copy_code(&self) {
        let code = self.auth_code_input.text();
        if !code.is_empty() {
            // Select all text and copy to clipboard
            self.auth_code_input.set_selection(0..code.len() as u32);
            // Use Windows clipboard API through nwg
            nwg::Clipboard::set_data_text(&self.window, &code);
            self.auth_url_label.set_text("Code copied to clipboard!");
        }
    }

    fn on_browse_source(&self) {
        let mut dialog = Default::default();
        if nwg::FileDialog::builder()
            .title("Select Source Directory")
            .action(nwg::FileDialogAction::OpenDirectory)
            .build(&mut dialog)
            .is_ok()
            && dialog.run(Some(&self.window))
        {
            if let Ok(directory) = dialog.get_selected_item() {
                self.settings_source_input
                    .set_text(&directory.to_string_lossy());
            }
        }
    }

    fn on_service_install(&self) {
        match ServiceManager::install() {
            Ok(msg) => {
                nwg::modal_info_message(&self.window, "Success", &msg);
                self.refresh_service_status();
            }
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to install service:\n{}", e),
                );
            }
        }
    }

    fn on_service_start(&self) {
        match ServiceManager::start() {
            Ok(msg) => {
                nwg::modal_info_message(&self.window, "Success", &msg);
                self.refresh_service_status();
            }
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to start service:\n{}", e),
                );
            }
        }
    }

    fn on_service_stop(&self) {
        match ServiceManager::stop() {
            Ok(msg) => {
                nwg::modal_info_message(&self.window, "Success", &msg);
                self.refresh_service_status();
            }
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to stop service:\n{}", e),
                );
            }
        }
    }

    fn on_service_uninstall(&self) {
        // Confirm before uninstalling
        let result = nwg::modal_message(
            &self.window,
            &nwg::MessageParams {
                title: "Confirm Uninstall",
                content: "Are you sure you want to uninstall the service?\n\nThis will stop and remove the Windows service.",
                buttons: nwg::MessageButtons::YesNo,
                icons: nwg::MessageIcons::Warning,
            },
        );

        if result == nwg::MessageChoice::Yes {
            match ServiceManager::uninstall() {
                Ok(msg) => {
                    nwg::modal_info_message(&self.window, "Success", &msg);
                    self.refresh_service_status();
                }
                Err(e) => {
                    nwg::modal_error_message(
                        &self.window,
                        "Error",
                        &format!("Failed to uninstall service:\n{}", e),
                    );
                }
            }
        }
    }

    fn refresh_service_status(&self) {
        let status = ServiceManager::get_status();
        self.service_status_label
            .set_text(&format!("Service Status: {}", status));

        // Check if authentication is configured
        let is_authenticated = self.config.borrow().credential.is_device_flow();

        // Update button states based on service status
        match status {
            ServiceStatus::NotInstalled => {
                // Install only enabled if authenticated
                self.service_install_button.set_enabled(is_authenticated);
                self.service_start_button.set_enabled(false);
                self.service_stop_button.set_enabled(false);
                self.service_uninstall_button.set_enabled(false);
            }
            ServiceStatus::Stopped => {
                self.service_install_button.set_enabled(false);
                self.service_start_button.set_enabled(true);
                self.service_stop_button.set_enabled(false);
                self.service_uninstall_button.set_enabled(true);
            }
            ServiceStatus::Running => {
                self.service_install_button.set_enabled(false);
                self.service_start_button.set_enabled(false);
                self.service_stop_button.set_enabled(true);
                self.service_uninstall_button.set_enabled(true);
            }
            ServiceStatus::Unknown => {
                // Install only enabled if authenticated
                self.service_install_button.set_enabled(is_authenticated);
                self.service_start_button.set_enabled(true);
                self.service_stop_button.set_enabled(true);
                self.service_uninstall_button.set_enabled(true);
            }
        }
    }

    fn on_status_refresh(&self) {
        // Update service status in Service section
        self.refresh_service_status();
        // Update detailed info in Status section
        let info = ServiceManager::get_detailed_info();
        self.status_info_label.set_text(&info);
    }

    fn on_save(&self) {
        // Save config from UI
        let config = self.save_config_from_ui();

        // Validate HTTPS URL requirement
        if config.api.https_only {
            let url = config.api.base_url.trim().to_lowercase();
            if !url.starts_with("https://") {
                nwg::modal_error_message(
                    &self.window,
                    "Validation Error",
                    "Server URL must use HTTPS when https_only is enabled.\n\nEither:\n- Change URL to start with https://\n- Or disable https_only in config (not recommended)",
                );
                return;
            }
        }

        // Validate source directory exists
        if !config.src.source_dir.exists() {
            let result = nwg::modal_message(
                &self.window,
                &nwg::MessageParams {
                    title: "Warning",
                    content: &format!(
                        "Source directory does not exist:\n{}\n\nSave anyway?",
                        config.src.source_dir.display()
                    ),
                    buttons: nwg::MessageButtons::YesNo,
                    icons: nwg::MessageIcons::Warning,
                },
            );
            if result == nwg::MessageChoice::No {
                return;
            }
        }

        // Validate cron expression before saving
        if let Err(e) = Self::validate_cron_static(&config.scheduler.crontab) {
            nwg::modal_error_message(&self.window, "Validation Error", &e);
            return;
        }

        // Validate patterns before saving
        if let Err(e) = Self::validate_patterns(&config.src.include_patterns, "include") {
            nwg::modal_error_message(&self.window, "Validation Error", &e);
            return;
        }
        if let Err(e) = Self::validate_patterns(&config.src.exclude_patterns, "exclude") {
            nwg::modal_error_message(&self.window, "Validation Error", &e);
            return;
        }

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
        // Create application icon (using system icon as placeholder)
        nwg::Icon::builder()
            .source_system(Some(nwg::OemIcon::Information))
            .build(&mut data.app_icon)?;

        // Main window with icon
        nwg::Window::builder()
            .size((900, 600))
            .position((200, 100))
            .title("Data Exporter Configurator")
            .icon(Some(&data.app_icon))
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
            .build(&mut data.window)?;

        // Create fonts (increased sizes for better readability)
        nwg::Font::builder()
            .family("Segoe UI")
            .size(22)
            .weight(700)
            .build(&mut data.title_font)?;

        nwg::Font::builder()
            .family("Segoe UI")
            .size(18)
            .weight(600)
            .build(&mut data.heading_font)?;

        nwg::Font::builder()
            .family("Segoe UI")
            .size(16)
            .build(&mut data.normal_font)?;

        nwg::Font::builder()
            .family("Segoe UI")
            .size(16)
            .weight(700) // Bold
            .build(&mut data.error_font)?;

        // === Navigation panel (left side) ===
        nwg::Frame::builder()
            .position((0, 0))
            .size((200, 600))
            .parent(&data.window)
            .build(&mut data.nav_frame)?;

        nwg::Label::builder()
            .text("Data Exporter")
            .position((15, 15))
            .size((170, 35))
            .font(Some(&data.heading_font))
            .parent(&data.nav_frame)
            .build(&mut data.nav_title)?;

        nwg::Button::builder()
            .text("Authentication")
            .position((15, 60))
            .size((150, 45))
            .font(Some(&data.normal_font))
            .parent(&data.nav_frame)
            .build(&mut data.nav_auth_button)?;

        nwg::Button::builder()
            .text("Settings")
            .position((15, 115))
            .size((150, 45))
            .font(Some(&data.normal_font))
            .parent(&data.nav_frame)
            .build(&mut data.nav_settings_button)?;

        nwg::Button::builder()
            .text("Schedule")
            .position((15, 170))
            .size((150, 45))
            .font(Some(&data.normal_font))
            .parent(&data.nav_frame)
            .build(&mut data.nav_schedule_button)?;

        nwg::Button::builder()
            .text("Service")
            .position((15, 225))
            .size((150, 45))
            .font(Some(&data.normal_font))
            .parent(&data.nav_frame)
            .build(&mut data.nav_service_button)?;

        nwg::Button::builder()
            .text("Status")
            .position((15, 280))
            .size((150, 45))
            .font(Some(&data.normal_font))
            .parent(&data.nav_frame)
            .build(&mut data.nav_status_button)?;

        nwg::Label::builder()
            .text("Authentication")
            .position((220, 20))
            .size((660, 40))
            .font(Some(&data.title_font))
            .parent(&data.window)
            .build(&mut data.section_title)?;

        // === Section: Authentication ===
        nwg::Label::builder()
            .text("Configure authentication using Device Authorization Flow")
            .position((220, 75))
            .size((660, 25))
            .parent(&data.window)
            .build(&mut data.auth_desc_label)?;

        nwg::Label::builder()
            .text("Site Name:")
            .position((220, 115))
            .size((160, 25))
            .parent(&data.window)
            .build(&mut data.auth_site_name_label)?;

        nwg::TextInput::builder()
            .text("")
            .position((390, 115))
            .size((470, 28))
            .font(Some(&data.normal_font))
            .parent(&data.window)
            .build(&mut data.auth_site_name_input)?;

        nwg::Label::builder()
            .text("Description (optional):")
            .position((220, 160))
            .size((160, 25))
            .parent(&data.window)
            .build(&mut data.auth_site_desc_label)?;

        nwg::TextInput::builder()
            .text("")
            .position((390, 160))
            .size((470, 28))
            .font(Some(&data.normal_font))
            .parent(&data.window)
            .build(&mut data.auth_site_desc_input)?;

        nwg::Label::builder()
            .text("Status: Not authenticated")
            .position((220, 210))
            .size((660, 25))
            .parent(&data.window)
            .build(&mut data.auth_status_label)?;

        nwg::Button::builder()
            .text("Start Device Authorization")
            .position((220, 250))
            .size((240, 40))
            .parent(&data.window)
            .build(&mut data.auth_button)?;

        nwg::Label::builder()
            .text("")
            .position((220, 310))
            .size((160, 28))
            .parent(&data.window)
            .build(&mut data.auth_code_label)?;

        nwg::TextInput::builder()
            .text("")
            .position((390, 310))
            .size((300, 35))
            .font(Some(&data.heading_font))
            .readonly(true)
            .parent(&data.window)
            .build(&mut data.auth_code_input)?;

        nwg::Button::builder()
            .text("Copy Code")
            .position((700, 310))
            .size((100, 35))
            .parent(&data.window)
            .build(&mut data.auth_copy_button)?;

        nwg::Label::builder()
            .text("")
            .position((220, 360))
            .size((660, 35))
            .parent(&data.window)
            .build(&mut data.auth_url_label)?;

        // === Section: Settings ===
        nwg::Label::builder()
            .text("Server URL:")
            .position((220, 85))
            .size((160, 25))
            .parent(&data.window)
            .build(&mut data.settings_server_label)?;

        nwg::TextInput::builder()
            .text("https://")
            .position((390, 85))
            .size((470, 28))
            .font(Some(&data.normal_font))
            .parent(&data.window)
            .build(&mut data.settings_server_input)?;

        nwg::Label::builder()
            .text("Source Directory:")
            .position((220, 135))
            .size((160, 25))
            .parent(&data.window)
            .build(&mut data.settings_source_label)?;

        nwg::TextInput::builder()
            .position((390, 135))
            .size((370, 28))
            .font(Some(&data.normal_font))
            .parent(&data.window)
            .build(&mut data.settings_source_input)?;

        nwg::Button::builder()
            .text("Browse...")
            .position((770, 135))
            .size((90, 28))
            .parent(&data.window)
            .build(&mut data.settings_source_browse)?;

        nwg::Label::builder()
            .text("Include Patterns:")
            .position((220, 185))
            .size((160, 25))
            .parent(&data.window)
            .build(&mut data.settings_include_label)?;

        nwg::TextInput::builder()
            .text("")
            .position((390, 185))
            .size((470, 28))
            .font(Some(&data.normal_font))
            .parent(&data.window)
            .build(&mut data.settings_include_input)?;

        nwg::Label::builder()
            .text("Exclude Patterns:")
            .position((220, 235))
            .size((160, 25))
            .parent(&data.window)
            .build(&mut data.settings_exclude_label)?;

        nwg::TextInput::builder()
            .text("")
            .position((390, 235))
            .size((470, 28))
            .font(Some(&data.normal_font))
            .parent(&data.window)
            .build(&mut data.settings_exclude_input)?;

        nwg::Label::builder()
            .text("Patterns: Comma-separated glob patterns (e.g., *.dbf, nsf*.DBF)")
            .position((220, 280))
            .size((640, 40))
            .parent(&data.window)
            .build(&mut data.settings_pattern_help_label)?;

        // === Section: Schedule ===
        nwg::Label::builder()
            .text("Cron Expression:")
            .position((220, 85))
            .size((160, 25))
            .parent(&data.window)
            .build(&mut data.schedule_cron_label)?;

        nwg::TextInput::builder()
            .text("0 0 8,12,16,18 * * *")
            .position((390, 85))
            .size((470, 28))
            .font(Some(&data.normal_font))
            .parent(&data.window)
            .build(&mut data.schedule_cron_input)?;

        // Cron validation error - RichLabel with proper background color support
        nwg::RichLabel::builder()
            .text("")
            .position((390, 125))
            .size((470, 28))
            .h_align(nwg::HTextAlign::Center)
            .line_height(Some(36)) // Vertical centering via line height
            .background_color(Some([220, 53, 69])) // Bright red background
            .parent(&data.window)
            .build(&mut data.schedule_cron_error)?;

        nwg::Label::builder()
            .text("Examples:\n\n  0 0 8,12,16,18 * * *    Run at 8am, 12pm, 4pm, 6pm daily\n\n  0 0 */4 * * *           Run every 4 hours\n\n  0 30 9 * * *            Run at 9:30am daily")
            .position((220, 175))
            .size((660, 220))
            .font(Some(&data.normal_font))
            .parent(&data.window)
            .build(&mut data.schedule_help_label)?;

        // === Section: Service ===
        nwg::Label::builder()
            .text("Service Status: Click 'Refresh Status' to check")
            .position((220, 85))
            .size((450, 25))
            .parent(&data.window)
            .build(&mut data.service_status_label)?;

        nwg::Button::builder()
            .text("Refresh Status")
            .position((680, 80))
            .size((130, 35))
            .parent(&data.window)
            .build(&mut data.service_refresh_button)?;

        nwg::Button::builder()
            .text("Install Service")
            .position((220, 140))
            .size((160, 45))
            .parent(&data.window)
            .build(&mut data.service_install_button)?;

        nwg::Button::builder()
            .text("Start Service")
            .position((395, 140))
            .size((160, 45))
            .parent(&data.window)
            .build(&mut data.service_start_button)?;

        nwg::Button::builder()
            .text("Stop Service")
            .position((570, 140))
            .size((160, 45))
            .parent(&data.window)
            .build(&mut data.service_stop_button)?;

        nwg::Button::builder()
            .text("Uninstall Service")
            .position((220, 200))
            .size((160, 45))
            .parent(&data.window)
            .build(&mut data.service_uninstall_button)?;

        // === Section: Status ===
        nwg::Label::builder()
            .text("Service information will appear here")
            .position((220, 85))
            .size((660, 280))
            .parent(&data.window)
            .build(&mut data.status_info_label)?;

        nwg::Button::builder()
            .text("Refresh")
            .position((220, 380))
            .size((130, 45))
            .parent(&data.window)
            .build(&mut data.status_refresh_button)?;

        // === Bottom buttons ===
        nwg::Button::builder()
            .text("Save")
            .position((660, 540))
            .size((110, 45))
            .font(Some(&data.normal_font))
            .parent(&data.window)
            .build(&mut data.save_button)?;

        nwg::Button::builder()
            .text("Cancel")
            .position((780, 540))
            .size((110, 45))
            .font(Some(&data.normal_font))
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
                        if handle == ui.window {
                            ui.exit();
                        }
                    }
                    nwg::Event::OnButtonClick => {
                        // Navigation
                        if handle == ui.nav_auth_button {
                            ui.show_section(Section::Auth);
                        } else if handle == ui.nav_settings_button {
                            ui.show_section(Section::Settings);
                        } else if handle == ui.nav_schedule_button {
                            ui.show_section(Section::Schedule);
                        } else if handle == ui.nav_service_button {
                            ui.show_section(Section::Service);
                        } else if handle == ui.nav_status_button {
                            ui.show_section(Section::Status);
                        }
                        // Actions
                        else if handle == ui.auth_button {
                            ui.on_auth_button();
                        } else if handle == ui.auth_copy_button {
                            ui.on_copy_code();
                        } else if handle == ui.settings_source_browse {
                            ui.on_browse_source();
                        } else if handle == ui.service_refresh_button {
                            ui.refresh_service_status();
                        } else if handle == ui.service_install_button {
                            ui.on_service_install();
                        } else if handle == ui.service_start_button {
                            ui.on_service_start();
                        } else if handle == ui.service_stop_button {
                            ui.on_service_stop();
                        } else if handle == ui.service_uninstall_button {
                            ui.on_service_uninstall();
                        } else if handle == ui.status_refresh_button {
                            ui.on_status_refresh();
                        } else if handle == ui.save_button {
                            ui.on_save();
                        } else if handle == ui.cancel_button {
                            ui.on_cancel();
                        }
                    }
                    nwg::Event::OnTextInput => {
                        // Real-time cron validation
                        if handle == ui.schedule_cron_input {
                            ui.validate_cron_expression();
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
