use cuboid_core::{Action, AppConfig, HotkeyBinding, Language, NormalizedRect, RuntimeEvent};
use egui::{Color32, Event, Key, Modifiers, RichText, Sense, Stroke, StrokeKind, Ui};

pub mod extension;
pub mod theme;

pub use extension::{ExtensionChange, ExtensionPage, LocalizedText, PageIcon, UiExtension};
pub use theme::Palette;

const MOD_ALT: u32 = 0x0001;
const MOD_CONTROL: u32 = 0x0002;
const MOD_SHIFT: u32 = 0x0004;
const MOD_WIN: u32 = 0x0008;
const ACTION_CARD_MIN_WIDTH: f32 = 210.0;
const ACTION_CARD_SPACING: f32 = 10.0;
const ACTION_MAX_COLUMNS: usize = 3;
const SHORTCUT_CARD_MIN_WIDTH: f32 = 470.0;
const SHORTCUT_CARD_SPACING: f32 = 10.0;
const SHORTCUT_MAX_COLUMNS: usize = 3;
const NAVIGATION_WIDTH: f32 = 152.0;
const NAVIGATION_HORIZONTAL_MARGIN: i8 = 18;
const CONTENT_HORIZONTAL_MARGIN: i8 = 24;
const CONTENT_VERTICAL_MARGIN: i8 = 10;

#[derive(Clone, Debug)]
pub enum UiIntent {
    Apply(Action),
    /// Places the active window on the given fraction of its work area.
    ApplyArea(NormalizedRect),
    Save(AppConfig),
    SetShortcutCaptureActive(bool),
    Import,
    Export(AppConfig),
    Hide,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum Page {
    #[default]
    Actions,
    Shortcuts,
    /// A page contributed by the host, identified by its index.
    Extension(usize),
    General,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShortcutCapture {
    Existing(usize),
    New(Action),
}

pub struct UiState {
    config: AppConfig,
    page: Page,
    status: String,
    shortcut_capture: Option<ShortcutCapture>,
    win_keys_down: [bool; 2],
    extension: Option<Box<dyn UiExtension>>,
}

impl UiState {
    pub fn new(config: AppConfig) -> Self {
        let status = text(config.language, "Cuboid è pronto", "Cuboid is ready").to_owned();
        Self {
            config,
            page: Page::default(),
            status,
            shortcut_capture: None,
            win_keys_down: [false; 2],
            extension: None,
        }
    }

    /// Builds the settings window with additional host-provided pages.
    pub fn with_extension(config: AppConfig, extension: Box<dyn UiExtension>) -> Self {
        Self {
            extension: Some(extension),
            ..Self::new(config)
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    fn extension_pages(&self) -> Vec<ExtensionPage> {
        self.extension
            .as_ref()
            .map(|extension| extension.pages().to_vec())
            .unwrap_or_default()
    }

    pub fn show(&mut self, root: &mut egui::Ui) -> Vec<UiIntent> {
        let mut intents = Vec::new();
        let mut changed = false;
        let shortcut_capture_was_active = self.shortcut_capture.is_some();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(0))
            .show(root, |ui| {
                let panel_height = ui.available_height();
                let navigation_outer_width =
                    NAVIGATION_WIDTH + f32::from(NAVIGATION_HORIZONTAL_MARGIN) * 2.0;
                let content_outer_width = (ui.available_width() - navigation_outer_width).max(0.0);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let navigation_fill = Palette::of(ui).surface_alt;
                    ui.allocate_ui_with_layout(
                        egui::vec2(navigation_outer_width, panel_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .fill(navigation_fill)
                                .inner_margin(egui::Margin::symmetric(
                                    NAVIGATION_HORIZONTAL_MARGIN,
                                    20,
                                ))
                                .show(ui, |ui| {
                                    ui.set_min_width(NAVIGATION_WIDTH);
                                    ui.set_min_height((panel_height - 40.0).max(0.0));
                                    self.navigation(ui, &mut intents);
                                });
                        },
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(content_outer_width, panel_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .fill(Palette::of(ui).panel)
                                .inner_margin(egui::Margin::symmetric(
                                    CONTENT_HORIZONTAL_MARGIN,
                                    CONTENT_VERTICAL_MARGIN,
                                ))
                                .show(ui, |ui| {
                                    let width = (content_outer_width
                                        - f32::from(CONTENT_HORIZONTAL_MARGIN) * 2.0)
                                        .max(0.0);
                                    let content_height = (panel_height
                                        - f32::from(CONTENT_VERTICAL_MARGIN) * 2.0)
                                        .max(0.0);
                                    ui.set_min_width(width);
                                    ui.set_min_height(content_height);
                                    let content_rect = egui::Rect::from_min_size(
                                        ui.available_rect_before_wrap().min,
                                        egui::vec2(width, content_height),
                                    );
                                    let body_rect = content_rect;
                                    ui.scope_builder(
                                        egui::UiBuilder::new()
                                            .max_rect(body_rect)
                                            .layout(egui::Layout::top_down(egui::Align::Min)),
                                        |ui| {
                                            self.page_header(ui);
                                            ui.add_space(18.0);
                                            if self.page != Page::Shortcuts {
                                                self.shortcut_capture = None;
                                                self.win_keys_down = [false; 2];
                                            }
                                            match self.page {
                                                Page::Actions => {
                                                    self.actions_page(ui, &mut intents);
                                                }
                                                Page::Shortcuts => {
                                                    changed |=
                                                        self.shortcuts_page(ui, body_rect.bottom());
                                                }
                                                Page::Extension(index) => {
                                                    changed |= self
                                                        .extension_page(ui, index, &mut intents)
                                                        .persists();
                                                }
                                                Page::General => {
                                                    changed |=
                                                        scrollable(ui, "general-scroll", |ui| {
                                                            self.general_page(ui, &mut intents)
                                                        });
                                                }
                                            }
                                        },
                                    );
                                    ui.allocate_rect(content_rect, Sense::hover());
                                });
                        },
                    );
                });
            });

        if changed {
            match self.config.validate() {
                Ok(()) => intents.push(UiIntent::Save(self.config.clone())),
                Err(error) => {
                    self.status = format!(
                        "{}: {error}",
                        text(
                            self.config.language,
                            "Configurazione non valida",
                            "Invalid configuration"
                        )
                    );
                }
            }
        }
        let shortcut_capture_is_active = self.shortcut_capture.is_some();
        if shortcut_capture_is_active != shortcut_capture_was_active {
            intents.push(UiIntent::SetShortcutCaptureActive(
                shortcut_capture_is_active,
            ));
        }
        intents
    }

    pub fn handle_event(&mut self, event: RuntimeEvent) {
        self.status = match event {
            RuntimeEvent::Ready => text(
                self.config.language,
                "Runtime Windows pronto",
                "Windows runtime ready",
            )
            .to_owned(),
            RuntimeEvent::Applied { action, .. } => format!(
                "{}: {}",
                text(self.config.language, "Azione applicata", "Action applied"),
                action_name(self.config.language, action)
            ),
            RuntimeEvent::AreaApplied { .. } => {
                text(self.config.language, "Area applicata", "Area applied").to_owned()
            }
            RuntimeEvent::HotkeyConflict { action } => format!(
                "{}: {}",
                text(
                    self.config.language,
                    "Scorciatoia già in uso",
                    "Shortcut already in use"
                ),
                action_name(self.config.language, action)
            ),
            RuntimeEvent::NoActiveWindow => text(
                self.config.language,
                "Nessuna finestra idonea attiva",
                "No eligible active window",
            )
            .to_owned(),
            RuntimeEvent::AccessDenied => text(
                self.config.language,
                "Windows impedisce il controllo di una finestra elevata",
                "Windows prevents controlling an elevated window",
            )
            .to_owned(),
            RuntimeEvent::Failed(message) => format!(
                "{}: {message}",
                text(
                    self.config.language,
                    "Operazione non riuscita",
                    "Operation failed"
                )
            ),
        };
    }

    pub fn replace_config(&mut self, config: AppConfig) {
        self.config = config;
        self.cancel_shortcut_capture();
    }

    pub fn cancel_shortcut_capture(&mut self) -> bool {
        let was_active = self.shortcut_capture.take().is_some();
        self.win_keys_down = [false; 2];
        was_active
    }

    pub fn set_status(&mut self, italian: &str, english: &str) {
        self.status = text(self.config.language, italian, english).to_owned();
    }

    pub fn set_error(&mut self, message: impl std::fmt::Display) {
        self.status = format!(
            "{}: {message}",
            text(self.config.language, "Errore", "Error")
        );
    }

    fn navigation(&mut self, ui: &mut Ui, intents: &mut Vec<UiIntent>) {
        let palette = Palette::of(ui);
        ui.heading(
            RichText::new("Cuboid")
                .size(26.0)
                .strong()
                .color(palette.brand_text),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(text(
                self.config.language,
                "Gestione finestre",
                "Window manager",
            ))
            .size(12.0)
            .color(palette.text_secondary),
        );
        ui.add_space(28.0);
        let mut items: Vec<(Page, PageIcon, &str)> = vec![
            (
                Page::Actions,
                PageIcon::Panels,
                text(self.config.language, "Azioni", "Actions"),
            ),
            (
                Page::Shortcuts,
                PageIcon::Keys,
                text(self.config.language, "Scorciatoie", "Shortcuts"),
            ),
        ];
        let extension_pages = self.extension_pages();
        for (index, page) in extension_pages.iter().enumerate() {
            items.push((
                Page::Extension(index),
                page.icon,
                page.label.resolve(self.config.language),
            ));
        }
        items.push((
            Page::General,
            PageIcon::Dial,
            text(self.config.language, "Generale", "General"),
        ));
        for (page, icon, label) in items {
            if navigation_item(ui, icon, self.page == page, label).clicked() {
                self.page = page;
            }
            ui.add_space(4.0);
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if ui
                .add_sized(
                    [ui.available_width(), 34.0],
                    egui::Button::new(text(
                        self.config.language,
                        "Nascondi nella tray",
                        "Hide to tray",
                    )),
                )
                .clicked()
            {
                self.cancel_shortcut_capture();
                intents.push(UiIntent::Hide);
            }
            ui.add_space(8.0);
            let palette = Palette::of(ui);
            egui::Frame::new()
                .fill(palette.surface)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(10, 7))
                .show(ui, |ui| {
                    ui.set_max_width(NAVIGATION_WIDTH - 20.0);
                    ui.label(
                        RichText::new(&self.status)
                            .size(11.0)
                            .color(palette.text_secondary),
                    );
                });
        });
    }

    fn page_header(&self, ui: &mut Ui) {
        let extension_pages = self.extension_pages();
        let (title, description) = match self.page {
            Page::Actions => (
                LocalizedText::new("Disponi le finestre", "Arrange windows"),
                LocalizedText::new(
                    "Scegli una posizione per la finestra attiva.",
                    "Choose a position for the active window.",
                ),
            ),
            Page::Shortcuts => (
                LocalizedText::new("Scorciatoie da tastiera", "Keyboard shortcuts"),
                LocalizedText::new(
                    "Personalizza i comandi globali. Le modifiche sono immediate.",
                    "Customize global commands. Changes apply immediately.",
                ),
            ),
            Page::Extension(index) => match extension_pages.get(index) {
                Some(page) => (page.title, page.description),
                None => (LocalizedText::new("", ""), LocalizedText::new("", "")),
            },
            Page::General => (
                LocalizedText::new("Impostazioni generali", "General settings"),
                LocalizedText::new(
                    "Configura il comportamento e le preferenze di Cuboid.",
                    "Configure Cuboid behavior and preferences.",
                ),
            ),
        };
        let palette = Palette::of(ui);
        ui.heading(
            RichText::new(title.resolve(self.config.language))
                .size(28.0)
                .strong()
                .color(palette.text_primary),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(description.resolve(self.config.language))
                .size(13.0)
                .color(palette.text_secondary),
        );
    }

    fn extension_page(
        &mut self,
        ui: &mut Ui,
        index: usize,
        intents: &mut Vec<UiIntent>,
    ) -> ExtensionChange {
        let language = self.config.language;
        let Some(extension) = self.extension.as_mut() else {
            return ExtensionChange::Unchanged;
        };
        let Some(page) = extension.pages().get(index).map(|page| page.id) else {
            return ExtensionChange::Unchanged;
        };
        scrollable(ui, page, |ui| extension.show(page, ui, language, intents))
    }

    fn actions_page(&mut self, ui: &mut Ui, intents: &mut Vec<UiIntent>) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, italian, english, actions) in action_groups() {
                ui.push_id(id, |ui| {
                    ui.label(
                        RichText::new(text(self.config.language, italian, english))
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(5.0);
                    let columns = action_column_count(ui.available_width());
                    let card_width = (ui.available_width()
                        - ACTION_CARD_SPACING * (columns.saturating_sub(1) as f32))
                        / columns as f32;
                    egui::Grid::new("cards")
                        .num_columns(columns)
                        .spacing([ACTION_CARD_SPACING, ACTION_CARD_SPACING])
                        .show(ui, |ui| {
                            for (index, action) in actions.iter().copied().enumerate() {
                                if action_card(
                                    ui,
                                    card_width,
                                    action,
                                    self.config.language,
                                    shortcut_for(
                                        &self.config.hotkeys,
                                        action,
                                        self.config.language,
                                    ),
                                )
                                .clicked()
                                {
                                    intents.push(UiIntent::Apply(action));
                                }
                                if (index + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                            if actions.len() % columns != 0 {
                                ui.end_row();
                            }
                        });
                    ui.add_space(14.0);
                });
            }
        });
    }

    fn shortcuts_page(&mut self, ui: &mut Ui, content_bottom: f32) -> bool {
        let language = self.config.language;
        let mut changed = self.capture_shortcut(ui);
        let mut remove = None;
        let toolbar_height = ui.spacing().interact_size.y;
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), toolbar_height),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                if ui
                    .add_enabled(
                        self.shortcut_capture.is_none(),
                        egui::Button::new(text(language, "Aggiungi scorciatoia", "Add shortcut")),
                    )
                    .clicked()
                {
                    let action = Action::ALL
                        .into_iter()
                        .find(|action| {
                            !self
                                .config
                                .hotkeys
                                .iter()
                                .any(|binding| binding.action == *action)
                        })
                        .unwrap_or(Action::LeftHalf);
                    self.shortcut_capture = Some(ShortcutCapture::New(action));
                }
            },
        );
        ui.add_space(8.0);

        if let Some(ShortcutCapture::New(mut action)) = self.shortcut_capture {
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("new-shortcut-action")
                        .width(190.0)
                        .selected_text(action_name(language, action))
                        .show_ui(ui, |ui| {
                            for candidate in Action::ALL {
                                ui.selectable_value(
                                    &mut action,
                                    candidate,
                                    action_name(language, candidate),
                                );
                            }
                        });
                    ui.label(text(
                        language,
                        "Premi la combinazione desiderata…",
                        "Press the desired shortcut…",
                    ));
                    if ui
                        .small_button(text(language, "Annulla", "Cancel"))
                        .clicked()
                    {
                        self.shortcut_capture = None;
                        self.win_keys_down = [false; 2];
                    } else {
                        self.shortcut_capture = Some(ShortcutCapture::New(action));
                    }
                });
            });
            ui.add_space(8.0);
        }

        let scroll_height = (content_bottom - ui.available_rect_before_wrap().top()).max(0.0);
        egui::ScrollArea::vertical()
            .max_height(scroll_height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let columns = shortcut_column_count(ui.available_width());
                let card_width = (ui.available_width()
                    - SHORTCUT_CARD_SPACING * (columns.saturating_sub(1) as f32))
                    / columns as f32;
                egui::Grid::new("shortcut-cards")
                    .num_columns(columns)
                    .spacing([SHORTCUT_CARD_SPACING, SHORTCUT_CARD_SPACING])
                    .show(ui, |ui| {
                        for index in 0..self.config.hotkeys.len() {
                            ui.allocate_ui_with_layout(
                                egui::vec2(card_width, 66.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(card_width);
                                    ui.group(|ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            let binding = &mut self.config.hotkeys[index];
                                            egui::ComboBox::from_id_salt(("action", index))
                                                .width(170.0)
                                                .selected_text(action_name(
                                                    language,
                                                    binding.action,
                                                ))
                                                .show_ui(ui, |ui| {
                                                    for action in Action::ALL {
                                                        changed |= ui
                                                            .selectable_value(
                                                                &mut binding.action,
                                                                action,
                                                                action_name(language, action),
                                                            )
                                                            .changed();
                                                    }
                                                });
                                            let capture_active = self.shortcut_capture
                                                == Some(ShortcutCapture::Existing(index));
                                            let label = if capture_active {
                                                text(
                                                    language,
                                                    "Premi la nuova combinazione…",
                                                    "Press the new shortcut…",
                                                )
                                                .to_owned()
                                            } else {
                                                format_shortcut(binding, language)
                                            };
                                            let shortcut_width =
                                                (ui.available_width() - 36.0).max(120.0);
                                            if ui
                                                .add_sized(
                                                    [shortcut_width, 30.0],
                                                    egui::Button::new(label),
                                                )
                                                .clicked()
                                            {
                                                self.shortcut_capture =
                                                    Some(ShortcutCapture::Existing(index));
                                            }
                                            if ui
                                                .add_sized([28.0, 30.0], egui::Button::new("×"))
                                                .clicked()
                                            {
                                                remove = Some(index);
                                            }
                                        });
                                    });
                                },
                            );
                            if (index + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                        if self.config.hotkeys.len() % columns != 0 {
                            ui.end_row();
                        }
                    });
            });

        if let Some(index) = remove {
            self.config.hotkeys.remove(index);
            self.shortcut_capture = None;
            self.win_keys_down = [false; 2];
            changed = true;
        }
        changed
    }

    fn capture_shortcut(&mut self, ui: &mut Ui) -> bool {
        let Some(target) = self.shortcut_capture else {
            return false;
        };

        let events = ui.input(|input| input.events.clone());
        ui.input_mut(|input| {
            input
                .events
                .retain(|event| !matches!(event, Event::Key { .. } | Event::ModifiersChanged(_)));
        });
        let mut captured = None;
        for event in events {
            let Event::Key {
                key,
                physical_key,
                pressed,
                repeat,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            match key {
                Key::SuperLeft => self.win_keys_down[0] = pressed,
                Key::SuperRight => self.win_keys_down[1] = pressed,
                Key::ShiftLeft
                | Key::ShiftRight
                | Key::ControlLeft
                | Key::ControlRight
                | Key::AltLeft
                | Key::AltRight => {}
                _ if pressed && !repeat => {
                    if let Some(virtual_key) = virtual_key_for(physical_key.unwrap_or(key)) {
                        let mut modifier_bits = modifier_bits(modifiers);
                        if self.win_keys_down.iter().any(|pressed| *pressed) {
                            modifier_bits |= MOD_WIN;
                        }
                        captured = Some(HotkeyBinding {
                            action: match target {
                                ShortcutCapture::Existing(index) => {
                                    self.config.hotkeys[index].action
                                }
                                ShortcutCapture::New(action) => action,
                            },
                            modifiers: modifier_bits,
                            virtual_key,
                        });
                    } else {
                        self.status = text(
                            self.config.language,
                            "Questo tasto non è supportato come scorciatoia globale",
                            "This key is not supported as a global shortcut",
                        )
                        .to_owned();
                    }
                }
                _ => {}
            }
        }

        let Some(binding) = captured else {
            return false;
        };
        let replaced_index = match target {
            ShortcutCapture::Existing(index) => Some(index),
            ShortcutCapture::New(_) => None,
        };
        let duplicate = self
            .config
            .hotkeys
            .iter()
            .enumerate()
            .any(|(index, existing)| {
                Some(index) != replaced_index
                    && existing.modifiers == binding.modifiers
                    && existing.virtual_key == binding.virtual_key
            });
        if duplicate {
            self.status = format!(
                "{}: {}",
                text(
                    self.config.language,
                    "Combinazione già assegnata",
                    "Shortcut already assigned"
                ),
                format_shortcut(&binding, self.config.language)
            );
            return false;
        }

        match target {
            ShortcutCapture::Existing(index) => self.config.hotkeys[index] = binding,
            ShortcutCapture::New(_) => self.config.hotkeys.push(binding),
        }
        self.shortcut_capture = None;
        self.win_keys_down = [false; 2];
        true
    }

    fn general_page(&mut self, ui: &mut Ui, intents: &mut Vec<UiIntent>) -> bool {
        let mut changed = false;
        let language = self.config.language;
        surface_frame(ui).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::Grid::new("general-settings")
                .num_columns(2)
                .spacing([24.0, 14.0])
                .show(ui, |ui| {
                    ui.label(text(language, "Lingua", "Language"));
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(match self.config.language {
                            Language::Italian => "Italiano",
                            Language::English => "English",
                        })
                        .show_ui(ui, |ui| {
                            changed |= ui
                                .selectable_value(
                                    &mut self.config.language,
                                    Language::Italian,
                                    "Italiano",
                                )
                                .changed();
                            changed |= ui
                                .selectable_value(
                                    &mut self.config.language,
                                    Language::English,
                                    "English",
                                )
                                .changed();
                        });
                    ui.end_row();

                    ui.label(text(language, "Snap trascinando", "Drag snapping"));
                    changed |= ui
                        .checkbox(
                            &mut self.config.drag_snap_enabled,
                            text(language, "Abilitato", "Enabled"),
                        )
                        .changed();
                    ui.end_row();

                    ui.label(text(language, "Gap", "Gap"));
                    changed |= setting_slider(ui, &mut self.config.gap, 0..=64, " px");
                    ui.end_row();

                    ui.label(text(language, "Soglia snap", "Snap threshold"));
                    changed |=
                        setting_slider(ui, &mut self.config.snap_threshold, 100..=1_500, " /10000");
                    ui.end_row();

                    ui.label(text(language, "Avvio automatico", "Launch at login"));
                    changed |= ui
                        .checkbox(
                            &mut self.config.launch_at_login,
                            text(language, "Avvia Cuboid", "Start Cuboid"),
                        )
                        .changed();
                    ui.end_row();
                });
        });

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if ui
                .button(text(
                    language,
                    "Importa configurazione",
                    "Import configuration",
                ))
                .clicked()
            {
                intents.push(UiIntent::Import);
            }
            if ui
                .button(text(
                    language,
                    "Esporta configurazione",
                    "Export configuration",
                ))
                .clicked()
            {
                intents.push(UiIntent::Export(self.config.clone()));
            }
        });
        changed
    }
}

/// A slider on a settings card, labelled for accessibility clients and wide
/// enough to aim at a value.
///
/// The handle carries the accent, so it stays apart from the rail: egui draws
/// both from `widgets.inactive`.
fn setting_slider<Num: egui::emath::Numeric>(
    ui: &mut Ui,
    value: &mut Num,
    range: std::ops::RangeInclusive<Num>,
    suffix: &str,
) -> bool {
    let accent = Palette::of(ui).brand_fill;
    ui.scope(|ui| {
        ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(2.0, accent);
        ui.spacing_mut().slider_width = theme::SLIDER_WIDTH;
        ui.add(egui::Slider::new(value, range).suffix(suffix))
            .changed()
    })
    .inner
}

fn scrollable<R>(ui: &mut Ui, id: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, add_contents)
        .inner
}

fn navigation_item(ui: &mut Ui, icon: PageIcon, selected: bool, label: &str) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });

    let palette = Palette::of(ui);
    let fill = if selected {
        palette.brand_soft
    } else if response.hovered() {
        theme::mix(palette.surface_alt, palette.brand_soft, 0.6)
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 8.0, fill);
    if selected {
        let indicator = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 2.0, rect.center().y),
            egui::vec2(3.0, 18.0),
        );
        painter.rect_filled(indicator, 2.0, palette.brand_fill);
    }

    let foreground = if selected {
        palette.brand_text
    } else if response.hovered() {
        palette.text_primary
    } else {
        palette.text_secondary
    };
    let icon_bounds = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 18.0, rect.center().y),
        egui::vec2(16.0, 16.0),
    );
    paint_navigation_icon(&painter, icon_bounds, icon, foreground);
    painter.text(
        egui::pos2(rect.left() + 38.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        foreground,
    );
    response
}

fn paint_navigation_icon(
    painter: &egui::Painter,
    bounds: egui::Rect,
    icon: PageIcon,
    color: Color32,
) {
    let stroke = Stroke::new(1.4, color);
    match icon {
        PageIcon::Panels => {
            painter.rect_stroke(bounds, 3.0, stroke, StrokeKind::Inside);
            painter.line_segment(
                [
                    egui::pos2(bounds.center().x, bounds.top()),
                    egui::pos2(bounds.center().x, bounds.bottom()),
                ],
                stroke,
            );
        }
        PageIcon::Keys => {
            painter.rect_stroke(bounds, 3.0, stroke, StrokeKind::Inside);
            for offset in [4.0, 8.0, 12.0] {
                painter.circle_filled(
                    egui::pos2(bounds.left() + offset, bounds.center().y),
                    1.1,
                    color,
                );
            }
        }
        PageIcon::Grid => {
            for row in 0..2 {
                for column in 0..2 {
                    let min = bounds.min + egui::vec2(column as f32 * 9.0, row as f32 * 9.0);
                    painter.rect_stroke(
                        egui::Rect::from_min_size(min, egui::vec2(7.0, 7.0)),
                        2.0,
                        stroke,
                        StrokeKind::Inside,
                    );
                }
            }
        }
        PageIcon::Stack => {
            let back = bounds.translate(egui::vec2(3.0, -3.0)).shrink(1.5);
            painter.rect_stroke(back, 2.0, stroke, StrokeKind::Inside);
            painter.rect_filled(
                bounds.translate(egui::vec2(-2.0, 2.0)).shrink(1.5),
                2.0,
                color,
            );
        }
        PageIcon::Dial => {
            painter.circle_stroke(bounds.center(), 6.5, stroke);
            painter.circle_filled(bounds.center(), 2.0, color);
        }
    }
}

/// A framed surface for grouped settings, shared with host-provided pages.
pub fn surface_frame(ui: &Ui) -> egui::Frame {
    let palette = Palette::of(ui);
    egui::Frame::group(ui.style())
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(14))
}

type ActionGroup = (&'static str, &'static str, &'static str, &'static [Action]);

fn action_column_count(available_width: f32) -> usize {
    (((available_width + ACTION_CARD_SPACING) / (ACTION_CARD_MIN_WIDTH + ACTION_CARD_SPACING))
        .floor() as usize)
        .clamp(1, ACTION_MAX_COLUMNS)
}

fn action_groups() -> [ActionGroup; 5] {
    [
        (
            "halves",
            "Metà",
            "Halves",
            &[
                Action::LeftHalf,
                Action::RightHalf,
                Action::TopHalf,
                Action::BottomHalf,
                Action::CenterHalf,
            ],
        ),
        (
            "corners",
            "Angoli",
            "Corners",
            &[
                Action::TopLeft,
                Action::TopRight,
                Action::BottomLeft,
                Action::BottomRight,
            ],
        ),
        (
            "thirds",
            "Terzi",
            "Thirds",
            &[
                Action::FirstThird,
                Action::CenterThird,
                Action::LastThird,
                Action::FirstTwoThirds,
                Action::CenterTwoThirds,
                Action::LastTwoThirds,
            ],
        ),
        (
            "window",
            "Dimensione e stato",
            "Size and state",
            &[
                Action::Maximize,
                Action::AlmostMaximize,
                Action::MaximizeHeight,
                Action::Center,
                Action::Grow,
                Action::Shrink,
                Action::Restore,
            ],
        ),
        (
            "move",
            "Monitor e movimento",
            "Displays and movement",
            &[
                Action::PreviousMonitor,
                Action::NextMonitor,
                Action::MoveLeft,
                Action::MoveRight,
                Action::MoveUp,
                Action::MoveDown,
            ],
        ),
    ]
}

fn shortcut_column_count(available_width: f32) -> usize {
    (((available_width + SHORTCUT_CARD_SPACING) / (SHORTCUT_CARD_MIN_WIDTH + SHORTCUT_CARD_SPACING))
        .floor() as usize)
        .clamp(1, SHORTCUT_MAX_COLUMNS)
}

fn shortcut_for(bindings: &[HotkeyBinding], action: Action, language: Language) -> Option<String> {
    bindings
        .iter()
        .find(|binding| binding.action == action)
        .map(|binding| format_shortcut(binding, language))
}

fn format_shortcut(binding: &HotkeyBinding, language: Language) -> String {
    let mut parts = Vec::new();
    if binding.modifiers & MOD_CONTROL != 0 {
        parts.push("Ctrl".to_owned());
    }
    if binding.modifiers & MOD_ALT != 0 {
        parts.push("Alt".to_owned());
    }
    if binding.modifiers & MOD_SHIFT != 0 {
        parts.push("Shift".to_owned());
    }
    if binding.modifiers & MOD_WIN != 0 {
        parts.push("Win".to_owned());
    }
    parts.push(key_name(binding.virtual_key, language));
    parts.join(" + ")
}

fn key_name(virtual_key: u32, language: Language) -> String {
    match virtual_key {
        0x08 => "Backspace".to_owned(),
        0x09 => "Tab".to_owned(),
        0x0D => text(language, "Invio", "Enter").to_owned(),
        0x13 => text(language, "Pausa", "Pause").to_owned(),
        0x14 => text(language, "Bloc Maiusc", "Caps Lock").to_owned(),
        0x1B => "Esc".to_owned(),
        0x20 => text(language, "Spazio", "Space").to_owned(),
        0x21 => text(language, "Pagina su", "Page Up").to_owned(),
        0x22 => text(language, "Pagina giù", "Page Down").to_owned(),
        0x23 => text(language, "Fine", "End").to_owned(),
        0x24 => "Home".to_owned(),
        0x25 => text(language, "Freccia sinistra", "Left Arrow").to_owned(),
        0x26 => text(language, "Freccia su", "Up Arrow").to_owned(),
        0x27 => text(language, "Freccia destra", "Right Arrow").to_owned(),
        0x28 => text(language, "Freccia giù", "Down Arrow").to_owned(),
        0x2C => text(language, "Stamp", "Print Screen").to_owned(),
        0x2D => text(language, "Ins", "Insert").to_owned(),
        0x2E => text(language, "Canc", "Delete").to_owned(),
        0x60..=0x69 => format!("Numpad {}", virtual_key - 0x60),
        0x6A => text(language, "Numpad Moltiplica", "Numpad Multiply").to_owned(),
        0x6B => text(language, "Numpad Più", "Numpad Add").to_owned(),
        0x6D => text(language, "Numpad Meno", "Numpad Subtract").to_owned(),
        0x6E => text(language, "Numpad Decimale", "Numpad Decimal").to_owned(),
        0x6F => text(language, "Numpad Dividi", "Numpad Divide").to_owned(),
        0x70..=0x87 => format!("F{}", virtual_key - 0x6F),
        0x90 => "Num Lock".to_owned(),
        0x91 => "Scroll Lock".to_owned(),
        0xA6 => text(language, "Browser indietro", "Browser Back").to_owned(),
        0xA7 => text(language, "Browser avanti", "Browser Forward").to_owned(),
        0xAD => text(language, "Disattiva audio", "Volume Mute").to_owned(),
        0xAE => text(language, "Volume giù", "Volume Down").to_owned(),
        0xAF => text(language, "Volume su", "Volume Up").to_owned(),
        0xB0 => text(language, "Traccia successiva", "Next Track").to_owned(),
        0xB1 => text(language, "Traccia precedente", "Previous Track").to_owned(),
        0xB2 => text(language, "Arresta contenuto", "Stop Media").to_owned(),
        0xB3 => text(language, "Riproduci/Pausa", "Play/Pause").to_owned(),
        0xBA => ";".to_owned(),
        0xBB => "+".to_owned(),
        0xBC => ",".to_owned(),
        0xBD => "-".to_owned(),
        0xBE => ".".to_owned(),
        0xBF => "/".to_owned(),
        0xC0 => "`".to_owned(),
        0xDB => "[".to_owned(),
        0xDC => "\\".to_owned(),
        0xDD => "]".to_owned(),
        0xDE => "'".to_owned(),
        0xE2 => text(language, "Tasto ISO", "ISO key").to_owned(),
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(virtual_key)
            .map(|value| value.to_string())
            .unwrap_or_else(|| unknown_key_name(virtual_key, language)),
        _ => unknown_key_name(virtual_key, language),
    }
}

fn unknown_key_name(virtual_key: u32, language: Language) -> String {
    format!(
        "{} (0x{virtual_key:02X})",
        text(language, "Tasto sconosciuto", "Unknown key")
    )
}

fn modifier_bits(modifiers: Modifiers) -> u32 {
    let mut bits = 0;
    if modifiers.ctrl {
        bits |= MOD_CONTROL;
    }
    if modifiers.alt {
        bits |= MOD_ALT;
    }
    if modifiers.shift {
        bits |= MOD_SHIFT;
    }
    bits
}

fn virtual_key_for(key: Key) -> Option<u32> {
    Some(match key {
        Key::ArrowDown => 0x28,
        Key::ArrowLeft => 0x25,
        Key::ArrowRight => 0x27,
        Key::ArrowUp => 0x26,
        Key::Escape => 0x1B,
        Key::Tab => 0x09,
        Key::Backspace => 0x08,
        Key::Enter => 0x0D,
        Key::Space => 0x20,
        Key::Insert => 0x2D,
        Key::Delete => 0x2E,
        Key::Home => 0x24,
        Key::End => 0x23,
        Key::PageUp => 0x21,
        Key::PageDown => 0x22,
        Key::Colon | Key::Semicolon => 0xBA,
        Key::Comma => 0xBC,
        Key::Backslash | Key::Pipe => 0xDC,
        Key::Slash | Key::Questionmark => 0xBF,
        Key::Exclamationmark => 0x31,
        Key::OpenBracket | Key::OpenCurlyBracket => 0xDB,
        Key::CloseBracket | Key::CloseCurlyBracket => 0xDD,
        Key::Backtick => 0xC0,
        Key::Minus => 0xBD,
        Key::Period => 0xBE,
        Key::Plus | Key::Equals => 0xBB,
        Key::Quote => 0xDE,
        Key::Num0 => 0x30,
        Key::Num1 => 0x31,
        Key::Num2 => 0x32,
        Key::Num3 => 0x33,
        Key::Num4 => 0x34,
        Key::Num5 => 0x35,
        Key::Num6 => 0x36,
        Key::Num7 => 0x37,
        Key::Num8 => 0x38,
        Key::Num9 => 0x39,
        Key::A => 0x41,
        Key::B => 0x42,
        Key::C => 0x43,
        Key::D => 0x44,
        Key::E => 0x45,
        Key::F => 0x46,
        Key::G => 0x47,
        Key::H => 0x48,
        Key::I => 0x49,
        Key::J => 0x4A,
        Key::K => 0x4B,
        Key::L => 0x4C,
        Key::M => 0x4D,
        Key::N => 0x4E,
        Key::O => 0x4F,
        Key::P => 0x50,
        Key::Q => 0x51,
        Key::R => 0x52,
        Key::S => 0x53,
        Key::T => 0x54,
        Key::U => 0x55,
        Key::V => 0x56,
        Key::W => 0x57,
        Key::X => 0x58,
        Key::Y => 0x59,
        Key::Z => 0x5A,
        Key::F1 => 0x70,
        Key::F2 => 0x71,
        Key::F3 => 0x72,
        Key::F4 => 0x73,
        Key::F5 => 0x74,
        Key::F6 => 0x75,
        Key::F7 => 0x76,
        Key::F8 => 0x77,
        Key::F9 => 0x78,
        Key::F10 => 0x79,
        Key::F11 => 0x7A,
        Key::F12 => 0x7B,
        Key::F13 => 0x7C,
        Key::F14 => 0x7D,
        Key::F15 => 0x7E,
        Key::F16 => 0x7F,
        Key::F17 => 0x80,
        Key::F18 => 0x81,
        Key::F19 => 0x82,
        Key::F20 => 0x83,
        Key::F21 => 0x84,
        Key::F22 => 0x85,
        Key::F23 => 0x86,
        Key::F24 => 0x87,
        Key::BrowserBack => 0xA6,
        Key::IntlBackslash => 0xE2,
        Key::Copy
        | Key::Cut
        | Key::Paste
        | Key::F25
        | Key::F26
        | Key::F27
        | Key::F28
        | Key::F29
        | Key::F30
        | Key::F31
        | Key::F32
        | Key::F33
        | Key::F34
        | Key::F35
        | Key::ShiftLeft
        | Key::ShiftRight
        | Key::ControlLeft
        | Key::ControlRight
        | Key::AltLeft
        | Key::AltRight
        | Key::SuperLeft
        | Key::SuperRight => return None,
    })
}

fn action_card(
    ui: &mut Ui,
    width: f32,
    action: Action,
    language: Language,
    shortcut: Option<String>,
) -> egui::Response {
    let name = action_name(language, action);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 82.0), Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name));
    let palette = Palette::of(ui);
    let (fill, border) = if response.is_pointer_button_down_on() {
        (palette.brand_soft, palette.brand_fill)
    } else if response.hovered() {
        (
            theme::mix(palette.surface, palette.brand_soft, 0.7),
            palette.brand_fill,
        )
    } else {
        (palette.surface, palette.border)
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 10.0, fill);
    painter.rect_stroke(rect, 10.0, Stroke::new(1.0, border), StrokeKind::Inside);

    let preview = egui::Rect::from_min_size(
        rect.left_center() + egui::vec2(14.0, -22.0),
        egui::vec2(58.0, 44.0),
    );
    paint_action_preview(
        &painter,
        preview,
        action,
        palette.brand_text,
        palette.text_secondary,
    );
    painter.text(
        egui::pos2(preview.right() + 14.0, rect.center().y - 9.0),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(14.0),
        palette.text_on(fill),
    );
    painter.text(
        egui::pos2(preview.right() + 14.0, rect.center().y + 13.0),
        egui::Align2::LEFT_CENTER,
        shortcut.unwrap_or_else(|| text(language, "Non assegnata", "Unassigned").to_owned()),
        egui::FontId::proportional(11.0),
        palette.text_secondary,
    );
    response
}

fn paint_action_preview(
    painter: &egui::Painter,
    bounds: egui::Rect,
    action: Action,
    accent: Color32,
    foreground: Color32,
) {
    if matches!(action, Action::NextMonitor | Action::PreviousMonitor) {
        let first = egui::Rect::from_min_size(bounds.min, egui::vec2(22.0, 30.0));
        let second = egui::Rect::from_min_size(
            egui::pos2(bounds.right() - 22.0, bounds.top()),
            egui::vec2(22.0, 30.0),
        );
        painter.rect_stroke(first, 3.0, Stroke::new(1.0, foreground), StrokeKind::Inside);
        painter.rect_stroke(
            second,
            3.0,
            Stroke::new(1.0, foreground),
            StrokeKind::Inside,
        );
        let from = if action == Action::NextMonitor {
            first.right_center()
        } else {
            second.left_center()
        };
        let to = if action == Action::NextMonitor {
            second.left_center()
        } else {
            first.right_center()
        };
        paint_arrow(painter, from, to, accent);
        return;
    }

    let monitor = bounds.shrink2(egui::vec2(2.0, 4.0));
    painter.rect_stroke(
        monitor,
        4.0,
        Stroke::new(1.0, foreground),
        StrokeKind::Inside,
    );
    if action == Action::Restore {
        let back = egui::Rect::from_min_max(
            monitor.min + egui::vec2(7.0, 5.0),
            monitor.max - egui::vec2(7.0, 9.0),
        );
        let front = back.translate(egui::vec2(-4.0, 4.0));
        painter.rect_stroke(back, 2.0, Stroke::new(1.0, accent), StrokeKind::Inside);
        painter.rect_stroke(front, 2.0, Stroke::new(1.5, accent), StrokeKind::Inside);
        return;
    }

    let area = match action {
        Action::LeftHalf => [0.0, 0.0, 0.5, 1.0],
        Action::RightHalf => [0.5, 0.0, 1.0, 1.0],
        Action::TopHalf => [0.0, 0.0, 1.0, 0.5],
        Action::BottomHalf => [0.0, 0.5, 1.0, 1.0],
        Action::TopLeft => [0.0, 0.0, 0.5, 0.5],
        Action::TopRight => [0.5, 0.0, 1.0, 0.5],
        Action::BottomLeft => [0.0, 0.5, 0.5, 1.0],
        Action::BottomRight => [0.5, 0.5, 1.0, 1.0],
        Action::FirstThird => [0.0, 0.0, 1.0 / 3.0, 1.0],
        Action::CenterThird => [1.0 / 3.0, 0.0, 2.0 / 3.0, 1.0],
        Action::LastThird => [2.0 / 3.0, 0.0, 1.0, 1.0],
        Action::FirstTwoThirds => [0.0, 0.0, 2.0 / 3.0, 1.0],
        Action::CenterTwoThirds => [1.0 / 6.0, 0.0, 5.0 / 6.0, 1.0],
        Action::LastTwoThirds => [1.0 / 3.0, 0.0, 1.0, 1.0],
        Action::CenterHalf => [0.25, 0.0, 0.75, 1.0],
        Action::Maximize => [0.0, 0.0, 1.0, 1.0],
        Action::AlmostMaximize => [0.06, 0.06, 0.94, 0.94],
        Action::MaximizeHeight => [0.25, 0.0, 0.75, 1.0],
        Action::Center => [0.25, 0.22, 0.75, 0.78],
        Action::Grow => [0.14, 0.12, 0.86, 0.88],
        Action::Shrink => [0.3, 0.28, 0.7, 0.72],
        Action::MoveLeft | Action::MoveRight | Action::MoveUp | Action::MoveDown => {
            [0.28, 0.24, 0.72, 0.76]
        }
        Action::NextMonitor | Action::PreviousMonitor | Action::Restore => unreachable!(),
    };
    let highlighted = normalized_preview_rect(monitor, area);
    painter.rect_filled(highlighted, 2.0, accent.gamma_multiply(0.72));
    painter.rect_stroke(
        highlighted,
        2.0,
        Stroke::new(1.0, accent),
        StrokeKind::Inside,
    );

    let direction = match action {
        Action::MoveLeft => Some(egui::vec2(-8.0, 0.0)),
        Action::MoveRight => Some(egui::vec2(8.0, 0.0)),
        Action::MoveUp => Some(egui::vec2(0.0, -7.0)),
        Action::MoveDown => Some(egui::vec2(0.0, 7.0)),
        _ => None,
    };
    if let Some(direction) = direction {
        paint_arrow(
            painter,
            highlighted.center() - direction * 0.5,
            highlighted.center() + direction * 0.5,
            foreground,
        );
    }
}

fn normalized_preview_rect(bounds: egui::Rect, area: [f32; 4]) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(
            bounds.left() + bounds.width() * area[0],
            bounds.top() + bounds.height() * area[1],
        ),
        egui::pos2(
            bounds.left() + bounds.width() * area[2],
            bounds.top() + bounds.height() * area[3],
        ),
    )
    .shrink(1.5)
}

fn paint_arrow(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, color: Color32) {
    let direction = (to - from).normalized();
    let normal = egui::vec2(-direction.y, direction.x);
    painter.line_segment([from, to], Stroke::new(1.5, color));
    painter.line_segment(
        [to, to - direction * 4.0 + normal * 3.0],
        Stroke::new(1.5, color),
    );
    painter.line_segment(
        [to, to - direction * 4.0 - normal * 3.0],
        Stroke::new(1.5, color),
    );
}

/// Builds an identifier that does not collide with `existing`, shared with
/// host-provided pages.
pub fn unique_id<'a>(prefix: &str, existing: impl Iterator<Item = &'a str>) -> String {
    unique_value(|number| format!("{prefix}-{number}"), existing)
}

/// Builds a value from a numbered pattern that does not collide with `existing`.
///
/// The comparison is made on the complete value, so a page that decorates the
/// number (`application-1.exe`) must decorate the candidates too. Values that
/// differ only in case count as taken.
pub fn unique_value<'a>(
    candidate: impl Fn(usize) -> String,
    existing: impl Iterator<Item = &'a str>,
) -> String {
    let existing = existing
        .map(|value| value.trim().to_lowercase())
        .collect::<Vec<_>>();
    let attempts = existing.len() + 1;
    (1..=attempts)
        .map(&candidate)
        .find(|value| !existing.contains(&value.trim().to_lowercase()))
        .unwrap_or_else(|| candidate(attempts + 1))
}

/// Picks the string matching the interface language.
pub fn text<'a>(language: Language, italian: &'a str, english: &'a str) -> &'a str {
    match language {
        Language::Italian => italian,
        Language::English => english,
    }
}

/// The localized name of a standard action.
pub fn action_name(language: Language, action: Action) -> &'static str {
    let italian = match action {
        Action::LeftHalf => "Metà sinistra",
        Action::RightHalf => "Metà destra",
        Action::TopHalf => "Metà superiore",
        Action::BottomHalf => "Metà inferiore",
        Action::TopLeft => "Alto sinistra",
        Action::TopRight => "Alto destra",
        Action::BottomLeft => "Basso sinistra",
        Action::BottomRight => "Basso destra",
        Action::FirstThird => "Primo terzo",
        Action::CenterThird => "Terzo centrale",
        Action::LastThird => "Ultimo terzo",
        Action::FirstTwoThirds => "Primi 2/3",
        Action::CenterTwoThirds => "2/3 centrali",
        Action::LastTwoThirds => "Ultimi 2/3",
        Action::CenterHalf => "Metà centrale",
        Action::Center => "Centra",
        Action::AlmostMaximize => "Quasi massimizza",
        Action::MaximizeHeight => "Massimizza altezza",
        Action::Grow => "Ingrandisci",
        Action::Shrink => "Riduci",
        Action::MoveLeft => "Sposta a sinistra",
        Action::MoveRight => "Sposta a destra",
        Action::MoveUp => "Sposta in alto",
        Action::MoveDown => "Sposta in basso",
        Action::Maximize => "Massimizza",
        Action::Restore => "Ripristina",
        Action::NextMonitor => "Monitor successivo",
        Action::PreviousMonitor => "Monitor precedente",
    };
    let english = match action {
        Action::LeftHalf => "Left half",
        Action::RightHalf => "Right half",
        Action::TopHalf => "Top half",
        Action::BottomHalf => "Bottom half",
        Action::TopLeft => "Top left",
        Action::TopRight => "Top right",
        Action::BottomLeft => "Bottom left",
        Action::BottomRight => "Bottom right",
        Action::FirstThird => "First third",
        Action::CenterThird => "Center third",
        Action::LastThird => "Last third",
        Action::FirstTwoThirds => "First 2/3",
        Action::CenterTwoThirds => "Center 2/3",
        Action::LastTwoThirds => "Last 2/3",
        Action::CenterHalf => "Center half",
        Action::Center => "Center",
        Action::AlmostMaximize => "Almost maximize",
        Action::MaximizeHeight => "Maximize height",
        Action::Grow => "Grow",
        Action::Shrink => "Shrink",
        Action::MoveLeft => "Move left",
        Action::MoveRight => "Move right",
        Action::MoveUp => "Move up",
        Action::MoveDown => "Move down",
        Action::Maximize => "Maximize",
        Action::Restore => "Restore",
        Action::NextMonitor => "Next monitor",
        Action::PreviousMonitor => "Previous monitor",
    };
    text(language, italian, english)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paints one frame of a settings slider with the application style and
    /// collects the rectangles it produced.
    fn slider_rectangles() -> Vec<(egui::Rect, Color32)> {
        let ctx = egui::Context::default();
        // Both themes are set to the light one, so the test reads the colours
        // it asserts on whichever theme the context resolves to.
        ctx.set_style_of(egui::Theme::Light, theme::style(false));
        ctx.set_style_of(egui::Theme::Dark, theme::style(false));
        let mut value: u16 = 0;

        ctx.begin_pass(egui::RawInput {
            // Without a screen, egui considers every rectangle off-screen and
            // paints nothing at all.
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        });
        // A plain background layer: no shadow and no fade-in to read through.
        let mut ui = egui::Ui::new(
            ctx.clone(),
            egui::Id::new("slider-under-test"),
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 200.0),
            )),
        );
        setting_slider(&mut ui, &mut value, 0..=64, " px");
        let mut output = ctx.end_pass();
        // The test never uploads textures to a GPU, so the deltas are dropped
        // deliberately instead of being left unapplied.
        output.textures_delta.clear();

        let mut rectangles = Vec::new();
        for clipped in output.shapes {
            collect_rectangles(clipped.shape, &mut rectangles);
        }
        rectangles
    }

    /// Rectangles reach the painter nested inside the shapes of the containers
    /// that produced them, so they are collected recursively.
    fn collect_rectangles(shape: egui::Shape, rectangles: &mut Vec<(egui::Rect, Color32)>) {
        match shape {
            egui::Shape::Rect(rect) => rectangles.push((rect.rect, rect.fill)),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect_rectangles(shape, rectangles);
                }
            }
            _ => {}
        }
    }

    /// egui paints the rail of a slider with `widgets.inactive.bg_fill`. Filling
    /// it with the colour of the card underneath left the settings page showing
    /// a handle floating over an invisible rail.
    #[test]
    fn the_rail_of_a_slider_is_painted_in_its_own_colour() {
        let rectangles = slider_rectangles();
        // The rail spans the whole slider and is only a few pixels tall, which
        // tells it apart from the card painted behind it.
        let rail = rectangles.iter().copied().find(|(rect, fill)| {
            rect.width() >= theme::SLIDER_WIDTH - 1.0
                && (4.0..=8.0).contains(&rect.height())
                && fill.a() == 255
        });

        let (rect, fill) =
            rail.unwrap_or_else(|| panic!("no rail was painted; rectangles: {rectangles:#?}"));
        assert_eq!(fill, Palette::LIGHT.track);
        assert_ne!(
            fill,
            Palette::LIGHT.surface,
            "a rail the colour of the card it sits on is invisible"
        );
        assert!(rect.width() >= theme::SLIDER_WIDTH - 1.0);
    }
}
