use proptest::prelude::*;
use quboid_core::{Action, LayoutEngine, Rect};

#[test]
fn standard_actions_match_known_negative_coordinate_layout() {
    let work = Rect::new(-1920, 0, 0, 1080);
    let current = Rect::new(-1500, 100, -900, 700);
    let cases = [
        (Action::LeftHalf, Rect::new(-1920, 0, -960, 1080)),
        (Action::RightHalf, Rect::new(-960, 0, 0, 1080)),
        (Action::TopHalf, Rect::new(-1920, 0, 0, 540)),
        (Action::BottomHalf, Rect::new(-1920, 540, 0, 1080)),
        (Action::TopLeft, Rect::new(-1920, 0, -960, 540)),
        (Action::TopRight, Rect::new(-960, 0, 0, 540)),
        (Action::BottomLeft, Rect::new(-1920, 540, -960, 1080)),
        (Action::BottomRight, Rect::new(-960, 540, 0, 1080)),
        (Action::FirstThird, Rect::new(-1920, 0, -1280, 1080)),
        (Action::CenterThird, Rect::new(-1280, 0, -640, 1080)),
        (Action::LastThird, Rect::new(-640, 0, 0, 1080)),
        (Action::FirstTwoThirds, Rect::new(-1920, 0, -640, 1080)),
        (Action::CenterTwoThirds, Rect::new(-1600, 0, -320, 1080)),
        (Action::LastTwoThirds, Rect::new(-1280, 0, 0, 1080)),
        (Action::CenterHalf, Rect::new(-1440, 0, -480, 1080)),
        (Action::Center, Rect::new(-1260, 240, -660, 840)),
        (Action::AlmostMaximize, Rect::new(-1824, 54, -96, 1026)),
        (Action::MaximizeHeight, Rect::new(-1500, 0, -900, 1080)),
        (Action::Grow, Rect::new(-1560, 40, -840, 760)),
        (Action::Shrink, Rect::new(-1440, 160, -960, 640)),
        (Action::MoveLeft, Rect::new(-1596, 100, -996, 700)),
        (Action::MoveRight, Rect::new(-1404, 100, -804, 700)),
        (Action::MoveUp, Rect::new(-1500, 46, -900, 646)),
        (Action::MoveDown, Rect::new(-1500, 154, -900, 754)),
    ];

    for (action, expected) in cases {
        assert_eq!(
            LayoutEngine::target(action, current, work),
            Some(expected),
            "{action:?}"
        );
    }
}

#[test]
fn system_actions_do_not_claim_a_geometric_target() {
    let work = Rect::new(0, 0, 1920, 1080);
    let current = Rect::new(100, 100, 900, 700);

    for action in [
        Action::Maximize,
        Action::Restore,
        Action::NextMonitor,
        Action::PreviousMonitor,
    ] {
        assert_eq!(LayoutEngine::target(action, current, work), None);
    }
}

#[test]
fn invalid_rectangles_are_rejected() {
    let valid = Rect::new(0, 0, 100, 100);
    let empty = Rect::new(0, 0, 0, 100);
    let inverted = Rect::new(100, 100, 0, 0);

    assert_eq!(LayoutEngine::target(Action::LeftHalf, empty, valid), None);
    assert_eq!(
        LayoutEngine::target(Action::LeftHalf, valid, inverted),
        None
    );
    assert_eq!(LayoutEngine::map_to_monitor(empty, valid, valid), None);
}

#[test]
fn monitor_mapping_scales_and_clamps_window_geometry() {
    let source = Rect::new(-1920, 0, 0, 1080);
    let target = Rect::new(0, -1440, 2560, 0);

    assert_eq!(
        LayoutEngine::map_to_monitor(Rect::new(-1440, 270, -480, 810), source, target),
        Some(Rect::new(640, -1080, 1920, -360))
    );
    assert_eq!(
        LayoutEngine::map_to_monitor(Rect::new(-2200, -200, 200, 1280), source, target),
        Some(Rect::new(0, -1440, 2560, 0))
    );
}

#[test]
fn rect_reports_dimensions_validity_and_intersection() {
    let first = Rect::new(-100, -50, 300, 250);
    let second = Rect::new(100, 100, 500, 400);

    assert_eq!(first.width(), 400);
    assert_eq!(first.height(), 300);
    assert!(first.is_valid());
    assert_eq!(first.intersection_area(second), 30_000);
    assert_eq!(first.intersection_area(Rect::new(301, 251, 500, 500)), 0);
}

#[test]
fn halves_cover_odd_negative_work_area_without_overlap() {
    let work = Rect::new(-1921, 0, 0, 1081);
    let current = Rect::new(-1000, 100, -500, 600);
    let left = LayoutEngine::target(Action::LeftHalf, current, work).unwrap();
    let right = LayoutEngine::target(Action::RightHalf, current, work).unwrap();

    assert_eq!(left.left, work.left);
    assert_eq!(left.right, right.left);
    assert_eq!(right.right, work.right);
    assert_eq!(left.width() + right.width(), work.width());
}

#[test]
fn thirds_distribute_all_pixels() {
    let work = Rect::new(0, 0, 100, 80);
    let current = Rect::new(0, 0, 10, 10);
    let first = LayoutEngine::target(Action::FirstThird, current, work).unwrap();
    let center = LayoutEngine::target(Action::CenterThird, current, work).unwrap();
    let last = LayoutEngine::target(Action::LastThird, current, work).unwrap();

    assert_eq!(first.right, center.left);
    assert_eq!(center.right, last.left);
    assert_eq!(first.width() + center.width() + last.width(), 100);
}

#[test]
fn monitor_mapping_preserves_relative_geometry() {
    let from = Rect::new(-1920, 0, 0, 1080);
    let to = Rect::new(0, 0, 2560, 1440);
    let current = Rect::new(-1920, 0, -960, 1080);

    assert_eq!(
        LayoutEngine::map_to_monitor(current, from, to),
        Some(Rect::new(0, 0, 1280, 1440))
    );
}

#[test]
fn incremental_moves_stay_inside_work_area() {
    let work = Rect::new(-100, -50, 900, 750);
    let current = Rect::new(-100, -50, 300, 250);

    assert_eq!(
        LayoutEngine::target(Action::MoveLeft, current, work),
        Some(current)
    );
    assert_eq!(
        LayoutEngine::target(Action::MoveDown, current, work),
        Some(Rect::new(-100, -10, 300, 290))
    );
}

proptest! {
    #[test]
    fn every_geometric_target_is_valid_and_contained(
        left in -10_000i32..10_000,
        top in -10_000i32..10_000,
        width in 1i32..5_000,
        height in 1i32..5_000,
        x_seed in 0u32..10_000,
        y_seed in 0u32..10_000,
        width_seed in 1u32..10_000,
        height_seed in 1u32..10_000,
    ) {
        let work = Rect::new(left, top, left + width, top + height);
        let x = i32::try_from(x_seed % width as u32).unwrap_or(0);
        let y = i32::try_from(y_seed % height as u32).unwrap_or(0);
        let available_width = width - x;
        let available_height = height - y;
        let current_width =
            1 + i32::try_from(width_seed % available_width as u32).unwrap_or(0);
        let current_height =
            1 + i32::try_from(height_seed % available_height as u32).unwrap_or(0);
        let current = Rect::new(
            left + x,
            top + y,
            left + x + current_width,
            top + y + current_height,
        );

        for action in Action::ALL {
            if let Some(target) = LayoutEngine::target(action, current, work) {
                prop_assert!(target.is_valid());
                prop_assert!(target.left >= work.left);
                prop_assert!(target.top >= work.top);
                prop_assert!(target.right <= work.right);
                prop_assert!(target.bottom <= work.bottom);
            }
        }
    }
}

#[test]
fn the_gap_keeps_its_size_on_a_scaled_display() {
    assert_eq!(LayoutEngine::gap_pixels(12, 96), 12);
    assert_eq!(LayoutEngine::gap_pixels(12, 120), 15);
    assert_eq!(LayoutEngine::gap_pixels(12, 144), 18);
    assert_eq!(LayoutEngine::gap_pixels(12, 192), 24);
    assert_eq!(LayoutEngine::gap_pixels(0, 192), 0);
}

#[test]
fn an_unreported_dpi_leaves_the_gap_at_its_configured_size() {
    assert_eq!(LayoutEngine::gap_pixels(12, 0), 12);
}

#[test]
fn the_gap_is_capped_before_it_is_scaled() {
    assert_eq!(
        LayoutEngine::gap_pixels(u16::MAX, 192),
        LayoutEngine::gap_pixels(quboid_core::MAX_GAP, 192)
    );
}
