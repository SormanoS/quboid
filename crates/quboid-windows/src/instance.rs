//! Keeps one Quboid to a Windows session.
//!
//! Global shortcuts cannot be shared. `RegisterHotKey` hands a combination to
//! one thread and refuses everyone after it, so a second Quboid comes up
//! looking healthy, with a tray icon and a settings window, while owning none
//! of the shortcuts the first one already took. An upgrade is enough to reach
//! that state: the installer replaces the executable without stopping what is
//! running.

use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, WPARAM},
        System::Threading::CreateMutexW,
        UI::WindowsAndMessaging::{
            FindWindowExW, HWND_MESSAGE, PostMessageW, RegisterWindowMessageW,
        },
    },
    core::{HSTRING, w},
};

/// Title of the hidden window a running Quboid answers on.
///
/// A message-only window is never seen and never listed, so the name only has
/// to be unlikely to collide with somebody else's.
pub(crate) const SIGNAL_WINDOW_TITLE: &str = "Quboid message sink";

/// Name of the message a second instance sends to ask the first to show itself.
///
/// `RegisterWindowMessageW` turns it into an identifier every process agrees
/// on, without any of them owning it.
const SHOW_MESSAGE_NAME: windows::core::PCWSTR = w!("QuboidShowSettings");

/// Proof that this process is the only Quboid in the session.
///
/// The claim lasts as long as the value does: dropping it, or the process
/// ending for any reason, lets the next Quboid through.
#[derive(Debug)]
pub struct SingleInstance {
    handle: HANDLE,
}

impl SingleInstance {
    /// Claims the session for this process, or reports that another Quboid
    /// already holds it.
    ///
    /// The name is scoped to the session, so two people signed in at once each
    /// get their own Quboid.
    #[must_use]
    pub fn acquire(product: &str) -> Option<Self> {
        let name = HSTRING::from(format!("Local\\{product}-single-instance"));
        let handle = unsafe { CreateMutexW(None, true, &name) };
        match handle {
            Ok(handle) => {
                // The handle is returned either way; only the error says whether
                // this process is the one that created the mutex.
                if unsafe { windows::Win32::Foundation::GetLastError() } == ERROR_ALREADY_EXISTS {
                    let _ = unsafe { CloseHandle(handle) };
                    None
                } else {
                    Some(Self { handle })
                }
            }
            Err(_) => {
                // Without the mutex there is no way to tell, and refusing to
                // start would be worse than starting twice.
                Some(Self {
                    handle: HANDLE::default(),
                })
            }
        }
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            let _ = unsafe { CloseHandle(self.handle) };
        }
    }
}

/// Asks the Quboid that is already running to bring its window to the front.
///
/// Returns whether anything was there to listen. A second launch is somebody
/// looking for the settings window, so handing them the one that exists is the
/// whole point of stopping the second instance.
pub fn show_running_instance() -> bool {
    let title = HSTRING::from(SIGNAL_WINDOW_TITLE);
    let Ok(hwnd) = (unsafe { FindWindowExW(Some(HWND_MESSAGE), None, w!("STATIC"), &title) })
    else {
        return false;
    };
    if hwnd.is_invalid() {
        return false;
    }
    unsafe { PostMessageW(Some(hwnd), show_message(), WPARAM(0), LPARAM(0)) }.is_ok()
}

/// The identifier every Quboid resolves [`SHOW_MESSAGE_NAME`] to.
pub(crate) fn show_message() -> u32 {
    unsafe { RegisterWindowMessageW(SHOW_MESSAGE_NAME) }
}

/// A window that exists only to be posted to.
///
/// It has no pixels and no parent on the desktop, so it costs nothing and
/// cannot be stumbled over. Messages posted to it land in the queue of the
/// thread that made it, which is the runtime loop.
pub(crate) struct SignalWindow {
    hwnd: HWND,
}

impl SignalWindow {
    pub(crate) fn new() -> Option<Self> {
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, WINDOW_EX_STYLE, WINDOW_STYLE,
        };

        let title = HSTRING::from(SIGNAL_WINDOW_TITLE);
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                &title,
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        };
        hwnd.ok()
            .filter(|hwnd| !hwnd.is_invalid())
            .map(|hwnd| Self { hwnd })
    }
}

impl Drop for SignalWindow {
    fn drop(&mut self) {
        use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;

        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use windows::Win32::UI::WindowsAndMessaging::{MSG, PM_REMOVE, PeekMessageW};

    #[test]
    #[serial]
    fn the_second_quboid_is_turned_away_until_the_first_lets_go() {
        let first = SingleInstance::acquire("Quboid-test-turned-away")
            .expect("nothing else holds this name");

        assert!(
            SingleInstance::acquire("Quboid-test-turned-away").is_none(),
            "a second instance must not claim a session that is already taken"
        );

        drop(first);
        assert!(
            SingleInstance::acquire("Quboid-test-turned-away").is_some(),
            "the session must be free again once the first instance is gone"
        );
    }

    #[test]
    #[serial]
    fn two_products_do_not_get_in_each_others_way() {
        let _one = SingleInstance::acquire("Quboid-test-product-one").expect("free name");
        assert!(SingleInstance::acquire("Quboid-test-product-two").is_some());
    }

    #[test]
    #[serial]
    fn a_launch_with_nobody_listening_is_reported_as_such() {
        assert!(
            !show_running_instance(),
            "no signal window exists in this test process yet"
        );
    }

    #[test]
    #[serial]
    fn the_running_instance_hears_the_launch_that_was_handed_to_it() {
        let _window = SignalWindow::new().expect("the message-only window could be created");

        assert!(show_running_instance(), "the window should have been found");

        // The message is posted to a window this thread owns, so it lands in
        // this thread's queue, which is where the runtime loop reads it.
        let wanted = show_message();
        let mut message = MSG::default();
        let arrived = unsafe { PeekMessageW(&mut message, None, wanted, wanted, PM_REMOVE) };
        assert!(arrived.as_bool(), "the posted message never arrived");
    }
}
