use super::*;
use oag_vex::track::{Junction, Path};

/// Eight control points ten units apart along `+x`, tangent `+x`, down `-y`, lateral `+z`.
fn straight() -> AiTrack {
    let points = (0..8)
        .map(|k| SplinePoint {
            pos: [k as f32 * 10.0, 0.0, 0.0],
            tangent: [1.0, 0.0, 0.0],
            down: [0.0, -1.0, 0.0],
            lateral: [0.0, 0.0, 1.0],
            progress: 0.0,
            half_width_left: 12.0,
            half_width_right: 12.0,
            ai_bound_left: -4.0,
            ai_bound_right: 4.0,
            racing_line: 0.0,
            section_id: 0,
            flags: 0,
            light_scale: [0xff; 4],
            unk_0x5c: Some(0xff),
            unk_0x5d: Some(0),
            unk_0x66: Some(253),
            cage: [0; 8],
        })
        .collect();
    AiTrack {
        version: 0x105,
        paths: vec![Path {
            points,
            max_spacing: 10.0,
            entry: None,
            exit: None,
        }],
        junctions: Vec::new(),
    }
}

/// Paths 0 and 1 split from junction 0 and merge at junction 1; path 2 closes the ring.
/// Path 1 sits 40 units to the side of path 0.
fn split() -> AiTrack {
    let base = straight();
    let path = base.paths[0].clone();
    let mut aside = path.clone();
    for point in &mut aside.points {
        point.pos[2] += 40.0;
    }
    AiTrack {
        version: base.version,
        paths: vec![
            Path {
                entry: Some(0),
                exit: Some(1),
                ..path.clone()
            },
            Path {
                entry: Some(0),
                exit: Some(1),
                ..aside
            },
            Path {
                entry: Some(1),
                exit: Some(0),
                ..path
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
    }
}

#[test]
fn no_cage_without_the_header_flag_or_the_bytes() {
    assert!(CageTrack::new(&straight(), false).is_none());
    assert!(CageTrack::new(&straight(), true).is_some());
    let mut old = straight();
    old.version = 0x103;
    assert!(
        CageTrack::new(&old, true).is_none(),
        "before 0x104 the bytes are zeroed"
    );
    let mut short = straight();
    short.version = oag_vex::track::SHORT_POINT_VERSION;
    assert!(
        CageTrack::new(&short, true).is_none(),
        "short points carry no bytes"
    );
}

#[test]
fn the_locate_is_the_nearest_control_point() {
    let cage = CageTrack::new(&straight(), true).expect("a cage");
    assert_eq!(cage.point_count(), 8);
    // Control points every 10 units along +x, lifted 3 up.
    let located = cage.locate(Vec3::new(31.0, 0.0, 2.0)).expect("located");
    assert_eq!(located.point, Vec3::new(30.0, 3.0, 0.0));
}

#[test]
fn a_fork_offers_the_other_branch_as_the_sibling() {
    let ai = split();
    assert_eq!(sibling(&ai, 0), Some(1));
    assert_eq!(sibling(&ai, 1), Some(0));
    // Path 2's entry junction has one successor; its exit junction (0) has one
    // predecessor, itself: no sibling.
    assert_eq!(sibling(&ai, 2), None);
    let cage = CageTrack::new(&ai, true).expect("a cage");
    let located = cage.locate(Vec3::new(20.0, 0.0, 41.0)).expect("located");
    assert_eq!(located.point, Vec3::new(20.0, 3.0, 40.0));
    let sibling = located.sibling.expect("the other branch");
    // The sibling's section is the main branch's point nearest the located one.
    assert!((sibling.left - located.section.left).z.abs() > 39.0);
}

/// A craft 300 units off the cage's points: the original's reset distance.
fn far_off(craft_state: oag_physics::CraftState, state_timer: f32) -> oag_physics::ShipState {
    let mut state = oag_physics::ShipState {
        craft_state,
        state_timer,
        ..oag_physics::ShipState::default()
    };
    state.body.position = Vec3::new(30.0, 300.0, 0.0);
    state
}

fn caged(state: &mut oag_physics::ShipState, eliminator_route: bool) -> bool {
    let cage = CageTrack::new(&straight(), true).expect("a cage");
    let wall = oag_physics::WallResponse {
        synced_position: state.body.position,
        synced_orientation: state.body.orientation,
        ..oag_physics::WallResponse::default()
    };
    pass(
        Some(&cage),
        state,
        &oag_physics::Handling::default(),
        &wall,
        eliminator_route,
    )
    .reset
}

#[test]
fn a_wreck_leaves_the_cage_in_state_6() {
    use oag_physics::CraftState::{Destroyed, Eliminated, Racing};
    assert!(
        caged(&mut far_off(Racing, 0.0), false),
        "the reset distance"
    );
    assert!(
        caged(&mut far_off(Destroyed, 0.3), false),
        "state 4 is caged"
    );
    assert!(
        caged(&mut far_off(Eliminated, 0.7), false),
        "state 5's dwell is caged"
    );
    assert!(
        !caged(&mut far_off(Eliminated, 0.0), false),
        "state 6 is skipped"
    );
    assert!(
        caged(&mut far_off(Eliminated, 0.0), true),
        "an Eliminator wreck goes to state 8, which is caged"
    );
}
