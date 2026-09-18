use serde::{Deserialize, Serialize};

use crate::{AppConfig, NormalizedRect, Rect};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    FirstThird,
    CenterThird,
    LastThird,
    FirstTwoThirds,
    CenterTwoThirds,
    LastTwoThirds,
    CenterHalf,
    Center,
    AlmostMaximize,
    MaximizeHeight,
    Grow,
    Shrink,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Maximize,
    Restore,
    /// Puts the active window back where it was before the last placement.
    Undo,
    NextMonitor,
    PreviousMonitor,
    /// Shows every shortcut at once. The odd one out: it teaches instead of
    /// moving a window, so no window has to be active for it to mean anything.
    ShowShortcuts,
}

impl Action {
    pub const ALL: [Self; 30] = [
        Self::LeftHalf,
        Self::RightHalf,
        Self::TopHalf,
        Self::BottomHalf,
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
        Self::FirstThird,
        Self::CenterThird,
        Self::LastThird,
        Self::FirstTwoThirds,
        Self::CenterTwoThirds,
        Self::LastTwoThirds,
        Self::CenterHalf,
        Self::Center,
        Self::AlmostMaximize,
        Self::MaximizeHeight,
        Self::Grow,
        Self::Shrink,
        Self::MoveLeft,
        Self::MoveRight,
        Self::MoveUp,
        Self::MoveDown,
        Self::Maximize,
        Self::Restore,
        Self::Undo,
        Self::NextMonitor,
        Self::PreviousMonitor,
        Self::ShowShortcuts,
    ];

    pub const fn label_key(self) -> &'static str {
        match self {
            Self::LeftHalf => "left_half",
            Self::RightHalf => "right_half",
            Self::TopHalf => "top_half",
            Self::BottomHalf => "bottom_half",
            Self::TopLeft => "top_left",
            Self::TopRight => "top_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomRight => "bottom_right",
            Self::FirstThird => "first_third",
            Self::CenterThird => "center_third",
            Self::LastThird => "last_third",
            Self::FirstTwoThirds => "first_two_thirds",
            Self::CenterTwoThirds => "center_two_thirds",
            Self::LastTwoThirds => "last_two_thirds",
            Self::CenterHalf => "center_half",
            Self::Center => "center",
            Self::AlmostMaximize => "almost_maximize",
            Self::MaximizeHeight => "maximize_height",
            Self::Grow => "grow",
            Self::Shrink => "shrink",
            Self::MoveLeft => "move_left",
            Self::MoveRight => "move_right",
            Self::MoveUp => "move_up",
            Self::MoveDown => "move_down",
            Self::Maximize => "maximize",
            Self::Restore => "restore",
            Self::Undo => "undo",
            Self::NextMonitor => "next_monitor",
            Self::PreviousMonitor => "previous_monitor",
            Self::ShowShortcuts => "show_shortcuts",
        }
    }
}

#[derive(Clone, Debug)]
pub enum RuntimeCommand {
    Apply(Action),
    /// Places the active window on the given fraction of its work area.
    ApplyArea(NormalizedRect),
    UpdateConfig(AppConfig),
    SetHotkeysSuspended(bool),
    Stop,
}

#[derive(Clone, Debug)]
pub enum RuntimeEvent {
    Ready,
    Applied {
        action: Action,
        rect: Option<Rect>,
    },
    AreaApplied {
        rect: Rect,
    },
    /// The monitors changed and this many windows were put back where they were.
    ArrangementRestored {
        windows: usize,
    },
    /// The user asked to see every shortcut at once.
    ShortcutsRequested,
    /// Undo was asked for a window that has nothing left to take back.
    NothingToUndo,
    HotkeyConflict {
        action: Action,
    },
    NoActiveWindow,
    AccessDenied,
    Failed(String),
}
