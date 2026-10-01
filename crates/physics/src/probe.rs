//! A fixed scenario the determinism gate runs the real simulation over.
//!
//! The twin of [`oag_core::probe`], and here for the same reason: the gate needs
//! something to bite on that both `tests/determinism.rs` and
//! `examples/physics_determinism_report.rs` can call, so the test's committed hashes and
//! the report CI prints describe the same run rather than two runs that happen
//! to look alike.
//!
//! Unlike `oag_core::probe` this is **not** a miniature simulation. It is
//! [`crate::step`] - the same entry point `oag_game`'s race loop steps - driven
//! by a scripted input over a synthetic world, so a change to the force law
//! changes these hashes and is meant to.
//!
//! # No disc image
//!
//! The world here is four quads built in this file, not a track read off a CHD.
//! `data/` is gitignored and absent in CI, so a disc-backed determinism run
//! would never execute on Windows or macOS - which is where a portability bug
//! shows up. Determinism is a property of the arithmetic and the branch
//! structure rather than of the geometry, so a corridor is enough.
//!
//! # What this does not cover, so nobody assumes it does
//!
//! A gate that silently covers half a crate is worse than none, because it still
//! passes. Not exercised here: [`crate::maglock`] (it needs a track sample in the
//! [`Environment`], which would have to be synthesised - a second scenario for
//! whoever needs it), [`crate::reset`], and the swept tunnelling path in
//! [`crate::wall`], which only fires when a ship crosses a wall entirely within
//! one frame.

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
    /// Thrust, both steering signs, both airbrakes, a brake and a sideshift,
    /// with the corridor walls close enough to be scraped.
    Corridor,
    /// The same corridor flown with sustained pitch input, so the orientation
    /// tumbles and the sub-step renormalisation is worked hard.
    Aerobatic,
}

/// What one run of the simulation is worth, for hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunResult {
    /// Hash of the final state only.
    pub final_hash: u64,
    /// Hash of the state at every tick, folded together.
    ///
    /// Catches a divergence that happens mid-run and then cancels out, which a
    /// final-state hash alone would miss.
    pub trajectory_hash: u64,
}

/// A ground plane at `y = 0` and two walls, at `x = +-40`.
///
/// A straight corridor, deliberately: this is a bit-exact fingerprint of the
/// force law, not a track. The walls are close enough that a thrusting,
/// steering ship reaches one well inside the shorter run - a determinism gate
/// over a ship that never touches anything would not cover
/// [`crate::wall`] at all, and the contact solve is the most branch-dense
/// arithmetic in the crate.
///
/// **Every triangle is wound so its normal already faces the corridor**, which
/// is what the original's own `dot(boxCentre - sample, n) > 0` gate requires.
/// Winding them the other way would make this fixture's behaviour depend on
/// whether the contact path flips normals, and the gate would then move for a
/// reason that has nothing to do with the force law.
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

/// Arbitrary round numbers, in the **already-scaled in-memory form**
/// [`Handling`] holds. **Not values from any ship** - see
/// `docs/architecture/adr/0006-no-copyrighted-content.md`.
///
/// The same shape `tests/ship_dynamics.rs` uses, deliberately duplicated rather
/// than shared. Two reasons, and the second is the real one: integration tests
/// cannot share code without a `tests/common/` module, and sharing would let
/// somebody tuning that file's fixture silently invalidate the hashes committed
/// here.
///
/// Every field is named, with no `..` rest pattern anywhere, so a new
/// [`Handling`] field is a compile error in this file rather than a silent zero
/// that the gate then claims to cover.
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
        // Both scripts cross a pad twice - see [`environment`] - so these are
        // live rather than aspirational. `time` of `0.5` is 30 ticks of boost
        // off a 6-tick crossing, which is long enough that the ramp-down and the
        // flat tail are both exercised.
        speedup_pads: SpeedupPads {
            amount: 50.0,
            time: 0.5,
        },
        // `<Special speedpad_jump>`, the tilt toward the hull's up axis. A round
        // number, **not** the disc's - see the module docs.
        speedpad_jump: 0.2,
        // `<Special roll_cost/roll_speed/roll_turbotime>`. Round numbers, **not**
        // the disc's, same as `speedpad_jump` above. No probe script taps out a
        // barrel roll, so these three hold their defaults for every tick of
        // every run here - see `crates/physics/tests/determinism.rs`.
        roll_cost: 8.0,
        roll_speed: 1.5,
        roll_turbotime: 0.5,
    }
}

/// The first tick of each of the two speed-pad crossings.
///
/// Two crossings and not one, because the branch that needed covering is
/// **gated**: [`PAD_PLAIN`] is crossed with the pitch axis on the wrong side of
/// [`crate::engine::SPEEDPAD_JUMP_THRESHOLD`] and [`PAD_TILTED`] with it on the
/// right side, so a reference hash sees the tilt both applied and not.
///
/// **Both are placed inside stretches where [`controls`] already holds the axis
/// the way each needs, so this change adds no input to the script at all.**
/// `1500..1800` holds `steer_y = +0.8` and `1800..2100` holds `-0.8`, and the
/// gate wants negative - d-pad Up reaches the simulation as a *negative*
/// `steer_y`, see `crate::engine::speedup_pad`. Placing them anywhere else would
/// have meant editing the control script too, and then the reference would have
/// moved for two reasons at once with no way to tell them apart.
///
/// The tilted window also runs on for 200 ticks after the crossing, far longer
/// than the boost's 30, which is what exercises the tilt outliving the pad.
pub const PAD_PLAIN: u32 = 1600;
/// The tilted crossing. See [`PAD_PLAIN`].
pub const PAD_TILTED: u32 = 1900;
/// How many ticks each crossing lasts. A real crossing at racing speed is a
/// handful of ticks; the boost outlives it by design.
pub const PAD_TICKS: u32 = 6;

/// The world around the ship at a given tick: a speed pad, twice, and nothing
/// else.
///
/// [`Environment`] is otherwise all defaults, which is what it was before this
/// existed. The pad's push direction is the corridor's own axis, unit length the
/// way a pad's matrix row is.
#[must_use]
pub fn environment(tick: u32) -> Environment {
    let inside = |first: u32| (first..first + PAD_TICKS).contains(&tick);
    Environment {
        pad_hit: (inside(PAD_PLAIN) || inside(PAD_TILTED)).then_some(Vec3::NEG_Z),
        ..Environment::default()
    }
}

/// The input at a given tick, as a pure function of the script and the tick.
///
/// Written as integer arithmetic rather than read from a file, so a reference
/// row cannot drift when a committed scenario is edited - and with no `sin`
/// anywhere, because a transcendental in the *fixture* would make a libm
/// difference look like a physics bug.
///
/// **Nothing here draws from the generator.** The point is to hash the physics;
/// an RNG regression would otherwise masquerade as one, and `oag-core`'s gate
/// already covers the generator on its own.
#[must_use]
pub fn controls(script: Script, tick: u32) -> ShipControls {
    let mut controls = ShipControls {
        thrust: 1.0,
        ..ShipControls::default()
    };

    match tick {
        // Settle on the cushion under thrust alone.
        0..300 => {}
        // Steer into one wall, then the other: both signs of the steering
        // torque, the lateral grip and two wall contacts.
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
        // Pitch up hard enough to leave the floor, then land again. These two
        // stretches are also where [`environment`] puts the two speed-pad
        // crossings, because they already hold the pitch axis on either side of
        // `crate::engine::SPEEDPAD_JUMP_THRESHOLD` - the tilt branch is gated on
        // the **negative** side, which is where the input layer puts d-pad Up.
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

/// Every field of a [`Body`], in declaration order.
///
/// The destructure is the point: adding a field to [`Body`] stops this file
/// compiling instead of silently leaving the new field out of the hash.
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
    // The accumulators are left populated on exit from `step`, so they are
    // genuine tick-boundary state rather than scratch.
    hasher.write_vec3(force);
    hasher.write_vec3(torque);
    // Constant through a run, and hashed anyway: a parameter-plumbing
    // regression is exactly what this should catch.
    hasher.write_f32(mass);
    hasher.write_vec3(inertia);
}

/// Every field of a [`ShipState`], in declaration order.
///
/// [`crate::forces::Evaluated`] and [`crate::WallResponse`] are
/// deliberately **not** hashed: they are derived reporting state, and including
/// them would couple the gate to fields whose own docs say "reporting only".
pub fn hash_state(hasher: &mut StateHasher, state: &ShipState) {
    let ShipState {
        body,
        airbrake_left,
        airbrake_right,
        thrust,
        brake,
        steer,
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
        roll_payout_timer,
        roll_axis_zone,
        time_since_landing,
        // Deliberately not hashed: the race writes it from the countdown clock
        // every tick (`oag_game::race::Race::tick`), so it is a function of a
        // value the world hash already carries, and writing it would move every
        // committed reference for a field no probe script ever sets.
        on_grid: _,
        // Not hashed, for `on_grid`'s reason: the race writes it from the countdown clock.
        released: _,
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
    // Zero through every probe script - no producer exists yet to write it, see
    // `ShipState::pending_impulse`'s own doc - the same fixed-run-of-bytes shape
    // as `pad_direction` below before a probe script crosses a pad.
    hasher.write_vec3(pending_impulse);
    hasher.write_f32(slowdown_timer);
    hasher.write_f32(grounded);
    hasher.write_f32(grounded_prev);
    hasher.write_f32(sideshift_timers[0]);
    hasher.write_f32(sideshift_timers[1]);
    // The gesture machines' own state. `Script::Aerobatic` fires a sideshift
    // through `ShipControls::sideshift`, which bypasses the tap windows and the
    // armed latch, so those two hold their defaults for every tick of every
    // script; `shift_lockout` does not, because the direct request still
    // refreshes it once the timer runs.
    hasher.write_f32(shift_tap_windows[0]);
    hasher.write_f32(shift_tap_windows[1]);
    hasher.write_u8(u8::from(shift_armed));
    hasher.write_f32(shift_lockout);
    // The barrel roll's own gesture state. None of the probe scripts taps out
    // a roll, so these six contribute a fixed run of bytes per tick today -
    // the same shape `pad_timer` had before a scenario crossed a pad - and the
    // first scenario that does one will see all six move together. The axis
    // leg is the reason "none of them taps one out" needs checking rather than
    // assuming: `controls`'s slalom holds `steer_x` at `+-0.8`, inside
    // `crate::barrel_roll::AXIS_TAP_THRESHOLD`, so no crossing is ever
    // recorded. A script that pushed the axis past `0.9` would arm rolls.
    hasher.write_u8(roll_taps[0]);
    hasher.write_u8(roll_taps[1]);
    hasher.write_u8(roll_taps[2]);
    hasher.write_f32(roll_tap_timer);
    hasher.write_f32(roll_phase);
    hasher.write_f32(roll_target);
    hasher.write_f32(roll_payout_timer);
    // `None` is `0`, and the two sides are the original's own `1`/`2`, the same
    // encoding `roll_taps` above carries.
    hasher.write_u8(match roll_axis_zone {
        None => 0,
        Some(direction) => direction as u8,
    });
    hasher.write_f32(time_since_landing);
    // Written only once the launch boost has run, so a craft that never had the
    // disc's `<StartBoost>` (every probe script, every committed reference)
    // hashes as it did before the field existed. Idle is `0` and nothing more;
    // see `crate::launch::LaunchState`.
    if !launch.is_idle() {
        hasher.write_u8(1);
        hasher.write_f32(launch.multiplier);
        hasher.write_u8(launch.grade as u8);
        hasher.write_u8(u8::from(launch.latched));
        hasher.write_u32(launch.ticks);
    }
    hasher.write_f32(time_airborne);
    hasher.write_f32(mag_lock_blend);
    // Both stay at their defaults through every probe script - none crosses a
    // pad - so these two contribute a fixed run of bytes per tick and nothing
    // else. That is why adding them moved the committed hashes without any
    // behaviour changing; see the history note in `tests/determinism.rs`.
    hasher.write_f32(pad_timer);
    hasher.write_vec3(pad_direction);
    // A discriminant byte, or "no contact" hashes the same as a contact at the
    // origin with a zero normal.
    match mag_contact {
        None => hasher.write_u8(0),
        Some(contact) => {
            hasher.write_u8(1);
            hasher.write_vec3(contact.point);
            hasher.write_vec3(contact.normal);
        }
    }
    // The energy pool. It reaches the hash even though nothing in the force law
    // reads it, because a wall writes it: `crate::damage::apply_contact` turns
    // this frame's contact impulses into a subtraction, so a change to the
    // contact solver that this gate would otherwise see only as a velocity moves
    // this field too. The probe scripts scrape a wall, so unlike `pad_timer`
    // this is not a fixed run of bytes.
    hasher.write_f32(shield);
    // A discriminant byte and the timer. Both hold their defaults through every
    // probe script - nothing here ever empties a pool - so they contribute a
    // fixed run of bytes per tick, the same shape as `pad_timer` before the
    // scenario crossed a pad. Hashed anyway, because the alternative is a
    // destroyed craft that the gate cannot see.
    hasher.write_u8(match craft_state {
        crate::damage::CraftState::Racing => 0,
        crate::damage::CraftState::Destroyed => 1,
        crate::damage::CraftState::Eliminated => 2,
    });
    hasher.write_f32(state_timer);
    // Zero through every probe script for the same reason as the two above: no
    // probe fires a pickup, and no probe *can* - there is no weapon pad in a
    // corridor. Another fixed run of bytes, hashed because a turbo the gate
    // could not see would let a replay of a single race diverge silently.
    hasher.write_f32(turbo_timer);
    // And the same again for a fired Shield: no probe script can collect one.
    // It differs from `turbo_timer` in one way that matters to this gate - a
    // running shield makes `crate::damage::apply_contact` return early, so it
    // does not merely add bytes, it can suppress a write to `shield` above.
    // Zero through every script here, but the first scenario that fires one will
    // see both fields move together.
    hasher.write_f32(shield_pickup_timer);
}

/// The ship a run starts with: on the corridor's centre line, resting on the
/// cushion rather than dropped onto it.
#[must_use]
pub fn start(handling: &Handling) -> ShipState {
    let mut state = ShipState {
        grounded: 1.0,
        grounded_prev: 1.0,
        ..ShipState::default()
    };
    state.body.mass = handling.physical.mass;
    state.body.position = Vec3::new(0.0, handling.antigrav.ride_height, 0.0);
    // A full pool, because `ShipState::default()` starts at zero and a pool that
    // is already empty cannot be depleted - the corridor script scrapes both
    // walls, and hashing a field that never moves is coverage in name only. This
    // is `crate::damage::reset`, which is what a race does when a ship takes the
    // grid.
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
