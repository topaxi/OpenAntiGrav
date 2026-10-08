//! What the two-probe air cushion in [`super`] is asserted to do. Split out of `hover.rs` under the
//! 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;
use crate::collide::{CollisionWorld, Surface, TriangleSoup};
use crate::ship::Body;

fn flat_floor() -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-500.0, 0.0, -500.0],
            [-500.0, 0.0, 500.0],
            [500.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        Surface::Floor,
        0,
    ));
    world
}

/// Arbitrary round numbers, chosen so the arithmetic is checkable by hand.
/// **Not recovered values**: no handling data is reproduced in this
/// repository, and none of these came from a ship.
fn test_handling() -> Handling {
    Handling {
        antigrav: crate::params::Antigrav {
            ride_height: 20.0,
            rebound: 1.0,
            ..crate::params::Antigrav::default()
        },
        physical: crate::params::Physical {
            mass: 1.0,
            normal_gravity: 10.0,
            ..crate::params::Physical::default()
        },
        dimensions: crate::params::Dimensions {
            length: 4.0,
            ..crate::params::Dimensions::default()
        },
        ..Handling::ZERO
    }
}

fn state_at(height: f32) -> ShipState {
    ShipState {
        body: Body {
            position: Vec3::new(0.0, height, 0.0),
            ..Body::default()
        },
        grounded_prev: 1.0,
        ..ShipState::default()
    }
}

/// The spring vanishes at the target, approached from below. It was once asserted *at* the target
/// with the probe in contact, which the recovered reach makes impossible: `Ship_CastHoverProbes`
/// ends the ray at `probe - up * craft+0x2f0`, so the target is where a probe stops finding
/// anything ([`probe`]). The property is unchanged (force proportional to `target - height`, zero
/// with it), so it is asserted as a limit.
#[test]
fn the_spring_vanishes_as_a_probe_approaches_the_target_height() {
    let world = flat_floor();
    let handling = test_handling();
    let target = 5.0;

    let near = probe(
        &state_at(target - 0.001),
        &handling,
        &Environment::default(),
        &world,
        Vec3::ZERO,
        target,
    );
    let far = probe(
        &state_at(target - 0.01),
        &handling,
        &Environment::default(),
        &world,
        Vec3::ZERO,
        target,
    );

    assert!(near.contact && far.contact);
    assert!(near.force.y > 0.0);
    // Ten times closer to the target, ten times less force: linear in the
    // compression, with nothing else in the term.
    assert!((near.force.y * 10.0 - far.force.y).abs() < 1e-4);
}

/// The damping is a multiplier on the spring rather than a summand, so where
/// the spring is vanishing no amount of vertical velocity produces a force.
/// Counter-intuitive, and exactly what the binary does.
#[test]
fn damping_cannot_produce_a_force_where_the_spring_is_zero() {
    let world = flat_floor();
    let handling = test_handling();
    let target = 5.0;
    let mut state = state_at(target - 0.001);
    state.body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);

    let probe = probe(
        &state,
        &handling,
        &Environment::default(),
        &world,
        Vec3::ZERO,
        target,
    );
    // The damper multiplies by at most `1 + rebound * 2`, three times a spring of
    // `mass * 0.3 * 0.001 * K * gravity` = `0.004` on this fixture. So even 50 units/s closing
    // cannot lever a vanishing spring into a real force: the bound is `0.012`, not the `2.0` a
    // summed `-c * v` damper would give.
    assert!(probe.force.length() < 0.02, "force was {:?}", probe.force);
}

#[test]
fn a_probe_below_the_target_height_is_pushed_along_the_ships_up_axis() {
    let world = flat_floor();
    let handling = test_handling();
    let probe = probe(
        &state_at(3.0),
        &handling,
        &Environment::default(),
        &world,
        Vec3::ZERO,
        5.0,
    );
    assert!(probe.force.y > 0.0);
    assert_eq!(probe.force.x, 0.0);
    assert_eq!(probe.force.z, 0.0);
}

/// **The suspension is compression-only**, replacing a test that asserted the opposite.
/// `a_probe_above_the_target_height_is_pulled_back_down` pinned a spring that pulled a too-high
/// ship down, a consequence of casting `ride_height` while springing against a
/// `0.75 * ride_height` target, which the binary does not have. The ray's far end is the target
/// (`0x0884a0b4`), so above it there is no contact, force or pull: the probe is airborne.
#[test]
fn a_probe_above_the_target_height_finds_nothing_at_all() {
    let world = flat_floor();
    let handling = test_handling();
    let probe = probe(
        &state_at(8.0),
        &handling,
        &Environment::default(),
        &world,
        Vec3::ZERO,
        5.0,
    );
    assert!(!probe.contact);
    assert_eq!(probe.force, Vec3::ZERO);
}

/// The target height is the raycast length, so a probe further above the floor than the height it
/// holds finds nothing. (The name says `ride_height` because the fixture's target derives from
/// it; the reach is the target, see [`probe`].)
#[test]
fn a_probe_beyond_ride_height_finds_no_surface() {
    let world = flat_floor();
    let handling = test_handling();
    let probe = probe(
        &state_at(handling.antigrav.ride_height + 1.0),
        &handling,
        &Environment::default(),
        &world,
        Vec3::ZERO,
        5.0,
    );
    assert!(!probe.contact);
    assert_eq!(probe.force, Vec3::ZERO);
}

#[test]
fn a_wall_is_not_a_hoverable_surface() {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-500.0, 0.0, -500.0],
            [-500.0, 0.0, 500.0],
            [500.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        Surface::Wall,
        0,
    ));

    let probe = probe(
        &state_at(3.0),
        &test_handling(),
        &Environment::default(),
        &world,
        Vec3::ZERO,
        5.0,
    );
    assert!(!probe.contact);
}

/// Penetration escape fires on `Floor` and on nothing else. **Narrower than contact, the part that
/// is easy to get backwards**: the gate is `craft+0x208 == 1`, the `Floor` class, applied *before*
/// the collider's type is looked at, so the ordering says "any surface" while the gate says "one".
/// This module escaped on `Floor` or `MagFloor` until 2026-08-12 (an open divergence in
/// `HANDOVER.md`) and briefly on everything, which was worse.
#[test]
fn penetration_escape_fires_on_a_floor_and_on_nothing_else() {
    let world = |surface| {
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [-500.0, 0.0, -500.0],
                [-500.0, 0.0, 500.0],
                [500.0, 0.0, 0.0],
            ],
            vec![[0, 1, 2]],
            Vec::new(),
            surface,
            0,
        ));
        world
    };
    // Half a unit above the surface, so inside `PENETRATION_LIMIT`.
    let inside = |surface| {
        probe(
            &state_at(0.5),
            &test_handling(),
            &Environment::default(),
            &world(surface),
            Vec3::ZERO,
            5.0,
        )
    };

    let floor = inside(Surface::Floor);
    assert!(
        (floor.escape.y - (PENETRATION_LIMIT - 0.5)).abs() < 1e-5,
        "a floor should push the body out by the shortfall, got {:?}",
        floor.escape
    );
    assert!(floor.contact, "and it still carries the suspension");

    // A magstrip holds the craft kinematically and needs no hard
    // constraint, so it hovers without escaping.
    let mag = inside(Surface::MagFloor);
    assert!(mag.contact, "a magstrip is still a hoverable surface");
    assert_eq!(mag.escape, Vec3::ZERO);

    // And a wall is `crate::wall`'s business in both respects.
    let wall = inside(Surface::Wall);
    assert!(!wall.contact, "a wall must never carry the suspension");
    assert_eq!(wall.force, Vec3::ZERO);
    assert_eq!(wall.escape, Vec3::ZERO);

    // Clear of the floor there is nothing to escape from.
    let clear = probe(
        &state_at(3.0),
        &test_handling(),
        &Environment::default(),
        &world(Surface::Floor),
        Vec3::ZERO,
        5.0,
    );
    assert_eq!(clear.escape, Vec3::ZERO);
}

#[test]
fn a_mag_floor_hovers_exactly_like_a_floor() {
    let handling = test_handling();
    let state = state_at(3.0);

    let mut floor = CollisionWorld::new();
    floor.push(TriangleSoup::new(
        vec![
            [-500.0, 0.0, -500.0],
            [-500.0, 0.0, 500.0],
            [500.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        Surface::Floor,
        0,
    ));
    let mut mag = CollisionWorld::new();
    mag.push(TriangleSoup::new(
        vec![
            [-500.0, 0.0, -500.0],
            [-500.0, 0.0, 500.0],
            [500.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        Surface::MagFloor,
        0,
    ));

    let a = probe(
        &state,
        &handling,
        &Environment::default(),
        &floor,
        Vec3::ZERO,
        5.0,
    );
    let b = probe(
        &state,
        &handling,
        &Environment::default(),
        &mag,
        Vec3::ZERO,
        5.0,
    );
    assert_eq!(a.force, b.force);
}

#[test]
fn a_full_magstrip_blend_cancels_the_ordinary_suspension() {
    let world = flat_floor();
    let handling = test_handling();
    let mut state = state_at(3.0);
    state.mag_lock_blend = 1.0;

    let probe = probe(
        &state,
        &handling,
        &Environment::default(),
        &world,
        Vec3::ZERO,
        5.0,
    );
    assert!(probe.contact);
    assert_eq!(probe.force, Vec3::ZERO);
}

#[test]
fn penetration_escape_engages_only_within_the_last_unit() {
    let world = flat_floor();
    let handling = test_handling();

    let clear = probe(
        &state_at(1.5),
        &handling,
        &Environment::default(),
        &world,
        Vec3::ZERO,
        5.0,
    );
    assert_eq!(clear.escape, Vec3::ZERO);

    let deep = probe(
        &state_at(0.25),
        &handling,
        &Environment::default(),
        &world,
        Vec3::ZERO,
        5.0,
    );
    assert_eq!(deep.escape, Vec3::new(0.0, 0.75, 0.0));
}

#[test]
fn both_probes_reach_the_floor_under_a_level_ship() {
    let world = flat_floor();
    let handling = test_handling();
    let hover = evaluate(
        &state_at(3.0),
        &handling,
        &Environment::default(),
        &world,
        5.0,
    );
    assert_eq!(hover.contacts, 2);
    assert_eq!(hover.average_normal, Vec3::Y);
}

/// A level ship over a level floor has nothing to align to, which is why the
/// alignment torque's sign cannot be pinned by a flat-floor test.
#[test]
fn a_level_ship_over_a_level_floor_gets_no_alignment_torque() {
    let world = flat_floor();
    let hover = evaluate(
        &state_at(3.0),
        &test_handling(),
        &Environment::default(),
        &world,
        5.0,
    );
    assert_eq!(hover.alignment_torque, Vec3::ZERO);
}

/// The alignment torque points in the **aligning** direction: the invariant the `-400` versus
/// `+400` question reduces to, asserted on the torque rather than through a simulation. A torque
/// along `+cross(up, n)` moves the up axis along `n - up * (n . up)`, from `up` toward `n`.
/// Nothing about the gain's magnitude, the inertia tensor, the probe placement or the target
/// height can change that sign, which is why the sign is settleable and the magnitude is not
/// ([`ALIGNMENT_GAIN`]).
#[test]
fn the_alignment_torque_points_from_the_ships_up_axis_toward_the_surface_normal() {
    let world = flat_floor();
    let handling = test_handling();

    for angle in [-0.6f32, -0.02, 0.02, 0.6] {
        let mut state = state_at(3.0);
        state.body.orientation = oag_core::math::Quat::from_rotation_z(angle);

        let hover = evaluate(&state, &handling, &Environment::default(), &world, 5.0);
        assert!(hover.contacts > 0);

        let aligning = state.body.up().cross(hover.average_normal);
        assert!(
            hover.alignment_torque.dot(aligning) > 0.0,
            "torque {:?} was not aligning at a roll of {angle}",
            hover.alignment_torque
        );
    }
}

/// The documented property of the alignment torque is the absence of a pitch
/// component, which is independent of its direction, so it gets its own test.
#[test]
fn the_alignment_torque_never_has_a_pitch_component() {
    let world = flat_floor();
    let handling = test_handling();

    // Rolled and yawed away from level, so `cross(up, normal)` is non-zero.
    let mut state = state_at(3.0);
    state.body.orientation =
        oag_core::math::Quat::from_rotation_z(0.2) * oag_core::math::Quat::from_rotation_y(0.4);

    let hover = evaluate(&state, &handling, &Environment::default(), &world, 5.0);
    assert!(hover.contacts > 0);
    assert_ne!(hover.alignment_torque, Vec3::ZERO);

    let right = state.body.right();
    assert!(
        hover.alignment_torque.dot(right).abs() < 1e-5,
        "pitch component was {}",
        hover.alignment_torque.dot(right)
    );
}

/// A banked craft on the grid gets no bank-to-yaw coupling, and the same craft racing does:
/// `Ship_HoverTwoPoint` guards the term with `craft+0x2a4 != 0`. Measured on PPSSPP, the
/// original's craft holds its heading through the countdown on a banked start and yaws from GO
/// (`docs/physics/grid-state.md`).
#[test]
fn the_bank_to_yaw_coupling_is_skipped_on_the_grid_and_only_there() {
    let world = flat_floor();
    let handling = test_handling();
    let mut state = state_at(3.0);
    state.body.orientation = oag_core::math::Quat::from_rotation_z(0.2);

    let racing = evaluate(&state, &handling, &Environment::default(), &world, 5.0);
    assert!(
        racing.local_angular_torque.y.abs() > 1.0,
        "a banked craft yaws"
    );

    state.on_grid = true;
    let held = evaluate(&state, &handling, &Environment::default(), &world, 5.0);
    assert_eq!(held.local_angular_torque, Vec3::ZERO);
    // Only that term: the surface alignment still levels a craft on the grid,
    // which is what the original's pitch and roll settle through the countdown
    // shows.
    assert_eq!(held.alignment_torque, racing.alignment_torque);
}

#[test]
fn an_airborne_ship_gets_none_of_the_grounded_terms() {
    let world = flat_floor();
    let handling = test_handling();
    let hover = evaluate(
        &state_at(handling.antigrav.ride_height + 10.0),
        &handling,
        &Environment::default(),
        &world,
        5.0,
    );
    assert_eq!(hover.contacts, 0);
    assert_eq!(hover.alignment_torque, Vec3::ZERO);
    assert_eq!(hover.downforce, Vec3::ZERO);
    assert_eq!(hover.local_angular_torque, Vec3::ZERO);
    assert_eq!(hover.escape, Vec3::ZERO);
}

#[test]
fn the_rebound_coefficient_leaves_the_landing_window_at_the_plain_rebound() {
    let handling = Handling {
        antigrav: crate::params::Antigrav {
            rebound: 2.0,
            landing_rebound: 7.0,
            ..crate::params::Antigrav::default()
        },
        ..Handling::ZERO
    };

    assert_eq!(rebound_coefficient(&handling, LANDING_WINDOW), 2.0);
    assert_eq!(rebound_coefficient(&handling, 10.0), 2.0);
    // Inside the window, at touchdown, only `landing_rebound` contributes.
    assert_eq!(rebound_coefficient(&handling, 0.0), 3.5);
}

/// Two penetrating probes each ask for a teleport, and applying both would
/// move the body by their sum, which is further than either wanted. The larger
/// wins. That is a choice made here and not a finding, so it is pinned.
#[test]
fn two_penetrating_probes_move_the_body_by_the_deeper_one_and_not_by_their_sum() {
    // A floor at y = 0 under the front probe and a floor at y = -0.5 under the
    // rear one, both wound to face up.
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-100.0, 0.0, -100.0],
            [-100.0, 0.0, -0.1],
            [100.0, 0.0, -0.1],
            [100.0, 0.0, -100.0],
            [-100.0, -0.5, 0.1],
            [-100.0, -0.5, 100.0],
            [100.0, -0.5, 100.0],
            [100.0, -0.5, 0.1],
        ],
        vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]],
        Vec::new(),
        Surface::Floor,
        0,
    ));

    let handling = test_handling();
    // The probes hang [`PROBE_DROP_RAW`] * [`TARGET_GLOBAL_SCALE`] below the centre of mass, so
    // the body sits that much higher to put the front probe 0.2 above its floor; the numbers
    // below live at the probes.
    let hover = evaluate(
        &state_at(0.2 + PROBE_DROP_RAW * TARGET_GLOBAL_SCALE),
        &handling,
        &Environment::default(),
        &world,
        5.0,
    );

    assert_eq!(hover.contacts, 2);
    // Front probe 0.2 above its floor, rear 0.7 above its own. Approximate: the `1.125` drop adds
    // a subtraction, so the heights land a single ulp off the round numbers.
    assert!((hover.probes[0].escape - Vec3::new(0.0, 0.8, 0.0)).length() < 1e-6);
    assert!((hover.probes[1].escape - Vec3::new(0.0, 0.3, 0.0)).length() < 1e-6);
    assert!((hover.escape - Vec3::new(0.0, 0.8, 0.0)).length() < 1e-6);
}

/// The roll oscillator is stable, and the margin is pinned so it cannot drift back.
///
/// Rewritten twice. With the transcribed gain `400` reaching the body as a bare angular
/// acceleration the oscillator ran at 20 rad/s with `det = 1.0402`, growing roll about 2 % a tick
/// until the ship inverted. A fitted `ALIGNMENT_INERTIA = 19.8` made it stable at `4.49` rad/s,
/// the frequency measured off the original's step response.
///
/// **The divisor is now the recovered inertia tensor** and the fitted constant is gone: the
/// accumulators hold torque, so [`crate::integrate`] divides this term by
/// [`crate::forces::ship_inertia`]'s roll entry, `15.6` (`Body_SetBoxInertia`'s literal box).
/// **The numbers below are the NEW behaviour, not the original's `4.49`**: `400 / 15.6` gives
/// about `5.06` rad/s, 13 % fast, pinned rather than tuned out. The likeliest explanation is that
/// the term is spread across roll (`15.6`) and yaw (`21.6`), the fit's `19.8` sitting between;
/// nothing has measured it.
///
/// The arithmetic is **not** the `h <= c/k` form: the acceleration is computed once from the
/// frame's starting state and held across all three sub-steps, so the governing step is the frame
/// `H = 1/60` ([`crate::integrate`]). Any change to [`ALIGNMENT_GAIN`],
/// [`crate::forces::ROLL_INVERSE_INERTIA`], [`crate::passive::ROLL_DAMPING`] or
/// [`crate::ship::SUBSTEPS`] moves these numbers, which is the point.
#[test]
fn the_roll_oscillator_is_stable_by_the_margin_that_was_measured() {
    let k = ALIGNMENT_GAIN * crate::forces::ROLL_INVERSE_INERTIA;
    let c = -crate::passive::ROLL_DAMPING;
    let h = 1.0f32 / 60.0;

    let det = (1.0 - k * h * h / 3.0) * (1.0 - c * h) + k * h * h - c * k * h * h * h / 3.0;
    let per_tick = det.sqrt();

    assert!(
        det < 1.0,
        "the roll oscillator is unstable again (det {det}); a ship at rest will \
         tumble within a few hundred ticks. See ALIGNMENT_GAIN for the measurement."
    );
    assert!(
        (per_tick - 0.98553).abs() < 1e-3,
        "roll now decays {per_tick} per tick, not the 0.98553 the recovered \
         tensor gives"
    );

    // The frequency the recovered tensor produces, against the original's own
    // measured 4.49 rad/s. Pinned as what this crate does, with the gap stated.
    let omega = k.sqrt();
    assert!(
        (omega - 5.06).abs() < 0.05,
        "the oscillator runs at {omega} rad/s, not the 5.06 that 400/15.6 gives"
    );
    assert!(
        omega > 4.49,
        "the recovered tensor is stiffer than the original's measured 4.49 \
         rad/s, not softer; if this ever flips, the 13 % gap has changed sign \
         and the explanation in this test's docs is wrong"
    );

    // And the stability condition in the form the documentation quotes.
    let needed = (2.0 / 3.0) * k * h;
    assert!(
        c > needed,
        "damping {c} is below the {needed} this frame time needs"
    );
}

/// `ride_height` is the primary term of the hover target (the correction
/// `docs/ghidra/functions/psp-pulse-usa/engine.md` made to `docs/physics/README.md`) and, since
/// the `craft+0x74` offset was dropped, the *only* term. `antigrav_height_adjust` is left set in
/// the fixture on purpose: the assertion is that it does **not** contribute, pinning the removal
/// ([`target_height`] on why a positive offset leaves the model with no resting height).
#[test]
fn the_hover_target_is_built_from_ride_height() {
    let handling = Handling {
        antigrav: crate::params::Antigrav {
            ride_height: 12.0,
            ..crate::params::Antigrav::default()
        },
        pitch: crate::params::Pitch {
            antigrav_height_adjust: 3.0,
            ..crate::params::Pitch::default()
        },
        ..Handling::ZERO
    };

    assert_eq!(
        target_height(&handling, 0.0, 0.0),
        12.0 * TARGET_GLOBAL_SCALE
    );
}

/// The target must never exceed the raycast length: they are the same field, and a target beyond
/// the reach has no fixed point (every reportable height is below it, so the spring only pushes
/// up). Asserted across the slowdown timer, the only other term that moves the target under
/// ordinary suspension.
///
/// **Not across `mag_lock_blend`**: a full lock scales the target by 1.2, beyond the reach. That
/// is harmless because the same blend multiplies the probe force by `1 - mag_lock_blend`,
/// cancelling the suspension ([`probe`]), and on a strip [`crate::maglock`] decides the height by
/// displacement at `0.8` of this target.
#[test]
fn the_hover_target_never_exceeds_the_probes_reach() {
    let handling = Handling {
        antigrav: crate::params::Antigrav {
            ride_height: 5.5,
            ..crate::params::Antigrav::default()
        },
        pitch: crate::params::Pitch {
            antigrav_height_adjust: 1.0,
            ..crate::params::Pitch::default()
        },
        ..Handling::ZERO
    };

    for timer in [0.0f32, 1.5, 100.0] {
        let target = target_height(&handling, 0.0, timer);
        assert!(
            target <= handling.antigrav.ride_height,
            "target {target} exceeded the reach at slowdown timer {timer}"
        );
    }
}

/// A magstrip lock raises the target by a fifth, and the slowdown timer lowers it by up
/// to four units while it runs.
#[test]
fn the_hover_target_responds_to_the_magstrip_blend_and_the_slowdown_timer() {
    let handling = Handling {
        antigrav: crate::params::Antigrav {
            ride_height: 10.0,
            ..crate::params::Antigrav::default()
        },
        ..Handling::ZERO
    };

    let k2 = TARGET_GLOBAL_SCALE;
    assert_eq!(target_height(&handling, 1.0, 0.0), 12.0 * k2);
    assert_eq!(target_height(&handling, 0.0, 1.5), 8.5 * k2);
    // Clamped at four, however long the timer says.
    assert_eq!(target_height(&handling, 0.0, 100.0), 6.0 * k2);
    // And a spent timer takes nothing off.
    assert_eq!(target_height(&handling, 0.0, 0.0), 10.0 * k2);
}

/// A title's grid clamp lowers the target before the global scale and never raises it; `None`
/// is the ordinary target to the bit.
#[test]
fn the_grid_clamp_lowers_the_pre_scale_target_and_none_changes_nothing() {
    let handling = Handling {
        antigrav: crate::params::Antigrav {
            ride_height: 5.5,
            ..crate::params::Antigrav::default()
        },
        ..Handling::ZERO
    };

    let k2 = TARGET_GLOBAL_SCALE;
    assert_eq!(
        capped_target_height(&handling, 0.0, 0.0, None),
        target_height(&handling, 0.0, 0.0)
    );
    assert_eq!(
        capped_target_height(&handling, 0.0, 0.0, Some(3.0)),
        3.0 * k2
    );
    // A clamp above the race value is no clamp.
    assert_eq!(
        capped_target_height(&handling, 0.0, 0.0, Some(9.0)),
        5.5 * k2
    );
    // The magstrip gain is applied before the clamp, as the original orders it.
    assert_eq!(
        capped_target_height(&handling, 1.0, 0.0, Some(3.0)),
        3.0 * k2
    );
}

/// `Handling::ZERO` has `ride_height = 0.0`, so the probe segment has zero
/// length and cannot hit anything. That is the mechanism behind the
/// crate-level "nothing accelerates" invariant, so it is pinned here too.
#[test]
fn a_zero_handling_ship_never_finds_the_floor() {
    let world = flat_floor();
    let hover = evaluate(
        &state_at(0.0),
        &Handling::ZERO,
        &Environment::default(),
        &world,
        0.0,
    );
    assert_eq!(hover.contacts, 0);
}

/// Zone runs `Ship_HoverFourCorner`, whose bank-to-yaw gain is `50.0` where the
/// two-point law's is `30.0` (`0x0884b778`, `lui 0x4248`). Both keep the grid guard.
#[test]
fn the_four_corner_bank_coupling_is_fifty_over_thirty_and_keeps_the_grid_guard() {
    let world = flat_floor();
    let handling = test_handling();
    let mut state = state_at(3.0);
    state.body.orientation = oag_core::math::Quat::from_rotation_z(0.2);

    let two_point = evaluate(&state, &handling, &Environment::default(), &world, 5.0);
    state.four_corner = true;
    let four_corner = evaluate(&state, &handling, &Environment::default(), &world, 5.0);
    let ratio = four_corner.local_angular_torque.y / two_point.local_angular_torque.y;
    assert!((ratio - 50.0 / 30.0).abs() < 1e-5, "ratio was {ratio}");

    state.on_grid = true;
    let held = evaluate(&state, &handling, &Environment::default(), &world, 5.0);
    assert_eq!(held.local_angular_torque, Vec3::ZERO);
}
