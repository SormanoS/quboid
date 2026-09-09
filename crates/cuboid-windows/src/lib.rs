#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

#[cfg(target_os = "windows")]
pub mod adapters;

#[cfg(target_os = "windows")]
mod appearance;
#[cfg(target_os = "windows")]
mod locale;
#[cfg(target_os = "windows")]
mod runtime;
#[cfg(target_os = "windows")]
mod startup;

#[cfg(target_os = "windows")]
pub use adapters::{
    AllWindows, EdgeSnapResolver, RuntimeAdapters, ShortcutResolver, SingleActionShortcuts,
    SnapRequest, SnapResolver, WindowFilter,
};
#[cfg(target_os = "windows")]
pub use appearance::{
    RegistryIssue, RegistrySetting, RgbColor, WINDOWS_BLUE, WindowsAppearance,
    WindowsAppearanceProvider, WindowsTheme,
};
#[cfg(target_os = "windows")]
pub use locale::system_language;
#[cfg(target_os = "windows")]
pub use runtime::{Runtime, RuntimeError};
#[cfg(target_os = "windows")]
pub use startup::set_launch_at_login;

#[cfg(not(target_os = "windows"))]
compile_error!("cuboid-windows only supports Windows");
