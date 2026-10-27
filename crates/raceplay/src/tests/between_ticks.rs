//! [`crate::between_ticks`]: the blend, its snaps, and that a draw alpha
//! reaches no simulation state.

use super::*;
use crate::CannonDraw;
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

#[test]
fn a_projectile_in_flight_is_drawn_between_its_ticks() {
    let mut race = super::cannon_draw::one_round(2);
    let slot = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .position(|p| p.kind.is_some())
        .expect("one round in flight");
    let before = race.sim.world.projectiles.slots[slot].position;
    race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
    race.tick(&PlayerInputs::none());
    let after = race.sim.world.projectiles.slots[slot].position;
    assert!(before.distance(after) > 0.5, "the round did not fly");
    let rounds = |race: &Race| race.cannon_model_matrices(&CannonDraw::default());
    race.set_draw_alpha(0.5);
    let drawn = translation(rounds(&race)[0]);
    assert!(
        drawn.abs_diff_eq(before.lerp(after, 0.5), 1e-3),
        "drawn at {drawn}, not halfway along {before} -> {after}"
    );
    race.set_draw_alpha(1.0);
    assert_eq!(translation(rounds(&race)[0]), after);

    // The slot refilled inside the tick: a new shot, drawn where it is.
    race.tick(&PlayerInputs::none());
    race.sim.world.projectiles.slots[slot].lifetime += 1.0;
    race.set_draw_alpha(0.5);
    assert_eq!(
        translation(rounds(&race)[0]),
        race.sim.world.projectiles.slots[slot].position
    );
}

#[test]
fn the_trail_head_follows_the_drawn_nozzle_and_only_between_ticks() {
    let mut race = gliding_race();
    race.view.nozzles = vec![Some(Vec3::new(0.0, 0.0, 4.0)); MAX_SHIPS];
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.trail_head_of(0),
        None,
        "a tick-exact draw keeps the sample"
    );
    race.set_draw_alpha(0.5);
    assert_eq!(
        race.trail_head_of(0).map(|head| head.position),
        race.nozzle_of(0)
    );
    assert_eq!(race.trail_head_of(0).map(|head| head.alpha), Some(0.5));
    assert_ne!(
        race.nozzle_of(0),
        Some(
            crate::drawable::model_matrix_of(&race.sim.world.ships[0])
                .transform_point3(Vec3::new(0.0, 0.0, 4.0))
        ),
        "the nozzle is not on the blended hull"
    );
}

#[test]
fn the_clock_runs_between_ticks_only_while_the_world_steps() {
    let mut race = gliding_race();
    race.tick(&PlayerInputs::none());
    let tick = race.sim.world.tick as f32;
    assert_eq!(race.clock_seconds(), tick / 60.0);
    race.set_draw_alpha(0.25);
    assert!((race.clock_seconds() - (tick - 0.75) / 60.0).abs() < 1e-6);
    // A cosmetic tick holds the world still, and the clock holds with it.
    race.tick_cosmetics();
    race.set_draw_alpha(0.25);
    assert_eq!(race.clock_seconds(), tick / 60.0);
}

#[test]
fn the_world_meshes_clock_is_the_race_clock_plus_the_captures_offset_and_nothing_else() {
    let mut race = gliding_race();
    race.tick(&PlayerInputs::none());
    // A race has no offset: the two clocks are one.
    assert_eq!(race.world_clock_offset(), 0.0);
    assert_eq!(race.world_clock_seconds(), race.clock_seconds());
    race.view.world_clock_offset = 11.605;
    assert!((race.world_clock_seconds() - race.clock_seconds() - 11.605).abs() < 1e-5);
    // The race's own clock, which the HUD and the gantry read, does not move.
    assert_eq!(race.clock_seconds(), race.sim.world.tick as f32 / 60.0);
}

#[test]
fn the_ghost_is_drawn_between_its_two_poses() {
    let mut race = gliding_race();
    race.tick(&PlayerInputs::none());
    let before = oag_replay::Pose {
        position: Vec3::new(0.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
    };
    let now = oag_replay::Pose {
        position: Vec3::new(4.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
    };
    assert_eq!(
        race.ghost_draw_matrix(&now, &before),
        Race::ghost_model_matrix(&now)
    );
    race.set_draw_alpha(0.5);
    let drawn = translation(race.ghost_draw_matrix(&now, &before));
    assert!(
        drawn.abs_diff_eq(Vec3::new(2.0, 0.0, 0.0), 1e-4),
        "ghost at {drawn}"
    );
}

#[test]
fn the_body_a_blob_shadow_casts_from_is_the_drawn_one() {
    let mut race = gliding_race();
    let before = race.sim.world.ships[0].physics.body.position;
    race.tick(&PlayerInputs::none());
    let after = race.sim.world.ships[0].physics.body.position;
    assert_eq!(race.body_between(0), race.sim.world.ships[0].physics.body);
    race.set_draw_alpha(0.25);
    let drawn = race.body_between(0).position;
    assert!(
        drawn.abs_diff_eq(before.lerp(after, 0.25), 1e-4),
        "cast from {drawn}, not a quarter of {before} -> {after}"
    );
    assert!(drawn.abs_diff_eq(translation(race.ship_model_matrix_of(0)), 1e-4));
}

/// The airbrake flaps and the shield shell's swell are drawn between their
/// last two ticks' values, and at alpha 1 are the tick's exactly.
#[test]
fn the_flaps_and_the_shield_swell_move_between_ticks() {
    let mut race = gliding_race();
    race.view.flaps[0] = [20.0, 40.0];
    race.view.shield[0].activate();
    // What the tick captures as its previous pose, before it steps anything.
    let before_swell = race.view.shield[0].swell();
    race.tick(&PlayerInputs::none());
    // What this tick moved them to: the flaps out, a hit landing on the shell.
    race.view.flaps[0] = [60.0, 0.0];
    race.view.shield[0].hit();
    let state = race.view.shield[0];
    assert_ne!(
        before_swell,
        state.swell(),
        "the hit did not swell the shell"
    );

    let at_tick = race.airbrake_flaps_of(0);
    assert_eq!(race.shield_scale_of(0), state.scale());
    for alpha in [0.0f32, 0.25, 0.5, 0.75] {
        race.set_draw_alpha(alpha);
        let levels = race.flaps_between(0);
        assert!(
            (levels[0] - (20.0 + 40.0 * alpha)).abs() < 1e-4,
            "{alpha}: {levels:?}"
        );
        assert!(
            (levels[1] - (40.0 - 40.0 * alpha)).abs() < 1e-4,
            "{alpha}: {levels:?}"
        );
        let swell = before_swell + (state.swell() - before_swell) * alpha;
        let expected = state.scale() - state.swell() + swell;
        assert!((race.shield_scale_of(0) - expected).abs() < 1e-5, "{alpha}");
    }
    race.set_draw_alpha(1.0);
    assert_eq!(race.airbrake_flaps_of(0), at_tick);
    assert_eq!(race.shield_scale_of(0).to_bits(), state.scale().to_bits());
}
