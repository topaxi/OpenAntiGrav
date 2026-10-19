//! HD's `.probes`: the baked environment probes a craft's hull is lit from.
//!
//! **The environment maps a Wipeout HD hull samples are disc data, not a render.**
//! The original's live `paraboloidIblTex` (64x32) and `paraboloidReflectionTex`
//! (128x64) are a weighted blend of a few neighbouring probes of this file,
//! picked by the craft's position and written by an SPU job. Measured on RPCS3
//! (Harimau on Talon's Junction): the live maps equal `0.652 * probe[852] +
//! 0.340 * probe[854]` to an RMS of 0.4-0.65 of 255, and eleven more samples of a
//! drive round the circuit fit the same way at 0.4-0.5.
//!
//! ```text
//! u32 BE  count           862 on Talon's Junction
//! u32 BE  16              the stride of the position table
//! f32 BE  position[count][4]       x y z z (the fourth repeats z)
//! u8      irradiance[count][1536]  64x32, 12 bytes per 4x4 block
//! u8      reflection[count][6144]  128x64, same block format
//! u8      tail[count * 31]         24 + 4 + 3 bytes per probe, unread here
//! ```
//!
//! Talon's Junction closes exactly at the tail; a longer circuit can carry more
//! after it (Sol 2's reversed file is 123,632 bytes over), which this reader
//! leaves alone: the loader (`0x003c1120`) reads the tail's arrays and stops.
//!
//! A block is two `BGR888` endpoints (6 bytes) then sixteen 3-bit indices, least
//! significant first, row-major inside the block; a texel is
//! `c0 + (c1 - c0) * index / 7`. Blocks run row-major across the image.
//!
//! The probes come in left/right pairs (entries `2k` and `2k + 1`, about 20 units
//! apart across the track), a section every 12-23 units along it. The craft's
//! weights are bilinear over the four probes of the two sections its position
//! falls between, clamped at the edges ([`Probes::locate`]).
//!
//! Confidence: layout, block format and the blend **85** (the file holds at least
//! the layout on all 28 files, closes exactly on Talon's Junction, and the decoded
//! blend reproduces the live maps); the locate law **70** (it fits six of twelve drive samples to 0.06 in
//! weight at one frame of position lag and the rest at zero or two frames of lag,
//! and one sample, at a corner, names a section pair this law does not).
//!
//! The tail (`24n + 4n + 3n` bytes) is not read: the first `3n` look like an
//! average colour per probe and the `24n` like twelve half floats. Nothing here
//! claims what they are.

use std::fmt;

/// The irradiance map's size in texels: `paraboloidIblTex`.
pub const IRRADIANCE_SIZE: (usize, usize) = (64, 32);
/// The reflection map's size in texels: `paraboloidReflectionTex`.
pub const REFLECTION_SIZE: (usize, usize) = (128, 64);

const IRRADIANCE_BYTES: usize = 1536;
const REFLECTION_BYTES: usize = 6144;
const TAIL_PER_PROBE: usize = 31;
const HEADER: usize = 8;
const STRIDE: u32 = 16;

/// Why a `.probes` file was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Shorter than its own header.
    Truncated,
    /// The second header word is not [`STRIDE`].
    Stride(u32),
    /// Shorter than the table, both image sets and the tail the loader reads.
    Length { expected: usize, found: usize },
    /// An odd count: probes come in left/right pairs.
    OddCount(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(f, "probes: shorter than the header"),
            Self::Stride(s) => write!(f, "probes: stride word {s}, expected {STRIDE}"),
            Self::Length { expected, found } => {
                write!(f, "probes: {found} bytes, the layout closes at {expected}")
            }
            Self::OddCount(n) => write!(
                f,
                "probes: {n} entries, left/right pairs need an even count"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// One probe set: every probe of a circuit direction.
#[derive(Debug, Clone)]
pub struct Probes {
    positions: Vec<[f32; 3]>,
    irradiance: Vec<u8>,
    reflection: Vec<u8>,
}

/// Up to four probes and the weights a position blends them with. The weights sum
/// to one; entries with weight zero are not listed.
#[derive(Debug, Clone, PartialEq)]
pub struct Taps {
    pub probes: [usize; 4],
    pub weights: [f32; 4],
}

fn be_u32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn be_f32(b: &[u8], at: usize) -> f32 {
    f32::from_bits(be_u32(b, at))
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mid(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ]
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// The fraction of `axis` that `from_origin` reaches, clamped to `0..=1`; a
/// degenerate axis reads as the start.
fn project01(from_origin: [f32; 3], axis: [f32; 3]) -> f32 {
    let len2 = dot(axis, axis);
    if len2 <= f32::EPSILON {
        return 0.0;
    }
    (dot(from_origin, axis) / len2).clamp(0.0, 1.0)
}

impl Probes {
    /// Reads a whole `.probes` file.
    ///
    /// # Errors
    ///
    /// [`Error`] when the header, the stride word or the total length disagree
    /// with the layout in the module doc.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < HEADER {
            return Err(Error::Truncated);
        }
        let count = be_u32(bytes, 0) as usize;
        let stride = be_u32(bytes, 4);
        if stride != STRIDE {
            return Err(Error::Stride(stride));
        }
        let expected = HEADER
            + count * (STRIDE as usize + IRRADIANCE_BYTES + REFLECTION_BYTES + TAIL_PER_PROBE);
        if bytes.len() < expected {
            return Err(Error::Length {
                expected,
                found: bytes.len(),
            });
        }
        if count % 2 != 0 {
            return Err(Error::OddCount(count));
        }
        let positions = (0..count)
            .map(|i| {
                let at = HEADER + i * STRIDE as usize;
                [
                    be_f32(bytes, at),
                    be_f32(bytes, at + 4),
                    be_f32(bytes, at + 8),
                ]
            })
            .collect();
        let irr_at = HEADER + count * STRIDE as usize;
        let refl_at = irr_at + count * IRRADIANCE_BYTES;
        Ok(Self {
            positions,
            irradiance: bytes[irr_at..refl_at].to_vec(),
            reflection: bytes[refl_at..refl_at + count * REFLECTION_BYTES].to_vec(),
        })
    }

    /// How many probes the file holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Probe `i`'s world position.
    #[must_use]
    pub fn position(&self, i: usize) -> [f32; 3] {
        self.positions[i]
    }

    fn sections(&self) -> usize {
        self.positions.len() / 2
    }

    fn centre(&self, section: usize) -> [f32; 3] {
        mid(self.positions[2 * section], self.positions[2 * section + 1])
    }

    /// The probes a position blends and their weights: bilinear over the two
    /// sections the position lies between, along the track by the projection on
    /// the line between the section centres and across it by the projection on
    /// the left-right axis, both clamped. Confidence 70, see the module doc; the
    /// original uses a position one to two frames old, which this does not.
    ///
    /// An empty set returns a single zero-weight tap on probe 0.
    #[must_use]
    pub fn locate(&self, p: [f32; 3]) -> Taps {
        let sections = self.sections();
        if sections == 0 {
            return Taps {
                probes: [0; 4],
                weights: [0.0; 4],
            };
        }
        let nearest = (0..sections)
            .map(|s| {
                let d = sub(self.centre(s), p);
                (s, dot(d, d))
            })
            .fold(
                (0, f32::INFINITY),
                |best, c| if c.1 < best.1 { c } else { best },
            )
            .0;
        let (a, b) = if sections == 1 {
            (0, 0)
        } else if nearest + 1 == sections
            || (nearest > 0
                && dot(
                    sub(p, self.centre(nearest)),
                    sub(self.centre(nearest + 1), self.centre(nearest)),
                ) < 0.0)
        {
            (nearest - 1, nearest)
        } else {
            (nearest, nearest + 1)
        };
        let u = project01(sub(p, self.centre(a)), sub(self.centre(b), self.centre(a)));
        let left = lerp(self.positions[2 * a], self.positions[2 * b], u);
        let right = lerp(self.positions[2 * a + 1], self.positions[2 * b + 1], u);
        let v = project01(sub(p, left), sub(right, left));
        Taps {
            probes: [2 * a, 2 * b, 2 * a + 1, 2 * b + 1],
            weights: [(1.0 - u) * (1.0 - v), u * (1.0 - v), (1.0 - u) * v, u * v],
        }
    }

    /// The blend of `taps` over the irradiance map, 64x32 `RGBA8`, alpha 255.
    #[must_use]
    pub fn blend_irradiance(&self, taps: &Taps) -> Vec<u8> {
        blend(&self.irradiance, IRRADIANCE_BYTES, IRRADIANCE_SIZE, taps)
    }

    /// The blend of `taps` over the reflection map, 128x64 `RGBA8`, alpha 255.
    #[must_use]
    pub fn blend_reflection(&self, taps: &Taps) -> Vec<u8> {
        blend(&self.reflection, REFLECTION_BYTES, REFLECTION_SIZE, taps)
    }

    /// Probe `i`'s irradiance map alone, 64x32 `RGBA8`.
    #[must_use]
    pub fn irradiance(&self, i: usize) -> Vec<u8> {
        decode(
            &self.irradiance[i * IRRADIANCE_BYTES..][..IRRADIANCE_BYTES],
            IRRADIANCE_SIZE.0,
        )
    }

    /// Probe `i`'s reflection map alone, 128x64 `RGBA8`.
    #[must_use]
    pub fn reflection(&self, i: usize) -> Vec<u8> {
        decode(
            &self.reflection[i * REFLECTION_BYTES..][..REFLECTION_BYTES],
            REFLECTION_SIZE.0,
        )
    }
}

/// Decodes one probe image of `width` texels to `RGBA8`.
fn decode(bytes: &[u8], width: usize) -> Vec<u8> {
    let blocks_per_row = width / 4;
    let height = bytes.len() / 12 / blocks_per_row * 4;
    let mut out = vec![255u8; width * height * 4];
    for (block, data) in bytes.chunks_exact(12).enumerate() {
        let (bx, by) = (block % blocks_per_row, block / blocks_per_row);
        let c0 = [f32::from(data[2]), f32::from(data[1]), f32::from(data[0])];
        let c1 = [f32::from(data[5]), f32::from(data[4]), f32::from(data[3])];
        let mut bits = 0u64;
        for (k, &byte) in data[6..12].iter().enumerate() {
            bits |= u64::from(byte) << (8 * k);
        }
        for t in 0..16 {
            let index = ((bits >> (3 * t)) & 7) as f32 / 7.0;
            let at = ((by * 4 + t / 4) * width + bx * 4 + t % 4) * 4;
            for c in 0..3 {
                out[at + c] = (c0[c] + (c1[c] - c0[c]) * index + 0.5) as u8;
            }
        }
    }
    out
}

fn blend(set: &[u8], bytes: usize, size: (usize, usize), taps: &Taps) -> Vec<u8> {
    let mut acc = vec![0.0f32; size.0 * size.1 * 4];
    for (&probe, &weight) in taps.probes.iter().zip(&taps.weights) {
        if weight <= 0.0 {
            continue;
        }
        let image = decode(&set[probe * bytes..][..bytes], size.0);
        for (a, &texel) in acc.iter_mut().zip(&image) {
            *a += f32::from(texel) * weight;
        }
    }
    let total: f32 = taps.weights.iter().sum();
    let norm = if total > 0.0 { 1.0 / total } else { 0.0 };
    acc.chunks_exact(4)
        .flat_map(|t| {
            [
                (t[0] * norm + 0.5) as u8,
                (t[1] * norm + 0.5) as u8,
                (t[2] * norm + 0.5) as u8,
                255,
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests;
