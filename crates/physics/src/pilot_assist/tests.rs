use super::*;
use oag_core::math::Quat;

/// Invented numbers in HD's shape.
const PARAMS: Params = Params {
    la_dist_const: 10.0,
    la_dist_vel_mul: 0.5,
    la_dist_max: 40.0,
    spring_mul: -100.0,
    torque_mul: 60.0,
    max_torque: 1000.0,
    max_ang_vel: 3.0,
    general_thrust_percent: 99.0,
    thrust_percent_on_use: 92.0,
    penalty_duration: 3.0,
};

/// A straight along `-z` (this engine's forward), 20 wide each side, lateral `+x`
/// (the craft's right), level.
fn straight() -> Corridor {
    Corridor {
        position: Vec3::ZERO,
        tangent: Vec3::NEG_Z,
        down: Vec3::NEG_Y,
        lateral: Vec3::X,
        half_width_left: 20.0,
        half_width_right: 20.0,
    }
}

fn craft(x: f32, speed: f32) -> Body {
    Body {
        position: Vec3::new(x, 0.0, 0.0),
        linear_velocity: Vec3::NEG_Z * speed,
        ..Body::default()
    }
}

fn input(enabled: bool) -> Input {
    Input {
        params: PARAMS,
        enabled,
        here: [Some(straight()), None],
        ahead: [Some(straight()), None],
    }
}

fn settled() -> State {
    State {
        enabled: true,
        blend: 1.0,
        ..State::default()
    }
}

#[test]
fn look_ahead_grows_with_speed_up_to_its_ceiling() {
    assert_eq!(
        look_ahead(&craft(0.0, 0.0), &PARAMS),
        Vec3::new(0.0, 0.0, -10.0)
    );
    assert_eq!(
        look_ahead(&craft(0.0, 20.0), &PARAMS),
        Vec3::new(0.0, 0.0, -20.0)
    );
    assert_eq!(
        look_ahead(&craft(0.0, 500.0), &PARAMS),
        Vec3::new(0.0, 0.0, -40.0)
    );
}

#[test]
fn in_the_middle_it_does_nothing() {
    let mut state = settled();
    let out = update(
        &mut state,
        &input(true),
        &craft(0.0, 100.0),
        true,
        1.0 / 60.0,
    );
    assert_eq!(out, Output::default());
    assert_eq!(state.acting, 0);
    assert_eq!(state.penalty_timer, 0.0);
}

/// HD's live run: a craft near the right-hand wall is yawed left (positive local yaw
/// here), pushed left, flags `-1` and starts the penalty.
#[test]
fn near_the_right_wall_it_turns_and_pushes_left() {
    let mut state = settled();
    let out = update(
        &mut state,
        &input(true),
        &craft(17.0, 100.0),
        true,
        1.0 / 60.0,
    );
    // Ahead: d = 17, radius 2: right = 20 - 2 - 17 = 1, so no intrusion yet ahead...
    // Here: radius 5: right = 20 - 5 - 17 = -2, push = -2 * -100 = 200 along -right.
    assert_eq!(out.world_force, Vec3::new(-200.0, 0.0, 0.0));
    let mut state = settled();
    let out = update(
        &mut state,
        &input(true),
        &craft(19.0, 100.0),
        true,
        1.0 / 60.0,
    );
    // Ahead: right = 20 - 2 - 19 = -1, torque = -60 (HD sign), yaw +60 here.
    assert_eq!(out.local_yaw, 60.0);
    assert_eq!(state.acting, -1);
    assert_eq!(state.penalty_timer, 3.0);
}

#[test]
fn near_the_left_wall_it_mirrors() {
    let mut state = settled();
    let out = update(
        &mut state,
        &input(true),
        &craft(-19.0, 100.0),
        true,
        1.0 / 60.0,
    );
    assert_eq!(out.local_yaw, -60.0);
    assert_eq!(state.acting, 1);
    assert!(out.world_force.x > 0.0);
}

#[test]
fn the_torque_is_clamped_and_withheld_while_spinning() {
    let mut state = settled();
    let out = update(
        &mut state,
        &input(true),
        &craft(60.0, 100.0),
        true,
        1.0 / 60.0,
    );
    assert_eq!(out.local_yaw, 1000.0);
    let mut spinning = craft(19.0, 100.0);
    spinning.angular_velocity = Vec3::new(0.0, 3.5, 0.0);
    let mut state = settled();
    let out = update(&mut state, &input(true), &spinning, true, 1.0 / 60.0);
    assert_eq!(out.local_yaw, 0.0);
    assert_eq!(state.acting, 0);
}

#[test]
fn facing_back_down_the_track_reads_no_push() {
    let mut body = craft(19.0, 100.0);
    body.orientation = Quat::from_rotation_y(core::f32::consts::PI);
    let mut state = settled();
    let out = update(&mut state, &input(true), &body, true, 1.0 / 60.0);
    assert_eq!(out, Output::default());
}

/// Measured on RPCS3: `0.03, 0.23, 0.43 ...` at 0.1 s steps.
#[test]
fn the_blend_rises_at_two_per_second_and_falls_at_twenty() {
    let mut state = State::default();
    for _ in 0..30 {
        update(&mut state, &input(true), &craft(0.0, 50.0), true, 0.01);
    }
    assert!((state.blend - 0.6).abs() < 1e-5, "{}", state.blend);
    update(&mut state, &input(true), &craft(0.0, 50.0), false, 0.01);
    assert!((state.blend - 0.4).abs() < 1e-5, "{}", state.blend);
}

#[test]
fn an_upside_down_craft_lets_go() {
    let mut body = craft(0.0, 50.0);
    body.orientation = Quat::from_rotation_z(core::f32::consts::PI);
    let mut state = settled();
    update(&mut state, &input(true), &body, true, 0.1);
    assert_eq!(state.blend, 0.0);
}

/// Measured on RPCS3: `99` enabled, `92` while the penalty runs, and the penalty keeps
/// running (and costing) after the option goes off.
#[test]
fn the_throttle_pays_for_it_even_after_switching_off() {
    let mut state = State::default();
    assert_eq!(thrust_scale(&state, Some(&PARAMS)), 1.0);
    update(&mut state, &input(true), &craft(0.0, 50.0), true, 0.1);
    assert_eq!(thrust_scale(&state, Some(&PARAMS)), 99.0 * 0.01);
    // Blend is only 0.4 here, so the intrusion has to be deep to pass the threshold.
    update(&mut state, &input(true), &craft(40.0, 100.0), true, 0.1);
    assert_eq!(thrust_scale(&state, Some(&PARAMS)), 92.0 * 0.01);
    update(&mut state, &input(false), &craft(0.0, 50.0), true, 1.0);
    assert!(!state.enabled);
    assert_eq!(thrust_scale(&state, Some(&PARAMS)), 92.0 * 0.01);
    update(&mut state, &input(false), &craft(0.0, 50.0), true, 2.5);
    assert_eq!(thrust_scale(&state, Some(&PARAMS)), 1.0);
}

#[test]
fn off_from_the_start_it_never_leaves_its_default() {
    let mut state = State::default();
    for x in [-19.0, 0.0, 19.0] {
        let out = update(
            &mut state,
            &input(false),
            &craft(x, 100.0),
            true,
            1.0 / 60.0,
        );
        assert_eq!(out, Output::default());
    }
    assert_eq!(state, State::default());
}

/// On a fork the record the craft fits better wins, unless the other is much nearer
/// in height.
#[test]
fn on_a_fork_the_better_fitting_record_wins() {
    let narrow = Corridor {
        half_width_right: 16.0,
        ..straight()
    };
    let mut fork = input(true);
    fork.here = [Some(narrow), Some(straight())];
    let mut state = settled();
    let out = update(&mut state, &fork, &craft(14.0, 0.0), true, 1.0 / 60.0);
    // `straight` fits (20 - 5 - 14 = 1, no push); `narrow` would push.
    assert_eq!(out.world_force, Vec3::ZERO);
    let far_below = Corridor {
        position: Vec3::new(0.0, -100.0, 0.0),
        ..straight()
    };
    fork.here = [Some(narrow), Some(far_below)];
    let out = update(&mut state, &fork, &craft(14.0, 0.0), true, 1.0 / 60.0);
    assert!(out.world_force.x < 0.0, "the near record pushes");
}
