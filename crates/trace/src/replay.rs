//! The simulated side: run our own physics over a recording's scenario and emit
//! a trace in the same shape.
//!
//! The run is seeded from the recording's **first row** - position, basis,
//! velocity and the five control states - so the two start from the same state by
//! construction and tick 0 is an initial condition rather than a measurement.
//! Everything after that is ours.
//!
//! # Sampled before the step, not after
//!
//! `scripts/psp-trace.py` breaks on `Ship_UpdateCraft`'s entry, so a recorded row
//! is the craft as the update *found* it. This module emits its row before
//! stepping for the same reason. Emitting after the step instead would compare
//! every field one tick out of phase, which looks exactly like a small systematic
//! lag in the physics and is not one.
//!
//! # What cannot be replayed, and is not faked
//!
//! - **Angular velocity is in the capture only if the capture is a recent one.**
//!   A trace with the `avel_*` columns seeds the body's rotation from its first
//!   row like every other initial condition; one taken before those columns
//!   existed starts the ship's rotation at rest, whatever the original was doing,
//!   and on a capture that begins in a corner that alone will diverge the
//!   orientation. Nothing is faked in for the older case - see
//!   [`crate::trace::REQUIRED_COLUMNS`] - so a capture that begins in a corner
//!   still wants re-taking rather than a zero.
//!   **Which reading of the column is right is open**, so it is an option:
//!   [`crate::trace::AngularReading`].
//! - **Pitch is not in the capture** either: the craft's control block holds
//!   `steer` but no second axis, so [`oag_physics::ShipControls::steer_y`] is
//!   always zero here.
//! - **The recorded control states are already ramped.** `steer`, `brake` and both
//!   airbrakes are the original's *output* states, not the stick positions that
//!   produced them, so [`Inputs::FromTrace`] feeds a ramped signal into our own
//!   ramp and will lag on any tick where the input is moving. [`Inputs::Held`]
//!   is the honest mode for a capture taken with `psp-trace.py --hold`, where the
//!   input is constant and known, and [`Inputs::Scripted`] is the honest mode for
//!   one taken with `psp-trace.py --script`, where it is varying and *still*
//!   known, because both sides read the same committed file.
//!   A second, smaller reason to prefer either: a recorded row is read at the
//!   *entry* of `Ship_UpdateCraft`, so its control states are what tick n-1
//!   wrote, and feeding them as tick n's input lags our ramps by one frame on top
//!   of the double ramp. Under an authored input neither effect exists.
//! - **`time_since_landing` and the leap timer are not in the capture**, so a run
//!   starts outside the landing window with no leap in progress.

use std::num::NonZeroUsize;

use oag_core::math::{Mat3, Quat, Vec3};
use oag_gameplay::input::{Input, button};
use oag_gameplay::{InputSnapshot, Ship, World, ship_controls};
use oag_physics::controls::CONTROL_RANGE;
use oag_physics::{Environment, Handling, Raycaster, ShipState};

use crate::trace::{AngularReading, Frame, Trace};

/// The world's generator seed.
///
/// Fixed and arbitrary: nothing in the recovered force law draws from the
/// generator. The same value `oag_game::race::SEED` uses, written again rather
/// than imported because nothing may depend on the composition root.
pub const SEED: u64 = 1;

/// How the recorded basis maps onto the body's axes.
///
/// The capture names its three rows `right`, `up` and `fwd`, and row 0 was
/// **measured to be the ship's left**: holding left gives `steer = -96` and
/// `+1.51 rad/s` about row 1, holding right the mirror of that, 199/199
/// consistent, so forward rotates toward row 0 on a left turn. Confidence 84.
///
/// # Why the alternative reading flips two rows, not one
///
/// The recorded basis is positively oriented - `cross(row0, row1) = row2` on
/// 200/200 ticks - while [`oag_physics::Body`]'s is not: `right x up = -forward`,
/// because forward is `-Z`. So mapping one onto the other takes an **odd** number
/// of sign flips, and a reading that flipped only row 0 would describe a
/// reflection rather than a rotation. Row 0 and row 2 are both named from use
/// sites that fix an axis but not its sign, so the coherent alternative to "row 0
/// is left" is "row 0 is right and row 2 points backwards", which is what
/// [`Self::RightUpBack`] is.
///
/// It is a switch rather than a constant because this is exactly the kind of
/// finding a trace comparison exists to settle: run a comparison both ways and
/// see which keeps the orientation inside `1e-4` rad.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Basis {
    /// Rows are left, up and forward. The measured reading.
    #[default]
    LeftUpForward,
    /// Rows are right, up and backward, as the capture's column names say -
    /// except that row 2 must then be the tail, not the nose.
    RightUpBack,
}

impl Basis {
    /// The body's right, up and forward axes, given a recorded basis.
    #[must_use]
    pub fn to_body(self, row0: Vec3, row1: Vec3, row2: Vec3) -> (Vec3, Vec3, Vec3) {
        match self {
            Self::LeftUpForward => (-row0, row1, row2),
            Self::RightUpBack => (row0, row1, -row2),
        }
    }

    /// The recorded basis, given the body's axes. The inverse of
    /// [`Self::to_body`], and an involution in both readings.
    #[must_use]
    pub fn to_rows(self, right: Vec3, up: Vec3, forward: Vec3) -> (Vec3, Vec3, Vec3) {
        self.to_body(right, up, forward)
    }
}

/// Where each tick's delta comes from.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum DeltaSource {
    /// The recording's own `dt`, which is variable and averages `1/59.94`.
    ///
    /// The default, and the only one that makes a tick-for-tick comparison
    /// meaningful: the original integrates the frame it actually got.
    #[default]
    Trace,
    /// A fixed delta, for asking what our own 60 Hz timestep costs. ADR-0007.
    Fixed(f32),
}

/// One fixed input, held for the whole run.
///
/// The same five quantities a scripted tick carries, and the same type: a held
/// input is a one-state script, and saying so in the type system is what stops
/// the two modes from drifting apart.
pub type Held = crate::script::State;

/// How the run is driven.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Inputs {
    /// Rebuild each tick's input from that tick's recorded control states.
    ///
    /// The general mode, and the approximate one: see the module docs on ramping.
    #[default]
    FromTrace,
    /// Hold one input for the whole run, matching `psp-trace.py --hold`.
    Held(Held),
    /// Drive each tick from an authored [`Script`](crate::script::Script),
    /// matching `psp-trace.py --script`.
    ///
    /// The only mode where a *varying* input is known rather than inferred, and
    /// the only one that is reusable: the same committed file drove the capture
    /// this run is being compared against. A script shorter than the recording
    /// holds its last state - see [`Script::at`](crate::script::Script::at).
    Scripted(Vec<Held>),
}

/// How to replay a recording.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Options {
    /// Where the input comes from.
    pub inputs: Inputs,
    /// Where the delta comes from.
    pub dt: DeltaSource,
    /// How the recorded basis maps onto the body's axes.
    pub basis: Basis,
    /// What the recorded angular-velocity column means, which decides both how
    /// the body's rotation is seeded and how ours is written back out.
    pub angular: AngularReading,
    /// Put the ship back on the recording's own state every this many ticks.
    ///
    /// `None` - the default - seeds tick 0 and never again, which is what every
    /// run did before this existed and is the right thing for a short scenario.
    ///
    /// # Why a long comparison needs it
    ///
    /// The original integrates the frame duration it actually measured
    /// ([ADR-0007](../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md)),
    /// and those durations follow host load. Two captures of the *same script
    /// from the original itself* diverge by 100 units at tick 495, because `dt`
    /// agrees on about 1 % of ticks. So a single-seeded run of three thousand
    /// ticks measures the divergence of two chaotic trajectories, and no
    /// implementation - not even a byte-exact one - can pass it.
    ///
    /// Re-seeding turns the same recording into a sequence of independent short
    /// comparisons: each window asks "given exactly where the original was,
    /// where does our force law put the ship over the next `n` ticks?", which is
    /// the question the force law can actually answer. The error stops
    /// compounding, so a max error is a statement about the *worst window*
    /// rather than about how long the run happened to be.
    ///
    /// It is deliberately not the default. A reseeded run cannot tell you the
    /// one thing a single-seeded run can - how long we track the original before
    /// coming apart - and reporting one as the other would be the exact class of
    /// silent misreading this harness exists to prevent. Both numbers are worth
    /// having; they are not the same number.
    pub reseed: Option<NonZeroUsize>,
}

/// The ship state a recording's first row describes.
///
/// Mass is copied from the parameter set into the body because the force law
/// reads `handling.physical.mass` and the integrator reads `body.mass`: they are
/// one quantity stored twice and keeping them equal is the integrating layer's
/// job, which here is this function. The inertia is left at its default, which is
/// the same known gap `oag_game::race` records - nothing in the ship data says
/// how the original builds a tensor.
///
/// The body's rotation is seeded too, when the recording carries one: a capture
/// without the `avel_*` columns leaves it at rest, which is what every run did
/// before those columns existed.
#[must_use]
pub fn initial_state(
    frame: &Frame,
    handling: &Handling,
    basis: Basis,
    angular: AngularReading,
) -> ShipState {
    let mut state = ShipState {
        thrust: frame.throttle,
        brake: frame.brake,
        steer: frame.steer,
        airbrake_left: frame.airbrake_left,
        airbrake_right: frame.airbrake_right,
        grounded: frame.grounded,
        grounded_prev: frame.grounded,
        ..ShipState::default()
    };
    state.body.mass = handling.physical.mass;
    state.body.position = frame.position;
    state.body.linear_velocity = frame.velocity;
    state.body.orientation = orientation_of(frame, basis);
    // Seed the rotation from whichever angular column the capture has, and take
    // the *rate* one first: `body+0x150` is the angular velocity by definition,
    // while `body+0x160` is `I * omega` and seeding a body's angular velocity
    // from it directly - which this did until the tensor was recovered - starts
    // the run spinning between 15.6 and 21.6 times too fast on every axis.
    if let Some(recorded) = frame.angular_rate {
        state.body.angular_velocity = angular.to_world(recorded, frame.rows());
    } else if let Some(recorded) = frame.angular_velocity {
        let local_momentum =
            state.body.orientation.inverse() * angular.to_world(recorded, frame.rows());
        let inertia = state.body.inertia;
        let local_rate = Vec3::new(
            divide_or_zero(local_momentum.x, inertia.x),
            divide_or_zero(local_momentum.y, inertia.y),
            divide_or_zero(local_momentum.z, inertia.z),
        );
        state.body.angular_velocity = state.body.orientation * local_rate;
    }
    state
}

/// Component-wise division that treats a zero denominator as a zero result,
/// matching `oag_physics::integrate`'s own handling of a degenerate tensor.
fn divide_or_zero(numerator: f32, denominator: f32) -> f32 {
    if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}

/// The orientation a recorded basis describes.
///
/// [`oag_physics::Body`] holds an orientation, not a matrix, and its axes are
/// `X = right`, `Y = up`, `-Z = forward`; so the rotation's third column is the
/// negated forward, not the forward. Getting that wrong yaws the whole run by
/// half a turn on tick 0, which is loud rather than subtle, and the test below is
/// what keeps it that way.
///
/// The columns are re-orthonormalised on the way in - the capture writes seven
/// significant digits, so a recorded basis arrives about `1e-6` off orthonormal
/// and `Quat::from_mat3` wants a rotation.
#[must_use]
pub fn orientation_of(frame: &Frame, basis: Basis) -> Quat {
    let (right, up, forward) = basis.to_body(frame.row0, frame.up, frame.forward);
    let (right, up, forward) = (
        right.normalize_or_zero(),
        up.normalize_or_zero(),
        forward.normalize_or_zero(),
    );
    if right == Vec3::ZERO || up == Vec3::ZERO || forward == Vec3::ZERO {
        return Quat::IDENTITY;
    }
    // Gram-Schmidt off the forward axis, which is the one the ship is aimed
    // along: rounding is shared out rather than concentrated in whichever column
    // happened to be last.
    let z = -forward;
    let x = (right - z * right.dot(z)).normalize_or_zero();
    if x == Vec3::ZERO {
        return Quat::IDENTITY;
    }
    let y = z.cross(x);
    debug_assert!(
        (y - up).length() < 1e-2,
        "the recorded basis is not the one this reading describes"
    );
    Quat::from_mat3(&Mat3::from_cols(x, y, z)).normalize()
}

/// Runs our simulation over a recording's scenario and returns a trace of it.
///
/// One ship in slot 0 of a [`World`], stepped through
/// [`oag_physics::step`] exactly as `oag_game`'s race loop steps it - the input
/// goes through [`oag_gameplay::ship_controls`], so "cross is thrust" is read out
/// of the one place that says so rather than restated here.
///
/// The returned trace has one row per row of the input, in the same columns, and
/// its tick numbers continue the recording's.
#[must_use]
pub fn replay<R: Raycaster + ?Sized>(
    trace: &Trace,
    handling: &Handling,
    environment: &Environment,
    raycaster: &R,
    options: &Options,
) -> Trace {
    let Some(first) = trace.frames.first() else {
        return Trace::default();
    };

    let mut world = World::new(SEED);
    world.ships[0] = Ship {
        physics: initial_state(first, handling, options.basis, options.angular),
        handling: *handling,
        segment: 0,
        active: true,
    };
    world.ship_count = 1;

    let mut buttons = Input::new();
    // Seeded from the recording so tick 0 agrees by construction; after that it
    // is the previous tick's *forward-projected* speed, `dot(velocity, forward)`,
    // which is what the original's own field was measured to hold - to a max of
    // 9.7e-6 over 200 ticks, the capture's print rounding at that speed. Not the
    // velocity's magnitude: the two only separate once the ship is sliding, where
    // they differ by the cosine of the slip angle.
    let mut speed_cached = first.speed_cached;
    let mut out = Trace {
        frames: Vec::with_capacity(trace.len()),
    };

    for (index, recorded) in trace.frames.iter().enumerate() {
        // Put the ship back on the recording, when asked. Deliberately *before*
        // the row is emitted, so a window's first row is the seed itself and
        // reads as an initial condition exactly the way tick 0 does - the same
        // property that makes tick 0 uninformative rather than a false match.
        // `initial_state` is reused rather than reimplemented: a second seeding
        // path would be one more thing to keep in agreement, and the whole point
        // of a window is that it starts where tick 0 starts.
        if let Some(every) = options.reseed
            && index > 0
            && index % every.get() == 0
        {
            world.ships[0].physics =
                initial_state(recorded, handling, options.basis, options.angular);
            speed_cached = recorded.speed_cached;
        }

        let dt = match options.dt {
            DeltaSource::Trace => recorded.dt,
            DeltaSource::Fixed(dt) => dt,
        };
        let state = &world.ships[0].physics;
        out.frames.push(frame_of(
            state,
            first.tick + index as u64,
            dt,
            speed_cached,
            options,
        ));
        speed_cached = state.body.linear_velocity.dot(state.body.forward());

        let snapshot = snapshot_for(recorded, &mut buttons, &options.inputs, index);
        let controls = ship_controls(&snapshot);
        let ship = &mut world.ships[0];
        oag_physics::step(
            &mut ship.physics,
            &controls,
            &ship.handling,
            environment,
            raycaster,
            dt,
        );
        world.tick += 1;
    }

    out
}

/// How to drive a run that has no recording behind it.
///
/// [`replay`] takes its length, its per-tick delta and its initial condition from
/// a capture. A scenario run has none of those: it is an input script, a track
/// and a start line, which is what `oag-trace drive` and therefore
/// `just scripted-sim` are. So the three come from here instead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveOptions {
    /// How many ticks to step. A script shorter than this holds its last state,
    /// exactly as [`Inputs::Scripted`] does.
    pub ticks: usize,
    /// The timestep, in seconds. Ours is fixed at 60 Hz - ADR-0007 - and there
    /// is no recording here to take a variable delta from, so unlike [`replay`]
    /// this is not a choice between two sources.
    pub dt: f32,
    /// How our axes are written into the recorded basis columns.
    pub basis: Basis,
    /// Which reading the angular-velocity column is written in.
    pub angular: AngularReading,
}

impl Default for DriveOptions {
    fn default() -> Self {
        Self {
            ticks: 0,
            dt: 1.0 / 60.0,
            basis: Basis::default(),
            angular: AngularReading::default(),
        }
    }
}

/// Runs an input script through our physics from a given start state.
///
/// The same stepping loop [`replay`] uses - same [`World`], same
/// [`oag_gameplay::ship_controls`], same sampled-before-the-step convention - with
/// the capture taken out of it. What comes back is a trace in the capture's own
/// columns, so everything downstream (`oag-trace show`, `oag-trace compare`, the
/// CSV writer) works on it unchanged, and a scenario run can be diffed against a
/// capture of the same scenario the day one exists.
///
/// `initial` is a whole [`ShipState`] rather than a pose because that is the only
/// honest signature: what a ship starts with includes its mass, its inertia and
/// which way its suspension is loaded, and a function that took a position and
/// invented the rest would be hiding the interesting half.
#[must_use]
pub fn drive<R: Raycaster + ?Sized>(
    initial: ShipState,
    handling: &Handling,
    environment: &Environment,
    raycaster: &R,
    script: &[Held],
    options: &DriveOptions,
) -> Trace {
    let mut world = World::new(SEED);
    world.ships[0] = Ship {
        physics: initial,
        handling: *handling,
        segment: 0,
        active: true,
    };
    world.ship_count = 1;

    let mut buttons = Input::new();
    let mut speed_cached = {
        let body = &world.ships[0].physics.body;
        body.linear_velocity.dot(body.forward())
    };
    let mut out = Trace {
        frames: Vec::with_capacity(options.ticks),
    };
    let frame_options = Options {
        inputs: Inputs::Scripted(script.to_vec()),
        dt: DeltaSource::Fixed(options.dt),
        basis: options.basis,
        angular: options.angular,
        // A scenario run has no recording to be put back onto.
        reseed: None,
    };

    for index in 0..options.ticks {
        let state = &world.ships[0].physics;
        out.frames.push(frame_of(
            state,
            index as u64,
            options.dt,
            speed_cached,
            &frame_options,
        ));
        speed_cached = state.body.linear_velocity.dot(state.body.forward());

        // `Frame::default()` is only ever read by `Inputs::FromTrace`, which this
        // is not: the script is the whole of the input here.
        let snapshot = snapshot_for(
            &Frame::default(),
            &mut buttons,
            &frame_options.inputs,
            index,
        );
        let controls = ship_controls(&snapshot);
        let ship = &mut world.ships[0];
        oag_physics::step(
            &mut ship.physics,
            &controls,
            &ship.handling,
            environment,
            raycaster,
            options.dt,
        );
        world.tick += 1;
    }

    out
}

/// One tick of our state, in the recording's columns.
///
/// The angular velocity is written back in the recording's own convention rather
/// than ours, so the comparison happens in the recorded column's space and no
/// recorded number is reinterpreted on the way in. `timer_2e0` is `None` because
/// nothing in [`ShipState`] models that gate - it is captured so a recording can
/// say which of the two engine gates fired, and a field we do not simulate is
/// reported as not compared rather than as agreement.
fn frame_of(state: &ShipState, tick: u64, dt: f32, speed_cached: f32, options: &Options) -> Frame {
    let body = &state.body;
    let (row0, up, forward) = options
        .basis
        .to_rows(body.right(), body.up(), body.forward());
    // Two angular columns, and they are not the same quantity. `body+0x150` is
    // the rotation rate, so it takes the body's angular velocity unchanged;
    // `body+0x160` is the angular **momentum**, so what goes there is `I * omega`
    // expressed in the body frame - built here as a world vector whose projection
    // onto the (orthonormal) recorded rows is exactly that, because that is what
    // `AngularReading::to_recorded` then takes.
    let local_momentum = (body.orientation.inverse() * body.angular_velocity) * body.inertia;
    Frame {
        angular_velocity: Some(
            options
                .angular
                .to_recorded(body.orientation * local_momentum, (row0, up, forward)),
        ),
        angular_rate: Some(
            options
                .angular
                .to_recorded(body.angular_velocity, (row0, up, forward)),
        ),
        stun_timer: Some(state.stun_timer),
        timer_2e0: None,
        tick,
        dt,
        grounded: state.grounded,
        throttle: state.thrust,
        brake: state.brake,
        steer: state.steer,
        airbrake_left: state.airbrake_left,
        airbrake_right: state.airbrake_right,
        speed_cached,
        row0,
        up,
        forward,
        position: body.position,
        velocity: body.linear_velocity,
        speed: body.linear_velocity.length(),
    }
}

/// The input snapshot for one tick.
///
/// The button edges are carried across ticks in `buttons` rather than rebuilt per
/// tick, because a snapshot built from scratch would report every held button as
/// freshly pressed on every tick. That matters for [`Inputs::Scripted`] and not
/// for the other two: a script is the only mode where a button goes down part-way
/// through a run, so it is the only one where an edge is ever real.
fn snapshot_for(
    recorded: &Frame,
    buttons: &mut Input,
    inputs: &Inputs,
    index: usize,
) -> InputSnapshot {
    let (mask, snapshot) = match inputs {
        Inputs::FromTrace => (
            if recorded.throttle > 0.0 {
                1u32 << button::CROSS
            } else {
                0
            },
            InputSnapshot {
                stick_x: recorded.steer / CONTROL_RANGE,
                stick_y: 0.0,
                airbrake_left: recorded.airbrake_left / CONTROL_RANGE,
                airbrake_right: recorded.airbrake_right / CONTROL_RANGE,
                ..InputSnapshot::new()
            },
        ),
        Inputs::Held(held) => (held.buttons, snapshot_of(held)),
        // A script shorter than the recording holds its last state rather than
        // releasing everything; `Script::at` is where that is argued.
        Inputs::Scripted(states) => {
            let state = states
                .get(index)
                .or_else(|| states.last())
                .copied()
                .unwrap_or_default();
            (state.buttons, snapshot_of(&state))
        }
    };
    buttons.begin_frame(mask);
    InputSnapshot {
        buttons: *buttons,
        ..snapshot
    }
    .sanitised()
}

/// The axes of one authored input state, without its button edges.
fn snapshot_of(state: &Held) -> InputSnapshot {
    InputSnapshot {
        stick_x: state.stick_x,
        stick_y: state.stick_y,
        airbrake_left: state.airbrake_left,
        airbrake_right: state.airbrake_right,
        ..InputSnapshot::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::{Field, Tolerances, compare};
    use oag_physics::CollisionWorld;

    /// How heavy a test ship is, and why.
    ///
    /// **There is no force-free configuration of the recovered force law.** Two of
    /// its terms are constants rather than handling parameters - rolling
    /// resistance is a flat `2.0` opposing motion
    /// ([`oag_physics::passive::ROLLING_RESISTANCE`]) and the airborne quadratic
    /// drag coefficient is `-0.002` - so an all-zero parameter set still
    /// decelerates: at mass 1 that is 2 units/s^2, which is a third of a slow
    /// ship's velocity every tick. They are *forces*, so mass is the only dial
    /// that makes them negligible, and a ship this heavy accelerates at 2e-6
    /// units/s^2: nothing over a two-second run.
    ///
    /// This is a property of the harness's fixtures, not a claim about any ship.
    const TEST_MASS: f32 = 1e6;

    /// A parameter set that is inert except for mass, so a replay is as close to a
    /// free body as the force law allows. The real values are read off the
    /// player's own disc.
    fn inert_handling() -> Handling {
        let mut handling = Handling::ZERO;
        handling.physical.mass = TEST_MASS;
        handling
    }

    fn frame(position: Vec3, velocity: Vec3) -> Frame {
        Frame {
            position,
            velocity,
            speed: velocity.length(),
            speed_cached: velocity.length(),
            ..Frame::default()
        }
    }

    /// A straight coast at constant velocity, in the recording's own conventions.
    ///
    /// A recorded basis is positively oriented - `cross(row0, up) = forward` - so
    /// a ship aimed along `+z` with up `+y` records row 0 along `+x`, and under
    /// the measured reading that row 0 is the ship's **left**, making the body's
    /// right `-x`. Getting this fixture wrong is the same mistake as getting the
    /// reading wrong, so it is spelled out rather than eyeballed.
    fn coasting(ticks: usize, dt: f32, velocity: Vec3) -> Trace {
        Trace {
            frames: (0..ticks)
                .map(|tick| Frame {
                    tick: tick as u64,
                    dt,
                    row0: Vec3::X,
                    up: Vec3::Y,
                    forward: Vec3::Z,
                    ..frame(velocity * (tick as f32 * dt), velocity)
                })
                .collect(),
        }
    }

    #[test]
    fn the_fixture_basis_is_positively_oriented_like_a_real_one() {
        let frame = coasting(1, 1.0 / 60.0, Vec3::ZERO).frames[0];
        assert!(frame.basis_is_positively_oriented(1e-6));
    }

    #[test]
    fn the_recorded_basis_becomes_the_body_s_orientation() {
        let recorded = coasting(1, 1.0 / 60.0, Vec3::ZERO).frames[0];
        let orientation = orientation_of(&recorded, Basis::LeftUpForward);
        // Row 0 is the ship's left, so the body's right is -x; row 2 is the nose,
        // so the body's forward - which is its own -z - is +z.
        assert!((orientation * Vec3::X - Vec3::NEG_X).length() < 1e-6);
        assert!((orientation * Vec3::Y - Vec3::Y).length() < 1e-6);
        assert!((orientation * Vec3::NEG_Z - Vec3::Z).length() < 1e-6);
    }

    /// Both readings of the basis must be rotations. One flipped row would be a
    /// reflection, which `Quat::from_mat3` turns into silent nonsense rather than
    /// an error - so the alternative reading flips two rows, and this is what says
    /// so in arithmetic.
    #[test]
    fn both_readings_of_the_basis_are_proper_rotations() {
        let recorded = coasting(1, 1.0 / 60.0, Vec3::ZERO).frames[0];
        for basis in [Basis::LeftUpForward, Basis::RightUpBack] {
            let q = orientation_of(&recorded, basis);
            let (x, y, z) = (q * Vec3::X, q * Vec3::Y, q * Vec3::Z);
            assert!(
                (x.cross(y) - z).length() < 1e-5,
                "{basis:?} is a reflection"
            );
            assert!((q.length() - 1.0).abs() < 1e-6);
        }
    }

    /// The two readings disagree about which way the ship faces, which is the
    /// whole point of being able to run a comparison both ways.
    #[test]
    fn the_two_readings_of_the_basis_face_opposite_ways() {
        let recorded = coasting(1, 1.0 / 60.0, Vec3::ZERO).frames[0];
        let measured = orientation_of(&recorded, Basis::LeftUpForward) * Vec3::NEG_Z;
        let other = orientation_of(&recorded, Basis::RightUpBack) * Vec3::NEG_Z;
        assert!((measured + other).length() < 1e-6, "{measured} vs {other}");
    }

    #[test]
    fn a_frame_round_trips_through_a_reading_of_the_basis() {
        for basis in [Basis::LeftUpForward, Basis::RightUpBack] {
            let (right, up, forward) = basis.to_body(Vec3::X, Vec3::Y, Vec3::Z);
            assert_eq!(
                basis.to_rows(right, up, forward),
                (Vec3::X, Vec3::Y, Vec3::Z)
            );
        }
    }

    #[test]
    fn a_replay_starts_exactly_where_the_recording_starts() {
        let recorded = coasting(8, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        assert_eq!(simulated.len(), recorded.len());
        assert_eq!(simulated.frames[0].position, recorded.frames[0].position);
        assert_eq!(simulated.frames[0].velocity, recorded.frames[0].velocity);
        assert_eq!(simulated.frames[0].tick, recorded.frames[0].tick);
        assert!((simulated.frames[0].row0 - recorded.frames[0].row0).length() < 1e-6);
    }

    /// A near-free body is the harness's own self-check: if *this* diverges, the
    /// bug is in the harness rather than in the physics. See [`TEST_MASS`] for why
    /// "near".
    #[test]
    fn a_coasting_ship_tracks_a_straight_line_recording() {
        let recorded = coasting(120, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert!(!comparison.diverged(), "{comparison}");
    }

    /// `speed_cached` is the previous tick's *forward-projected* speed, not the
    /// previous tick's speed. A straight-line capture cannot tell those apart, so
    /// this fixture puts the velocity across the nose, where the magnitude reading
    /// would emit 22 and the projection emits nothing.
    #[test]
    fn speed_cached_is_the_forward_projection_and_not_the_magnitude() {
        let across = Vec3::new(22.0, 0.0, 0.0);
        // Aimed along +z (row 2 is the nose) while travelling along +x.
        let recorded = Trace {
            frames: (0..4)
                .map(|tick| Frame {
                    tick,
                    dt: 1.0 / 60.0,
                    row0: Vec3::X,
                    up: Vec3::Y,
                    forward: Vec3::Z,
                    ..frame(across * (tick as f32 / 60.0), across)
                })
                .collect(),
        };
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        // Tick 0 is seeded from the recording, so the first tick the replay
        // computes for itself is tick 1.
        assert!(
            simulated.frames[1].speed_cached.abs() < 1e-3,
            "{} is the magnitude, not the projection",
            simulated.frames[1].speed_cached
        );
        // And the magnitude really is 22 here, so the two readings are separated
        // by this fixture rather than merely agreeing on a small number.
        assert!((simulated.frames[1].speed - 22.0).abs() < 1e-3);
    }

    /// Replaying the same recording twice must produce the same trace, or nothing
    /// the harness reports means anything. `docs/architecture/determinism.md`.
    #[test]
    fn a_replay_is_reproducible() {
        let recorded = coasting(60, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        let run = || {
            replay(
                &recorded,
                &inert_handling(),
                &Environment::default(),
                &CollisionWorld::new(),
                &Options::default(),
            )
        };
        assert_eq!(run(), run());
    }

    /// Sampling after the step instead of before it is the mistake this pins: it
    /// shifts every row by one tick, which reads as a lag in the physics.
    #[test]
    fn rows_are_sampled_before_the_step_not_after() {
        let dt = 1.0 / 60.0;
        let velocity = Vec3::new(0.0, 0.0, 22.0);
        let recorded = coasting(4, dt, velocity);
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        // Row 1 must hold one tick of travel, not two and not none.
        let travelled = simulated.frames[1].position - simulated.frames[0].position;
        assert!((travelled - velocity * dt).length() < 1e-5, "{travelled}");
    }

    #[test]
    fn a_held_input_ignores_what_the_recording_says_the_controls_were() {
        let mut recorded = coasting(4, 1.0 / 60.0, Vec3::ZERO);
        for frame in &mut recorded.frames {
            frame.throttle = 100.0;
        }
        let options = Options {
            inputs: Inputs::Held(Held::default()),
            ..Options::default()
        };
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &options,
        );
        // Tick 0 is the seeded initial condition, so the recording's throttle is
        // still there; from tick 1 the held input - nothing - is what drives it.
        assert_eq!(simulated.frames[0].throttle, 100.0);
        assert_eq!(simulated.frames[1].throttle, 0.0);
    }

    #[test]
    fn a_trace_derived_input_carries_the_recorded_throttle_through() {
        let mut recorded = coasting(4, 1.0 / 60.0, Vec3::ZERO);
        for frame in &mut recorded.frames {
            frame.throttle = 100.0;
        }
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        assert!(simulated.frames.iter().all(|f| f.throttle == 100.0));
    }

    /// The point of a script: the input *changes* part-way through a run, and it
    /// changes on the tick the file says rather than on whatever tick the
    /// recording happened to.
    #[test]
    fn a_scripted_input_changes_on_the_tick_the_script_says() {
        let mut recorded = coasting(6, 1.0 / 60.0, Vec3::ZERO);
        for frame in &mut recorded.frames {
            // The recording says thrust throughout, and the script must win.
            frame.throttle = 100.0;
        }
        let script = crate::script::Script::parse("3 none\n3 cross\n").expect("parses");
        let options = Options {
            inputs: Inputs::Scripted(script.states.clone()),
            ..Options::default()
        };
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &options,
        );
        // Tick 0 is the seeded initial condition, so it still carries the
        // recording's throttle; ticks 1 and 2 are the script's `none`, and the
        // thrust the script asks for at tick 3 shows in the row *after* it,
        // because a row is sampled before its own step.
        assert_eq!(simulated.frames[1].throttle, 0.0);
        assert_eq!(simulated.frames[3].throttle, 0.0);
        assert!(simulated.frames[4].throttle > 0.0);
    }

    /// A script's steering must reach the force law through the same axis a pad's
    /// would, or a scripted turn is not the turn a player would take. Asserted on
    /// the snapshot rather than on the ship's own `steer`, which is downstream of
    /// a ramp whose rate the inert test parameter set holds at zero.
    #[test]
    fn a_scripted_dpad_reaches_the_stick_axis_and_the_cross_bit() {
        let script = crate::script::Script::parse("2 cross left\n").expect("parses");
        let inputs = Inputs::Scripted(script.states.clone());
        let mut buttons = Input::new();
        let snapshot = snapshot_for(&Frame::default(), &mut buttons, &inputs, 0);
        assert_eq!(snapshot.stick_x, -1.0, "left is negative stick x");
        assert!(snapshot.buttons.is_held(button::CROSS));
        assert!(
            snapshot.buttons.is_pressed(button::LEFT),
            "tick 0 is an edge"
        );

        let snapshot = snapshot_for(&Frame::default(), &mut buttons, &inputs, 1);
        assert!(
            !snapshot.buttons.is_pressed(button::LEFT),
            "a button held across two scripted ticks is not pressed twice"
        );
    }

    /// A script shorter than the recording holds its last state rather than
    /// releasing everything, which would put a deceleration into the comparison
    /// that nobody asked for.
    #[test]
    fn a_short_script_holds_its_last_state_for_the_rest_of_the_run() {
        let recorded = coasting(8, 1.0 / 60.0, Vec3::ZERO);
        let script = crate::script::Script::parse("2 cross\n").expect("parses");
        let options = Options {
            inputs: Inputs::Scripted(script.states.clone()),
            ..Options::default()
        };
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &options,
        );
        assert!(simulated.frames[7].throttle > 0.0);
    }

    /// A one-state script and the equivalent held input are the same run. They
    /// share a type for exactly this reason, and this is what says so.
    #[test]
    fn a_one_state_script_is_the_same_run_as_the_equivalent_held_input() {
        let recorded = coasting(30, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        let script = crate::script::Script::parse("30 cross\n").expect("parses");
        let run = |inputs| {
            replay(
                &recorded,
                &inert_handling(),
                &Environment::default(),
                &CollisionWorld::new(),
                &Options {
                    inputs,
                    ..Options::default()
                },
            )
        };
        assert_eq!(
            run(Inputs::Scripted(script.states.clone())),
            run(Inputs::Held(script.states[0]))
        );
    }

    /// The recorded yaw rate the captures show with the stick held over, and the
    /// signal the angular-velocity column was added to compare against:
    /// `+1.51 rad/s`, `docs/ghidra/functions/psp-pulse/engine.md`.
    const RECORDED_YAW_RATE: f32 = 1.51;

    /// Tick 0 is an initial condition, not a measurement - the same thing that is
    /// already true of position, velocity and the basis. So a recorded angular
    /// velocity must come back out of tick 0 unchanged under **every** reading:
    /// the seeding and the writing-back are one transformation and its inverse,
    /// and a reading where they are not would put a tick-0 error into the
    /// comparison that looks exactly like a physics bug.
    #[test]
    fn a_recorded_rotation_survives_tick_zero_under_every_reading() {
        for angular in AngularReading::ALL {
            let mut recorded = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
            let recorded_rate = Vec3::new(0.1, RECORDED_YAW_RATE, -0.05);
            for frame in &mut recorded.frames {
                frame.angular_velocity = Some(recorded_rate);
            }
            let simulated = replay(
                &recorded,
                &inert_handling(),
                &Environment::default(),
                &CollisionWorld::new(),
                &Options {
                    angular,
                    ..Options::default()
                },
            );
            let written = simulated.frames[0]
                .angular_velocity
                .expect("a replay always writes one");
            assert!(
                (written - recorded_rate).length() < 1e-5,
                "{angular:?}: {written} is not {recorded_rate}"
            );
        }
    }

    /// A capture taken before the column existed leaves the rotation at rest,
    /// which is what every run did before it existed. Nothing is invented for the
    /// older file - see `crate::trace::REQUIRED_COLUMNS`.
    #[test]
    fn a_recording_without_an_angular_velocity_starts_at_rest() {
        let recorded = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        assert_eq!(recorded.frames[0].angular_velocity, None);
        let state = initial_state(
            &recorded.frames[0],
            &inert_handling(),
            Basis::default(),
            AngularReading::default(),
        );
        assert_eq!(state.body.angular_velocity, Vec3::ZERO);
    }

    /// And the seeding is not decorative: a ship handed the recorded turn rate
    /// must actually be turning, or the column would be read and then ignored.
    ///
    /// **Both angular columns seed, and they are not the same quantity.**
    /// `omega_*` (`body+0x150`) is the rate and seeds it directly; `avel_*`
    /// (`body+0x160`) is `I * omega` and has to be divided by the tensor on the
    /// way in. This test drives both and asserts they agree when handed the same
    /// physical rotation - the version of it that seeded a body's angular
    /// velocity straight from the momentum column started every run spinning
    /// `21.6x` too fast about the up axis, and passed, because it only asked
    /// whether the nose had moved.
    #[test]
    fn a_seeded_rotation_actually_turns_the_ship() {
        let rate = Vec3::new(0.0, RECORDED_YAW_RATE, 0.0);
        let momentum = rate * oag_physics::forces::ship_inertia();

        let mut by_rate = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        for frame in &mut by_rate.frames {
            frame.angular_rate = Some(rate);
        }
        let mut by_momentum = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        for frame in &mut by_momentum.frames {
            frame.angular_velocity = Some(momentum);
        }
        let still = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));

        let run = |recorded| {
            replay(
                recorded,
                &inert_handling(),
                &Environment::default(),
                &CollisionWorld::new(),
                &Options::default(),
            )
        };
        let (turned, from_momentum, stayed) = (run(&by_rate), run(&by_momentum), run(&still));

        let sweep = |a: &crate::trace::Frame, b: &crate::trace::Frame| {
            crate::compare::checks(a, b, &crate::compare::Tolerances::default())
                .iter()
                .flatten()
                .find(|check| check.field == Field::Forward)
                .expect("the forward axis is always compared")
                .error
        };

        // A tick of 1.51 rad/s is about 0.025 rad, so tick 1's noses are that far
        // apart; the ship left at rest has not moved its at all.
        assert!(
            sweep(&turned.frames[1], &stayed.frames[1]) > 0.01,
            "the rate column did not turn the ship"
        );
        // And the momentum column, divided by the tensor, is the same rotation.
        assert!(
            sweep(&turned.frames[1], &from_momentum.frames[1]) < 1e-5,
            "the two columns disagree about the same physical rotation"
        );
    }

    /// The simulated side always carries the timers it models and never the one
    /// it does not, so `timer_2e0` is reported as not compared rather than as
    /// agreement. `craft+0x2e0`'s arming condition has never been read.
    #[test]
    fn a_replay_carries_the_stun_timer_and_not_the_gate_it_does_not_model() {
        let recorded = coasting(4, 1.0 / 60.0, Vec3::ZERO);
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        assert_eq!(simulated.frames[0].stun_timer, Some(0.0));
        assert_eq!(simulated.frames[0].timer_2e0, None);
    }

    /// A driven run has no recording behind it, so nothing can be seeded from
    /// one: `drive` must produce exactly the ticks it was asked for, from the
    /// state it was handed.
    #[test]
    fn a_driven_run_is_as_long_as_it_was_asked_for_and_starts_where_it_was_put() {
        let mut initial = ShipState::default();
        initial.body.mass = TEST_MASS;
        initial.body.position = Vec3::new(1.0, 2.0, 3.0);
        let script = crate::script::Script::parse("40 cross\n").expect("parses");
        let run = drive(
            initial,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &script.states,
            &DriveOptions {
                ticks: 40,
                ..DriveOptions::default()
            },
        );
        assert_eq!(run.len(), 40);
        assert_eq!(run.frames[0].position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(run.frames[0].tick, 0);
        assert_eq!(run.frames[39].tick, 39);
    }

    /// A script shorter than the run holds its last state here too, which is the
    /// same rule [`Script::at`](crate::script::Script::at) argues for and the
    /// reason `--ticks` may exceed a scenario's own length.
    #[test]
    fn a_driven_run_holds_a_short_script_s_last_state() {
        let mut initial = ShipState::default();
        initial.body.mass = TEST_MASS;
        let script = crate::script::Script::parse("2 none\n2 cross\n").expect("parses");
        let run = drive(
            initial,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &script.states,
            &DriveOptions {
                ticks: 20,
                ..DriveOptions::default()
            },
        );
        assert_eq!(run.frames[1].throttle, 0.0);
        assert!(run.frames[19].throttle > 0.0, "the last state is held");
    }

    /// Driving the same scenario twice must give the same trace, for the same
    /// reason replaying one twice must: `docs/architecture/determinism.md`. This
    /// is the property `just scripted-sim` rests on - it is the only side of the
    /// comparison that *can* be reproduced exactly, the emulator's start pose
    /// being repeatable only to about 2.5 degrees.
    #[test]
    fn a_driven_run_is_reproducible() {
        let script = crate::script::Script::parse("10 cross\n10 cross left\n").expect("parses");
        let run = || {
            let mut initial = ShipState::default();
            initial.body.mass = TEST_MASS;
            drive(
                initial,
                &inert_handling(),
                &Environment::default(),
                &CollisionWorld::new(),
                &script.states,
                &DriveOptions {
                    ticks: 20,
                    ..DriveOptions::default()
                },
            )
        };
        assert_eq!(run(), run());
    }

    /// `drive` and `replay` are the same stepping loop with different sources for
    /// the length, the delta and the seed. Handed the same three, they must agree
    /// tick for tick - otherwise a scenario run and a comparison run would be
    /// measuring two different simulations and nobody would notice.
    #[test]
    fn driving_and_replaying_the_same_scenario_agree() {
        let dt = 1.0 / 60.0;
        let recorded = coasting(30, dt, Vec3::new(0.0, 0.0, 22.0));
        let script = crate::script::Script::parse("30 cross\n").expect("parses");
        let handling = inert_handling();
        let replayed = replay(
            &recorded,
            &handling,
            &Environment::default(),
            &CollisionWorld::new(),
            &Options {
                inputs: Inputs::Scripted(script.states.clone()),
                dt: DeltaSource::Fixed(dt),
                ..Options::default()
            },
        );
        let driven = drive(
            initial_state(
                &recorded.frames[0],
                &handling,
                Basis::default(),
                AngularReading::default(),
            ),
            &handling,
            &Environment::default(),
            &CollisionWorld::new(),
            &script.states,
            &DriveOptions {
                ticks: 30,
                dt,
                ..DriveOptions::default()
            },
        );
        assert_eq!(driven.len(), replayed.len());
        for (a, b) in driven.frames.iter().zip(replayed.frames.iter()) {
            assert_eq!(a.position, b.position, "tick {}", a.tick);
            assert_eq!(a.velocity, b.velocity, "tick {}", a.tick);
            assert_eq!(a.throttle, b.throttle, "tick {}", a.tick);
        }
    }

    #[test]
    fn an_empty_recording_replays_to_an_empty_trace() {
        let simulated = replay(
            &Trace::default(),
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        assert!(simulated.is_empty());
    }

    /// The whole harness is worthless if it cannot say *when* a run went wrong, so
    /// this drives a real divergence through it: gravity in the parameter set that
    /// the recording's straight line does not have.
    #[test]
    fn a_physics_difference_shows_up_as_a_dated_divergence() {
        let recorded = coasting(120, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        let mut handling = inert_handling();
        handling.physical.normal_gravity = 9.8;
        handling.physical.flight_gravity = 9.8;
        let simulated = replay(
            &recorded,
            &handling,
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        let divergence = comparison
            .first_divergence
            .clone()
            .expect("gravity must show");
        assert!(
            divergence.tick > 0,
            "tick 0 is the seeded initial condition"
        );
        assert_eq!(
            comparison.field(Field::Position).trend.verdict,
            crate::compare::TrendVerdict::Growing,
            "a constant acceleration is a systematic error, not a bounded one"
        );
    }

    fn reseeding(every: usize) -> Options {
        Options {
            reseed: NonZeroUsize::new(every),
            ..Options::default()
        }
    }

    /// The default must stay exactly what it was, or every existing invocation
    /// silently changes meaning.
    #[test]
    fn a_run_is_seeded_once_unless_asked_otherwise() {
        assert_eq!(Options::default().reseed, None);

        let recorded = coasting(120, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        let mut handling = inert_handling();
        handling.physical.normal_gravity = 9.8;
        handling.physical.flight_gravity = 9.8;
        let once = replay(
            &recorded,
            &handling,
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        let explicit = replay(
            &recorded,
            &handling,
            &Environment::default(),
            &CollisionWorld::new(),
            &Options {
                reseed: None,
                ..Options::default()
            },
        );
        assert_eq!(once.to_csv(), explicit.to_csv());
    }

    /// The property the whole option exists for: error stops compounding.
    ///
    /// The same falling-under-gravity divergence as
    /// [`a_physics_difference_shows_up_as_a_dated_divergence`], which grows as
    /// `t^2` when the run is seeded once. Re-seeded every twenty ticks it cannot
    /// exceed what twenty ticks of that acceleration produce, however long the
    /// recording is - so the max error stops being a statement about the run's
    /// length and becomes one about the window's.
    #[test]
    fn reseeding_bounds_the_error_by_the_window_rather_than_the_run() {
        let recorded = coasting(600, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        let mut handling = inert_handling();
        handling.physical.normal_gravity = 9.8;
        handling.physical.flight_gravity = 9.8;

        let seeded_once = replay(
            &recorded,
            &handling,
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        );
        let reseeded = replay(
            &recorded,
            &handling,
            &Environment::default(),
            &CollisionWorld::new(),
            &reseeding(20),
        );

        let once = compare(&recorded, &seeded_once, &Tolerances::default());
        let windowed = compare(&recorded, &reseeded, &Tolerances::default());
        let (once, windowed) = (
            once.field(Field::Position).max_error,
            windowed.field(Field::Position).max_error,
        );
        // Free fall over a window of `n` ticks goes as `n^2`, so twenty ticks of
        // it against six hundred is a factor of nine hundred. An order of
        // magnitude is the assertion; the exact ratio is arithmetic nobody should
        // have to keep true.
        assert!(
            windowed * 10.0 < once,
            "reseeded error {windowed} is not bounded well below the compounded {once}"
        );
    }

    /// A window's first row is the recording's own state, so it is an initial
    /// condition and not a measurement - the same property tick 0 has, and the
    /// reason the seed happens before the row is emitted rather than after.
    #[test]
    fn every_window_starts_exactly_on_the_recording() {
        let recorded = coasting(100, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        let mut handling = inert_handling();
        handling.physical.normal_gravity = 9.8;
        handling.physical.flight_gravity = 9.8;
        let simulated = replay(
            &recorded,
            &handling,
            &Environment::default(),
            &CollisionWorld::new(),
            &reseeding(25),
        );

        for tick in [0, 25, 50, 75] {
            assert_eq!(
                simulated.frames[tick].position, recorded.frames[tick].position,
                "tick {tick} is a seeded row and must agree exactly"
            );
        }
        assert_ne!(
            simulated.frames[24].position, recorded.frames[24].position,
            "the tick before a seed is a measurement and must be free to diverge"
        );
    }
}
