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
