//! What `Trace::summary` reports: clean-flight and angular-velocity
//! statistics off a capture.
//!
//! Split out of `trace.rs` when adding the camera fov columns pushed it past
//! its size ratchet (`scripts/check-file-size.py`), the same reason
//! `flare.rs` exists - this group is self-contained and [`super::Trace`]
//! re-exports every name.

use std::fmt;

use oag_core::math::Vec3;

/// How far `speed / |velocity|` may sit from `1.0` and still count as clean
/// (free) flight.
///
/// `speed` and `|velocity|` agree to `1e-6` in free flight - the captures write
/// seven significant digits - so this is a wide margin rather than a tight fit,
/// matching the `1e-4` already used ad hoc in
/// `crates/trace/tests/wall_contact_ground_truth.rs` before this existed.
pub const CLEAN_TOLERANCE: f32 = 1e-4;

/// Below this speed, `speed / |velocity|` stops measuring contact at all and
/// starts measuring rounding.
///
/// The normal impulse dominates at low speed and restitution can push
/// `|velocity|` above `speed` outright, which is what produced a `-380 %`
/// "friction" reading on a craft wedged at 0.15 units/s on the lap capture -
/// see `HANDOVER.md`. Split two ways in [`Summary`], deliberately: it does
/// **not** change [`Summary::clean_ticks`], which counts every tick with no
/// floor to match the fraction already on record, but it **does** stop a tick
/// below it from being named by [`Summary::first_contact`] - at exactly zero
/// velocity the ratio is `0/0` (`NaN`), and a fresh capture starting from a
/// dead stop would otherwise report tick 0 as the first contact on every run.
pub const UNMEASURABLE_SPEED: f32 = 1.0;

/// How far `cross(row0, up)` may sit from `forward` and still count as a
/// positively oriented basis.
///
/// The capture writes seven significant digits, so an exactly orthonormal basis
/// arrives with about `1e-6` of rounding in it.
pub const BASIS_TOLERANCE: f32 = 1e-4;

/// What the recorded angular-velocity column means.
///
/// Two independent binary questions, so four readings, and **neither is settled
/// by static reading**:
///
/// - **Sign.** `docs/ghidra/functions/ps2-pulse-eu/craft-update.md` derives
///   `w_game = -w_physics` from the PS2 integrator on three independent legs, so
///   the engine's angular velocity is the negative of the physical one. That is a
///   result about the *accumulators*; whether the stored velocity carries the
///   same convention is the obvious reading and not a measured one.
/// - **Frame.** The same page names `body+0x160` `angularVelocityLocal` and then
///   lists "which frame `body+0x150` and `body+0x160` are each expressed in" as
///   unresolved, and `docs/ghidra/functions/psp-pulse-usa/engine.md` caps the
///   local/world split of the angular accumulators at confidence 74.
///
/// So this is a switch, for the same reason [`crate::replay::Basis`] is: it is
/// exactly the kind of finding a capture exists to settle rather than a constant
/// to be chosen. Unlike the basis question it needs **no simulation at all** -
/// the recorded rows differentiate into an angular velocity of their own, and
/// [`Summary::angular_readings`] scores all four against it.
///
/// The local reading takes the recorded rows to be the body's axes in world
/// coordinates, which is what the rest of this crate already reads them as; it is
/// independent of [`crate::replay::Basis`], which is about mapping those rows
/// onto [`oag_physics::Body`]'s own axes rather than about the game's frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AngularReading {
    /// The column is the physical angular velocity, in world space.
    World,
    /// Body-local components, negated: `w_game = -w_physics` applied to the
    /// column the PS2 page calls `angularVelocityLocal`. The reading the sign
    /// result and the name point at together, and so the default.
    #[default]
    NegatedLocal,
    /// Body-local components, in the physical sign.
    Local,
    /// World space, negated.
    NegatedWorld,
}

impl AngularReading {
    /// Every reading, in report order.
    pub const ALL: [Self; 4] = [
        Self::NegatedLocal,
        Self::Local,
        Self::NegatedWorld,
        Self::World,
    ];

    /// The reading's name in a report.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::NegatedWorld => "negated-world",
            Self::Local => "local",
            Self::NegatedLocal => "negated-local",
        }
    }

    /// The physical, world-space angular velocity a recorded column describes,
    /// given the recorded basis rows of the same tick.
    #[must_use]
    pub fn to_world(self, recorded: Vec3, rows: (Vec3, Vec3, Vec3)) -> Vec3 {
        let (row0, row1, row2) = rows;
        match self {
            Self::World => recorded,
            Self::NegatedWorld => -recorded,
            Self::Local => row0 * recorded.x + row1 * recorded.y + row2 * recorded.z,
            Self::NegatedLocal => -(row0 * recorded.x + row1 * recorded.y + row2 * recorded.z),
        }
    }

    /// The column a physical, world-space angular velocity would be recorded as.
    /// The inverse of [`Self::to_world`] for an orthonormal basis.
    #[must_use]
    pub fn to_recorded(self, world: Vec3, rows: (Vec3, Vec3, Vec3)) -> Vec3 {
        let (row0, row1, row2) = rows;
        let local = || Vec3::new(world.dot(row0), world.dot(row1), world.dot(row2));
        match self {
            Self::World => world,
            Self::NegatedWorld => -world,
            Self::Local => local(),
            Self::NegatedLocal => -local(),
        }
    }
}

/// How well one reading of the angular-velocity column matches the recorded
/// basis's own rotation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AngularFit {
    /// The reading scored.
    pub reading: AngularReading,
    /// Root-mean-square difference between the recorded column and what the
    /// basis derivative says it should hold, in rad/s.
    pub rms_error: f32,
}

/// What [`super::Trace::summary`] found.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Summary {
    /// Ticks in the trace.
    pub ticks: usize,
    /// Smallest frame delta.
    pub dt_min: f32,
    /// Largest frame delta.
    pub dt_max: f32,
    /// Mean frame delta.
    pub dt_mean: f32,
    /// Slowest recorded speed.
    pub speed_min: f32,
    /// Fastest recorded speed.
    pub speed_max: f32,
    /// Distance travelled along the path, summed tick to tick.
    pub path_length: f32,
    /// Straight-line distance from the first position to the last.
    pub displacement: f32,
    /// How many ticks satisfy `cross(row0, up) = forward`.
    pub positively_oriented: usize,
    /// How many ticks carry an angular velocity. Zero for a capture taken before
    /// the column existed, which is not an error - see [`super::REQUIRED_COLUMNS`].
    pub angular_ticks: usize,
    /// Root-mean-square magnitude of the recorded angular velocity, in rad/s.
    /// The scale the fits below are to be read against: an `rms_error` of 0.01
    /// means nothing without knowing whether the signal is 1.5 or 0.01.
    pub angular_rms: f32,
    /// Every reading of the column scored against the recorded basis's own
    /// rotation, best first, or `None` when the trace has no angular velocity or
    /// is too short to differentiate.
    ///
    /// **This is the measurement that settles [`AngularReading`]**, and it needs
    /// no simulation: an orthonormal basis recorded on two consecutive ticks
    /// determines the rotation between them, so the column can be checked against
    /// the rows that sit beside it in the same file. A capture where one reading
    /// scores far below the other three has answered both open questions at once.
    pub angular_readings: Option<[AngularFit; 4]>,
    /// How many ticks were inside the collision-stun window, where the original
    /// produces no thrust and no lateral grip. `None` when the capture has no
    /// `stun_timer` column.
    ///
    /// The headline number for the force-balance question: a straight-line
    /// capture with a nonzero count here is a *stunned* ship coasting, and its
    /// speed is not an equilibrium of the engine force law at all.
    pub stunned_ticks: Option<usize>,
    /// How many ticks had the second engine gate at `craft+0x2e0` above zero.
    pub timer_2e0_ticks: Option<usize>,
    /// How many ticks pass the free-flight contact detector:
    /// `speed / |velocity|` within [`CLEAN_TOLERANCE`] of `1.0`.
    ///
    /// Counted over **every** tick with no speed floor, matching how the
    /// **32.7 %**/**171** figures in `HANDOVER.md` for the reference lap
    /// capture were first read by hand - reproduced exactly
    /// (`1029/3146`, first contact `171`) once this existed, which is what
    /// pins the convention here rather than the floor-excluding one
    /// [`Self::unmeasurable_ticks`] might suggest. The working rule this
    /// exists to make mechanical: *"Check a capture is clean before fitting
    /// anything to it"* - `speed` and `|velocity|` agree to `1e-6` in free
    /// flight, so a mismatch is a free contact detector. Two reference
    /// captures once stood as the M4 blocker for a session and a half because
    /// nobody ran this check.
    pub clean_ticks: usize,
    /// The first tick that fails the clean test at or above
    /// [`UNMEASURABLE_SPEED`], or `None` if every such tick passes.
    ///
    /// A tick below the floor is skipped when *searching* for this - it still
    /// contributes to [`Self::clean_ticks`] above, which counts every tick
    /// with no floor, but it cannot be the tick this field names. Two reasons:
    /// at exactly zero velocity the ratio is `0/0` (`NaN`, which never compares
    /// clean and would otherwise poison this field with tick 0 on any capture
    /// that starts from a dead stop), and just above zero it is real but
    /// unreliable in the same way `unmeasurable_ticks` describes. Verified
    /// against both anchors this module reproduces: neither the lap capture's
    /// **171** nor the standing start's **186** moves, because in both files
    /// nothing below the floor precedes the real first contact.
    pub first_contact: Option<usize>,
    /// How many ticks sit below [`UNMEASURABLE_SPEED`].
    ///
    /// Excluded from being **named** by [`Self::first_contact`] (see its docs)
    /// but **not excluded** from [`Self::clean_ticks`]'s count, which counts
    /// every tick with no floor - folding the floor into that fraction would
    /// produce a different number from the one already on record.
    ///
    /// Below a few units per second the ratio stops measuring contact at all:
    /// the normal impulse dominates and restitution can push `|velocity|`
    /// above `speed` outright, which is what produced a `-380 %` "friction"
    /// reading on a craft wedged at 0.15 units/s on the lap capture. A high
    /// count here alongside a low `clean_ticks` fraction is a hint to look at
    /// `speed_min` before trusting the headline number, not a reason to
    /// recompute it with a floor.
    pub unmeasurable_ticks: usize,
}

impl Summary {
    /// The reading of the angular-velocity column that best matches the recorded
    /// basis's own rotation.
    #[must_use]
    pub fn best_angular_reading(&self) -> Option<AngularFit> {
        self.angular_readings.map(|fits| fits[0])
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} tick(s)", self.ticks)?;
        writeln!(
            f,
            "dt: min {:.6}, mean {:.6} ({:.2} Hz), max {:.6}",
            self.dt_min,
            self.dt_mean,
            if self.dt_mean > 0.0 {
                1.0 / self.dt_mean
            } else {
                0.0
            },
            self.dt_max
        )?;
        writeln!(f, "speed: {:.3} to {:.3}", self.speed_min, self.speed_max)?;
        writeln!(
            f,
            "travelled {:.3} unit(s) along the path, {:.3} from start to finish",
            self.path_length, self.displacement
        )?;
        writeln!(
            f,
            "cross(row0, up) = forward on {}/{} tick(s)",
            self.positively_oriented, self.ticks
        )?;

        if self.angular_ticks == 0 {
            writeln!(
                f,
                "angular velocity: not in this capture (recorded before the column existed)"
            )?;
        } else {
            writeln!(
                f,
                "angular velocity: {}/{} tick(s), rms {:.4} rad/s",
                self.angular_ticks, self.ticks, self.angular_rms
            )?;
            if let Some(fits) = self.angular_readings {
                writeln!(
                    f,
                    "  against the basis derivative, best first: {}",
                    fits.iter()
                        .map(|fit| format!("{} {:.4}", fit.reading.as_str(), fit.rms_error))
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
            }
        }

        writeln!(
            f,
            "clean (speed / |velocity| ~= 1.0): {}/{} tick(s), first contact {} \
             ({} tick(s) below {UNMEASURABLE_SPEED} unit/s: counted in the \
             fraction, skipped when dating first contact - see the field docs)",
            self.clean_ticks,
            self.ticks,
            self.first_contact
                .map_or_else(|| "none".to_string(), |tick| tick.to_string()),
            self.unmeasurable_ticks
        )?;

        match self.stunned_ticks {
            None => write!(f, "stun timer: not in this capture"),
            Some(0) => write!(f, "stun timer: never armed"),
            Some(ticks) => write!(
                f,
                // "the run" rather than "the original": a `Summary` is printed
                // for our own traces too - `oag-trace drive` prints one - and a
                // report that calls our output the original's is a lie in the
                // one place a reader is least likely to check.
                "stun timer: armed on {}/{} tick(s) - no thrust and no lateral \
                 grip on those",
                ticks, self.ticks
            ),
        }?;
        match self.timer_2e0_ticks {
            None => Ok(()),
            Some(0) => write!(f, "; craft+0x2e0 never above zero"),
            Some(ticks) => write!(
                f,
                "; craft+0x2e0 above zero on {}/{} tick(s)",
                ticks, self.ticks
            ),
        }
    }
}
