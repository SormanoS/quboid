use std::{ffi::OsString, fs, path::Path};

use quboid_core::{
    Action, AppConfig, ConfigError, ConfigStorage, ConfigStorageError, HotkeyBinding,
    InstalledConfigAdapter, Language, MAX_CONFIG_BYTES, NORMALIZED_SCALE, PortableConfigAdapter,
};

#[test]
fn default_configuration_roundtrips_through_public_storage_interface() {
    let directory = tempfile::tempdir().unwrap();
    let storage = PortableConfigAdapter::new(directory.path());
    let config = AppConfig::default();

    storage.save(&config).unwrap();

    assert_eq!(storage.load().unwrap().config, config);
    assert_eq!(
        storage
            .import(&storage.export(&config).unwrap())
            .unwrap()
            .config,
        config
    );
}

#[test]
fn default_shortcuts_match_rectangle_windows_mapping() {
    let config = AppConfig::default();
    let binding = |action| {
        config
            .hotkeys
            .iter()
            .find(|binding| binding.action == action)
            .map(|binding| (binding.modifiers, binding.virtual_key))
    };

    assert_eq!(config.hotkeys.len(), 22);
    assert_eq!(binding(Action::LeftHalf), Some((0x0003, 0x25)));
    assert_eq!(binding(Action::TopLeft), Some((0x0003, 0x55)));
    assert_eq!(binding(Action::CenterTwoThirds), Some((0x0003, 0x52)));
    assert_eq!(binding(Action::Maximize), Some((0x0003, 0x0D)));
    assert_eq!(binding(Action::MaximizeHeight), Some((0x0007, 0x26)));
    assert_eq!(binding(Action::NextMonitor), Some((0x000B, 0x27)));
    assert_eq!(binding(Action::AlmostMaximize), None);
    assert_eq!(binding(Action::CenterHalf), None);
}

#[test]
fn installed_and_portable_adapters_keep_independent_documents() {
    let directory = tempfile::tempdir().unwrap();
    let portable = PortableConfigAdapter::new(directory.path());
    let installed = InstalledConfigAdapter::new(directory.path());
    let english = AppConfig {
        language: Language::English,
        ..AppConfig::default()
    };

    portable.save(&english).unwrap();
    installed.save(&AppConfig::default()).unwrap();

    assert_eq!(portable.load().unwrap().config, english);
    assert_eq!(installed.load().unwrap().config, AppConfig::default());
    assert_ne!(portable.config_path(), installed.config_path());
}

#[test]
fn saving_replaces_document_and_keeps_previous_version_as_backup() {
    let directory = tempfile::tempdir().unwrap();
    let storage = PortableConfigAdapter::new(directory.path());
    let italian = AppConfig::default();
    let english = AppConfig {
        language: Language::English,
        ..italian.clone()
    };
    storage.save(&italian).unwrap();
    let original = fs::read(storage.config_path()).unwrap();

    storage.save(&english).unwrap();

    assert_eq!(storage.load().unwrap().config, english);
    assert_eq!(
        fs::read(backup_path(storage.config_path())).unwrap(),
        original
    );
    assert!(!temporary_path(storage.config_path()).exists());
}

#[test]
fn version_one_document_is_migrated_without_losing_user_settings() {
    let storage = PortableConfigAdapter::new("unused");
    let version_one = br#"{
        "schema_version": 1,
        "language": "english",
        "launch_at_login": true,
        "drag_snap_enabled": false,
        "hotkeys": []
    }"#;

    let imported = storage.import(version_one).unwrap();

    assert_eq!(
        imported.config.schema_version,
        quboid_core::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(imported.config.language, Language::English);
    assert!(imported.config.launch_at_login);
    assert!(!imported.config.drag_snap_enabled);
    assert!(imported.discarded_sections.is_empty());
}

#[test]
fn legacy_document_keeps_base_settings_and_reports_unsupported_sections() {
    let storage = PortableConfigAdapter::new("unused");
    let legacy = br#"{
        "schema_version": 3,
        "language": "english",
        "launch_at_login": false,
        "drag_snap_enabled": true,
        "hotkeys": [{ "action": "left_half", "modifiers": 3, "virtual_key": 37 }],
        "gap": 12,
        "snap_threshold": 640,
        "layouts": [
            {
                "id": "work",
                "name": "Work",
                "zones": [
                    { "id": "left", "name": "Left", "bounds": {
                        "left": 0, "top": 0, "right": 5000, "bottom": 10000 } }
                ]
            }
        ],
        "application_rules": [
            { "application": "editor.exe", "layout_id": "work", "excluded": false }
        ],
        "action_cycles": [
            { "id": "halves", "actions": ["left_half", "right_half"] }
        ]
    }"#;

    let imported = storage.import(legacy).unwrap();

    assert_eq!(imported.config.language, Language::English);
    assert_eq!(imported.config.gap, 12);
    assert_eq!(imported.config.snap_threshold, 640);
    assert_eq!(imported.config.hotkeys.len(), 1);
    assert_eq!(
        imported.discarded_sections,
        vec![
            "layouts".to_owned(),
            "application_rules".to_owned(),
            "action_cycles".to_owned()
        ]
    );
}

#[test]
fn legacy_document_without_extra_sections_reports_nothing_discarded() {
    let storage = PortableConfigAdapter::new("unused");
    let legacy = br#"{
        "schema_version": 3,
        "language": "italian",
        "launch_at_login": false,
        "drag_snap_enabled": true,
        "hotkeys": [],
        "gap": 0,
        "snap_threshold": 500,
        "layouts": [],
        "application_rules": [],
        "action_cycles": []
    }"#;

    let imported = storage.import(legacy).unwrap();

    assert!(imported.discarded_sections.is_empty());
}

#[test]
fn exported_documents_never_contain_separately_licensed_sections() {
    let storage = PortableConfigAdapter::new("unused");
    let json = storage.export(&AppConfig::default()).unwrap();
    let document = String::from_utf8(json).unwrap();

    for section in ["layouts", "application_rules", "action_cycles"] {
        assert!(
            !document.contains(section),
            "the Base document exported {section}"
        );
    }
}

#[test]
fn a_migrated_document_keeps_its_unsupported_sections_in_a_durable_sidecar() {
    let directory = tempfile::tempdir().unwrap();
    let storage = PortableConfigAdapter::new(directory.path());
    let legacy = br#"{
        "schema_version": 3,
        "language": "english",
        "launch_at_login": false,
        "drag_snap_enabled": true,
        "hotkeys": [],
        "gap": 0,
        "snap_threshold": 500,
        "layouts": [
            {
                "id": "work",
                "name": "Work",
                "zones": [
                    { "id": "left", "name": "Left", "bounds": {
                        "left": 0, "top": 0, "right": 5000, "bottom": 10000 } }
                ]
            }
        ],
        "application_rules": [
            { "application": "editor.exe", "layout_id": "work", "excluded": false }
        ],
        "action_cycles": [
            { "id": "halves", "actions": ["left_half", "right_half"] }
        ]
    }"#;
    fs::write(storage.config_path(), legacy).unwrap();

    let loaded = storage.load().unwrap();
    let archive = quboid_core::unsupported_sections_path(storage.config_path());

    assert_eq!(loaded.archived_sections.as_deref(), Some(archive.as_path()));
    let archived: serde_json::Value = serde_json::from_slice(&fs::read(&archive).unwrap()).unwrap();
    assert_eq!(archived["schema_version"], serde_json::json!(3));
    assert_eq!(archived["layouts"][0]["id"], serde_json::json!("work"));
    assert_eq!(
        archived["application_rules"][0]["application"],
        serde_json::json!("editor.exe")
    );
    assert_eq!(
        archived["action_cycles"][0]["id"],
        serde_json::json!("halves")
    );

    // Base rewrites the document without those sections, repeatedly, and the
    // archive stays exactly as it was written the first time.
    let original_archive = fs::read(&archive).unwrap();
    storage.save(&loaded.config).unwrap();
    let reloaded = storage.load().unwrap();
    storage.save(&reloaded.config).unwrap();

    assert!(reloaded.discarded_sections.is_empty());
    assert_eq!(fs::read(&archive).unwrap(), original_archive);
    assert!(
        !String::from_utf8(fs::read(storage.config_path()).unwrap())
            .unwrap()
            .contains("layouts")
    );
}

#[test]
fn a_document_from_another_distribution_is_imported_without_its_foreign_sections() {
    let storage = PortableConfigAdapter::new("unused");
    let foreign = br#"{
        "schema_version": 1,
        "base": {
            "schema_version": 3,
            "language": "english",
            "launch_at_login": true,
            "drag_snap_enabled": true,
            "hotkeys": [{ "action": "left_half", "modifiers": 3, "virtual_key": 37 }],
            "gap": 5,
            "snap_threshold": 500
        },
        "extension": { "anything": [1, 2, 3] }
    }"#;

    let imported = storage.import(foreign).unwrap();

    assert_eq!(imported.config.language, Language::English);
    assert_eq!(imported.config.gap, 5);
    assert_eq!(imported.config.hotkeys.len(), 1);
    assert_eq!(
        imported.config.schema_version,
        quboid_core::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(imported.discarded_sections, vec!["extension".to_owned()]);
}

#[test]
fn a_document_whose_base_section_is_unreadable_is_still_rejected() {
    let storage = PortableConfigAdapter::new("unused");

    for invalid in [
        br#"{"schema_version":1,"base":{"language":"italian"}}"#.as_slice(),
        br#"{"schema_version":1,"base":{"schema_version":999}}"#.as_slice(),
        br#"{"schema_version":1,"base":{"base":{"schema_version":4}}}"#.as_slice(),
    ] {
        assert!(storage.import(invalid).is_err());
    }
}

#[test]
fn invalid_imports_are_rejected_without_changing_saved_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let storage = PortableConfigAdapter::new(directory.path());
    storage.save(&AppConfig::default()).unwrap();
    let saved = fs::read(storage.config_path()).unwrap();

    for invalid in [
        br#"{"schema_version":999}"#.as_slice(),
        br#"{"schema_version":4,"unknown":true}"#.as_slice(),
        br#"{"language":"italian"}"#.as_slice(),
        br#"{not json"#.as_slice(),
    ] {
        assert!(storage.import(invalid).is_err());
        assert_eq!(fs::read(storage.config_path()).unwrap(), saved);
    }
}

#[test]
fn configuration_size_limit_applies_to_import_and_load() {
    let directory = tempfile::tempdir().unwrap();
    let storage = PortableConfigAdapter::new(directory.path());
    let oversized = vec![b' '; MAX_CONFIG_BYTES + 1];

    assert!(matches!(
        storage.import(&oversized),
        Err(ConfigStorageError::TooLarge { .. })
    ));
    fs::write(storage.config_path(), oversized).unwrap();
    assert!(matches!(
        storage.load(),
        Err(ConfigStorageError::TooLarge { .. })
    ));
}

#[test]
fn semantic_validation_rejects_every_invalid_configuration_category() {
    let mut cases = Vec::new();

    let future = AppConfig {
        schema_version: 999,
        ..AppConfig::default()
    };
    cases.push((future, ConfigError::FutureVersion(999)));

    let old = AppConfig {
        schema_version: 1,
        ..AppConfig::default()
    };
    cases.push((old, ConfigError::UnsupportedVersion(1)));

    let mut missing_key = AppConfig::default();
    missing_key.hotkeys[0].virtual_key = 0;
    cases.push((missing_key, ConfigError::InvalidHotkey(0)));

    let mut duplicate_key = AppConfig::default();
    let duplicate_index = duplicate_key.hotkeys.len();
    duplicate_key.hotkeys.push(duplicate_key.hotkeys[0].clone());
    cases.push((duplicate_key, ConfigError::DuplicateHotkey(duplicate_index)));

    let invalid_gap = AppConfig {
        gap: NORMALIZED_SCALE + 1,
        ..AppConfig::default()
    };
    cases.push((invalid_gap, ConfigError::InvalidGap(NORMALIZED_SCALE + 1)));

    let invalid_threshold = AppConfig {
        snap_threshold: NORMALIZED_SCALE + 1,
        ..AppConfig::default()
    };
    cases.push((
        invalid_threshold,
        ConfigError::InvalidSnapThreshold(NORMALIZED_SCALE + 1),
    ));

    for (config, expected) in cases {
        assert_eq!(config.validate(), Err(expected));
    }
}

#[test]
fn complete_base_configuration_with_custom_hotkeys_is_valid() {
    let mut config = AppConfig {
        gap: 8,
        snap_threshold: 700,
        ..AppConfig::default()
    };
    config.hotkeys.push(HotkeyBinding {
        action: Action::TopLeft,
        modifiers: 0x0006,
        virtual_key: 0x54,
    });

    assert_eq!(config.validate(), Ok(()));
}

fn backup_path(path: &Path) -> std::path::PathBuf {
    suffixed(path, ".bak")
}

fn temporary_path(path: &Path) -> std::path::PathBuf {
    suffixed(path, ".tmp")
}

fn suffixed(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    name.into()
}
