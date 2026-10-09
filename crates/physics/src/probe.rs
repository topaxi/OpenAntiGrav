//! A fixed scenario the determinism gate runs the real simulation over.
//!
//! The twin of [`oag_core::probe`]: the gate needs something both `tests/determinism.rs` and
//! `examples/physics_determinism_report.rs` can call, so the committed hashes and the report CI
//! prints describe the same run.
//!
//! Unlike `oag_core::probe` this is **not** a miniature simulation: it is [`crate::step`], the
//! entry point `oag_game`'s race loop uses, driven by a scripted input over a synthetic world,
//! so a change to the force law changes these hashes and is meant to.
//!
//! # No disc image
//!
//! The world is four quads built here, not a track off a CHD: `data/` is gitignored and absent
//! in CI, so a disc-backed run would never execute on Windows or macOS, where a portability bug
//! shows up. Determinism is a property of the arithmetic and branch structure, so a corridor is
//! enough.
//!
//! # What this does not cover
//!
//! A gate that silently covers half a crate still passes. Not exercised: [`crate::maglock`]
//! (needs a synthesised track sample in the [`Environment`]; a second scenario for whoever
//! needs it), [`crate::reset`], and the swept tunnelling path in [`crate::wall`], which fires
//! only when a ship crosses a wall within one frame.

use crate::collide::{Surface, TriangleSoup};
use crate::params::{
    Airbrake, Antigrav, Brakes, Dimensions, Engine, Physical, Pitch, SpeedupPads, Turning,
};
use crate::{
    Body, CollisionWorld, Environment, Handling, ShipControls, ShipState, Sideshift, step,
};
use oag_core::hash::StateHasher;
use oag_core::math::Vec3;

/// Our own fixed timestep. ADR-0007.
const TICK: f32 = 1.0 / 60.0;

/// Which committed input script a reference row runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    /// Thrust, both steering signs, both airbrakes, a brake and a sideshift, with the corridor
    /// walls close enough to be scraped.
    Corridor,
    /// The same corridor flown with sustained pitch input, so the orientation tumbles and the
    /// sub-step renormalisation is worked hard.
    Aerobatic,
}

/// What one run of the simulation is worth, for hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunResult {
    /// Hash of the final state only.
    pub final_hash: u64,
    /// Hash of the state at every tick, folded together: catches a mid-run divergence that
    /// cancels out, which a final-state hash would miss.
    pub trajectory_hash: u64,
}

/// A ground plane at `y = 0` and two walls, at `x = +-40`.
///
/// A straight corridor, deliberately: a bit-exact fingerprint of the force law, not a track.
/// The walls are close enough that a thrusting, steering ship reaches one well inside the
/// shorter run (a gate over a ship that touches nothing would not cover [`crate::wall`], the
/// most branch-dense arithmetic in the crate).
///
/// **Every triangle is wound so its normal already faces the corridor**, as the original's
/// `dot(boxCentre - sample, n) > 0` gate requires. The other winding would make the fixture
/// depend on whether the contact path flips normals, and the gate would move for a reason
/// unrelated to the force law.
#[must_use]
pub fn corridor() -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-2000.0, 0.0, -2000.0],
            [-2000.0, 0.0, 2000.0],
            [2000.0, 0.0, 2000.0],
            [2000.0, 0.0, -2000.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        Surface::Floor,
        0,
    ));
    // Normal -X: the corridor is on the near side.
    world.push(TriangleSoup::new(
        vec![
            [40.0, -50.0, -2000.0],
            [40.0, 50.0, -2000.0],
            [40.0, 50.0, 2000.0],
            [40.0, -50.0, 2000.0],
        ],
        vec![[0, 2, 1], [0, 3, 2]],
        Vec::new(),
        Surface::Wall,
        1,
    ));
    // Normal +X, for the same reason mirrored.
    world.push(TriangleSoup::new(
        vec![
            [-40.0, -50.0, -2000.0],
            [-40.0, 50.0, -2000.0],
            [-40.0, 50.0, 2000.0],
            [-40.0, -50.0, 2000.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        Surface::Wall,
        2,
    ));
    world
}

/// Arbitrary round numbers in the **already-scaled in-memory form** [`Handling`] holds. **Not
/// values from any ship** (`docs/architecture/adr/0006-no-copyrighted-content.md`).
///
/// The shape `tests/ship_dynamics.rs` uses, duplicated rather than shared: integration tests
/// cannot share code without a `tests/common/` module, and, the real reason, sharing would let
/// someone tuning that fixture silently invalidate the hashes committed here. Every field is
/// named with no `..` rest pattern, so a new [`Handling`] field is a compile error here, not a
/// silent zero the gate claims to cover.
#[must_use]
pub fn handling() -> Handling {
    Handling {
        airbrake: Airbrake {
            amount: 0.002,
            drag: 5.0,
            falloff: 400.0,
            gain: 800.0,
            turn: 3.0,
            slidegrip: 0.01,
            sideshift: 6.0,
        },
        antigrav: Antigrav {
            grip_air: 1.0,
            grip_ground: 2.0,
            landing_rebound: 2.0,
            rebound: 1.0,
            rebound_jump_time: 0.0,
            ride_height: 20.0,
        },
        brakes: Brakes {
            amount: -0.5,
            falloff: 200.0,
            gain: 400.0,
        },
        engine: Engine {
            accelcap: 20.0,
            amount: 0.4,
            falloff: 1.0,
            gain: 1.0,
            turbo: 0.0,
        },
        physical: Physical {
            flight_gravity: 10.0,
            mass: 1.0,
            normal_gravity: 10.0,
            track_gravity: 0.0,
        },
        pitch: Pitch {
            pitch_air: 0.02,
            pitch_ground: 0.005,
            pitch_damping: 3.0,
            antigrav_height_adjust: 0.0,
        },
        turning: Turning {
            amount: 0.01,
            falloff: 200.0,
            gain: 400.0,
        },
        dimensions: Dimensions {
            height: 1.0,
            length: 4.0,
            width: 2.0,
            shield: 100.0,
            weight_distribution: 0.5,
        },
        // Both scripts cross a pad twice ([`environment`]), so these are live: `time` of `0.5`
        // is 30 ticks of boost off a 6-tick crossing, exercising the ramp-down and flat tail.
        speedup_pads: SpeedupPads {
            amount: 50.0,
            time: 0.5,
        },
        // `<Special speedpad_jump>`, a round number, **not** the disc's.
        speedpad_jump: 0.2,
        // `<Special roll_cost/roll_speed/roll_turbotime>`: round numbers, **not** the disc's.
        // No probe script taps out a roll, so these hold their defaults every tick
        // (`crates/physics/tests/determinism.rs`).
        roll_cost: 8.0,
        roll_speed: 1.5,
        roll_turbotime: 0.5,
    }
}

/// The first tick of each of the two speed-pad crossings.
///
/// Two, because the branch to cover is **gated**: [`PAD_PLAIN`] is crossed with the pitch axis
/// on the wrong side of [`crate::engine::SPEEDPAD_JUMP_THRESHOLD`] and [`PAD_TILTED`] on the
/// right side, so a reference hash sees the tilt applied and not.
///
/// **Both sit where [`controls`] already holds the axis each needs, so no input was added to the
/// script**: `1500..1800` holds `steer_y = +0.8` and `1800..2100` `-0.8`, and the gate wants
/// negative (d-pad Up arrives as negative `steer_y`, `crate::engine::speedup_pad`). Moving them
/// would have meant editing the control script and moving the reference for two reasons at once.
/// The tilted window runs 200 ticks past the crossing, far longer than the boost's 30, which
/// exercises the tilt outliving the pad.
pub const PAD_PLAIN: u32 = 1600;
/// The tilted crossing. See [`PAD_PLAIN`].
pub const PAD_TILTED: u32 = 1900;
/// How many ticks each crossing lasts. A real one at racing speed is a handful; the boost
/// outlives it by design.
pub const PAD_TICKS: u32 = 6;

/// The world around the ship at a given tick: a speed pad, twice, and nothing else.
/// [`Environment`] is otherwise default. The pad's push direction is the corridor's axis, unit
/// length like a pad's matrix row.
#[must_use]
pub fn environment(tick: u32) -> Environment {
    let inside = |first: u32| (first..first + PAD_TICKS).contains(&tick);
    Environment {
        pad_hit: (inside(PAD_PLAIN) || inside(PAD_TILTED)).then_some(Vec3::NEG_Z),
        ..Environment::default()
    }
}

/// The input at a given tick, a pure function of the script and the tick.
///
/// Integer arithmetic, not a file, so a reference row cannot drift when a committed scenario is
/// edited, and with no `sin`: a transcendental in the *fixture* would make a libm difference
/// look like a physics bug. **Nothing here draws from the generator**: an RNG regression would
/// masquerade as physics, and `oag-core`'s gate covers the generator.
#[must_use]
pub fn controls(script: Script, tick: u32) -> ShipControls {
    let mut controls = ShipControls {
        thrust: 1.0,
        ..ShipControls::default()
    };

    match tick {
        // Settle on the cushion under thrust alone.
        0..300 => {}
        // Steer into one wall, then the other: both steering signs, lateral grip, two wall
        // contacts.
        300..600 => controls.steer_x = 0.6,
        600..900 => {
            controls.steer_x = -0.6;
            controls.airbrake_left = 1.0;
        }
        // Both airbrakes together is the only thing that raises the brake state.
        900..1200 => {
            controls.thrust = 0.5;
            controls.airbrake_left = 1.0;
            controls.airbrake_right = 1.0;
        }
        // One tick of sideshift, then its decay.
        1200 => controls.sideshift = Sideshift::Left,
        1201..1500 => {}
        // Pitch up to leave the floor, then land. These stretches are also where
        // [`environment`] puts the pad crossings, as they hold the pitch axis either side of
        // `crate::engine::SPEEDPAD_JUMP_THRESHOLD` (the tilt is gated on the **negative** side,
        // where the input layer puts d-pad Up).
        1500..1800 => controls.steer_y = 0.8,
        1800..2100 => {
            controls.thrust = 0.0;
            controls.steer_y = -0.8;
        }
        // A sustained slalom for the rest of the run.
        _ => {
            controls.steer_x = if (tick / 37).is_multiple_of(2) {
                0.8
            } else {
                -0.8
            };
        }
    }

    if script == Script::Aerobatic {
        controls.thrust = 1.0;
        controls.steer_y = match (tick / 23) % 3 {
            0 => 0.9,
            1 => -0.9,
            _ => 0.0,
        };
    }

    controls
}

/// Every field of a [`Body`], in declaration order. The destructure is the point: a new
/// [`Body`] field stops this file compiling instead of silently missing the hash.
pub fn hash_body(hasher: &mut StateHasher, body: &Body) {
    let Body {
        position,
        orientation,
        linear_velocity,
        angular_velocity,
        force,
        torque,
        mass,
        inertia,
    } = *body;
    hasher.write_vec3(position);
    hasher.write_quat(orientation);
    hasher.write_vec3(linear_velocity);
    hasher.write_vec3(angular_velocity);
    // The accumulators are left populated on exit from `step`: tick-boundary state, not scratch.
    hasher.write_vec3(force);
    hasher.write_vec3(torque);
    // Constant through a run and hashed anyway: a parameter-plumbing regression is what this
    // should catch.
    hasher.write_f32(mass);
    hasher.write_vec3(inertia);
}

/// Every field of a [`ShipState`], in declaration order. [`crate::forces::Evaluated`] and
/// [`crate::WallResponse`] are **not** hashed: derived reporting state, and hashing them would
/// couple the gate to fields documented "reporting only".
pub fn hash_state(hasher: &mut StateHasher, state: &ShipState) {
    let ShipState {
        body,
        airbrake_left,
        airbrake_right,
        thrust,
        brake,
        steer,
        // Not hashed (`ShipState::camera_lean_follower`): derived from the controls, read only by
        // the cockpit camera.
        camera_lean_follower: _,
        camera_lean: _,
        reverse_controls,
        stun_timer,
        wall_contact_prev,
        pending_impulse,
        slowdown_timer,
        grounded,
        grounded_prev,
        sideshift_timers,
        shift_tap_windows,
        shift_armed,
        shift_lockout,
        roll_taps,
        roll_tap_timer,
        roll_phase,
        roll_target,
        roll_armed,
        roll_payout_timer,
        roll_axis_zone,
        time_since_landing,
        // Not hashed: the race writes it from the countdown clock every tick
        // (`oag_raceplay::Race::tick`), a function of a value the world hash already carries,
        // and hashing it would move every committed reference for a field no probe sets.
        on_grid: _,
        // Not hashed, for `on_grid`'s reason.
        released: _,
        // Not hashed: the race writes it from the mode every tick.
        four_corner: _,
        // Not hashed, for `on_grid`'s reason.
        hover_cap: _,
        // Not hashed: the race writes it from the title every tick.
        hover_rig: _,
        launch,
        time_airborne,
        mag_lock_blend,
        pad_timer,
        pad_direction,
        mag_contact,
        shield,
        craft_state,
        state_timer,
        turbo_timer,
        shield_pickup_timer,
    } = *state;

    hash_body(hasher, &body);
    hasher.write_f32(airbrake_left);
    hasher.write_f32(airbrake_right);
    hasher.write_f32(thrust);
    hasher.write_f32(brake);
    hasher.write_f32(steer);
    hasher.write_f32(reverse_controls);
    hasher.write_f32(stun_timer);
    hasher.write_u8(u8::from(wall_contact_prev));
    // Zero through every probe script (no producer yet, `ShipState::pending_impulse`): a fixed
    // run of bytes, like `pad_direction` before a script crosses a pad.
    hasher.write_vec3(pending_impulse);
    hasher.write_f32(slowdown_timer);
    hasher.write_f32(grounded);
    hasher.write_f32(grounded_prev);
    hasher.write_f32(sideshift_timers[0]);
    hasher.write_f32(sideshift_timers[1]);
    // The gesture machines' state. `Script::Aerobatic` fires a sideshift through
    // `ShipControls::sideshift`, which bypasses the tap windows and armed latch, so those hold
    // defaults every tick; `shift_lockout` does not, as the direct request still refreshes it.
    hasher.write_f32(shift_tap_windows[0]);
    hasher.write_f32(shift_tap_windows[1]);
    hasher.write_u8(u8::from(shift_armed));
    hasher.write_f32(shift_lockout);
    // The barrel roll's gesture state. No probe script taps out a roll, so these contribute a
    // fixed run of bytes today and the first scenario that rolls moves all six together. The
    // axis leg needs checking, not assuming: `controls`'s slalom holds `steer_x` at `+-0.8`,
    // inside `crate::barrel_roll::AXIS_TAP_THRESHOLD`, so no crossing is recorded; an axis past
    // `0.9` would arm rolls.
    hasher.write_u8(roll_taps[0]);
    hasher.write_u8(roll_taps[1]);
    hasher.write_u8(roll_taps[2]);
    hasher.write_f32(roll_tap_timer);
    hasher.write_f32(roll_phase);
    hasher.write_f32(roll_target);
    if roll_armed {
        hasher.write_u8(1);
    }
    hasher.write_f32(roll_payout_timer);
    // `None` is `0`; the sides are the original's own `1`/`2`, as in `roll_taps`.
    hasher.write_u8(match roll_axis_zone {
        None => 0,
        Some(direction) => direction as u8,
    });
    hasher.write_f32(time_since_landing);
    // Written only once the launch boost has run, so a craft that never had the disc's
    // `<StartBoost>` (every probe script and committed reference) hashes as before the field
    // existed (`crate::launch::LaunchState`).
    if !launch.is_idle() {
        hasher.write_u8(1);
        hasher.write_f32(launch.multiplier);
        hasher.write_u8(launch.grade as u8);
        hasher.write_u8(u8::from(launch.latched));
        hasher.write_u32(launch.ticks);
    }
    hasher.write_f32(time_airborne);
    hasher.write_f32(mag_lock_blend);
    // Both hold defaults through every probe script (none crosses a pad): a fixed run of bytes,
    // which is why adding them moved the committed hashes with no behaviour change (history note
    // in `tests/determinism.rs`).
    hasher.write_f32(pad_timer);
    hasher.write_vec3(pad_direction);
    // A discriminant byte, or "no contact" hashes like a contact at the origin with a zero normal.
    match mag_contact {
        None => hasher.write_u8(0),
        Some(contact) => {
            hasher.write_u8(1);
            hasher.write_vec3(contact.point);
            hasher.write_vec3(contact.normal);
        }
    }
    // The energy pool reaches the hash though the force law does not read it, because a wall
    // writes it (`crate::damage::apply_contact`): a contact-solver change this gate would see
    // only as a velocity moves this field too. The scripts scrape a wall, so unlike `pad_timer`
    // this is not a fixed run of bytes.
    hasher.write_f32(shield);
    // A discriminant byte and the timer: defaults through every probe script (nothing empties a
    // pool), a fixed run of bytes, hashed because a destroyed craft the gate cannot see is worse.
    hasher.write_u8(match craft_state {
        crate::damage::CraftState::Racing => 0,
        crate::damage::CraftState::Destroyed => 1,
        crate::damage::CraftState::Eliminated => 2,
    });
    hasher.write_f32(state_timer);
    // Zero through every probe script: none fires a pickup and a corridor has no weapon pad.
    // Hashed because a turbo the gate cannot see would let a replay diverge silently.
    hasher.write_f32(turbo_timer);
    // And for a fired Shield, which no probe script can collect. Unlike `turbo_timer`, a running
    // shield makes `crate::damage::apply_contact` return early, so it can suppress a write to
    // `shield` above; the first scenario that fires one sees both fields move together.
    hasher.write_f32(shield_pickup_timer);
}

/// The ship a run starts with: on the corridor's centre line, resting on the cushion.
#[must_use]
pub fn start(handling: &Handling) -> ShipState {
    let mut state = ShipState {
        grounded: 1.0,
        grounded_prev: 1.0,
        ..ShipState::default()
    };
    state.body.mass = handling.physical.mass;
    state.body.position = Vec3::new(0.0, handling.antigrav.ride_height, 0.0);
    // A full pool (`crate::damage::reset`, as a race does on the grid): `ShipState::default()`
    // starts at zero and an empty pool cannot be depleted, so hashing it would be coverage in
    // name only.
    crate::damage::reset(&mut state, &handling.dimensions);
    state
}

/// Steps the real simulation for `ticks` and hashes what it did.
#[must_use]
pub fn run(script: Script, ticks: u32) -> RunResult {
    let handling = handling();
    let world = corridor();
    let mut state = start(&handling);

    let mut trajectory = StateHasher::new();
    for tick in 0..ticks {
        step(
            &mut state,
            &controls(script, tick),
            &handling,
            &environment(tick),
            &world,
            TICK,
        );
        hash_state(&mut trajectory, &state);
    }

    let mut final_state = StateHasher::new();
    final_state.write_u32(ticks);
    hash_state(&mut final_state, &state);

    RunResult {
        final_hash: final_state.finish(),
        trajectory_hash: trajectory.finish(),
    }
}
