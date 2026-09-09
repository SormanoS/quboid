use std::{io, path::Path};

use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Registers or removes the current-user startup entry named `product`.
pub fn set_launch_at_login(product: &str, enabled: bool, executable: &Path) -> io::Result<()> {
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
