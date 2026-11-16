//! Omega's `.shprobes`: the baked spherical-harmonic ambient a craft's hull is lit from.
//!
//! **A hull's ambient is `kmix * max(0.01, SH(n))`, and `SH` is nine rgb coefficients taken
//! from this file.** The original blends them per craft from the circuit's records and binds
//! them as `shCoeffs` (`docs/ghidra/functions/ps4-omega-eu/hull-tone.md`, "The probe file";
//! loader at `0x013c102c` in the 1.07 track load). Little-endian, beside `track.vex`:
//!
//! ```text
//! u32 magic   'SHPB' (the bytes read "BPHS")
//! u32 count   a multiple of 3
//! count x { f32 position[3]; f32 sh[27]; }     120 bytes each, sh in the shader's own order
//! ```
//!
//! Records come in triples: three points across the track about 18.6 apart, and consecutive
//! triples step 1.5 along it. The coefficients are used raw (`sh[3 k + c]` is channel `c` of
//! basis function `k`).
//!
//! The craft's blend is two records of one lane, one in each of the triples either side of it
//! ([`ShProbes::locate`]). The live block of the original equals `record 1388 + 0.076 *
//! (record 1391 - record 1388)` to five decimals; a third record's weight was never seen, so the
//! lateral law is unread and the nearest lane is taken. Confidence: layout and raw use 90, the
//! along-track blend 60.

use std::fmt;

use crate::probes::{lerp, project01, sub};

const MAGIC: [u8; 4] = *b"BPHS";
const HEADER: usize = 8;
const FLOATS: usize = 30;
const RECORD: usize = FLOATS * 4;

/// The coefficients one blend yields: nine rgb triples, `[3 * k + c]`.
pub type Coefficients = [f32; 27];

/// Why a `.shprobes` file was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Shorter than its own header.
    Truncated,
    /// The first word is not `SHPB`.
    Magic([u8; 4]),
    /// A count that is not a multiple of 3.
    Count(usize),
    /// Shorter than the records the count names.
    Length { expected: usize, found: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(f, "shprobes: shorter than the header"),
            Self::Magic(m) => write!(f, "shprobes: magic {m:02x?}, expected SHPB"),
            Self::Count(n) => write!(f, "shprobes: {n} records, triples need a multiple of 3"),
            Self::Length { expected, found } => {
                write!(f, "shprobes: {found} bytes, the count names {expected}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Every record of a circuit's `.shprobes`.
#[derive(Debug, Clone)]
pub struct ShProbes {
    /// Per record: the position, then the 27 coefficients.
    records: Vec<[f32; FLOATS]>,
}

/// Two records of one lane and the weight of the second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Taps {
    pub from: usize,
    pub to: usize,
    pub weight: f32,
}

impl ShProbes {
    /// Reads a whole `.shprobes` file.
    ///
    /// # Errors
    ///
    /// [`Error`] when the header or the length disagrees with the layout in the module doc.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let Some((magic, rest)) = bytes.split_first_chunk::<4>() else {
            return Err(Error::Truncated);
        };
        let Some((count, body)) = rest.split_first_chunk::<4>() else {
            return Err(Error::Truncated);
        };
        if *magic != MAGIC {
            return Err(Error::Magic(*magic));
        }
        let count = u32::from_le_bytes(*count) as usize;
        if !count.is_multiple_of(3) {
            return Err(Error::Count(count));
        }
        let expected = HEADER + count * RECORD;
        if bytes.len() < expected {
            return Err(Error::Length {
                expected,
                found: bytes.len(),
            });
        }
        let records = body[..count * RECORD]
            .as_chunks::<RECORD>()
            .0
            .iter()
            .map(|record| {
                let mut floats = [0.0f32; FLOATS];
                for (out, word) in floats.iter_mut().zip(record.as_chunks::<4>().0) {
                    *out = f32::from_le_bytes(*word);
                }
                floats
            })
            .collect();
        Ok(Self { records })
    }

    /// How many records the file holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Record `i`'s world position.
    #[must_use]
    pub fn position(&self, i: usize) -> [f32; 3] {
        let r = &self.records[i];
        [r[0], r[1], r[2]]
    }

    /// Record `i`'s nine rgb coefficients.
    #[must_use]
    pub fn coefficients(&self, i: usize) -> Coefficients {
        let mut out = [0.0; 27];
        out.copy_from_slice(&self.records[i][3..]);
        out
    }

    fn centre(&self, triple: usize) -> [f32; 3] {
        let at = |lane: usize| self.position(3 * triple + lane);
        let (a, b, c) = (at(0), at(1), at(2));
        [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ]
    }

    /// The two records `p` blends: the lane of the nearer triple that `p` is nearest, in the
    /// triple either side of `p` along the track, weighted by the projection of `p` on the
    /// line between the two (clamped to `0..=1`).
    ///
    /// An empty file returns a zero-weight tap on record 0.
    #[must_use]
    pub fn locate(&self, p: [f32; 3]) -> Taps {
        let triples = self.records.len() / 3;
        if triples == 0 {
            return Taps {
                from: 0,
                to: 0,
                weight: 0.0,
            };
        }
        let distance2 = |a: [f32; 3]| {
            let d = sub(a, p);
            crate::probes::dot(d, d)
        };
        let nearest = (0..triples)
            .map(|t| (t, distance2(self.centre(t))))
            .fold(
                (0, f32::INFINITY),
                |best, c| if c.1 < best.1 { c } else { best },
            )
            .0;
        let lane = (0..3)
            .map(|l| (l, distance2(self.position(3 * nearest + l))))
            .fold(
                (0, f32::INFINITY),
                |best, c| if c.1 < best.1 { c } else { best },
            )
            .0;
        let at = |t: usize| self.position(3 * t + lane);
        let (a, b) = if triples == 1 {
            (0, 0)
        } else if nearest + 1 == triples
            || (nearest > 0
                && crate::probes::dot(sub(p, at(nearest)), sub(at(nearest + 1), at(nearest))) < 0.0)
        {
            (nearest - 1, nearest)
        } else {
            (nearest, nearest + 1)
        };
        Taps {
            from: 3 * a + lane,
            to: 3 * b + lane,
            weight: project01(sub(p, at(a)), sub(at(b), at(a))),
        }
    }

    /// The blend of `taps`: `from + weight * (to - from)`, per coefficient.
    #[must_use]
    pub fn blend(&self, taps: Taps) -> Coefficients {
        let (a, b) = (self.coefficients(taps.from), self.coefficients(taps.to));
        let mut out = [0.0; 27];
        for k in 0..9 {
            let mixed = lerp(
                [a[3 * k], a[3 * k + 1], a[3 * k + 2]],
                [b[3 * k], b[3 * k + 1], b[3 * k + 2]],
                taps.weight,
            );
            out[3 * k..3 * k + 3].copy_from_slice(&mixed);
        }
        out
    }
}

#[cfg(test)]
mod tests;
