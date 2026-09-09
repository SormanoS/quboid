#![cfg(target_os = "windows")]

use std::{
    mem::size_of,
    sync::{Once, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use quboid_core::{
    Action, AppConfig, ConfigError, HotkeyBinding, NormalizedRect, Rect, RuntimeCommand,
    RuntimeEvent,
};
use quboid_windows::{Runtime, RuntimeAdapters, RuntimeError, WindowFilter, set_launch_at_login};
use serial_test::serial;
use windows::{
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute},
            Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
        },
        System::Threading::{AttachThreadInput, GetCurrentThreadId},
        UI::{
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
            Input::KeyboardAndMouse::{SetActiveWindow, SetFocus},
            WindowsAndMessaging::{
                BringWindowToTop, CreateWindowExW, DestroyWindow, DispatchMessageW,
                GetForegroundWindow, GetMessageW, GetWindowRect, GetWindowThreadProcessId,
                IsZoomed, MSG, PostThreadMessageW, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
                TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_QUIT, WS_OVERLAPPEDWINDOW,
                WS_VISIBLE,
            },
        },
    },
    core::w,
};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const EVENT_TIMEOUT: Duration = Duration::from_secs(5);
/// A rectangle the runtime must reject, used as a barrier between commands.
const INVALID_AREA: NormalizedRect = NormalizedRect::new(0, 0, 0, 0);
static DPI_AWARENESS: Once = Once::new();

#[test]
#[serial]
fn runtime_moves_the_foreground_window_to_the_left_half() {
    let runtime = start_runtime(base_config());
    let window = TestWindow::new();
    window.make_foreground();
    let work = window.work_area();
    let expected = Rect::new(
        work.left,
        work.top,
        work.left + work.width() / 2,
        work.bottom,
    );

    runtime
        .commands()
        .send(RuntimeCommand::Apply(Action::LeftHalf))
        .unwrap();

    let event = receive_matching(&runtime, |event| {
        matches!(
            event,
            RuntimeEvent::Applied {
                action: Action::LeftHalf,
                ..
            }
        )
    });
    assert!(matches!(
        event,
        RuntimeEvent::Applied {
            rect: Some(rect),
            ..
        } if rect == expected
    ));
    assert_rect_close(window.visible_rect(), expected, 8);
}

#[test]
#[serial]
fn restore_returns_the_window_to_its_original_placement() {
    let runtime = start_runtime(base_config());
    let window = TestWindow::new();
    window.make_foreground();
    let original = window.visible_rect();

    runtime
        .commands()
        .send(RuntimeCommand::Apply(Action::BottomRight))
        .unwrap();
    receive_matching(&runtime, |event| {
        matches!(event, RuntimeEvent::Applied { .. })
    });
    runtime
        .commands()
        .send(RuntimeCommand::Apply(Action::Restore))
        .unwrap();
    receive_matching(&runtime, |event| {
        matches!(
            event,
            RuntimeEvent::Applied {
                action: Action::Restore,
                ..
            }
        )
    });

    assert_rect_close(window.visible_rect(), original, 8);
}

#[test]
#[serial]
fn configured_area_is_applied_with_the_configured_gap() {
    let mut config = base_config();
    config.gap = 12;
    let runtime = start_runtime(config);
    let window = TestWindow::new();
    window.make_foreground();
    let work = window.work_area();
    let expected = Rect::new(
        work.left + work.width() / 4 + 12,
        work.top + work.height() / 4 + 12,
        work.left + work.width() * 3 / 4 - 12,
        work.top + work.height() * 3 / 4 - 12,
    );

    runtime
        .commands()
        .send(RuntimeCommand::ApplyArea(NormalizedRect::new(
            2_500, 2_500, 7_500, 7_500,
        )))
        .unwrap();

    let event = receive_matching(&runtime, |event| {
        matches!(event, RuntimeEvent::AreaApplied { .. })
    });
    assert!(matches!(
        event,
        RuntimeEvent::AreaApplied { rect } if rect == expected
    ));
    assert_rect_close(window.visible_rect(), expected, 8);
}

#[test]
#[serial]
fn an_injected_window_filter_can_veto_the_active_window() {
    struct BlockEverything;

    impl WindowFilter for BlockEverything {
        fn allows(&self, _application: Option<&str>) -> bool {
            false
        }
    }

    let runtime = start_runtime_with(
        base_config(),
        RuntimeAdapters {
            windows: Box::new(BlockEverything),
            ..RuntimeAdapters::default()
        },
    );
    let window = TestWindow::new();
    window.make_foreground();
    let original = window.visible_rect();

    runtime
        .commands()
        .send(RuntimeCommand::Apply(Action::LeftHalf))
        .unwrap();

    assert!(matches!(
        receive_matching(&runtime, |event| matches!(
            event,
            RuntimeEvent::NoActiveWindow
        )),
        RuntimeEvent::NoActiveWindow
    ));
    assert_rect_close(window.visible_rect(), original, 0);
}

#[test]
#[serial]
fn an_injected_window_filter_also_vetoes_an_applied_area() {
    struct BlockEverything;

    impl WindowFilter for BlockEverything {
        fn allows(&self, _application: Option<&str>) -> bool {
            false
        }
    }

    let runtime = start_runtime_with(
        base_config(),
        RuntimeAdapters {
            windows: Box::new(BlockEverything),
            ..RuntimeAdapters::default()
        },
    );
    let window = TestWindow::new();
    window.make_foreground();
    let original = window.visible_rect();

    runtime
        .commands()
        .send(RuntimeCommand::ApplyArea(NormalizedRect::new(
            2_500, 2_500, 7_500, 7_500,
        )))
        .unwrap();

    assert!(matches!(
        receive_matching(&runtime, |event| matches!(
            event,
            RuntimeEvent::NoActiveWindow
        )),
        RuntimeEvent::NoActiveWindow
    ));
    assert_rect_close(window.visible_rect(), original, 0);
}

#[test]
#[serial]
fn updating_configuration_registers_a_new_global_hotkey() {
    let first = start_runtime(base_config());
    let mut updated = base_config();
    updated.hotkeys.push(HotkeyBinding {
        action: Action::Center,
        modifiers: 0x0002 | 0x0001,
        virtual_key: 0x7C,
    });
    first
        .commands()
        .send(RuntimeCommand::UpdateConfig(updated.clone()))
        .unwrap();
    first
        .commands()
        .send(RuntimeCommand::ApplyArea(INVALID_AREA))
        .unwrap();
    receive_matching(&first, |event| matches!(event, RuntimeEvent::Failed(_)));

    let second = Runtime::start(updated).unwrap();
    let conflict = receive_matching(&second, |event| {
        matches!(
            event,
            RuntimeEvent::HotkeyConflict {
                action: Action::Center
            }
        )
    });

    assert!(matches!(
        conflict,
        RuntimeEvent::HotkeyConflict {
            action: Action::Center
        }
    ));
}

#[test]
#[serial]
fn updating_configuration_does_not_repeat_an_unchanged_hotkey_conflict() {
    let mut config = base_config();
    config.hotkeys.push(HotkeyBinding {
        action: Action::Center,
        modifiers: 0x0002 | 0x0001,
        virtual_key: 0x7D,
    });
    let _owner = start_runtime(config.clone());
    let contender = Runtime::start(config.clone()).unwrap();

    assert!(matches!(
        receive_matching(&contender, |event| matches!(
            event,
            RuntimeEvent::HotkeyConflict {
                action: Action::Center
            }
        )),
        RuntimeEvent::HotkeyConflict {
            action: Action::Center
        }
    ));
    receive_matching(&contender, |event| matches!(event, RuntimeEvent::Ready));

    contender
        .commands()
        .send(RuntimeCommand::UpdateConfig(config))
        .unwrap();
    contender
        .commands()
        .send(RuntimeCommand::ApplyArea(INVALID_AREA))
        .unwrap();

    let events = contender.events();
    match events.recv_timeout(EVENT_TIMEOUT).unwrap() {
        RuntimeEvent::HotkeyConflict { action } => {
            panic!("unchanged conflict was repeated for {action:?}");
        }
        RuntimeEvent::Failed(_) => {}
        event => panic!("unexpected runtime event: {event:?}"),
    }
}

#[test]
#[serial]
fn suspending_hotkeys_releases_them_until_the_runtime_resumes() {
    let mut config = base_config();
    config.hotkeys.push(HotkeyBinding {
        action: Action::Center,
        modifiers: 0x0002 | 0x0001,
        virtual_key: 0x7C,
    });
    let first = start_runtime(config.clone());

    first
        .commands()
        .send(RuntimeCommand::SetHotkeysSuspended(true))
        .unwrap();
    first
        .commands()
        .send(RuntimeCommand::ApplyArea(INVALID_AREA))
        .unwrap();
    receive_matching(&first, |event| matches!(event, RuntimeEvent::Failed(_)));

    let _second = start_runtime(config);
    first
        .commands()
        .send(RuntimeCommand::SetHotkeysSuspended(false))
        .unwrap();

    assert!(matches!(
        receive_matching(&first, |event| matches!(
            event,
            RuntimeEvent::HotkeyConflict {
                action: Action::Center
            }
        )),
        RuntimeEvent::HotkeyConflict {
            action: Action::Center
        }
    ));
}

#[test]
#[serial]
fn maximize_and_restore_publish_state_changes_and_update_the_window() {
    let runtime = start_runtime(base_config());
    let window = TestWindow::new();
    window.make_foreground();

    runtime
        .commands()
        .send(RuntimeCommand::Apply(Action::Maximize))
        .unwrap();
    assert!(matches!(
        receive_matching(&runtime, |event| matches!(
            event,
            RuntimeEvent::Applied {
                action: Action::Maximize,
                ..
            }
        )),
        RuntimeEvent::Applied { rect: None, .. }
    ));
    assert!(unsafe { IsZoomed(window.hwnd) }.as_bool());

    runtime
        .commands()
        .send(RuntimeCommand::Apply(Action::Restore))
        .unwrap();
    receive_matching(&runtime, |event| {
        matches!(
            event,
            RuntimeEvent::Applied {
                action: Action::Restore,
                ..
            }
        )
    });
    assert!(!unsafe { IsZoomed(window.hwnd) }.as_bool());
}

#[test]
fn invalid_configuration_is_rejected_before_starting_a_runtime_thread() {
    let config = AppConfig {
        schema_version: 0,
        ..AppConfig::default()
    };

    assert!(matches!(
        Runtime::start(config),
        Err(RuntimeError::Config(ConfigError::UnsupportedVersion(0)))
    ));
}

#[test]
#[serial]
fn an_invalid_area_is_reported_as_a_runtime_failure() {
    let runtime = start_runtime(base_config());
    let window = TestWindow::new();
    window.make_foreground();

    runtime
        .commands()
        .send(RuntimeCommand::ApplyArea(INVALID_AREA))
        .unwrap();

    assert!(matches!(
        receive_matching(&runtime, |event| matches!(event, RuntimeEvent::Failed(_))),
        RuntimeEvent::Failed(_)
    ));
}

#[test]
#[serial]
fn launch_at_login_writes_and_removes_the_current_user_run_value() {
    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = current_user.create_subkey(RUN_KEY).unwrap();
    let original = run.get_value::<String, _>("Quboid").ok();
    let _restore = RegistryValueRestore {
        original: original.clone(),
    };
    let executable = std::env::current_exe().unwrap();

    set_launch_at_login("Quboid", true, &executable).unwrap();
    assert_eq!(
        run.get_value::<String, _>("Quboid").unwrap(),
        format!("\"{}\"", executable.display())
    );

    set_launch_at_login("Quboid", false, &executable).unwrap();
    assert!(run.get_value::<String, _>("Quboid").is_err());
}

fn base_config() -> AppConfig {
    AppConfig {
        drag_snap_enabled: false,
        hotkeys: Vec::new(),
        ..AppConfig::default()
    }
}

fn start_runtime(config: AppConfig) -> Runtime {
    start_runtime_with(config, RuntimeAdapters::default())
}

fn start_runtime_with(config: AppConfig, adapters: RuntimeAdapters) -> Runtime {
    DPI_AWARENESS.call_once(|| unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    });
    let runtime = Runtime::start_with(config, adapters).unwrap();
    assert!(matches!(
        receive_matching(&runtime, |event| matches!(event, RuntimeEvent::Ready)),
        RuntimeEvent::Ready
    ));
    runtime
}

fn receive_matching(runtime: &Runtime, predicate: impl Fn(&RuntimeEvent) -> bool) -> RuntimeEvent {
    let events = runtime.events();
    loop {
        let event = events
            .recv_timeout(EVENT_TIMEOUT)
            .expect("runtime event timed out");
        if predicate(&event) {
            return event;
        }
        if let RuntimeEvent::Failed(message) = event {
            panic!("runtime failed: {message}");
        }
        if matches!(
            event,
            RuntimeEvent::NoActiveWindow
                | RuntimeEvent::AccessDenied
                | RuntimeEvent::Applied { .. }
                | RuntimeEvent::AreaApplied { .. }
        ) {
            panic!("unexpected runtime event: {event:?}");
        }
    }
}

struct TestWindow {
    hwnd: HWND,
    owner_thread: u32,
    thread: Option<JoinHandle<()>>,
}

impl TestWindow {
    fn new() -> Self {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("quboid-test-window".to_owned())
            .spawn(move || {
                let hwnd = unsafe {
                    CreateWindowExW(
                        WINDOW_EX_STYLE::default(),
                        w!("STATIC"),
                        w!("Quboid runtime integration test"),
                        WINDOW_STYLE(WS_OVERLAPPEDWINDOW.0 | WS_VISIBLE.0),
                        200,
                        180,
                        800,
                        500,
                        None,
                        None,
                        None,
                        None,
                    )
                    .unwrap()
                };
                unsafe {
                    let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
                }
                ready_tx
                    .send((hwnd.0 as usize, unsafe { GetCurrentThreadId() }))
                    .unwrap();

                let mut message = MSG::default();
                while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
                    unsafe {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
            })
            .unwrap();
        let (window, owner_thread) = ready_rx.recv_timeout(EVENT_TIMEOUT).unwrap();
        Self {
            hwnd: HWND(window as *mut core::ffi::c_void),
            owner_thread,
            thread: Some(thread),
        }
    }

    fn make_foreground(&self) {
        let current_thread = unsafe { GetCurrentThreadId() };
        let foreground = unsafe { GetForegroundWindow() };
        let foreground_thread = if foreground.0.is_null() {
            current_thread
        } else {
            unsafe { GetWindowThreadProcessId(foreground, None) }
        };
        if foreground_thread != current_thread {
            unsafe {
                assert!(AttachThreadInput(current_thread, foreground_thread, true).as_bool());
            }
        }
        if self.owner_thread != current_thread && self.owner_thread != foreground_thread {
            unsafe {
                assert!(AttachThreadInput(current_thread, self.owner_thread, true).as_bool());
            }
        }
        unsafe {
            BringWindowToTop(self.hwnd).unwrap();
            let _ = SetActiveWindow(self.hwnd);
            let _ = SetFocus(Some(self.hwnd));
            let _ = SetForegroundWindow(self.hwnd);
        }
        assert_eq!(unsafe { GetForegroundWindow() }, self.hwnd);
        if self.owner_thread != current_thread && self.owner_thread != foreground_thread {
            unsafe {
                assert!(AttachThreadInput(current_thread, self.owner_thread, false).as_bool());
            }
        }
        if foreground_thread != current_thread {
            unsafe {
                assert!(AttachThreadInput(current_thread, foreground_thread, false).as_bool());
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    fn work_area(&self) -> Rect {
        let monitor = unsafe { MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONEAREST) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        assert!(unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool());
        Rect::new(
            info.rcWork.left,
            info.rcWork.top,
            info.rcWork.right,
            info.rcWork.bottom,
        )
    }

    fn visible_rect(&self) -> Rect {
        let mut rect = RECT::default();
        if unsafe {
            DwmGetWindowAttribute(
                self.hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&mut rect as *mut RECT).cast(),
                size_of::<RECT>() as u32,
            )
        }
        .is_err()
        {
            unsafe {
                GetWindowRect(self.hwnd, &mut rect).unwrap();
            }
        }
        Rect::new(rect.left, rect.top, rect.right, rect.bottom)
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        unsafe {
            PostThreadMessageW(
                self.owner_thread,
                WM_QUIT,
                Default::default(),
                Default::default(),
            )
            .unwrap();
        }
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

fn assert_rect_close(actual: Rect, expected: Rect, tolerance: i32) {
    assert!(
        (actual.left - expected.left).abs() <= tolerance,
        "left: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.top - expected.top).abs() <= tolerance,
        "top: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.right - expected.right).abs() <= tolerance,
        "right: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.bottom - expected.bottom).abs() <= tolerance,
        "bottom: {actual:?} != {expected:?}"
    );
}

struct RegistryValueRestore {
    original: Option<String>,
}

impl Drop for RegistryValueRestore {
    fn drop(&mut self) {
        const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        let Ok((run, _)) = current_user.create_subkey(RUN_KEY) else {
            return;
        };
        if let Some(original) = &self.original {
            let _ = run.set_value("Quboid", original);
        } else {
            let _ = run.delete_value("Quboid");
        }
    }
}
