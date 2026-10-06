//! Collision geometry: an indexed triangle soup, separate from the render mesh.
//!
//! `CollisionNode_ParseChunks` (`0x08934a48`) reads a chunked payload holding one
//! triangle soup per collidable piece of the track: no BSP, quadtree or
//! heightfield, and **not** the render mesh.
//!
//! ```text
//! payload:
//!   +0x00  u32   all ones on every node seen; the loader reads it and
//!                does not act on it
//!   +0x04  u32   object count
//!         then   one object per count
//!
//! object:
//!   +0x00  u32   chunk count, 3 on every object seen
//!         then   one chunk per count, in the order 1, 3, 2
//!
//! chunk:
//!   +0x00  u32   type
//!   +0x04  u16   element stride, matching the type
//!   +0x06  u16   count, elements rather than bytes
//!   +0x08         count * stride bytes
//! ```
//!
//! | Chunk | Stride | Contents |
//! | ---: | ---: | --- |
//! | 1 | `0x0c` | Vertex positions, 3 x `f32` |
//! | 2 | `0x06` | Triangle indices, 3 x `u16` |
//! | 3 | `0x04` | One `f32` **per vertex** |
//!
//! # The checks that settle the layout
//!
//! Validated against **319 collision nodes across two platforms** (130 in the
//! PSP `Data.wad`, 189 in the PS2 `WADS2.WAD`): 18,745 objects, 56,235 chunks,
//! 602,086 vertices, 594,615 triangles. Four invariants hold exactly:
//!
//! 1. **The walk closes.** It consumes the payload down to its zero-filled
//!    16-byte alignment padding: [`CollisionGeometry::padded_len`] equals the
//!    payload length on 319 of 319 nodes (53 need no padding). A parse one field
//!    out cannot do that; it is the argument that settled [`WO Track`](crate::track).
//! 2. **The chunk header's second field is the element stride**, matching the
//!    type on 56,235 of 56,235 chunks. The loader hard-codes strides and discards
//!    it; as a check it makes a drifting walk fail on the next chunk. See
//!    [`Error::StrideMismatch`].
//! 3. **Chunk 3 is per-vertex**, on 18,745 of 18,745 objects (predicted from
//!    `0x0881835c` indexing it by a triangle's three vertex indices). See
//!    [`Error::ScalarCountMismatch`].
//! 4. **Every triangle index names a vertex**, across all 1.78 million.
//!
//! Confidence **94**: exact agreement with shipped data on a second platform's
//! build; not higher because nothing has been traced under an emulator (see
//! `docs/reverse-engineering/confidence-rubric.md`).
//!
//! Two observations that constrain what the data means:
//!
//! - **Every vertex scalar in both builds is exactly `1.0`** (all 602,086), so no
//!   shipped track uses the field.
//! - **The first word is `0xffff_ffff` on all 319 nodes**; calling it a version
//!   is a reading, not an observation.
//!
//! # The five node types are one class
//!
//! `Collision_RegisterNodeClasses` (`0x08934d44`) registers all five collision
//! [`.vex`](crate::vex) classes with one vtable. The class ID selects only a
//! surface type and a friction constant, so a magstrip is ordinary floor with
//! surface type 3. See [`SurfaceKind`].
//!
//! `Cage Collision` is not dead content: **the PS2 build ships 6 cage nodes and
//! the PSP build none**, matching `Definition.xml`'s per-track
//! `collisionCageEnabled`.
//!
//! Friction is `-1.0` for [`Floor`](SurfaceKind::Floor),
//! [`MagFloor`](SurfaceKind::MagFloor) and [`Reset`](SurfaceKind::Reset), a
//! **sentinel, not a value**: contact combination averages the two sides but
//! forces zero if either is negative. It is [`None`] here; [`combine_friction`]
//! is the rule.
//!
//! `collider+0x64` is a **friction coefficient**, not restitution:
//! `Body_ResolveContact` (`0x0884e968`) takes restitution from `body+0x388` and
//! uses `collider+0x64` only to scale the contact's tangential relative
//! velocity. See `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
//!
//! See `docs/formats/collision.md` for the evidence and
//! `docs/ghidra/functions/psp-pulse-usa/collision.md` for the addresses.

use std::fmt;
use std::ops::Range;

use crate::vex;
use oag_formats::ByteOrder;

/// Bytes of payload header before the first object.
pub const HEADER_LEN: usize = 8;

/// Bytes of object header before its first chunk: one `u32` chunk count.
pub const OBJECT_HEADER_LEN: usize = 4;

/// Bytes of chunk header before its payload.
pub const CHUNK_HEADER_LEN: usize = 8;

/// Alignment of a collision node's payload inside a `.vex` file.
///
/// The chunk walk ends short of `data_size` by up to 15 zero bytes (266 of 319
/// nodes), always exactly `padded_len() - encoded_len()`, which makes the closure
/// check exact.
pub const PAYLOAD_ALIGN: usize = 16;

/// Chunk type holding vertex positions, 3 x `f32`.
pub const CHUNK_VERTICES: u32 = 1;

/// Chunk type holding triangle indices, 3 x `u16`.
pub const CHUNK_TRIANGLES: u32 = 2;

/// Chunk type holding one `f32` per vertex.
pub const CHUNK_VERTEX_SCALARS: u32 = 3;

/// Bytes per vertex in a [`CHUNK_VERTICES`] chunk.
pub const VERTEX_STRIDE: usize = 0x0c;

/// Bytes per triangle in a [`CHUNK_TRIANGLES`] chunk.
pub const TRIANGLE_STRIDE: usize = 0x06;

/// Bytes per scalar in a [`CHUNK_VERTEX_SCALARS`] chunk.
pub const SCALAR_STRIDE: usize = 0x04;

/// The value of the payload's first word on every node examined.
pub const HEADER_WORD: u32 = 0xffff_ffff;

/// What a vertex scalar is when the chunk is absent.
///
/// `CollisionMesh_AvgVertexScalar` (`0x0881835c`) returns `1.0` for a missing
/// array, so absent means "neutral" (every shipped scalar is this anyway).
pub const DEFAULT_VERTEX_SCALAR: f32 = 1.0;

/// Friction of a [`Wall`](SurfaceKind::Wall) surface, from `collider+0x64`.
pub const WALL_FRICTION: f32 = 0.05;

/// Bytes per element of a chunk type, or `None` if unknown.
///
/// Only these three types appear in either build. A chunk also declares its own
/// stride at `+0x04`; the two agree everywhere, so this is a check.
#[must_use]
pub fn chunk_stride(kind: u32) -> Option<usize> {
    match kind {
        CHUNK_VERTICES => Some(VERTEX_STRIDE),
        CHUNK_TRIANGLES => Some(TRIANGLE_STRIDE),
        CHUNK_VERTEX_SCALARS => Some(SCALAR_STRIDE),
        _ => None,
    }
}

/// Something wrong with a collision payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the payload header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// A structure runs past the end of the payload.
    OutOfBounds {
        /// What was being read.
        what: &'static str,
        /// Index of the object being read.
        object: usize,
        /// Where it would end.
        end: usize,
        /// Bytes available.
        len: usize,
    },
    /// A chunk type with no known stride.
    ///
    /// Fatal rather than skipped: the declared stride could step over a fourth
    /// type, but its contents would be unknown, and a decoder that quietly
    /// ignores data is worse than one that stops (as
    /// [`vex::Error::UnsupportedVertexType`] does).
    UnknownChunk {
        /// Index of the object it appeared in.
        object: usize,
        /// The type found.
        kind: u32,
    },
    /// A chunk whose declared stride is not the one its type implies.
    ///
    /// They agree on all 56,235 chunks, so a disagreement means the walk drifted
    /// or the type numbering is wrong; the bytes after it cannot be trusted.
    StrideMismatch {
        /// Index of the object.
        object: usize,
        /// The chunk type.
        kind: u32,
        /// The stride the chunk declares.
        declared: u16,
        /// The stride the type implies.
        expected: usize,
    },
    /// Two chunks of the same type in one object; refused, since which wins would
    /// be a guess (the loader keeps one pointer per array).
    DuplicateChunk {
        /// Index of the object.
        object: usize,
        /// The type that repeated.
        kind: u32,
    },
    /// A triangle index that does not name a vertex.
    BadTriangleIndex {
        /// Index of the object.
        object: usize,
        /// Index of the triangle within the object.
        triangle: usize,
        /// The value found.
        index: u16,
        /// Vertices available.
        vertex_count: usize,
    },
    /// A [`CHUNK_VERTEX_SCALARS`] chunk that is not one entry per vertex.
    ///
    /// Carries the triangle count too: `scalars == triangles` would mean the
    /// per-triangle reading was right on some asset the survey missed.
    ScalarCountMismatch {
        /// Index of the object.
        object: usize,
        /// Scalars found.
        scalars: usize,
        /// Vertices found.
        vertices: usize,
        /// Triangles found.
        triangles: usize,
    },
    /// A node payload the chunk walk does not account for.
    ///
    /// Only from [`from_vex`], which holds it to
    /// [`CollisionGeometry::padded_len`]; see [`PAYLOAD_ALIGN`].
    PayloadNotAccountedFor {
        /// Index of the node in the file's node list.
        node: usize,
        /// Bytes the walk consumed.
        consumed: usize,
        /// Bytes the node declares.
        len: usize,
    },
    /// The enclosing `.vex` file could not be walked (only from [`from_vex`]).
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
            Self::TooShort { got } => {
                write!(f, "need at least {HEADER_LEN} bytes, got {got}")
            }
            Self::OutOfBounds {
                what,
                object,
                end,
                len,
            } => write!(
                f,
                "object {object}: {what} ends at {end} but the payload is {len} bytes"
            ),
            Self::UnknownChunk { object, kind } => write!(
                f,
                "object {object}: chunk type {kind} is not one of \
                 {CHUNK_VERTICES}, {CHUNK_TRIANGLES} or {CHUNK_VERTEX_SCALARS}"
            ),
            Self::StrideMismatch {
                object,
                kind,
                declared,
                expected,
            } => write!(
                f,
                "object {object}: chunk type {kind} declares a stride of \
                 {declared}, not {expected}"
            ),
            Self::DuplicateChunk { object, kind } => {
                write!(f, "object {object}: two chunks of type {kind}")
            }
            Self::BadTriangleIndex {
                object,
                triangle,
                index,
                vertex_count,
            } => write!(
                f,
                "object {object}: triangle {triangle} names vertex {index}, but \
                 there are only {vertex_count}"
            ),
            Self::ScalarCountMismatch {
                object,
                scalars,
                vertices,
                triangles,
            } => write!(
                f,
                "object {object}: {scalars} vertex scalars for {vertices} \
                 vertices ({triangles} triangles)"
            ),
            Self::PayloadNotAccountedFor {
                node,
                consumed,
                len,
            } => write!(
                f,
                "node {node}: the chunk walk consumed {consumed} of {len} bytes, \
                 which is not the {PAYLOAD_ALIGN}-byte padding every shipped \
                 node has"
            ),
            Self::Vex(error) => write!(f, "{error}"),
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

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// Which collision class a node is, and therefore how it behaves.
///
/// Pulse's five share one vtable and one parser; the class ID selects a surface
/// type and a friction constant. [`TrackWall`](Self::TrackWall) is a sixth that
/// only Wipeout HD authors; it takes the same parser, and what selects its
/// surface type on HD is unread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    /// `Wall Collision`, surface type 0. The only springy surface. 40 nodes on
    /// the PSP disc, 59 on the PS2 one.
    Wall,
    /// `Floor Collision`, surface type 1. 40 nodes on PSP, 59 on PS2.
    Floor,
    /// `Reset Collision`, surface type 2. Excluded from ordinary raycasts; a
    /// contact triggers a respawn. 26 nodes on PSP, 32 on PS2.
    Reset,
    /// `Mag Floor Collision`, surface type 3: the magstrip surface. Byte-identical
    /// to [`Floor`](Self::Floor) at load apart from the type field. 24 nodes on
    /// PSP, 33 on PS2.
    MagFloor,
    /// `Cage Collision`, which the loader parses and then skips.
    ///
    /// **Only the PS2 build ships any** (6 nodes, none on PSP): content for a
    /// platform whose loader still drops it, matching the per-track
    /// `collisionCageEnabled` plugin attribute.
    Cage,
    /// `collision_trackwall`: the barrier along the road, **Wipeout HD only**.
    ///
    /// Exactly one node per circuit on all 16 of HD's, 3,036 to 4,974 triangles,
    /// 97 % near-vertical, co-extensive with the floor rather than
    /// [`Wall`](Self::Wall)'s larger scenery volume. Neither Pulse disc authors
    /// one. See
    /// [`vex::CLASS_TRACK_WALL_COLLISION`](crate::vex::CLASS_TRACK_WALL_COLLISION)
    /// for the four measurements.
    TrackWall,
}

impl SurfaceKind {
    /// Every kind, in class-ID order.
    pub const ALL: [Self; 6] = [
        Self::Floor,
        Self::Wall,
        Self::Reset,
        Self::MagFloor,
        Self::Cage,
        Self::TrackWall,
    ];

    /// The kind a `.vex` class ID selects, or `None` if it is not a collision
    /// class. The IDs come from `classes`
    /// ([`vex::classes_of`]).
    ///
    /// **Only version 6's are recovered.** Pure PSP's `Data.wad` has 171 `.vex`
    /// files (23,677 nodes) and none of Pulse's five IDs; its whole class-ID
    /// space differs (`0x6d` and `0x11e` dominate where Pulse has `0x6e` and
    /// `0x125`). Where Pure keeps collision geometry is open, so a version-4
    /// table returns `None` for every kind.
    #[must_use]
    pub fn from_class_id(class_id: u32, classes: vex::classes::Classes) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.class_id(classes) == Some(class_id))
    }

    /// The `.vex` class ID of this kind under `classes`, or `None` where that
    /// version's ID is not recovered ("nobody found it yet", not "no such
    /// surface").
    #[must_use]
    pub fn class_id(self, classes: vex::classes::Classes) -> Option<u32> {
        match self {
            Self::Wall => classes.wall_collision,
            Self::Floor => classes.floor_collision,
            Self::Reset => classes.reset_collision,
            Self::MagFloor => classes.mag_floor_collision,
            Self::Cage => classes.cage_collision,
            Self::TrackWall => classes.track_wall_collision,
        }
    }

    /// The node name from the class table. [`TrackWall`](Self::TrackWall) is the
    /// exception: it is the name HD's circuits give the node.
    #[must_use]
    pub fn node_name(self) -> &'static str {
        match self {
            Self::Wall => "Wall Collision",
            Self::Floor => "Floor Collision",
            Self::Reset => "Reset Collision",
            Self::MagFloor => "Mag Floor Collision",
            Self::Cage => "Cage Collision",
            Self::TrackWall => "collision_trackwall",
        }
    }

    /// The surface-type enum the loader stores at `+0x6c`.
    ///
    /// `None` for [`Cage`](Self::Cage) (never reaches a collider) and for
    /// [`TrackWall`](Self::TrackWall), whose type is written by **HD's** loader,
    /// unread (no PS3 binary has been disassembled). What the physics does with
    /// it is decided in `oag_gameplay::collision`.
    #[must_use]
    pub fn surface_type(self) -> Option<u8> {
        match self {
            Self::Wall => Some(0),
            Self::Floor => Some(1),
            Self::Reset => Some(2),
            Self::MagFloor => Some(3),
            Self::Cage | Self::TrackWall => None,
        }
    }

    /// Surface friction, where `None` means **frictionless**.
    ///
    /// The file's `-1.0` for floors is a sentinel (see [`combine_friction`]);
    /// `None` keeps that unlosable, since a negative coefficient would *add*
    /// tangential velocity in the contact response.
    ///
    /// [`TrackWall`](Self::TrackWall) reports [`WALL_FRICTION`], a stated
    /// assumption rather than a measurement: `None` would make the barrier
    /// frictionless, and HD's own value is unread.
    #[must_use]
    pub fn friction(self) -> Option<f32> {
        match self {
            Self::Wall | Self::TrackWall => Some(WALL_FRICTION),
            Self::Floor | Self::Reset | Self::MagFloor => None,
            Self::Cage => None,
        }
    }
}

/// Friction of a contact between two colliders.
///
/// `Collision_AddContact` (`0x08816864`) averages the two values but forces zero
/// if **either** is negative, so a floor against a wall is `0.0`, not the average
/// of `-1.0` and `0.05`.
#[must_use]
pub fn combine_friction(a: Option<f32>, b: Option<f32>) -> f32 {
    match (a, b) {
        (Some(a), Some(b)) => (a + b) * 0.5,
        _ => 0.0,
    }
}

/// One chunk of an object, as found: the evidence trail, and what makes
/// [`CollisionMesh::encoded_len`] computable when a chunk is absent. Every
/// shipped object holds exactly three, in the order 1, 3, 2: the scalars
/// **before** the indices keeps the `f32` array 4-byte aligned when an odd
/// triangle count leaves the indices on a half-word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Chunk type from `+0x00`.
    pub kind: u32,
    /// Element stride from `+0x04`: redundant with [`chunk_stride`] and checked
    /// against it (the loader never reads it, but the exporter fills it correctly
    /// on all 56,235 chunks).
    pub stride: u16,
    /// Elements, not bytes.
    pub count: usize,
    /// Range of the chunk's payload within the collision payload.
    pub payload: Range<usize>,
}

impl Chunk {
    /// Bytes this chunk occupies, header included.
    #[must_use]
    pub fn encoded_len(&self) -> usize {
        CHUNK_HEADER_LEN + self.payload.len()
    }
}

/// One collision object: an indexed triangle soup.
///
/// Small: the shipped average is 32 vertices and 32 triangles, and a track's
/// floor is 85 to 127 of them, which is what the per-mesh sweep and prune is
/// sized for.
#[derive(Debug, Clone, PartialEq)]
pub struct CollisionMesh {
    /// Vertex positions, in the node's local space.
    pub vertices: Vec<[f32; 3]>,
    /// Triangles, as index triples into [`vertices`](Self::vertices); every index
    /// is in range ([`parse_chunks`] refuses a payload where one is not).
    pub triangles: Vec<[u16; 3]>,
    /// One scalar per vertex, [`DEFAULT_VERTEX_SCALAR`] when the chunk is absent.
    /// **What it means is not known, and no shipped track uses it**: all 602,086
    /// scalars in the PSP and PS2 builds are exactly `1.0`. It reaches the
    /// raycast hit result and accumulates into ship state, so it is wired up;
    /// "grip, friction or roughness" is a guess at confidence 40 and not named.
    pub vertex_scalars: Vec<f32>,
    /// The chunks found, in the order they appeared.
    pub chunks: Vec<Chunk>,
}

impl CollisionMesh {
    /// Whether a [`CHUNK_VERTEX_SCALARS`] chunk was present.
    ///
    /// [`vertex_scalars`](Self::vertex_scalars) is filled either way, so this
    /// tells all-`1.0` stored scalars (every shipped file) from none stored.
    #[must_use]
    pub fn has_vertex_scalars(&self) -> bool {
        self.chunks.iter().any(|c| c.kind == CHUNK_VERTEX_SCALARS)
    }

    /// The three corners of a triangle, or `None` if there is no such triangle.
    #[must_use]
    pub fn triangle(&self, triangle: usize) -> Option<[[f32; 3]; 3]> {
        let indices = self.triangles.get(triangle)?;
        Some([
            self.vertices[usize::from(indices[0])],
            self.vertices[usize::from(indices[1])],
            self.vertices[usize::from(indices[2])],
        ])
    }

    /// The mean of a triangle's three vertex scalars, as
    /// `CollisionMesh_AvgVertexScalar` (`0x0881835c`) computes it and the raycast
    /// hit result carries it; [`DEFAULT_VERTEX_SCALAR`] when the chunk was absent.
    #[must_use]
    pub fn avg_vertex_scalar(&self, triangle: usize) -> Option<f32> {
        let indices = self.triangles.get(triangle)?;
        let sum = self.vertex_scalars[usize::from(indices[0])]
            + self.vertex_scalars[usize::from(indices[1])]
            + self.vertex_scalars[usize::from(indices[2])];
        Some(sum / 3.0)
    }

    /// Axis-aligned bounds of the vertices, or `None` when there are none.
    ///
    /// Our own computation: whether the loader reads or derives the box at
    /// `+0x00`/`+0x10` of the collider object is not established.
    #[must_use]
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let first = *self.vertices.first()?;
        let mut lo = first;
        let mut hi = first;
        for v in &self.vertices {
            for axis in 0..3 {
                lo[axis] = lo[axis].min(v[axis]);
                hi[axis] = hi[axis].max(v[axis]);
            }
        }
        Some((lo, hi))
    }

    /// Bytes this object accounts for, its chunk count included.
    #[must_use]
    pub fn encoded_len(&self) -> usize {
        OBJECT_HEADER_LEN + self.chunks.iter().map(Chunk::encoded_len).sum::<usize>()
    }
}

/// A decoded collision payload: a header word and its objects.
#[derive(Debug, Clone, PartialEq)]
pub struct CollisionGeometry {
    /// The `u32` at `+0x00`, [`HEADER_WORD`] on every node examined. The loader
    /// reads it as a version and does nothing with it; nothing here distinguishes
    /// a version field from an unused slot.
    pub version: u32,
    /// The objects, in file order.
    pub meshes: Vec<CollisionMesh>,
}

impl CollisionGeometry {
    /// Bytes the decoded structure accounts for: the chunk walk's length,
    /// excluding the node's alignment padding. See [`padded_len`](Self::padded_len).
    #[must_use]
    pub fn encoded_len(&self) -> usize {
        HEADER_LEN
            + self
                .meshes
                .iter()
                .map(CollisionMesh::encoded_len)
                .sum::<usize>()
    }

    /// [`encoded_len`](Self::encoded_len) rounded up to [`PAYLOAD_ALIGN`].
    ///
    /// **Equal to the node payload's length on 319 of 319 shipped nodes**, the
    /// check that settles the layout and why [`from_vex`] refuses a node where it
    /// does not hold.
    #[must_use]
    pub fn padded_len(&self) -> usize {
        self.encoded_len().next_multiple_of(PAYLOAD_ALIGN)
    }

    /// Vertices across every object.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.meshes.iter().map(|m| m.vertices.len()).sum()
    }

    /// Triangles across every object.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.meshes.iter().map(|m| m.triangles.len()).sum()
    }
}

/// Decodes a collision node's payload.
///
/// Standalone over raw bytes, so it is testable without a disc image;
/// [`from_vex`] is the convenience layer.
///
/// # Errors
///
/// Refuses a payload too short for its header, any structure running past the
/// end, an unknown chunk type, a declared stride contradicting its type, a
/// repeated chunk type, a triangle index naming no vertex, and a vertex-scalar
/// chunk that is not one entry per vertex.
///
/// Trailing bytes are **not** an error: a node payload carries up to 15 bytes of
/// alignment padding and this does not know its input is a whole node. Compare
/// [`CollisionGeometry::padded_len`] against the payload length, as [`from_vex`]
/// does.
pub fn parse_chunks(payload: &[u8], order: ByteOrder) -> Result<CollisionGeometry> {
    if payload.len() < HEADER_LEN {
        return Err(Error::TooShort { got: payload.len() });
    }

    let version = u32_at(order, payload, 0);
    let object_count = u32_at(order, payload, 4) as usize;

    // The count is a u32 out of a file, so not a capacity: an object is at least
    // its chunk count.
    let capacity = object_count.min(payload.len() / OBJECT_HEADER_LEN);
    let mut meshes = Vec::with_capacity(capacity);

    let mut at = HEADER_LEN;
    for object in 0..object_count {
        let (mesh, next) = parse_object(payload, order, at, object)?;
        meshes.push(mesh);
        at = next;
    }

    Ok(CollisionGeometry { version, meshes })
}

/// Decodes one object, returning it and the offset just past it.
fn parse_object(
    payload: &[u8],
    order: ByteOrder,
    at: usize,
    object: usize,
) -> Result<(CollisionMesh, usize)> {
    let mut cursor = end_of(
        "object header",
        object,
        at,
        OBJECT_HEADER_LEN,
        payload.len(),
    )?;
    let chunk_count = u32_at(order, payload, at) as usize;

    let capacity = chunk_count.min((payload.len() - cursor) / CHUNK_HEADER_LEN);
    let mut chunks: Vec<Chunk> = Vec::with_capacity(capacity);

    for _ in 0..chunk_count {
        let body = end_of(
            "chunk header",
            object,
            cursor,
            CHUNK_HEADER_LEN,
            payload.len(),
        )?;

        let kind = u32_at(order, payload, cursor);
        let declared = u16_at(order, payload, cursor + 4);
        let count = usize::from(u16_at(order, payload, cursor + 6));

        let stride = chunk_stride(kind).ok_or(Error::UnknownChunk { object, kind })?;
        // Checking the unused declared stride makes a drifting walk fail here
        // rather than decode plausible garbage.
        if usize::from(declared) != stride {
            return Err(Error::StrideMismatch {
                object,
                kind,
                declared,
                expected: stride,
            });
        }
        if chunks.iter().any(|c| c.kind == kind) {
            return Err(Error::DuplicateChunk { object, kind });
        }

        // count is a u16 and stride at most 12: no overflow, chunk under 800 KiB.
        let end = end_of("chunk payload", object, body, count * stride, payload.len())?;

        chunks.push(Chunk {
            kind,
            stride: declared,
            count,
            payload: body..end,
        });
        cursor = end;
    }

    let find = |kind: u32| chunks.iter().find(|c| c.kind == kind);

    let vertices = find(CHUNK_VERTICES).map_or_else(Vec::new, |chunk| {
        let body = &payload[chunk.payload.clone()];
        body.as_chunks::<VERTEX_STRIDE>()
            .0
            .iter()
            .map(|v| {
                [
                    f32_at(order, v, 0),
                    f32_at(order, v, 4),
                    f32_at(order, v, 8),
                ]
            })
            .collect()
    });

    let triangles: Vec<[u16; 3]> = find(CHUNK_TRIANGLES).map_or_else(Vec::new, |chunk| {
        let body = &payload[chunk.payload.clone()];
        body.as_chunks::<TRIANGLE_STRIDE>()
            .0
            .iter()
            .map(|t| {
                [
                    u16_at(order, t, 0),
                    u16_at(order, t, 2),
                    u16_at(order, t, 4),
                ]
            })
            .collect()
    });

    // One that names no vertex is refused here rather than found by a consumer.
    for (triangle, indices) in triangles.iter().enumerate() {
        for &index in indices {
            if usize::from(index) >= vertices.len() {
                return Err(Error::BadTriangleIndex {
                    object,
                    triangle,
                    index,
                    vertex_count: vertices.len(),
                });
            }
        }
    }

    let vertex_scalars = match find(CHUNK_VERTEX_SCALARS) {
        Some(chunk) => {
            // The array is indexed by vertex, so its length is the vertex count
            // (true on 18,745 of 18,745 shipped objects).
            if chunk.count != vertices.len() {
                return Err(Error::ScalarCountMismatch {
                    object,
                    scalars: chunk.count,
                    vertices: vertices.len(),
                    triangles: triangles.len(),
                });
            }
            payload[chunk.payload.clone()]
                .as_chunks::<SCALAR_STRIDE>()
                .0
                .iter()
                .map(|s| f32_at(order, s, 0))
                .collect()
        }
        None => vec![DEFAULT_VERTEX_SCALAR; vertices.len()],
    };

    Ok((
        CollisionMesh {
            vertices,
            triangles,
            vertex_scalars,
            chunks,
        },
        cursor,
    ))
}

/// One collision node of a `.vex` scene.
#[derive(Debug, Clone, PartialEq)]
pub struct CollisionNode {
    /// Which collision class the node is.
    pub kind: SurfaceKind,
    /// Index of the node in [`vex::nodes`]'s output, so a caller can look up its
    /// world transform with [`vex::world_transforms`].
    pub node_index: usize,
    /// The node's Maya name, when it carries one.
    pub name: Option<String>,
    /// The decoded geometry.
    pub geometry: CollisionGeometry,
}

/// Finds and decodes every collision node in a `.vex` file.
///
/// The PSP `Data.wad` holds 130 such nodes and the PS2 `WADS2.WAD` 189, every
/// one decoding with its payload fully accounted for. Confidence **94**; see the
/// module docs.
///
/// [`Cage`](SurfaceKind::Cage) nodes are returned like any other (the original
/// skips them at load, so a consumer wanting that filters; the PS2 disc has 6).
/// # Errors
///
/// Propagates a `.vex` walk failure, a node payload that does not lie inside the
/// file, a payload the chunk walk does not account for
/// ([`Error::PayloadNotAccountedFor`]), and anything [`parse_chunks`] refuses.
pub fn from_vex(file: &[u8]) -> Result<Vec<CollisionNode>> {
    let mut out = Vec::new();
    // The table comes from the file's own version word, so a generation whose
    // collision IDs are unrecovered yields no nodes. Propagated, not swallowed:
    // a decode failure must read differently from a file with no collision
    // (`a_vex_walk_failure_is_propagated_rather_than_swallowed`).
    let classes = vex::classes_of(file)?;
    // From the file's own magic, not the platform.
    let order = vex::byte_order(file);

    for (node_index, node) in vex::nodes(file)?.into_iter().enumerate() {
        let Some(kind) = SurfaceKind::from_class_id(node.class_id, classes) else {
            continue;
        };
        let range = node.payload();
        let payload = file.get(range.clone()).ok_or(Error::OutOfBounds {
            what: "node payload",
            object: node_index,
            end: range.end,
            len: file.len(),
        })?;

        let geometry = parse_chunks(payload, order)?;
        // Enforced closure check: a non-collision class ID or a drifted walk does
        // not land on the 16-byte boundary.
        if geometry.padded_len() != payload.len() {
            return Err(Error::PayloadNotAccountedFor {
                node: node_index,
                consumed: geometry.encoded_len(),
                len: payload.len(),
            });
        }

        out.push(CollisionNode {
            kind,
            node_index,
            name: node.name,
            geometry,
        });
    }

    Ok(out)
}

/// End of a structure, or an error naming what did not fit.
fn end_of(what: &'static str, object: usize, at: usize, bytes: usize, len: usize) -> Result<usize> {
    let end = at.checked_add(bytes).ok_or(Error::OutOfBounds {
        what,
        object,
        end: usize::MAX,
        len,
    })?;
    if end > len {
        return Err(Error::OutOfBounds {
            what,
            object,
            end,
            len,
        });
    }
    Ok(end)
}

fn u16_at(order: ByteOrder, data: &[u8], at: usize) -> u16 {
    order.u16(data, at)
}

fn u32_at(order: ByteOrder, data: &[u8], at: usize) -> u32 {
    order.u32(data, at)
}

fn f32_at(order: ByteOrder, data: &[u8], at: usize) -> f32 {
    order.f32(data, at)
}

#[cfg(test)]
mod tests;
