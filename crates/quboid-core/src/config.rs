use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::{Action, NORMALIZED_SCALE};

pub const CURRENT_SCHEMA_VERSION: u32 = 5;
pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;
pub const CONFIG_FILE_NAME: &str = "config.json";
pub const INSTALLED_DIRECTORY_NAME: &str = "Quboid";

/// Suffix of the sidecar that keeps the sections a load had to leave behind, so
/// that the settings of another distribution survive the first Base save.
pub const UNSUPPORTED_SECTIONS_FILE_SUFFIX: &str = ".unsupported.json";

const DEFAULT_SNAP_THRESHOLD: u16 = 500;

/// Sections that older documents stored alongside the Base settings and that this
/// build cannot represent. They are reported instead of being silently rewritten.
const UNSUPPORTED_SECTIONS: [&str; 3] = ["layouts", "application_rules", "action_cycles"];

/// The section another distribution wraps the Base settings in. Everything next
/// to it in such a document belongs to that distribution, not to this build.
const NESTED_BASE_SECTION: &str = "base";

const SCHEMA_VERSION_FIELD: &str = "schema_version";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Italian,
    #[default]
    English,
}

impl Language {
    /// The interface language a BCP 47 locale tag asks for, such as the
    /// `it-CH` Windows reports for its user locale.
    ///
    /// Only the primary subtag is read, and anything Quboid does not translate
    /// falls back to English.
    pub fn from_locale_tag(tag: &str) -> Self {
        let primary = tag
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match primary.as_str() {
            "it" => Self::Italian,
            _ => Self::English,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotkeyBinding {
    pub action: Action,
    pub modifiers: u32,
    pub virtual_key: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub schema_version: u32,
    pub language: Language,
    pub launch_at_login: bool,
    pub drag_snap_enabled: bool,
    pub hotkeys: Vec<HotkeyBinding>,
    #[serde(default)]
    pub gap: u16,
    #[serde(default = "default_snap_threshold")]
    pub snap_threshold: u16,
    /// Whether windows Quboid placed are put back where they were once the
    /// monitors change, which is what a dock or an undock does to them.
    #[serde(default = "default_restore_on_display_change")]
    pub restore_on_display_change: bool,
}

const fn default_snap_threshold() -> u16 {
    DEFAULT_SNAP_THRESHOLD
}

const fn default_restore_on_display_change() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            language: Language::default(),
            launch_at_login: false,
            drag_snap_enabled: true,
            hotkeys: rectangle_default_hotkeys(),
            gap: 0,
            snap_threshold: DEFAULT_SNAP_THRESHOLD,
            restore_on_display_change: true,
        }
    }
}

fn rectangle_default_hotkeys() -> Vec<HotkeyBinding> {
    const ALT: u32 = 0x0001;
    const CONTROL: u32 = 0x0002;
    const SHIFT: u32 = 0x0004;
    const WIN: u32 = 0x0008;
    const CONTROL_ALT: u32 = CONTROL | ALT;

    [
        (Action::LeftHalf, CONTROL_ALT, 0x25),
        (Action::RightHalf, CONTROL_ALT, 0x27),
        (Action::TopHalf, CONTROL_ALT, 0x26),
        (Action::BottomHalf, CONTROL_ALT, 0x28),
        (Action::TopLeft, CONTROL_ALT, 0x55),
        (Action::TopRight, CONTROL_ALT, 0x49),
        (Action::BottomLeft, CONTROL_ALT, 0x4A),
        (Action::BottomRight, CONTROL_ALT, 0x4B),
        (Action::FirstThird, CONTROL_ALT, 0x44),
        (Action::FirstTwoThirds, CONTROL_ALT, 0x45),
        (Action::CenterThird, CONTROL_ALT, 0x46),
        (Action::CenterTwoThirds, CONTROL_ALT, 0x52),
        (Action::LastTwoThirds, CONTROL_ALT, 0x54),
        (Action::LastThird, CONTROL_ALT, 0x47),
        (Action::Maximize, CONTROL_ALT, 0x0D),
        (Action::MaximizeHeight, CONTROL_ALT | SHIFT, 0x26),
        (Action::Grow, CONTROL_ALT, 0xBB),
        (Action::Shrink, CONTROL_ALT, 0xBD),
        (Action::Center, CONTROL_ALT, 0x43),
        (Action::Restore, CONTROL_ALT, 0x2E),
        (Action::PreviousMonitor, CONTROL_ALT | WIN, 0x25),
        (Action::NextMonitor, CONTROL_ALT | WIN, 0x27),
    ]
    .into_iter()
    .map(|(action, modifiers, virtual_key)| HotkeyBinding {
        action,
        modifiers,
        virtual_key,
    })
    .collect()
}

fn legacy_default_hotkeys() -> Vec<HotkeyBinding> {
    const CONTROL_ALT: u32 = 0x0002 | 0x0001;
    [
        (Action::LeftHalf, 0x25),
        (Action::RightHalf, 0x27),
        (Action::Maximize, 0x26),
        (Action::Restore, 0x28),
    ]
    .into_iter()
    .map(|(action, virtual_key)| HotkeyBinding {
        action,
        modifiers: CONTROL_ALT,
        virtual_key,
    })
    .collect()
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(ConfigError::FutureVersion(self.schema_version));
        }
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedVersion(self.schema_version));
        }
        if self.gap > NORMALIZED_SCALE {
            return Err(ConfigError::InvalidGap(self.gap));
        }
        if self.snap_threshold > NORMALIZED_SCALE {
            return Err(ConfigError::InvalidSnapThreshold(self.snap_threshold));
        }

        for (index, binding) in self.hotkeys.iter().enumerate() {
            if binding.virtual_key == 0 {
                return Err(ConfigError::InvalidHotkey(index));
            }
            if self.hotkeys[..index].iter().any(|other| {
                other.modifiers == binding.modifiers && other.virtual_key == binding.virtual_key
            }) {
                return Err(ConfigError::DuplicateHotkey(index));
            }
        }

        Ok(())
    }
}

/// The outcome of reading a configuration document: the settings this build
/// understands, plus the sections it could not represent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigImport {
    pub config: AppConfig,
    pub discarded_sections: Vec<String>,
    /// Where those sections were archived, when reading a stored document had to
    /// leave them behind. The archive is written once and never rewritten.
    pub archived_sections: Option<PathBuf>,
    /// Whether no document existed and the settings are the built-in defaults,
    /// the one moment a caller may still choose for the user.
    pub defaulted: bool,
}

impl ConfigImport {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            discarded_sections: Vec::new(),
            archived_sections: None,
            defaulted: false,
        }
    }

    #[must_use]
    fn defaulted(mut self) -> Self {
        self.defaulted = true;
        self
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ConfigError {
    #[error("configuration schema version {0} is newer than this build supports")]
    FutureVersion(u32),
    #[error("configuration schema version {0} is not supported")]
    UnsupportedVersion(u32),
    #[error("hotkey at index {0} has no key")]
    InvalidHotkey(usize),
    #[error("hotkey at index {0} duplicates an earlier binding")]
    DuplicateHotkey(usize),
    #[error("gap {0} is outside the normalized range")]
    InvalidGap(u16),
    #[error("snap threshold {0} is outside the normalized range")]
    InvalidSnapThreshold(u16),
}

#[derive(Debug, Error)]
pub enum ConfigStorageError {
    #[error("configuration I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("configuration JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    InvalidConfig(#[from] ConfigError),
    #[error("configuration is {actual} bytes; maximum size is {maximum} bytes")]
    TooLarge { actual: u64, maximum: usize },
    #[error("configuration has no valid schema_version")]
    MissingSchemaVersion,
    #[error("configuration was rejected: {0}")]
    Rejected(String),
}

pub trait ConfigStorage {
    fn load(&self) -> Result<ConfigImport, ConfigStorageError>;
    fn save(&self, config: &AppConfig) -> Result<(), ConfigStorageError>;
    fn import(&self, json: &[u8]) -> Result<ConfigImport, ConfigStorageError>;
    fn export(&self, config: &AppConfig) -> Result<Vec<u8>, ConfigStorageError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledConfigAdapter {
    path: PathBuf,
}

impl InstalledConfigAdapter {
    pub fn new(app_data_root: impl AsRef<Path>) -> Self {
        Self {
            path: app_data_root
                .as_ref()
                .join(INSTALLED_DIRECTORY_NAME)
                .join(CONFIG_FILE_NAME),
        }
    }

    pub fn config_path(&self) -> &Path {
        &self.path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableConfigAdapter {
    path: PathBuf,
}

impl PortableConfigAdapter {
    pub fn new(executable_directory: impl AsRef<Path>) -> Self {
        Self {
            path: executable_directory.as_ref().join(CONFIG_FILE_NAME),
        }
    }

    pub fn config_path(&self) -> &Path {
        &self.path
    }
}

macro_rules! impl_config_storage {
    ($adapter:ty) => {
        impl ConfigStorage for $adapter {
            fn load(&self) -> Result<ConfigImport, ConfigStorageError> {
                load_path(&self.path)
            }

            fn save(&self, config: &AppConfig) -> Result<(), ConfigStorageError> {
                save_path(&self.path, config)
            }

            fn import(&self, json: &[u8]) -> Result<ConfigImport, ConfigStorageError> {
                import_json(json)
            }

            fn export(&self, config: &AppConfig) -> Result<Vec<u8>, ConfigStorageError> {
                export_json(config)
            }
        }
    };
}

impl_config_storage!(InstalledConfigAdapter);
impl_config_storage!(PortableConfigAdapter);

fn load_path(path: &Path) -> Result<ConfigImport, ConfigStorageError> {
    let Some(json) = read_document(path)? else {
        return Ok(ConfigImport::new(AppConfig::default()).defaulted());
    };
    let mut imported = import_json(&json)?;
    if !imported.discarded_sections.is_empty() {
        imported.archived_sections =
            archive_discarded_sections(path, &json, &imported.discarded_sections);
    }
    Ok(imported)
}

/// The sidecar that keeps the sections of `config_path` this build cannot
/// represent.
pub fn unsupported_sections_path(config_path: &Path) -> PathBuf {
    path_with_suffix(config_path, UNSUPPORTED_SECTIONS_FILE_SUFFIX)
}

/// Copies the sections a load left behind next to the document, once, before a
/// later save rewrites the document without them.
///
/// The archive is never overwritten, and archiving is best effort: a read-only
/// directory must not stop Quboid from starting.
fn archive_discarded_sections(path: &Path, json: &[u8], sections: &[String]) -> Option<PathBuf> {
    let archive = unsupported_sections_path(path);
    let value: Value = serde_json::from_slice(json).ok()?;
    let object = value.as_object()?;

    let mut carried = Map::new();
    for section in sections {
        if let Some(value) = object.get(section) {
            carried.insert(section.clone(), value.clone());
        }
    }
    if carried.is_empty() {
        return None;
    }
    if let Some(version) = object.get(SCHEMA_VERSION_FIELD) {
        carried.insert(SCHEMA_VERSION_FIELD.to_owned(), version.clone());
    }

    let json = serialize_document(&Value::Object(carried)).ok()?;
    match write_new_document(&archive, &json) {
        Ok(()) => Some(archive),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Some(archive),
        Err(_) => None,
    }
}

fn write_new_document(path: &Path, json: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(json)?;
    file.sync_all()
}

/// Writes `json` to `path` through a temporary file, keeping the previous
/// document as a `.bak` sibling.
pub fn write_document(path: &Path, json: &[u8]) -> Result<(), ConfigStorageError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let temporary = path_with_suffix(path, ".tmp");
    let backup = path_with_suffix(path, ".bak");
    remove_if_exists(&temporary)?;

    let result = (|| -> Result<(), ConfigStorageError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(json)?;
        file.sync_all()?;
        drop(file);

        let had_previous = path.exists();
        if had_previous {
            remove_if_exists(&backup)?;
            fs::rename(path, &backup)?;
        }

        if let Err(error) = fs::rename(&temporary, path) {
            if had_previous {
                let _ = fs::rename(&backup, path);
            }
            return Err(error.into());
        }

        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Reads a document, honouring the shared size limit.
pub fn read_document(path: &Path) -> Result<Option<Vec<u8>>, ConfigStorageError> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    let file_size = file.metadata()?.len();
    if file_size > MAX_CONFIG_BYTES as u64 {
        return Err(ConfigStorageError::TooLarge {
            actual: file_size,
            maximum: MAX_CONFIG_BYTES,
        });
    }

    let mut json = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_CONFIG_BYTES as u64 + 1)
        .read_to_end(&mut json)?;
    Ok(Some(json))
}

fn save_path(path: &Path, config: &AppConfig) -> Result<(), ConfigStorageError> {
    let json = export_json(config)?;
    write_document(path, &json)
}

fn import_json(json: &[u8]) -> Result<ConfigImport, ConfigStorageError> {
    import_value(parse_document(json)?)
}

/// Reads a Base configuration document, migrating older versions and reporting
/// the sections this build cannot represent.
pub fn import_document(json: &[u8]) -> Result<ConfigImport, ConfigStorageError> {
    import_json(json)
}

/// Reads an already parsed configuration document.
///
/// Documents written by another distribution keep the Base settings in a `base`
/// section: those are read, and everything stored next to them is reported as
/// discarded instead of failing as an unknown field.
pub fn import_value(value: Value) -> Result<ConfigImport, ConfigStorageError> {
    let (value, mut discarded_sections) = if is_nested_document(&value) {
        split_nested_document(value)?
    } else {
        (value, Vec::new())
    };

    let version = schema_version(&value)?;
    let (migrated, migration_discarded) = migrate(value, version)?;
    discarded_sections.extend(migration_discarded);
    let config: AppConfig = serde_json::from_value(migrated)?;
    config.validate()?;
    Ok(ConfigImport {
        config,
        discarded_sections,
        archived_sections: None,
        defaulted: false,
    })
}

/// Whether the document keeps the Base settings in a nested section, the shape
/// another distribution writes when it stores its own settings beside them.
fn is_nested_document(value: &Value) -> bool {
    value.get(NESTED_BASE_SECTION).is_some_and(Value::is_object)
}

/// Splits such a document into the Base section and the names of the sections
/// that belong to the distribution that wrote it.
fn split_nested_document(value: Value) -> Result<(Value, Vec<String>), ConfigStorageError> {
    let mut object = expect_object(value)?;
    let base = object
        .remove(NESTED_BASE_SECTION)
        .ok_or(ConfigStorageError::MissingSchemaVersion)?;
    let foreign = object
        .into_iter()
        .filter(|(name, value)| name != SCHEMA_VERSION_FIELD && !is_empty_section(value))
        .map(|(name, _)| name)
        .collect();
    Ok((base, foreign))
}

/// Parses a configuration document after enforcing the shared size limit.
pub fn parse_document(json: &[u8]) -> Result<Value, ConfigStorageError> {
    if json.len() > MAX_CONFIG_BYTES {
        return Err(ConfigStorageError::TooLarge {
            actual: json.len() as u64,
            maximum: MAX_CONFIG_BYTES,
        });
    }
    Ok(serde_json::from_slice(json)?)
}

/// Serializes a document with the shared formatting and size limit.
pub fn serialize_document<T: Serialize>(document: &T) -> Result<Vec<u8>, ConfigStorageError> {
    let mut json = serde_json::to_vec_pretty(document)?;
    json.push(b'\n');
    if json.len() > MAX_CONFIG_BYTES {
        return Err(ConfigStorageError::TooLarge {
            actual: json.len() as u64,
            maximum: MAX_CONFIG_BYTES,
        });
    }
    Ok(json)
}

fn export_json(config: &AppConfig) -> Result<Vec<u8>, ConfigStorageError> {
    config.validate()?;
    serialize_document(config)
}

/// Reads the `schema_version` of a configuration document.
pub fn schema_version(value: &Value) -> Result<u32, ConfigStorageError> {
    let version = value
        .as_object()
        .and_then(|object| object.get("schema_version"))
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or(ConfigStorageError::MissingSchemaVersion)?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(ConfigError::FutureVersion(version).into());
    }
    Ok(version)
}

fn migrate(mut value: Value, mut version: u32) -> Result<(Value, Vec<String>), ConfigStorageError> {
    let mut discarded = Vec::new();
    while version < CURRENT_SCHEMA_VERSION {
        value = match version {
            1 => migrate_v1_to_v2(value)?,
            2 => migrate_v2_to_v3(value)?,
            3 => migrate_v3_to_v4(value, &mut discarded)?,
            4 => migrate_v4_to_v5(value)?,
            unsupported => return Err(ConfigError::UnsupportedVersion(unsupported).into()),
        };
        version += 1;
    }
    Ok((value, discarded))
}

fn migrate_v1_to_v2(value: Value) -> Result<Value, ConfigStorageError> {
    let mut object = expect_object(value)?;
    let defaults = AppConfig::default();
    object.insert("schema_version".to_owned(), Value::from(2_u32));
    insert_serialized(&mut object, "gap", &defaults.gap)?;
    insert_serialized(&mut object, "snap_threshold", &defaults.snap_threshold)?;
    Ok(Value::Object(object))
}

fn migrate_v2_to_v3(value: Value) -> Result<Value, ConfigStorageError> {
    let mut object = expect_object(value)?;
    let legacy = serde_json::to_value(legacy_default_hotkeys())?;
    if object.get("hotkeys") == Some(&legacy) {
        insert_serialized(&mut object, "hotkeys", &rectangle_default_hotkeys())?;
    }
    object.insert("schema_version".to_owned(), Value::from(3_u32));
    Ok(Value::Object(object))
}

/// Version 4 keeps only the settings this build owns. Sections that belonged to
/// separately licensed features are dropped and reported to the caller.
fn migrate_v3_to_v4(
    value: Value,
    discarded: &mut Vec<String>,
) -> Result<Value, ConfigStorageError> {
    let mut object = expect_object(value)?;
    for section in UNSUPPORTED_SECTIONS {
        if let Some(removed) = object.remove(section)
            && !is_empty_section(&removed)
        {
            discarded.push(section.to_owned());
        }
    }
    object.insert("schema_version".to_owned(), Value::from(4_u32));
    Ok(Value::Object(object))
}

/// Version 5 records whether a display change puts windows back where they
/// were. An older document never said, so it gets the default.
fn migrate_v4_to_v5(value: Value) -> Result<Value, ConfigStorageError> {
    let mut object = expect_object(value)?;
    let defaults = AppConfig::default();
    insert_serialized(
        &mut object,
        "restore_on_display_change",
        &defaults.restore_on_display_change,
    )?;
    object.insert("schema_version".to_owned(), Value::from(5_u32));
    Ok(Value::Object(object))
}

fn is_empty_section(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(items) => items.is_empty(),
        Value::Object(entries) => entries.is_empty(),
        _ => false,
    }
}

fn expect_object(value: Value) -> Result<Map<String, Value>, ConfigStorageError> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(ConfigStorageError::MissingSchemaVersion),
    }
}

fn insert_serialized<T: Serialize>(
    object: &mut Map<String, Value>,
    key: &str,
    value: &T,
) -> Result<(), serde_json::Error> {
    object.insert(key.to_owned(), serde_json::to_value(value)?);
    Ok(())
}

fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static TEST_DIRECTORY_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("target")
                .join("quboid-core-config-tests")
                .join(format!("{}-{id}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn default_config_is_valid_and_serializable() {
        let config = AppConfig::default();
        config.validate().unwrap();
        let json = export_json(&config).unwrap();
        let restored = import_json(&json).unwrap();
        assert_eq!(restored.config, config);
        assert!(restored.discarded_sections.is_empty());
    }

    #[test]
    fn missing_file_loads_defaults() {
        let directory = TestDirectory::new();
        let storage = PortableConfigAdapter::new(&directory.0);
        let imported = storage.load().unwrap();
        assert_eq!(imported.config, AppConfig::default());
        assert!(
            imported.defaulted,
            "a load without a document must report that nothing was stored yet"
        );
        assert!(!storage.config_path().exists());
    }

    #[test]
    fn a_stored_document_is_not_reported_as_defaulted() {
        let directory = TestDirectory::new();
        let storage = PortableConfigAdapter::new(&directory.0);
        storage.save(&AppConfig::default()).unwrap();
        assert!(!storage.load().unwrap().defaulted);
    }

    #[test]
    fn locale_tags_choose_the_interface_language() {
        assert_eq!(Language::from_locale_tag("it"), Language::Italian);
        assert_eq!(Language::from_locale_tag("IT-it"), Language::Italian);
        assert_eq!(Language::from_locale_tag("it_IT"), Language::Italian);
        assert_eq!(Language::from_locale_tag("en-GB"), Language::English);
        assert_eq!(Language::from_locale_tag("fr-FR"), Language::English);
        assert_eq!(Language::from_locale_tag(""), Language::English);
    }

    #[test]
    fn roundtrip_preserves_base_configuration() {
        let directory = TestDirectory::new();
        let storage = PortableConfigAdapter::new(&directory.0);
        let config = AppConfig {
            gap: 75,
            snap_threshold: 625,
            language: Language::English,
            ..AppConfig::default()
        };

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
    fn migrates_v1_sequentially() {
        let storage = PortableConfigAdapter::new("unused");
        let v1 = br#"{
            "schema_version": 1,
            "language": "english",
            "launch_at_login": true,
            "drag_snap_enabled": false,
            "hotkeys": []
        }"#;

        let migrated = storage.import(v1).unwrap().config;
        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.language, Language::English);
        assert!(migrated.launch_at_login);
        assert!(!migrated.drag_snap_enabled);
        assert_eq!(migrated.snap_threshold, DEFAULT_SNAP_THRESHOLD);
    }

    #[test]
    fn future_version_is_rejected_without_overwriting_it() {
        let directory = TestDirectory::new();
        let storage = PortableConfigAdapter::new(&directory.0);
        let future = br#"{"schema_version":999,"future_data":"keep me"}"#;
        fs::write(storage.config_path(), future).unwrap();

        assert!(matches!(
            storage.load(),
            Err(ConfigStorageError::InvalidConfig(
                ConfigError::FutureVersion(999)
            ))
        ));
        assert_eq!(fs::read(storage.config_path()).unwrap(), future);
    }

    #[test]
    fn corrupt_file_is_rejected_without_overwriting_it() {
        let directory = TestDirectory::new();
        let storage = PortableConfigAdapter::new(&directory.0);
        let corrupt = b"{not json";
        fs::write(storage.config_path(), corrupt).unwrap();

        assert!(matches!(storage.load(), Err(ConfigStorageError::Json(_))));
        assert_eq!(fs::read(storage.config_path()).unwrap(), corrupt);
    }

    #[test]
    fn import_enforces_size_limit() {
        let storage = PortableConfigAdapter::new("unused");
        let oversized = vec![b' '; MAX_CONFIG_BYTES + 1];
        assert!(matches!(
            storage.import(&oversized),
            Err(ConfigStorageError::TooLarge { .. })
        ));
    }

    #[test]
    fn duplicate_hotkey_is_rejected() {
        let mut config = AppConfig::default();
        let duplicate_index = config.hotkeys.len();
        config.hotkeys.push(config.hotkeys[0].clone());
        assert_eq!(
            config.validate(),
            Err(ConfigError::DuplicateHotkey(duplicate_index))
        );
    }

    #[test]
    fn v2_legacy_defaults_upgrade_to_rectangle_shortcuts() {
        let storage = PortableConfigAdapter::new("unused");
        let document = serde_json::json!({
            "schema_version": 2,
            "language": "italian",
            "launch_at_login": false,
            "drag_snap_enabled": true,
            "hotkeys": legacy_default_hotkeys(),
            "gap": 0,
            "snap_threshold": DEFAULT_SNAP_THRESHOLD,
        });

        let migrated = storage
            .import(&serde_json::to_vec(&document).unwrap())
            .unwrap()
            .config;

        assert_eq!(migrated.hotkeys, rectangle_default_hotkeys());
    }

    #[test]
    fn v2_custom_shortcuts_are_preserved() {
        let storage = PortableConfigAdapter::new("unused");
        let custom = vec![HotkeyBinding {
            action: Action::Center,
            modifiers: 0x0002,
            virtual_key: 0x43,
        }];
        let document = serde_json::json!({
            "schema_version": 2,
            "language": "italian",
            "launch_at_login": false,
            "drag_snap_enabled": true,
            "hotkeys": custom,
            "gap": 0,
            "snap_threshold": DEFAULT_SNAP_THRESHOLD,
        });

        let migrated = storage
            .import(&serde_json::to_vec(&document).unwrap())
            .unwrap()
            .config;

        assert_eq!(migrated.hotkeys, custom);
    }

    #[test]
    fn save_is_atomic_and_keeps_backup() {
        let directory = TestDirectory::new();
        let storage = PortableConfigAdapter::new(&directory.0);
        let first = AppConfig::default();
        storage.save(&first).unwrap();
        let first_json = fs::read(storage.config_path()).unwrap();

        let mut second = first.clone();
        second.language = Language::English;
        storage.save(&second).unwrap();

        assert_eq!(storage.load().unwrap().config, second);
        assert_eq!(
            fs::read(path_with_suffix(storage.config_path(), ".bak")).unwrap(),
            first_json
        );
        assert!(!path_with_suffix(storage.config_path(), ".tmp").exists());

        let mut invalid = second;
        invalid.schema_version += 1;
        assert!(storage.save(&invalid).is_err());
        assert_eq!(storage.load().unwrap().config.language, Language::English);
    }

    #[test]
    fn adapters_use_portable_and_installed_locations() {
        let directory = TestDirectory::new();
        let portable = PortableConfigAdapter::new(&directory.0);
        let installed = InstalledConfigAdapter::new(&directory.0);

        assert_eq!(portable.config_path(), directory.0.join(CONFIG_FILE_NAME));
        assert_eq!(
            installed.config_path(),
            directory
                .0
                .join(INSTALLED_DIRECTORY_NAME)
                .join(CONFIG_FILE_NAME)
        );

        let portable_config = AppConfig {
            language: Language::English,
            ..AppConfig::default()
        };
        portable.save(&portable_config).unwrap();
        installed.save(&AppConfig::default()).unwrap();
        assert_eq!(portable.load().unwrap().config, portable_config);
        assert_eq!(installed.load().unwrap().config, AppConfig::default());
    }

    #[test]
    fn a_migrated_document_archives_the_sections_it_cannot_represent_exactly_once() {
        let directory = TestDirectory::new();
        let storage = PortableConfigAdapter::new(&directory.0);
        let legacy = br#"{
            "schema_version": 3,
            "language": "english",
            "launch_at_login": false,
            "drag_snap_enabled": true,
            "hotkeys": [],
            "gap": 0,
            "snap_threshold": 500,
            "layouts": [{ "id": "work" }],
            "application_rules": [],
            "action_cycles": [{ "id": "halves" }]
        }"#;
        fs::write(storage.config_path(), legacy).unwrap();

        let loaded = storage.load().unwrap();
        let archive = unsupported_sections_path(storage.config_path());

        assert_eq!(
            loaded.discarded_sections,
            vec!["layouts".to_owned(), "action_cycles".to_owned()]
        );
        assert_eq!(loaded.archived_sections.as_deref(), Some(archive.as_path()));
        let archived: Value = serde_json::from_slice(&fs::read(&archive).unwrap()).unwrap();
        assert_eq!(archived["schema_version"], Value::from(3));
        assert_eq!(archived["layouts"][0]["id"], Value::from("work"));
        assert_eq!(archived["action_cycles"][0]["id"], Value::from("halves"));
        assert!(archived.get("application_rules").is_none());

        // A later save drops the sections from the document; the archive keeps the
        // only remaining copy and is never rewritten.
        let archived_json = fs::read(&archive).unwrap();
        storage.save(&loaded.config).unwrap();
        let reloaded = storage.load().unwrap();

        assert!(reloaded.discarded_sections.is_empty());
        assert!(reloaded.archived_sections.is_none());
        assert_eq!(fs::read(&archive).unwrap(), archived_json);
    }

    #[test]
    fn a_document_from_another_distribution_keeps_its_base_section() {
        let storage = PortableConfigAdapter::new("unused");
        let foreign = br#"{
            "schema_version": 1,
            "base": {
                "schema_version": 4,
                "language": "english",
                "launch_at_login": false,
                "drag_snap_enabled": true,
                "hotkeys": [],
                "gap": 7,
                "snap_threshold": 500
            },
            "extra": { "anything": true },
            "empty": []
        }"#;

        let imported = storage.import(foreign).unwrap();

        assert_eq!(imported.config.language, Language::English);
        assert_eq!(imported.config.gap, 7);
        assert_eq!(imported.discarded_sections, vec!["extra".to_owned()]);
    }
}
