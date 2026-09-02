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
use oag_formats::track;
use oag_gameplay::input::{Button, Input};
use oag_gameplay::{ControlScheme, InputSnapshot, Ship, World, ship_controls};
use oag_physics::controls::CONTROL_RANGE;
use oag_physics::{Environment, Handling, Raycaster, ShipState, TrackSample};

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
    /// Which control scheme maps the script's buttons.
    ///
    /// Only the sideshift gesture differs between the two, so every scenario
    /// that does not tap `l`/`r` or hold the sideshift button runs identically
    /// under either. It is a field rather than a constant because a *gesture*
    /// scenario is only meaningful under the scheme that has that gesture - see
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub scheme: ControlScheme,
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
    let Some((x, y, z)) = look_columns(right, forward) else {
        return Quat::IDENTITY;
    };
    debug_assert!(
        (y - up.normalize_or_zero()).length() < 1e-2,
        "the recorded basis is not the one this reading describes"
    );
    Quat::from_mat3(&Mat3::from_cols(x, y, z)).normalize()
}

/// The orthonormal columns of a rotation whose `-Z` is `forward` and whose `X`
/// stays as close to `right` as orthonormality allows, or `None` for a
/// degenerate input.
///
/// Gram-Schmidt off the forward axis, which is the one the frame is aimed
/// along: rounding is shared out rather than concentrated in whichever column
/// happened to be last. `Y` is derived, so the caller decides what to do when
/// it disagrees with a recorded up axis - the ship reading asserts, the camera
/// reading adapts.
fn look_columns(right: Vec3, forward: Vec3) -> Option<(Vec3, Vec3, Vec3)> {
    let (right, forward) = (right.normalize_or_zero(), forward.normalize_or_zero());
    if right == Vec3::ZERO || forward == Vec3::ZERO {
        return None;
    }
    let z = -forward;
    let x = (right - z * right.dot(z)).normalize_or_zero();
    if x == Vec3::ZERO {
        return None;
    }
    Some((x, z.cross(x), z))
}

/// The orientation a recorded camera pose describes, in the same convention as
/// [`orientation_of`]: `X = right`, `Y = up`, `-Z` = the look direction, so the
/// result plugs straight into a look-down-negative-Z renderer.
///
/// **The camera node stores its rotation transposed relative to the ship
/// node**: the camera's world axes are the *columns* of the recorded rows, and
/// the look direction is the negated third column. Measured on a live
/// `--camera` capture (2026-08-04, v1.20.4, `UCUS98712`): the eye sits 11.64
/// units from the ship - camera.md's own `OPT_CLOSE` distance to three
/// digits - and `-col2` points at the ship with `dot = +0.987` (the residual
/// is the raised aim point), while the stored `cam_fwd` row points `-0.333`.
/// A transposed store also explains the node's negated-eye field: rows that
/// are world-to-camera and a negated translation are the two halves of a view
/// matrix kept apart. Confidence 80: one session, one view setting, but the
/// competing reading is ruled out by sign.
///
/// The third camera axis is **derived**, not read, so a rounding-level
/// left-handedness cannot reach `Quat::from_mat3`; the [`Basis`] left/right
/// switch is about the rigid body's rows and does not apply here.
///
/// `None` for a capture without camera columns or with a degenerate pose.
#[must_use]
pub fn camera_orientation_of(frame: &Frame) -> Option<Quat> {
    let (row0, row1, row2) = (frame.camera_row0?, frame.camera_up?, frame.camera_forward?);
    let col0 = Vec3::new(row0.x, row1.x, row2.x);
    let col1 = Vec3::new(row0.y, row1.y, row2.y).normalize_or_zero();
    let col2 = Vec3::new(row0.z, row1.z, row2.z);
    let (mut x, mut y, z) = look_columns(col0, -col2)?;
    if y.dot(col1) < 0.0 {
        x = -x;
        y = -y;
    }
    Some(Quat::from_mat3(&Mat3::from_cols(x, y, z)).normalize())
}

/// Table order's nearest sample to `position`, and the sample after it.
///
/// The same "where am I and what is next" pair `oag_game::race::Race::tick`
/// reads for the player every tick - the nearest table entry and its table-order
/// successor - at this crate's own resolution. Resampled independently by
/// [`crate::main`]'s `resample` rather than shared: `oag-trace` cannot depend on
/// `oag-game`, the composition root that owns `Spline` (rule 2,
/// `scripts/check-dependency-rules.py`), so the resampling is duplicated crate to
/// crate on purpose, the same way `oag_render::track` duplicates it a third time.
///
/// `None` when `samples` is empty. The successor is `None` past the end of the
/// table, exactly as the player's own locator leaves it - [`maglock::Hold`]
/// already treats a missing second sample as "use the first alone".
#[must_use]
fn locate(samples: &[track::Sample], position: Vec3) -> (Option<TrackSample>, Option<TrackSample>) {
    let index = samples
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = Vec3::from_array(a.pos).distance_squared(position);
            let db = Vec3::from_array(b.pos).distance_squared(position);
            da.total_cmp(&db)
        })
        .map(|(index, _)| index);
    let track_sample = index.map(|i| track_sample_of(&samples[i]));
    let track_sample_next = index.and_then(|i| samples.get(i + 1)).map(track_sample_of);
    (track_sample, track_sample_next)
}

/// The conversion `oag_game::race::Spline::track_sample` makes, restated here for
/// the reason [`locate`] gives: the disc's sample is the *unlifted* surface
/// point, and the hold wants it lifted back to where the running game holds it,
/// with `down` passed raw for the probe direction.
#[must_use]
fn track_sample_of(sample: &track::Sample) -> TrackSample {
    let down = Vec3::from_array(sample.down);
    TrackSample {
        position: Vec3::from_array(sample.pos) - down * track::HOVER_LIFT,
        down,
    }
}

/// Rebuilds `base` with this tick's `track_sample`/`track_sample_next`, from
/// `track` located at `position` - `base` unchanged when `track` is `None`, so a
/// caller with no spline (a `run` with no `--source`, `--no-collision` aside)
/// loses nothing it did not already lack.
#[must_use]
fn located_environment(
    base: &Environment,
    track: Option<&[track::Sample]>,
    position: Vec3,
) -> Environment {
    let (track_sample, track_sample_next) = match track {
        Some(samples) => locate(samples, position),
        None => (None, None),
    };
    Environment {
        track_sample,
        track_sample_next,
        ..*base
    }
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
///
/// `track` is this tick's locator: the track's spline, resampled, so the
/// magstrip hold in [`oag_physics::maglock`] has a section to read. `None` runs
/// exactly as before - the hold's blend stays at zero for the whole run, since
/// [`oag_physics::maglock::probe`] has nothing to build a ray direction from
/// without a sample.
#[must_use]
pub fn replay<R: Raycaster + ?Sized>(
    trace: &Trace,
    handling: &Handling,
    environment: &Environment,
    raycaster: &R,
    options: &Options,
    track: Option<&[track::Sample]>,
) -> Trace {
    let Some(first) = trace.frames.first() else {
        return Trace::default();
    };

    let mut world = World::new(SEED);
    world.ships[0] = Ship {
        physics: initial_state(first, handling, options.basis, options.angular),
        handling: *handling,
        segment: 0,
        // Slot 0 is flown by the script, not by a driver, and a replay has no
        // racing line for one to follow anyway.
        driver: oag_ai::Driver::default(),
        // A replay has no course either, so nothing ever advances this.
        standing: oag_race::Standing::default(),
        autopilot_timer: 0.0,
        // A replay has no track and therefore no `Weapon Pad`, so nothing can
        // ever fill this. Named rather than left to `..Default::default()` so
        // that a pickup reaching a replay is a compile error to think about
        // rather than a silent empty slot.
        pickup: oag_gameplay::Held::empty(),
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
        let controls = ship_controls(&snapshot, options.scheme);
        let position = world.ships[0].physics.body.position;
        let env = located_environment(environment, track, position);
        let ship = &mut world.ships[0];
        oag_physics::step(
            &mut ship.physics,
            &controls,
            &ship.handling,
            &env,
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
    /// Which control scheme maps the script's buttons.
    ///
    /// Only the sideshift gesture differs between the two, so every scenario
    /// that does not tap `l`/`r` or hold the sideshift button runs identically
    /// under either. It is a field rather than a constant because a *gesture*
    /// scenario is only meaningful under the scheme that has that gesture - see
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub scheme: ControlScheme,
}

impl Default for DriveOptions {
    fn default() -> Self {
        Self {
            ticks: 0,
            dt: 1.0 / 60.0,
            basis: Basis::default(),
            angular: AngularReading::default(),
            scheme: ControlScheme::default(),
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
///
/// `track` is [`replay`]'s own locator - see its doc comment.
#[must_use]
pub fn drive<R: Raycaster + ?Sized>(
    initial: ShipState,
    handling: &Handling,
    environment: &Environment,
    raycaster: &R,
    script: &[Held],
    options: &DriveOptions,
    track: Option<&[track::Sample]>,
) -> Trace {
    let script = script.to_vec();
    drive_with(
        initial,
        handling,
        environment,
        raycaster,
        options,
        track,
        |tick, _| {
            // A script shorter than the run holds its last state; `Script::at` argues why.
            Some(
                script
                    .get(tick)
                    .or_else(|| script.last())
                    .copied()
                    .unwrap_or_default(),
            )
        },
    )
    .0
}

/// Runs a *controller* through our physics, and writes down what it pressed.
///
/// The same loop [`drive`] is, with the script replaced by a closure that is
/// handed the state at the start of every tick and answers with the input for
/// that tick. `drive` itself is this with a closure that reads an array, so a
/// planned run and a scripted run are stepped by one piece of code rather than
/// two that have to be kept agreeing.
///
/// The second return value is the input the controller chose, tick by tick, in
/// the order it chose it - which is exactly a [`crate::Script`]'s `states`, and
/// therefore an authored artefact that can be replayed into either runtime. That
/// is the point of the signature: a controller is a *way of finding* a script,
/// not a thing anything downstream has to know about.
///
/// Returning `None` from the closure ends the run. The frame for that tick has
/// already been recorded, so the trace's last frame is the state the controller
/// stopped on, and the recorded input is one shorter than the trace.
///
/// `track` is [`replay`]'s own locator - see its doc comment.
#[must_use]
pub fn drive_with<R, F>(
    initial: ShipState,
    handling: &Handling,
    environment: &Environment,
    raycaster: &R,
    options: &DriveOptions,
    track: Option<&[track::Sample]>,
    mut controller: F,
) -> (Trace, Vec<Held>)
where
    R: Raycaster + ?Sized,
    F: FnMut(usize, &ShipState) -> Option<Held>,
{
    let mut world = World::new(SEED);
    world.ships[0] = Ship {
        physics: initial,
        handling: *handling,
        segment: 0,
        // Slot 0 is flown by the script, not by a driver, and a replay has no
        // racing line for one to follow anyway.
        driver: oag_ai::Driver::default(),
        // A replay has no course either, so nothing ever advances this.
        standing: oag_race::Standing::default(),
        autopilot_timer: 0.0,
        // A replay has no track and therefore no `Weapon Pad`, so nothing can
        // ever fill this. Named rather than left to `..Default::default()` so
        // that a pickup reaching a replay is a compile error to think about
        // rather than a silent empty slot.
        pickup: oag_gameplay::Held::empty(),
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
    let mut chosen = Vec::with_capacity(options.ticks);
    let mut frame_options = Options {
        inputs: Inputs::Held(Held::default()),
        dt: DeltaSource::Fixed(options.dt),
        basis: options.basis,
        angular: options.angular,
        scheme: options.scheme,
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

        let Some(held) = controller(index, state) else {
            break;
        };
        chosen.push(held);
        frame_options.inputs = Inputs::Held(held);

        // `Frame::default()` is only ever read by `Inputs::FromTrace`, which this
        // is not: the controller is the whole of the input here.
        let snapshot = snapshot_for(
            &Frame::default(),
            &mut buttons,
            &frame_options.inputs,
            index,
        );
        let controls = ship_controls(&snapshot, options.scheme);
        let position = world.ships[0].physics.body.position;
        let env = located_environment(environment, track, position);
        let ship = &mut world.ships[0];
        oag_physics::step(
            &mut ship.physics,
            &controls,
            &ship.handling,
            &env,
            raycaster,
            options.dt,
        );
        world.tick += 1;
    }

    (out, chosen)
}

/// One tick of our state, in the recording's columns.
///
/// The angular velocity is written back in the recording's own convention rather
/// than ours, so the comparison happens in the recorded column's space and no
/// recorded number is reinterpreted on the way in. `timer_2e0` is `None` because
/// nothing in [`ShipState`] models that gate - it is captured so a recording can
/// say which of the two engine gates fired, and a field we do not simulate is
/// reported as not compared rather than as agreement. The flare group is `None`
/// for the same reason, with a dependency rule behind it - see the field.
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
        // The replay simulates the pool, so it is compared. It starts full only
        // if the caller filled it - `oag_physics::damage::reset` - and a replay
        // that did not will read a flat zero against the capture's pool rather
        // than silently agreeing.
        shield: Some(state.shield),
        // Neither camera fov term is modelled by `ShipState` - `SPEED_FOV_GAIN_DEG`
        // in `crates/game/src/race.rs` reproduces the additive term as a
        // projection-time constant, not as simulation state, and the intercept
        // has no known writer at all. Reported absent for the same reason
        // `timer_2e0` above is: a field we do not simulate must not read as
        // agreement.
        fov_intercept: None,
        fov_additive: None,
        // Not modelled by `ShipState` either - it is the constructor's
        // human-vs-AI argument, not physics state - so reported absent for the
        // same reason as the two fov terms above.
        controller_class: None,
        // `ShipState::sideshift_timers` carries the shift force but not the tap
        // window or the lockout, so there is nothing to compare the other three
        // columns against yet - reported absent rather than a false agreement.
        // See handover/sideshift-has-no-runtime-leg.md.
        ss_tap_window_l: None,
        ss_tap_window_r: None,
        ss_shift_l: None,
        ss_shift_r: None,
        ss_lockout: None,
        // The replay simulates the ship, not the original's camera rig, so the
        // camera pose is not compared rather than reported as agreement.
        camera_row0: None,
        camera_up: None,
        camera_forward: None,
        camera_position: None,
        // The flare state machine is `oag_render::exhaust::Exhaust`, and this
        // crate must not depend on `oag-render` (`just check-deps`). So a replay
        // reports the flare as not compared rather than as agreement; the side
        // that *can* write it is `oag-game --race --trace-out`, which owns both.
        flare: None,
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
                Button::Cross.bit()
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
mod tests;
