use super::*;

#[test]
fn the_panel_fades_in_over_seven_tenths_and_the_wipe_takes_two_seconds() {
    let at = |entered| Progress {
        entered,
        left: None,
    };
    assert_eq!(at(0.0).alpha(), 0.0);
    assert!((at(0.35).alpha() - 0.5).abs() < 1e-6);
    assert_eq!(at(0.7).alpha(), 1.0);
    assert!((at(1.0).wipe() - 0.5).abs() < 1e-6);
    assert_eq!(at(3.0).wipe(), 1.0);
}

#[test]
fn leaving_runs_seven_tenths_from_full_width_whatever_the_wipe_had_reached() {
    let leaving = |left| Progress {
        entered: 1.0,
        left: Some(left),
    };
    assert_eq!(leaving(0.0).wipe(), 1.0);
    assert!((leaving(0.35).wipe() - 0.5).abs() < 1e-6);
    assert!((leaving(0.35).alpha() - 0.5).abs() < 1e-6);
    assert!(!leaving(0.7).visible());
}

#[test]
fn a_panel_skipped_before_it_had_faded_in_leaves_from_where_it_was() {
    let early = Progress {
        entered: 0.35,
        left: Some(0.35),
    };
    assert_eq!(early.alpha(), 0.0);
}

#[test]
fn a_wider_window_repeats_the_bars_outward_in_whole_tiles_and_leaves_the_texts() {
    let mut lists = DrawLists {
        frame: vec![
            Draw::GradientFill {
                rect: [0.0, 0.0, 480.0, 40.0],
                left: [0.0; 4],
                right: [0.0; 4],
            },
            Draw::GradientFill {
                rect: [0.0, 40.0, 240.0, 1.0],
                left: [0.0; 4],
                right: [1.0; 4],
            },
            Draw::GradientFill {
                rect: [240.0, 40.0, 240.0, 1.0],
                left: [1.0; 4],
                right: [0.0; 4],
            },
            Draw::TiledSprite {
                rect: [0.0, 0.0, 480.0, 40.0],
                uv: [0.0, 0.0, 32.0, 16.0],
                repeat: [30.0, 5.0],
                color: [1.0; 4],
            },
        ],
        body: Vec::new(),
        frame_bottom: vec![false; 4],
        right: 0.0,
    };
    // 50 units each side, the tile 16 wide: 64.
    lists.extend_width(50.0);
    let Draw::GradientFill { rect, .. } = &lists.frame[0] else {
        panic!()
    };
    assert_eq!(*rect, [-64.0, 0.0, 608.0, 40.0]);
    let Draw::GradientFill { rect, .. } = &lists.frame[1] else {
        panic!()
    };
    assert_eq!(*rect, [-64.0, 40.0, 304.0, 1.0]);
    let Draw::GradientFill { rect, .. } = &lists.frame[2] else {
        panic!()
    };
    assert_eq!(*rect, [240.0, 40.0, 304.0, 1.0]);
    let Draw::TiledSprite { rect, repeat, .. } = &lists.frame[3] else {
        panic!()
    };
    assert_eq!(rect[2] / repeat[0], 16.0);
    assert_eq!(rect[0], -64.0);
}

#[test]
fn a_narrower_or_equal_window_changes_nothing() {
    let mut lists = DrawLists {
        frame: vec![Draw::Fill {
            rect: [0.0, 0.0, 480.0, 40.0],
            color: [0.0; 4],
        }],
        body: Vec::new(),
        frame_bottom: vec![false],
        right: 0.0,
    };
    let before = lists.clone();
    lists.extend_width(0.0);
    lists.extend_width(-3.0);
    assert_eq!(lists, before);
}
