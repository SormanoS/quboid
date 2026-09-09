use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    mem::size_of,
    os::windows::ffi::OsStringExt,
    path::Path,
    sync::Mutex,
    thread::{self, JoinHandle},
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender, unbounded};
use quboid_core::{
    Action, AppConfig, LayoutEngine, NormalizedRect, Point, Rect, RuntimeCommand, RuntimeEvent,
};
use thiserror::Error;
use tracing::{debug, error};

use crate::adapters::{RuntimeAdapters, SnapRequest, SnapResolver, WindowFilter};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT},
        Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute},
        Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST,
            MONITORINFOEXW, MonitorFromWindow,
        },
        System::{
            SystemServices::SS_GRAYRECT,
            Threading::{
                OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
                QueryFullProcessImageNameW,
            },
        },
        UI::{
            Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext},
            Input::KeyboardAndMouse::{
                HOT_KEY_MODIFIERS, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DestroyWindow, DispatchMessageW, EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZESTART, GA_ROOT, GWL_EXSTYLE,
                GetAncestor, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW,
                GetWindowPlacement, GetWindowRect, GetWindowThreadProcessId, HWND_TOPMOST,
                IsWindowVisible, LWA_ALPHA, PM_REMOVE, PeekMessageW, SET_WINDOW_POS_FLAGS, SW_HIDE,
                SW_MAXIMIZE, SW_RESTORE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER,
                SWP_SHOWWINDOW, SetLayeredWindowAttributes, SetWindowPlacement, SetWindowPos,
                ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WINDOWPLACEMENT,
                WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_HOTKEY, WS_EX_LAYERED,
                WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
            },
        },
    },
    core::{BOOL, Error as WindowsError, PWSTR, w},
};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_APPLICATION_HOTKEY_ID: i32 = 0xBFFF;

pub struct Runtime {
    commands: Sender<RuntimeCommand>,
    events: Receiver<RuntimeEvent>,
    thread: Option<JoinHandle<()>>,
}

impl Runtime {
    /// Starts the runtime with the standard Base adapters.
    pub fn start(config: AppConfig) -> Result<Self, RuntimeError> {
        Self::start_with(config, RuntimeAdapters::default())
    }

    /// Starts the runtime with host-provided snap, shortcut and window adapters.
    pub fn start_with(config: AppConfig, adapters: RuntimeAdapters) -> Result<Self, RuntimeError> {
        config.validate()?;
        let (command_tx, command_rx) = unbounded();
        let (event_tx, event_rx) = unbounded();
        let thread = thread::Builder::new()
            .name("quboid-win32".to_owned())
            .spawn(move || run_message_loop(config, adapters, command_rx, event_tx))
            .map_err(RuntimeError::Thread)?;

        Ok(Self {
            commands: command_tx,
            events: event_rx,
            thread: Some(thread),
        })
    }

    pub fn commands(&self) -> Sender<RuntimeCommand> {
        self.commands.clone()
    }

    pub fn events(&self) -> Receiver<RuntimeEvent> {
        self.events.clone()
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.commands.send(RuntimeCommand::Stop);
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                error!("Win32 runtime thread panicked");
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Config(#[from] quboid_core::ConfigError),
    #[error("failed to start Win32 runtime thread: {0}")]
    Thread(#[source] std::io::Error),
}

fn run_message_loop(
    config: AppConfig,
    adapters: RuntimeAdapters,
    commands: Receiver<RuntimeCommand>,
    events: Sender<RuntimeEvent>,
) {
    let RuntimeAdapters {
        mut snap,
        mut shortcuts,
        windows: window_filter,
    } = adapters;
    let previous_dpi_context =
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let mut message = Default::default();
    unsafe {
        let _ = PeekMessageW(&mut message, None, 0, 0, PM_REMOVE);
    }

    let mut hotkeys = register_hotkeys(&config, &events);
    let mut hotkeys_suspended = false;
    let _ = events.send(RuntimeEvent::Ready);
    let mut windows = WindowManager::new(window_filter);
    windows.update_config(&config);
    let mut drag_snap = config
        .drag_snap_enabled
        .then(|| DragSnap::install(config.clone()))
        .transpose()
        .map_err(|error| {
            let _ = events.send(RuntimeEvent::Failed(format!(
                "drag snapping could not start: {error}"
            )));
        })
        .ok()
        .flatten();

    let mut running = true;
    while running {
        unsafe {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                if message.message == WM_HOTKEY {
                    if !hotkeys_suspended {
                        let id = message.wParam.0 as i32;
                        if let Some(hotkey) = hotkeys
                            .iter_mut()
                            .find(|hotkey| hotkey.registered && hotkey.id == Some(id))
                        {
                            let action = shortcuts.resolve(&hotkey.binding);
                            apply_and_report(action, &mut windows, &events);
                        }
                    }
                } else {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }

        if let Some(drag_snap) = &mut drag_snap {
            drag_snap.process_events(&mut windows, snap.as_mut());
        }

        match commands.recv_timeout(POLL_INTERVAL) {
            Ok(RuntimeCommand::Apply(action)) => {
                apply_and_report(action, &mut windows, &events);
            }
            Ok(RuntimeCommand::ApplyArea(bounds)) => match windows.apply_area(bounds) {
                Ok(rect) => {
                    let _ = events.send(RuntimeEvent::AreaApplied { rect });
                }
                Err(ApplyError::NoActiveWindow) => {
                    let _ = events.send(RuntimeEvent::NoActiveWindow);
                }
                Err(ApplyError::Windows(error)) if error.code().0 == 5 => {
                    let _ = events.send(RuntimeEvent::AccessDenied);
                }
                Err(error) => {
                    let _ = events.send(RuntimeEvent::Failed(error.to_string()));
                }
            },
            Ok(RuntimeCommand::UpdateConfig(updated)) => match updated.validate() {
                Ok(()) => {
                    reconcile_hotkeys(&mut hotkeys, &updated, &events, !hotkeys_suspended);
                    windows.update_config(&updated);
                    drag_snap = None;
                    if updated.drag_snap_enabled {
                        match DragSnap::install(updated.clone()) {
                            Ok(installed) => drag_snap = Some(installed),
                            Err(error) => {
                                let _ = events.send(RuntimeEvent::Failed(format!(
                                    "drag snapping could not start: {error}"
                                )));
                            }
                        }
                    }
                }
                Err(error) => {
                    let _ = events.send(RuntimeEvent::Failed(format!(
                        "configuration was rejected: {error}"
                    )));
                }
            },
            Ok(RuntimeCommand::SetHotkeysSuspended(suspended)) => {
                if suspended && !hotkeys_suspended {
                    hotkeys_suspended = suspend_hotkeys(&mut hotkeys, &events);
                } else if !suspended && hotkeys_suspended {
                    resume_hotkeys(&mut hotkeys, &events);
                    hotkeys_suspended = false;
                }
            }
            Ok(RuntimeCommand::Stop) | Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                running = false;
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
        }
    }

    unregister_hotkeys(&hotkeys);
    if !previous_dpi_context.0.is_null() {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(previous_dpi_context);
        }
    }
}

struct RegisteredHotkey {
    id: Option<i32>,
    combination: HotkeyCombination,
    registered: bool,
    binding: quboid_core::HotkeyBinding,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct HotkeyCombination {
    modifiers: u32,
    virtual_key: u32,
}

impl HotkeyCombination {
    fn from_binding(binding: &quboid_core::HotkeyBinding) -> Self {
        Self {
            modifiers: binding.modifiers,
            virtual_key: binding.virtual_key,
        }
    }
}

fn register_hotkeys(config: &AppConfig, events: &Sender<RuntimeEvent>) -> Vec<RegisteredHotkey> {
    let mut registered = Vec::new();
    reconcile_hotkeys(&mut registered, config, events, true);
    registered
}

fn reconcile_hotkeys(
    hotkeys: &mut Vec<RegisteredHotkey>,
    config: &AppConfig,
    events: &Sender<RuntimeEvent>,
    registration_enabled: bool,
) {
    let desired = config
        .hotkeys
        .iter()
        .map(HotkeyCombination::from_binding)
        .collect::<HashSet<_>>();

    for hotkey in hotkeys
        .iter()
        .filter(|hotkey| hotkey.registered && !desired.contains(&hotkey.combination))
    {
        if let Some(id) = hotkey.id {
            let _ = unsafe { UnregisterHotKey(None, id) };
        }
    }
    hotkeys.retain(|hotkey| desired.contains(&hotkey.combination));

    let mut used_ids = hotkeys
        .iter()
        .filter_map(|hotkey| hotkey.id)
        .collect::<HashSet<_>>();
    let mut next_id = 1;

    for binding in &config.hotkeys {
        let combination = HotkeyCombination::from_binding(binding);
        if let Some(hotkey) = hotkeys
            .iter_mut()
            .find(|hotkey| hotkey.combination == combination)
        {
            hotkey.binding = binding.clone();
            if !hotkey.registered {
                if hotkey.id.is_none() {
                    hotkey.id = allocate_hotkey_id(&mut used_ids, &mut next_id);
                }
                if registration_enabled {
                    hotkey.registered = hotkey
                        .id
                        .is_some_and(|id| register_hotkey(id, combination).is_ok());
                }
            }
            continue;
        }

        let id = allocate_hotkey_id(&mut used_ids, &mut next_id);
        let is_registered =
            registration_enabled && id.is_some_and(|id| register_hotkey(id, combination).is_ok());
        if registration_enabled && !is_registered {
            let _ = events.send(RuntimeEvent::HotkeyConflict {
                action: binding.action,
            });
        }
        hotkeys.push(RegisteredHotkey {
            id,
            combination,
            registered: is_registered,
            binding: binding.clone(),
        });
    }
}

fn suspend_hotkeys(hotkeys: &mut [RegisteredHotkey], events: &Sender<RuntimeEvent>) -> bool {
    for hotkey in hotkeys.iter_mut().filter(|hotkey| hotkey.registered) {
        if let Some(id) = hotkey.id {
            if let Err(error) = unsafe { UnregisterHotKey(None, id) } {
                resume_hotkeys(hotkeys, events);
                let _ = events.send(RuntimeEvent::Failed(format!(
                    "global hotkeys could not be suspended: {error}"
                )));
                return false;
            }
        }
        hotkey.registered = false;
    }
    true
}

fn resume_hotkeys(hotkeys: &mut [RegisteredHotkey], events: &Sender<RuntimeEvent>) {
    for hotkey in hotkeys.iter_mut().filter(|hotkey| !hotkey.registered) {
        hotkey.registered = hotkey
            .id
            .is_some_and(|id| register_hotkey(id, hotkey.combination).is_ok());
        if !hotkey.registered {
            let _ = events.send(RuntimeEvent::HotkeyConflict {
                action: hotkey.binding.action,
            });
        }
    }
}

fn allocate_hotkey_id(used_ids: &mut HashSet<i32>, next_id: &mut i32) -> Option<i32> {
    while *next_id <= MAX_APPLICATION_HOTKEY_ID {
        let id = *next_id;
        *next_id += 1;
        if used_ids.insert(id) {
            return Some(id);
        }
    }
    None
}

fn register_hotkey(id: i32, combination: HotkeyCombination) -> windows::core::Result<()> {
    let modifiers = HOT_KEY_MODIFIERS(combination.modifiers | MOD_NOREPEAT.0);
    unsafe { RegisterHotKey(None, id, modifiers, combination.virtual_key) }
}

fn unregister_hotkeys(hotkeys: &[RegisteredHotkey]) {
    for hotkey in hotkeys.iter().filter(|hotkey| hotkey.registered) {
        if let Some(id) = hotkey.id {
            let _ = unsafe { UnregisterHotKey(None, id) };
        }
    }
}

fn apply_and_report(action: Action, windows: &mut WindowManager, events: &Sender<RuntimeEvent>) {
    match windows.apply(action) {
        Ok(rect) => {
            let _ = events.send(RuntimeEvent::Applied { action, rect });
        }
        Err(ApplyError::NoActiveWindow) => {
            let _ = events.send(RuntimeEvent::NoActiveWindow);
        }
        Err(ApplyError::Windows(error)) if error.code().0 == 5 => {
            let _ = events.send(RuntimeEvent::AccessDenied);
        }
        Err(error) => {
            let _ = events.send(RuntimeEvent::Failed(error.to_string()));
        }
    }
}

struct WindowManager {
    placements: HashMap<usize, SavedPlacement>,
    gap: i32,
    filter: Box<dyn WindowFilter>,
}

impl WindowManager {
    fn new(filter: Box<dyn WindowFilter>) -> Self {
        Self {
            placements: HashMap::new(),
            gap: 0,
            filter,
        }
    }

    fn update_config(&mut self, config: &AppConfig) {
        self.gap = i32::from(config.gap.min(64));
    }

    fn is_eligible(&self, hwnd: HWND) -> bool {
        is_eligible_window(hwnd) && self.filter.allows(window_application(hwnd).as_deref())
    }

    fn apply(&mut self, action: Action) -> Result<Option<Rect>, ApplyError> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() || !self.is_eligible(hwnd) {
            return Err(ApplyError::NoActiveWindow);
        }

        let process_id = window_process_id(hwnd);
        if action == Action::Restore {
            if let Some(saved) = self.placements.remove(&(hwnd.0 as usize))
                && saved.process_id == process_id
            {
                unsafe {
                    SetWindowPlacement(hwnd, &saved.placement)?;
                }
                return Ok(None);
            }
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            return Ok(None);
        }

        self.remember_placement(hwnd, process_id)?;
        if action == Action::Maximize {
            unsafe {
                let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            }
            return Ok(None);
        }

        let geometry = window_geometry(hwnd)?;
        let monitors = enumerate_monitors()?;
        let monitor_index = select_monitor(geometry.visible, hwnd, &monitors)?;
        let current_work = monitors[monitor_index].work;

        let target = match action {
            Action::NextMonitor | Action::PreviousMonitor if monitors.len() > 1 => {
                let target_index = if action == Action::NextMonitor {
                    (monitor_index + 1) % monitors.len()
                } else {
                    (monitor_index + monitors.len() - 1) % monitors.len()
                };
                LayoutEngine::map_to_monitor(
                    geometry.visible,
                    current_work,
                    monitors[target_index].work,
                )
            }
            Action::NextMonitor | Action::PreviousMonitor => Some(geometry.visible),
            _ => LayoutEngine::target(action, geometry.visible, current_work),
        }
        .ok_or(ApplyError::InvalidGeometry)?;
        let target = match action {
            Action::Center
            | Action::AlmostMaximize
            | Action::MaximizeHeight
            | Action::Grow
            | Action::Shrink
            | Action::MoveLeft
            | Action::MoveRight
            | Action::MoveUp
            | Action::MoveDown
            | Action::NextMonitor
            | Action::PreviousMonitor => target,
            _ => target.inset(self.gap),
        };

        let outer_target = geometry.outer_for_visible(target);
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            SetWindowPos(
                hwnd,
                None,
                outer_target.left,
                outer_target.top,
                outer_target.width(),
                outer_target.height(),
                SET_WINDOW_POS_FLAGS(SWP_NOACTIVATE.0 | SWP_NOZORDER.0),
            )?;
        }
        debug!(
            ?action,
            ?target,
            monitor = %monitors[monitor_index].device_name(),
            "applied window action"
        );
        Ok(Some(target))
    }

    fn snap_dragged_window(&mut self, hwnd: HWND, target: Rect) -> Result<(), ApplyError> {
        if !self.is_eligible(hwnd) {
            return Err(ApplyError::NoActiveWindow);
        }
        let process_id = window_process_id(hwnd);
        self.remember_placement(hwnd, process_id)?;
        let geometry = window_geometry(hwnd)?;
        let outer_target = geometry.outer_for_visible(target);
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                outer_target.left,
                outer_target.top,
                outer_target.width(),
                outer_target.height(),
                SET_WINDOW_POS_FLAGS(SWP_NOACTIVATE.0 | SWP_NOZORDER.0),
            )?;
        }
        Ok(())
    }

    fn apply_area(&mut self, bounds: NormalizedRect) -> Result<Rect, ApplyError> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() || !self.is_eligible(hwnd) {
            return Err(ApplyError::NoActiveWindow);
        }
        let geometry = window_geometry(hwnd)?;
        let monitors = enumerate_monitors()?;
        let monitor_index = select_monitor(geometry.visible, hwnd, &monitors)?;
        let target = LayoutEngine::area_target(bounds, monitors[monitor_index].work)
            .ok_or(ApplyError::InvalidArea)?
            .inset(self.gap);
        self.snap_dragged_window(hwnd, target)?;
        Ok(target)
    }

    fn remember_placement(&mut self, hwnd: HWND, process_id: u32) -> Result<(), ApplyError> {
        let key = hwnd.0 as usize;
        if self
            .placements
            .get(&key)
            .is_some_and(|saved| saved.process_id == process_id)
        {
            return Ok(());
        }

        let mut placement = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        unsafe {
            GetWindowPlacement(hwnd, &mut placement)?;
        }
        self.placements.insert(
            key,
            SavedPlacement {
                process_id,
                placement,
            },
        );
        Ok(())
    }
}

struct SavedPlacement {
    process_id: u32,
    placement: WINDOWPLACEMENT,
}

#[derive(Clone, Copy)]
struct WindowGeometry {
    outer: Rect,
    visible: Rect,
}

impl WindowGeometry {
    fn outer_for_visible(self, target: Rect) -> Rect {
        let left_inset = self.visible.left - self.outer.left;
        let top_inset = self.visible.top - self.outer.top;
        let right_inset = self.outer.right - self.visible.right;
        let bottom_inset = self.outer.bottom - self.visible.bottom;
        Rect::new(
            target.left - left_inset,
            target.top - top_inset,
            target.right + right_inset,
            target.bottom + bottom_inset,
        )
    }
}

fn window_geometry(hwnd: HWND) -> Result<WindowGeometry, ApplyError> {
    let mut outer = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut outer)?;
    }
    let outer = Rect::new(outer.left, outer.top, outer.right, outer.bottom);
    if !outer.is_valid() {
        return Err(ApplyError::InvalidGeometry);
    }

    let mut visible = RECT::default();
    let visible = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut visible as *mut RECT).cast(),
            size_of::<RECT>() as u32,
        )
        .ok()
        .map(|_| Rect::new(visible.left, visible.top, visible.right, visible.bottom))
        .filter(|rect| rect.is_valid())
        .unwrap_or(outer)
    };
    Ok(WindowGeometry { outer, visible })
}

fn is_eligible_window(hwnd: HWND) -> bool {
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return false;
    }
    if unsafe { GetAncestor(hwnd, GA_ROOT) } != hwnd {
        return false;
    }
    let extended_style = WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32);
    if extended_style.0 & (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) != 0 {
        return false;
    }

    let mut cloaked = 0u32;
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            size_of::<u32>() as u32,
        )
        .is_err()
            || cloaked == 0
    }
}

fn window_process_id(hwnd: HWND) -> u32 {
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    process_id
}

fn window_application(hwnd: HWND) -> Option<String> {
    let process_id = window_process_id(hwnd);
    if process_id == 0 {
        return None;
    }
    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    unsafe {
        let _ = CloseHandle(process);
    }
    if result.is_err() {
        return None;
    }
    let path = OsString::from_wide(&buffer[..length as usize]);
    Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
}

#[derive(Clone, Copy)]
struct Monitor {
    handle: HMONITOR,
    work: Rect,
    device_name: [u16; 32],
}

impl Monitor {
    fn device_name(&self) -> String {
        let length = self
            .device_name
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(self.device_name.len());
        String::from_utf16_lossy(&self.device_name[..length])
    }
}

fn select_monitor(window: Rect, hwnd: HWND, monitors: &[Monitor]) -> Result<usize, ApplyError> {
    let best = monitors
        .iter()
        .enumerate()
        .max_by_key(|(_, monitor)| window.intersection_area(monitor.work));
    if let Some((index, monitor)) = best
        && window.intersection_area(monitor.work) > 0
    {
        return Ok(index);
    }

    let fallback = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    monitors
        .iter()
        .position(|monitor| monitor.handle == fallback)
        .ok_or(ApplyError::NoMonitor)
}

fn enumerate_monitors() -> Result<Vec<Monitor>, ApplyError> {
    let mut monitors = Vec::<Monitor>::new();
    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM((&mut monitors as *mut Vec<Monitor>) as isize),
        )
        .ok()?;
    }
    monitors.sort_by_key(|monitor| (monitor.work.left, monitor.work.top));
    if monitors.is_empty() {
        Err(ApplyError::NoMonitor)
    } else {
        Ok(monitors)
    }
}

unsafe extern "system" fn collect_monitor(
    monitor: HMONITOR,
    _device_context: HDC,
    _monitor_rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(data.0 as *mut Vec<Monitor>) };
    let mut info = MONITORINFOEXW {
        monitorInfo: windows::Win32::Graphics::Gdi::MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.as_bool() {
        monitors.push(Monitor {
            handle: monitor,
            work: Rect::new(
                info.monitorInfo.rcWork.left,
                info.monitorInfo.rcWork.top,
                info.monitorInfo.rcWork.right,
                info.monitorInfo.rcWork.bottom,
            ),
            device_name: info.szDevice,
        });
    }
    BOOL(1)
}

#[derive(Debug, Error)]
enum ApplyError {
    #[error("there is no active window")]
    NoActiveWindow,
    #[error("the active window has no monitor")]
    NoMonitor,
    #[error("window geometry is invalid")]
    InvalidGeometry,
    #[error("the requested area is outside the normalized range")]
    InvalidArea,
    #[error(transparent)]
    Windows(#[from] WindowsError),
}

static WIN_EVENT_SENDER: Mutex<Option<Sender<WinEvent>>> = Mutex::new(None);

struct DragSnap {
    _system_hook: WinEventHook,
    _location_hook: WinEventHook,
    events: Receiver<WinEvent>,
    session: Option<DragSession>,
    config: AppConfig,
    overlay: OverlayWindow,
}

impl DragSnap {
    fn install(config: AppConfig) -> Result<Self, WindowsError> {
        let overlay = OverlayWindow::new()?;
        let (sender, events) = crossbeam_channel::bounded(64);
        if let Ok(mut slot) = WIN_EVENT_SENDER.lock() {
            *slot = Some(sender);
        }

        let system_hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_MOVESIZESTART,
                EVENT_SYSTEM_MOVESIZEEND,
                None,
                Some(win_event_callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        let location_hook = unsafe {
            SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                None,
                Some(win_event_callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        if system_hook.0.is_null() || location_hook.0.is_null() {
            if !system_hook.0.is_null() {
                unsafe {
                    let _ = UnhookWinEvent(system_hook);
                }
            }
            if !location_hook.0.is_null() {
                unsafe {
                    let _ = UnhookWinEvent(location_hook);
                }
            }
            return Err(WindowsError::from_thread());
        }

        Ok(Self {
            _system_hook: WinEventHook(system_hook),
            _location_hook: WinEventHook(location_hook),
            events,
            session: None,
            config,
            overlay,
        })
    }

    fn process_events(&mut self, windows: &mut WindowManager, snap: &mut dyn SnapResolver) {
        while let Ok(event) = self.events.try_recv() {
            let hwnd = HWND(event.window as *mut core::ffi::c_void);
            match event.kind {
                EVENT_SYSTEM_MOVESIZESTART => {
                    if windows.is_eligible(hwnd) {
                        self.session = Some(DragSession {
                            hwnd,
                            target: None,
                            application: window_application(hwnd),
                        });
                        self.overlay.hide();
                    }
                }
                EVENT_OBJECT_LOCATIONCHANGE if event.object_id == 0 => {
                    if let Some(session) = &mut self.session
                        && session.hwnd == hwnd
                    {
                        session.target = snap_target_at_cursor(
                            &self.config,
                            snap,
                            session.application.as_deref(),
                        )
                        .ok()
                        .flatten();
                        if let Some(target) = session.target {
                            self.overlay.show(target);
                        } else {
                            self.overlay.hide();
                        }
                    }
                }
                EVENT_SYSTEM_MOVESIZEEND => {
                    self.overlay.hide();
                    if let Some(session) = self.session.take()
                        && session.hwnd == hwnd
                        && let Some(target) = session.target
                        && let Err(error) = windows.snap_dragged_window(hwnd, target)
                    {
                        error!(%error, "failed to snap dragged window");
                    }
                }
                _ => {}
            }
        }
    }
}

impl Drop for DragSnap {
    fn drop(&mut self) {
        if let Ok(mut slot) = WIN_EVENT_SENDER.lock() {
            *slot = None;
        }
    }
}

struct OverlayWindow {
    hwnd: HWND,
}

impl OverlayWindow {
    fn new() -> Result<Self, WindowsError> {
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(
                    WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0 | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0,
                ),
                w!("STATIC"),
                w!(""),
                WINDOW_STYLE(WS_POPUP.0 | SS_GRAYRECT.0),
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )?
        };
        unsafe {
            SetLayeredWindowAttributes(hwnd, Default::default(), 92, LWA_ALPHA)?;
        }
        Ok(Self { hwnd })
    }

    fn show(&self, target: Rect) {
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                target.left,
                target.top,
                target.width(),
                target.height(),
                SET_WINDOW_POS_FLAGS(SWP_NOACTIVATE.0 | SWP_SHOWWINDOW.0),
            );
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
    }

    fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

struct DragSession {
    hwnd: HWND,
    target: Option<Rect>,
    application: Option<String>,
}

struct WinEventHook(HWINEVENTHOOK);

impl Drop for WinEventHook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWinEvent(self.0);
        }
    }
}

#[derive(Clone, Copy)]
struct WinEvent {
    kind: u32,
    window: usize,
    object_id: i32,
}

unsafe extern "system" fn win_event_callback(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object_id: i32,
    _child_id: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if hwnd.0.is_null() {
        return;
    }
    if let Ok(sender) = WIN_EVENT_SENDER.lock()
        && let Some(sender) = sender.as_ref()
    {
        let _ = sender.try_send(WinEvent {
            kind: event,
            window: hwnd.0 as usize,
            object_id,
        });
    }
}

/// Locates the monitor under the cursor and asks the injected adapter which
/// rectangle the drag should snap to.
fn snap_target_at_cursor(
    config: &AppConfig,
    snap: &mut dyn SnapResolver,
    application: Option<&str>,
) -> Result<Option<Rect>, ApplyError> {
    let mut cursor = POINT::default();
    unsafe {
        GetCursorPos(&mut cursor)?;
    }
    let cursor = Point::new(cursor.x, cursor.y);
    let monitors = enumerate_monitors()?;
    let work = monitors
        .iter()
        .map(|monitor| monitor.work)
        .find(|work| work.contains(cursor))
        .or_else(|| {
            monitors
                .iter()
                .min_by_key(|monitor| {
                    let center_x = monitor.work.left + monitor.work.width() / 2;
                    let center_y = monitor.work.top + monitor.work.height() / 2;
                    i64::from((cursor.x - center_x).abs()) + i64::from((cursor.y - center_y).abs())
                })
                .map(|monitor| monitor.work)
        })
        .ok_or(ApplyError::NoMonitor)?;

    Ok(snap.snap_target(SnapRequest {
        cursor,
        work_area: work,
        gap: i32::from(config.gap.min(64)),
        snap_threshold: config.snap_threshold,
        application,
    }))
}
