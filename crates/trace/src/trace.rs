//! The recorded side: one CSV of per-tick ship state, in the original's own
//! conventions.
//!
//! The format is exactly what `scripts/psp-trace.py` writes, and the column list
//! below is a transcription of that script's `CRAFT_FIELDS` and `BODY_FIELDS`
//! rather than a redesign of them. A trace is **derived game data**: it lives
//! under `data/traces/`, which `.gitignore` covers, and is never committed. See
//! `docs/reverse-engineering/verification-protocol.md`.
//!
//! # Two things the recorded numbers are not
//!
//! - **They are not the state after a tick.** The capture breaks on
//!   `Ship_UpdateCraft`'s *entry*, so a row is the craft as the update found it:
//!   the position the previous tick left behind, and control states the previous
//!   tick wrote. A simulated trace must therefore be sampled before its step, not
//!   after, or every field is compared one tick out of phase.
//! - **They are not full precision.** The capture writes `%.7g`, which is about
//!   what an `f32` carries, so a difference below roughly `1e-7` relative is the
//!   file's own rounding rather than a divergence.

use std::fmt;

use oag_core::math::Vec3;

/// Every column a current capture writes, in the order it writes them.
///
/// Parsing is by name rather than by position - the header is read and each
/// column located in it - so a capture that grows a column stays readable.
/// [`Trace::columns`] is what [`Trace::to_csv`] writes, which is this list
/// filtered down to the columns the trace in hand actually carries.
pub const COLUMNS: [&str; 30] = [
    "tick",
    "dt",
    "grounded",
    "throttle",
    "brake",
    "steer",
    "airbrake_l",
    "airbrake_r",
    "speed_cached",
    "stun_timer",
    "timer_2e0",
    "right_x",
    "right_y",
    "right_z",
    "up_x",
    "up_y",
    "up_z",
    "fwd_x",
    "fwd_y",
    "fwd_z",
    "pos_x",
    "pos_y",
    "pos_z",
    "vel_x",
    "vel_y",
    "vel_z",
    "speed",
    "avel_x",
    "avel_y",
    "avel_z",
];

/// The columns every trace must carry, which is what the capture wrote before
/// the angular-velocity and timer columns were added.
///
/// **Captures already taken are not reproducible.** Each one is a hand-driven
/// PPSSPP session, so a column added today can never be backfilled into a file
/// recorded yesterday, and a reader that demanded it would retire the whole
/// existing corpus. So the ones in [`OPTIONAL_COLUMNS`] are read when present and
/// left absent when not - absent, not zero: a zero angular velocity is a claim
/// that the ship was not rotating, and a capture that never measured it must not
/// be able to make that claim. See [`Frame::angular_velocity`].
pub const REQUIRED_COLUMNS: [&str; 25] = [
    "tick",
    "dt",
    "grounded",
    "throttle",
    "brake",
    "steer",
    "airbrake_l",
    "airbrake_r",
    "speed_cached",
    "right_x",
    "right_y",
    "right_z",
    "up_x",
    "up_y",
    "up_z",
    "fwd_x",
    "fwd_y",
    "fwd_z",
    "pos_x",
    "pos_y",
    "pos_z",
    "vel_x",
    "vel_y",
    "vel_z",
    "speed",
];

/// The columns a trace may carry and an older one does not.
pub const OPTIONAL_COLUMNS: [&str; 5] = ["stun_timer", "timer_2e0", "avel_x", "avel_y", "avel_z"];

/// The three columns of the angular velocity, which are all present or all
/// absent: two thirds of a vector is a broken capture, not an old one.
pub const ANGULAR_COLUMNS: [&str; 3] = ["avel_x", "avel_y", "avel_z"];

/// One tick of ship state, as the original held it.
///
/// Every field is in the original's units and the original's frame. Nothing is
/// converted on the way in: a harness that quietly rescaled the recording would
/// be comparing against its own assumptions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Which tick of the capture this is. The capture numbers from zero.
    pub tick: u64,
    /// The frame delta the original was about to integrate, in seconds.
    ///
    /// Variable, mean `1/59.94` over the captures taken so far - which is the
    /// measurement behind ADR-0007 - so a comparison run should normally be fed
    /// this rather than a fixed 60 Hz step.
    pub dt: f32,
    /// Probes in contact over two: `0.0`, `0.5` or `1.0`. Compared exactly.
    pub grounded: f32,
    /// Throttle, on the original's `0..=100` control scale. Raw input, not ramped.
    pub throttle: f32,
    /// Brake, `0..=100`, ramped.
    pub brake: f32,
    /// Steering, `-100..=100`, ramped.
    pub steer: f32,
    /// Left airbrake, `0..=100`, ramped.
    pub airbrake_left: f32,
    /// Right airbrake, `0..=100`, ramped.
    pub airbrake_right: f32,
    /// The craft's cached speed at `craft+0x2ec`, which is **one tick stale**
    /// (199/199 ticks of the capture this was measured on).
    pub speed_cached: f32,
    /// Row 0 of the body's basis - the CSV's `right_*` columns.
    ///
    /// **Named `right` in the capture and measured to be the ship's left.**
    /// Holding left gives `steer = -96` and `+1.51 rad/s` about row 1, holding
    /// right the mirror of that, 199/199 consistent; so forward rotates *toward*
    /// row 0 on a left turn. Confidence 84, `docs/overview/roadmap.md`. The field
    /// is called `row0` here so that nothing in this crate can read a sign out of
    /// a name, and [`crate::replay::Basis`] is where the reconciliation happens -
    /// as a switch, because the finding is exactly the kind a trace comparison
    /// exists to falsify.
    pub row0: Vec3,
    /// Row 1 of the basis: the ship's up axis.
    pub up: Vec3,
    /// Row 2 of the basis: the ship's forward axis.
    pub forward: Vec3,
    /// Centre of mass in world space.
    pub position: Vec3,
    /// Linear velocity in world space.
    pub velocity: Vec3,
    /// The body's own speed field, which is the velocity's length.
    pub speed: f32,
    /// Angular velocity at `body+0x160`, **raw, in the original's own sign and
    /// frame**, or `None` for a capture taken before the column existed.
    ///
    /// The offset rests on two legs, one per binary: PSP `Body_ClearVelocity`
    /// (`0x0884da5c`) zeroes `body+0x140` and `body+0x160` and nothing else, and
    /// `+0x140` is the linear velocity this capture already verifies against the
    /// position delta; PS2 `Ship_ApplyAngularDamping` (`0x0015c1b0`) reads
    /// `body+0x160` as the angular velocity it damps.
    ///
    /// **Its sign and its frame are open**, and nothing here resolves them - see
    /// [`AngularReading`], and [`Summary::angular_readings`] for the check that
    /// settles it out of a capture's own basis rows.
    ///
    /// `None` means "this capture did not measure it", which is not
    /// `Some(Vec3::ZERO)`: the second is a claim that the ship was not rotating.
    /// Everything downstream treats the absence as *not compared* rather than as
    /// agreement.
    pub angular_velocity: Option<Vec3>,
    /// The collision stun timer at `craft+0x290`, in seconds, or `None` for a
    /// capture taken before the column existed.
    ///
    /// Above zero it means the frame produced **no thrust and no lateral grip**:
    /// `Ship_UpdateEngine`'s prologue returns having zeroed the throttle state
    /// (`0x0884c634`, confidence 88) and `Ship_ApplyLateralGrip` returns early
    /// (`0x08848b78`). `Ship_ApplyCollisionImpulse` (`0x0883f274`) arms it with
    /// `+= 0.5` on a hit, so it doubles as the capture's wall-contact indicator.
    pub stun_timer: Option<f32>,
    /// The second gate on the same early return, at `craft+0x2e0`, or `None`.
    ///
    /// Above zero with flag `0x10` clear it kills thrust exactly as the stun
    /// timer does. **What arms it has never been read**, so it carries its offset
    /// for a name rather than a guess: below 50 confidence nothing gets named
    /// (ADR-0005). Recorded because a capture is the only thing that can say
    /// which of the two gates fired.
    pub timer_2e0: Option<f32>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            tick: 0,
            dt: 0.0,
            grounded: 0.0,
            throttle: 0.0,
            brake: 0.0,
            steer: 0.0,
            airbrake_left: 0.0,
            airbrake_right: 0.0,
            speed_cached: 0.0,
            row0: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::Z,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            speed: 0.0,
            angular_velocity: None,
            stun_timer: None,
            timer_2e0: None,
        }
    }
}

impl Frame {
    /// Whether the basis is positively oriented under ordinary component
    /// arithmetic: `cross(row0, up) = forward`.
    ///
    /// Measured true on 200 of 200 ticks of the first real capture, which is what
    /// removed frame handedness as an explanation for the two disputed cross
    /// product signs in the force law. Kept here so any future trace is checked
    /// against the same invariant rather than the claim being taken on trust.
    #[must_use]
    pub fn basis_is_positively_oriented(&self, tolerance: f32) -> bool {
        (self.row0.cross(self.up) - self.forward).length() <= tolerance
    }

    /// The recorded basis rows as a triple, in the file's own order.
    #[must_use]
    pub fn rows(&self) -> (Vec3, Vec3, Vec3) {
        (self.row0, self.up, self.forward)
    }

    /// The world-space angular velocity that carries this tick's basis onto the
    /// next one, in the ordinary physical sign, or `None` for a zero `dt`.
    ///
    /// For an orthonormal triad carried by a rotation of angle `t` about `n`,
    /// Rodrigues gives `sum cross(r_i, r_i') = 2 * n * sin(t)` **exactly** - the
    /// `dot(n, r)` terms cancel over the triad and the third term sums to
    /// `n x n`. So half the summed cross product over the frame delta recovers
    /// `n * sin(t)/dt`, which underestimates the true `n * t/dt` by `t^2/6`. At
    /// 60 Hz and the 1.5 rad/s the captures show, `t` is about a fortieth of a
    /// radian and that is one part in ten thousand: 1.6e-4 rad/s, well inside any
    /// tolerance this is used at.
    ///
    /// This is what makes the recorded angular-velocity column checkable against
    /// the recording itself rather than against a simulation: see
    /// [`Summary::angular_readings`].
    #[must_use]
    pub fn basis_rotation_rate(&self, next: &Self) -> Option<Vec3> {
        // A NaN delta is not a positive one, so the comparison is written the way
        // round that answers `None` for it rather than dividing by it.
        if self.dt.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return None;
        }
        let sum =
            self.row0.cross(next.row0) + self.up.cross(next.up) + self.forward.cross(next.forward);
        Some(sum / (2.0 * self.dt))
    }

    fn value(&self, column: &str) -> Option<f64> {
        Some(f64::from(match column {
            "tick" => return Some(self.tick as f64),
            "dt" => self.dt,
            "grounded" => self.grounded,
            "throttle" => self.throttle,
            "brake" => self.brake,
            "steer" => self.steer,
            "airbrake_l" => self.airbrake_left,
            "airbrake_r" => self.airbrake_right,
            "speed_cached" => self.speed_cached,
            "right_x" => self.row0.x,
            "right_y" => self.row0.y,
            "right_z" => self.row0.z,
            "up_x" => self.up.x,
            "up_y" => self.up.y,
            "up_z" => self.up.z,
            "fwd_x" => self.forward.x,
            "fwd_y" => self.forward.y,
            "fwd_z" => self.forward.z,
            "pos_x" => self.position.x,
            "pos_y" => self.position.y,
            "pos_z" => self.position.z,
            "vel_x" => self.velocity.x,
            "vel_y" => self.velocity.y,
            "vel_z" => self.velocity.z,
            "speed" => self.speed,
            "avel_x" => self.angular_velocity?.x,
            "avel_y" => self.angular_velocity?.y,
            "avel_z" => self.angular_velocity?.z,
            "stun_timer" => self.stun_timer?,
            "timer_2e0" => self.timer_2e0?,
            _ => return None,
        }))
    }

    /// Whether the frame carries a column, which for an optional one is whether
    /// the capture it came from measured it at all.
    #[must_use]
    pub fn has(&self, column: &str) -> bool {
        match column {
            "avel_x" | "avel_y" | "avel_z" => self.angular_velocity.is_some(),
            "stun_timer" => self.stun_timer.is_some(),
            "timer_2e0" => self.timer_2e0.is_some(),
            other => REQUIRED_COLUMNS.contains(&other),
        }
    }

    fn set(&mut self, column: &str, value: f32) {
        match column {
            "tick" => self.tick = value as u64,
            "dt" => self.dt = value,
            "grounded" => self.grounded = value,
            "throttle" => self.throttle = value,
            "brake" => self.brake = value,
            "steer" => self.steer = value,
            "airbrake_l" => self.airbrake_left = value,
            "airbrake_r" => self.airbrake_right = value,
            "speed_cached" => self.speed_cached = value,
            "right_x" => self.row0.x = value,
            "right_y" => self.row0.y = value,
            "right_z" => self.row0.z = value,
            "up_x" => self.up.x = value,
            "up_y" => self.up.y = value,
            "up_z" => self.up.z = value,
            "fwd_x" => self.forward.x = value,
            "fwd_y" => self.forward.y = value,
            "fwd_z" => self.forward.z = value,
            "pos_x" => self.position.x = value,
            "pos_y" => self.position.y = value,
            "pos_z" => self.position.z = value,
            "vel_x" => self.velocity.x = value,
            "vel_y" => self.velocity.y = value,
            "vel_z" => self.velocity.z = value,
            "speed" => self.speed = value,
            // The three angular columns arrive one at a time, so the first of
            // them promotes the field out of `None`; `Trace::parse` has already
            // rejected a header carrying some of them and not others.
            "avel_x" => self.angular_velocity.get_or_insert(Vec3::ZERO).x = value,
            "avel_y" => self.angular_velocity.get_or_insert(Vec3::ZERO).y = value,
            "avel_z" => self.angular_velocity.get_or_insert(Vec3::ZERO).z = value,
            "stun_timer" => self.stun_timer = Some(value),
            "timer_2e0" => self.timer_2e0 = Some(value),
            _ => {}
        }
    }
}

/// A whole capture: the frames, in tick order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trace {
    /// One per tick, in the order they were recorded.
    pub frames: Vec<Frame>,
}

/// Why a CSV could not be read as a trace.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The file held no header row.
    #[error("empty trace: no header row")]
    Empty,
    /// The header is missing a column the comparison needs.
    #[error("trace header is missing the column {0:?}")]
    MissingColumn(String),
    /// A row has a different number of fields than the header.
    #[error("line {line}: expected {expected} field(s), found {found}")]
    Width {
        /// One-based line number in the file.
        line: usize,
        /// How many the header declared.
        expected: usize,
        /// How many the row held.
        found: usize,
    },
    /// A field would not parse as a number.
    #[error("line {line}, column {column:?}: {value:?} is not a number")]
    NotANumber {
        /// One-based line number in the file.
        line: usize,
        /// Which column it was in.
        column: String,
        /// What was there instead.
        value: String,
    },
}

impl Trace {
    /// Reads a trace out of CSV text.
    ///
    /// Blank lines and lines starting with `#` are skipped, which is for
    /// hand-annotated fixtures. It does **not** rescue a capture run with
    /// `2>&1`: `scripts/psp-trace.py` writes its `craft at 0x...` line to stderr
    /// unprefixed, so a merged file fails on the header rather than skipping it.
    /// Capture with `--out`, or keep the streams apart.
    ///
    /// Columns are located by header name, and every column in
    /// [`REQUIRED_COLUMNS`] must be present: silently defaulting a missing one
    /// would produce a comparison that passes because it never looked. The
    /// [`OPTIONAL_COLUMNS`] are read when the header has them and left absent
    /// when it does not, which is what keeps a capture taken before they existed
    /// readable - see [`REQUIRED_COLUMNS`] for why that matters.
    ///
    /// # Errors
    ///
    /// [`Error::Empty`] for a file with no header, [`Error::MissingColumn`] for a
    /// header that does not carry every column of [`REQUIRED_COLUMNS`] - or that
    /// carries some of [`ANGULAR_COLUMNS`] and not all three - and
    /// [`Error::Width`] or [`Error::NotANumber`] for a malformed row.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.trim()))
            .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'));

        let (_, header) = lines.next().ok_or(Error::Empty)?;
        let header: Vec<&str> = header.split(',').map(str::trim).collect();
        for wanted in REQUIRED_COLUMNS {
            if !header.contains(&wanted) {
                return Err(Error::MissingColumn(wanted.to_owned()));
            }
        }
        // A vector arrives whole or not at all. A header with `avel_x` and no
        // `avel_y` is a truncated or edited file rather than an older capture,
        // and reading it as a rotation about one axis would be an invention.
        if ANGULAR_COLUMNS.iter().any(|c| header.contains(c)) {
            for wanted in ANGULAR_COLUMNS {
                if !header.contains(&wanted) {
                    return Err(Error::MissingColumn(wanted.to_owned()));
                }
            }
        }

        let mut frames = Vec::new();
        for (line, row) in lines {
            let fields: Vec<&str> = row.split(',').map(str::trim).collect();
            if fields.len() != header.len() {
                return Err(Error::Width {
                    line,
                    expected: header.len(),
                    found: fields.len(),
                });
            }
            let mut frame = Frame::default();
            for (column, field) in header.iter().zip(&fields) {
                let value: f32 = field.parse().map_err(|_| Error::NotANumber {
                    line,
                    column: (*column).to_owned(),
                    value: (*field).to_owned(),
                })?;
                frame.set(column, value);
            }
            frames.push(frame);
        }
        Ok(Self { frames })
    }

    /// How many ticks the trace holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether the trace holds no ticks at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The columns this trace carries, in the capture's own order.
    ///
    /// [`REQUIRED_COLUMNS`] always, plus each of [`OPTIONAL_COLUMNS`] that
    /// *every* frame holds. All-or-nothing because a CSV column is: a file cannot
    /// have `stun_timer` on some rows and not others, so a trace where only some
    /// frames carry one was assembled in memory rather than read, and writing it
    /// out would have to invent a value for the rest.
    #[must_use]
    pub fn columns(&self) -> Vec<&'static str> {
        COLUMNS
            .into_iter()
            .filter(|column| {
                REQUIRED_COLUMNS.contains(column)
                    || (!self.frames.is_empty() && self.frames.iter().all(|f| f.has(column)))
            })
            .collect()
    }

    /// Writes the trace back out in the capture's own column order.
    ///
    /// Floats are written with Rust's shortest round-tripping form rather than
    /// the capture's `%.7g`, so a simulated trace written here and read back is
    /// the same trace. A *recorded* trace round-tripped through this will
    /// therefore not be byte-identical to the file it came from, only
    /// numerically identical to what that file said.
    ///
    /// Only the columns [`Self::columns`] reports are written, so a trace read
    /// from a capture that predates the angular-velocity column writes it back
    /// without one rather than filling the gap with a zero that would read as a
    /// measurement.
    #[must_use]
    pub fn to_csv(&self) -> String {
        let columns = self.columns();
        let mut out = String::new();
        out.push_str(&columns.join(","));
        out.push('\n');
        for frame in &self.frames {
            for (index, column) in columns.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if *column == "tick" {
                    out.push_str(&frame.tick.to_string());
                } else {
                    let value = frame.value(column).unwrap_or(f64::NAN) as f32;
                    out.push_str(&value.to_string());
                }
            }
            out.push('\n');
        }
        out
    }

    /// A one-paragraph description of what is in the trace.
    ///
    /// Cheap orientation before a comparison: how long it is, whether the deltas
    /// look like the vblank-locked frame time the captures show, how far the ship
    /// actually went - a stalled ship is the false-start trap, not a bug - and
    /// whether the recorded basis is a positively oriented set on every tick.
    #[must_use]
    pub fn summary(&self) -> Summary {
        let mut summary = Summary {
            ticks: self.len(),
            ..Summary::default()
        };
        let Some(first) = self.frames.first() else {
            return summary;
        };
        let last = self.frames.last().copied().unwrap_or(*first);

        summary.dt_min = f32::INFINITY;
        summary.speed_min = f32::INFINITY;
        let mut dt_total = 0.0f64;
        let mut distance = 0.0f64;
        let mut previous = first.position;
        for frame in &self.frames {
            summary.dt_min = summary.dt_min.min(frame.dt);
            summary.dt_max = summary.dt_max.max(frame.dt);
            dt_total += f64::from(frame.dt);
            summary.speed_min = summary.speed_min.min(frame.speed);
            summary.speed_max = summary.speed_max.max(frame.speed);
            distance += f64::from((frame.position - previous).length());
            previous = frame.position;
            if frame.basis_is_positively_oriented(BASIS_TOLERANCE) {
                summary.positively_oriented += 1;
            }
        }
        summary.dt_mean = (dt_total / self.len() as f64) as f32;
        summary.path_length = distance as f32;
        summary.displacement = (last.position - first.position).length();

        summary.angular_ticks = self
            .frames
            .iter()
            .filter(|f| f.angular_velocity.is_some())
            .count();
        if self.frames.iter().all(|f| f.stun_timer.is_some()) {
            summary.stunned_ticks = Some(
                self.frames
                    .iter()
                    .filter(|f| f.stun_timer > Some(0.0))
                    .count(),
            );
        }
        if self.frames.iter().all(|f| f.timer_2e0.is_some()) {
            summary.timer_2e0_ticks = Some(
                self.frames
                    .iter()
                    .filter(|f| f.timer_2e0 > Some(0.0))
                    .count(),
            );
        }
        summary.angular_readings = self.angular_readings(&mut summary.angular_rms);
        summary
    }

    /// Scores every [`AngularReading`] of the recorded column against the
    /// rotation the recorded basis itself performs, best first.
    ///
    /// The error is measured in the *recorded* column's own space - the basis
    /// derivative is mapped forward through each reading rather than the column
    /// being mapped back - so all four numbers are in rad/s and comparable to
    /// each other and to `signal_rms`, which is set to the recorded column's own
    /// root-mean-square magnitude.
    fn angular_readings(&self, signal_rms: &mut f32) -> Option<[AngularFit; 4]> {
        let mut squared = [0.0f64; 4];
        let mut signal = 0.0f64;
        let mut count = 0usize;
        for pair in self.frames.windows(2) {
            let (frame, next) = (&pair[0], &pair[1]);
            let (Some(recorded), Some(measured)) =
                (frame.angular_velocity, frame.basis_rotation_rate(next))
            else {
                continue;
            };
            let rows = frame.rows();
            for (slot, reading) in AngularReading::ALL.into_iter().enumerate() {
                let predicted = reading.to_recorded(measured, rows);
                squared[slot] += f64::from((predicted - recorded).length_squared());
            }
            signal += f64::from(recorded.length_squared());
            count += 1;
        }
        if count == 0 {
            return None;
        }
        let root = |total: f64| (total / count as f64).sqrt() as f32;
        *signal_rms = root(signal);
        let mut fits = std::array::from_fn(|slot| AngularFit {
            reading: AngularReading::ALL[slot],
            rms_error: root(squared[slot]),
        });
        // Best first, and a total order so a report is deterministic: `f32` has
        // no `Ord`, and a NaN error - which a poisoned capture would produce -
        // must sort last rather than wherever a partial comparison leaves it.
        fits.sort_by(|a: &AngularFit, b: &AngularFit| {
            a.rms_error
                .partial_cmp(&b.rms_error)
                .unwrap_or(std::cmp::Ordering::Greater)
        });
        Some(fits)
    }
}

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
/// - **Sign.** `docs/ghidra/functions/ps2-pulse/craft-update.md` derives
///   `w_game = -w_physics` from the PS2 integrator on three independent legs, so
///   the engine's angular velocity is the negative of the physical one. That is a
///   result about the *accumulators*; whether the stored velocity carries the
///   same convention is the obvious reading and not a measured one.
/// - **Frame.** The same page names `body+0x160` `angularVelocityLocal` and then
///   lists "which frame `body+0x150` and `body+0x160` are each expressed in" as
///   unresolved, and `docs/ghidra/functions/psp-pulse/engine.md` caps the
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

/// What [`Trace::summary`] found.
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
    /// the column existed, which is not an error - see [`REQUIRED_COLUMNS`].
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

        match self.stunned_ticks {
            None => write!(f, "stun timer: not in this capture"),
            Some(0) => write!(f, "stun timer: never armed"),
            Some(ticks) => write!(
                f,
                "stun timer: armed on {}/{} tick(s) - the original produced no thrust \
                 and no lateral grip there",
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Two ticks in exactly the shape `scripts/psp-trace.py` writes: the same
    /// header, in the same order, with `%.7g`-shaped values. Hand-authored, not
    /// captured - a real trace is derived game data and cannot be committed.
    pub(crate) const FIXTURE: &str = "\
tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
stun_timer,timer_2e0,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,avel_x,avel_y,avel_z
0,0.016683,1,100,0,0,0,0,21.98,0,0,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22,0,0,0
1,0.016683,1,100,0,0,0,0,22,0,0,1,0,0,0,1,0,0,0,1,10,2.5,-29.63301,0,0,22.1,22.1,0,0,0
";

    /// The same two ticks as a capture taken **before** the angular-velocity and
    /// timer columns existed - which is every trace under `data/traces/` today.
    ///
    /// This fixture is the backwards-compatibility contract in one constant. It
    /// is not a stale copy of [`FIXTURE`] to be updated alongside it: it is a
    /// frozen record of a file format that real, unreproducible captures are
    /// already written in, and it must keep parsing unchanged forever.
    pub(crate) const LEGACY_FIXTURE: &str = "\
tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed
0,0.016683,1,100,0,0,0,0,21.98,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22
1,0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,10,2.5,-29.63301,0,0,22.1,22.1
";

    #[test]
    fn the_fixture_header_is_the_capture_script_s_header() {
        // If `scripts/psp-trace.py` grows a column, this is what fails first.
        let header = FIXTURE.lines().next().unwrap();
        assert_eq!(header, COLUMNS.join(","));
    }

    /// The header this crate expects, read out of the capture script itself.
    ///
    /// The doc comment on [`COLUMNS`] says the list is a transcription of
    /// `scripts/psp-trace.py`'s `CRAFT_FIELDS` and `BODY_FIELDS`. A transcription
    /// that nothing checks is a copy waiting to drift, and the failure mode is
    /// quiet: a column added on one side only makes every trace unreadable, or
    /// worse, shifts what a name means. So the script is the source of truth and
    /// this reads it.
    fn header_the_capture_script_writes() -> Vec<String> {
        const SCRIPT: &str = include_str!("../../../scripts/psp-trace.py");
        let mut columns = vec!["tick".to_owned()];
        for list in ["CRAFT_FIELDS = [", "BODY_FIELDS = ["] {
            let start = SCRIPT
                .find(list)
                .unwrap_or_else(|| panic!("{list} is not in scripts/psp-trace.py"))
                + list.len();
            let body = &SCRIPT[start..];
            let end = body.find("\n]").expect("an unterminated field list");
            // Every entry is `("name", 0x...)`, so the names are exactly the
            // quoted strings inside the list.
            columns.extend(body[..end].split('"').skip(1).step_by(2).map(str::to_owned));
        }
        columns
    }

    /// The one test that can catch the capture script and this crate drifting
    /// apart, which no fixture can: a fixture is written by hand to match
    /// whatever this file already says.
    #[test]
    fn the_column_list_is_the_capture_script_s_own() {
        assert_eq!(header_the_capture_script_writes(), COLUMNS.to_vec());
    }

    #[test]
    fn the_legacy_fixture_is_exactly_the_required_columns() {
        let header = LEGACY_FIXTURE.lines().next().unwrap();
        assert_eq!(header, REQUIRED_COLUMNS.join(","));
    }

    /// The optional columns are the difference between the two, and every one of
    /// them is in the full header. A column added to `COLUMNS` and forgotten in
    /// `OPTIONAL_COLUMNS` would be silently unwritable by [`Trace::to_csv`].
    #[test]
    fn every_column_is_either_required_or_optional() {
        for column in COLUMNS {
            assert!(
                REQUIRED_COLUMNS.contains(&column) ^ OPTIONAL_COLUMNS.contains(&column),
                "{column} is in neither list, or in both"
            );
        }
        assert_eq!(
            COLUMNS.len(),
            REQUIRED_COLUMNS.len() + OPTIONAL_COLUMNS.len()
        );
        for column in ANGULAR_COLUMNS {
            assert!(OPTIONAL_COLUMNS.contains(&column));
        }
    }

    #[test]
    fn a_capture_shaped_csv_parses() {
        let trace = Trace::parse(FIXTURE).unwrap();
        assert_eq!(trace.len(), 2);
        let first = trace.frames[0];
        assert_eq!(first.tick, 0);
        assert_eq!(first.dt, 0.016_683);
        assert_eq!(first.grounded, 1.0);
        assert_eq!(first.throttle, 100.0);
        assert_eq!(first.speed_cached, 21.98);
        assert_eq!(first.row0, Vec3::X);
        assert_eq!(first.up, Vec3::Y);
        assert_eq!(first.forward, Vec3::Z);
        assert_eq!(first.position, Vec3::new(10.0, 2.5, -30.0));
        assert_eq!(first.velocity, Vec3::new(0.0, 0.0, 22.0));
        assert_eq!(trace.frames[1].tick, 1);
    }

    #[test]
    fn columns_are_located_by_name_not_by_position() {
        let reordered = "\
speed,vel_z,vel_y,vel_x,pos_z,pos_y,pos_x,fwd_z,fwd_y,fwd_x,up_z,up_y,up_x,\
right_z,right_y,right_x,speed_cached,airbrake_r,airbrake_l,steer,brake,throttle,\
grounded,dt,tick
22,22,0,0,-30,2.5,10,1,0,0,0,1,0,0,0,1,21.98,0,0,0,0,100,1,0.016683,0
";
        let trace = Trace::parse(reordered).unwrap();
        assert_eq!(
            trace.frames[0],
            Trace::parse(LEGACY_FIXTURE).unwrap().frames[0]
        );
    }

    /// The whole backwards-compatibility contract: every capture under
    /// `data/traces/` predates these columns, none of them can be re-taken
    /// without a hand-driven PPSSPP session, and all of them must keep working.
    #[test]
    fn a_capture_without_the_new_columns_still_parses() {
        let trace = Trace::parse(LEGACY_FIXTURE).unwrap();
        assert_eq!(trace.len(), 2);
        assert_eq!(trace.frames[0].speed_cached, 21.98);
        assert_eq!(trace.frames[0].position, Vec3::new(10.0, 2.5, -30.0));
    }

    /// Absent, not zero. A zero angular velocity is a claim that the ship was not
    /// rotating; a capture that never measured it must not be able to make that
    /// claim, because that is exactly how a comparison passes without looking.
    #[test]
    fn an_absent_column_is_absent_rather_than_zero() {
        let frame = Trace::parse(LEGACY_FIXTURE).unwrap().frames[0];
        assert_eq!(frame.angular_velocity, None);
        assert_eq!(frame.stun_timer, None);
        assert_eq!(frame.timer_2e0, None);
        assert_ne!(frame.angular_velocity, Some(Vec3::ZERO));
    }

    /// Writing a legacy trace back out must not grow it a column, or the absence
    /// would be laundered into a zero by one round trip through this crate.
    #[test]
    fn a_legacy_trace_round_trips_without_growing_a_column() {
        let trace = Trace::parse(LEGACY_FIXTURE).unwrap();
        let csv = trace.to_csv();
        assert_eq!(csv.lines().next().unwrap(), REQUIRED_COLUMNS.join(","));
        assert_eq!(Trace::parse(&csv).unwrap(), trace);
        assert_eq!(trace.columns(), REQUIRED_COLUMNS.to_vec());
    }

    #[test]
    fn a_current_trace_round_trips_with_every_column() {
        let trace = Trace::parse(FIXTURE).unwrap();
        let csv = trace.to_csv();
        assert_eq!(csv.lines().next().unwrap(), COLUMNS.join(","));
        assert_eq!(Trace::parse(&csv).unwrap(), trace);
    }

    #[test]
    fn the_new_columns_are_read_when_they_are_there() {
        let text = FIXTURE
            .replacen(",22,22,0,0,0\n", ",22,22,0.25,-1.5,0.125\n", 1)
            .replacen("21.98,0,0,", "21.98,0.5,0.75,", 1);
        let frame = Trace::parse(&text).unwrap().frames[0];
        assert_eq!(frame.angular_velocity, Some(Vec3::new(0.25, -1.5, 0.125)));
        assert_eq!(frame.stun_timer, Some(0.5));
        assert_eq!(frame.timer_2e0, Some(0.75));
    }

    /// Two thirds of a vector is a truncated or edited file, not an older
    /// capture, and reading it as a rotation about one axis would be an
    /// invention.
    #[test]
    fn a_partial_angular_velocity_is_an_error() {
        let text = FIXTURE.replace(",avel_z", "").replace(",0,0,0\n", ",0,0\n");
        assert_eq!(
            Trace::parse(&text),
            Err(Error::MissingColumn("avel_z".to_owned()))
        );
    }

    #[test]
    fn a_missing_column_is_an_error_not_a_default() {
        let text = FIXTURE.replacen("speed_cached", "not_the_column", 1);
        assert_eq!(
            Trace::parse(&text),
            Err(Error::MissingColumn("speed_cached".to_owned()))
        );
    }

    #[test]
    fn a_short_row_is_an_error() {
        let text = format!("{FIXTURE}2,0.016683\n");
        assert_eq!(
            Trace::parse(&text),
            Err(Error::Width {
                line: 4,
                expected: COLUMNS.len(),
                found: 2
            })
        );
    }

    #[test]
    fn a_non_numeric_field_names_itself() {
        let text = FIXTURE.replacen("21.98", "nan-ish", 1);
        assert_eq!(
            Trace::parse(&text),
            Err(Error::NotANumber {
                line: 2,
                column: "speed_cached".to_owned(),
                value: "nan-ish".to_owned()
            })
        );
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let text = format!("# craft at 0x08c00000\n\n{FIXTURE}");
        assert_eq!(Trace::parse(&text).unwrap().len(), 2);
    }

    #[test]
    fn an_empty_file_is_an_error() {
        assert_eq!(Trace::parse(""), Err(Error::Empty));
    }

    #[test]
    fn a_trace_round_trips_through_csv() {
        let trace = Trace::parse(FIXTURE).unwrap();
        assert_eq!(Trace::parse(&trace.to_csv()).unwrap(), trace);
    }

    #[test]
    fn the_summary_measures_what_the_ship_did() {
        let summary = Trace::parse(FIXTURE).unwrap().summary();
        assert_eq!(summary.ticks, 2);
        assert_eq!(summary.positively_oriented, 2, "cross(x, y) = z");
        assert!((summary.displacement - 0.366_99).abs() < 1e-4);
        assert!((summary.dt_mean - 0.016_683).abs() < 1e-6);
    }

    /// How far the ship is rolled in [`turning`], in radians.
    ///
    /// **Not zero, and not a right angle.** A ship rotating about world `+y` with
    /// its own up axis along `+y` records the same three numbers under the local
    /// and the world reading, so a level fixture can separate the *sign* question
    /// and is blind to the *frame* one. Rolling it puts the rotation axis across
    /// all three body axes and separates all four readings.
    const ROLL: f32 = 0.5;

    /// The recorded yaw rate the captures actually show, in rad/s: holding left
    /// gives `+1.51` about row 1 on 199 of 199 ticks, per
    /// `docs/ghidra/functions/psp-pulse/engine.md`. Used as the fixture's signal
    /// so the numbers in these tests are the size of the real ones.
    const YAW_RATE: f32 = 1.51;

    /// A ship turning steadily about world `+y` at [`YAW_RATE`], rolled by
    /// [`ROLL`], with the angular-velocity column written under `reading`.
    ///
    /// The basis is `Ry(theta) * B` for a fixed rolled triad `B`, so the rotation
    /// carrying one tick onto the next is `Ry(omega * dt)` in world space on every
    /// tick: the world angular velocity is exactly `(0, YAW_RATE, 0)` and the
    /// recorded column is whatever `reading` says that is.
    fn turning(ticks: usize, dt: f32, reading: AngularReading) -> Trace {
        let (roll_sin, roll_cos) = ROLL.sin_cos();
        let base = (
            Vec3::new(roll_cos, roll_sin, 0.0),
            Vec3::new(-roll_sin, roll_cos, 0.0),
            Vec3::Z,
        );
        let world = Vec3::new(0.0, YAW_RATE, 0.0);
        Trace {
            frames: (0..ticks)
                .map(|tick| {
                    let (sin, cos) = (YAW_RATE * tick as f32 * dt).sin_cos();
                    let yaw =
                        |v: Vec3| Vec3::new(v.x * cos + v.z * sin, v.y, -v.x * sin + v.z * cos);
                    let rows = (yaw(base.0), yaw(base.1), yaw(base.2));
                    Frame {
                        tick: tick as u64,
                        dt,
                        row0: rows.0,
                        up: rows.1,
                        forward: rows.2,
                        angular_velocity: Some(reading.to_recorded(world, rows)),
                        ..Frame::default()
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn the_turning_fixture_is_a_positively_oriented_basis_like_a_real_one() {
        for frame in turning(4, 1.0 / 60.0, AngularReading::default()).frames {
            assert!(frame.basis_is_positively_oriented(1e-5));
        }
    }

    /// The recorded rows differentiate into the rotation that carries one onto
    /// the next, which is what makes the new column checkable against the file it
    /// arrived in rather than against a simulation.
    #[test]
    fn the_basis_derivative_recovers_the_rate_the_fixture_turns_at() {
        let trace = turning(4, 1.0 / 60.0, AngularReading::default());
        let measured = trace.frames[0]
            .basis_rotation_rate(&trace.frames[1])
            .expect("a nonzero dt");
        assert!(
            (measured - Vec3::new(0.0, YAW_RATE, 0.0)).length() < 1e-3,
            "{measured}"
        );
    }

    #[test]
    fn a_zero_delta_has_no_rotation_rate_rather_than_an_infinite_one() {
        let frame = Frame::default();
        assert_eq!(frame.basis_rotation_rate(&Frame::default()), None);
    }

    /// [`AngularReading::ALL`] is in report order rather than declaration order,
    /// and [`Trace::angular_readings`] indexes its accumulators against it. A
    /// variant added to the enum and forgotten in `ALL` would simply never be
    /// scored, and the report would look complete while missing a candidate.
    #[test]
    fn every_reading_is_in_the_list_exactly_once() {
        for reading in [
            AngularReading::World,
            AngularReading::NegatedWorld,
            AngularReading::Local,
            AngularReading::NegatedLocal,
        ] {
            assert_eq!(
                AngularReading::ALL
                    .iter()
                    .filter(|r| **r == reading)
                    .count(),
                1,
                "{reading:?}"
            );
        }
        assert_eq!(AngularReading::ALL.len(), 4);
    }

    /// Every reading must be invertible, or seeding a run from a recorded column
    /// and writing our own back out would not be the same transformation twice.
    /// The same property [`crate::replay::Basis`] is pinned on.
    #[test]
    fn every_reading_round_trips() {
        let frame = turning(1, 1.0 / 60.0, AngularReading::default()).frames[0];
        let rows = frame.rows();
        let w = Vec3::new(0.3, -1.51, 0.07);
        for reading in AngularReading::ALL {
            let there_and_back = reading.to_world(reading.to_recorded(w, rows), rows);
            assert!(
                (there_and_back - w).length() < 1e-5,
                "{reading:?}: {there_and_back} is not {w}"
            );
        }
    }

    /// **The measurement the column exists to make.** A capture written under one
    /// reading must name that reading and no other, with the three wrong ones an
    /// order of magnitude worse - and this is decided by the recording alone, so
    /// the first real capture settles both open questions without a run.
    #[test]
    fn a_capture_names_the_reading_it_was_written_under() {
        for reading in AngularReading::ALL {
            let summary = turning(40, 1.0 / 60.0, reading).summary();
            let fits = summary.angular_readings.expect("40 ticks of it");
            assert_eq!(summary.angular_ticks, 40);
            assert_eq!(fits[0].reading, reading, "{reading:?}: {summary}");
            assert!(
                fits[0].rms_error < 1e-2,
                "{reading:?} should fit itself: {}",
                fits[0].rms_error
            );
            assert!(
                fits[1].rms_error > 10.0 * fits[0].rms_error.max(1e-3),
                "{reading:?} is not distinguished from {:?}: {} vs {}",
                fits[1].reading,
                fits[0].rms_error,
                fits[1].rms_error
            );
            assert!((summary.angular_rms - YAW_RATE).abs() < 1e-3);
        }
    }

    #[test]
    fn a_capture_without_the_column_scores_no_readings_and_says_so() {
        let summary = Trace::parse(LEGACY_FIXTURE).unwrap().summary();
        assert_eq!(summary.angular_ticks, 0);
        assert_eq!(summary.angular_readings, None);
        assert_eq!(summary.best_angular_reading(), None);
        assert_eq!(summary.stunned_ticks, None);
        assert!(summary.to_string().contains("not in this capture"));
    }

    /// The stun timer is the capture's wall-contact indicator, and a straight-line
    /// capture that spent ticks inside the window is a *stunned* ship coasting -
    /// not an equilibrium of the engine force law. So the count is in the summary
    /// rather than only in a comparison.
    #[test]
    fn the_summary_counts_the_ticks_the_original_spent_stunned() {
        let mut trace = Trace::parse(FIXTURE).unwrap();
        assert_eq!(trace.summary().stunned_ticks, Some(0));
        trace.frames[1].stun_timer = Some(0.5);
        let summary = trace.summary();
        assert_eq!(summary.stunned_ticks, Some(1));
        assert_eq!(summary.timer_2e0_ticks, Some(0));
        assert!(summary.to_string().contains("armed on 1/2"), "{summary}");
    }

    #[test]
    fn a_left_handed_basis_is_reported_as_such() {
        let frame = Frame {
            forward: -Vec3::Z,
            ..Frame::default()
        };
        assert!(!frame.basis_is_positively_oriented(BASIS_TOLERANCE));
    }
}
