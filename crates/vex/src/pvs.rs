//! The authored potentially-visible set: the `section` node's `0x3c9` payload.
//!
//! **The visibility set is shipped data, not computed here.** Every track
//! partitions itself into at most 64 `section` nodes, each carrying a 64-bit
//! mask naming the sections visible from inside it; the original looks it up
//! and nothing more. See
//! `docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md`.
//!
//! ```text
//! payload:
//!   +0x00  u8    index          array index, also the PVS bit position
//!   +0x01  u8    has_bounds     selects the optional bbox block
//!   +0x02  u8[6] pad            never read
//!   +0x08  u64   pvs_mask       one bit per section visible from here
//!         then, only when has_bounds:
//!   +0x10  f32[4] bbox_min      the fourth component is not a coordinate
//!   +0x20  f32[4] bbox_max
//! ```
//!
//! Confidence **90** for the mask and **85** for the bounding box, carried over
//! from `docs/formats/track.md`, which holds the evidence; this module adds no
//! new claim about the layout.
//!
//! # What the layout will punish a consumer for assuming
//!
//! 1. **The cap is 64 and it is real**, from the mask width and a 64-entry
//!    gather buffer in the original. `01_Track` carries exactly 64 `section`
//!    nodes, the cap predicted from the mask width hit on the nose, the
//!    strongest evidence the mask reading is right. See
//!    [`Error::IndexOutOfRange`].
//! 2. **Ids are not dense.** `09_Track`'s reversed variant has 44 `section`
//!    nodes and a maximum index of 45, so an id is the node's own `index`, never
//!    its array position, and [`TrackPvs`] is indexed by id over a fixed 64
//!    slots.
//! 3. **An id can be authored more than once**, and a mask can name sections that
//!    do not exist. Both are authoring slop, found by measurement and quantified
//!    in `crates/vex/tests/pvs_ground_truth.rs` (one bit in fifty names a
//!    section its file does not declare; one id on four tracks is authored three
//!    times with identical masks). Duplicates are unioned; a bit for a missing
//!    section is inert.
//! 4. **The out-of-range answer is "everything".** The original's lookup at
//!    `0x0891e908` returns all ones for an index it does not have: the
//!    conservative fallback that keeps a craft that left the authored partition
//!    (fallen off, reset, over a gap) from watching the world disappear.
//!    [`ALL_VISIBLE`] reproduces it.
//!
//! # Which geometry a section governs
//!
//! The payload says which sections are visible from which; **which geometry
//! belongs to a section is structural**: a `section` node governs its parent's
//! whole subtree, tracks being authored as sibling groups (one transform per
//! group holding the `section` and the group's meshes). [`governing_sections`]
//! derives it; `docs/formats/track.md` holds the evidence, including the far-LOD
//! case proving membership is not spatial. Per-draw-call masks are
//! `oag_render::pvs`'s business.

use crate::vex::{self, Node};
use oag_formats::ByteOrder;
use std::fmt;

/// The hard cap on sections, from the 64-bit mask width.
pub const MAX_SECTIONS: usize = 64;

/// Every section visible: the answer for an id the track does not declare,
/// matching the original lookup at `0x0891e908` (all ones out of range). Culling
/// with it draws everything, always correct and merely slow.
pub const ALL_VISIBLE: u64 = u64::MAX;

/// Bytes of `section` payload before the optional bounding box.
const FIXED_LEN: usize = 0x10;

/// Bytes of bounding box, two `f32[4]`s.
const BOUNDS_LEN: usize = 0x20;

/// An axis-aligned box, as authored.
///
/// The payload stores four floats per corner; the fourth is VFPU-alignment
/// padding, dropped here since nothing reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    /// Whether `point` lies inside, inclusive on both faces.
    #[must_use]
    pub fn contains(&self, point: [f32; 3]) -> bool {
        (0..3).all(|i| point[i] >= self.min[i] && point[i] <= self.max[i])
    }

    /// The smallest box containing both.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let mut out = self;
        for i in 0..3 {
            out.min[i] = self.min[i].min(other.min[i]);
            out.max[i] = self.max[i].max(other.max[i]);
        }
        out
    }

    /// The centre of the box.
    #[must_use]
    pub fn centre(&self) -> [f32; 3] {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }
}

/// What can go wrong reading a `section` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The payload is shorter than the fields it must carry.
    TooShort {
        node: usize,
        got: usize,
        need: usize,
    },
    /// An index at or above [`MAX_SECTIONS`], which no mask could address. No
    /// control point on any of the 40 PSP track files carries a `section_id` above
    /// 63, so hitting this means a wrong read offset, not a track with more
    /// sections than the original supports.
    IndexOutOfRange { node: usize, index: u8 },
    /// The scene tree itself would not parse.
    Vex(vex::Error),
}

impl From<vex::Error> for Error {
    fn from(error: vex::Error) -> Self {
        Self::Vex(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { node, got, need } => {
                write!(
                    f,
                    "section node {node}: payload is {got} bytes, needs {need}"
                )
            }
            Self::IndexOutOfRange { node, index } => write!(
                f,
                "section node {node}: index {index} is at or above the {MAX_SECTIONS}-section cap"
            ),
            Self::Vex(error) => write!(f, "reading the scene tree: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Vex(error) => Some(error),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// One decoded `section` node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Section {
    /// The node's own index, which is also its bit position in every mask.
    pub index: u8,
    /// Sections visible from here, with this section's own bit set (the original
    /// ORs it in at load too, so the mask is never zero).
    pub visible: u64,
    /// The authored bounding box, when the node carries one.
    pub bounds: Option<Aabb>,
}

/// Every `section` node of one track, indexed by id.
///
/// The layout is flat and fixed-size: 64 masks is 512 bytes, one or two cache
/// lines of the hot path, a lookup a bounds check and an array index. See the
/// ADR for why this is preferred to the offsets-plus-ids pair a variable-size
/// visible list would need.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackPvs {
    /// One mask per id. [`ALL_VISIBLE`] where no node declares that id, so an
    /// undeclared id behaves like an out-of-range one.
    masks: [u64; MAX_SECTIONS],
    /// Authored bounds per id, absent where the node had `has_bounds == 0`.
    bounds: [Option<Aabb>; MAX_SECTIONS],
    /// Which ids a node actually declared. Sparse, by design - see the module
    /// docs.
    declared: u64,
}

impl Default for TrackPvs {
    fn default() -> Self {
        Self::empty()
    }
}

impl TrackPvs {
    /// A set that declares nothing and hides nothing: every lookup returns
    /// [`ALL_VISIBLE`], so a track with no `section` nodes draws as it does today.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            masks: [ALL_VISIBLE; MAX_SECTIONS],
            bounds: [None; MAX_SECTIONS],
            declared: 0,
        }
    }

    /// Reads every `section` node in an already-walked scene tree.
    ///
    /// `data` is the whole `.vex` file and `nodes` the result of
    /// [`vex::nodes`].
    pub fn from_nodes(data: &[u8], nodes: &[Node]) -> Result<Self> {
        let mut pvs = Self::empty();
        let section_class = vex::classes_of(data).ok().and_then(|c| c.section);
        let order = vex::byte_order(data);
        for (at, node) in nodes.iter().enumerate() {
            if Some(node.class_id) != section_class {
                continue;
            }
            let section = parse_section(data, node, at, order)?;
            let id = usize::from(section.index);
            if pvs.declared & (1u64 << id) == 0 {
                pvs.declared |= 1u64 << id;
                pvs.masks[id] = section.visible;
                pvs.bounds[id] = section.bounds;
                continue;
            }
            // A repeated index: union rather than last-write-wins, since a union
            // only draws more and which node the original keeps is unread. The
            // only duplicate on shipped data is one id authored three times with
            // byte-identical masks (2 of 40 PSP tracks, 2 of 59 PS2); see
            // `crates/vex/tests/pvs_ground_truth.rs`.
            pvs.masks[id] |= section.visible;
            pvs.bounds[id] = match (pvs.bounds[id], section.bounds) {
                (Some(a), Some(b)) => Some(a.union(b)),
                (existing, None) => existing,
                (None, new) => new,
            };
        }
        Ok(pvs)
    }

    /// Reads every `section` node of a `.vex` file.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let nodes = vex::nodes(data)?;
        Self::from_nodes(data, &nodes)
    }

    /// Sections visible from `id`.
    ///
    /// Returns [`ALL_VISIBLE`] for an id past the cap or one no node declared,
    /// reproducing the original. **This is the whole safety story of the runtime
    /// path**: confusion about where the camera is degrades to drawing everything,
    /// not nothing.
    #[must_use]
    pub fn visible_from(&self, id: u8) -> u64 {
        let id = usize::from(id);
        if id >= MAX_SECTIONS {
            return ALL_VISIBLE;
        }
        self.masks[id]
    }

    /// The union of [`Self::visible_from`] over several ids.
    ///
    /// The renderer needs this because the camera and craft are not reliably in
    /// the same section (a chase camera lags through a corner and swings wide on a
    /// crash) and a set of neighbours is unioned in as padding. An empty iterator
    /// yields [`ALL_VISIBLE`]: "no idea where we are" means "draw everything".
    #[must_use]
    pub fn visible_from_any(&self, ids: impl IntoIterator<Item = u8>) -> u64 {
        let mut ids = ids.into_iter().peekable();
        if ids.peek().is_none() {
            return ALL_VISIBLE;
        }
        ids.fold(0, |mask, id| mask | self.visible_from(id))
    }

    /// Whether a node declared `id`.
    #[must_use]
    pub fn declares(&self, id: u8) -> bool {
        usize::from(id) < MAX_SECTIONS && self.declared & (1u64 << id) != 0
    }

    /// How many sections the track declares.
    #[must_use]
    pub fn len(&self) -> usize {
        self.declared.count_ones() as usize
    }

    /// Whether the track declares no sections at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.declared == 0
    }

    /// The declared ids, ascending. Sparse - see the module docs.
    pub fn ids(&self) -> impl Iterator<Item = u8> + '_ {
        (0..MAX_SECTIONS as u8).filter(|&id| self.declares(id))
    }

    /// The authored bounds of `id`, when it has any.
    #[must_use]
    pub fn bounds_of(&self, id: u8) -> Option<Aabb> {
        let id = usize::from(id);
        if id >= MAX_SECTIONS {
            return None;
        }
        self.bounds[id]
    }

    /// The declared section whose authored box contains `point`.
    ///
    /// Boxes are axis-aligned and adjacent sections overlap freely, so the lowest
    /// containing id wins, deterministically. `None` when the point is outside
    /// every box or no section carries bounds; the caller must treat that as
    /// [`ALL_VISIBLE`], not "nothing is visible".
    ///
    /// **This is not how the original finds its current section** (unrecovered).
    /// Prefer the spline's authored `SplinePoint::section_id`; keep this for
    /// positions with no meaningful spline parameter.
    #[must_use]
    pub fn section_at(&self, point: [f32; 3]) -> Option<u8> {
        self.ids()
            .find(|&id| self.bounds_of(id).is_some_and(|b| b.contains(point)))
    }
}

/// The authored section governing each node of a scene tree, or `None` for a
/// node no section governs.
///
/// The association is structural, not spatial: **a `section` node governs its
/// parent's whole subtree.** A track is authored as sibling groups, one
/// transform per group, whose first child is the `section` and whose other
/// children are the group's geometry:
///
/// ```text
/// Transform
/// +-- section  (id, PVS mask, box)
/// +-- Transform -- Mesh
/// +-- Transform -- Mesh
/// ...
/// ```
///
/// All 64 of Moa Therma's sections have this shape, and it is what makes a
/// far-LOD copy of the track disappear while racing on the real one: the copy's
/// group carries a section only a few distant sections list in their masks,
/// though its *geometry* occupies the same world-space boxes as the craft's. A
/// spatial rule places it with the craft; the authored rule hides it. See
/// `docs/formats/track.md` and
/// `crates/render/tests/pvs_placement_ground_truth.rs`.
///
/// A node above every section group answers `None`, which callers must treat as
/// "always visible". If several sections share one parent the lowest node index
/// wins (no shipped group does; a determinism guard).
pub fn governing_sections(data: &[u8], nodes: &[Node]) -> Result<Vec<Option<u8>>> {
    // parent node index -> the id of its section child.
    let mut section_of_parent: Vec<Option<u8>> = vec![None; nodes.len()];
    let section_class = vex::classes_of(data).ok().and_then(|c| c.section);
    let order = vex::byte_order(data);
    for (at, node) in nodes.iter().enumerate() {
        if Some(node.class_id) != section_class {
            continue;
        }
        let section = parse_section(data, node, at, order)?;
        if let Some(parent) = node.parent
            && section_of_parent[parent].is_none()
        {
            section_of_parent[parent] = Some(section.index);
        }
    }

    Ok((0..nodes.len())
        .map(|index| {
            let mut current = Some(index);
            while let Some(i) = current {
                if let Some(id) = section_of_parent[i] {
                    return Some(id);
                }
                current = nodes[i].parent;
            }
            None
        })
        .collect())
}

/// Sections reachable within a few control points along the spline.
///
/// "Next door" must mean adjacency along the track, not `id + 1`: ids are
/// neither dense nor in track order. Consecutive control points carry
/// `section_id`, so a change between two is an edge, and junctions join path
/// ends. Built from authored data; the only free parameter is the hop count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionAdjacency {
    within: [u64; MAX_SECTIONS],
}

impl Default for SectionAdjacency {
    fn default() -> Self {
        Self::none()
    }
}

impl SectionAdjacency {
    /// No neighbours: every section pads to itself alone.
    #[must_use]
    pub fn none() -> Self {
        let mut within = [0u64; MAX_SECTIONS];
        for (id, mask) in within.iter_mut().enumerate() {
            *mask = 1u64 << id;
        }
        Self { within }
    }

    /// Builds the table from a track's spline graph.
    ///
    /// `hops` is how far to spread; `0` reproduces [`Self::none`]. The renderer
    /// uses two, covering a craft that crossed a boundary the camera has not yet
    /// reached and vice versa.
    #[must_use]
    pub fn from_track(track: &crate::track::AiTrack, hops: u32) -> Self {
        let mut direct = [0u64; MAX_SECTIONS];
        let mut link = |a: u8, b: u8| {
            let (a, b) = (usize::from(a), usize::from(b));
            if a < MAX_SECTIONS && b < MAX_SECTIONS {
                direct[a] |= 1u64 << b;
                direct[b] |= 1u64 << a;
            }
        };

        for path in &track.paths {
            for pair in path.points.windows(2) {
                link(pair[0].section_id, pair[1].section_id);
            }
        }

        // A junction joins the last point of each arriving path to the first of
        // each leaving one; without it a ring built from several paths has a seam
        // the padding does not cross.
        for junction in &track.junctions {
            for arriving in junction.prev.iter().flatten() {
                let Some(end) = track.paths.get(*arriving).and_then(|p| p.points.last()) else {
                    continue;
                };
                for leaving in junction.next.iter().flatten() {
                    let Some(start) = track.paths.get(*leaving).and_then(|p| p.points.first())
                    else {
                        continue;
                    };
                    link(end.section_id, start.section_id);
                }
            }
        }

        let mut within = Self::none().within;
        for _ in 0..hops {
            let previous = within;
            for (id, mask) in within.iter_mut().enumerate() {
                let mut spread = *mask;
                for neighbour in set_bits(previous[id]) {
                    spread |= direct[usize::from(neighbour)];
                }
                *mask = spread;
            }
        }

        Self { within }
    }

    /// Sections within the configured number of hops of `id`, including `id`.
    ///
    /// An id past the cap pads to nothing (the caller already treats it as
    /// [`ALL_VISIBLE`]).
    #[must_use]
    pub fn near(&self, id: u8) -> u64 {
        let id = usize::from(id);
        if id >= MAX_SECTIONS {
            return 0;
        }
        self.within[id]
    }
}

/// The set bits of a mask, ascending, as section ids.
pub fn set_bits(mask: u64) -> impl Iterator<Item = u8> {
    (0..MAX_SECTIONS as u8).filter(move |id| mask & (1u64 << id) != 0)
}

fn parse_section(data: &[u8], node: &Node, at: usize, order: ByteOrder) -> Result<Section> {
    let payload = node.payload();
    let got = payload.len();
    if got < FIXED_LEN || payload.end > data.len() {
        return Err(Error::TooShort {
            node: at,
            got,
            need: FIXED_LEN,
        });
    }
    let base = payload.start;
    let index = data[base];
    if usize::from(index) >= MAX_SECTIONS {
        return Err(Error::IndexOutOfRange { node: at, index });
    }
    let has_bounds = data[base + 1] != 0;
    // **One 64-bit read, not two 32-bit ones**, silently different: identical on
    // a little-endian file (so a `lo`/`hi` pair sat here for a year), but on a
    // big-endian one swapping each word in place leaves the halves the wrong way
    // round. `docs/formats/hd-status.md` measures it: 55.6 % of set bits naming
    // an undeclared section against 22.2 % for the correct reading, with no
    // error either way; 15 of HD's 24 circuits go from clean to 100 % dangling.
    let mask = order.u64(data, base + 0x08);
    // The own bit is OR'd in at load, as the original does, keeping the hot path
    // a plain array read.
    let visible = mask | (1u64 << index);

    let bounds = if has_bounds {
        if got < FIXED_LEN + BOUNDS_LEN {
            return Err(Error::TooShort {
                node: at,
                got,
                need: FIXED_LEN + BOUNDS_LEN,
            });
        }
        Some(Aabb {
            min: [
                order.f32(data, base + 0x10),
                order.f32(data, base + 0x14),
                order.f32(data, base + 0x18),
            ],
            max: [
                order.f32(data, base + 0x20),
                order.f32(data, base + 0x24),
                order.f32(data, base + 0x28),
            ],
        })
    } else {
        None
    };

    Ok(Section {
        index,
        visible,
        bounds,
    })
}

#[cfg(test)]
mod tests;
