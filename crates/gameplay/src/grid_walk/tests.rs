use super::*;
use oag_vex::track::{AiTrack, Path, SplinePoint};

/// A straight path along `+x`, `count` control points six units apart, `+z` lateral,
/// level, with half-widths from `half_width`.
fn straight(count: usize, half_width: impl Fn(f32) -> (f32, f32)) -> AiTrack {
    let points = (0..count)
        .map(|k| {
            let x = k as f32 * 6.0;
            let (left, right) = half_width(x);
            SplinePoint {
                pos: [x, 0.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                down: [0.0, -1.0, 0.0],
                lateral: [0.0, 0.0, 1.0],
                progress: 0.0,
                half_width_left: left,
                half_width_right: right,
                ai_bound_left: -left + 8.0,
                ai_bound_right: right - 8.0,
                racing_line: 0.0,
                section_id: 0,
                flags: 0,
                light_scale: [0xff; 4],
            }
        })
        .collect();
    AiTrack {
        version: 0x105,
        paths: vec![Path {
            points,
            max_spacing: 6.0,
            entry: None,
            exit: None,
        }],
        junctions: Vec::new(),
    }
}

fn level(count: usize) -> AiTrack {
    straight(count, |_| (12.0, 12.0))
}

/// `p(k+1) = scale * (p(k) + scale * 19.8)` along a level straight: the located record's
/// own tangent is `scale` long and the record's position is `scale` of the foot.
fn expected_x(start: f32, steps: usize) -> f32 {
    let mut x = RECORD_SCALE * start;
    for _ in 0..steps {
        x = RECORD_SCALE * (x + RECORD_SCALE * GRID_ROW_PITCH);
    }
    x
}

#[test]
fn the_record_is_scaled_by_the_half_float_weights() {
    // The scale is `6 * 0x3155` read as a half float, and `FUN_0887c7e8`'s `w` lane read
    // `0.999755859375` on every record watched live.
    assert!((RECORD_SCALE - 0.999_755_86).abs() < 1e-9);
    let at = locate(&level(60), Vec3::new(150.0, 0.0, 2.0)).expect("a track");
    assert!((at.tangent.length() - RECORD_SCALE).abs() < 1e-6);
    assert!((at.position.x - 150.0 * RECORD_SCALE).abs() < 1e-3);
    // Lifted three units above the surface line, then scaled with the rest.
    assert!((at.position.y - 3.0 * RECORD_SCALE).abs() < 1e-5);
}

#[test]
fn locate_projects_onto_the_curve_rather_than_snapping_to_a_sample() {
    // 150.37 is between control points (150 and 156) and between any four-a-segment sample.
    let at = locate(&level(60), Vec3::new(150.37, 0.0, 4.0)).expect("a track");
    assert!(
        (at.position.x - 150.37 * RECORD_SCALE).abs() < 1e-3,
        "{}",
        at.position.x
    );
    assert!(at.position.z.abs() < 1e-4, "foot of the perpendicular");
}

#[test]
fn each_step_is_the_located_tangent_and_each_located_record_is_scaled_again() {
    let track = level(60);
    let slots = walk(&track, Vec3::new(150.0, 0.0, 10.0)).expect("a grid");
    // Slot 8 is the first located record; each slot ahead of it is one step on.
    for (index, pose) in slots.iter().enumerate().rev() {
        let steps = GRID_SLOTS as usize - 1 - index;
        let want = expected_x(150.0, steps);
        assert!(
            (pose.position.x - want).abs() < 2e-3,
            "slot {}: x {} against {want}",
            index + 1,
            pose.position.x
        );
    }
    // Without the scale, seven steps are `7 * 19.8` and no more; with it the front slot is
    // over a third of a unit short of that, which is what the loop above is pinned to.
    let unscaled = 150.0 + 7.0 * GRID_ROW_PITCH;
    assert!(
        (unscaled - slots[0].position.x) > 0.3,
        "{} against {unscaled}",
        slots[0].position.x
    );
}

#[test]
fn the_sides_alternate_about_the_corridor_midpoint_starting_on_the_nodes_own() {
    let track = level(60);
    // The node is right of the midpoint (z positive is the lateral axis).
    let right = walk(&track, Vec3::new(150.0, 0.0, 10.0)).expect("a grid");
    for (index, pose) in right.iter().enumerate() {
        let slot = index + 1;
        let want = if slot % 2 == 0 { 10.0 } else { -10.0 };
        assert!(
            (pose.position.z - want).abs() < 0.01,
            "slot {slot}: z {}",
            pose.position.z
        );
    }
    // And the other side of the midpoint flips the whole pattern.
    let left = walk(&track, Vec3::new(150.0, 0.0, -10.0)).expect("a grid");
    for (index, pose) in left.iter().enumerate() {
        let slot = index + 1;
        let want = if slot % 2 == 0 { -10.0 } else { 10.0 };
        assert!(
            (pose.position.z - want).abs() < 0.01,
            "slot {slot}: z {}",
            pose.position.z
        );
    }
}

#[test]
fn the_heading_runs_along_the_edges_not_along_the_tangent() {
    // Every tangent is `+x`, but the right half-width grows 0.1 a unit, so the right
    // edge's chord leans `0.1` toward `+z` and the unit sum of the two chords does too.
    let track = straight(60, |x| (12.0, 12.0 + 0.1 * x));
    let slots = walk(&track, Vec3::new(150.0, 0.0, 10.0)).expect("a grid");
    let forward = slots[3].orientation * Vec3::NEG_Z;
    let right_chord = Vec3::new(1.0, 0.0, 0.1).normalize();
    let want = (Vec3::X + right_chord).normalize();
    assert!(
        (forward - want).length() < 2e-3,
        "{forward:?} against {want:?}"
    );
    assert!(
        forward.z > 0.04,
        "a tangent heading would read zero: {forward:?}"
    );
}

#[test]
fn a_node_far_above_its_located_sample_is_not_trusted() {
    // `25_Track` reversed: the nearest control point is on the ramp over the node.
    assert!(walk(&level(60), Vec3::new(150.0, 30.0, 10.0)).is_none());
    assert!(walk(&level(60), Vec3::new(150.0, 4.0, 10.0)).is_some());
}

#[test]
fn an_empty_track_has_no_grid() {
    let empty = AiTrack {
        version: 0x105,
        paths: Vec::new(),
        junctions: Vec::new(),
    };
    assert!(locate(&empty, Vec3::ZERO).is_none());
    assert!(walk(&empty, Vec3::ZERO).is_none());
}
