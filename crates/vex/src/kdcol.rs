//! `track_col.col`: Wipeout 2048's collision, in a container of its own.
//!
//! Every earlier title keeps its collision inside the track's
//! [`.vex`](crate::vex) file as nodes of five named classes
//! ([`collision`](crate::collision) reads those). Wipeout 2048 authors **no
//! collision node instances** and ships a separate `track_col.col` beside each
//! `track.vex`: a k-d tree over one triangle soup, with a surface byte per
//! triangle.
//!
//! ```text
//! +0x00  char[4]  "kdtr"
//! +0x04  char[4]  ASCII version, four hex digits, must read as 1
//!        "----"   section tag, repeated before every section below
//!        u32      node stride in bytes - 24 on every 2048 file, 19 on every
//!                 Omega Collection file; see `NodeLayout`
//!        u32      node count N
//!        node[N]  the k-d tree, `stride` bytes each - see `Node`
//!        "----"
//!        u32      leaf index count M
//!        u16[M]   triangle indices, the leaves' own contents
//!        "----"
//!        f32[6]   the tree's bounds: **centre, then half-extent**
//!        "----"
//!        …        the triangle soup - see `Mesh`
//!        "----"   one final tag, and the file ends
//! ```
//!
//! ```text
//! the triangle soup (`Backend/General/Collision/SimpleMesh.cpp`):
//!        u16      vertex count V
//!        f32[3V]  positions, world space
//!        u32      bytes per triangle, 6 on every file seen and refused otherwise
//!        u16      triangle count T
//!        u16[3T]  triangle indices
//!        u16      surface byte count, T on every file seen
//!        u8[T]    one surface byte per triangle - see `SurfaceByte`
//!        f32[6]   the soup's own bounds: **centre, then half-extent**
//! ```
//!
//! # Omega Collection's node is 19 bytes, and nothing else moved
//!
//! All 38 `track_col.col` files on the PS4 Omega Collection state a node stride
//! of **19** where 2048's state 24, and in each the next `"----"` sits at exactly
//! `0x14 + 19 * N`. Everything after the node array is the same container: on
//! `environments2048\altima` it is **byte-identical** to 2048's Vita file, which
//! is smaller by exactly `5 * N`. See [`NodeLayout::Packed`] and
//! `docs/formats/2048-collision.md` for the field order and figures.
//!
//! # The bounds are centre and half-extent, not min and max
//!
//! First read the other way, which looks plausible. On `altima` the pair
//! `(-113.495, 333.607, 350.514)` / `(1007.426, 383.553, 1246.650)` is `centre ±
//! extent` of the soup's vertices **exactly, on all six numbers**, and nothing
//! like a min and a max.
//!
//! # What is here for the record rather than for use
//!
//! The [`Node`] array is parsed and checked and then unused: this engine casts
//! rays against a [`TriangleSoup`](../../../crates/physics) of its own. It is
//! read so the format page stays checkable against the file
//! (`every_leaf_names_triangles_that_exist`).
//!
//! See `docs/formats/2048-collision.md` for the evidence and confidence scores,
//! and `docs/ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md`
//! for the two loaders this is read from.

use std::fmt;

use crate::collision::{
    CollisionGeometry, CollisionMesh, CollisionNode, DEFAULT_VERTEX_SCALAR, SurfaceKind,
};

/// Magic at `+0x00`.
pub const MAGIC: &[u8; 4] = b"kdtr";

/// The tag before every section, and once more at the end of the file.
pub const SECTION_TAG: &[u8; 4] = b"----";

/// The only version `KdTree_Load` accepts.
pub const VERSION: u32 = 1;

/// Bytes per k-d tree node in 2048's files; see [`NodeLayout::Wide`].
pub const NODE_LEN: usize = 0x18;

/// Bytes per k-d tree node in the Omega Collection's files; see
/// [`NodeLayout::Packed`].
pub const NODE_LEN_PACKED: usize = 19;

/// Bytes per triangle, stated by the file and **refused if anything else**: the
/// original reads `stride * triangle_count` bytes into a buffer allocated as `6 *
/// triangle_count`, so a larger stride is a heap overflow in the real game.
pub const TRIANGLE_LEN: usize = 6;

/// The child index that means "no child".
pub const NULL_CHILD: i32 = -1;

/// Something wrong with a `track_col.col`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The magic is not [`MAGIC`].
    BadMagic {
        /// What the first four bytes were.
        magic: [u8; 4],
    },
    /// The version does not read as four hex digits, or is not [`VERSION`].
    BadVersion {
        /// The four bytes, as they read.
        version: String,
    },
    /// A section tag was not [`SECTION_TAG`].
    BadSectionTag {
        /// Which section was expected next.
        what: &'static str,
        /// Offset the tag was read at.
        at: usize,
    },
    /// A field or array runs past the end of the file.
    OutOfBounds {
        /// What did not fit.
        what: &'static str,
        /// Where it would have ended.
        end: usize,
        /// The file's length.
        len: usize,
    },
    /// A k-d node stride that is neither [`NODE_LEN`] nor [`NODE_LEN_PACKED`].
    BadNodeStride {
        /// The value read.
        stride: usize,
    },
    /// A stride the format does not permit.
    BadStride {
        /// What the stride is for.
        what: &'static str,
        /// The value read.
        stride: usize,
        /// The only value the loader can handle.
        expected: usize,
    },
    /// An index naming an element that does not exist.
    BadIndex {
        /// What the index is into.
        what: &'static str,
        /// The value read.
        index: u32,
        /// How many there are.
        count: usize,
    },
    /// The file has bytes left over after the final tag.
    TrailingBytes {
        /// How many.
        extra: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic { magic } => {
                write!(f, "magic is {magic:02x?}, not {:02x?}", MAGIC)
            }
            Self::BadVersion { version } => write!(f, "version {version:?} is not {VERSION}"),
            Self::BadSectionTag { what, at } => {
                write!(f, "no \"----\" before the {what} section at {at:#x}")
            }
            Self::OutOfBounds { what, end, len } => {
                write!(f, "{what} ends at {end} but the file is {len} bytes")
            }
            Self::BadNodeStride { stride } => write!(
                f,
                "k-d node stride is {stride}, and only {NODE_LEN} (2048) and \
                 {NODE_LEN_PACKED} (Omega Collection) read"
            ),
            Self::BadStride {
                what,
                stride,
                expected,
            } => write!(f, "{what} stride is {stride}, and only {expected} loads"),
            Self::BadIndex { what, index, count } => {
                write!(f, "{what} index {index} names one of {count}")
            }
            Self::TrailingBytes { extra } => write!(f, "{extra} byte(s) after the last section"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// An axis-aligned box, as the file stores one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The box's centre.
    pub centre: [f32; 3],
    /// Half the box's size on each axis, so the box is `centre ± extent`.
    pub extent: [f32; 3],
}

impl Bounds {
    /// The low corner.
    #[must_use]
    pub fn min(&self) -> [f32; 3] {
        std::array::from_fn(|a| self.centre[a] - self.extent[a])
    }

    /// The high corner.
    #[must_use]
    pub fn max(&self) -> [f32; 3] {
        std::array::from_fn(|a| self.centre[a] + self.extent[a])
    }
}

/// One k-d tree node. **Read for the record; nothing here traverses it.**
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    /// Index of the low child, or `None` on a leaf. Stored as an index and
    /// turned into a pointer at load; `KdTree_Load`'s fix-up loop says these two
    /// fields are children and [`NULL_CHILD`] the null.
    pub low: Option<usize>,
    /// Index of the high child, or `None` on a leaf.
    pub high: Option<usize>,
    /// Which axis the node splits on, `0`/`1`/`2`, or `None` on a leaf.
    pub axis: Option<u8>,
    /// Where on that axis it splits. Meaningless on a leaf, where it is zero.
    pub split: f32,
    /// How many triangles a leaf holds, and `0` on an internal node.
    pub triangle_count: usize,
    /// Where this leaf's run starts in [`KdCollision::leaves`].
    pub first_leaf: usize,
    /// The upper half of the word [`triangle_count`](Self::triangle_count) is
    /// the lower half of, or `None` in a layout with no such word.
    ///
    /// **One value per file**, the same on every node, so it is carried rather
    /// than named (`0x000b` on altima, `0x0033` on arena, park and square,
    /// `0x0084` on mall, `0x001c` on subway, ...). Always `None` for
    /// [`NodeLayout::Packed`].
    pub unknown: Option<u16>,
}

impl Node {
    /// Whether this node holds triangles rather than children.
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        self.low.is_none() && self.high.is_none()
    }
}

/// How a file lays out one k-d node, stated by the file's own stride field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeLayout {
    /// 2048's 24-byte node ([`NODE_LEN`]):
    ///
    /// ```text
    /// +0x00  i32  low child, -1 for none
    /// +0x04  i32  high child, -1 for none
    /// +0x08  u32  split axis 0/1/2, all-ones on a leaf
    /// +0x0c  f32  split position
    /// +0x10  u32  triangle count in the low half, `unknown` in the high
    /// +0x14  u32  first leaf index
    /// ```
    Wide,
    /// The Omega Collection's 19-byte node ([`NODE_LEN_PACKED`]), a packed
    /// record with no alignment:
    ///
    /// ```text
    /// +0x00  i32  low child, -1 for none
    /// +0x04  i32  high child, -1 for none
    /// +0x08  u8   split axis 0/1/2, 0xff on a leaf
    /// +0x09  u16  triangle count
    /// +0x0b  f32  split position
    /// +0x0f  u32  first leaf index
    /// ```
    ///
    /// **Confidence 85.** Measured against 2048's own file for the one circuit
    /// the packages ship byte-identically past the node array (`altima`, 27,199
    /// nodes): children, axis, triangle count and leaf start agree on all, the
    /// split on 22,582 exactly and on the other 4,617 to within one ulp (a
    /// different export rebuilt the tree). Nothing is asserted about the other
    /// nine shared circuits, which Omega re-exported with different trees. The
    /// record is unread by this engine either way.
    Packed,
}

impl NodeLayout {
    /// Bytes per node.
    #[must_use]
    pub const fn len(self) -> usize {
        match self {
            Self::Wide => NODE_LEN,
            Self::Packed => NODE_LEN_PACKED,
        }
    }

    /// Never true; here because a `len` without it is a lint.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }
}

/// The triangle soup the tree indexes.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// Vertex positions, already in world space - see [`collision_nodes`].
    pub vertices: Vec<[f32; 3]>,
    /// Triangles, as index triples into [`vertices`](Self::vertices).
    pub triangles: Vec<[u16; 3]>,
    /// One surface byte per triangle. See [`SurfaceByte`].
    pub surfaces: Vec<u8>,
    /// The soup's own bounds.
    pub bounds: Bounds,
}

/// A decoded `track_col.col`.
#[derive(Debug, Clone, PartialEq)]
pub struct KdCollision {
    /// How the file laid its nodes out.
    pub layout: NodeLayout,
    /// The k-d tree's nodes, in file order.
    pub nodes: Vec<Node>,
    /// Every leaf's triangle indices, concatenated; a leaf names a run of them.
    pub leaves: Vec<u16>,
    /// The tree's own bounds, which are slightly larger than the soup's.
    pub bounds: Bounds,
    /// The geometry.
    pub mesh: Mesh,
}

/// Decodes a `track_col.col`.
///
/// # Errors
/// # Errors
///
/// Refuses a wrong magic or version, a missing section tag, any array past the
/// end of the file, a triangle stride the original could not load, an index
/// naming nothing, and bytes left after the last tag. Each is a decode failure,
/// not a track with no collision; the caller should say so.
pub fn parse(file: &[u8]) -> Result<KdCollision> {
    let mut r = Reader { file, at: 0 };
    let magic: [u8; 4] = r.array("magic")?;
    if &magic != MAGIC {
        return Err(Error::BadMagic { magic });
    }
    let raw: [u8; 4] = r.array("version")?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    if u32::from_str_radix(text.trim(), 16).ok() != Some(VERSION) {
        return Err(Error::BadVersion { version: text });
    }

    r.tag("nodes")?;
    let stride = r.u32("node stride")? as usize;
    let layout = match stride {
        NODE_LEN => NodeLayout::Wide,
        NODE_LEN_PACKED => NodeLayout::Packed,
        _ => return Err(Error::BadNodeStride { stride }),
    };
    let node_count = r.u32("node count")? as usize;
    let mut nodes = Vec::with_capacity(node_count.min(r.remaining() / layout.len()));
    for _ in 0..node_count {
        nodes.push(match layout {
            NodeLayout::Wide => r.wide_node(node_count)?,
            NodeLayout::Packed => r.packed_node(node_count)?,
        });
    }

    r.tag("leaf indices")?;
    let leaf_count = r.u32("leaf index count")? as usize;
    let mut leaves = Vec::with_capacity(leaf_count.min(r.remaining() / 2));
    for _ in 0..leaf_count {
        leaves.push(r.u16("leaf index")?);
    }

    r.tag("tree bounds")?;
    let bounds = r.bounds()?;

    r.tag("triangle soup")?;
    let vertex_count = usize::from(r.u16("vertex count")?);
    let mut vertices = Vec::with_capacity(vertex_count.min(r.remaining() / 12));
    for _ in 0..vertex_count {
        vertices.push([r.f32("vertex")?, r.f32("vertex")?, r.f32("vertex")?]);
    }
    let stride = r.u32("triangle stride")? as usize;
    if stride != TRIANGLE_LEN {
        return Err(Error::BadStride {
            what: "triangle",
            stride,
            expected: TRIANGLE_LEN,
        });
    }
    let triangle_count = usize::from(r.u16("triangle count")?);
    let mut triangles = Vec::with_capacity(triangle_count.min(r.remaining() / TRIANGLE_LEN));
    for _ in 0..triangle_count {
        let mut corners = [0u16; 3];
        for corner in &mut corners {
            let index = r.u16("vertex")?;
            if usize::from(index) >= vertex_count {
                return Err(Error::BadIndex {
                    what: "vertex",
                    index: u32::from(index),
                    count: vertex_count,
                });
            }
            *corner = index;
        }
        triangles.push(corners);
    }
    let surface_count = usize::from(r.u16("surface byte count")?);
    let surfaces = r.bytes("surface bytes", surface_count)?.to_vec();
    let mesh_bounds = r.bounds()?;

    for &index in &leaves {
        if usize::from(index) >= triangle_count {
            return Err(Error::BadIndex {
                what: "leaf",
                index: u32::from(index),
                count: triangle_count,
            });
        }
    }

    r.tag("end")?;
    if r.remaining() != 0 {
        return Err(Error::TrailingBytes {
            extra: r.remaining(),
        });
    }

    Ok(KdCollision {
        layout,
        nodes,
        leaves,
        bounds,
        mesh: Mesh {
            vertices,
            triangles,
            surfaces,
            bounds: mesh_bounds,
        },
    })
}

/// The `.vex` collision class one surface byte stands for, or `None` for a value
/// nothing has placed.
///
/// **Six of the nine shipped values are read out of the executable.**
/// `TrackCollision_MeshFromNode` (`0x8126f800`) switches on a collision node's
/// `.vex` class ID and passes exactly these bytes to the mesh builder:
///
/// | class | ID | byte |
/// | --- | --- | --- |
/// | `Floor Collision` | `0x3b9` | 2 |
/// | `Mag Floor Collision` | `0x3e6` | 3 |
/// | `Wall Collision` | `0x3ba` | 4 |
/// | `Force Field Collision` | `0x3f2` | 5 |
/// | `Track Wall Collision` | `0x3ed` | 6 |
/// | `Reset Collision` | `0x3cd` | 7 |
///
/// `Force Field Collision` is declared only by 2048 (its own class-name table).
/// `Cage Collision` (`0x3e7`) is branched past before any byte is chosen, as
/// [`SurfaceKind::Cage`] documents on Pulse.
///
/// The **same six** fall out of an independent measurement: 2048's DLC re-ships
/// twelve HD circuits, and matching the two titles' triangles by position over
/// 170,744 pairs reproduces this table with no disagreement. See
/// `docs/formats/2048-collision.md`.
///
/// `None` for `10`, `11` and `12`, which no `.vex` class produces and only 2048's
/// own circuits carry; see [`is_measured_floor`].
#[must_use]
pub fn class_of(surface: u8) -> Option<SurfaceKind> {
    Some(match surface {
        2 => SurfaceKind::Floor,
        3 => SurfaceKind::MagFloor,
        4 => SurfaceKind::Wall,
        5 | 6 => SurfaceKind::TrackWall,
        7 => SurfaceKind::Reset,
        _ => return None,
    })
}

/// Whether a surface byte no `.vex` class produces is nonetheless measured to be
/// drivable floor.
///
/// **`10` and `11`: yes, from three measurements.** They appear on 2048's own
/// circuits only. 99.4 % and 99.9 % of their triangles are near-horizontal
/// (87.6 % for the confirmed [`SurfaceKind::Floor`] byte); 91.1 % and 74.8 % sit
/// *under the racing line* (confirmed floor: 59.1 %); and counting them lifts
/// the racing line's ground coverage from 1.1 % to 47.7 % on `cathedral` (9.6 to
/// 27.5 % on `tower`, 10.5 to 32.7 % on `park`) **while changing nothing on any
/// circuit that does not use them**, the control. Confidence **85**.
///
/// **`12`: also yes, on much thinner evidence**: 102 near-horizontal triangles
/// on `altima` alone (0.1 % of its coverage), indistinguishable from flat
/// scenery. Confidence **50**; grouped here because invisible floor versus a
/// hole is a coin toss and this side fails visibly.
#[must_use]
pub fn is_measured_floor(surface: u8) -> bool {
    // Three consecutive measured values, not a range anything states; clippy's
    // `manual_range_patterns` is why it is a range.
    matches!(surface, 10..=12)
}

/// The physics surface a byte drives, however it was placed.
#[must_use]
pub fn surface_kind(surface: u8) -> Option<SurfaceKind> {
    class_of(surface).or(is_measured_floor(surface).then_some(SurfaceKind::Floor))
}

/// Groups a decoded soup into the [`CollisionNode`]s the rest of the engine
/// consumes, one per surface kind.
///
/// **The vertices are already world space**, so no transform is composed onto
/// them; measured here by matching 170,744 of 2048's triangles to HD's
/// world-space ones within one unit.
///
/// Kinds come out in [`SurfaceKind::ALL`] order, never map order: a collider's
/// index *is* its position in the world and
/// `oag_physics::forces::Environment::self_collider` compares against it, so
/// this ordering feeds simulation state.
///
/// A byte [`surface_kind`] does not place is dropped and its count returned, so
/// the caller can say so rather than race on a subset.
#[must_use]
pub fn collision_nodes(decoded: &KdCollision) -> (Vec<CollisionNode>, Vec<(u8, usize)>) {
    let mesh = &decoded.mesh;
    let mut unplaced: Vec<(u8, usize)> = Vec::new();
    for &surface in &mesh.surfaces {
        if surface_kind(surface).is_none() {
            match unplaced.iter_mut().find(|(byte, _)| *byte == surface) {
                Some((_, count)) => *count += 1,
                None => unplaced.push((surface, 1)),
            }
        }
    }
    unplaced.sort_unstable();

    let mut out = Vec::new();
    for (node_index, kind) in SurfaceKind::ALL.into_iter().enumerate() {
        // One pass per kind, keeping only the vertices that kind's triangles
        // name, so a collider's bounding volume is its own.
        let mut remap = vec![u16::MAX; mesh.vertices.len()];
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for (triangle, &surface) in mesh.triangles.iter().zip(&mesh.surfaces) {
            if surface_kind(surface) != Some(kind) {
                continue;
            }
            let mut corners = [0u16; 3];
            for (corner, &index) in corners.iter_mut().zip(triangle) {
                let slot = &mut remap[usize::from(index)];
                if *slot == u16::MAX {
                    *slot = u16::try_from(vertices.len()).unwrap_or(u16::MAX);
                    vertices.push(mesh.vertices[usize::from(index)]);
                }
                *corner = *slot;
            }
            triangles.push(corners);
        }
        if triangles.is_empty() {
            continue;
        }
        out.push(CollisionNode {
            kind,
            node_index,
            name: Some(kind.node_name().to_string()),
            geometry: CollisionGeometry {
                version: crate::collision::HEADER_WORD,
                meshes: vec![CollisionMesh {
                    // No scalar chunk in this container: absent means neutral, not
                    // zero (`DEFAULT_VERTEX_SCALAR`).
                    vertex_scalars: vec![DEFAULT_VERTEX_SCALAR; vertices.len()],
                    vertices,
                    triangles,
                    chunks: Vec::new(),
                }],
            },
        });
    }
    (out, unplaced)
}

/// The `track_col.col` beside a `track.vex`, or `None` for a name that is not a
/// track file.
///
/// A reversed circuit's `track_reversed.vex` pairs with `track_col_reversed.col`
/// (`_col` goes between `track` and `_reversed`). Omega ships all twelve
/// reversed circuits that way; 2048's base package ships no reversed `.col`, so
/// there the reversed name is not found. Same sibling-name idiom as
/// `oag_mesh::mesh::rcs::sibling_name`: the pairing is by archive directory
/// position, not anything either file states.
#[must_use]
pub fn sibling_name(track: &str) -> Option<String> {
    let cut = track.rfind(['/', '\\'])? + 1;
    let stem = track[cut..].strip_suffix(".vex")?;
    let (base, reversed) = match stem.len().checked_sub(REVERSED.len()) {
        Some(at) if stem.is_char_boundary(at) && stem[at..].eq_ignore_ascii_case(REVERSED) => {
            (&stem[..at], &stem[at..])
        }
        _ => (stem, ""),
    };
    if !base.eq_ignore_ascii_case("track") {
        return None;
    }
    Some(format!("{}{base}_col{reversed}.col", &track[..cut]))
}

/// The suffix that turns `track` into the reversed circuit's `track_reversed`.
const REVERSED: &str = "_reversed";

/// A cursor that refuses to read past the end.
struct Reader<'a> {
    file: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.file.len().saturating_sub(self.at)
    }

    fn bytes(&mut self, what: &'static str, len: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(len).ok_or(Error::OutOfBounds {
            what,
            end: usize::MAX,
            len: self.file.len(),
        })?;
        let slice = self.file.get(self.at..end).ok_or(Error::OutOfBounds {
            what,
            end,
            len: self.file.len(),
        })?;
        self.at = end;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self, what: &'static str) -> Result<[u8; N]> {
        Ok(self.bytes(what, N)?.try_into().expect("N bytes"))
    }

    fn tag(&mut self, what: &'static str) -> Result<()> {
        let at = self.at;
        let tag: [u8; 4] = self.array("section tag")?;
        if &tag == SECTION_TAG {
            Ok(())
        } else {
            Err(Error::BadSectionTag { what, at })
        }
    }

    fn u8(&mut self, what: &'static str) -> Result<u8> {
        Ok(u8::from_le_bytes(self.array(what)?))
    }

    fn wide_node(&mut self, count: usize) -> Result<Node> {
        let low = self.child("k-d node child", count)?;
        let high = self.child("k-d node child", count)?;
        let axis = self.u32("k-d node axis")?;
        let split = self.f32("k-d node split")?;
        let counts = self.u32("k-d node leaf run")?;
        let first_leaf = self.u32("k-d node leaf start")? as usize;
        Ok(Node {
            low,
            high,
            // Internal nodes state 0/1/2, a leaf all-ones (the children's null word).
            axis: u8::try_from(axis).ok().filter(|_| axis < 3),
            split,
            triangle_count: (counts & 0xffff) as usize,
            first_leaf,
            unknown: Some((counts >> 16) as u16),
        })
    }

    fn packed_node(&mut self, count: usize) -> Result<Node> {
        let low = self.child("k-d node child", count)?;
        let high = self.child("k-d node child", count)?;
        let axis = self.u8("k-d node axis")?;
        let triangle_count = usize::from(self.u16("k-d node leaf run")?);
        let split = self.f32("k-d node split")?;
        let first_leaf = self.u32("k-d node leaf start")? as usize;
        Ok(Node {
            low,
            high,
            axis: Some(axis).filter(|&a| a < 3),
            split,
            triangle_count,
            first_leaf,
            unknown: None,
        })
    }

    fn u16(&mut self, what: &'static str) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array(what)?))
    }

    fn u32(&mut self, what: &'static str) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array(what)?))
    }

    fn f32(&mut self, what: &'static str) -> Result<f32> {
        Ok(f32::from_le_bytes(self.array(what)?))
    }

    fn child(&mut self, what: &'static str, count: usize) -> Result<Option<usize>> {
        let raw = self.u32(what)? as i32;
        if raw == NULL_CHILD {
            return Ok(None);
        }
        let index = usize::try_from(raw).ok().filter(|i| *i < count);
        index.map(Some).ok_or(Error::BadIndex {
            what,
            index: raw as u32,
            count,
        })
    }

    fn bounds(&mut self) -> Result<Bounds> {
        Ok(Bounds {
            centre: [
                self.f32("bounds")?,
                self.f32("bounds")?,
                self.f32("bounds")?,
            ],
            extent: [
                self.f32("bounds")?,
                self.f32("bounds")?,
                self.f32("bounds")?,
            ],
        })
    }
}

#[cfg(test)]
mod tests;
