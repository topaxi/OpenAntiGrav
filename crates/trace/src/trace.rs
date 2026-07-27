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

/// The CSV's columns, in the order `scripts/psp-trace.py` writes them.
///
/// Parsing is by name rather than by position - the header is read and each
/// column located in it - so a capture that grows a column stays readable. The
/// order here is what [`Trace::to_csv`] writes.
pub const COLUMNS: [&str; 25] = [
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
            _ => return None,
        }))
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
    /// Columns are located by header name, and every column in [`COLUMNS`] must
    /// be present: silently defaulting a missing one would produce a comparison
    /// that passes because it never looked.
    ///
    /// # Errors
    ///
    /// [`Error::Empty`] for a file with no header, [`Error::MissingColumn`] for a
    /// header that does not carry every column of [`COLUMNS`], and
    /// [`Error::Width`] or [`Error::NotANumber`] for a malformed row.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.trim()))
            .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'));

        let (_, header) = lines.next().ok_or(Error::Empty)?;
        let header: Vec<&str> = header.split(',').map(str::trim).collect();
        for wanted in COLUMNS {
            if !header.contains(&wanted) {
                return Err(Error::MissingColumn(wanted.to_owned()));
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

    /// Writes the trace back out in the capture's own column order.
    ///
    /// Floats are written with Rust's shortest round-tripping form rather than
    /// the capture's `%.7g`, so a simulated trace written here and read back is
    /// the same trace. A *recorded* trace round-tripped through this will
    /// therefore not be byte-identical to the file it came from, only
    /// numerically identical to what that file said.
    #[must_use]
    pub fn to_csv(&self) -> String {
        let mut out = String::new();
        out.push_str(&COLUMNS.join(","));
        out.push('\n');
        for frame in &self.frames {
            for (index, column) in COLUMNS.iter().enumerate() {
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
        summary
    }
}

/// How far `cross(row0, up)` may sit from `forward` and still count as a
/// positively oriented basis.
///
/// The capture writes seven significant digits, so an exactly orthonormal basis
/// arrives with about `1e-6` of rounding in it.
pub const BASIS_TOLERANCE: f32 = 1e-4;

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
        write!(
            f,
            "cross(row0, up) = forward on {}/{} tick(s)",
            self.positively_oriented, self.ticks
        )
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
        assert_eq!(trace.frames[0], Trace::parse(FIXTURE).unwrap().frames[0]);
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
                expected: 25,
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

    #[test]
    fn a_left_handed_basis_is_reported_as_such() {
        let frame = Frame {
            forward: -Vec3::Z,
            ..Frame::default()
        };
        assert!(!frame.basis_is_positively_oriented(BASIS_TOLERANCE));
    }
}
