use std::{cell::RefCell, rc::Rc};

use cuboid_core::{Action, AppConfig, Language, NormalizedRect, RuntimeEvent};
use cuboid_ui::{
    ExtensionChange, ExtensionPage, LocalizedText, PageIcon, UiExtension, UiIntent, UiState,
};
use egui::{Key, Modifiers};
use egui_kittest::{Harness, kittest::Queryable};

#[test]
fn action_button_emits_apply_intent() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);

    harness.get_by_label("Metà sinistra").click();
    harness.run();

    assert!(
        intents
            .borrow()
            .iter()
            .any(|intent| matches!(intent, UiIntent::Apply(Action::LeftHalf)))
    );
}

#[test]
fn rectangle_action_cards_expose_new_standard_layouts() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let harness = ui_harness(&state, &intents);

    let _card = harness.get_by_label("2/3 centrali");
}

#[test]
fn sidebar_brand_has_a_clear_vertical_hierarchy() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let harness = ui_harness(&state, &intents);

    let title = harness.get_by_label("Cuboid").rect();
    let subtitle = harness.get_by_label("Gestione finestre").rect();
    let first_navigation_item = harness.get_by_label("Azioni").rect();

    assert!(title.bottom() < subtitle.top());
    assert!(subtitle.bottom() < first_navigation_item.top());
    assert!((title.left() - subtitle.left()).abs() < 1.0);
}

#[test]
fn general_settings_emit_valid_configuration_and_file_intents() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);
    harness.get_by_label("Generale").click();
    harness.run();

    harness.get_by_label("Abilitato").click();
    harness.run();
    harness.get_by_label("Importa configurazione").click();
    harness.get_by_label("Esporta configurazione").click();
    harness.run();

    let intents = intents.borrow();
    assert!(intents.iter().any(|intent| {
        matches!(
            intent,
            UiIntent::Save(config) if !config.drag_snap_enabled && config.validate().is_ok()
        )
    }));
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, UiIntent::Import))
    );
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, UiIntent::Export(_)))
    );
}

#[test]
fn the_general_sliders_are_wide_enough_to_aim_at_a_value() {
    use egui::accesskit::Role;
    use egui_kittest::{Node, kittest::By};

    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);
    harness.get_by_label("Generale").click();
    harness.run();

    let sliders: Vec<Node<'_>> = harness.query_all(By::new().role(Role::Slider)).collect();

    assert_eq!(sliders.len(), 2, "gap and snap threshold are both sliders");
    for slider in sliders {
        let rect = slider.rect();
        assert!(
            rect.width() >= 200.0,
            "a slider rail only {} wide cannot be aimed at",
            rect.width()
        );
    }
}

#[test]
fn shortcut_editor_adds_a_valid_binding() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);
    harness.get_by_label("Scorciatoie").click();
    harness.run();

    harness.get_by_label("Aggiungi scorciatoia").click();
    harness.run();
    let _prompt = harness.get_by_label("Premi la combinazione desiderata…");
    assert!(
        !intents
            .borrow()
            .iter()
            .any(|intent| matches!(intent, UiIntent::Save(_)))
    );

    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::ALT, Key::W);
    harness.run();

    assert!(intents.borrow().iter().any(|intent| {
        matches!(
            intent,
            UiIntent::Save(config) if config.hotkeys.len() == 23 && config.validate().is_ok()
        )
    }));
}

#[test]
fn shortcut_editor_shows_readable_localized_key_names() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);
    harness.get_by_label("Scorciatoie").click();
    harness.run();

    let _shortcut = harness.get_by_label("Ctrl + Alt + Freccia sinistra");
}

#[test]
fn shortcut_editor_uses_responsive_columns_and_keeps_the_status_at_the_bottom() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness_with_size(&state, &intents, egui::Vec2::new(1_920.0, 900.0));
    harness.get_by_label("Scorciatoie").click();
    harness.run();

    let first = harness.get_by_label("Ctrl + Alt + Freccia sinistra").rect();
    let second = harness.get_by_label("Ctrl + Alt + Freccia destra").rect();
    let third = harness.get_by_label("Ctrl + Alt + Freccia su").rect();
    assert!((first.center().y - second.center().y).abs() < 1.0);
    assert!((second.center().y - third.center().y).abs() < 1.0);
    assert!(first.left() < second.left() && second.left() < third.left());
    let status_y = harness.get_by_label("Cuboid è pronto").rect().center().y;
    assert!(status_y > 800.0, "status y-position was {status_y}");
}

#[test]
fn shortcut_editor_keeps_scrolled_cards_inside_the_content_viewport() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness_with_size(&state, &intents, egui::Vec2::new(720.0, 560.0));
    harness.get_by_label("Scorciatoie").click();
    harness.run();

    harness
        .get_by_label("Ctrl + Alt + Win + Freccia destra")
        .scroll_to_me();
    harness.run();

    let heading_bottom = harness
        .get_by_label("Scorciatoie da tastiera")
        .rect()
        .bottom();
    let content_bottom = 550.0;
    for (label, card_content) in [
        (
            "Monitor successivo",
            harness.get_by_value("Monitor successivo").rect(),
        ),
        (
            "Ctrl + Alt + Win + Freccia destra",
            harness
                .get_by_label("Ctrl + Alt + Win + Freccia destra")
                .rect(),
        ),
    ] {
        assert!(
            card_content.top() > heading_bottom,
            "{label} escaped above the content viewport: {card_content:?}"
        );
        assert!(
            card_content.bottom() < content_bottom,
            "{label} escaped below the content viewport: {card_content:?}"
        );
    }
}

#[test]
fn shortcut_editor_rejects_duplicate_combinations_before_saving() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);
    harness.get_by_label("Scorciatoie").click();
    harness.run();
    harness.get_by_label("Aggiungi scorciatoia").click();
    harness.run();

    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::ALT, Key::ArrowLeft);
    harness.run();

    assert!(
        !intents
            .borrow()
            .iter()
            .any(|intent| matches!(intent, UiIntent::Save(_)))
    );
    let _status = harness.get_by_label("Combinazione già assegnata: Ctrl + Alt + Freccia sinistra");
    let _prompt = harness.get_by_label("Premi la combinazione desiderata…");
}

#[test]
fn editing_a_shortcut_suspends_global_hotkeys_until_capture_finishes() {
    let mut config = italian_config();
    config.hotkeys.retain(|binding| binding.virtual_key != 0xBD);
    let state = Rc::new(RefCell::new(UiState::new(config)));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);
    harness.get_by_label("Scorciatoie").click();
    harness.run();

    harness
        .get_by_label("Ctrl + Alt + Freccia sinistra")
        .click();
    harness.run();

    assert!(
        intents
            .borrow()
            .iter()
            .any(|intent| matches!(intent, UiIntent::SetShortcutCaptureActive(true)))
    );
    intents.borrow_mut().clear();

    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::ALT, Key::Minus);
    harness.run();

    let intents = intents.borrow();
    let save_index = intents
        .iter()
        .position(|intent| {
            matches!(
                intent,
                UiIntent::Save(config)
                    if config.hotkeys.first().is_some_and(|binding| {
                        binding.modifiers == 0x0003 && binding.virtual_key == 0xBD
                    })
            )
        })
        .expect("captured shortcut was not saved");
    let resume_index = intents
        .iter()
        .position(|intent| matches!(intent, UiIntent::SetShortcutCaptureActive(false)))
        .expect("global hotkeys were not resumed");
    assert!(save_index < resume_index);
}

#[test]
fn cancelling_shortcut_capture_resumes_global_hotkeys() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);
    harness.get_by_label("Scorciatoie").click();
    harness.run();
    harness
        .get_by_label("Ctrl + Alt + Freccia sinistra")
        .click();
    harness.run();
    intents.borrow_mut().clear();

    harness.get_by_label("Premi la nuova combinazione…").click();
    harness.run();
    harness.get_by_label("Azioni").click();
    harness.run();

    assert!(
        intents
            .borrow()
            .iter()
            .any(|intent| matches!(intent, UiIntent::SetShortcutCaptureActive(false)))
    );
}

#[test]
fn hiding_during_shortcut_capture_resumes_global_hotkeys() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);
    harness.get_by_label("Scorciatoie").click();
    harness.run();
    harness
        .get_by_label("Ctrl + Alt + Freccia sinistra")
        .click();
    harness.run();
    intents.borrow_mut().clear();

    harness.get_by_label("Nascondi nella tray").click();
    harness.run();

    let intents = intents.borrow();
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, UiIntent::Hide))
    );
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, UiIntent::SetShortcutCaptureActive(false)))
    );
}

#[test]
fn shortcut_capture_uses_the_physical_key_for_layout_dependent_symbols() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);
    harness.get_by_label("Scorciatoie").click();
    harness.run();
    harness.get_by_label("Aggiungi scorciatoia").click();
    harness.run();

    harness.event(egui::Event::Key {
        key: Key::Colon,
        physical_key: Some(Key::Period),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::SHIFT,
    });
    harness.run();

    assert!(intents.borrow().iter().any(|intent| {
        matches!(
            intent,
            UiIntent::Save(config)
                if config.hotkeys.last().is_some_and(|binding| {
                    binding.modifiers == 0x0004 && binding.virtual_key == 0xBE
                })
        )
    }));
}

#[test]
fn the_shell_shows_only_the_base_pages_without_an_extension() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let harness = ui_harness(&state, &intents);

    let _actions = harness.get_by_label("Azioni");
    let _shortcuts = harness.get_by_label("Scorciatoie");
    let _general = harness.get_by_label("Generale");
    assert!(harness.query_by_label("Layout").is_none());
    assert!(harness.query_by_label("Applicazioni").is_none());
}

#[test]
fn an_extension_page_is_reachable_and_forwards_its_intents_and_changes() {
    let state = Rc::new(RefCell::new(UiState::with_extension(
        italian_config(),
        Box::new(TestExtension),
    )));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);

    harness.get_by_label("Prova").click();
    harness.run();
    let _title = harness.get_by_label("Pagina di prova");

    harness.get_by_label("Applica area").click();
    harness.run();

    let intents = intents.borrow();
    assert!(intents.iter().any(|intent| {
        matches!(
            intent,
            UiIntent::ApplyArea(bounds) if *bounds == NormalizedRect::new(0, 0, 5_000, 10_000)
        )
    }));
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, UiIntent::Save(config) if config.validate().is_ok())),
        "a change reported by the extension was not persisted"
    );
}

#[test]
fn a_change_still_in_progress_is_not_written_yet() {
    let state = Rc::new(RefCell::new(UiState::with_extension(
        italian_config(),
        Box::new(TestExtension),
    )));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);

    harness.get_by_label("Prova").click();
    harness.run();
    harness.get_by_label("Modifica in corso").click();
    harness.run();

    assert!(
        !intents
            .borrow()
            .iter()
            .any(|intent| matches!(intent, UiIntent::Save(_))),
        "a change reported as still in progress was written to disk"
    );
}

#[test]
fn generated_values_avoid_the_ones_already_taken() {
    assert_eq!(
        cuboid_ui::unique_id("zone", ["zone-1", "zone-3"].into_iter()),
        "zone-2"
    );
    assert_eq!(
        cuboid_ui::unique_value(
            |number| format!("application-{number}.exe"),
            ["Application-1.exe"].into_iter()
        ),
        "application-2.exe"
    );
}

#[test]
fn runtime_status_is_exposed_to_accessibility_clients() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    state
        .borrow_mut()
        .handle_event(RuntimeEvent::NoActiveWindow);
    let intents = Rc::new(RefCell::new(Vec::new()));
    let harness = ui_harness(&state, &intents);

    let _status = harness.get_by_label("Nessuna finestra idonea attiva");
}

#[test]
fn tray_hide_button_emits_hide_intent() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness(&state, &intents);

    harness.get_by_label("Nascondi nella tray").click();
    harness.run();

    assert!(
        intents
            .borrow()
            .iter()
            .any(|intent| matches!(intent, UiIntent::Hide))
    );
}

#[test]
fn shortcut_toolbar_and_cards_stay_next_to_the_page_header() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness_with_size(&state, &intents, egui::Vec2::new(720.0, 560.0));
    harness.get_by_label("Scorciatoie").click();
    harness.run();

    let header_bottom = harness
        .get_by_label("Scorciatoie da tastiera")
        .rect()
        .bottom();
    let toolbar = harness.get_by_label("Aggiungi scorciatoia").rect();
    let first_card = harness.get_by_label("Ctrl + Alt + Freccia sinistra").rect();

    assert!(
        toolbar.top() > header_bottom && toolbar.top() - header_bottom < 80.0,
        "the toolbar drifted away from the header: header_bottom={header_bottom}, toolbar={toolbar:?}"
    );
    assert!(
        first_card.top() > toolbar.bottom() && first_card.top() - toolbar.bottom() < 60.0,
        "the first shortcut card sank towards the bottom: toolbar={toolbar:?}, card={first_card:?}"
    );
    assert!(
        first_card.bottom() < 280.0,
        "the shortcut list started in the lower half of the window: {first_card:?}"
    );
}

#[test]
fn sidebar_footer_stays_compact_and_anchored_to_the_bottom() {
    let state = Rc::new(RefCell::new(UiState::new(italian_config())));
    let intents = Rc::new(RefCell::new(Vec::new()));
    let mut harness = ui_harness_with_size(&state, &intents, egui::Vec2::new(720.0, 560.0));
    harness.get_by_label("Generale").click();
    harness.run();

    let status = harness.get_by_label("Cuboid è pronto").rect();
    let tray_button = harness.get_by_label("Nascondi nella tray").rect();
    let page_title = harness.get_by_label("Impostazioni generali").rect();
    assert!(
        tray_button.center().y > 500.0,
        "tray button was not anchored to the bottom: {tray_button:?}"
    );
    assert!(
        status.bottom() < tray_button.top(),
        "sidebar footer controls overlapped: status={status:?}, tray={tray_button:?}"
    );
    assert!(
        status.right() < page_title.left() && tray_button.right() < page_title.left(),
        "sidebar footer escaped into the content area"
    );
}

#[test]
fn english_configuration_localizes_actions_and_runtime_status() {
    let config = AppConfig {
        language: Language::English,
        ..AppConfig::default()
    };
    let state = Rc::new(RefCell::new(UiState::new(config)));
    state.borrow_mut().handle_event(RuntimeEvent::AccessDenied);
    let intents = Rc::new(RefCell::new(Vec::new()));
    let harness = ui_harness(&state, &intents);

    let _action = harness.get_by_label("Left half");
    let _status = harness.get_by_label("Windows prevents controlling an elevated window");
}

/// The defaults rendered in Italian, the language most assertions below read.
fn italian_config() -> AppConfig {
    AppConfig {
        language: Language::Italian,
        ..AppConfig::default()
    }
}

struct TestExtension;
impl TestExtension {
    const PAGES: &'static [ExtensionPage] = &[ExtensionPage {
        id: "test-page",
        icon: PageIcon::Grid,
        label: LocalizedText::new("Prova", "Test"),
        title: LocalizedText::new("Pagina di prova", "Test page"),
        description: LocalizedText::new("Una pagina ospite.", "A hosted page."),
    }];
}

impl UiExtension for TestExtension {
    fn pages(&self) -> &[ExtensionPage] {
        Self::PAGES
    }

    fn show(
        &mut self,
        _page: &str,
        ui: &mut egui::Ui,
        _language: Language,
        intents: &mut Vec<UiIntent>,
    ) -> ExtensionChange {
        let mut change = ExtensionChange::Unchanged;
        if ui.button("Applica area").clicked() {
            intents.push(UiIntent::ApplyArea(NormalizedRect::new(
                0, 0, 5_000, 10_000,
            )));
            change = change.merge(ExtensionChange::Persist);
        }
        if ui.button("Modifica in corso").clicked() {
            change = change.merge(ExtensionChange::Live);
        }
        change
    }
}

fn ui_harness(
    state: &Rc<RefCell<UiState>>,
    intents: &Rc<RefCell<Vec<UiIntent>>>,
) -> Harness<'static> {
    ui_harness_with_size(state, intents, egui::Vec2::new(800.0, 600.0))
}

fn ui_harness_with_size(
    state: &Rc<RefCell<UiState>>,
    intents: &Rc<RefCell<Vec<UiIntent>>>,
    size: egui::Vec2,
) -> Harness<'static> {
    let state = Rc::clone(state);
    let intents = Rc::clone(intents);
    Harness::builder().with_size(size).build_ui(move |ui| {
        // The settings window is tested with the style the application runs
        // with, so that a colour or metric that hides a control fails here.
        ui.set_style(cuboid_ui::theme::style(false));
        intents.borrow_mut().extend(state.borrow_mut().show(ui));
    })
}
