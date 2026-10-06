//! The authored potentially-visible set: the `section` node's `0x3c9` payload.
//!
//! **The visibility set is shipped data, not something this project computes.**
//! Every track on the disc partitions itself into at most 64 `section` nodes,
//! and each one carries a 64-bit mask naming the sections visible from inside
//! it. The artists baked it; the original binary looks it up and nothing more.
//! See `docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md` for
//! why this crate reads that set rather than deriving its own.
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
//! Confidence **90** for the mask and **85** for the bounding box, both carried
//! over unchanged from `docs/formats/track.md`, which holds the evidence. This
//! module adds no new claim about the layout; it implements the one already
//! recorded there.
//!
//! # Three things the layout will punish a consumer for assuming
//!
//! 1. **The cap is 64 and it is real.** It falls out of the mask width and a
//!    64-entry gather buffer in the original. `01_Track` carries exactly 64
//!    `section` nodes - the cap predicted from the mask width, hit on the nose,
//!    which is the strongest single piece of evidence that the mask reading is
//!    right. See [`Error::IndexOutOfRange`].
//! 2. **Ids are not dense.** `09_Track`'s reversed variant has 44 `section`
//!    nodes and a maximum index of 45, so an id is what the node's own `index`
//!    field says and never its position in the array. A consumer that assumes
//!    `id < count` is wrong on a shipped track. This is why [`TrackPvs`] is
//!    indexed by id over a fixed 64 slots rather than by a packed vector.
//! 3. **An id can be authored more than once**, and the masks a mask names need
//!    not all exist. Both are ordinary authoring slop rather than parse errors,
//!    both were found by measurement rather than predicted, and both are
//!    quantified in `crates/vex/tests/pvs_ground_truth.rs`: one bit in
//!    fifty names a section its own file does not declare, and one id on four
//!    tracks is authored three times over with identical masks. Duplicates are
//!    unioned here; a mask bit for a section that does not exist is inert,
//!    because nothing ever asks about it.
//! 4. **The out-of-range answer is "everything".** The original's lookup at
//!    `0x0891e908` returns all ones for an index it does not have. That is not
//!    an error path to tighten up: it is the conservative fallback that keeps a
//!    craft which has left the authored partition - fallen off, been reset,
//!    airborne over a gap - from watching the world disappear. [`ALL_VISIBLE`]
//!    reproduces it.
//!
//! # Which geometry a section governs
//!
//! The payload says which sections are visible from which; **which geometry
//! belongs to a section is structural**: a `section` node governs its
//! parent's whole subtree, because tracks are authored as sibling groups -
//! one transform per group holding the `section` and the group's meshes.
//! [`governing_sections`] derives it, and
//! `docs/formats/track.md` holds the evidence, including the far-LOD case
//! that proves membership is not spatial. Turning that into per-draw-call
//! masks is `oag_render::pvs`'s business.

use crate::vex::{self, Node};
use oag_formats::ByteOrder;
use std::fmt;

/// The hard cap on sections, from the 64-bit mask width.
pub const MAX_SECTIONS: usize = 64;

/// Every section visible - the answer for an id the track does not declare.
///
/// Matches the original lookup at `0x0891e908`, which returns all ones out of
/// range. Culling with this mask draws everything, which is always correct and
/// merely slow.
pub const ALL_VISIBLE: u64 = u64::MAX;

/// Bytes of `section` payload before the optional bounding box.
const FIXED_LEN: usize = 0x10;

/// Bytes of bounding box, two `f32[4]`s.
const BOUNDS_LEN: usize = 0x20;

/// An axis-aligned box, as authored.
///
/// The payload stores four floats per corner. The fourth is not a coordinate -
/// it is padding to the vector alignment the original's VFPU loads want - and
/// is dropped here rather than preserved, because nothing reads it.
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
    /// An index at or above [`MAX_SECTIONS`], which no mask could address.
    ///
    /// Corroborated as impossible on shipped data: no control point on any of
    /// the 40 PSP track files carries a `section_id` above 63. Hitting this
    /// means the payload is being read at the wrong offset, not that a track
    /// has more sections than the original supports.
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
    /// Sections visible from here, with this section's own bit set.
    ///
    /// The own-bit OR happens at load in the original too, so a section always
    /// sees itself and the mask is never zero.
    pub visible: u64,
    /// The authored bounding box, when the node carries one.
    pub bounds: Option<Aabb>,
}

/// Every `section` node of one track, indexed by id.
///
/// The layout is deliberately flat and fixed-size: 64 masks is 512 bytes, one
/// or two cache lines of the hot path, and a lookup is a bounds check and an
/// array index with no indirection. See the ADR for why this is preferred to
/// the offsets-plus-ids pair a variable-size visible list would need.
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
    /// A set that declares nothing, and therefore hides nothing.
    ///
    /// Every lookup returns [`ALL_VISIBLE`], so a track with no `section` nodes
    /// draws exactly as it does today. This is the value the renderer falls
    /// back to rather than refusing to draw.
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
            // A repeated index. Both parts are unioned rather than
            // last-write-wins, because a union can only ever draw more, and
            // which node the original keeps has not been read. On shipped data
            // the choice is moot: the only duplicate anywhere is one id
            // authored three times over, in 2 of 40 PSP tracks and 2 of 59 PS2
            // ones, with byte-identical masks every time. See
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
    /// reproducing the original's out-of-range answer. **This is the whole
    /// safety story of the runtime path**: any confusion about where the camera
    /// is degrades to drawing everything rather than to drawing nothing.
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
    /// The renderer needs this because the camera and the craft are not
    /// reliably in the same section - a chase camera lags through a corner and
    /// swings wide on a crash - and because a set of neighbours is unioned in
    /// as padding. An empty iterator yields [`ALL_VISIBLE`] rather than zero:
    /// "no idea where we are" must mean "draw everything".
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
    /// Boxes are axis-aligned and adjacent sections along a winding track
    /// overlap freely, so more than one can contain a point. The lowest such id
    /// wins, deterministically. Returns `None` when the point is outside every
    /// box or when no section carries bounds - both of which the caller must
    /// treat as [`ALL_VISIBLE`] rather than as "nothing is visible".
    ///
    /// **This is not how the original finds its current section.** Nothing has
    /// been recovered about that. Prefer the spline's own
    /// `SplinePoint::section_id`, which is authored, and keep this for
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
/// parent's whole subtree.** On disc a track is authored as sibling groups -
/// one transform per group, whose first child is the `section` and whose
/// remaining children are the group's geometry:
///
/// ```text
/// Transform
/// +-- section  (id, PVS mask, box)
/// +-- Transform -- Mesh
/// +-- Transform -- Mesh
/// ...
/// ```
///
/// Every one of Moa Therma's 64 sections has exactly this shape, and it is
/// what makes a far-LOD copy of the track disappear while racing on the real
/// one: the copy's group carries a section only a handful of distant vantage
/// sections list in their masks, even though its *geometry* occupies the same
/// world-space boxes as the sections the craft is in. A spatial rule places it
/// with the craft; the authored rule hides it. See
/// `docs/formats/track.md` and
/// `crates/render/tests/pvs_placement_ground_truth.rs`.
///
/// A node above every section group - the world root, or a group transform
/// with no `section` child on the path up - answers `None`, which callers
/// must treat as "always visible", never as "never visible".
///
/// If several sections share one parent the lowest node index wins; shipped
/// data has no such group, so the tie-break is a determinism guard rather
/// than a behaviour anyone relies on.
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
/// The padding the renderer applies is "also draw what is next door", and
/// *next door* has to mean adjacency along the track, not `id + 1`. Ids are
/// neither dense nor guaranteed to run in track order, so numeric neighbours
/// are not spatial ones. The authored spline knows the truth: consecutive
/// control points carry `section_id`, so a change between two of them is an
/// edge, and junctions join the ends of paths.
///
/// Built from authored data throughout. The only free parameter is how many
/// hops to take.
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
    /// `hops` is how far to spread; `0` reproduces [`Self::none`]. Two is the
    /// value the renderer uses, which covers a craft that has crossed a
    /// boundary the camera has not yet reached and vice versa.
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

        // A junction joins the last point of each arriving path to the first
        // point of each leaving one. Without this a ring track built from
        // several paths would have a seam the padding does not cross.
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
    /// An id past the cap pads to nothing, because the caller will already be
    /// treating it as [`ALL_VISIBLE`] and there is nothing to add.
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
    // **One 64-bit read, not two 32-bit ones**, and the difference is silent.
    // On a little-endian file the two spellings are identical, which is why the
    // field sat here as a `lo`/`hi` pair for a year without anything noticing.
    // On a big-endian file they are not: swapping each word in place leaves the
    // halves the wrong way round, and `docs/formats/hd-status.md` measures what
    // that costs - 55.6 % of set bits naming a section the file does not
    // declare, against 22.2 % for the correct reading, with no error raised
    // either way. 15 of Wipeout HD's 24 circuits go from clean to 100 %
    // dangling.
    let mask = order.u64(data, base + 0x08);
    // The own bit is OR'd in at load in the original, so a section always sees
    // itself. Doing it here rather than at lookup keeps the hot path a plain
    // array read.
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
