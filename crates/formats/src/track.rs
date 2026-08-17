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

use crate::ByteOrder;
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
/// `order` is the containing `.vex`'s, from [`vex::byte_order`]. A `Start
/// Position` payload is a bare 4x4 matrix with no magic, so unlike the spline
/// itself it cannot say which way round it is.
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
/// **Five were converted on 2026-08-09**, and one of them was not a test:
/// `oag_trace`'s own `ai_of` looked the node up by the version-6 constant, so
/// `oag-trace track --source` against a Pure disc reported "has no WO Track
/// node" for a track that has one. That is fixed and checked on
/// `pure-psp-eu.chd`, which is the Pure pressing `oag_pulse`'s deny-list does
/// not refuse - `pure-psp-usa.chd` is rejected at open by serial, so the bug was
/// never reachable there. A sixth site, `pvs_ground_truth`'s section walk, went
/// to [`vex::classes_of`] instead, because it wants every matching node rather
/// than the first. Three spellings survive on purpose -
/// `pvs_ground_truth::pure_does_not_share_pulses_class_numbering` counts nodes
/// matching *Pulse's* id on Pure's disc, which is the whole claim it makes, and
/// `track_ground_truth` restates `0x3bb` from `docs/formats/track.md` at two
/// sites so the test checks the documented id rather than the crate's.
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

/// Which way round a payload's words are, from its own magic, or `None` when
/// the magic is neither.
///
/// The four bytes spell `dtOW` on the PSP and PS2 and `WOtd` on the PS3 - the
/// same [`MAGIC`] word, written on hosts of opposite endianness. So a `WO Track`
/// payload says which it is even though it sits inside a `.vex` that already
/// said, and this parser needs no argument and no caller change to read a
/// Wipeout HD circuit.
///
/// That is worth stating because it was nearly got wrong:
/// `docs/formats/hd-status.md` records the magic as "`WOtd`" on both, which
/// reads as "the magic is not a discriminator". Measured on the shipped files,
/// `16_Track` opens `64 74 4f 57` and `talons_junction` opens `57 4f 74 64`.
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

    // Read the counts and check every point block fits before allocating
    // anything: the counts are u32 from an untrusted file, and reserving on
    // trust is how a 2 GiB allocation happens.
    let mut counts = Vec::with_capacity(path_count.min(payload.len() / PATH_LEN));
    let mut at = points_at;
    for i in 0..path_count {
        let count = u32_at(order, payload, paths_at + i * PATH_LEN) as usize;
        at = end_of("control points", at, count, POINT_LEN, payload.len())?;
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
            points.push(point_at(order, payload, points_from + k * POINT_LEN));
        }
        points_from += count * POINT_LEN;

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

fn point_at(order: ByteOrder, payload: &[u8], at: usize) -> SplinePoint {
    SplinePoint {
        pos: vec3_at(order, payload, at),
        tangent: vec3_at(order, payload, at + 0x10),
        down: vec3_at(order, payload, at + 0x20),
        lateral: vec3_at(order, payload, at + 0x30),
        half_width_left: f32_at(order, payload, at + 0x44),
        half_width_right: f32_at(order, payload, at + 0x48),
        ai_bound_left: f32_at(order, payload, at + 0x4c),
        ai_bound_right: f32_at(order, payload, at + 0x50),
        racing_line: f32_at(order, payload, at + 0x54),
        section_id: payload[at + 0x60],
        flags: payload[at + 0x61],
    }
}

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
