use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use cuboid_core::{
    AppConfig, CONFIG_FILE_NAME, ConfigStorage, ConfigStorageError, INSTALLED_DIRECTORY_NAME,
    InstalledConfigAdapter, Language, PortableConfigAdapter, RuntimeCommand, RuntimeEvent,
};
use cuboid_ui::{UiExtension, UiIntent, UiState, theme};
use cuboid_windows::{
    RegistrySetting, Runtime, RuntimeAdapters, WindowsAppearance, WindowsAppearanceProvider,
    WindowsTheme, set_launch_at_login,
};
use eframe::egui;
use tracing_subscriber::EnvFilter;
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};

const APPEARANCE_POLL_INTERVAL: Duration = Duration::from_secs(2);
const PORTABLE_FLAG: &str = "portable.flag";

/// Where a distribution keeps its documents and logs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageLocation {
    /// Installed next to the user's roaming application data.
    Installed { app_data: PathBuf },
    /// Portable: everything lives next to the executable.
    Portable { directory: PathBuf },
}

impl StorageLocation {
    /// Detects the location from the running executable and the environment.
    pub fn detect() -> Result<Self, String> {
        let executable = std::env::current_exe()
            .map_err(|error| format!("cannot locate the executable: {error}"))?;
        let directory = executable
            .parent()
            .ok_or_else(|| "the executable has no parent directory".to_owned())?;
        if directory.join(PORTABLE_FLAG).exists() {
            return Ok(Self::Portable {
                directory: directory.to_owned(),
            });
        }

        let app_data = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| "APPDATA is not available".to_owned())?;
        Ok(Self::Installed { app_data })
    }

    /// The log directory for `product_directory`, e.g. `Cuboid`.
    pub fn log_directory(&self, product_directory: &str) -> Result<PathBuf, String> {
        match self {
            Self::Portable { directory } => Ok(directory.join("logs")),
            Self::Installed { .. } => std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|directory| directory.join(product_directory).join("logs"))
                .ok_or_else(|| "LOCALAPPDATA is not available".to_owned()),
        }
    }

    /// The directory a portable document lives in, if this location is portable.
    pub fn portable_directory(&self) -> Option<&Path> {
        match self {
            Self::Portable { directory } => Some(directory),
            Self::Installed { .. } => None,
        }
    }

    /// The roaming application data root, if this location is an installation.
    pub fn app_data_root(&self) -> Option<&Path> {
        match self {
            Self::Installed { app_data } => Some(app_data),
            Self::Portable { .. } => None,
        }
    }
}

/// Everything a distribution contributes to the shared application shell.
pub struct Profile {
    /// Product name: window title, tray tooltip and startup registry value.
    pub product: &'static str,
    /// Where the settings document is read from and written to.
    pub storage: Box<dyn ConfigStorage>,
    /// Where rolling log files are written.
    pub log_directory: PathBuf,
    /// Extra settings pages, if any.
    pub ui_extension: Option<Box<dyn UiExtension>>,
    /// Snap, shortcut and window adapters for the Windows runtime.
    pub adapters: RuntimeAdapters,
}

impl Profile {
    /// The Base profile: standard adapters, no extra pages.
    pub fn base() -> Result<Self, String> {
        let location = StorageLocation::detect()?;
        let storage: Box<dyn ConfigStorage> = match &location {
            StorageLocation::Portable { directory } => {
                Box::new(PortableConfigAdapter::new(directory))
            }
            StorageLocation::Installed { app_data } => {
                Box::new(InstalledConfigAdapter::new(app_data))
            }
        };
        Ok(Self {
            product: "Cuboid",
            storage,
            log_directory: location.log_directory(INSTALLED_DIRECTORY_NAME)?,
            ui_extension: None,
            adapters: RuntimeAdapters::default(),
        })
    }
}

/// Starts logging, the Windows runtime, the tray icon and the settings window.
pub fn run(profile: Profile) -> eframe::Result {
    let Profile {
        product,
        storage,
        log_directory,
        ui_extension,
        adapters,
    } = profile;

    std::fs::create_dir_all(&log_directory).expect("Cuboid could not create its log directory");
    let file_appender = tracing_appender::rolling::daily(log_directory, "cuboid.log");
    let (log_writer, log_guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(log_writer)
        .with_target(false)
        .compact()
        .init();

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let imported = storage
        .load()
        .expect("Cuboid could not load its configuration");
    if !imported.discarded_sections.is_empty() {
        tracing::warn!(
            sections = ?imported.discarded_sections,
            archive = ?imported.archived_sections,
            "the configuration contained settings this build does not support; they were not loaded, and a copy was kept next to the document"
        );
    }
    let config = imported.config;
    if let Ok(executable) = std::env::current_exe()
        && let Err(error) = set_launch_at_login(product, config.launch_at_login, &executable)
    {
        tracing::error!(%error, "could not synchronize launch-at-login setting");
    }
    let runtime = Runtime::start_with(config.clone(), adapters)
        .expect("Cuboid could not start its Windows runtime");
    let commands = runtime.commands();
    let events = runtime.events();
    let tray =
        create_tray_icon(config.language, product).expect("Cuboid could not create its tray icon");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(product)
            .with_inner_size([1_040.0, 720.0])
            .with_min_inner_size([760.0, 560.0])
            .with_visible(false),
        ..Default::default()
    };

    eframe::run_native(
        product,
        options,
        Box::new(move |creation_context| {
            configure_egui(&creation_context.egui_ctx);
            let appearance_provider = WindowsAppearanceProvider::new();
            let appearance = appearance_provider.read();
            report_appearance_fallbacks(&appearance);
            apply_windows_style(&creation_context.egui_ctx, &appearance);

            let ui = match ui_extension {
                Some(extension) => UiState::with_extension(config, extension),
                None => UiState::new(config),
            };
            Ok(Box::new(DesktopApp {
                ui,
                _runtime: runtime,
                commands,
                events,
                tray,
                storage,
                product,
                _log_guard: log_guard,
                allow_close: false,
                appearance_provider,
                appearance,
                next_appearance_check: Instant::now() + APPEARANCE_POLL_INTERVAL,
            }))
        }),
    )
}

struct DesktopApp {
    ui: UiState,
    _runtime: Runtime,
    commands: Sender<RuntimeCommand>,
    events: Receiver<RuntimeEvent>,
    tray: TrayControls,
    storage: Box<dyn ConfigStorage>,
    product: &'static str,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
    allow_close: bool,
    appearance_provider: WindowsAppearanceProvider,
    appearance: WindowsAppearance,
    next_appearance_check: Instant,
}

impl eframe::App for DesktopApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        if now >= self.next_appearance_check {
            let appearance = self.appearance_provider.read();
            if appearance != self.appearance {
                report_appearance_fallbacks(&appearance);
                apply_windows_style(ctx, &appearance);
                self.appearance = appearance;
            }
            self.next_appearance_check = now + APPEARANCE_POLL_INTERVAL;
        }

        while let Ok(event) = self.events.try_recv() {
            self.ui.handle_event(event);
        }

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.tray.open_id {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if event.id == self.tray.quit_id {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        if !self.allow_close && ctx.input(|input| input.viewport().close_requested()) {
            if self.ui.cancel_shortcut_capture() {
                let _ = self
                    .commands
                    .send(RuntimeCommand::SetHotkeysSuspended(false));
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let intents = self.ui.show(ui);
        for intent in intents {
            match intent {
                UiIntent::Apply(action) => {
                    let _ = self.commands.send(RuntimeCommand::Apply(action));
                }
                UiIntent::ApplyArea(bounds) => {
                    let _ = self.commands.send(RuntimeCommand::ApplyArea(bounds));
                }
                UiIntent::Save(config) => {
                    if self.apply_startup_setting(&config).is_ok() {
                        match self.storage.save(&config) {
                            Ok(()) => {
                                self.tray.set_language(config.language);
                                let _ = self.commands.send(RuntimeCommand::UpdateConfig(config));
                                self.ui
                                    .set_status("Configurazione salvata", "Configuration saved");
                            }
                            Err(error) => self.ui.set_error(error),
                        }
                    }
                }
                UiIntent::SetShortcutCaptureActive(active) => {
                    let _ = self
                        .commands
                        .send(RuntimeCommand::SetHotkeysSuspended(active));
                }
                UiIntent::Import => self.import_config(),
                UiIntent::Export(config) => self.export_config(&config),
                UiIntent::Hide => {
                    let _ = self
                        .commands
                        .send(RuntimeCommand::SetHotkeysSuspended(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
            }
        }
    }
}

fn configure_egui(ctx: &egui::Context) {
    ctx.options_mut(|options| options.zoom_with_keyboard = false);
}

impl DesktopApp {
    fn apply_startup_setting(&mut self, config: &AppConfig) -> Result<(), ()> {
        let executable = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                self.ui.set_error(error);
                return Err(());
            }
        };
        if let Err(error) = set_launch_at_login(self.product, config.launch_at_login, &executable) {
            self.ui.set_error(error);
            return Err(());
        }
        Ok(())
    }

    fn import_config(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Cuboid JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        let result = std::fs::read(path)
            .map_err(ConfigStorageError::from)
            .and_then(|json| self.storage.import(&json))
            .and_then(|imported| {
                self.storage.save(&imported.config)?;
                Ok(imported)
            });
        match result {
            Ok(imported) => {
                self.tray.set_language(imported.config.language);
                self.ui.replace_config(imported.config.clone());
                let _ = self
                    .commands
                    .send(RuntimeCommand::UpdateConfig(imported.config));
                if imported.discarded_sections.is_empty() {
                    self.ui
                        .set_status("Configurazione importata", "Configuration imported");
                } else {
                    self.ui.set_status(
                        "Configurazione importata; impostazioni non supportate ignorate",
                        "Configuration imported; unsupported settings were ignored",
                    );
                }
            }
            Err(error) => self.ui.set_error(error),
        }
    }

    fn export_config(&mut self, config: &AppConfig) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Cuboid JSON", &["json"])
            .set_file_name(CONFIG_FILE_NAME)
            .save_file()
        else {
            return;
        };
        let result = self
            .storage
            .export(config)
            .and_then(|json| std::fs::write(path, json).map_err(ConfigStorageError::from));
        match result {
            Ok(()) => self
                .ui
                .set_status("Configurazione esportata", "Configuration exported"),
            Err(error) => self.ui.set_error(error),
        }
    }
}

struct TrayControls {
    _icon: TrayIcon,
    open: MenuItem,
    quit: MenuItem,
    open_id: MenuId,
    quit_id: MenuId,
    product: &'static str,
}

impl TrayControls {
    fn set_language(&self, language: Language) {
        let (open, quit) = tray_labels(language, self.product);
        self.open.set_text(open);
        self.quit.set_text(quit);
    }
}

fn create_tray_icon(language: Language, product: &'static str) -> Result<TrayControls, String> {
    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let border = x < 3 || y < 3 || x >= SIZE - 3 || y >= SIZE - 3;
            let divider = (15..=17).contains(&x) || (15..=17).contains(&y);
            let (r, g, b, a) = if border || divider {
                (238, 242, 255, 255)
            } else {
                (72, 104, 210, 255)
            };
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }
    let icon =
        Icon::from_rgba(rgba, SIZE, SIZE).map_err(|error| format!("invalid tray icon: {error}"))?;
    let menu = Menu::new();
    let (open_label, quit_label) = tray_labels(language, product);
    let open = MenuItem::new(open_label, true, None);
    let quit = MenuItem::new(quit_label, true, None);
    menu.append_items(&[&open, &quit])
        .map_err(|error| format!("failed to create tray menu: {error}"))?;
    let open_id = open.id().clone();
    let quit_id = quit.id().clone();
    let tray = TrayIconBuilder::new()
        .with_tooltip(product)
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .map_err(|error| format!("failed to create tray icon: {error}"))?;
    Ok(TrayControls {
        _icon: tray,
        open,
        quit,
        open_id,
        quit_id,
        product,
    })
}

fn tray_labels(language: Language, product: &str) -> (String, String) {
    match language {
        Language::Italian => (format!("Apri {product}"), "Esci".to_owned()),
        Language::English => (format!("Open {product}"), "Exit".to_owned()),
    }
}

fn report_appearance_fallbacks(appearance: &WindowsAppearance) {
    if let RegistrySetting::Unavailable(issue) = &appearance.theme {
        tracing::warn!(
            ?issue,
            "Windows app theme is unavailable; following eframe's current system theme"
        );
    }
    if let RegistrySetting::Unavailable(issue) = &appearance.accent {
        tracing::debug!(
            ?issue,
            "Windows accent is unavailable; Cuboid uses its own brand palette anyway"
        );
    }
}

fn apply_windows_style(ctx: &egui::Context, appearance: &WindowsAppearance) {
    ctx.set_style_of(egui::Theme::Dark, theme::style(true));
    ctx.set_style_of(egui::Theme::Light, theme::style(false));

    match appearance.theme {
        RegistrySetting::Value(WindowsTheme::Dark) => ctx.set_theme(egui::Theme::Dark),
        RegistrySetting::Value(WindowsTheme::Light) => ctx.set_theme(egui::Theme::Light),
        RegistrySetting::Unavailable(_) => ctx.set_theme(egui::ThemePreference::System),
    }
}

#[cfg(test)]
mod tests {
    use super::{configure_egui, tray_labels};
    use cuboid_core::Language;

    #[test]
    fn keyboard_zoom_is_disabled() {
        let ctx = eframe::egui::Context::default();

        configure_egui(&ctx);

        assert!(!ctx.options(|options| options.zoom_with_keyboard));
    }

    #[test]
    fn tray_labels_name_the_running_product() {
        assert_eq!(
            tray_labels(Language::Italian, "Cuboid"),
            ("Apri Cuboid".to_owned(), "Esci".to_owned())
        );
        assert_eq!(
            tray_labels(Language::English, "Cuboid"),
            ("Open Cuboid".to_owned(), "Exit".to_owned())
        );
    }
}
