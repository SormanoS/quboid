use std::{io, path::Path};

use windows::{
    ApplicationModel::{StartupTask, StartupTaskState},
    Win32::{
        Foundation::APPMODEL_ERROR_NO_PACKAGE, Storage::Packaging::Appx::GetCurrentPackageFullName,
    },
    core::HSTRING,
};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// The task the package manifest declares. The two must agree: Windows looks
/// the task up by this identifier and fails when it does not exist.
const STARTUP_TASK_ID: &str = "QuboidStartup";

/// Registers or removes the current-user startup entry named `product`.
///
/// A packaged build cannot use the Run key. Windows owns startup for packaged
/// apps and drives it through the task the manifest declares, which the user
/// can also turn off from Task Manager. An unpackaged build has no such task,
/// so it keeps writing the Run key.
pub fn set_launch_at_login(product: &str, enabled: bool, executable: &Path) -> io::Result<()> {
    if has_package_identity() {
        set_startup_task(enabled)
    } else {
        set_run_key(product, enabled, executable)
    }
}

/// Reports whether the process runs with package identity, which is what
/// decides between the two startup mechanisms.
fn has_package_identity() -> bool {
    let mut length = 0;
    // Asking for the name without a buffer reports the length that would be
    // needed, or that there is no package at all.
    let result = unsafe { GetCurrentPackageFullName(&mut length, None) };
    result != APPMODEL_ERROR_NO_PACKAGE
}

fn set_startup_task(enabled: bool) -> io::Result<()> {
    let task = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))
        .and_then(|operation| operation.join())
        .map_err(startup_task_error)?;

    if !enabled {
        return task.Disable().map_err(startup_task_error);
    }

    let state = task
        .RequestEnableAsync()
        .and_then(|operation| operation.join())
        .map_err(startup_task_error)?;

    // Enabling is a request, not a command: once the user turns the task off
    // in Task Manager, Windows keeps refusing it and reports that here instead
    // of failing. Saying so beats leaving the setting looking enabled.
    match state {
        StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy => Ok(()),
        StartupTaskState::DisabledByUser => Err(io::Error::other(
            "Windows keeps the app disabled at startup because it was turned off in Task Manager",
        )),
        StartupTaskState::DisabledByPolicy => Err(io::Error::other(
            "a Windows policy prevents the app from starting at login",
        )),
        _ => Err(io::Error::other(
            "Windows did not enable the app at startup",
        )),
    }
}

fn startup_task_error(error: windows::core::Error) -> io::Error {
    io::Error::other(format!("startup task {STARTUP_TASK_ID}: {error}"))
}

fn set_run_key(product: &str, enabled: bool, executable: &Path) -> io::Result<()> {
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = current_user.create_subkey(RUN_KEY)?;
    if enabled {
        let command = format!("\"{}\"", executable.display());
        run.set_value(product, &command)
    } else {
        match run.delete_value(product) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_process_has_no_package_identity() {
        assert!(!has_package_identity());
    }
}
