//! The comparison engine: two traces in, the first divergent tick out.
//!
//! What matters is **not** whether the two agree. It is the tick at which they
//! stop agreeing and by how much, because a run that tracks the original for 400
//! ticks and then drifts has a different bug from one that is wrong at tick 1.
//! And because a systematic error is still systematic while it is small, every
//! field also carries a **trend**: an error that grows every tick is a bug even
//! while it is inside tolerance. Both requirements are
//! `docs/reverse-engineering/verification-protocol.md`'s, not this module's.

use std::fmt;

use oag_core::math::Vec3;

use crate::trace::{Frame, Trace};

/// What kind of quantity a field is, which is what decides how it is compared.
///
/// The split is the protocol's: an integer or a discrete state has no reason to
/// be approximately right, so it is compared exactly and no tolerance applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// World-space position. Absolute *or* relative.
    Position,
    /// An axis of the body's basis, compared as the angle between the two axes.
    Orientation,
    /// A velocity or a speed. Relative, because it integrates into position.
    Velocity,
    /// A control state on the original's `0..=100` scale.
    ///
    /// **Not in the protocol's table.** The table predates the capture format,
    /// which records five control states per tick, and they are neither positions
    /// nor velocities. Compared relatively at the velocity tolerance, which is a
    /// provisional choice recorded here rather than hidden in a constant.
    Control,
    /// An angular velocity, in rad/s.
    ///
    /// **Not in the protocol's table either**, for the same reason: there was no
    /// angular-velocity column when the table was written. Relative like a linear
    /// velocity, because it integrates into orientation the same way - but with a
    /// nonzero absolute floor by default, which linear velocity does not have. A
    /// straight-line capture records an angular velocity of *exactly* zero, and a
    /// purely relative test against zero calls any simulated `1e-9` a relative
    /// error of one: every straight capture would report a divergence on tick 0
    /// and bury the real one. See [`Tolerances::angular_velocity_absolute`].
    AngularVelocity,
    /// A timer counting down in seconds.
    ///
    /// **Not in the protocol's table**, whose "timers, counters: exact" row is
    /// about integers. These are floats decremented by a variable `dt`, so exact
    /// agreement is not achievable; they are compared absolutely, at a tolerance
    /// far below the frame time so that a timer running a frame long is a
    /// divergence.
    Timer,
    /// Anything integral or quantised: compared exactly.
    Discrete,
}

/// One traced quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Centre of mass, as a distance between the two positions.
    Position,
    /// Linear velocity, as the length of the difference.
    Velocity,
    /// The body's own speed field.
    Speed,
    /// The craft's cached speed, which the original leaves one tick stale.
    SpeedCached,
    /// Angle between the recorded and simulated row 0 of the basis.
    Row0,
    /// Angle between the up axes.
    Up,
    /// Angle between the forward axes.
    Forward,
    /// Probes in contact over two. Quantised, so exact.
    Grounded,
    /// Throttle state.
    Throttle,
    /// Brake state.
    Brake,
    /// Steering state.
    Steer,
    /// Left airbrake state.
    AirbrakeLeft,
    /// Right airbrake state.
    AirbrakeRight,
    /// Angular velocity, as the length of the difference, in the **recorded**
    /// column's own convention - see [`crate::trace::AngularReading`]. Compared
    /// there rather than in ours so that nothing reinterprets a recorded number
    /// on the way in.
    ///
    /// Absent from a capture taken before the column existed, and then **not
    /// compared** rather than compared against a zero.
    AngularVelocity,
    /// The collision stun timer, which gates thrust and lateral grip.
    StunTimer,
}

impl Field {
    /// Every field, in report order.
    pub const ALL: [Self; FIELD_COUNT] = [
        Self::Position,
        Self::Velocity,
        Self::Speed,
        Self::SpeedCached,
        Self::Row0,
        Self::Up,
        Self::Forward,
        Self::Grounded,
        Self::Throttle,
        Self::Brake,
        Self::Steer,
        Self::AirbrakeLeft,
        Self::AirbrakeRight,
        Self::AngularVelocity,
        Self::StunTimer,
    ];

    /// The field's name in a report.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Position => "position",
            Self::Velocity => "velocity",
            Self::Speed => "speed",
            Self::SpeedCached => "speed_cached",
            Self::Row0 => "orientation.row0",
            Self::Up => "orientation.up",
            Self::Forward => "orientation.forward",
            Self::Grounded => "grounded",
            Self::Throttle => "throttle",
            Self::Brake => "brake",
            Self::Steer => "steer",
            Self::AirbrakeLeft => "airbrake_l",
            Self::AirbrakeRight => "airbrake_r",
            Self::AngularVelocity => "angular_velocity",
            Self::StunTimer => "stun_timer",
        }
    }

    /// How this field is compared.
    #[must_use]
    pub fn quantity(self) -> Quantity {
        match self {
            Self::Position => Quantity::Position,
            Self::Velocity | Self::Speed | Self::SpeedCached => Quantity::Velocity,
            Self::Row0 | Self::Up | Self::Forward => Quantity::Orientation,
            Self::Grounded => Quantity::Discrete,
            Self::Throttle
            | Self::Brake
            | Self::Steer
            | Self::AirbrakeLeft
            | Self::AirbrakeRight => Quantity::Control,
            Self::AngularVelocity => Quantity::AngularVelocity,
            Self::StunTimer => Quantity::Timer,
        }
    }

    /// The unit the field's error is measured in.
    #[must_use]
    pub fn unit(self) -> &'static str {
        match self.quantity() {
            Quantity::Position => "units",
            Quantity::Orientation => "rad",
            Quantity::Velocity => "units/s",
            Quantity::AngularVelocity => "rad/s",
            Quantity::Timer => "s",
            Quantity::Control | Quantity::Discrete => "",
        }
    }
}

/// How many fields a tick is compared on.
///
/// "Compared on" is the upper bound rather than the count: a field whose column
/// is missing from either trace is skipped, and [`FieldSummary::compared_ticks`]
/// is how many ticks it actually got.
pub const FIELD_COUNT: usize = 15;

/// The tolerance table, from the verification protocol.
///
/// Provisional there and provisional here: they are what "below perceptibility
/// at track scale" was estimated to mean before any real measurement existed, so
/// they are data rather than constants, and the CLI can override each one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerances {
    /// Position passes at 0.01 units absolute **or** this relatively.
    pub position_absolute: f32,
    /// The relative half of the position tolerance.
    pub position_relative: f32,
    /// Angle between two basis axes, in radians.
    pub orientation_radians: f32,
    /// Velocity, relative.
    pub velocity_relative: f32,
    /// An absolute floor under the velocity comparison. **Zero by default**, so
    /// the default behaviour is exactly the protocol's.
    ///
    /// It exists because a purely relative test has no meaning when both
    /// magnitudes are near zero: on a standing start, a simulated `1e-9` against a
    /// recorded `0` is a relative error of 1, and reporting the first tick of
    /// every such scenario as the divergence would bury the real one. Raising this
    /// is a deliberate widening of the protocol and should be said out loud in
    /// whatever the comparison is quoted in.
    pub velocity_absolute: f32,
    /// A control state, relative.
    pub control_relative: f32,
    /// An absolute floor under the control comparison, on the `0..=100` scale.
    /// Zero by default, for the same reason as [`Self::velocity_absolute`].
    pub control_absolute: f32,
    /// Angular velocity, relative. The same `1e-3` linear velocity gets, for the
    /// same reason: it integrates into the attitude.
    pub angular_velocity_relative: f32,
    /// An absolute floor under the angular-velocity comparison, in rad/s.
    ///
    /// **Nonzero by default, unlike the other two floors**, and deliberately so.
    /// The captures show a yaw rate near `1.5` rad/s in a corner and *exactly*
    /// zero on a straight - a recorded `0` against a simulated `1e-9` is a
    /// relative error of one, so a purely relative test would report tick 0 of
    /// every straight-line capture as the divergence and bury the real one.
    ///
    /// `1e-4` rad/s. At the 60 Hz the captures run at that integrates to `1.7e-6`
    /// rad of attitude per tick, a sixtieth of [`Self::orientation_radians`], so
    /// it cannot hide an orientation divergence behind an angular-velocity one.
    pub angular_velocity_absolute: f32,
    /// A timer, absolute, in seconds. `1e-3`, about a sixteenth of a frame at
    /// 60 Hz: a timer that runs a whole frame long is a divergence, a timer that
    /// differs by the capture's own `%.7g` rounding is not.
    pub timer_absolute: f32,
}

impl Default for Tolerances {
    fn default() -> Self {
        Self {
            position_absolute: 0.01,
            position_relative: 1e-4,
            orientation_radians: 1e-4,
            velocity_relative: 1e-3,
            velocity_absolute: 0.0,
            control_relative: 1e-3,
            control_absolute: 0.0,
            angular_velocity_relative: 1e-3,
            angular_velocity_absolute: 1e-4,
            timer_absolute: 1e-3,
        }
    }
}

impl Tolerances {
    /// How a quantity's tolerance reads in a report.
    #[must_use]
    pub fn describe(&self, quantity: Quantity) -> String {
        match quantity {
            Quantity::Position => format!(
                "{} absolute or {:e} relative",
                self.position_absolute, self.position_relative
            ),
            Quantity::Orientation => format!("{:e} rad", self.orientation_radians),
            Quantity::Velocity => describe_relative(self.velocity_relative, self.velocity_absolute),
            Quantity::Control => describe_relative(self.control_relative, self.control_absolute),
            Quantity::AngularVelocity => describe_relative(
                self.angular_velocity_relative,
                self.angular_velocity_absolute,
            ),
            Quantity::Timer => format!("{:e} absolute", self.timer_absolute),
            Quantity::Discrete => "exact".to_owned(),
        }
    }

    fn exceeded(&self, quantity: Quantity, error: f32, relative: f32) -> bool {
        match quantity {
            Quantity::Position => {
                error > self.position_absolute && relative > self.position_relative
            }
            Quantity::Orientation => error > self.orientation_radians,
            Quantity::Velocity => {
                error > self.velocity_absolute && relative > self.velocity_relative
            }
            Quantity::Control => error > self.control_absolute && relative > self.control_relative,
            Quantity::AngularVelocity => {
                error > self.angular_velocity_absolute && relative > self.angular_velocity_relative
            }
            Quantity::Timer => error > self.timer_absolute,
            Quantity::Discrete => error != 0.0,
        }
    }
}

/// A field's unit with the space in front of it, or nothing at all for the
/// dimensionless ones. A trailing space in a report is the sort of thing that
/// gets copied into an issue and looks like a truncated line.
fn unit_suffix(field: Field) -> String {
    let unit = field.unit();
    if unit.is_empty() {
        String::new()
    } else {
        format!(" {unit}")
    }
}

fn describe_relative(relative: f32, absolute: f32) -> String {
    if absolute > 0.0 {
        format!("{absolute:e} absolute or {relative:e} relative")
    } else {
        format!("{relative:e} relative")
    }
}

/// What was compared, so a report can quote both sides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sampled {
    /// A single number from each trace.
    Scalar {
        /// From the recording.
        recorded: f32,
        /// From the simulation.
        simulated: f32,
    },
    /// A vector from each trace.
    Vector {
        /// From the recording.
        recorded: Vec3,
        /// From the simulation.
        simulated: Vec3,
    },
}

impl Sampled {
    /// Whether both sides are finite numbers.
    ///
    /// A NaN or an infinity is never "within tolerance": every tolerance test is
    /// a `>` comparison, and `NaN > x` is `false`, so a poisoned run would
    /// otherwise report perfect agreement on every field at once. That is the
    /// single most misleading thing this tool could say, and a tumbling ship or a
    /// degenerate quaternion is exactly what it will be pointed at.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Scalar {
                recorded,
                simulated,
            } => recorded.is_finite() && simulated.is_finite(),
            Self::Vector {
                recorded,
                simulated,
            } => recorded.is_finite() && simulated.is_finite(),
        }
    }
}

impl fmt::Display for Sampled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar {
                recorded,
                simulated,
            } => write!(f, "recorded {recorded}, simulated {simulated}"),
            Self::Vector {
                recorded,
                simulated,
            } => write!(
                f,
                "recorded ({:.6}, {:.6}, {:.6}), simulated ({:.6}, {:.6}, {:.6})",
                recorded.x, recorded.y, recorded.z, simulated.x, simulated.y, simulated.z
            ),
        }
    }
}

/// One field of one tick, compared.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Check {
    /// Which field.
    pub field: Field,
    /// Both sides of it.
    pub sampled: Sampled,
    /// The error in the field's own unit: a distance, an angle in radians, or a
    /// difference on the control scale.
    pub error: f32,
    /// The error over the larger of the two magnitudes, or zero when both are.
    pub relative: f32,
    /// Whether it is outside tolerance.
    pub exceeded: bool,
}

/// The first tick at which something left tolerance.
#[derive(Debug, Clone, PartialEq)]
pub struct Divergence {
    /// The tick number the recording gave it.
    pub tick: u64,
    /// Its index in the trace, which is what to seek to in the CSV.
    pub index: usize,
    /// The field that went first, and by how much.
    ///
    /// "First" is in [`Field::ALL`] order, which is arbitrary between fields that
    /// went on the *same* tick - so the rest of that tick's failures are in
    /// [`Self::also`] rather than discarded. Several fields going at once is
    /// itself a signal: one term wrong shows up in one field, a wrong initial
    /// condition in all of them.
    pub check: Check,
    /// Everything else that was outside tolerance on the same tick.
    pub also: Vec<Check>,
}

/// Whether a field's error is going anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendVerdict {
    /// Never left zero.
    Exact,
    /// Not systematically growing.
    Bounded,
    /// Growing over the run: a systematic error, even inside tolerance.
    Growing,
    /// Shrinking: a transient the run recovers from.
    Shrinking,
}

impl fmt::Display for TrendVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Exact => "exact",
            Self::Bounded => "bounded",
            Self::Growing => "growing",
            Self::Shrinking => "shrinking",
        })
    }
}

/// How much the last quarter's mean error must exceed the first quarter's before
/// the error counts as growing rather than noisy.
///
/// Two. A factor rather than a slope threshold because the slope's units are the
/// field's own, so no one number would suit an angle and a position at once.
pub const GROWTH_FACTOR: f64 = 2.0;

/// The shape of one field's error over the run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trend {
    /// Least-squares slope of the error against the tick index.
    pub slope_per_tick: f64,
    /// Mean error over the first quarter of the run.
    pub first_quarter_mean: f64,
    /// Mean error over the last quarter.
    pub last_quarter_mean: f64,
    /// What that adds up to.
    pub verdict: TrendVerdict,
}

/// Everything a run says about one field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldSummary {
    /// Which field.
    pub field: Field,
    /// How many ticks this field was actually compared on.
    ///
    /// Zero when neither trace carries the column - a capture taken before the
    /// angular-velocity column existed, or a field nothing on our side models.
    /// **Zero is not agreement**, and the report says "not compared" rather than
    /// showing a max error of zero, which would read as a perfect match.
    pub compared_ticks: usize,
    /// The largest error seen.
    pub max_error: f32,
    /// The tick it was seen at.
    pub max_error_tick: u64,
    /// The largest relative error seen.
    pub max_relative: f32,
    /// The first tick outside tolerance, if any.
    pub first_exceeded_tick: Option<u64>,
    /// Where the error is heading.
    pub trend: Trend,
}

/// The result of comparing a recording against a simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    /// Ticks in the recording.
    pub recorded_ticks: usize,
    /// Ticks in the simulated run.
    pub simulated_ticks: usize,
    /// How many were compared: the shorter of the two.
    pub compared_ticks: usize,
    /// The tolerances used.
    pub tolerances: Tolerances,
    /// The first tick outside tolerance in any field, if there is one.
    pub first_divergence: Option<Divergence>,
    /// The index at which the two traces' tick numbering stopped agreeing.
    ///
    /// Rows are paired by position, so a gap in one capture would otherwise
    /// silently compare tick 40 against tick 41 for the rest of the run.
    ///
    /// **This cannot see a dropped tick in a real capture.**
    /// `scripts/psp-trace.py` numbers its rows by counting breakpoint hits, so a
    /// hit that never arrived leaves no gap in the column to find. It catches an
    /// edited or spliced file, and that is all it claims to catch.
    pub first_misalignment: Option<usize>,
    /// One entry per field, in [`Field::ALL`] order.
    pub fields: [FieldSummary; FIELD_COUNT],
}

impl Comparison {
    /// Whether anything left tolerance.
    #[must_use]
    pub fn diverged(&self) -> bool {
        self.first_divergence.is_some()
    }

    /// The summary for one field.
    #[must_use]
    pub fn field(&self, field: Field) -> &FieldSummary {
        &self.fields[Field::ALL.iter().position(|f| *f == field).unwrap_or(0)]
    }
}

/// Compares one tick, field by field.
///
/// Public so a caller can compare a pair of frames without building a whole
/// trace - which is what the unit tests do.
#[must_use]
pub fn checks(
    recorded: &Frame,
    simulated: &Frame,
    tolerances: &Tolerances,
) -> [Option<Check>; FIELD_COUNT] {
    Field::ALL.map(|field| {
        // `None` is "neither trace carries this column", which is not a pass and
        // not a failure: it is the absence of a measurement, and it stays an
        // absence all the way into the report. See `FieldSummary::compared_ticks`.
        let (sampled, error) = measure(field, recorded, simulated)?;
        let relative = match sampled {
            Sampled::Scalar {
                recorded,
                simulated,
            } => relative_error(error, recorded.abs().max(simulated.abs())),
            Sampled::Vector {
                recorded,
                simulated,
            } => relative_error(error, recorded.length().max(simulated.length())),
        };
        // A non-finite value on either side is a divergence by definition, and
        // is tested before the tolerance rather than through it: see
        // [`Sampled::is_finite`].
        let finite = sampled.is_finite() && error.is_finite();
        Some(Check {
            field,
            sampled,
            error,
            relative,
            exceeded: !finite || tolerances.exceeded(field.quantity(), error, relative),
        })
    })
}

/// Both sides of a field and the error between them, or `None` when either side
/// does not carry the field at all.
///
/// Only the optional columns can answer `None`: a capture taken before the
/// angular-velocity column existed, or a field like `timer_2e0` that nothing in
/// `ShipState` models. Defaulting either to zero would compare a measurement
/// against an invention and could only ever produce a false agreement.
fn measure(field: Field, recorded: &Frame, simulated: &Frame) -> Option<(Sampled, f32)> {
    let vector = |recorded: Vec3, simulated: Vec3| {
        (
            Sampled::Vector {
                recorded,
                simulated,
            },
            (recorded - simulated).length(),
        )
    };
    let scalar = |recorded: f32, simulated: f32| {
        (
            Sampled::Scalar {
                recorded,
                simulated,
            },
            (recorded - simulated).abs(),
        )
    };
    let axis = |recorded: Vec3, simulated: Vec3| {
        (
            Sampled::Vector {
                recorded,
                simulated,
            },
            angle_between(recorded, simulated),
        )
    };

    Some(match field {
        Field::Position => vector(recorded.position, simulated.position),
        Field::Velocity => vector(recorded.velocity, simulated.velocity),
        Field::Speed => scalar(recorded.speed, simulated.speed),
        Field::SpeedCached => scalar(recorded.speed_cached, simulated.speed_cached),
        Field::Row0 => axis(recorded.row0, simulated.row0),
        Field::Up => axis(recorded.up, simulated.up),
        Field::Forward => axis(recorded.forward, simulated.forward),
        Field::Grounded => scalar(recorded.grounded, simulated.grounded),
        Field::Throttle => scalar(recorded.throttle, simulated.throttle),
        Field::Brake => scalar(recorded.brake, simulated.brake),
        Field::Steer => scalar(recorded.steer, simulated.steer),
        Field::AirbrakeLeft => scalar(recorded.airbrake_left, simulated.airbrake_left),
        Field::AirbrakeRight => scalar(recorded.airbrake_right, simulated.airbrake_right),
        Field::AngularVelocity => vector(recorded.angular_velocity?, simulated.angular_velocity?),
        Field::StunTimer => scalar(recorded.stun_timer?, simulated.stun_timer?),
    })
}

/// The angle between two axes, in radians.
///
/// `atan2(|a x b|, a.b)` rather than `acos(a.b)`, which loses all its precision
/// for nearly parallel vectors - which is the whole interesting range here, since
/// the tolerance is `1e-4` radians and `acos` near 1 cannot resolve that.
fn angle_between(a: Vec3, b: Vec3) -> f32 {
    // `normalize_or_zero` maps a NaN vector to zero, and the zero case below
    // returns an angle of nothing at all - which would turn a poisoned attitude
    // into perfect agreement. So non-finite is answered before normalising.
    if !a.is_finite() || !b.is_finite() {
        return f32::NAN;
    }
    let (a, b) = (a.normalize_or_zero(), b.normalize_or_zero());
    if a == Vec3::ZERO || b == Vec3::ZERO {
        return 0.0;
    }
    a.cross(b).length().atan2(a.dot(b))
}

fn relative_error(error: f32, scale: f32) -> f32 {
    if scale > 0.0 { error / scale } else { 0.0 }
}

/// Compares a recording against a simulated run of the same scenario.
///
/// Rows are paired by position, not by tick number: a simulated run is generated
/// from the recording and so starts where it starts. The tick numbers are still
/// checked against each other, and where they stop advancing together that is
/// reported as [`Comparison::first_misalignment`].
#[must_use]
pub fn compare(recorded: &Trace, simulated: &Trace, tolerances: &Tolerances) -> Comparison {
    let compared = recorded.len().min(simulated.len());
    let mut first_divergence = None;
    let mut first_misalignment = None;
    let mut errors: [Vec<f64>; FIELD_COUNT] = std::array::from_fn(|_| Vec::with_capacity(compared));
    // Two independent maxima. The largest error and the largest *relative* error
    // need not fall on the same tick, and the protocol judges velocity relatively:
    // folding them into one accumulator loses a slow tick's large relative error
    // the moment a fast tick sets a new absolute one.
    let mut max_error: [(f32, u64); FIELD_COUNT] = [(0.0, 0); FIELD_COUNT];
    let mut max_relative: [f32; FIELD_COUNT] = [0.0; FIELD_COUNT];
    let mut first_exceeded: [Option<u64>; FIELD_COUNT] = [None; FIELD_COUNT];
    let mut compared_fields: [usize; FIELD_COUNT] = [0; FIELD_COUNT];

    for index in 0..compared {
        let recorded_frame = &recorded.frames[index];
        let simulated_frame = &simulated.frames[index];

        if first_misalignment.is_none() {
            let recorded_offset = recorded_frame.tick.wrapping_sub(recorded.frames[0].tick);
            let simulated_offset = simulated_frame.tick.wrapping_sub(simulated.frames[0].tick);
            if recorded_offset != simulated_offset {
                first_misalignment = Some(index);
            }
        }

        for (slot, check) in checks(recorded_frame, simulated_frame, tolerances)
            .into_iter()
            .enumerate()
        {
            let Some(check) = check else { continue };
            compared_fields[slot] += 1;
            errors[slot].push(f64::from(check.error));
            if exceeds(check.error, max_error[slot].0) {
                max_error[slot] = (check.error, recorded_frame.tick);
            }
            if exceeds(check.relative, max_relative[slot]) {
                max_relative[slot] = check.relative;
            }
            if check.exceeded {
                first_exceeded[slot].get_or_insert(recorded_frame.tick);
                match &mut first_divergence {
                    None => {
                        first_divergence = Some(Divergence {
                            tick: recorded_frame.tick,
                            index,
                            check,
                            also: Vec::new(),
                        });
                    }
                    Some(divergence) if divergence.index == index => {
                        divergence.also.push(check);
                    }
                    Some(_) => {}
                }
            }
        }
    }

    let fields = std::array::from_fn(|slot| FieldSummary {
        field: Field::ALL[slot],
        compared_ticks: compared_fields[slot],
        max_error: max_error[slot].0,
        max_error_tick: max_error[slot].1,
        max_relative: max_relative[slot],
        first_exceeded_tick: first_exceeded[slot],
        trend: trend(&errors[slot]),
    });

    Comparison {
        recorded_ticks: recorded.len(),
        simulated_ticks: simulated.len(),
        compared_ticks: compared,
        tolerances: *tolerances,
        first_divergence,
        first_misalignment,
        fields,
    }
}

/// Whether `candidate` should replace `current` as the largest seen.
///
/// A non-finite candidate beats any finite one, so a NaN cannot hide behind a
/// comparison that is false for every NaN.
fn exceeds(candidate: f32, current: f32) -> bool {
    candidate > current || (!candidate.is_finite() && current.is_finite())
}

/// Where a series of per-tick errors is heading.
///
/// A least-squares slope *and* a first-quarter to last-quarter comparison,
/// because neither alone is enough: a slope says nothing about scale, and two
/// quarter means say nothing about what happened between them. The verdict comes
/// from the quarters and the slope's sign together.
#[must_use]
pub fn trend(errors: &[f64]) -> Trend {
    let n = errors.len();
    if n == 0 || errors.iter().all(|e| *e == 0.0) {
        return Trend {
            slope_per_tick: 0.0,
            first_quarter_mean: 0.0,
            last_quarter_mean: 0.0,
            verdict: TrendVerdict::Exact,
        };
    }

    let count = n as f64;
    let mean_x = (count - 1.0) / 2.0;
    let mean_y = errors.iter().sum::<f64>() / count;
    let mut covariance = 0.0;
    let mut variance = 0.0;
    for (index, error) in errors.iter().enumerate() {
        let dx = index as f64 - mean_x;
        covariance += dx * (error - mean_y);
        variance += dx * dx;
    }
    let slope = if variance > 0.0 {
        covariance / variance
    } else {
        0.0
    };

    // A quarter, floored at one sample, so a short run still says something.
    let quarter = (n / 4).max(1);
    let first_quarter_mean = errors[..quarter].iter().sum::<f64>() / quarter as f64;
    let last_quarter_mean = errors[n - quarter..].iter().sum::<f64>() / quarter as f64;

    let verdict = if last_quarter_mean > first_quarter_mean * GROWTH_FACTOR && slope > 0.0 {
        TrendVerdict::Growing
    } else if first_quarter_mean > last_quarter_mean * GROWTH_FACTOR && slope < 0.0 {
        TrendVerdict::Shrinking
    } else {
        TrendVerdict::Bounded
    };

    Trend {
        slope_per_tick: slope,
        first_quarter_mean,
        last_quarter_mean,
        verdict,
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "compared {} tick(s) ({} recorded, {} simulated)",
            self.compared_ticks, self.recorded_ticks, self.simulated_ticks
        )?;
        if self.recorded_ticks != self.simulated_ticks {
            writeln!(
                f,
                "note: the two runs are different lengths; the tail of the longer one was not compared"
            )?;
        }
        if let Some(index) = self.first_misalignment {
            writeln!(
                f,
                "note: tick numbering stops agreeing at row {index}; rows are paired by position, \
                 so everything after that compares two different ticks"
            )?;
        }

        match &self.first_divergence {
            None => writeln!(
                f,
                "\nno divergence: every field stayed inside tolerance for all {} tick(s)",
                self.compared_ticks
            )?,
            Some(divergence) => {
                let check = divergence.check;
                writeln!(
                    f,
                    "\nfirst divergence at tick {} (row {}), field {}",
                    divergence.tick,
                    divergence.index,
                    check.field.as_str()
                )?;
                writeln!(f, "  {}", check.sampled)?;
                writeln!(
                    f,
                    "  error {:.6}{} ({:.3e} relative), tolerance {}",
                    check.error,
                    unit_suffix(check.field),
                    check.relative,
                    self.tolerances.describe(check.field.quantity())
                )?;
                if !divergence.also.is_empty() {
                    writeln!(
                        f,
                        "  and on the same tick: {}",
                        divergence
                            .also
                            .iter()
                            .map(|other| {
                                format!(
                                    "{} by {:.6}{}",
                                    other.field.as_str(),
                                    other.error,
                                    unit_suffix(other.field)
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    )?;
                }
            }
        }

        writeln!(
            f,
            "\n{:<20} {:>12} {:>9} {:>12} {:>9}  trend",
            "field", "max error", "at tick", "max rel", "exceeded"
        )?;
        for summary in &self.fields {
            // A field neither trace carries reads as "not compared", never as a
            // max error of zero: the second is what a perfect match looks like,
            // and a comparison that never ran must not be able to claim one.
            if summary.compared_ticks == 0 {
                writeln!(
                    f,
                    "{:<20} {:>12} {:>9} {:>12} {:>9}  not compared (no column in one \
                     or both traces)",
                    summary.field.as_str(),
                    "-",
                    "-",
                    "-",
                    "-"
                )?;
                continue;
            }
            let exceeded = summary
                .first_exceeded_tick
                .map_or_else(|| "-".to_owned(), |tick| tick.to_string());
            let partial = if summary.compared_ticks == self.compared_ticks {
                String::new()
            } else {
                format!(
                    " [{}/{} tick(s)]",
                    summary.compared_ticks, self.compared_ticks
                )
            };
            writeln!(
                f,
                "{:<20} {:>12.3e} {:>9} {:>12.3e} {:>9}  {} (slope {:.3e}/tick, {:.3e} -> {:.3e}){}",
                summary.field.as_str(),
                summary.max_error,
                summary.max_error_tick,
                summary.max_relative,
                exceeded,
                summary.trend.verdict,
                summary.trend.slope_per_tick,
                summary.trend.first_quarter_mean,
                summary.trend.last_quarter_mean,
                partial
            )?;
        }
        write!(
            f,
            "\ntolerances: position {}, orientation {}, velocity {}, angular velocity {}, \
             control {}, timer {}, discrete {}",
            self.tolerances.describe(Quantity::Position),
            self.tolerances.describe(Quantity::Orientation),
            self.tolerances.describe(Quantity::Velocity),
            self.tolerances.describe(Quantity::AngularVelocity),
            self.tolerances.describe(Quantity::Control),
            self.tolerances.describe(Quantity::Timer),
            self.tolerances.describe(Quantity::Discrete)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::Trace;

    /// A capture-shaped fixture with `ticks` rows, where the ship travels along
    /// `+z` at a constant speed. Hand-authored: a real trace is derived game data
    /// and can never be committed, and inventing one that claimed to be a
    /// recording would be worse than having none.
    fn fixture(ticks: usize, mutate: impl Fn(usize, &mut [f32; 3])) -> Trace {
        let mut csv = String::from(
            "tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
             right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
             pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed\n",
        );
        for tick in 0..ticks {
            let mut position = [10.0, 2.5, -30.0 + tick as f32 * 0.366_99];
            mutate(tick, &mut position);
            csv.push_str(&format!(
                "{tick},0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,{},{},{},0,0,22,22\n",
                position[0], position[1], position[2]
            ));
        }
        Trace::parse(&csv).unwrap()
    }

    /// The recorded yaw rate the captures show when the stick is held over:
    /// `+1.51 rad/s` about row 1 on 199 of 199 ticks,
    /// `docs/ghidra/functions/psp-pulse/engine.md`.
    const RECORDED_YAW_RATE: f32 = 1.51;

    /// What `YAW_DRIVE_CALIBRATION` stands in for: the recovered steering law is
    /// verified at instruction level and predicts a yaw rate about 22x too high,
    /// so a run without the calibration turns 22 times too fast. The open
    /// question this column was added to move, `HANDOVER.md`.
    const YAW_AUTHORITY_ERROR: f32 = 22.1;

    /// The same fixture with an angular velocity of `rate` about the up axis on
    /// every tick. Hand-authored like everything else here.
    fn turning(ticks: usize, rate: f32) -> Trace {
        let mut trace = fixture(ticks, |_, _| {});
        for frame in &mut trace.frames {
            frame.angular_velocity = Some(Vec3::new(0.0, rate, 0.0));
            frame.stun_timer = Some(0.0);
        }
        trace
    }

    /// **The measurement this column was added for.** A run whose yaw authority is
    /// 22x off must be reported as a divergence in the angular velocity, on tick
    /// 0, rather than only showing up ticks later as an accumulated attitude
    /// error - which is what the harness could see before and why the yaw
    /// question could not move.
    #[test]
    fn a_yaw_rate_the_size_of_the_open_yaw_authority_error_diverges() {
        let recorded = turning(40, RECORDED_YAW_RATE);
        let simulated = turning(40, RECORDED_YAW_RATE / YAW_AUTHORITY_ERROR);
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        let divergence = comparison.first_divergence.clone().expect("must diverge");
        assert_eq!(divergence.tick, 0);
        let angular = comparison.field(Field::AngularVelocity);
        assert_eq!(angular.first_exceeded_tick, Some(0));
        assert_eq!(angular.compared_ticks, 40);
        assert!(
            (angular.max_error - RECORDED_YAW_RATE * (1.0 - 1.0 / YAW_AUTHORITY_ERROR)).abs()
                < 1e-3,
            "{}",
            angular.max_error
        );
    }

    /// The other half of the tolerance choice: a straight-line capture records an
    /// angular velocity of *exactly* zero, so a purely relative test would call
    /// any simulated dust a relative error of one and report tick 0 of every
    /// straight capture as the divergence. The absolute floor is what stops it.
    #[test]
    fn a_straight_line_is_not_diverged_by_dust_around_a_recorded_zero() {
        let recorded = turning(40, 0.0);
        let mut simulated = recorded.clone();
        for frame in &mut simulated.frames {
            frame.angular_velocity = Some(Vec3::new(1e-9, -2e-9, 1e-9));
        }
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert!(!comparison.diverged(), "{comparison}");

        // And the floor is low enough to still catch a real rotation: a
        // hundredth of the recorded turn rate is 1.5e-2, well over 1e-4.
        for frame in &mut simulated.frames {
            frame.angular_velocity = Some(Vec3::new(0.0, RECORDED_YAW_RATE / 100.0, 0.0));
        }
        assert!(compare(&recorded, &simulated, &Tolerances::default()).diverged());
    }

    /// An absent column is the absence of a measurement. It must not be compared
    /// against a zero, and it must not be reported as a field that agreed.
    #[test]
    fn an_absent_column_is_not_compared_rather_than_agreed() {
        // The recording predates the column; the simulated run always has one.
        let recorded = fixture(40, |_, _| {});
        let simulated = turning(40, RECORDED_YAW_RATE);
        assert_eq!(recorded.frames[0].angular_velocity, None);

        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        let angular = comparison.field(Field::AngularVelocity);
        assert_eq!(angular.compared_ticks, 0);
        assert_eq!(angular.first_exceeded_tick, None);
        assert_eq!(angular.max_error, 0.0, "nothing was measured");
        assert!(!comparison.diverged(), "{comparison}");

        // And the report says so out loud, because a max error of zero on its own
        // reads exactly like a perfect match.
        let report = comparison.to_string();
        assert!(report.contains("angular_velocity"), "{report}");
        assert!(report.contains("not compared"), "{report}");
    }

    /// Every field a legacy capture *does* carry is compared exactly as before,
    /// which is the whole backwards-compatibility claim.
    #[test]
    fn a_legacy_capture_still_compares_on_every_field_it_carries() {
        let recorded = fixture(40, |_, _| {});
        let comparison = compare(&recorded, &recorded, &Tolerances::default());
        for summary in &comparison.fields {
            match summary.field {
                Field::AngularVelocity | Field::StunTimer => {
                    assert_eq!(summary.compared_ticks, 0, "{:?}", summary.field);
                }
                other => assert_eq!(summary.compared_ticks, 40, "{other:?}"),
            }
        }
        assert!(!comparison.diverged());
    }

    /// The stun timer gates thrust and lateral grip, so a run that fails to arm
    /// it is a run producing thrust the original did not - the force-balance
    /// question in `HANDOVER.md`. Half a second of it is 500x the tolerance.
    #[test]
    fn a_stun_the_simulation_missed_is_a_divergence() {
        let mut recorded = turning(8, 0.0);
        let simulated = recorded.clone();
        for frame in &mut recorded.frames[3..] {
            frame.stun_timer = Some(0.5);
        }
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        let divergence = comparison.first_divergence.clone().expect("must diverge");
        assert_eq!(divergence.check.field, Field::StunTimer);
        assert_eq!(divergence.tick, 3);
        assert_eq!(comparison.field(Field::StunTimer).compared_ticks, 8);
    }

    #[test]
    fn identical_traces_do_not_diverge() {
        let trace = fixture(40, |_, _| {});
        let comparison = compare(&trace, &trace, &Tolerances::default());
        assert!(!comparison.diverged());
        assert_eq!(comparison.compared_ticks, 40);
        assert_eq!(comparison.field(Field::Position).max_error, 0.0);
        assert_eq!(
            comparison.field(Field::Position).trend.verdict,
            TrendVerdict::Exact
        );
    }

    #[test]
    fn a_difference_inside_tolerance_is_not_reported() {
        let recorded = fixture(40, |_, _| {});
        // 0.005 units, under the 0.01 absolute band, on every tick.
        let simulated = fixture(40, |_, position| position[0] += 0.005);
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert!(!comparison.diverged(), "{comparison}");
        let summary = comparison.field(Field::Position);
        assert!((summary.max_error - 0.005).abs() < 1e-6);
        assert_eq!(summary.first_exceeded_tick, None);
    }

    #[test]
    fn a_difference_beyond_tolerance_is_reported_at_the_right_tick() {
        let recorded = fixture(40, |_, _| {});
        let simulated = fixture(40, |tick, position| {
            if tick >= 7 {
                position[0] += 0.25;
            }
        });
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        let divergence = comparison.first_divergence.clone().expect("must diverge");
        assert_eq!(divergence.tick, 7);
        assert_eq!(divergence.index, 7);
        assert_eq!(divergence.check.field, Field::Position);
        assert!((divergence.check.error - 0.25).abs() < 1e-5);
        assert_eq!(
            comparison.field(Field::Position).first_exceeded_tick,
            Some(7)
        );
        // Everything else agreed, so nothing else may claim a divergence.
        assert!(
            comparison
                .fields
                .iter()
                .filter(|s| s.field != Field::Position)
                .all(|s| s.first_exceeded_tick.is_none())
        );
    }

    /// The protocol asks for the trend explicitly: an error that grows every tick
    /// is systematic even while it is still inside tolerance.
    #[test]
    fn a_growing_error_inside_tolerance_is_still_reported_as_growing() {
        let recorded = fixture(40, |_, _| {});
        let simulated = fixture(40, |tick, position| {
            position[0] += tick as f32 * 0.000_2;
        });
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert!(!comparison.diverged(), "0.0078 max, inside 0.01 absolute");
        let trend = comparison.field(Field::Position).trend;
        assert_eq!(trend.verdict, TrendVerdict::Growing);
        assert!(trend.slope_per_tick > 0.0);
        assert!(trend.last_quarter_mean > trend.first_quarter_mean);
    }

    #[test]
    fn a_constant_offset_is_bounded_not_growing() {
        let recorded = fixture(40, |_, _| {});
        let simulated = fixture(40, |_, position| position[0] += 0.005);
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert_eq!(
            comparison.field(Field::Position).trend.verdict,
            TrendVerdict::Bounded
        );
    }

    #[test]
    fn a_decaying_error_is_reported_as_shrinking() {
        let recorded = fixture(40, |_, _| {});
        let simulated = fixture(40, |tick, position| {
            position[0] += 0.005 / (1.0 + tick as f32);
        });
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert_eq!(
            comparison.field(Field::Position).trend.verdict,
            TrendVerdict::Shrinking
        );
    }

    /// Groundedness is quantised, so any difference at all is a bug: it is the
    /// one field where "close enough" has no meaning.
    #[test]
    fn a_discrete_field_has_no_slack() {
        let mut recorded = fixture(4, |_, _| {});
        let mut simulated = recorded.clone();
        simulated.frames[2].grounded = 0.5;
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        let divergence = comparison.first_divergence.clone().expect("must diverge");
        assert_eq!(divergence.check.field, Field::Grounded);
        assert_eq!(divergence.tick, 2);

        // And the smallest representable difference is still a difference.
        recorded.frames[1].grounded = 1.0;
        simulated.frames[1].grounded = 1.0 + f32::EPSILON;
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert_eq!(comparison.first_divergence.expect("must diverge").tick, 1);
    }

    #[test]
    fn orientation_is_compared_as_an_angle_in_radians() {
        let mut recorded = fixture(4, |_, _| {});
        let mut simulated = recorded.clone();
        // A milliradian of yaw, ten times the 1e-4 tolerance.
        let angle = 1e-3f32;
        for frame in &mut simulated.frames {
            frame.forward = Vec3::new(angle.sin(), 0.0, angle.cos());
            frame.row0 = Vec3::new(angle.cos(), 0.0, -angle.sin());
        }
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        let divergence = comparison.first_divergence.expect("must diverge");
        assert!(matches!(
            divergence.check.field,
            Field::Row0 | Field::Forward
        ));
        assert!((divergence.check.error - angle).abs() < 1e-6);

        // Half the tolerance is not a divergence.
        for frame in &mut simulated.frames {
            let small = 5e-5f32;
            frame.forward = Vec3::new(small.sin(), 0.0, small.cos());
            frame.row0 = Vec3::new(small.cos(), 0.0, -small.sin());
        }
        recorded.frames.iter_mut().for_each(|frame| {
            frame.forward = Vec3::Z;
            frame.row0 = Vec3::X;
        });
        assert!(!compare(&recorded, &simulated, &Tolerances::default()).diverged());
    }

    #[test]
    fn velocity_is_relative_so_a_fast_ship_gets_more_slack_than_a_slow_one() {
        let mut recorded = fixture(4, |_, _| {});
        let mut simulated = recorded.clone();
        for frame in &mut simulated.frames {
            // 0.01 on 22 units/s is 4.5e-4 relative: inside 1e-3.
            frame.velocity.z += 0.01;
            frame.speed += 0.01;
        }
        assert!(!compare(&recorded, &simulated, &Tolerances::default()).diverged());

        for frame in &mut recorded.frames {
            frame.velocity = Vec3::new(0.0, 0.0, 1.0);
            frame.speed = 1.0;
        }
        for frame in &mut simulated.frames {
            frame.velocity = Vec3::new(0.0, 0.0, 1.01);
            frame.speed = 1.01;
        }
        // The same absolute error on 1 unit/s is 1e-2 relative: outside it.
        assert!(compare(&recorded, &simulated, &Tolerances::default()).diverged());
    }

    /// Every tolerance test is a `>`, and `NaN > x` is false, so a poisoned run
    /// would report perfect agreement on every field at once - the single most
    /// misleading thing this tool could say. A tumbling ship is exactly what it
    /// will be pointed at, so this is pinned per quantity.
    #[test]
    fn a_non_finite_value_is_never_within_tolerance() {
        for poison in [f32::NAN, f32::INFINITY] {
            let recorded = fixture(4, |_, _| {});

            let mut simulated = recorded.clone();
            simulated.frames[2].position.x = poison;
            let comparison = compare(&recorded, &simulated, &Tolerances::default());
            let divergence = comparison
                .first_divergence
                .clone()
                .unwrap_or_else(|| panic!("{poison} position must diverge"));
            assert_eq!(divergence.tick, 2);
            assert_eq!(divergence.check.field, Field::Position);
            assert!(!comparison.field(Field::Position).max_error.is_finite());

            // An attitude, where the normalising in `angle_between` would
            // otherwise launder a NaN into an angle of zero.
            let mut simulated = recorded.clone();
            simulated.frames[1].forward = Vec3::new(poison, poison, poison);
            let comparison = compare(&recorded, &simulated, &Tolerances::default());
            let divergence = comparison
                .first_divergence
                .unwrap_or_else(|| panic!("{poison} attitude must diverge"));
            assert_eq!(divergence.tick, 1);
            assert_eq!(divergence.check.field, Field::Forward);

            // And a velocity, which is judged purely relatively.
            let mut simulated = recorded.clone();
            simulated.frames[3].speed = poison;
            let comparison = compare(&recorded, &simulated, &Tolerances::default());
            assert_eq!(
                comparison
                    .first_divergence
                    .unwrap_or_else(|| panic!("{poison} speed must diverge"))
                    .tick,
                3
            );
        }
    }

    /// The largest error and the largest *relative* error need not fall on the
    /// same tick, and the protocol judges velocity relatively - so a slow tick's
    /// large relative error must survive a fast tick's large absolute one.
    #[test]
    fn the_largest_relative_error_is_not_lost_to_a_later_absolute_one() {
        let mut recorded = fixture(6, |_, _| {});
        let mut simulated = recorded.clone();
        for frame in recorded.frames.iter_mut().chain(&mut simulated.frames) {
            frame.velocity = Vec3::ZERO;
            frame.speed = 0.0;
        }
        // Tick 1: 0.01 against 0.02 - half a unit of relative error, a hundredth
        // of one absolute.
        recorded.frames[1].speed = 0.01;
        simulated.frames[1].speed = 0.02;
        // Tick 5: fifty times the absolute error, a fiftieth of the relative one.
        recorded.frames[5].speed = 22.0;
        simulated.frames[5].speed = 22.5;

        let summary = *compare(&recorded, &simulated, &Tolerances::default()).field(Field::Speed);
        assert_eq!(
            summary.max_error_tick, 5,
            "the biggest error is the late one"
        );
        assert!((summary.max_error - 0.5).abs() < 1e-6);
        assert!(
            (summary.max_relative - 0.5).abs() < 1e-6,
            "the early tick's relative error survived: {}",
            summary.max_relative
        );
    }

    #[test]
    fn traces_of_different_lengths_compare_the_shared_prefix() {
        let recorded = fixture(40, |_, _| {});
        let simulated = fixture(10, |_, _| {});
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert_eq!(comparison.compared_ticks, 10);
        assert_eq!(comparison.recorded_ticks, 40);
        assert!(!comparison.diverged());
    }

    #[test]
    fn a_gap_in_one_trace_is_reported_as_misalignment() {
        let recorded = fixture(8, |_, _| {});
        let mut simulated = recorded.clone();
        for frame in &mut simulated.frames[4..] {
            frame.tick += 1;
        }
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert_eq!(comparison.first_misalignment, Some(4));
    }

    #[test]
    fn the_report_names_the_tick_the_field_and_the_tolerance() {
        let recorded = fixture(12, |_, _| {});
        let simulated = fixture(12, |tick, position| {
            if tick >= 3 {
                position[2] += 1.0;
            }
        });
        let report = compare(&recorded, &simulated, &Tolerances::default()).to_string();
        assert!(report.contains("first divergence at tick 3"), "{report}");
        assert!(report.contains("position"), "{report}");
        assert!(report.contains("0.01 absolute"), "{report}");
    }
}
