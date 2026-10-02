use super::*;

fn node(aim_x: f32) -> Station {
    Station {
        eye: Vec3::new(aim_x, 5.0, 40.0),
        aim: Vec3::new(aim_x, 0.0, 0.0),
    }
}

/// Three nodes a hundred units apart along x.
fn director(seed: u64) -> FinishCamera {
    FinishCamera::new(vec![node(0.0), node(100.0), node(200.0)], 0.5, seed)
}

fn craft(slot: usize, x: f32) -> Subject {
    Subject {
        slot,
        position: Vec3::new(x, 0.0, 0.0),
    }
}

#[test]
fn nothing_is_imposed_before_the_start_frame_and_the_director_starts_on_it() {
    let mut camera = director(1);
    let field = [craft(0, 5.0)];
    for since in 0..START_TICKS {
        assert!(camera.step(since, &field, 0).is_none(), "frame {since}");
    }
    assert_eq!(camera.mode(), None);
    let pose = camera.step(START_TICKS, &field, 0).expect("a node camera");
    assert_eq!(camera.mode(), Some(ViewMode::Track));
    assert_eq!(camera.subject(), Some(0));
    // The node whose aim point is nearest the player: the one at x = 0.
    assert_eq!(pose.eye, Vec3::new(0.0, 5.0, 40.0));
}

#[test]
fn the_first_node_is_the_one_whose_aim_point_is_nearest_the_player() {
    let mut camera = director(1);
    let pose = camera
        .step(START_TICKS, &[craft(0, 190.0)], 0)
        .expect("a node camera");
    assert_eq!(pose.eye.x, 200.0);
}

#[test]
fn a_circuit_with_no_camera_nodes_never_overrides_the_chase_camera() {
    let mut camera = FinishCamera::new(Vec::new(), 0.5, 1);
    assert!(!camera.has_nodes());
    assert!(camera.step(START_TICKS + 5, &[craft(0, 0.0)], 0).is_none());
}

#[test]
fn the_subject_is_repicked_every_600_frames_to_a_different_craft() {
    let field = [craft(0, 5.0), craft(1, 6.0), craft(2, 7.0)];
    let mut camera = director(7);
    camera.step(START_TICKS, &field, 0);
    for since in START_TICKS + 1..START_TICKS + 600 {
        camera.step(since, &field, 0);
        assert_eq!(camera.subject(), Some(0), "frame {since}");
    }
    camera.step(START_TICKS + 600, &field, 0);
    assert_ne!(
        camera.subject(),
        Some(0),
        "the first re-pick is a craft other than the previous subject"
    );
}

#[test]
fn a_lone_craft_stays_the_subject() {
    let field = [craft(0, 5.0)];
    let mut camera = director(7);
    for since in START_TICKS..START_TICKS + 1300 {
        camera.step(since, &field, 0);
        assert_eq!(camera.subject(), Some(0));
    }
}

#[test]
fn a_cut_waits_until_the_subject_is_sixty_units_from_the_aim_point() {
    let mut camera = director(3);
    camera.step(START_TICKS, &[craft(0, 0.0)], 0);
    let first = camera.node;
    // 59 units from the first aim point, still 41 from the second: stays.
    camera.step(START_TICKS + 1, &[craft(0, 59.0)], 0);
    assert_eq!(camera.node, first);
    // 61 from the first, 39 from the second (and 139 from the third): cuts to the second.
    camera.step(START_TICKS + 2, &[craft(0, 61.0)], 0);
    assert_eq!(camera.node, Some(1));
}

#[test]
fn a_subject_with_no_other_node_in_range_keeps_the_node() {
    let mut camera = FinishCamera::new(vec![node(0.0), node(500.0)], 0.5, 3);
    camera.step(START_TICKS, &[craft(0, 0.0)], 0);
    camera.step(START_TICKS + 1, &[craft(0, 250.0)], 0);
    assert_eq!(camera.node, Some(0));
}

#[test]
fn a_cut_rolls_a_mode_and_the_node_modes_set_the_view_width() {
    let mut camera = director(11);
    camera.step(START_TICKS, &[craft(0, 0.0)], 0);
    assert_eq!(
        camera.width, INITIAL_WIDTH,
        "no cut has called Camera_SetMode"
    );
    let mut x = 0.0;
    let mut cuts = 0;
    for since in START_TICKS + 1..START_TICKS + 300 {
        x += 2.0;
        let before = camera.node;
        camera.step(since, &[craft(0, x)], 0);
        if camera.node != before {
            cuts += 1;
            let mode = camera.mode().expect("running");
            match mode {
                ViewMode::Close => assert_eq!(camera.width, 17.0),
                ViewMode::Track => assert_eq!(camera.width, 50.0),
                ViewMode::Above | ViewMode::Front => {}
            }
        }
    }
    assert!(
        cuts >= 2,
        "the craft drove past three nodes, got {cuts} cuts"
    );
}

#[test]
fn the_mode_rolls_follow_the_original_thresholds() {
    let mut camera = director(99);
    let mut counts = [0u32; 4];
    for _ in 0..40_000 {
        camera.roll_mode();
        let index = match camera.mode {
            ViewMode::Front => 0,
            ViewMode::Above => 1,
            ViewMode::Close => 2,
            ViewMode::Track => 3,
        };
        counts[index] += 1;
    }
    // 3 is 26 %, 2 is 25 %, 6 is 25 %, 7 is 24 %.
    for (count, want) in counts.iter().zip([0.26, 0.25, 0.25, 0.24]) {
        let got = *count as f32 / 40_000.0;
        assert!(
            (got - want).abs() < 0.015,
            "{counts:?}: {got} against {want}"
        );
    }
}

#[test]
fn the_craft_relative_modes_leave_the_chase_camera() {
    let mut camera = director(1);
    camera.step(START_TICKS, &[craft(0, 0.0)], 0);
    for mode in [ViewMode::Front, ViewMode::Above] {
        camera.mode = mode;
        assert!(camera.step(START_TICKS + 1, &[craft(0, 0.0)], 0).is_none());
    }
    camera.mode = ViewMode::Close;
    assert!(camera.step(START_TICKS + 2, &[craft(0, 0.0)], 0).is_some());
}

#[test]
fn the_field_of_view_frames_the_view_width_at_the_distance() {
    // 50 units across at 25 units away is a quarter turn, 90 degrees.
    let eye = Vec3::new(0.0, 0.0, 25.0);
    let fov = destroy::framing_fov_degrees(50.0, eye.distance(Vec3::ZERO));
    assert!((fov - 90.0).abs() < 1e-3, "{fov}");
    // Twice as far is narrower.
    assert!(destroy::framing_fov_degrees(50.0, (eye * 2.0).distance(Vec3::ZERO)) < fov);
}

#[test]
fn the_fov_eases_to_its_target_and_the_camera_looks_at_the_subject() {
    let mut camera = director(5);
    let field = [craft(0, 0.0)];
    let mut pose = camera.step(START_TICKS, &field, 0).expect("a pose");
    for since in START_TICKS + 1..START_TICKS + 400 {
        pose = camera.step(since, &field, 0).expect("a pose");
    }
    let node = camera.nodes[camera.node.expect("a node")];
    let target = destroy::framing_fov_degrees(camera.width, node.eye.distance(Vec3::ZERO));
    let fov = pose.fov_deg.expect("a fov");
    assert!((fov - target).abs() < 0.05, "{fov} against {target}");
    // The camera's -Z points from the eye toward the subject, to the squash of its height.
    let forward = pose.orientation * Vec3::NEG_Z;
    let toward = (Vec3::ZERO - node.eye).normalize();
    assert!(
        forward.dot(toward) > 0.98,
        "forward {forward:?} against {toward:?}"
    );
    assert!(
        forward.y.abs() <= toward.y.abs() + 1e-4,
        "the height is squashed, never grown"
    );
}

#[test]
fn the_same_seed_flies_the_same_cuts() {
    let run = |seed| {
        let mut camera = director(seed);
        let mut trace = Vec::new();
        let mut x = 0.0;
        for since in START_TICKS..START_TICKS + 500 {
            x += 1.0;
            let pose = camera.step(since, &[craft(0, x), craft(1, x + 3.0)], 0);
            trace.push((
                camera.subject(),
                camera.node,
                camera.mode(),
                pose.map(|p| p.eye),
            ));
        }
        trace
    };
    assert_eq!(run(4), run(4));
}

#[test]
fn the_take_over_a_new_node_and_a_switch_to_the_chase_stand_in_each_count_as_a_cut() {
    let mut camera = director(3);
    assert_eq!(camera.cuts(), 0);
    camera.step(START_TICKS, &[craft(0, 0.0)], 0);
    assert_eq!(camera.cuts(), 1, "the take-over");
    camera.step(START_TICKS + 1, &[craft(0, 1.0)], 0);
    assert_eq!(camera.cuts(), 1, "the same node, the same shot");
    // Past sixty units: a new node. The mode roll may leave a node camera, which is a second jump.
    camera.mode = ViewMode::Track;
    camera.step(START_TICKS + 2, &[craft(0, 61.0)], 0);
    assert!(camera.cuts() >= 2, "a new node is a cut");
    // Back on a node camera whatever the roll chose, so what follows is deterministic.
    camera.mode = ViewMode::Track;
    camera.step(START_TICKS + 3, &[craft(0, 61.0)], 0);
    let before = camera.cuts();
    camera.mode = ViewMode::Front;
    camera.step(START_TICKS + 4, &[craft(0, 61.0)], 0);
    assert_eq!(
        camera.cuts(),
        before + 1,
        "node camera to the chase stand-in"
    );
    camera.step(START_TICKS + 5, &[craft(0, 61.0)], 0);
    assert_eq!(
        camera.cuts(),
        before + 1,
        "the chase stand-in keeps its shot"
    );
}

/// A wrecked player (slot 0, not in the live list) is never the subject, at the start or at
/// a re-pick, and the craft-relative modes show a node camera instead of the wreck.
#[test]
fn a_wrecked_player_is_never_followed_and_the_stand_in_modes_still_show_the_field() {
    let mut camera = director(7);
    let field = [craft(1, 5.0), craft(2, 50.0), craft(3, 150.0)];
    camera.step(START_TICKS, &field, 0).expect("a node camera");
    assert_ne!(camera.subject(), Some(0));
    let mut showing = 0;
    for since in START_TICKS + 1..START_TICKS + 3 * u64::from(SUBJECT_PERIOD_TICKS) {
        showing += u32::from(camera.step(since, &field, 0).is_some());
        assert_ne!(camera.subject(), Some(0), "tick {since}");
    }
    assert_eq!(
        showing,
        3 * SUBJECT_PERIOD_TICKS - 1,
        "never the chase stand-in"
    );
}
