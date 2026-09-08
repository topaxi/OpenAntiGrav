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

mod flare;
mod frame;

pub use flare::{FLARE_COLUMNS, Flare};
pub use frame::{
    ANGULAR_COLUMNS, ANGULAR_RATE_COLUMNS, CAMERA_COLUMNS, COLUMNS, Frame, OPTIONAL_COLUMNS,
    REQUIRED_COLUMNS,
};

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
    /// carries part of an all-or-nothing group ([`ANGULAR_COLUMNS`],
    /// [`ANGULAR_RATE_COLUMNS`], [`CAMERA_COLUMNS`], [`FLARE_COLUMNS`]) without
    /// the rest - and
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
        // The camera pose is one twelve-column group for the same reason: a
        // basis without its eye, or rows without a third, is not a pose. The
        // flare's eight are one group because they are one `--flare` read of one
        // object: a file with `boost_timer` and no `plume_timer` was edited.
        for group in [
            &ANGULAR_COLUMNS[..],
            &ANGULAR_RATE_COLUMNS[..],
            &CAMERA_COLUMNS[..],
            &FLARE_COLUMNS[..],
        ] {
            if group.iter().any(|c| header.contains(c)) {
                for wanted in group {
                    if !header.contains(wanted) {
                        return Err(Error::MissingColumn((*wanted).to_owned()));
                    }
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

        for frame in &self.frames {
            let magnitude = frame.velocity.length();
            let unmeasurable = magnitude < UNMEASURABLE_SPEED;
            if unmeasurable {
                summary.unmeasurable_ticks += 1;
            }
            if (frame.speed / magnitude - 1.0).abs() < CLEAN_TOLERANCE {
                summary.clean_ticks += 1;
            } else if summary.first_contact.is_none() && !unmeasurable {
                // A tick below the floor cannot *date* a contact even though it
                // still counts toward `clean_ticks` above: near zero speed the
                // ratio is undefined at exactly zero velocity (NaN, which never
                // compares clean) and unreliable just above it, so an early
                // near-rest tick must not poison the first-contact date the way
                // it is allowed to nudge the fraction. See `UNMEASURABLE_SPEED`.
                summary.first_contact = Some(frame.tick as usize);
            }
        }

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

mod summary;

pub use summary::{
    AngularFit, AngularReading, BASIS_TOLERANCE, CLEAN_TOLERANCE, Summary, UNMEASURABLE_SPEED,
};

#[cfg(test)]
mod tests;
