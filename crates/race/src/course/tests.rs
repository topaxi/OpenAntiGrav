//! What the closed course loop and its distances in [`super`] are asserted to do.
//!
//! Split out of `course.rs` under the 200-line cap on inline `#[cfg(test)]`
//! modules; see `scripts/check-file-size.py`.

use super::{Course, primary_ring};
use crate::testing::square_track;
use oag_core::math::Vec3;
use oag_vex::track::{AiTrack, Junction, Path};

#[test]
fn a_two_path_ring_walks_in_travel_order() {
    let ai = square_track(2);
    assert_eq!(primary_ring(&ai), Some(vec![0, 1]));
}

#[test]
fn a_four_path_ring_walks_all_four() {
    let ai = square_track(4);
    assert_eq!(primary_ring(&ai), Some(vec![0, 1, 2, 3]));
}

#[test]
fn a_single_path_with_no_junctions_is_its_own_ring() {
    let mut ai = square_track(1);
    ai.junctions.clear();
    ai.paths[0].entry = None;
    ai.paths[0].exit = None;
    assert_eq!(primary_ring(&ai), Some(vec![0]));
}

#[test]
fn a_chain_that_never_returns_is_not_a_ring() {
    let mut ai = square_track(2);
    // Path 1's exit leads nowhere, so no walk ever comes home.
    ai.junctions[1].next[0] = None;
    assert_eq!(primary_ring(&ai), None);
}

#[test]
fn a_degenerate_self_loop_does_not_beat_the_real_ring() {
    // Four paths: 0 -> 1 -> 2 -> 0 is the circuit, and path 3's exit points
    // back at itself. Both close. Taking the first hit in file order would
    // still find the circuit here, so the test also checks the case that
    // actually bites: a self-loop on a *lower* index than the circuit.
    let mut ai = square_track(4);
    ai.junctions[2].next[0] = Some(0);
    ai.junctions[3].next[0] = Some(3);
    assert_eq!(primary_ring(&ai), Some(vec![0, 1, 2]));

    let mut ai = square_track(4);
    ai.junctions[0].next[0] = Some(0);
    ai.junctions[3].next[0] = Some(1);
    assert_eq!(
        primary_ring(&ai),
        Some(vec![1, 2, 3]),
        "the one-path self-loop at index 0 was preferred over the circuit"
    );
}

#[test]
fn an_alternate_branch_is_left_off_the_ring() {
    // Three paths: 0 and 1 are the two sides of a split, 2 is the merge.
    // Junction 0 leaves path 2 and splits to 0 (primary) and 1 (alternate);
    // junction 1 merges 0 and 1 back into 2. That is `05_Track`'s shape.
    let base = square_track(3);
    let ai = AiTrack {
        version: 1,
        paths: vec![
            Path {
                entry: Some(0),
                exit: Some(1),
                ..base.paths[0].clone()
            },
            Path {
                entry: Some(0),
                exit: Some(1),
                ..base.paths[1].clone()
            },
            Path {
                entry: Some(1),
                exit: Some(0),
                ..base.paths[2].clone()
            },
        ],
        junctions: vec![
            Junction {
                prev: [Some(2), None],
                next: [Some(0), Some(1)],
            },
            Junction {
                prev: [Some(0), Some(1)],
                next: [Some(2), None],
            },
        ],
    };

    let ring = primary_ring(&ai).expect("the primary chain closes");
    assert!(ring.contains(&0), "the primary branch is on the ring");
    assert!(ring.contains(&2), "the merge path is on the ring");
    assert!(
        !ring.contains(&1),
        "the alternate branch is a shortcut, not the line"
    );

    // And the same answer through the accessor an AI line is built from:
    // the branch the lap does not drive is not in the order it drives.
    let course = Course::from_track(&ai, None).expect("a ring");
    let order = course.path_order();
    assert!(!order.contains(&1), "path order kept the alternate branch");
    assert_eq!(
        order.len(),
        2,
        "a lap drives two of the three paths, once each: {order:?}"
    );
}

#[test]
fn the_path_order_of_a_plain_ring_is_the_file_order() {
    // The nine circuits where nothing changes: file order already is travel
    // order, so anything mapping through this gets the identity.
    let course = Course::from_track(&square_track(4), None).expect("a ring");
    assert_eq!(course.path_order(), vec![0, 1, 2, 3]);
}

#[test]
fn distance_is_non_decreasing_and_starts_at_zero() {
    let course = Course::from_track(&square_track(2), None).expect("a ring");
    assert_eq!(course.progress_at(0), Some(0.0));
    assert!(course.length() > 0.0);

    let mut previous = 0.0;
    for index in 0..course.len() {
        let progress = course.progress_at(index).expect("in range");
        assert!(
            progress >= previous,
            "distance went backwards at {index}: {progress} after {previous}"
        );
        assert!(progress < course.length(), "distance reached the full lap");
        previous = progress;
    }
}

#[test]
fn the_ring_is_about_as_long_as_the_shape_it_was_built_from() {
    let course = Course::from_track(&square_track(2), None).expect("a ring");
    // A 30x30 square is 120 units round. The B-spline does not pass through
    // its control points, so it cuts every corner and comes out shorter;
    // what matters is that it is the right order of magnitude and closed.
    assert!(
        (60.0..=130.0).contains(&course.length()),
        "ring length {} is not a plausible 30x30 loop",
        course.length()
    );
}

#[test]
fn the_tangent_points_toward_the_next_ring_point() {
    let course = Course::from_track(&square_track(2), None).expect("a ring");
    let tangent = course.tangent(0).expect("in range");
    assert!(
        (tangent.length() - 1.0).abs() < 1e-4,
        "tangent is not unit length"
    );
    let expected = (course.position(1).unwrap() - course.position(0).unwrap()).normalize();
    assert!((tangent - expected).length() < 1e-4);
}

#[test]
fn the_tangent_wraps_at_the_last_point() {
    let course = Course::from_track(&square_track(2), None).expect("a ring");
    let last = course.len() - 1;
    let tangent = course.tangent(last).expect("in range");
    let expected = (course.position(0).unwrap() - course.position(last).unwrap()).normalize();
    assert!((tangent - expected).length() < 1e-4);
}

#[test]
fn the_tangent_is_none_out_of_range() {
    let course = Course::from_track(&square_track(2), None).expect("a ring");
    assert_eq!(course.tangent(course.len()), None);
}

#[test]
fn the_start_line_moves_to_the_point_nearest_the_grid_slot() {
    let ai = square_track(2);
    let plain = Course::from_track(&ai, None).expect("a ring");
    assert_eq!(plain.start_index(), 0);

    // The far corner of the square, which is nowhere near point 0.
    let moved = Course::from_track(&ai, Some(Vec3::new(30.0, 0.0, 30.0))).expect("a ring");
    assert_ne!(moved.start_index(), 0);
    assert_eq!(moved.progress_at(moved.start_index()), Some(0.0));
}

#[test]
fn locating_a_point_on_the_ring_returns_its_own_distance() {
    let course = Course::from_track(&square_track(2), None).expect("a ring");
    for index in [0, 3, 11, course.len() - 1] {
        let position = course.position(index).expect("in range");
        let located = course.locate(position, None).expect("on the ring");
        assert_eq!(located.index, index);
        assert_eq!(located.progress, course.progress_at(index).unwrap());
        assert!(
            located.offset < 1e-3,
            "offset {} is not zero",
            located.offset
        );
    }
}

#[test]
fn a_hint_finds_the_same_point_a_full_scan_does() {
    let course = Course::from_track(&square_track(2), None).expect("a ring");
    for index in 0..course.len() {
        let position = course.position(index).expect("in range");
        let global = course.locate(position, None).expect("on the ring");
        let hinted = course.locate(position, Some(index)).expect("on the ring");
        assert_eq!(global.index, hinted.index, "hint disagreed at {index}");
    }
}

#[test]
fn the_window_wraps_around_the_start_of_the_table() {
    let course = Course::from_track(&square_track(2), None).expect("a ring");
    let last = course.len() - 1;
    // Standing on the last point with the hint still on the first one: the
    // window has to wrap backwards to find it.
    let position = course.position(last).expect("in range");
    let located = course.locate(position, Some(0)).expect("on the ring");
    assert_eq!(located.index, last);
}

#[test]
fn a_stale_hint_far_from_the_ship_reacquires_globally() {
    let course = Course::from_track(&square_track(2), None).expect("a ring");
    let target = course.len() / 2;
    let position = course.position(target).expect("in range");
    // A hint on the opposite side of the loop, well outside the window.
    let located = course.locate(position, Some(0)).expect("on the ring");
    assert_eq!(located.index, target, "the stale hint was not dropped");
}

/// A three-path ring whose fork's alternate forks again: path 2's exit splits
/// to 0 (the ring) and 3, and path 3's exit splits to 4 and 5, both of which
/// rejoin at path 1 - so every route stands in for path 0. Paths 3-5 sit
/// `stray` units above the ring.
fn nested_fork(stray: f32) -> AiTrack {
    let base = square_track(6);
    let moved = |p: &Path| Path {
        points: p
            .points
            .iter()
            .map(|pt| oag_vex::track::SplinePoint {
                pos: [pt.pos[0], pt.pos[1] + stray, pt.pos[2]],
                ..*pt
            })
            .collect(),
        ..p.clone()
    };
    let path = |from: &Path, entry: usize, exit: usize| Path {
        entry: Some(entry),
        exit: Some(exit),
        ..from.clone()
    };
    let junction = |prev: usize, next: [Option<usize>; 2]| Junction {
        prev: [Some(prev), None],
        next,
    };
    AiTrack {
        version: 1,
        paths: vec![
            path(&base.paths[0], 0, 1),
            path(&base.paths[1], 1, 2),
            path(&base.paths[2], 2, 0),
            path(&moved(&base.paths[0]), 0, 3),
            path(&moved(&base.paths[1]), 3, 4),
            path(&moved(&base.paths[1]), 3, 5),
        ],
        junctions: vec![
            junction(2, [Some(0), Some(3)]),
            junction(0, [Some(1), None]),
            junction(1, [Some(2), None]),
            junction(3, [Some(4), Some(5)]),
            junction(4, [Some(1), None]),
            junction(5, [Some(1), None]),
        ],
    }
}

#[test]
fn a_fork_inside_an_alternate_is_two_routes_with_their_own_coins() {
    let course = Course::from_track(&nested_fork(0.0), None).expect("a ring");
    assert_eq!(course.path_order(), vec![0, 1, 2]);
    let routes: Vec<(Vec<u16>, Vec<bool>)> = course
        .routes()
        .iter()
        .map(|r| (r.paths.clone(), r.choices.clone()))
        .collect();
    assert_eq!(
        routes,
        vec![
            (vec![3, 4], vec![true, false]),
            (vec![3, 5], vec![true, true])
        ],
        "one leaf per way back, primary first, each with the coins that pick it"
    );
    for route in course.routes() {
        assert_eq!(route.pre_fork, 2, "the fork is at path 2's exit");
        assert_eq!(course.path_of(route.split), Some(0));
        assert_eq!(course.path_of(route.merge), Some(1));
    }
    // Branch, the Repulser's narrower view, sees none of it: path 3 does not
    // rejoin the ring itself.
    assert!(course.branches().is_empty());
}

#[test]
fn a_craft_far_out_on_a_route_reads_progress_inside_the_span_it_replaces() {
    // Two hundred units out, far past the ring's reacquire distance.
    let course = Course::from_track(&nested_fork(200.0), None).expect("a ring");
    let route = &course.routes()[0];
    let split = course.progress_at(route.split).expect("in range");
    let merge = course.progress_at(route.merge).expect("in range");
    let before = (route.split + course.len() - 1) % course.len();
    let mut hint = Some(before);
    let mut last = split;
    for point in &route.positions {
        let located = course.locate(*point, hint).expect("located");
        assert!(
            located.offset < 1.0,
            "located on the route, not the ring: {located:?}"
        );
        assert!(
            located.progress >= last && located.progress <= merge,
            "progress {:.2} outside {last:.2}..={merge:.2}",
            located.progress
        );
        last = located.progress;
        hint = Some(located.index);
    }
}
