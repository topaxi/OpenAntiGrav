//! `WO Track`: the AI track spline graph, the thing a lap is actually made of.
//!
//! One node in a track's [`.vex`](crate::vex) file carries the whole driveable
//! path as a **graph of uniform cubic B-splines**. Each control point carries a
//! position, a full orientation frame, the track's half-width either side, an
//! explicit racing line and an AI corridor around it.
//!
//! ```text
//! payload, at the WO Track node's data:
//!   +0x00  u32    magic, 0x574f7464
//!   +0x04  u32    version, 0x105 in Pulse
//!   +0x08  u32    path count
//!   +0x0c  u32    junction count
//!   +0x10  u32    paths pointer, zero on disc
//!   +0x14  u32    junctions pointer, zero on disc
//!   +0x18  u32    1
//!   +0x1c  u32    0
//!   +0x20         reserved, 0x20 bytes, only when version >= 0x101
//!         then    paths,     0x20 bytes each
//!         then    junctions, 0x10 bytes each
//!         then    each path's control points in path order, 0x70 bytes each
//! ```
//!
//! # The reserved block is the whole trick
//!
//! Both array pointers are **zero on disc**: they are runtime pointers, patched
//! in as the loader walks a cursor through the payload. So the arrays are
//! positioned by a rule rather than read, and the rule has one non-obvious step.
//! For version `0x101` and up the loader claims **a second 0x20-byte block**
//! before the paths and hands its address to the track object, which pushes
//! everything after it along by 32 bytes.
//!
//! Reading the paths at `+0x20` instead of `+0x40` lands one field early, which
//! puts plausible-looking world coordinates in the orientation frame and reads a
//! `section_id` of 185 on a track that has 64 sections. It looks like a subtly
//! wrong struct rather than a wrong offset, which is what makes it expensive.
//!
//! # Why the details matter
//!
//! **Positions on disc sit on the track surface.** The load pass does
//! `pos -= 3 * down`, lifting each control point by [`HOVER_LIFT`]. Ships fly
//! along the lifted line; the surface line is what the exporter wrote. Both are
//! available here: [`SplinePoint::pos`] is the disc value and
//! [`SplinePoint::lifted_pos`] applies the lift.
//!
//! **The frame's second vector points down, not up.** On level ground it is
//! exactly `(0, -1, 0)`, and `+y` is world up. Calling it "up" and then lifting
//! along it moves the racing line *into* the track.
//!
//! **Junction slots are 2 in, 2 out.** Slots 0 and 1 are predecessors, 2 and 3
//! successors, and `0x7fffffff` is null. A shortcut is an alternate successor,
//! not a separate structure.
//!
//! See `docs/formats/track.md` for the evidence and the confidence scores.

use std::fmt;

use crate::vex;

/// Bytes of `WO Track` header.
pub const HEADER_LEN: usize = 0x20;

/// The reserved block between the header and the paths, from version `0x101`.
pub const RESERVED_LEN: usize = 0x20;

/// Bytes per path.
pub const PATH_LEN: usize = 0x20;

/// Bytes per junction.
pub const JUNCTION_LEN: usize = 0x10;

/// Bytes per spline control point.
pub const POINT_LEN: usize = 0x70;

/// Magic at `+0x00`, `WOtd` read big-endian.
pub const MAGIC: u32 = 0x574f_7464;

/// Lowest version the loader accepts.
pub const MIN_VERSION: u32 = 0x100;

/// How far the load pass lifts each control point along the frame's down axis.
pub const HOVER_LIFT: f32 = 3.0;

/// A null path or junction index.
const NULL_INDEX: u32 = 0x7fff_ffff;

/// Something wrong with a `WO Track` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not a `WO Track` payload.
    BadMagic {
        /// The value found at `+0x00`.
        magic: u32,
    },
    /// A version the loader itself would reject.
    UnsupportedVersion {
        /// The value found at `+0x04`.
        version: u32,
    },
    /// A structure runs past the end of the payload.
    OutOfBounds {
        /// What was being read.
        what: &'static str,
        /// Where it would end.
        end: usize,
        /// Bytes available.
        len: usize,
    },
    /// A path or junction index that does not name anything.
    BadIndex {
        /// What was being read.
        what: &'static str,
        /// The value found.
        index: u32,
        /// One past the largest valid value.
        count: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic { magic } => {
                write!(f, "expected magic {MAGIC:#010x}, got {magic:#010x}")
            }
            Self::UnsupportedVersion { version } => {
                write!(f, "version {version:#x} is below {MIN_VERSION:#x}")
            }
            Self::OutOfBounds { what, end, len } => {
                write!(f, "{what} ends at {end} but the payload is {len} bytes")
            }
            Self::BadIndex { what, index, count } => {
                write!(f, "{what} is {index}, but there are only {count}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// One control point of a spline path.
///
/// The four vectors are an orthonormal frame. `tangent` runs along the path,
/// `lateral` across it, and `down` is the surface normal negated: see the module
/// docs before assuming it points up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplinePoint {
    /// Control point, on the track surface. See [`Self::lifted_pos`].
    pub pos: [f32; 3],
    /// Unit vector along the path.
    pub tangent: [f32; 3],
    /// Unit vector into the track surface, `(0, -1, 0)` on level ground.
    pub down: [f32; 3],
    /// Unit vector across the path.
    pub lateral: [f32; 3],
    /// Track half-width to the left of the centre line.
    pub half_width_left: f32,
    /// Track half-width to the right.
    pub half_width_right: f32,
    /// Left edge of the AI corridor, as a lateral offset.
    pub ai_bound_left: f32,
    /// Right edge of the AI corridor.
    pub ai_bound_right: f32,
    /// The authored racing line, as a lateral offset from the centre.
    pub racing_line: f32,
    /// Which [`section`](crate::vex) this point belongs to, for visibility.
    pub section_id: u8,
    /// Flags, OR-accumulated across the four control points of a segment.
    pub flags: u8,
}

impl SplinePoint {
    /// The control point as the running game sees it, lifted to hover height.
    #[must_use]
    pub fn lifted_pos(&self) -> [f32; 3] {
        [
            self.pos[0] - HOVER_LIFT * self.down[0],
            self.pos[1] - HOVER_LIFT * self.down[1],
            self.pos[2] - HOVER_LIFT * self.down[2],
        ]
    }
}

/// Interpolated spline state, from [`Path::sample`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Position on the curve, on the surface line rather than the lifted one.
    pub pos: [f32; 3],
    /// Interpolated tangent. Not renormalised, matching the original.
    pub tangent: [f32; 3],
    /// Interpolated down axis.
    pub down: [f32; 3],
    /// Interpolated lateral axis.
    pub lateral: [f32; 3],
    /// Interpolated half-width to the left.
    pub half_width_left: f32,
    /// Interpolated half-width to the right.
    pub half_width_right: f32,
    /// Interpolated left edge of the AI corridor.
    pub ai_bound_left: f32,
    /// Interpolated right edge of the AI corridor.
    pub ai_bound_right: f32,
    /// Interpolated racing-line offset.
    pub racing_line: f32,
    /// Section of the control point the parameter sits on.
    pub section_id: u8,
    /// Flags OR-ed across the four control points of the segment.
    pub flags: u8,
}

/// One spline path: a run of control points between two junctions.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Control points, in order.
    pub points: Vec<SplinePoint>,
    /// The longest gap between consecutive control points in this path.
    ///
    /// Verified as exactly that on all 86 paths of the 40 PSP track files,
    /// which is why it is named rather than left as an unknown word.
    pub max_spacing: f32,
    /// Junction this path leaves from, if any.
    pub entry: Option<usize>,
    /// Junction this path arrives at, if any.
    pub exit: Option<usize>,
}

/// A 2-in, 2-out merge and split between paths.
///
/// A plain ring track is the degenerate case: one predecessor, one successor,
/// both alternates null.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Junction {
    /// Paths arriving here: primary, then alternate.
    pub prev: [Option<usize>; 2],
    /// Paths leaving here: primary, then alternate.
    pub next: [Option<usize>; 2],
}

/// A decoded `WO Track` payload.
#[derive(Debug, Clone, PartialEq)]
pub struct AiTrack {
    /// Format version from the header.
    pub version: u32,
    /// The paths, in file order. Junctions index into this.
    pub paths: Vec<Path>,
    /// The junctions, in file order. Paths index into this.
    pub junctions: Vec<Junction>,
}

impl AiTrack {
    /// Bytes the decoded structure accounts for.
    ///
    /// Equal to the payload length on every shipped track, which is the
    /// arithmetic self-check that settled the layout. A parser that is one
    /// structure out cannot make this come out even.
    #[must_use]
    pub fn encoded_len(&self) -> usize {
        let points: usize = self.paths.iter().map(|p| p.points.len()).sum();
        HEADER_LEN
            + reserved_len(self.version)
            + self.paths.len() * PATH_LEN
            + self.junctions.len() * JUNCTION_LEN
            + points * POINT_LEN
    }

    /// Total control points across every path.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.paths.iter().map(|p| p.points.len()).sum()
    }
}

impl Path {
    /// Evaluates the curve inside `segment`, at `t` in `0..=1`.
    ///
    /// A **uniform cubic B-spline** over control points `segment - 1 ..=
    /// segment + 2`, clamped at the ends of the path. The basis blends the
    /// scalars as well as the vectors, so width and racing line curve too.
    ///
    /// Note that a B-spline does not pass through its control points, so
    /// `sample(i, 0.0)` is not `points[i]`. That is the original's behaviour,
    /// not a rounding artefact.
    ///
    /// Returns `None` for an empty path or a segment past the end.
    #[must_use]
    pub fn sample(&self, segment: usize, t: f32) -> Option<Sample> {
        let n = self.points.len();
        if n == 0 || segment >= n {
            return None;
        }

        let at = |i: isize| -> &SplinePoint {
            let clamped = i.clamp(0, n as isize - 1) as usize;
            &self.points[clamped]
        };
        let i = segment as isize;
        let p = [at(i - 1), at(i), at(i + 1), at(i + 2)];
        let w = basis(t);

        let blend3 = |f: fn(&SplinePoint) -> [f32; 3]| -> [f32; 3] {
            let mut out = [0.0; 3];
            for (weight, point) in w.iter().zip(p.iter()) {
                let v = f(point);
                for k in 0..3 {
                    out[k] += weight * v[k];
                }
            }
            out
        };
        let blend1 = |f: fn(&SplinePoint) -> f32| -> f32 {
            w.iter().zip(p.iter()).map(|(a, b)| a * f(b)).sum()
        };

        Some(Sample {
            pos: blend3(|p| p.pos),
            tangent: blend3(|p| p.tangent),
            down: blend3(|p| p.down),
            lateral: blend3(|p| p.lateral),
            half_width_left: blend1(|p| p.half_width_left),
            half_width_right: blend1(|p| p.half_width_right),
            ai_bound_left: blend1(|p| p.ai_bound_left),
            ai_bound_right: blend1(|p| p.ai_bound_right),
            racing_line: blend1(|p| p.racing_line),
            section_id: p[1].section_id,
            flags: p.iter().fold(0, |acc, point| acc | point.flags),
        })
    }
}

/// The uniform cubic B-spline basis, weights for four control points.
///
/// `[(1-t)^3, 3t^3 - 6t^2 + 4, -3t^3 + 3t^2 + 3t + 1, t^3] / 6`. The constants
/// appear as VFPU immediates in the original's evaluator.
#[must_use]
pub fn basis(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    let sixth = 1.0 / 6.0;
    [
        (1.0 - t) * (1.0 - t) * (1.0 - t) * sixth,
        (3.0 * t3 - 6.0 * t2 + 4.0) * sixth,
        (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) * sixth,
        t3 * sixth,
    ]
}

/// Bytes of a `Start Position` node's payload: a row-major 4x4 matrix.
pub const START_POSITION_LEN: usize = 64;

/// Where a track says a ship begins, after the bind's own fix-up.
///
/// The payload is an authored 4x4 transform whose rows are the same
/// **left-up-forward** basis the running craft carries, and which the bind
/// handler (`0x08926ae8`) does not use as authored: it forces the up row to
/// world `(0, 1, 0)` and re-orthonormalises around it. [`start_position`]
/// reproduces that, so this is the frame the *game* starts from rather than the
/// frame the exporter wrote.
///
/// **One per track, and it is not the centreline.** All 40 PSP track files carry
/// exactly one `Start Position` node, and on every one of them it sits between
/// 3.3 and 20.5 units off the spline's own centreline - so it is a grid *slot*,
/// not a start-line marker, and the other seven slots are laid out by code that
/// has not been recovered. See `docs/formats/track.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StartPosition {
    /// World-space position of the slot.
    pub position: [f32; 3],
    /// The slot's left axis, unit length and perpendicular to [`Self::up`].
    pub left: [f32; 3],
    /// World up, `(0, 1, 0)`, which the bind forces rather than reads.
    pub up: [f32; 3],
    /// The slot's forward axis, unit length and perpendicular to [`Self::up`].
    pub forward: [f32; 3],
}

/// Decodes a `Start Position` node payload, applying the bind's fix-up.
///
/// `None` when the payload is not 64 bytes, or when the authored forward axis is
/// vertical and so has nothing left of it once the up component is removed -
/// degenerate rather than merely unusual, and no shipped track has one.
///
/// # What is reproduced, and what was assumed
///
/// The forced up row is read: `docs/formats/track.md` has it from the bind
/// handler, corroborated by the frame conventions everywhere else on that page.
/// **Which of the remaining two rows the original preserves was not read**, and
/// this keeps *forward*, on the grounds that where a ship points is the part of
/// a grid slot that is authored deliberately.
///
/// How much that choice can matter is bounded by the data rather than argued:
/// the authored up row is *already* world up on 31 of the 40 shipped track
/// files, and off it by at most **1.600 degrees** on the other nine, so any
/// re-orthonormalisation that forces up and keeps the frame right-handed lands
/// within that angle of any other. The fix-up is nonetheless not a no-op, which
/// is why it is applied rather than skipped.
#[must_use]
pub fn start_position(payload: &[u8]) -> Option<StartPosition> {
    if payload.len() < START_POSITION_LEN {
        return None;
    }
    let row = |r: usize| {
        [
            f32_at(payload, r * 16),
            f32_at(payload, r * 16 + 4),
            f32_at(payload, r * 16 + 8),
        ]
    };

    let up = [0.0, 1.0, 0.0];
    let authored = row(2);
    // Gram-Schmidt against an axis that is exactly `+y`, so the projection is
    // just the y component.
    let forward = normalize([authored[0], 0.0, authored[2]])?;
    let left = cross(up, forward);

    Some(StartPosition {
        position: row(3),
        left,
        up,
        forward,
    })
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> Option<[f32; 3]> {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length > 1e-6 {
        Some([v[0] / length, v[1] / length, v[2] / length])
    } else {
        None
    }
}

/// The `WO Track` node in an already-walked scene tree, found by the class id
/// the file's own version word implies.
///
/// Eleven call sites used to spell `n.class_id == vex::CLASS_WO_TRACK` by hand,
/// which is the version-6 id and finds nothing in a version-4 file. The payload
/// parser here has never needed that fix - [`MIN_VERSION`] is `0x100`, so
/// Pure's `0x103` payloads were always inside what it accepts. Only *finding*
/// the node was version-locked.
///
/// **All eleven are converted as of 2026-08-09**, and one of them was not a
/// test: `oag_trace`'s own `ai_of` looked the node up by the version-6 constant,
/// so `oag-trace run --source` against a Pure disc would have reported "has no
/// WO Track node" for a track that has one. Two spellings survive on purpose -
/// `pvs_ground_truth::pure_does_not_share_pulses_class_numbering` counts nodes
/// matching *Pulse's* id on Pure's disc, which is the whole claim it makes, and
/// `track_ground_truth` restates `0x3bb` from `docs/formats/track.md` so the
/// test checks the documented id rather than the crate's.
///
/// `None` for a file whose version has no class table, or whose table has no
/// `WO Track` id recovered, as well as for a `.vex` that simply authors none.
/// Those are not distinguishable here, and a caller that needs to tell them
/// apart should ask [`vex::classes_of`] itself.
#[must_use]
pub fn find_node<'a>(file: &[u8], nodes: &'a [vex::Node]) -> Option<&'a vex::Node> {
    let wo_track = vex::classes_of(file).ok()?.wo_track?;
    nodes.iter().find(|node| node.class_id == wo_track)
}

/// Bytes of reserved block for a given version.
#[must_use]
pub fn reserved_len(version: u32) -> usize {
    if version >= 0x101 { RESERVED_LEN } else { 0 }
}

/// Whether a payload carries the `WO Track` magic.
#[must_use]
pub fn has_magic(payload: &[u8]) -> bool {
    payload.len() >= 4 && u32_at(payload, 0) == MAGIC
}

/// Decodes a `WO Track` node payload.
///
/// # Errors
///
/// Refuses a wrong magic, a version the game's own loader would reject, any
/// structure that runs past the end of the payload, and any path or junction
/// index that names nothing. Trailing bytes are *not* an error: the loader does
/// not check, and neither does this.
pub fn parse(payload: &[u8]) -> Result<AiTrack> {
    if payload.len() < HEADER_LEN {
        return Err(Error::OutOfBounds {
            what: "header",
            end: HEADER_LEN,
            len: payload.len(),
        });
    }

    let magic = u32_at(payload, 0);
    if magic != MAGIC {
        return Err(Error::BadMagic { magic });
    }
    let version = u32_at(payload, 4);
    if version < MIN_VERSION {
        return Err(Error::UnsupportedVersion { version });
    }

    let path_count = u32_at(payload, 8) as usize;
    let junction_count = u32_at(payload, 12) as usize;

    let paths_at = HEADER_LEN + reserved_len(version);
    let junctions_at = end_of("paths", paths_at, path_count, PATH_LEN, payload.len())?;
    let points_at = end_of(
        "junctions",
        junctions_at,
        junction_count,
        JUNCTION_LEN,
        payload.len(),
    )?;

    // Read the counts and check every point block fits before allocating
    // anything: the counts are u32 from an untrusted file, and reserving on
    // trust is how a 2 GiB allocation happens.
    let mut counts = Vec::with_capacity(path_count.min(payload.len() / PATH_LEN));
    let mut at = points_at;
    for i in 0..path_count {
        let count = u32_at(payload, paths_at + i * PATH_LEN) as usize;
        at = end_of("control points", at, count, POINT_LEN, payload.len())?;
        counts.push(count);
    }

    let mut junctions = Vec::with_capacity(junction_count);
    for j in 0..junction_count {
        let base = junctions_at + j * JUNCTION_LEN;
        let slot = |k: usize| index_at(payload, base + k * 4, "junction slot", path_count);
        junctions.push(Junction {
            prev: [slot(0)?, slot(1)?],
            next: [slot(2)?, slot(3)?],
        });
    }

    let mut paths = Vec::with_capacity(path_count);
    let mut points_from = points_at;
    for (i, &count) in counts.iter().enumerate() {
        let base = paths_at + i * PATH_LEN;
        let mut points = Vec::with_capacity(count);
        for k in 0..count {
            points.push(point_at(payload, points_from + k * POINT_LEN));
        }
        points_from += count * POINT_LEN;

        paths.push(Path {
            points,
            max_spacing: f32_at(payload, base + 4),
            entry: index_at(payload, base + 0x0c, "path entry", junction_count)?,
            exit: index_at(payload, base + 0x10, "path exit", junction_count)?,
        });
    }

    Ok(AiTrack {
        version,
        paths,
        junctions,
    })
}

/// End of an array, or an error naming what did not fit.
fn end_of(what: &'static str, at: usize, count: usize, stride: usize, len: usize) -> Result<usize> {
    let end = count
        .checked_mul(stride)
        .and_then(|bytes| at.checked_add(bytes))
        .ok_or(Error::OutOfBounds {
            what,
            end: usize::MAX,
            len,
        })?;
    if end > len {
        return Err(Error::OutOfBounds { what, end, len });
    }
    Ok(end)
}

/// Reads one index, mapping the null sentinel to `None`.
fn index_at(payload: &[u8], at: usize, what: &'static str, count: usize) -> Result<Option<usize>> {
    let raw = u32_at(payload, at);
    if raw == NULL_INDEX {
        return Ok(None);
    }
    if raw as usize >= count {
        return Err(Error::BadIndex {
            what,
            index: raw,
            count,
        });
    }
    Ok(Some(raw as usize))
}

fn point_at(payload: &[u8], at: usize) -> SplinePoint {
    SplinePoint {
        pos: vec3_at(payload, at),
        tangent: vec3_at(payload, at + 0x10),
        down: vec3_at(payload, at + 0x20),
        lateral: vec3_at(payload, at + 0x30),
        half_width_left: f32_at(payload, at + 0x44),
        half_width_right: f32_at(payload, at + 0x48),
        ai_bound_left: f32_at(payload, at + 0x4c),
        ai_bound_right: f32_at(payload, at + 0x50),
        racing_line: f32_at(payload, at + 0x54),
        section_id: payload[at + 0x60],
        flags: payload[at + 0x61],
    }
}

fn vec3_at(payload: &[u8], at: usize) -> [f32; 3] {
    [
        f32_at(payload, at),
        f32_at(payload, at + 4),
        f32_at(payload, at + 8),
    ]
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn f32_at(data: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(data, at))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a payload with `paths` control-point counts, one junction per
    /// path, wired as a ring.
    fn build(version: u32, counts: &[usize]) -> Vec<u8> {
        let reserved = reserved_len(version);
        let paths_at = HEADER_LEN + reserved;
        let junctions_at = paths_at + counts.len() * PATH_LEN;
        let points_at = junctions_at + counts.len() * JUNCTION_LEN;
        let total: usize = counts.iter().sum();

        let mut out = vec![0u8; points_at + total * POINT_LEN];
        out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        out[4..8].copy_from_slice(&version.to_le_bytes());
        out[8..12].copy_from_slice(&(counts.len() as u32).to_le_bytes());
        out[12..16].copy_from_slice(&(counts.len() as u32).to_le_bytes());

        for (i, &count) in counts.iter().enumerate() {
            let base = paths_at + i * PATH_LEN;
            out[base..base + 4].copy_from_slice(&(count as u32).to_le_bytes());
            out[base + 4..base + 8].copy_from_slice(&7.5f32.to_le_bytes());
            // entry is the junction before this path, exit the one after.
            let entry = (i + counts.len() - 1) % counts.len();
            out[base + 0x0c..base + 0x10].copy_from_slice(&(entry as u32).to_le_bytes());
            out[base + 0x10..base + 0x14].copy_from_slice(&(i as u32).to_le_bytes());
        }

        for j in 0..counts.len() {
            let base = junctions_at + j * JUNCTION_LEN;
            let next = (j + 1) % counts.len();
            out[base..base + 4].copy_from_slice(&(j as u32).to_le_bytes());
            out[base + 4..base + 8].copy_from_slice(&NULL_INDEX.to_le_bytes());
            out[base + 8..base + 12].copy_from_slice(&(next as u32).to_le_bytes());
            out[base + 12..base + 16].copy_from_slice(&NULL_INDEX.to_le_bytes());
        }

        let mut at = points_at;
        for (i, &count) in counts.iter().enumerate() {
            for k in 0..count {
                // Position walks along +x so segment lengths are checkable.
                write_vec3(&mut out, at, [k as f32 * 5.0, 0.0, i as f32 * 100.0]);
                write_vec3(&mut out, at + 0x10, [1.0, 0.0, 0.0]);
                write_vec3(&mut out, at + 0x20, [0.0, -1.0, 0.0]);
                write_vec3(&mut out, at + 0x30, [0.0, 0.0, 1.0]);
                out[at + 0x44..at + 0x48].copy_from_slice(&20.0f32.to_le_bytes());
                out[at + 0x48..at + 0x4c].copy_from_slice(&20.0f32.to_le_bytes());
                out[at + 0x4c..at + 0x50].copy_from_slice(&(-9.0f32).to_le_bytes());
                out[at + 0x50..at + 0x54].copy_from_slice(&9.0f32.to_le_bytes());
                out[at + 0x54..at + 0x58].copy_from_slice(&0.5f32.to_le_bytes());
                out[at + 0x60] = k as u8;
                out[at + 0x61] = 1 << i;
                at += POINT_LEN;
            }
        }
        out
    }

    fn write_vec3(out: &mut [u8], at: usize, v: [f32; 3]) {
        for (k, c) in v.iter().enumerate() {
            out[at + k * 4..at + k * 4 + 4].copy_from_slice(&c.to_le_bytes());
        }
    }

    #[test]
    fn parses_a_two_path_ring() {
        let payload = build(0x105, &[3, 2]);
        let track = parse(&payload).expect("parse");

        assert_eq!(track.version, 0x105);
        assert_eq!(track.paths.len(), 2);
        assert_eq!(track.junctions.len(), 2);
        assert_eq!(track.paths[0].points.len(), 3);
        assert_eq!(track.paths[1].points.len(), 2);
        assert_eq!(track.point_count(), 5);
        assert_eq!(track.paths[0].entry, Some(1));
        assert_eq!(track.paths[0].exit, Some(0));
        assert_eq!(track.junctions[0].prev, [Some(0), None]);
        assert_eq!(track.junctions[0].next, [Some(1), None]);
    }

    /// The check that resolved the layout: a correct parse accounts for every
    /// byte. This is the synthetic version of the same test the ground-truth
    /// suite runs against all 40 shipped tracks.
    #[test]
    fn the_decoded_structure_accounts_for_every_byte() {
        for counts in [vec![1], vec![3, 2], vec![4, 4, 9]] {
            let payload = build(0x105, &counts);
            let track = parse(&payload).expect("parse");
            assert_eq!(track.encoded_len(), payload.len(), "counts {counts:?}");
        }
    }

    /// Version 0x100 has no reserved block, so every array moves 32 bytes
    /// earlier. Getting this backwards is the error the module docs warn about.
    #[test]
    fn the_reserved_block_exists_only_from_version_0x101() {
        assert_eq!(reserved_len(0x100), 0);
        assert_eq!(reserved_len(0x101), RESERVED_LEN);
        assert_eq!(reserved_len(0x105), RESERVED_LEN);

        let old = parse(&build(0x100, &[3, 2])).expect("parse 0x100");
        let new = parse(&build(0x105, &[3, 2])).expect("parse 0x105");
        assert_eq!(old.paths, new.paths);
        assert_eq!(new.encoded_len(), old.encoded_len() + RESERVED_LEN);
    }

    /// Reading the paths at `+0x20` on a `0x105` file is the mistake the module
    /// docs warn about, so pin that it cannot pass silently: either an index
    /// lands outside its array, or the structure stops accounting for every
    /// byte. On a real track it is the second one, which is why
    /// [`AiTrack::encoded_len`] exists.
    #[test]
    fn skipping_the_reserved_block_is_detectable() {
        let payload = build(0x105, &[3, 2]);
        let mut shifted = payload.clone();
        // Claiming 0x100 moves every array back by the reserved block without
        // touching a byte, which is exactly the misreading.
        shifted[4..8].copy_from_slice(&0x100u32.to_le_bytes());
        match parse(&shifted) {
            Err(_) => {}
            Ok(wrong) => assert_ne!(
                wrong.encoded_len(),
                shifted.len(),
                "a shifted parse must not look self-consistent"
            ),
        }
    }

    #[test]
    fn a_null_junction_slot_reads_as_none() {
        let track = parse(&build(0x105, &[2, 2])).expect("parse");
        assert_eq!(track.junctions[0].prev[1], None);
        assert_eq!(track.junctions[0].next[1], None);
    }

    #[test]
    fn rejects_a_wrong_magic() {
        let mut payload = build(0x105, &[2]);
        payload[0] = 0;
        assert!(matches!(parse(&payload), Err(Error::BadMagic { .. })));
    }

    #[test]
    fn rejects_a_version_the_game_would_reject() {
        let mut payload = build(0x105, &[2]);
        payload[4..8].copy_from_slice(&0xffu32.to_le_bytes());
        assert_eq!(
            parse(&payload),
            Err(Error::UnsupportedVersion { version: 0xff })
        );
    }

    #[test]
    fn rejects_a_truncated_payload() {
        let payload = build(0x105, &[3, 2]);
        for len in [0, 4, HEADER_LEN, HEADER_LEN + 8, payload.len() - 1] {
            assert!(
                matches!(parse(&payload[..len]), Err(Error::OutOfBounds { .. })),
                "truncating to {len} should not parse"
            );
        }
    }

    /// A hostile count must not reach a `Vec` reservation. `0x7fffffff` points
    /// would be 224 GiB.
    #[test]
    fn rejects_a_huge_point_count_before_allocating() {
        let mut payload = build(0x105, &[2]);
        let base = HEADER_LEN + RESERVED_LEN;
        payload[base..base + 4].copy_from_slice(&0x7fff_ffffu32.to_le_bytes());
        assert!(matches!(
            parse(&payload),
            Err(Error::OutOfBounds {
                what: "control points",
                ..
            })
        ));
    }

    #[test]
    fn rejects_a_huge_path_count_before_allocating() {
        let mut payload = build(0x105, &[2]);
        payload[8..12].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        assert!(matches!(
            parse(&payload),
            Err(Error::OutOfBounds { what: "paths", .. })
        ));
    }

    #[test]
    fn rejects_an_index_that_names_nothing() {
        let mut payload = build(0x105, &[2, 2]);
        let base = HEADER_LEN + RESERVED_LEN;
        payload[base + 0x0c..base + 0x10].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(
            parse(&payload),
            Err(Error::BadIndex {
                what: "path entry",
                index: 9,
                count: 2
            })
        );
    }

    #[test]
    fn has_magic_is_cheap_and_does_not_panic_on_short_input() {
        assert!(has_magic(&build(0x105, &[1])));
        assert!(!has_magic(&[]));
        assert!(!has_magic(&[0x64, 0x74, 0x4f]));
    }

    #[test]
    fn the_lift_moves_the_point_up_when_down_is_negative_y() {
        let track = parse(&build(0x105, &[1])).expect("parse");
        let p = track.paths[0].points[0];
        assert_eq!(p.down, [0.0, -1.0, 0.0]);
        assert_eq!(p.lifted_pos(), [p.pos[0], p.pos[1] + HOVER_LIFT, p.pos[2]]);
    }

    #[test]
    fn the_basis_is_a_partition_of_unity() {
        for step in 0..=10 {
            let t = step as f32 / 10.0;
            let w = basis(t);
            let sum: f32 = w.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6, "t={t} sum={sum}");
            assert!(w.iter().all(|&x| x >= 0.0), "t={t} has a negative weight");
        }
    }

    /// A B-spline is C2 continuous, so the end of one segment is the start of
    /// the next. If the basis or the control-point window were wrong, this is
    /// where it would show.
    #[test]
    fn segments_join_up() {
        let track = parse(&build(0x105, &[6])).expect("parse");
        let path = &track.paths[0];
        for segment in 0..4 {
            let a = path.sample(segment, 1.0).expect("sample");
            let b = path.sample(segment + 1, 0.0).expect("sample");
            for k in 0..3 {
                assert!(
                    (a.pos[k] - b.pos[k]).abs() < 1e-4,
                    "segment {segment} axis {k}: {} vs {}",
                    a.pos[k],
                    b.pos[k]
                );
            }
            assert!((a.half_width_left - b.half_width_left).abs() < 1e-4);
        }
    }

    /// On a straight, evenly spaced path the curve lies on the same line as the
    /// control points, so the sample can be checked against the exact value.
    #[test]
    fn sampling_a_straight_path_lands_on_the_line() {
        let track = parse(&build(0x105, &[5])).expect("parse");
        let path = &track.paths[0];
        // Control points sit at x = 0, 5, 10, 15, 20. With a uniform basis the
        // curve at t=0 in segment 2 is the average of its neighbours, weighted
        // 1/6, 4/6, 1/6, which for equal spacing is the control point itself.
        let s = path.sample(2, 0.0).expect("sample");
        assert!((s.pos[0] - 10.0).abs() < 1e-4, "got {}", s.pos[0]);
        assert!((s.pos[1]).abs() < 1e-6);
        // Halfway between control points 2 and 3.
        let mid = path.sample(2, 0.5).expect("sample");
        assert!((mid.pos[0] - 12.5).abs() < 1e-4, "got {}", mid.pos[0]);
    }

    #[test]
    fn sample_refuses_a_segment_past_the_end() {
        let track = parse(&build(0x105, &[2])).expect("parse");
        assert!(track.paths[0].sample(2, 0.0).is_none());
    }

    #[test]
    fn sample_or_accumulates_flags_across_the_window() {
        // Two paths, each with a distinct flag bit; within one path every point
        // carries the same bit, so the OR is that bit.
        let track = parse(&build(0x105, &[4, 4])).expect("parse");
        assert_eq!(track.paths[0].sample(1, 0.3).expect("sample").flags, 0b01);
        assert_eq!(track.paths[1].sample(1, 0.3).expect("sample").flags, 0b10);
    }

    /// Builds a `Start Position` payload from four rows.
    fn slot(rows: [[f32; 3]; 4]) -> Vec<u8> {
        let mut out = vec![0u8; START_POSITION_LEN];
        for (r, row) in rows.iter().enumerate() {
            for (c, value) in row.iter().enumerate() {
                let at = r * 16 + c * 4;
                out[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
            out[r * 16 + 12..r * 16 + 16].copy_from_slice(&f32::to_le_bytes(if r == 3 {
                1.0
            } else {
                0.0
            }));
        }
        out
    }

    #[test]
    fn a_start_position_reads_its_rows_as_left_up_forward() {
        // The identity every shipped slot satisfies exactly: cross(left, up) is
        // forward. A frame facing `+x` with `-z` to its left is the one
        // `16_Track` ships.
        let position = start_position(&slot([
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            [8.0, -50.0, -196.0],
        ]))
        .expect("a non-degenerate frame");

        assert_eq!(position.position, [8.0, -50.0, -196.0]);
        assert_eq!(position.forward, [1.0, 0.0, 0.0]);
        assert_eq!(position.up, [0.0, 1.0, 0.0]);
        assert_eq!(position.left, [0.0, 0.0, -1.0]);
    }

    /// The bind forces up rather than reading it, so an authored frame that is
    /// tilted comes back level - and the forward axis comes back perpendicular
    /// to the up it was given, not to the one it was written with.
    #[test]
    fn the_bind_levels_a_tilted_slot() {
        let tilt = 0.25f32;
        let position = start_position(&slot([
            [0.0, 0.0, -1.0],
            [-tilt, 1.0, 0.0],
            [1.0, tilt, 0.0],
            [0.0, 0.0, 0.0],
        ]))
        .expect("a non-degenerate frame");

        assert_eq!(position.up, [0.0, 1.0, 0.0]);
        assert!(position.forward[1].abs() < 1e-6, "{:?}", position.forward);
        assert!(
            (position.forward[0] - 1.0).abs() < 1e-6,
            "{:?}",
            position.forward
        );
        // Still right-handed after the fix-up.
        let expected = cross(position.left, position.up);
        for (want, got) in expected.iter().zip(position.forward) {
            assert!((want - got).abs() < 1e-6);
        }
    }

    #[test]
    fn a_vertical_forward_axis_has_no_slot_to_report() {
        assert!(
            start_position(&slot([
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ]))
            .is_none()
        );
    }

    #[test]
    fn a_short_start_position_payload_is_refused() {
        assert!(start_position(&[0u8; 63]).is_none());
    }
}
