//! [`crate::between_ticks`]: the blend, its snaps, and that a draw alpha
//! reaches no simulation state.

use super::*;
use crate::between_ticks::{SNAP_DISTANCE, blend_transform, blend_view, frame_alpha};
use oag_gameplay::PlayerInputs;

fn pose(yaw_degrees: f32, at: Vec3) -> Mat4 {
    Mat4::from_rotation_translation(Quat::from_rotation_y(yaw_degrees.to_radians()), at)
}

/// A disc-free grid gliding down the straight fixture past the countdown, so
/// every craft and the chase camera move on every tick. The headless race
/// will not do: with no spline its chase camera is NaN.
fn gliding_race() -> Race {
    let mut race = race_with_a_grid();
    for _ in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
    }
    for ship in &mut race.sim.world.ships {
        ship.physics.body.linear_velocity = Vec3::new(30.0, 0.0, 3.0);
    }
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    race
}

fn translation(matrix: Mat4) -> Vec3 {
    matrix.w_axis.truncate()
}

#[test]
fn alpha_one_returns_the_current_matrix_bit_for_bit() {
    let previous = pose(10.0, Vec3::new(1.0, 2.0, 3.0));
    let current = pose(37.0, Vec3::new(4.0, 2.5, -1.0)) * Mat4::from_scale(Vec3::splat(1.3));
    assert_eq!(blend_transform(previous, current, 1.0), current);
    assert_eq!(blend_view(previous, current, 1.0), current);
    assert_eq!(blend_transform(previous, current, f32::NAN), current);
}

#[test]
fn a_blend_moves_in_a_straight_line_and_turns_evenly() {
    let previous = pose(0.0, Vec3::ZERO);
    let current = pose(90.0, Vec3::new(10.0, 0.0, 0.0));
    let halfway = blend_transform(previous, current, 0.5);
    assert!(translation(halfway).abs_diff_eq(Vec3::new(5.0, 0.0, 0.0), 1e-5));
    let (_, rotation, _) = halfway.to_scale_rotation_translation();
    assert!(rotation.abs_diff_eq(Quat::from_rotation_y(45f32.to_radians()), 1e-5));
    let start = blend_transform(previous, current, 0.0);
    assert!(start.abs_diff_eq(previous, 1e-5));
}

#[test]
fn rotation_takes_the_shorter_arc() {
    // 175 to -175 degrees is ten degrees through 180, not 350 back through 0.
    let halfway = blend_transform(pose(175.0, Vec3::ZERO), pose(-175.0, Vec3::ZERO), 0.5);
    let forward = halfway.transform_vector3(Vec3::Z);
    assert!(
        forward.abs_diff_eq(Vec3::NEG_Z, 1e-4),
        "halfway faces {forward}, not -Z"
    );
}

#[test]
fn a_jump_past_the_snap_distance_draws_where_it_lands() {
    let previous = pose(0.0, Vec3::ZERO);
    let current = pose(0.0, Vec3::new(SNAP_DISTANCE + 1.0, 0.0, 0.0));
    assert_eq!(blend_transform(previous, current, 0.5), current);
    let near = pose(0.0, Vec3::new(SNAP_DISTANCE - 1.0, 0.0, 0.0));
    assert_ne!(blend_transform(previous, near, 0.5), near);
}

#[test]
fn a_view_blend_moves_the_eye_between_the_two_eyes() {
    let previous = pose(0.0, Vec3::new(0.0, 5.0, 0.0)).inverse();
    let current = pose(20.0, Vec3::new(8.0, 5.0, 0.0)).inverse();
    let eye = blend_view(previous, current, 0.25)
        .inverse()
        .w_axis
        .truncate();
    assert!(
        eye.abs_diff_eq(Vec3::new(2.0, 5.0, 0.0), 1e-4),
        "eye at {eye}"
    );
}

#[test]
fn the_frame_alpha_is_the_clock_s_only_while_the_race_steps() {
    let mut clock = TickClock::new(TickRate::from_hz(60));
    clock.advance(TickRate::from_hz(60).dt_nanos() / 4);
    let alpha = clock.interpolation_alpha();
    assert!(alpha > 0.0 && alpha < 1.0);
    assert_eq!(frame_alpha(&clock, true), alpha);
    assert_eq!(
        frame_alpha(&clock, false),
        1.0,
        "a paused race draws its tick"
    );
}

#[test]
fn outside_a_draw_every_reader_is_the_tick_exactly() {
    let race = gliding_race();
    assert_eq!(race.draw_alpha(), 1.0);
    assert_eq!(race.view(), race.tick_view());
    assert_eq!(race.view_unshaken(), race.tick_view_unshaken());
    assert_eq!(
        race.ship_model_matrix_of(0),
        crate::drawable::model_matrix_of(&race.sim.world.ships[0])
    );
}

#[test]
fn a_draw_sits_between_the_last_two_ticks() {
    let mut race = gliding_race();
    let before = translation(crate::drawable::model_matrix_of(&race.sim.world.ships[0]));
    let eye_before = race.camera_position();
    race.tick(&PlayerInputs::none());
    let after = translation(crate::drawable::model_matrix_of(&race.sim.world.ships[0]));
    let eye_after = race.camera_position();
    assert!(
        before.distance(after) > 1e-4,
        "the fixture did not move the craft"
    );

    race.set_draw_alpha(0.25);
    let drawn = translation(race.ship_model_matrix_of(0));
    assert!(
        drawn.abs_diff_eq(before.lerp(after, 0.25), 1e-4),
        "drawn at {drawn}, not a quarter of {before} -> {after}"
    );
    let eye = race.camera_position();
    assert!(
        eye.abs_diff_eq(eye_before.lerp(eye_after, 0.25), 1e-3),
        "eye at {eye}, not a quarter of {eye_before} -> {eye_after}"
    );

    race.set_draw_alpha(1.0);
    assert_eq!(translation(race.ship_model_matrix_of(0)), after);
    assert_eq!(race.camera_position(), eye_after);
}

#[test]
fn every_tick_path_captures_first_and_puts_the_alpha_back() {
    let mut race = gliding_race();
    let check = |race: &mut Race, step: &dyn Fn(&mut Race), name: &str| {
        // A plain tick first, so a step that captured nothing would leave a
        // previous pose one tick older than the one it started from.
        race.tick(&PlayerInputs::none());
        let held = crate::drawable::model_matrix_of(&race.sim.world.ships[0]);
        race.set_draw_alpha(0.5);
        step(race);
        assert_eq!(race.draw_alpha(), 1.0, "{name} left a draw alpha set");
        // What the previous pose is: the one the step started from, read
        // back through a blend at alpha zero.
        race.set_draw_alpha(0.0);
        let previous = race.ship_model_matrix_of(0);
        race.set_draw_alpha(1.0);
        assert!(
            previous.abs_diff_eq(held, 1e-4),
            "{name} did not capture the pose it started from"
        );
    };
    check(
        &mut race,
        &|race| _ = race.tick(&PlayerInputs::none()),
        "tick",
    );
    check(
        &mut race,
        &|race| race.tick_intro(&PlayerInputs::none()),
        "tick_intro",
    );
    check(&mut race, &Race::tick_cosmetics, "tick_cosmetics");
    check(&mut race, &Race::tick_finished, "tick_finished");
}

#[test]
fn a_respawn_snaps_the_craft_and_cuts_the_camera() {
    let mut race = gliding_race();
    race.tick(&PlayerInputs::none());
    race.sim.respawns[0] += 1;
    race.set_draw_alpha(0.5);
    assert_eq!(
        race.ship_model_matrix_of(0),
        crate::drawable::model_matrix_of(&race.sim.world.ships[0]),
        "a respawned craft swept from where it was"
    );
    assert_eq!(race.view(), race.tick_view(), "a cut was swept");
    // Another craft, which did not respawn, still blends.
    assert_ne!(
        race.ship_model_matrix_of(1),
        crate::drawable::model_matrix_of(&race.sim.world.ships[1])
    );
}

#[test]
fn a_view_cycle_snaps_the_camera() {
    let mut race = gliding_race();
    race.tick(&PlayerInputs::none());
    race.set_draw_alpha(0.5);
    assert_ne!(
        race.view(),
        race.tick_view(),
        "the fixture camera did not move"
    );
    race.set_camera_view(oag_display::display::CameraView::Internal);
    assert_eq!(race.view(), race.tick_view());
}

#[test]
fn settling_draws_the_current_tick() {
    let mut race = gliding_race();
    race.tick(&PlayerInputs::none());
    race.settle_poses();
    race.set_draw_alpha(0.5);
    assert_eq!(race.view(), race.tick_view());
}

/// The whole claim the simulation depends on: a draw alpha set and read
/// between every tick, as the window does, moves no state hash.
#[test]
fn a_draw_alpha_reaches_no_state_hash() {
    let mut drawn = Race::start(Setup::headless(Mode::SingleRace, 0xC0FFEE));
    let mut plain = Race::start(Setup::headless(Mode::SingleRace, 0xC0FFEE));
    for tick in 0..120u32 {
        drawn.set_draw_alpha((tick % 7) as f32 / 7.0);
        let _ = (
            drawn.view(),
            drawn.camera_position(),
            drawn.ship_model_matrix_of(0),
            drawn.projection(16.0 / 9.0, 1000.0, oag_display::display::Fov::default()),
            drawn.nozzle_of(0),
        );
        drawn.tick(&PlayerInputs::none());
        plain.tick(&PlayerInputs::none());
        drawn.drain_cues();
        plain.drain_cues();
        assert_eq!(
            drawn.sim.state_hash(),
            plain.sim.state_hash(),
            "a draw alpha moved the hash at tick {tick}"
        );
    }
}
