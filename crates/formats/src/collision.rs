//! Collision geometry: an indexed triangle soup, separate from the render mesh.
//!
//! `CollisionNode_ParseChunks` (`0x08934a48`) reads a chunked payload holding one
//! triangle soup per collidable piece of the track. There is no BSP, no quadtree
//! and no heightfield, and the geometry is **not** the render mesh: reusing the
//! visual mesh for collision would be wrong.
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
//! Validated against **319 collision nodes across two platforms**: 130 in the
//! PSP disc's `Data.wad` and 189 in the PS2 disc's `WADS2.WAD`, holding 18,745
//! objects, 56,235 chunks, 602,086 vertices and 594,615 triangles. Four
//! arithmetic invariants come out exactly, with no exceptions anywhere:
//!
//! 1. **The walk closes.** The chunk walk consumes the node payload down to its
//!    16-byte alignment padding, which is always zero-filled:
//!    [`CollisionGeometry::padded_len`] equals the payload length on 319 of 319
//!    nodes, and 53 of them need no padding at all. A parse one field out cannot
//!    make that come out even, which is the argument that settled
//!    [`WO Track`](crate::track).
//! 2. **The chunk header's second field is the element stride.** It matches the
//!    type's stride on 56,235 of 56,235 chunks. The loader hard-codes strides and
//!    discards the field, which is why it was recorded as unknown; using it as a
//!    check makes a drifting walk fail on the very next chunk rather than decode
//!    plausible garbage. See [`Error::StrideMismatch`].
//! 3. **Chunk 3 is per-vertex**, on 18,745 of 18,745 objects. That was a
//!    prediction from `0x0881835c` indexing the array by a triangle's three
//!    vertex indices; a per-triangle array would have been off by three on nearly
//!    every object. See [`Error::ScalarCountMismatch`].
//! 4. **Every triangle index names a vertex**, across all 1.78 million indices.
//!
//! Confidence **94** for the layout: exact arithmetic agreement with shipped
//! data, corroborated on a second platform's build. Not higher because nothing
//! has been traced under an emulator; see
//! `docs/reverse-engineering/confidence-rubric.md`.
//!
//! Two observations that are not part of the layout but constrain what the data
//! means:
//!
//! - **Every vertex scalar in both builds is exactly `1.0`.** All 602,086 of
//!   them. The array is always present and always neutral, so whatever the field
//!   is for, no shipped track uses it.
//! - **The first word is `0xffff_ffff` on all 319 nodes**, so calling it a
//!   version is a reading, not an observation. Nothing distinguishes it from an
//!   unused slot.
//!
//! # The five node types are one class
//!
//! `Collision_RegisterNodeClasses` (`0x08934d44`) registers all five collision
//! [`.vex`](crate::vex) classes with the same vtable. The class ID selects only a
//! surface type and a friction constant, so **magstrips are not special
//! geometry**: a magstrip is ordinary floor with surface type 3. See
//! [`SurfaceKind`].
//!
//! `Cage Collision` was recorded as dead content that the loader skips, "or
//! another SKU". It is the other SKU: **the PS2 build ships 6 cage nodes and the
//! PSP build ships none**, which fits `Definition.xml`'s per-track
//! `collisionCageEnabled` attribute.
//!
//! Friction is `-1.0` for [`Floor`](SurfaceKind::Floor),
//! [`MagFloor`](SurfaceKind::MagFloor) and [`Reset`](SurfaceKind::Reset), and it
//! is a **sentinel, not a value**: contact combination averages the two sides but
//! forces zero if either is negative. It is therefore [`None`] here, and
//! [`combine_friction`] is the rule.
//!
//! This float at `collider+0x64` was recorded as *restitution* until
//! `Body_ResolveContact` (`0x0884e968`) was read: the resolver takes restitution
//! from `body+0x388` and uses the combined `collider+0x64` only to scale the
//! contact's tangential relative velocity. It is a **friction coefficient**. See
//! `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
//!
//! See `docs/formats/collision.md` for the evidence and
//! `docs/ghidra/functions/psp-pulse-usa/collision.md` for the addresses.

use std::fmt;
use std::ops::Range;

use crate::ByteOrder;
use crate::vex;

/// Bytes of payload header before the first object.
pub const HEADER_LEN: usize = 8;

/// Bytes of object header before its first chunk: one `u32` chunk count.
pub const OBJECT_HEADER_LEN: usize = 4;

/// Bytes of chunk header before its payload.
pub const CHUNK_HEADER_LEN: usize = 8;

/// Alignment of a collision node's payload inside a `.vex` file.
///
/// The chunk walk ends short of the node's declared `data_size` by up to 15
/// zero bytes, on 266 of the 319 nodes examined. The remainder is always
/// `padded_len() - encoded_len()`, never anything else, which is what makes the
/// closure check exact rather than approximate.
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
/// `CollisionMesh_AvgVertexScalar` (`0x0881835c`) returns `1.0` rather than zero
/// when the array is missing, so an absent chunk means "neutral". Every scalar in
/// both shipped builds is this value anyway.
pub const DEFAULT_VERTEX_SCALAR: f32 = 1.0;

/// Friction of a [`Wall`](SurfaceKind::Wall) surface, from `collider+0x64`.
pub const WALL_FRICTION: f32 = 0.05;

/// Bytes per element of a chunk type, or `None` if the type is not known.
///
/// Only the three types below appear in either build. A chunk also declares its
/// own stride at `+0x04`, and the two agree on every chunk examined, so this
/// table is a check rather than the only source.
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
    /// Fatal rather than skipped. The chunk does declare its own stride, so a
    /// fourth type could in principle be stepped over, but its *contents* would
    /// still be unknown and a decoder that quietly ignores data is worse than one
    /// that stops. This is the same choice
    /// [`vex::Error::UnsupportedVertexType`] makes.
    UnknownChunk {
        /// Index of the object it appeared in.
        object: usize,
        /// The type found.
        kind: u32,
    },
    /// A chunk whose declared stride is not the one its type implies.
    ///
    /// The two agree on all 56,235 chunks in both shipped builds, so a
    /// disagreement means the walk has drifted out of alignment, or that the type
    /// numbering is not what it appears to be. Either way the bytes after it
    /// cannot be trusted.
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
    /// Two chunks of the same type in one object.
    ///
    /// Which one wins would be a guess, and the loader keeps a single pointer per
    /// array, so this is refused rather than resolved.
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
    /// Carries the triangle count as well, because per-triangle was the rival
    /// reading: `scalars` equal to `triangles` would mean the array is indexed by
    /// triangle after all, on some asset the survey did not cover.
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
    /// Only from [`from_vex`], which knows the payload is a whole node and can
    /// therefore hold it to [`CollisionGeometry::padded_len`]. See
    /// [`PAYLOAD_ALIGN`].
    PayloadNotAccountedFor {
        /// Index of the node in the file's node list.
        node: usize,
        /// Bytes the walk consumed.
        consumed: usize,
        /// Bytes the node declares.
        len: usize,
    },
    /// The enclosing `.vex` file could not be walked.
    ///
    /// Only from [`from_vex`]; [`parse_chunks`] never produces it.
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
/// Pulse's five share one vtable and one parser. The class ID selects a surface
/// type and a friction constant, nothing else, which is why a magstrip is not
/// special geometry. [`TrackWall`](Self::TrackWall) is a sixth that only Wipeout
/// HD authors; it takes the same parser, and what selects its surface type on
/// HD is unread - see its own note.
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
    /// **Only the PS2 build ships any**: 6 nodes there, none at all on PSP. So it
    /// is not dead content in general, it is content for a platform whose loader
    /// still drops it, which fits the per-track `collisionCageEnabled` attribute
    /// in the plugin definitions.
    Cage,
    /// `collision_trackwall`: the barrier along the road, and **Wipeout HD
    /// only**.
    ///
    /// Exactly one node per circuit on all 16 of HD's, 3,036 to 4,974 triangles
    /// each, 97 % of them near-vertical, and co-extensive with the floor rather
    /// than with [`Wall`](Self::Wall)'s much larger scenery volume. Neither
    /// Pulse disc authors one. See
    /// [`vex::CLASS_TRACK_WALL_COLLISION`](crate::vex::CLASS_TRACK_WALL_COLLISION)
    /// for the four measurements and what they do and do not settle.
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
    /// class.
    ///
    /// The IDs come from `classes`, which a caller reads off the file with
    /// [`vex::classes_of`].
    ///
    /// **Only version 6's are recovered.** Pure PSP's `Data.wad` holds 171
    /// `.vex` files with 23,677 nodes and not one of Pulse's five IDs: its whole
    /// class-ID space is different (`0x6d` and `0x11e` dominate where Pulse has
    /// `0x6e` and `0x125`). Where Pure keeps collision geometry is an open
    /// question, so a version-4 table returns `None` for every kind and this
    /// finds nothing - which is the honest answer rather than a decode failure.
    #[must_use]
    pub fn from_class_id(class_id: u32, classes: vex::classes::Classes) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.class_id(classes) == Some(class_id))
    }

    /// The `.vex` class ID of this kind under `classes`, or `None` where that
    /// version's ID has not been recovered.
    ///
    /// `None` is not "this version has no such surface": it is "nobody has found
    /// its ID yet". The two are indistinguishable from here, and saying the
    /// weaker thing is the honest one.
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

    /// The node name from the class table, as the exporter writes it.
    ///
    /// [`TrackWall`](Self::TrackWall) is the one that does not come from a class
    /// table: it is the name HD's own circuits give the node, spelled as they
    /// spell it.
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
    /// `None` twice over, for opposite reasons. [`Cage`](Self::Cage) never
    /// reaches a collider object at all. [`TrackWall`](Self::TrackWall)
    /// certainly does, but the loader that would write its type is **HD's**, and
    /// nothing here has disassembled a PS3 binary - so the honest answer is that
    /// the number is unread, not that there isn't one. What the *physics* does
    /// with it is a separate decision, taken in `oag_gameplay::collision`.
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
    /// The file's value for floors is `-1.0`, and that is a sentinel rather than
    /// a coefficient: see [`combine_friction`]. Representing it as `None`
    /// makes the distinction impossible to lose, because a negative coefficient
    /// would *add* tangential velocity in the contact response instead of
    /// removing it.
    ///
    /// [`TrackWall`](Self::TrackWall) reports [`WALL_FRICTION`] because that is
    /// what the surface it behaves as carries, and the alternative - `None` -
    /// is not a neutral placeholder here: it means *frictionless*, a barrier a
    /// craft would grind along losing no speed at all. HD's own value is unread,
    /// and this is a stated assumption rather than a measurement.
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
/// if **either** is negative, so one frictionless surface makes the whole contact
/// frictionless. A floor against a wall is therefore `0.0`, not the average of
/// `-1.0` and `0.05`.
#[must_use]
pub fn combine_friction(a: Option<f32>, b: Option<f32>) -> f32 {
    match (a, b) {
        (Some(a), Some(b)) => (a + b) * 0.5,
        _ => 0.0,
    }
}

/// One chunk of an object, as found.
///
/// Kept alongside the decoded arrays because it is the evidence trail: it is what
/// makes [`CollisionMesh::encoded_len`] computable when a chunk is absent, and
/// what lets a test report the order chunks appear in. Every object in both
/// shipped builds holds exactly three, in the order 1, 3, 2 - the scalars
/// **before** the indices, which keeps the `f32` array 4-byte aligned even when
/// an odd triangle count leaves the index array ending on a half-word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Chunk type from `+0x00`.
    pub kind: u32,
    /// Element stride from `+0x04`.
    ///
    /// Redundant with [`chunk_stride`], and checked against it: the loader
    /// hard-codes strides and never reads this, but the exporter fills it in
    /// correctly on all 56,235 chunks examined.
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
/// Objects are small: the shipped average is 32 vertices and 32 triangles, and a
/// track's floor is 85 to 127 of them rather than one big soup. That is what the
/// per-mesh sweep and prune is sized for.
#[derive(Debug, Clone, PartialEq)]
pub struct CollisionMesh {
    /// Vertex positions, in the node's local space.
    pub vertices: Vec<[f32; 3]>,
    /// Triangles, as index triples into [`vertices`](Self::vertices). Every index
    /// is in range: [`parse_chunks`] refuses a payload where one is not.
    pub triangles: Vec<[u16; 3]>,
    /// One scalar per vertex, [`DEFAULT_VERTEX_SCALAR`] when the chunk is absent.
    ///
    /// **What it means is not known, and no shipped track uses it.** Every one of
    /// the 602,086 scalars in the PSP and PS2 builds is exactly `1.0`. It reaches
    /// the raycast hit result and accumulates into ship state, so it is wired up;
    /// "grip, friction or roughness" is a guess at confidence 40 and it is
    /// deliberately not named.
    pub vertex_scalars: Vec<f32>,
    /// The chunks found, in the order they appeared.
    pub chunks: Vec<Chunk>,
}

impl CollisionMesh {
    /// Whether a [`CHUNK_VERTEX_SCALARS`] chunk was present.
    ///
    /// [`vertex_scalars`](Self::vertex_scalars) is filled either way, so this is
    /// the only way to tell a file that stored all-`1.0` scalars - which is every
    /// shipped file - from one that stored none.
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

    /// The mean of a triangle's three vertex scalars.
    ///
    /// What `CollisionMesh_AvgVertexScalar` (`0x0881835c`) computes, and the
    /// value the raycast hit result carries. Comes out
    /// [`DEFAULT_VERTEX_SCALAR`] when the chunk was absent, which is the
    /// original's behaviour rather than an approximation of it - and, on shipped
    /// data, is what it comes out as anyway.
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
    /// The collider object carries a box at `+0x00`/`+0x10`; whether the loader
    /// reads it or derives it has not been established, so this is our own
    /// computation over the decoded vertices rather than a decoded field.
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
    /// The `u32` at `+0x00`, [`HEADER_WORD`] on every node examined.
    ///
    /// Read as a version by the loader, which then does nothing with it. Since
    /// all 319 shipped nodes carry the same all-ones value, nothing here
    /// distinguishes a version field from an unused slot.
    pub version: u32,
    /// The objects, in file order.
    pub meshes: Vec<CollisionMesh>,
}

impl CollisionGeometry {
    /// Bytes the decoded structure accounts for.
    ///
    /// The chunk walk's length, excluding the enclosing node's alignment padding.
    /// See [`padded_len`](Self::padded_len) for the check that matters.
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
    /// **Equal to the node payload's length on 319 of 319 shipped nodes**, on both
    /// the PSP and the PS2 disc. That is the check that turns this layout from a
    /// plausible reading into a settled one, and it is why [`from_vex`] refuses a
    /// node where it does not hold.
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
/// Standalone over raw bytes on purpose: where the payload came from is a
/// separate question from what it contains, and this half is testable without a
/// disc image. [`from_vex`] is the convenience layer over it.
///
/// # Errors
///
/// Refuses a payload too short for its header, any structure running past the
/// end, an unknown chunk type, a chunk whose declared stride contradicts its
/// type, a repeated chunk type, a triangle index naming no vertex, and a
/// vertex-scalar chunk that is not one entry per vertex.
///
/// Trailing bytes are **not** an error here, because a node payload legitimately
/// carries up to 15 bytes of alignment padding and this function does not know
/// its input is a whole node. Compare
/// [`CollisionGeometry::padded_len`] against the payload length to check, which
/// is what [`from_vex`] does.
pub fn parse_chunks(payload: &[u8], order: ByteOrder) -> Result<CollisionGeometry> {
    if payload.len() < HEADER_LEN {
        return Err(Error::TooShort { got: payload.len() });
    }

    let version = u32_at(order, payload, 0);
    let object_count = u32_at(order, payload, 4) as usize;

    // The count is a u32 out of a file, so it is not a capacity. An object is at
    // least its chunk count, which bounds how many can possibly be present.
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
        // The exporter writes the stride into the header and the loader ignores
        // it. Checking it is free, and it is what makes a drifting walk fail here
        // instead of decoding plausible garbage from the wrong offset.
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

        // count is a u16 and stride is at most 12, so this cannot overflow and
        // the largest chunk a file can ask for is under 800 KiB.
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
        body.chunks_exact(VERTEX_STRIDE)
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
        body.chunks_exact(TRIANGLE_STRIDE)
            .map(|t| {
                [
                    u16_at(order, t, 0),
                    u16_at(order, t, 2),
                    u16_at(order, t, 4),
                ]
            })
            .collect()
    });

    // Indices are what a narrowphase dereferences, so one that names no vertex
    // has to be refused here rather than discovered by a consumer.
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
            // The prediction this checks: the array is indexed by vertex, so its
            // length is the vertex count and nothing else. True on 18,745 of
            // 18,745 shipped objects.
            if chunk.count != vertices.len() {
                return Err(Error::ScalarCountMismatch {
                    object,
                    scalars: chunk.count,
                    vertices: vertices.len(),
                    triangles: triangles.len(),
                });
            }
            payload[chunk.payload.clone()]
                .chunks_exact(SCALAR_STRIDE)
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
/// Collision geometry is stored as `.vex` nodes in the track model, alongside the
/// render meshes it is deliberately not made of. The PSP disc's `Data.wad` holds
/// 130 such nodes and the PS2 disc's `WADS2.WAD` holds 189, and every one of them
/// decodes with its payload fully accounted for. Confidence **94**; see the module
/// docs for the invariants and why it is not higher.
///
/// [`Cage`](SurfaceKind::Cage) nodes are returned like any other. The original
/// skips them at load, so a consumer wanting that behaviour should filter:
/// "the geometry is in the file and the engine ignores it" is worth being able to
/// see, and on the PS2 disc there are 6 of them to see.
///
/// # Errors
///
/// Propagates a `.vex` walk failure, a node payload that does not lie inside the
/// file, a payload the chunk walk does not account for
/// ([`Error::PayloadNotAccountedFor`]), and anything [`parse_chunks`] refuses.
pub fn from_vex(file: &[u8]) -> Result<Vec<CollisionNode>> {
    let mut out = Vec::new();
    // The table comes from the file's own version word, so a generation whose
    // collision IDs are unrecovered yields no nodes rather than matching
    // version 6's IDs against a numbering that does not contain them.
    //
    // Propagated, not swallowed: a file too short to hold a header, or a version
    // with no table at all, is a decode failure and reads differently from a
    // file that simply authors no collision. Returning an empty vector for both
    // is what `a_vex_walk_failure_is_propagated_rather_than_swallowed` exists to
    // stop.
    let classes = vex::classes_of(file)?;
    // From the file's own magic, not from any platform: see `vex::byte_order`.
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
        // The closure check, enforced rather than merely available: a class ID
        // that is not really a collision node, or a walk that drifted, does not
        // land on the payload's 16-byte boundary.
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
