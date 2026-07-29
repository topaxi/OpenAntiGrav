//! A fixed scenario the determinism gate runs the real simulation over.
//!
//! The twin of [`oag_core::probe`], and here for the same reason: the gate needs
//! something to bite on that both `tests/determinism.rs` and
//! `examples/determinism_report.rs` can call, so the test's committed hashes and
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
use crate::params::{Airbrake, Antigrav, Brakes, Dimensions, Engine, Physical, Pitch, Turning};
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
            easyshield: 120.0,
            weight_distribution: 0.5,
        },
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
        // Pitch up hard enough to leave the floor, then land again.
        1500..1800 => controls.steer_y = 0.8,
        1800..2100 => {
            controls.thrust = 0.0;
            controls.steer_y = -0.8;
        }
        // A sustained slalom for the rest of the run.
        _ => {
            controls.steer_x = if (tick / 37) % 2 == 0 { 0.8 } else { -0.8 };
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
        leap_timer,
        grounded,
        grounded_prev,
        sideshift_timers,
        time_since_landing,
        mag_lock_blend,
        mag_contact,
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
    hasher.write_f32(leap_timer);
    hasher.write_f32(grounded);
    hasher.write_f32(grounded_prev);
    hasher.write_f32(sideshift_timers[0]);
    hasher.write_f32(sideshift_timers[1]);
    hasher.write_f32(time_since_landing);
    hasher.write_f32(mag_lock_blend);
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
    state
}

/// Steps the real simulation for `ticks` and hashes what it did.
#[must_use]
pub fn run(script: Script, ticks: u32) -> RunResult {
    let handling = handling();
    let world = corridor();
    let environment = Environment::default();
    let mut state = start(&handling);

    let mut trajectory = StateHasher::new();
    for tick in 0..ticks {
        step(
            &mut state,
            &controls(script, tick),
            &handling,
            &environment,
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
