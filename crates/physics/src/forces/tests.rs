//! What one frame of force evaluation in [`super`] is asserted to do. Split out of `forces.rs`
//! under the 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;
use crate::collide::{CollisionWorld, Surface, TriangleSoup};
use crate::params::{Antigrav, Brakes, Engine, Physical};
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

/// Arbitrary round numbers, in the scaled in-memory form. **Not recovered
/// values.**
fn test_handling() -> Handling {
    Handling {
        antigrav: Antigrav {
            ride_height: 6.0,
            rebound: 1.0,
            ..Antigrav::default()
        },
        physical: Physical {
            mass: 1.0,
            normal_gravity: 10.0,
            flight_gravity: 4.0,
            ..Physical::default()
        },
        ..Handling::ZERO
    }
}

fn ship_at(height: f32) -> ShipState {
    ShipState {
        body: Body {
            position: Vec3::new(0.0, height, 0.0),
            ..Body::default()
        },
        ..ShipState::default()
    }
}

/// The recovered tensor, pinned as the three numbers the binary computes. Not a test of this
/// crate's arithmetic: `(15.6, 21.6, 15.6)` is what `scripts/trace-angular-fit.py` independently
/// measured off two real captures as `~(15, 21..22, 14..16)`, the point of contact between an
/// instruction read and a hardware measurement. Editing [`INERTIA_BOX_X`] or [`INERTIA_MASS`]
/// trips this when the tensor no longer matches the original's recordings.
#[test]
fn the_recovered_inertia_tensor_is_a_solid_box() {
    let xx =
        12.0 / (INERTIA_MASS * (INERTIA_BOX_Y * INERTIA_BOX_Y + INERTIA_BOX_Z * INERTIA_BOX_Z));
    let zz =
        12.0 / (INERTIA_MASS * (INERTIA_BOX_X * INERTIA_BOX_X + INERTIA_BOX_Y * INERTIA_BOX_Y));

    // The box is square in plan, so pitch and roll inertia are equal and the
    // yaw one is the odd axis out - the `x == z` symmetry the fit also found.
    assert_eq!(xx, zz);
    assert!(
        YAW_INVERSE_INERTIA < xx,
        "yaw must be the hardest axis to turn"
    );

    // (15.6, 21.6, 15.6), to a tolerance far tighter than the fit's own spread.
    assert!((1.0 / xx - 15.6).abs() < 0.01, "I_xx was {}", 1.0 / xx);
    assert!(
        (1.0 / YAW_INVERSE_INERTIA - 21.6).abs() < 0.01,
        "I_yy was {}",
        1.0 / YAW_INVERSE_INERTIA
    );
}

/// The staleness invariant, asserted where it is observable: on the frame a ship
/// first touches down, gravity must still be the **airborne** one even though the
/// ship ends that frame grounded.
#[test]
fn the_frame_a_ship_lands_uses_airborne_gravity_and_grounded_grip() {
    let handling = test_handling();
    let world = flat_floor();
    let mut state = ship_at(4.0);
    state.body.linear_velocity = Vec3::new(3.0, -1.0, 0.0);
    state.grounded = 0.0;

    let evaluated = evaluate(
        &mut state,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );

    assert_eq!(evaluated.control_grounded, 0.0);
    assert_eq!(evaluated.contact_grounded, 1.0);
    assert_eq!(state.grounded, 1.0);
    // `flight_gravity` is 4 and `normal_gravity` is 10, so this is unambiguous.
    assert_eq!(evaluated.gravity.y, -handling.physical.flight_gravity);
}

/// And the reverse on the frame it leaves the ground: gravity is still the
/// grounded one.
#[test]
fn the_frame_a_ship_takes_off_uses_grounded_gravity() {
    let handling = test_handling();
    let world = flat_floor();
    // Above `ride_height`, so no probe can reach the floor this frame.
    let mut state = ship_at(handling.antigrav.ride_height + 5.0);
    state.grounded = 1.0;

    let evaluated = evaluate(
        &mut state,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );

    assert_eq!(evaluated.control_grounded, 1.0);
    assert_eq!(evaluated.contact_grounded, 0.0);
    assert_eq!(evaluated.gravity.y, -handling.physical.normal_gravity);
}

/// The engine reads the same stale value, which is what makes a landing frame
/// give a ship 20 % thrust even though it ends that frame on the ground.
#[test]
fn the_engine_reads_the_stale_groundedness_too() {
    let handling = Handling {
        engine: Engine {
            accelcap: 1000.0,
            amount: 0.4,
            ..Engine::default()
        },
        ..test_handling()
    };
    let world = flat_floor();
    let full_throttle = ShipControls {
        thrust: 1.0,
        ..ShipControls::default()
    };

    let mut landing = ship_at(4.0);
    landing.grounded = 0.0;
    let landing_eval = evaluate(
        &mut landing,
        &full_throttle,
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );

    let mut settled = ship_at(4.0);
    settled.grounded = 1.0;
    let settled_eval = evaluate(
        &mut settled,
        &full_throttle,
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );

    assert_eq!(landing_eval.contact_grounded, settled_eval.contact_grounded);
    assert!(
        landing_eval.engine.thrust < settled_eval.engine.thrust,
        "landing thrust {} was not below settled thrust {}",
        landing_eval.engine.thrust,
        settled_eval.engine.thrust
    );
}

/// Braking is gated on the stale contact flag, not scaled by it, so a ship that
/// was airborne last frame gets no brake force at all this frame.
#[test]
fn braking_does_nothing_in_the_air() {
    let handling = Handling {
        brakes: Brakes {
            amount: -0.5,
            gain: 400.0,
            falloff: 200.0,
        },
        ..test_handling()
    };
    let world = flat_floor();
    let held = ShipControls {
        airbrake_left: 1.0,
        airbrake_right: 1.0,
        ..ShipControls::default()
    };

    let mut airborne = ship_at(4.0);
    airborne.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
    airborne.brake = 100.0;
    airborne.grounded = 0.0;
    let airborne_eval = evaluate(
        &mut airborne,
        &held,
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );
    assert_eq!(airborne_eval.brakes, Vec3::ZERO);

    let mut grounded = ship_at(4.0);
    grounded.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
    grounded.brake = 100.0;
    grounded.grounded = 1.0;
    let grounded_eval = evaluate(
        &mut grounded,
        &held,
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );
    assert!(grounded_eval.brakes.z > 0.0);
}

/// The weapon slowdown timer suppresses lateral grip entirely while it runs.
#[test]
fn lateral_grip_is_suppressed_while_the_slowdown_timer_runs() {
    let handling = Handling {
        antigrav: Antigrav {
            grip_ground: 2.0,
            grip_air: 1.0,
            ..test_handling().antigrav
        },
        ..test_handling()
    };
    let world = flat_floor();

    let mut sliding = ship_at(4.0);
    sliding.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
    sliding.grounded = 1.0;
    let free = evaluate(
        &mut sliding,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );
    assert_ne!(free.lateral_grip, Vec3::ZERO);

    let mut slowed = ship_at(4.0);
    slowed.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
    slowed.grounded = 1.0;
    slowed.slowdown_timer = 1.0;
    let held = evaluate(
        &mut slowed,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );
    assert_eq!(held.lateral_grip, Vec3::ZERO);
}

/// The slowdown timer decays inside the force evaluation and **lands negative, not at zero**, on
/// the tick it expires: the original is `if (t > 0.0f) t -= dt;` with no clamp, so the field
/// freezes one `dt` below zero. The residue matters: `crate::slowdown::add` adds into this field
/// before clamping, so the next hit is worth that much less. (`airbrake::advance_sideshift`
/// clamps its equivalents because nothing reads them arithmetically; see the comment beside the
/// decrement in [`super::evaluate`].)
#[test]
fn the_slowdown_timer_decays_past_zero_and_freezes_there() {
    let handling = test_handling();
    let world = flat_floor();
    let dt = 1.0 / 60.0;

    let mut ship = ship_at(4.0);
    ship.slowdown_timer = dt * 1.5;

    evaluate(
        &mut ship,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        dt,
    );
    assert_eq!(ship.slowdown_timer, dt * 1.5 - dt);

    // The tick that crosses zero: gated, so it goes negative exactly once.
    evaluate(
        &mut ship,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        dt,
    );
    let expired = ship.slowdown_timer;
    assert!(expired < 0.0, "expected a negative residue, got {expired}");

    // And freezes: an ungated `t -= dt` would drift without bound, which would
    // make an arbitrarily old craft immune to the next hit it takes.
    for _ in 0..600 {
        evaluate(
            &mut ship,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            dt,
        );
    }
    assert_eq!(ship.slowdown_timer, expired);
}

/// The collision stun suppresses lateral grip too, and is what counts itself
/// down - `Ship_ApplyLateralGrip` owns the decrement.
#[test]
fn the_collision_stun_suppresses_lateral_grip_and_counts_itself_down() {
    let handling = Handling {
        antigrav: Antigrav {
            grip_ground: 2.0,
            grip_air: 1.0,
            ..test_handling().antigrav
        },
        ..test_handling()
    };
    let world = flat_floor();
    let dt = 1.0 / 60.0;

    let mut stunned = ship_at(4.0);
    stunned.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
    stunned.grounded = 1.0;
    stunned.stun_timer = 0.5;

    let held = evaluate(
        &mut stunned,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        dt,
    );
    assert_eq!(held.lateral_grip, Vec3::ZERO);
    assert_eq!(stunned.stun_timer, 0.5 - dt);
}

/// The stun must actually expire, and stop at zero rather than going negative -
/// a negative timer would read as "not stunned" but is a trap for anything that
/// later tests the sign.
#[test]
fn the_collision_stun_stops_at_zero_and_grip_returns() {
    let handling = Handling {
        antigrav: Antigrav {
            grip_ground: 2.0,
            grip_air: 1.0,
            ..test_handling().antigrav
        },
        ..test_handling()
    };
    let world = flat_floor();
    let dt = 1.0 / 60.0;

    let mut ship = ship_at(4.0);
    ship.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
    ship.grounded = 1.0;
    ship.stun_timer = dt * 0.5;

    let during = evaluate(
        &mut ship,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        dt,
    );
    assert_eq!(during.lateral_grip, Vec3::ZERO);
    assert_eq!(ship.stun_timer, 0.0, "clamped rather than negative");

    ship.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
    ship.grounded = 1.0;
    let after = evaluate(
        &mut ship,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        dt,
    );
    assert_ne!(after.lateral_grip, Vec3::ZERO, "grip must come back");
}

/// The world force accumulator holds forces, not accelerations: gravity is the
/// only term carrying `mass`, so doubling the mass must not double the drag.
#[test]
fn only_gravity_scales_with_mass() {
    let handling = test_handling();
    let world = flat_floor();

    let mut light = ship_at(4.0);
    light.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
    light.grounded = 1.0;
    let light_eval = evaluate(
        &mut light,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );

    let mut heavy = ship_at(4.0);
    heavy.body.mass = 2.0;
    heavy.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
    heavy.grounded = 1.0;
    let heavy_eval = evaluate(
        &mut heavy,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );

    assert_eq!(heavy_eval.gravity, light_eval.gravity * 2.0);
    assert_eq!(heavy_eval.drag, light_eval.drag);
    assert_eq!(heavy_eval.rolling_resistance, light_eval.rolling_resistance);
}

/// The hover target is built from `ride_height`, which was previously believed
/// not to reach the force law at all.
#[test]
fn the_hover_target_follows_ride_height() {
    let mut handling = test_handling();
    let world = flat_floor();

    let mut low = ship_at(4.0);
    low.grounded = 1.0;
    let low_eval = evaluate(
        &mut low,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );

    handling.antigrav.ride_height *= 2.0;
    let mut high = ship_at(4.0);
    high.grounded = 1.0;
    let high_eval = evaluate(
        &mut high,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        1.0 / 60.0,
    );

    // Same height and the same everything else, but a higher target means more
    // lift.
    assert!(
        high_eval.hover.probes[0].force.y > low_eval.hover.probes[0].force.y,
        "{:?} was not above {:?}",
        high_eval.hover.probes[0].force,
        low_eval.hover.probes[0].force
    );
}

/// The end-to-end steering sign fix: holding right must swing the nose toward the ship's own right
/// axis over real ticks of `evaluate` and `integrate`. A unit assertion on `engine::steering`'s
/// return value would miss a sign error downstream in how the accumulators are drained onto the
/// body, the layer `docs/ghidra/functions/psp-pulse-usa/engine.md` left open until traced for this fix.
#[test]
fn holding_right_turns_the_ship_toward_its_own_right_axis() {
    let handling = Handling {
        turning: crate::params::Turning {
            amount: 0.02,
            gain: 400.0,
            falloff: 200.0,
        },
        ..Handling::ZERO
    };
    let world = flat_floor();
    // High enough above the floor that no hover probe ever makes contact, so
    // only steering is in play.
    let mut state = ship_at(1000.0);
    let controls = ShipControls {
        steer_x: 1.0,
        ..ShipControls::default()
    };
    let initial_right = state.body.right();

    let dt = 1.0 / 60.0;
    for _ in 0..30 {
        crate::integrate::step(
            &mut state,
            &controls,
            &handling,
            &Environment::default(),
            &world,
            dt,
        );
    }

    assert!(
        state.body.forward().dot(initial_right) > 0.0,
        "forward was {:?}, expected a component toward the initial right axis {initial_right:?}",
        state.body.forward()
    );
}

/// The airbrake twin of the test above, which would have caught Task #34's bug the day the
/// steering sign was fixed.
///
/// **`steer_x` is deliberately zero.** The only committed scenario for the airbrake yaw term,
/// `verification/scenarios/airbrake-asymmetric.inputs`, holds brake and steering on the same side
/// and steering is several times larger, so an inverted airbrake yaw still curves the run the
/// right way and shows only as a wrong rate. One brake alone separates them: nothing else can yaw
/// a ship flying level in a straight line, so the result's sign is this term's.
///
/// The direction is the original's: `angularLocal.y += fs * Airbrake.turn * (R - L) * 1e-3` on an
/// accumulator whose positive sense is nose-right (`w_game = -w_physics`), so `L > R` turns the
/// nose left, toward the braked side, as the game plays.
#[test]
fn braking_the_left_airbrake_alone_turns_the_ship_toward_its_own_left() {
    let handling = Handling {
        airbrake: crate::params::Airbrake {
            turn: 3.0,
            gain: 800.0,
            falloff: 400.0,
            ..crate::params::Airbrake::default()
        },
        ..Handling::ZERO
    };
    let world = flat_floor();
    // Far above the floor, so no hover probe, no contact and no alignment
    // torque: the airbrake yaw is the only thing that can rotate this ship.
    let mut state = ship_at(1000.0);
    // The three airbrake terms all carry `craft+0x2ec` as a factor, so a
    // stationary ship gets nothing at all.
    state.body.linear_velocity = state.body.forward() * 40.0;

    let controls = ShipControls {
        airbrake_left: 1.0,
        steer_x: 0.0,
        ..ShipControls::default()
    };
    let initial_right = state.body.right();

    let dt = 1.0 / 60.0;
    for _ in 0..30 {
        crate::integrate::step(
            &mut state,
            &controls,
            &handling,
            &Environment::default(),
            &world,
            dt,
        );
    }

    assert!(
        state.body.forward().dot(initial_right) < 0.0,
        "forward was {:?}; the left brake must swing it away from the initial \
         right axis {initial_right:?}, not toward it",
        state.body.forward()
    );
}

/// And the mirror, so that a term which somehow yawed left whatever it was
/// given could not pass the test above.
#[test]
fn braking_the_right_airbrake_alone_turns_the_ship_toward_its_own_right() {
    let handling = Handling {
        airbrake: crate::params::Airbrake {
            turn: 3.0,
            gain: 800.0,
            falloff: 400.0,
            ..crate::params::Airbrake::default()
        },
        ..Handling::ZERO
    };
    let world = flat_floor();
    let mut state = ship_at(1000.0);
    state.body.linear_velocity = state.body.forward() * 40.0;

    let controls = ShipControls {
        airbrake_right: 1.0,
        ..ShipControls::default()
    };
    let initial_right = state.body.right();

    let dt = 1.0 / 60.0;
    for _ in 0..30 {
        crate::integrate::step(
            &mut state,
            &controls,
            &handling,
            &Environment::default(),
            &world,
            dt,
        );
    }

    assert!(
        state.body.forward().dot(initial_right) > 0.0,
        "forward was {:?}, expected a component toward the initial right axis \
         {initial_right:?}",
        state.body.forward()
    );
}
