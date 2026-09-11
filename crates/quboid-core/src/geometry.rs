use serde::{Deserialize, Serialize};

use crate::Action;

pub const NORMALIZED_SCALE: u16 = 10_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// A rectangle expressed as a fraction of a work area, in `NORMALIZED_SCALE` units.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedRect {
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}

impl NormalizedRect {
    pub const fn new(left: u16, top: u16, right: u16, bottom: u16) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.left < self.right
            && self.top < self.bottom
            && self.right <= NORMALIZED_SCALE
            && self.bottom <= NORMALIZED_SCALE
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn width(self) -> i32 {
        self.right - self.left
    }

    pub const fn height(self) -> i32 {
        self.bottom - self.top
    }

    pub const fn is_valid(self) -> bool {
        self.right > self.left && self.bottom > self.top
    }

    pub fn intersection_area(self, other: Self) -> i64 {
        let width = (self.right.min(other.right) - self.left.max(other.left)).max(0);
        let height = (self.bottom.min(other.bottom) - self.top.max(other.top)).max(0);
        i64::from(width) * i64::from(height)
    }

    pub const fn contains(self, point: Point) -> bool {
        point.x >= self.left && point.x < self.right && point.y >= self.top && point.y < self.bottom
    }

    /// Shrinks the rectangle by `gap` on every side, keeping it unchanged when the
    /// gap would consume it.
    pub const fn inset(self, gap: i32) -> Self {
        if gap <= 0 || self.width() <= gap * 2 || self.height() <= gap * 2 {
            self
        } else {
            Self::new(
                self.left + gap,
                self.top + gap,
                self.right - gap,
                self.bottom - gap,
            )
        }
    }
}

pub struct LayoutEngine;

impl LayoutEngine {
    pub fn target(action: Action, current: Rect, work: Rect) -> Option<Rect> {
        if !current.is_valid() || !work.is_valid() {
            return None;
        }

        let x = Self::boundaries(work.left, work.right, 2);
        let y = Self::boundaries(work.top, work.bottom, 2);
        let thirds = Self::boundaries(work.left, work.right, 3);

        let result = match action {
            Action::LeftHalf => Rect::new(x[0], y[0], x[1], y[2]),
            Action::RightHalf => Rect::new(x[1], y[0], x[2], y[2]),
            Action::TopHalf => Rect::new(x[0], y[0], x[2], y[1]),
            Action::BottomHalf => Rect::new(x[0], y[1], x[2], y[2]),
            Action::TopLeft => Rect::new(x[0], y[0], x[1], y[1]),
            Action::TopRight => Rect::new(x[1], y[0], x[2], y[1]),
            Action::BottomLeft => Rect::new(x[0], y[1], x[1], y[2]),
            Action::BottomRight => Rect::new(x[1], y[1], x[2], y[2]),
            Action::FirstThird => Rect::new(thirds[0], work.top, thirds[1], work.bottom),
            Action::CenterThird => Rect::new(thirds[1], work.top, thirds[2], work.bottom),
            Action::LastThird => Rect::new(thirds[2], work.top, thirds[3], work.bottom),
            Action::FirstTwoThirds => Rect::new(thirds[0], work.top, thirds[2], work.bottom),
            Action::CenterTwoThirds => {
                let width = work.width() * 2 / 3;
                let left = work.left + (work.width() - width) / 2;
                Rect::new(left, work.top, left + width, work.bottom)
            }
            Action::LastTwoThirds => Rect::new(thirds[1], work.top, thirds[3], work.bottom),
            Action::CenterHalf => {
                let width = work.width() / 2;
                let left = work.left + (work.width() - width) / 2;
                Rect::new(left, work.top, left + width, work.bottom)
            }
            Action::Center => {
                let width = current.width().min(work.width());
                let height = current.height().min(work.height());
                let left = work.left + (work.width() - width) / 2;
                let top = work.top + (work.height() - height) / 2;
                Rect::new(left, top, left + width, top + height)
            }
            Action::AlmostMaximize => {
                let width = work.width() * 9 / 10;
                let height = work.height() * 9 / 10;
                let left = work.left + (work.width() - width) / 2;
                let top = work.top + (work.height() - height) / 2;
                Rect::new(left, top, left + width, top + height)
            }
            Action::MaximizeHeight => Self::clamp(
                Rect::new(current.left, work.top, current.right, work.bottom),
                work,
            ),
            Action::Grow => Self::resize_around_center(current, work, 120, 100),
            Action::Shrink => Self::resize_around_center(current, work, 80, 100),
            Action::MoveLeft => Self::translate(current, work, -work.width() / 20, 0),
            Action::MoveRight => Self::translate(current, work, work.width() / 20, 0),
            Action::MoveUp => Self::translate(current, work, 0, -work.height() / 20),
            Action::MoveDown => Self::translate(current, work, 0, work.height() / 20),
            Action::Maximize | Action::Restore | Action::NextMonitor | Action::PreviousMonitor => {
                return None;
            }
        };

        result.is_valid().then_some(result)
    }

    /// Maps a normalized area onto a work area, producing an absolute rectangle.
    pub fn area_target(bounds: NormalizedRect, work: Rect) -> Option<Rect> {
        if !bounds.is_valid() || !work.is_valid() {
            return None;
        }
        let scale = i64::from(NORMALIZED_SCALE);
        let x =
            |value: u16| i64::from(work.left) + i64::from(work.width()) * i64::from(value) / scale;
        let y =
            |value: u16| i64::from(work.top) + i64::from(work.height()) * i64::from(value) / scale;
        let target = Rect::new(
            x(bounds.left) as i32,
            y(bounds.top) as i32,
            x(bounds.right) as i32,
            y(bounds.bottom) as i32,
        );
        target.is_valid().then_some(target)
    }

    pub fn map_to_monitor(current: Rect, from: Rect, to: Rect) -> Option<Rect> {
        if !current.is_valid() || !from.is_valid() || !to.is_valid() {
            return None;
        }

        let relative_left = f64::from(current.left - from.left) / f64::from(from.width());
        let relative_top = f64::from(current.top - from.top) / f64::from(from.height());
        let relative_width = f64::from(current.width()) / f64::from(from.width());
        let relative_height = f64::from(current.height()) / f64::from(from.height());

        let width = (relative_width * f64::from(to.width()))
            .round()
            .clamp(1.0, f64::from(to.width())) as i32;
        let height = (relative_height * f64::from(to.height()))
            .round()
            .clamp(1.0, f64::from(to.height())) as i32;
        let left = (f64::from(to.left) + relative_left * f64::from(to.width())).round() as i32;
        let top = (f64::from(to.top) + relative_top * f64::from(to.height())).round() as i32;

        Some(Self::clamp(
            Rect::new(left, top, left + width, top + height),
            to,
        ))
    }

    fn boundaries(start: i32, end: i32, parts: usize) -> Vec<i32> {
        let span = i64::from(end - start);
        (0..=parts)
            .map(|index| {
                i64::from(start)
                    + span * i64::try_from(index).unwrap_or(0) / i64::try_from(parts).unwrap_or(1)
            })
            .map(|value| value as i32)
            .collect()
    }

    fn resize_around_center(current: Rect, work: Rect, numerator: i32, denominator: i32) -> Rect {
        let width = (current.width() * numerator / denominator).clamp(1, work.width());
        let height = (current.height() * numerator / denominator).clamp(1, work.height());
        let center_x = current.left + current.width() / 2;
        let center_y = current.top + current.height() / 2;
        Self::clamp(
            Rect::new(
                center_x - width / 2,
                center_y - height / 2,
                center_x - width / 2 + width,
                center_y - height / 2 + height,
            ),
            work,
        )
    }

    fn translate(current: Rect, work: Rect, horizontal: i32, vertical: i32) -> Rect {
        Self::clamp(
            Rect::new(
                current.left + horizontal,
                current.top + vertical,
                current.right + horizontal,
                current.bottom + vertical,
            ),
            work,
        )
    }

    fn clamp(rect: Rect, bounds: Rect) -> Rect {
        let width = rect.width().min(bounds.width());
        let height = rect.height().min(bounds.height());
        let left = rect.left.clamp(bounds.left, bounds.right - width);
        let top = rect.top.clamp(bounds.top, bounds.bottom - height);
        Rect::new(left, top, left + width, top + height)
    }
}
