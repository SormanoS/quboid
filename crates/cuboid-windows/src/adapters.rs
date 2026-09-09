use cuboid_core::{Action, HotkeyBinding, LayoutEngine, NORMALIZED_SCALE, Point, Rect};

/// Everything an adapter needs to resolve a drag into a target rectangle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapRequest<'a> {
    /// Cursor position, in virtual desktop coordinates.
    pub cursor: Point,
    /// Work area of the monitor under the cursor.
    pub work_area: Rect,
    /// Gap to leave around the snapped window, in pixels.
    pub gap: i32,
    /// Edge sensitivity, in `NORMALIZED_SCALE` units.
    pub snap_threshold: u16,
    /// Executable file name of the dragged window, lowercased.
    pub application: Option<&'a str>,
}

impl SnapRequest<'_> {
    /// Edge sensitivity resolved against the current work area, in pixels.
    pub fn threshold_pixels(&self) -> i32 {
        (i32::from(self.snap_threshold) * self.work_area.width().min(self.work_area.height())
            / i32::from(NORMALIZED_SCALE))
        .clamp(8, 96)
    }

    /// Whether the cursor is close enough to an edge for snapping to engage.
    pub fn near_edge(&self) -> bool {
        let edges = self.edges();
        edges.left || edges.right || edges.top || edges.bottom
    }

    fn edges(&self) -> Edges {
        let threshold = self.threshold_pixels();
        Edges {
            left: self.cursor.x - self.work_area.left <= threshold,
            right: self.work_area.right - self.cursor.x <= threshold,
            top: self.cursor.y - self.work_area.top <= threshold,
            bottom: self.work_area.bottom - self.cursor.y <= threshold,
        }
    }
}

#[derive(Clone, Copy)]
struct Edges {
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
}

/// Turns a drag into the rectangle the window should occupy.
pub trait SnapResolver: Send {
    fn snap_target(&mut self, request: SnapRequest<'_>) -> Option<Rect>;
}

/// Snaps to the standard halves, corners and maximized area.
#[derive(Clone, Copy, Debug, Default)]
pub struct EdgeSnapResolver;

impl SnapResolver for EdgeSnapResolver {
    fn snap_target(&mut self, request: SnapRequest<'_>) -> Option<Rect> {
        let work = request.work_area;
        let edges = request.edges();
        let action = match (edges.left, edges.right, edges.top, edges.bottom) {
            (true, _, true, _) => Some(Action::TopLeft),
            (_, true, true, _) => Some(Action::TopRight),
            (true, _, _, true) => Some(Action::BottomLeft),
            (_, true, _, true) => Some(Action::BottomRight),
            (true, _, _, _) => Some(Action::LeftHalf),
            (_, true, _, _) => Some(Action::RightHalf),
            (_, _, true, _) => Some(Action::Maximize),
            (_, _, _, true) => Some(Action::BottomHalf),
            _ => None,
        }?;
        if action == Action::Maximize {
            return Some(work);
        }
        LayoutEngine::target(action, work, work).map(|target| target.inset(request.gap))
    }
}

/// Chooses the action a pressed shortcut triggers.
pub trait ShortcutResolver: Send {
    fn resolve(&mut self, binding: &HotkeyBinding) -> Action;
}

/// Every shortcut always triggers the action it is bound to.
#[derive(Clone, Copy, Debug, Default)]
pub struct SingleActionShortcuts;

impl ShortcutResolver for SingleActionShortcuts {
    fn resolve(&mut self, binding: &HotkeyBinding) -> Action {
        binding.action
    }
}

/// Decides whether Cuboid may move a window, given its executable file name.
pub trait WindowFilter: Send {
    fn allows(&self, application: Option<&str>) -> bool;
}

/// Every eligible window may be managed.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllWindows;

impl WindowFilter for AllWindows {
    fn allows(&self, _application: Option<&str>) -> bool {
        true
    }
}

/// The adapters a runtime instance uses.
pub struct RuntimeAdapters {
    pub snap: Box<dyn SnapResolver>,
    pub shortcuts: Box<dyn ShortcutResolver>,
    pub windows: Box<dyn WindowFilter>,
}

impl Default for RuntimeAdapters {
    fn default() -> Self {
        Self {
            snap: Box::new(EdgeSnapResolver),
            shortcuts: Box::new(SingleActionShortcuts),
            windows: Box::new(AllWindows),
        }
    }
}

impl std::fmt::Debug for RuntimeAdapters {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RuntimeAdapters")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect::new(0, 0, 1_920, 1_080);

    fn request(x: i32, y: i32, gap: i32) -> SnapRequest<'static> {
        SnapRequest {
            cursor: Point::new(x, y),
            work_area: WORK,
            gap,
            snap_threshold: 500,
            application: None,
        }
    }

    #[test]
    fn left_edge_snaps_to_the_left_half() {
        assert_eq!(
            EdgeSnapResolver.snap_target(request(2, 540, 0)),
            Some(Rect::new(0, 0, 960, 1_080))
        );
    }

    #[test]
    fn corners_win_over_single_edges_and_honour_the_gap() {
        assert_eq!(
            EdgeSnapResolver.snap_target(request(2, 2, 10)),
            Some(Rect::new(10, 10, 950, 530))
        );
    }

    #[test]
    fn the_top_edge_maximizes_without_a_gap() {
        assert_eq!(
            EdgeSnapResolver.snap_target(request(960, 1, 10)),
            Some(WORK)
        );
    }

    #[test]
    fn the_centre_of_the_work_area_never_snaps() {
        assert_eq!(EdgeSnapResolver.snap_target(request(960, 540, 0)), None);
        assert!(!request(960, 540, 0).near_edge());
    }

    #[test]
    fn a_single_action_shortcut_always_resolves_to_its_binding() {
        let binding = HotkeyBinding {
            action: Action::LeftHalf,
            modifiers: 0x0003,
            virtual_key: 0x25,
        };

        assert_eq!(SingleActionShortcuts.resolve(&binding), Action::LeftHalf);
        assert_eq!(SingleActionShortcuts.resolve(&binding), Action::LeftHalf);
    }

    #[test]
    fn every_window_is_managed_by_default() {
        assert!(AllWindows.allows(Some("editor.exe")));
        assert!(AllWindows.allows(None));
    }
}
