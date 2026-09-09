use std::io;

use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const PERSONALIZE_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const APPS_USE_LIGHT_THEME: &str = "AppsUseLightTheme";
const DWM_KEY: &str = r"Software\Microsoft\Windows\DWM";
const COLORIZATION_COLOR: &str = "ColorizationColor";

pub const WINDOWS_BLUE: RgbColor = RgbColor::new(0x00, 0x67, 0xC0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowsTheme {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl RgbColor {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryIssue {
    Missing,
    Unreadable(String),
    InvalidDword(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistrySetting<T> {
    Value(T),
    Unavailable(RegistryIssue),
}

impl<T> RegistrySetting<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Unavailable(_) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsAppearance {
    pub theme: RegistrySetting<WindowsTheme>,
    pub accent: RegistrySetting<RgbColor>,
}

impl WindowsAppearance {
    pub fn theme_or(&self, fallback: WindowsTheme) -> WindowsTheme {
        self.theme.value().copied().unwrap_or(fallback)
    }

    pub fn accent_or_default(&self) -> RgbColor {
        self.accent.value().copied().unwrap_or(WINDOWS_BLUE)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsAppearanceProvider;

impl WindowsAppearanceProvider {
    pub const fn new() -> Self {
        Self
    }

    pub fn read(&self) -> WindowsAppearance {
        WindowsAppearance {
            theme: read_dword(PERSONALIZE_KEY, APPS_USE_LIGHT_THEME).and_then(decode_theme),
            accent: read_dword(DWM_KEY, COLORIZATION_COLOR).map(decode_colorization_color),
        }
    }
}

fn read_dword(path: &str, name: &str) -> RegistrySetting<u32> {
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let key = match current_user.open_subkey(path) {
        Ok(key) => key,
        Err(error) => return unavailable(error),
    };
    match key.get_value(name) {
        Ok(value) => RegistrySetting::Value(value),
        Err(error) => unavailable(error),
    }
}

fn unavailable<T>(error: io::Error) -> RegistrySetting<T> {
    if error.kind() == io::ErrorKind::NotFound {
        RegistrySetting::Unavailable(RegistryIssue::Missing)
    } else {
        RegistrySetting::Unavailable(RegistryIssue::Unreadable(error.to_string()))
    }
}

fn decode_theme(value: u32) -> RegistrySetting<WindowsTheme> {
    match value {
        0 => RegistrySetting::Value(WindowsTheme::Dark),
        1 => RegistrySetting::Value(WindowsTheme::Light),
        value => RegistrySetting::Unavailable(RegistryIssue::InvalidDword(value)),
    }
}

fn decode_colorization_color(argb: u32) -> RgbColor {
    RgbColor::new(
        ((argb >> 16) & 0xff) as u8,
        ((argb >> 8) & 0xff) as u8,
        (argb & 0xff) as u8,
    )
}

trait RegistrySettingExt<T> {
    fn map<U>(self, transform: impl FnOnce(T) -> U) -> RegistrySetting<U>;
    fn and_then<U>(self, transform: impl FnOnce(T) -> RegistrySetting<U>) -> RegistrySetting<U>;
}

impl<T> RegistrySettingExt<T> for RegistrySetting<T> {
    fn map<U>(self, transform: impl FnOnce(T) -> U) -> RegistrySetting<U> {
        match self {
            RegistrySetting::Value(value) => RegistrySetting::Value(transform(value)),
            RegistrySetting::Unavailable(issue) => RegistrySetting::Unavailable(issue),
        }
    }

    fn and_then<U>(self, transform: impl FnOnce(T) -> RegistrySetting<U>) -> RegistrySetting<U> {
        match self {
            RegistrySetting::Value(value) => transform(value),
            RegistrySetting::Unavailable(issue) => RegistrySetting::Unavailable(issue),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_documented_theme_values() {
        assert_eq!(decode_theme(0), RegistrySetting::Value(WindowsTheme::Dark));
        assert_eq!(decode_theme(1), RegistrySetting::Value(WindowsTheme::Light));
        assert_eq!(
            decode_theme(2),
            RegistrySetting::Unavailable(RegistryIssue::InvalidDword(2))
        );
    }

    #[test]
    fn decodes_colorization_color_as_argb() {
        assert_eq!(
            decode_colorization_color(0xC400_78D4),
            RgbColor::new(0x00, 0x78, 0xD4)
        );
    }

    #[test]
    fn uses_documented_fallbacks_for_unavailable_settings() {
        let appearance = WindowsAppearance {
            theme: RegistrySetting::Unavailable(RegistryIssue::Missing),
            accent: RegistrySetting::Unavailable(RegistryIssue::Unreadable("denied".to_owned())),
        };

        assert_eq!(
            appearance.theme_or(WindowsTheme::Light),
            WindowsTheme::Light
        );
        assert_eq!(appearance.accent_or_default(), WINDOWS_BLUE);
    }
}
