//! `WO Track`: the AI track spline graph a lap is made of.
//!
//! One node in a track's [`.vex`](crate::vex) file carries the whole driveable
//! path as a **graph of uniform cubic B-splines**: each control point has a
//! position, an orientation frame, the half-width either side, an explicit
//! racing line and an AI corridor.
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
//!         then    each path's control points in path order,
//!                 0x70 bytes each, or 0x60 from version 0x107
//! ```
//!
//! ```text
//! control point:
//!   +0x00  [f32;3]  pos       (16-byte slot)
//!   +0x10  [f32;3]  tangent
//!   +0x20  [f32;3]  down
//!   +0x30  [f32;3]  lateral
//!   +0x44  f32      half_width_left
//!   +0x48  f32      half_width_right
//!   +0x4c  f32      ai_bound_left
//!   +0x50  f32      ai_bound_right
//!   +0x54  f32      racing_line
//!          then     section_id and flags, one byte each, at +0x60/+0x61
//!                   before version 0x107 and at +0x5c/+0x5d from it
//! ```
//!
//! # Wipeout 2048 shortened the control point
//!
//! Version `0x107` (2048's, one above HD's `0x106`) drops the record from 112
//! bytes to 96. Everything through `racing_line` stays put; `section_id` and
//! `flags` move from `+0x60`/`+0x61` into the eight bytes `+0x58..+0x60` HD
//! leaves zero, at `+0x5c`/`+0x5d`. `+0x5a`/`+0x5b` are `0xff` and `0x00` on
//! every 2048 point measured, `+0x58`/`+0x59` vary; neither is placed.
//!
//! Measured: 2048's DLC re-ships twelve HD circuits with byte-identical path and
//! control-point counts, so HD's decoded record is ground truth for 2048's. See
//! `docs/formats/track.md`.
//!
//! # The reserved block is the whole trick
//!
//! Both array pointers are **zero on disc**: runtime pointers, patched as the
//! loader walks a cursor, so the arrays are positioned by a rule. For version
//! `0x101` and up the loader claims **a second 0x20-byte block** before the
//! paths, pushing everything after it along by 32 bytes.
//!
//! Reading the paths at `+0x20` instead of `+0x40` lands one field early: world
//! coordinates in the orientation frame, a `section_id` of 185 on a 64-section
//! track. It looks like a wrong struct, not a wrong offset.
//!
//! # Why the details matter
//!
//! **Positions on disc sit on the track surface.** The load pass does
//! `pos -= 3 * down`, lifting each point by [`HOVER_LIFT`]; ships fly the lifted
//! line. [`SplinePoint::pos`] is the disc value, [`SplinePoint::lifted_pos`]
//! applies the lift.
//!
//! **The frame's second vector points down, not up**: `(0, -1, 0)` on level
//! ground, `+y` being world up. Lifting along it as "up" moves the racing line
//! *into* the track.
//!
//! **Junction slots are 2 in, 2 out.** Slots 0 and 1 are predecessors, 2 and 3
//! successors, `0x7fffffff` null. A shortcut is an alternate successor.
//!
//! See `docs/formats/track.md` for the evidence and the confidence scores.

use std::fmt;

use crate::vex;
use oag_formats::ByteOrder;

/// Bytes of `WO Track` header.
pub const HEADER_LEN: usize = 0x20;

/// The reserved block between the header and the paths, from version `0x101`.
pub const RESERVED_LEN: usize = 0x20;

/// Bytes per path.
pub const PATH_LEN: usize = 0x20;

/// Bytes per junction.
pub const JUNCTION_LEN: usize = 0x10;

/// Bytes per spline control point, before version [`SHORT_POINT_VERSION`].
pub const POINT_LEN: usize = 0x70;

/// Bytes per spline control point from version [`SHORT_POINT_VERSION`].
pub const POINT_LEN_SHORT: usize = 0x60;

/// First version whose control point is [`POINT_LEN_SHORT`] bytes: Wipeout
/// 2048's.
pub const SHORT_POINT_VERSION: u32 = 0x107;

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
/// The four vectors are an orthonormal frame; `down` is the surface normal
/// negated (see the module docs).
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
    /// The authored normalised arc position round the circuit, `0.0..1.0`, at
    /// `+0x40`: the lap counter's parameter and what a Quake road span's `t`
    /// window is measured in (`crate::quake`). See `docs/formats/track.md`.
    pub progress: f32,
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
    /// Which [`section`](crate::vex) this point belongs to, for visibility;
    /// `0xff` on a track with no `section` node (several 2048 circuits).
    pub section_id: u8,
    /// Flags, OR-accumulated across the four control points of a segment.
    pub flags: u8,
    /// The hull light scales a craft over this point is drawn with: ambient,
    /// directional, and the two point-light classes, `0..=255`, at `+0x62`.
    ///
    /// Blended into `craft+0xb52..+0xb55` (`FUN_0887c7e8`, `FUN_0887c11c`),
    /// copied to the hull model's `+0x44..+0x47` each frame (`FUN_0883e444`) and
    /// multiplied into its GE light colours by `SceneLight_BuildLightingList`.
    /// `0xff` on every point of a track older than `0x103`, as the original
    /// forces, and on the short points of [`SHORT_POINT_VERSION`] onwards, whose
    /// layout moves these bytes unread. See `docs/formats/track.md`.
    pub light_scale: [u8; 4],
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
    /// [`SplinePoint::light_scale`] blended the original's way - see
    /// [`blend_light_scale`].
    pub light_scale: [u8; 4],
}

/// One spline path: a run of control points between two junctions.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Control points, in order.
    pub points: Vec<SplinePoint>,
    /// The longest gap between consecutive control points in this path, exact on
    /// all 86 paths of the 40 PSP track files (hence named, not an unknown word).
    pub max_spacing: f32,
    /// Junction this path leaves from, if any.
    pub entry: Option<usize>,
    /// Junction this path arrives at, if any.
    pub exit: Option<usize>,
}

/// A 2-in, 2-out merge and split between paths; a plain ring is one predecessor,
/// one successor, both alternates null.
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
    /// Bytes the decoded structure accounts for: equal to the payload length on
    /// every shipped track, the self-check that settled the layout.
    #[must_use]
    pub fn encoded_len(&self) -> usize {
        let points: usize = self.paths.iter().map(|p| p.points.len()).sum();
        HEADER_LEN
            + reserved_len(self.version)
            + self.paths.len() * PATH_LEN
            + self.junctions.len() * JUNCTION_LEN
            + points * point_len(self.version)
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
    /// segment + 2`, clamped at the path ends, blending scalars as well as
    /// vectors. It does not pass through its control points, so `sample(i, 0.0)`
    /// is not `points[i]`: the original's behaviour.
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
            light_scale: blend_light_scale(&w, &p),
        })
    }
}

/// [`SplinePoint::light_scale`] blended over a segment's four control points in
/// the original's integer arithmetic.
///
/// `FUN_0887c11c` adds `(byte * trunc(weight * 255)) >> 8` per point into a
/// `u8`: the sum truncates twice and wraps rather than saturating, so a run of
/// `0xff` points reads back a little under `0xff`. Weights are [`basis`]'s.
#[must_use]
pub fn blend_light_scale(weights: &[f32; 4], points: &[&SplinePoint; 4]) -> [u8; 4] {
    let mut out = [0u8; 4];
    for (weight, point) in weights.iter().zip(points) {
        let scaled = (weight * 255.0) as u32;
        for (acc, byte) in out.iter_mut().zip(point.light_scale) {
            *acc = acc.wrapping_add(((u32::from(byte) * scaled) >> 8) as u8);
        }
    }
    out
}

/// The uniform cubic B-spline basis, weights for four control points.
///
/// `[(1-t)^3, 3t^3 - 6t^2 + 4, -3t^3 + 3t^2 + 3t + 1, t^3] / 6`, the constants
/// of the original's VFPU evaluator.
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
/// The payload is an authored 4x4 transform whose rows are the **left-up-forward**
/// basis the craft carries; the bind handler (`0x08926ae8`) forces the up row to
/// world `(0, 1, 0)` and re-orthonormalises. [`start_position`] reproduces that,
/// so this is the frame the *game* starts from.
///
/// **One per track, not on the centreline**: all 40 PSP track files carry exactly
/// one, 3.3 to 20.5 units off the spline's centreline, so it is a grid *slot*,
/// and the other seven slots come from unrecovered code. See
/// `docs/formats/track.md`.
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
/// `None` when the payload is not 64 bytes, or the authored forward axis is
/// vertical (degenerate; no shipped track has one).
///
/// # What is reproduced, and what was assumed
///
/// The forced up row is read (`docs/formats/track.md`, from the bind handler).
/// **Which of the other two rows the original preserves was not read**; this
/// keeps *forward*, since where a ship points is the deliberately authored part
/// of a slot. The data bounds the choice: the authored up row is already world
/// up on 31 of 40 track files and off it by at most **1.600 degrees** on the
/// other nine, so any right-handed re-orthonormalisation lands within that
/// angle of another. The fix-up is still not a no-op, hence applied.
///
/// `order` is the containing `.vex`'s ([`vex::byte_order`]): the payload is a
/// bare 4x4 matrix with no magic.
#[must_use]
pub fn start_position(payload: &[u8], order: ByteOrder) -> Option<StartPosition> {
    if payload.len() < START_POSITION_LEN {
        return None;
    }
    let row = |r: usize| {
        [
            f32_at(order, payload, r * 16),
            f32_at(order, payload, r * 16 + 4),
            f32_at(order, payload, r * 16 + 8),
        ]
    };

    let up = [0.0, 1.0, 0.0];
    let authored = row(2);
    // Gram-Schmidt against exactly `+y`: the projection is the y component.
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
/// Spelling `class_id == vex::CLASS_WO_TRACK` is the version-6 id and finds
/// nothing in a version-4 file; the payload parser itself accepts Pure's `0x103`
/// ([`MIN_VERSION`] is `0x100`). Three test spellings survive on purpose:
/// `pvs_ground_truth::pure_does_not_share_pulses_class_numbering` counts nodes
/// matching *Pulse's* id on Pure's disc, and `track_ground_truth` restates
/// `0x3bb` from `docs/formats/track.md` at two sites.
///
/// `None` for a version with no class table, a table with no `WO Track` id
/// recovered, or a `.vex` that authors none (indistinguishable here; ask
/// [`vex::classes_of`] to tell them apart).
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

/// Bytes per control point for a given version.
#[must_use]
pub fn point_len(version: u32) -> usize {
    if version >= SHORT_POINT_VERSION {
        POINT_LEN_SHORT
    } else {
        POINT_LEN
    }
}

/// Which way round a payload's words are, from its own magic, or `None` when
/// the magic is neither.
///
/// The four bytes spell `dtOW` on the PSP and PS2 and `WOtd` on the PS3: the same
/// [`MAGIC`] on hosts of opposite endianness, so this parser reads a Wipeout HD
/// circuit with no argument. `docs/formats/hd-status.md` once recorded `WOtd` on
/// both; measured, `16_Track` opens `64 74 4f 57` and `talons_junction`
/// `57 4f 74 64`.
#[must_use]
pub fn byte_order(payload: &[u8]) -> Option<ByteOrder> {
    if payload.len() < 4 {
        return None;
    }
    [ByteOrder::Little, ByteOrder::Big]
        .into_iter()
        .find(|&order| order.u32(payload, 0) == MAGIC)
}

/// Whether a payload carries the `WO Track` magic, in either byte order.
#[must_use]
pub fn has_magic(payload: &[u8]) -> bool {
    byte_order(payload).is_some()
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

    let Some(order) = byte_order(payload) else {
        return Err(Error::BadMagic {
            magic: ByteOrder::Little.u32(payload, 0),
        });
    };
    let version = u32_at(order, payload, 4);
    if version < MIN_VERSION {
        return Err(Error::UnsupportedVersion { version });
    }

    let path_count = u32_at(order, payload, 8) as usize;
    let junction_count = u32_at(order, payload, 12) as usize;

    let paths_at = HEADER_LEN + reserved_len(version);
    let junctions_at = end_of("paths", paths_at, path_count, PATH_LEN, payload.len())?;
    let points_at = end_of(
        "junctions",
        junctions_at,
        junction_count,
        JUNCTION_LEN,
        payload.len(),
    )?;

    // Check every point block fits before allocating: counts are untrusted u32s
    // (reserving on trust is how a 2 GiB allocation happens).
    let mut counts = Vec::with_capacity(path_count.min(payload.len() / PATH_LEN));
    let mut at = points_at;
    for i in 0..path_count {
        let count = u32_at(order, payload, paths_at + i * PATH_LEN) as usize;
        at = end_of(
            "control points",
            at,
            count,
            point_len(version),
            payload.len(),
        )?;
        counts.push(count);
    }

    let mut junctions = Vec::with_capacity(junction_count);
    for j in 0..junction_count {
        let base = junctions_at + j * JUNCTION_LEN;
        let slot = |k: usize| index_at(order, payload, base + k * 4, "junction slot", path_count);
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
            points.push(point_at(
                order,
                payload,
                points_from + k * point_len(version),
                version,
            ));
        }
        points_from += count * point_len(version);

        paths.push(Path {
            points,
            max_spacing: f32_at(order, payload, base + 4),
            entry: index_at(order, payload, base + 0x0c, "path entry", junction_count)?,
            exit: index_at(order, payload, base + 0x10, "path exit", junction_count)?,
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
fn index_at(
    order: ByteOrder,
    payload: &[u8],
    at: usize,
    what: &'static str,
    count: usize,
) -> Result<Option<usize>> {
    let raw = u32_at(order, payload, at);
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

fn point_at(order: ByteOrder, payload: &[u8], at: usize, version: u32) -> SplinePoint {
    let section_at = if version >= SHORT_POINT_VERSION {
        0x5c
    } else {
        0x60
    };
    SplinePoint {
        pos: vec3_at(order, payload, at),
        tangent: vec3_at(order, payload, at + 0x10),
        down: vec3_at(order, payload, at + 0x20),
        lateral: vec3_at(order, payload, at + 0x30),
        progress: f32_at(order, payload, at + 0x40),
        half_width_left: f32_at(order, payload, at + 0x44),
        half_width_right: f32_at(order, payload, at + 0x48),
        ai_bound_left: f32_at(order, payload, at + 0x4c),
        ai_bound_right: f32_at(order, payload, at + 0x50),
        racing_line: f32_at(order, payload, at + 0x54),
        section_id: payload[at + section_at],
        flags: payload[at + section_at + 1],
        light_scale: if (LIGHT_SCALE_VERSION..SHORT_POINT_VERSION).contains(&version) {
            [
                payload[at + 0x62],
                payload[at + 0x63],
                payload[at + 0x64],
                payload[at + 0x65],
            ]
        } else {
            [0xff; 4]
        },
    }
}

/// The first spline version whose authored [`SplinePoint::light_scale`] the
/// original reads; older points are forced to `0xff` (`FUN_0887c7e8`'s tail,
/// a compare against `0x103`).
pub const LIGHT_SCALE_VERSION: u32 = 0x103;

fn vec3_at(order: ByteOrder, payload: &[u8], at: usize) -> [f32; 3] {
    [
        f32_at(order, payload, at),
        f32_at(order, payload, at + 4),
        f32_at(order, payload, at + 8),
    ]
}

fn u32_at(order: ByteOrder, data: &[u8], at: usize) -> u32 {
    order.u32(data, at)
}

fn f32_at(order: ByteOrder, data: &[u8], at: usize) -> f32 {
    order.f32(data, at)
}

#[cfg(test)]
mod tests;
