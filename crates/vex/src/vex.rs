//! The `.vex` scene format: models, tracks, everything 3D.
//!
//! A Maya scene export. The file is a 16-byte header followed by a depth-first
//! pre-order node tree, with embedded textures appended after it.
//!
//! ```text
//! file header, 16 bytes:
//!   +0x00  u32   version, 6 in Pulse
//!   +0x04  u32   size of the node tree
//!   +0x08  u32   size of the embedded texture block
//!   +0x0c  char  "VEXX"
//!
//! node:
//!   +0x00  u32   class_id
//!   +0x04  u16   header_size
//!   +0x08  u32   data_size
//!   +0x0c  u16   child_count      immediate children, not descendants
//!   +0x0e  u16   unknown          zero on all but a few dozen nodes
//!   +0x10  char  name, NUL-terminated, when header_size >= 0x20
//!
//! the next node begins at offset + header_size + data_size
//! ```
//!
//! # `child_count` is 16 bits
//!
//! `+0x0e` is usually zero, so a `u32` read works on most nodes and corrupts
//! every depth after the 54 of `01_Track`'s 2,071 nodes that have something
//! there (child count 1,572,865). In a pre-order tree the immediate child
//! counts sum to one less than the node count: true as a `u16` on every file
//! tried, 111 million as a `u32`.
//!
//! See `docs/formats/vex.md` for the class-ID table and the evidence.
//!
//! # Geometry is pre-batched GE draw calls
//!
//! Meshes are batches of PSP Graphics Engine draw calls, **never indexed**,
//! with vertices inline after each batch header. Positions are three `s16`
//! scaled by a per-batch `f32`:
//!
//! ```text
//! position = s16 / 32768.0 * scale
//! ```
//!
//! Missing the scale makes every model a uniform wrong size, which looks like
//! a units problem.

use std::fmt;

use oag_formats::ByteOrder;

/// Bytes of file header before the node tree.
pub const FILE_HEADER_LEN: usize = 16;

/// File magic, at `+0x0c` of the header, as the PSP and PS2 spell it.
pub const MAGIC: &[u8; 4] = b"VEXX";

/// The same magic on the PS3, written by the same exporter on a big-endian
/// host. It is the whole of how a file declares its [`ByteOrder`]; see
/// [`byte_order`].
pub const MAGIC_BE: &[u8; 4] = b"XXEV";

pub mod class_names;
pub mod classes;
pub mod matrix;

pub use matrix::{
    Anchored, IDENTITY, anchor_world, anim_anchors, class_world_transforms, multiply, transform,
    transform_point, world_transforms, world_transforms_at,
};

/// Class ID of a `Mesh` node (version 6 only; ask [`classes::for_version`]).
pub const CLASS_MESH: u32 = 0x125;

/// Class ID of a `Texture` node (version 6 only; ask [`classes::for_version`]).
pub const CLASS_TEXTURE: u32 = 0x3c1;

/// The collision and absorb class IDs, defined in [`classes`] beside the tables
/// that select them and re-exported here so every `CLASS_*` name is in one place.
pub use classes::{
    CLASS_ABSORB, CLASS_CAGE_COLLISION, CLASS_FLOOR_COLLISION, CLASS_MAG_FLOOR_COLLISION,
    CLASS_RESET_COLLISION, CLASS_TRACK_WALL_COLLISION, CLASS_WALL_COLLISION,
};

/// Class ID of a `WO Track` node: the AI spline graph, decoded by
/// [`track::parse`](crate::track::parse).
pub const CLASS_WO_TRACK: u32 = 0x3bb;

/// Class ID of a `Start Position` node, decoded by
/// [`track::start_position`](crate::track::start_position). Exactly one per
/// track file, on all 40 of the PSP disc's.
pub const CLASS_START_POSITION: u32 = 0x3bc;

/// Class ID of a `section` node: the authored visibility partition, decoded by
/// [`pvs::TrackPvs`](crate::pvs::TrackPvs).
///
/// **Not the lap structure**: a `section` carries a PVS bitmask and a bounding
/// box only; the track path is [`CLASS_WO_TRACK`].
pub const CLASS_SECTION: u32 = 0x3c9;

/// Class ID of a `Transform` node: a 4x4 matrix payload, or nothing for the
/// identity. 715 of `01_Track`'s 2,071 nodes.
pub const CLASS_TRANSFORM: u32 = 0x6e;

/// Class ID of an `Anim Transform` node: a transform whose translation,
/// rotation and scale are each a keyframe track.
///
/// 393 nodes over Pulse's twelve circuits, 474 meshes below them. Read through
/// the class's registration site (`0x0890009c`, passing `0x3c0` to
/// `Vex_RegisterClass`); see [`anim_transform`](crate::vex::anim_transform) and
/// `docs/ghidra/functions/psp-pulse-usa/anim-transform.md`.
pub const CLASS_ANIM_TRANSFORM: u32 = 0x3c0;

/// Class ID of a `LodGroup` node: an authored level of detail.
///
/// Its `child_count` (`u32` at `+0x50`) and, when `2`, a switch-distance `f32`
/// after it are authoring data the PSP binary never reads; see
/// `docs/formats/vex.md`, "`LodGroup`: authored, but never switched at runtime".
pub const CLASS_LOD_GROUP: u32 = 0x2ee;

/// Class ID of an `Engine Flare` node: the ship's exhaust. Its handler is in
/// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`: an additive camera-facing
/// quad at the nozzle, plus the `~ENGINE` sound and the `<Team>boost.vex` model.
pub const CLASS_ENGINE_FLARE: u32 = 0x3bf;

/// Class ID of a `ParticleSystem` node; the `.pob` it names parses in `oag-pob`.
pub const CLASS_PARTICLE_SYSTEM: u32 = 0x3c4;

/// Class ID of a `Trail` node: a position-history ribbon, unlike
/// [`CLASS_ENGINE_FLARE`]. Ring buffer and draw are in `exhaust.md`.
pub const CLASS_TRAIL: u32 = 0x3c8;

/// Class ID of an `exitglow` node.
pub const CLASS_EXITGLOW: u32 = 0x3e4;

/// Class ID of an `engine_fire` node.
pub const CLASS_ENGINE_FIRE: u32 = 0x3e5;

/// Class ID of a `Ship Collision Fx` node.
pub const CLASS_SHIP_COLLISION_FX: u32 = 0x3d0;

/// Class ID of a `Ship Muzzle` node.
pub const CLASS_SHIP_MUZZLE: u32 = 0x3e2;

/// Class ID of an `Airbrake` node.
pub const CLASS_AIRBRAKE: u32 = 0x3c5;

/// Class ID of a `Skycube` node: the track's sky.
///
/// **Its payload is a [`CLASS_MESH`] payload**, so [`mesh_materials`] and
/// [`mesh_batches`] decode it unchanged (same header words, bounding-box pair at
/// `+0x10`/`+0x20`, stride-`0x14` material array at `+0x30`), checked against
/// all 40 sky nodes by `crates/vex/tests/skycube_ground_truth.rs`.
///
/// Every track authors exactly one, parented to the world node, geometry
/// inline. Materials run 1, 5 or 6: six a full cube, five the cube without the
/// unseen face, one the Zone variants. `Data\Defaults\Skycube.vex` is a
/// **version-4** file in a version-6 archive, so no track can reference it.
pub const CLASS_SKYCUBE: u32 = 0x3c6;

/// Class ID of a `fogCube` node: a track's fog volume and parameters.
///
/// 128 bytes: a 64-byte row-major 4x4 as [`CLASS_TRANSFORM`] uses, two
/// `{rgb, 0, near, far}` sets of six floats, then eight bytes not yet read. 36
/// of the 40 track files author one, so a loader must handle its absence.
pub const CLASS_FOGCUBE: u32 = 0x3d3;

/// Class ID of an `AmbientLight` node: a flat colour added everywhere.
///
/// A 16-byte `{r, g, b, intensity}` payload of four `f32`, decoded by
/// [`lighting::ambient_lights`](crate::lighting::ambient_lights). Placement is
/// the node's transform chain, as for a pad. See `docs/formats/lighting.md`.
pub const CLASS_AMBIENT_LIGHT: u32 = 0x12c;

/// Class ID of a `DirectionalLight` node: a parallel light with no position.
/// Same payload shape as [`CLASS_AMBIENT_LIGHT`], decoded by
/// [`lighting::directional_lights`](crate::lighting::directional_lights).
pub const CLASS_DIRECTIONAL_LIGHT: u32 = 0x131;

/// Class ID of a `PointLight` node: a light that falls off with distance.
///
/// 32 bytes: `{r, g, b, range}` as four `f32`, then four `u32` observed as
/// `{1, 0, 0, 0}` on every sample, otherwise undecoded. Decoded by
/// [`lighting::point_lights`](crate::lighting::point_lights).
pub const CLASS_POINT_LIGHT: u32 = 0x132;

/// Class ID of a `Dynamic Point Light` node: presumably a moving light, per the
/// class name.
///
/// **No parser here.** Zero authored on the 40 PSP track files; the full-disc
/// sweep in `crates/vex/tests/lighting_ground_truth.rs` rechecks `Data.wad`.
pub const CLASS_DYNAMIC_POINT_LIGHT: u32 = 0x3c2;

/// Class ID of a `Speedup Pad` node: a boost pad on the track surface.
///
/// **Its payload is a [`CLASS_MESH`] payload**: the bind handler (`0x089264f4`)
/// calls the `Mesh` bind (`0x0890e998`) first, then reads its own fields. The
/// bounding-box pair at `+0x10`/`+0x20` is both the mesh bounds and the trigger
/// volume. Confidence 85; see [`pads`](crate::pads) and `docs/formats/pads.md`.
///
/// `01_Track` authors nine, each an instance of one shared payload under a
/// different [`CLASS_TRANSFORM`] parent, so placement is in the transform chain
/// and the volume is in local space.
pub const CLASS_SPEEDUP_PAD: u32 = 0x3bd;

/// Class ID of a `Weapon Pad` node: a pickup pad. Shares [`CLASS_SPEEDUP_PAD`]'s
/// base vtable, so the payload decodes the same way; nothing consumes one yet.
pub const CLASS_WEAPON_PAD: u32 = 0x3be;

/// The `.vex` class-ID to name table, read whole out of a shipped executable
/// (866 records; see [`class_names`]).
use class_names::CLASS_NAMES;

/// The shipped name for a class ID, or `None` if the ID is not in
/// [`CLASS_NAMES`] (not in the part of the table that was read, not "invalid").
#[must_use]
pub fn class_name(class_id: u32) -> Option<&'static str> {
    CLASS_NAMES
        .iter()
        .find(|(id, _)| *id == class_id)
        .map(|(_, name)| *name)
}

/// Every node of one class, in tree order.
pub fn nodes_by_class(nodes: &[Node], class_id: u32) -> impl Iterator<Item = &Node> {
    nodes.iter().filter(move |n| n.class_id == class_id)
}

/// Divisor for `s16` positions and the `f32` scale. The game's offline tool uses
/// 32767; 32768 matches the hardware and the 0.003% difference is irrelevant.
const POSITION_DIVISOR: f32 = 32768.0;

/// Something wrong with a `.vex` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the file header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// A format version this project has no class table for.
    ///
    /// An error rather than a fall back to version 6: the numberings share no
    /// id, so the wrong table finds nothing and looks like an empty file.
    UnknownVersion {
        /// The version word the file declares.
        version: u32,
    },
    /// A structure points outside the file.
    OutOfBounds {
        /// What was being read.
        what: &'static str,
        /// Where it would end.
        end: usize,
        /// Size of the file.
        len: usize,
    },
    /// A vertex type this build does not handle: an unexplored asset class or a
    /// decoding error, either worth hearing about loudly.
    UnsupportedVertexType {
        /// The value found.
        vertex_type: u16,
    },
    /// A PS2 batch's VIF command stream did not walk.
    Vif {
        /// Offset of the batch header in the mesh payload.
        at: usize,
        /// What the walker said. Boxed to keep this enum small.
        error: Box<crate::vif::Error>,
    },
    /// A PS2 batch's packet framing did not add up.
    ///
    /// Separate from [`Self::Vif`]: these checks decide whether the packet was
    /// read as the right *kind* of thing (three lengths that must close, the
    /// attribute set agreeing with the vertex type).
    Packet {
        /// Which check failed.
        what: &'static str,
        /// Offset of the batch header in the mesh payload.
        at: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { got } => {
                write!(f, "need at least {FILE_HEADER_LEN} bytes, got {got}")
            }
            Self::UnknownVersion { version } => write!(
                f,
                "no class table for .vex version {version}; \
                 known versions are 6 (Pulse), 4 and 3"
            ),
            Self::OutOfBounds { what, end, len } => {
                write!(f, "{what} ends at {end} but the file is {len} bytes")
            }
            Self::UnsupportedVertexType { vertex_type } => {
                write!(f, "unsupported GU vertex type {vertex_type:#06x}")
            }
            Self::Vif { at, error } => write!(f, "batch at {at}: {error}"),
            Self::Packet { what, at } => write!(f, "batch at {at}: {what}"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// How each vertex component is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexLayout {
    /// Bytes per vertex.
    pub stride: usize,
    /// Offset of the three `s16` position components.
    pub position: usize,
    /// Offset of the two texture coordinates, and their format.
    pub texcoord: Option<(usize, TexcoordFormat)>,
    /// Offset of the three `s8` normal components.
    pub normal: Option<usize>,
    /// Offset of the colour, and how it is packed.
    pub colour: Option<(usize, ColourFormat)>,
}

/// How a vertex colour is packed: the GU's four colour formats. Ship models use
/// only [`Abgr8888`](ColourFormat::Abgr8888); **tracks** use
/// [`Abgr4444`](ColourFormat::Abgr4444).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourFormat {
    /// 16-bit `BGR5650`, no alpha.
    Bgr5650,
    /// 16-bit `ABGR5551`, one alpha bit.
    Abgr5551,
    /// 16-bit `ABGR4444`.
    Abgr4444,
    /// 32-bit `ABGR8888`.
    Abgr8888,
}

impl ColourFormat {
    /// Bytes occupied, which is also this component's alignment.
    #[must_use]
    pub fn size(self) -> usize {
        match self {
            Self::Bgr5650 | Self::Abgr5551 | Self::Abgr4444 => 2,
            Self::Abgr8888 => 4,
        }
    }

    /// Expands to RGBA8888.
    ///
    /// The 16-bit formats widen by **bit replication**, not shifting: 5 bits of
    /// `0x1f` must become `0xff`, and `0x1f << 3` gives `0xf8` (dark surfaces).
    #[must_use]
    pub fn to_rgba(self, raw: u32) -> [u8; 4] {
        let expand = |value: u32, bits: u32| -> u8 {
            let max = (1u32 << bits) - 1;
            if max == 0 {
                return 255;
            }
            ((value * 255 + max / 2) / max) as u8
        };
        match self {
            Self::Bgr5650 => [
                expand(raw & 0x1f, 5),
                expand((raw >> 5) & 0x3f, 6),
                expand((raw >> 11) & 0x1f, 5),
                255,
            ],
            Self::Abgr5551 => [
                expand(raw & 0x1f, 5),
                expand((raw >> 5) & 0x1f, 5),
                expand((raw >> 10) & 0x1f, 5),
                expand((raw >> 15) & 1, 1),
            ],
            Self::Abgr4444 => [
                expand(raw & 0xf, 4),
                expand((raw >> 4) & 0xf, 4),
                expand((raw >> 8) & 0xf, 4),
                expand((raw >> 12) & 0xf, 4),
            ],
            Self::Abgr8888 => [
                (raw & 0xff) as u8,
                ((raw >> 8) & 0xff) as u8,
                ((raw >> 16) & 0xff) as u8,
                ((raw >> 24) & 0xff) as u8,
            ],
        }
    }
}

/// How texture coordinates are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexcoordFormat {
    /// Two `u8`, divided by 128.
    U8,
    /// Two `f32`.
    F32,
}

impl VertexLayout {
    /// Derives the layout from a GU vertex type.
    ///
    /// Mirrors the game's own stride calculator rather than the general PSP
    /// rule; the two agree on every reachable combination, but a disagreement
    /// here then shows as an error rather than silently shifted vertices.
    pub fn from_vertex_type(vertex_type: u16) -> Result<Self> {
        // Position is always three s16: the game hard-codes `+ 6`.
        if vertex_type & 0x0180 != 0x0100 {
            return Err(Error::UnsupportedVertexType { vertex_type });
        }
        // Weights, indices, morphs and transform-2D are never handled.
        if vertex_type & 0xFE00 != 0 {
            return Err(Error::UnsupportedVertexType { vertex_type });
        }

        let mut offset = 0usize;
        let mut align = 2usize;

        let texcoord = match vertex_type & 3 {
            0 => None,
            1 => {
                let at = offset;
                offset += 2;
                Some((at, TexcoordFormat::U8))
            }
            3 => {
                let at = offset;
                offset += 8;
                align = 4;
                Some((at, TexcoordFormat::F32))
            }
            // u16 texcoords fall through the game's branch without advancing the
            // offset (pruned case or latent bug): refuse rather than guess.
            _ => return Err(Error::UnsupportedVertexType { vertex_type }),
        };

        // GU colour format; 1 to 3 are undefined by the hardware.
        let colour = match (vertex_type >> 2) & 7 {
            0 => None,
            4 => Some(ColourFormat::Bgr5650),
            5 => Some(ColourFormat::Abgr5551),
            6 => Some(ColourFormat::Abgr4444),
            7 => Some(ColourFormat::Abgr8888),
            _ => return Err(Error::UnsupportedVertexType { vertex_type }),
        }
        .map(|format| {
            let size = format.size();
            offset = offset.next_multiple_of(size);
            align = align.max(size);
            let at = offset;
            offset += size;
            (at, format)
        });

        let normal = if vertex_type & 0x60 == 0x20 {
            offset = offset.next_multiple_of(2);
            let at = offset;
            offset += 3;
            Some(at)
        } else if vertex_type & 0x60 != 0 {
            return Err(Error::UnsupportedVertexType { vertex_type });
        } else {
            None
        };

        offset = offset.next_multiple_of(2);
        let position = offset;
        let stride = (offset + 6).next_multiple_of(align);

        Ok(Self {
            stride,
            position,
            texcoord,
            normal,
            colour,
        })
    }
}

/// One decoded vertex, in model units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Position in model units, scale already applied.
    pub position: [f32; 3],
    /// Unit normal, if the format carries one.
    pub normal: Option<[f32; 3]>,
    /// Texture coordinates, if the format carries them.
    pub texcoord: Option<[f32; 2]>,
    /// Vertex colour as RGBA, if the format carries one.
    pub colour: Option<[u8; 4]>,
}

/// One batch of draw calls.
#[derive(Debug, Clone)]
pub struct Batch {
    /// Render-pass selection bits.
    pub pass_mask: u16,
    /// Index into the mesh's material array.
    pub material_index: u8,
    /// GU primitive type. 3 is triangles, 4 is a triangle strip.
    pub primitive_type: u8,
    /// The raw GU vertex type.
    pub vertex_type: u16,
    /// Per-batch position scale.
    pub scale: f32,
    /// Decoded vertices.
    pub vertices: Vec<Vertex>,
    /// The vertex count the batch header declares, at `+0x04`.
    ///
    /// **Not always `vertices.len()`.** On PSP they agree except where a batch
    /// uses its alternate block, counted at `+0x06`. On PS2 it is the count of
    /// the draw the batch *describes*, while `vertices` holds what its VIF
    /// packet unpacks: a strip split across chunks unpacks two extra per
    /// boundary. See [`vif_vertices`].
    pub declared_vertex_count: u16,
    /// The batch's bounding box in model units.
    ///
    /// Every decoded vertex must fall inside it: the standing check on layout
    /// and scale. PSP: `s16` at `+0x18` and `+0x20` in vertex space, scaled like
    /// positions. PS2: `f32` at `+0x20` and `+0x30`, already in position space.
    pub bounds: ([f32; 3], [f32; 3]),
    /// Byte 3 of the on-disk batch header, the same on PSP and PS2. Bit `0x40`
    /// selects the alternate block (see
    /// [`declared_vertex_count`](Self::declared_vertex_count)); bit `0x10` is
    /// decoded by [`is_additive_blend`](Self::is_additive_blend), confirmed on
    /// the PSP draw path only. The rest are undecoded.
    pub header_flags: u8,
}

/// GU primitive type for a triangle list.
pub const PRIM_TRIANGLES: u8 = 3;

/// GU primitive type for a triangle strip.
pub const PRIM_TRIANGLE_STRIP: u8 = 4;

impl Batch {
    /// Expands the batch into triangles, as index triples into
    /// [`vertices`](Self::vertices).
    ///
    /// Strips alternate winding every triangle; getting that wrong points every
    /// other face inwards. Empty for primitive types other than triangles and
    /// strips; none other appear in the models examined.
    #[must_use]
    pub fn triangles(&self) -> Vec<[u32; 3]> {
        let count = self.vertices.len();
        match self.primitive_type {
            PRIM_TRIANGLES => (0..count / 3)
                .map(|t| {
                    let i = (t * 3) as u32;
                    [i, i + 1, i + 2]
                })
                .collect(),
            PRIM_TRIANGLE_STRIP => (0..count.saturating_sub(2))
                .map(|i| {
                    let i = i as u32;
                    // Odd triangles flip winding.
                    if i.is_multiple_of(2) {
                        [i, i + 1, i + 2]
                    } else {
                        [i + 1, i, i + 2]
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// A node in the scene tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Class ID; see the table in `docs/formats/vex.md`.
    pub class_id: u32,
    /// Offset of the node header from the start of the file.
    pub offset: usize,
    /// Bytes of node header.
    pub header_size: usize,
    /// Bytes of node payload.
    pub data_size: usize,
    /// Number of immediate children.
    pub child_count: usize,
    /// Byte length of the header's named-attribute list ([`node_attributes`]
    /// walks it), `0` where the node carries none. `AnimTransform_Bind`
    /// (`0x088fe5a4`) tests exactly this short before walking it. The name stays
    /// `unk_0x0e` until a rename gets its own commit.
    pub unk_0x0e: u16,
    /// Node name from the header, when it carries one.
    pub name: Option<String>,
    /// Depth in the tree, zero for the root.
    pub depth: usize,
    /// Index of this node's parent in the vector [`nodes`] returned, `None` for
    /// the root. A mesh's world matrix multiplies the [`Transform`](CLASS_TRANSFORM)
    /// matrices up this chain.
    pub parent: Option<usize>,
}

impl Node {
    /// Range of the node's payload within the file.
    #[must_use]
    pub fn payload(&self) -> std::ops::Range<usize> {
        let start = self.offset + self.header_size;
        start..start + self.data_size
    }
}

/// Little-endian reads, for the parts of this module that are PSP and PS2 only.
///
/// The file header and node walk go through [`byte_order`] instead, because a
/// PS3 `.vex` has those. It has no geometry: a PS3 `Mesh` node is a bounding-box
/// pair and a reference into a `.rcsmodel`, so every batch, vertex and
/// embedded-texture decoder below runs on little-endian files.
///
/// **That is a claim about these decoders, not the format**: [`anim_transform`]
/// was once written little-endian on that strength, and HD's 5,518 nodes of the
/// class all decoded to nothing.
mod le {
    use oag_formats::ByteOrder;

    pub fn u16_at(data: &[u8], at: usize) -> u16 {
        ByteOrder::Little.u16(data, at)
    }

    pub fn u32_at(data: &[u8], at: usize) -> u32 {
        ByteOrder::Little.u32(data, at)
    }

    pub fn f32_at(data: &[u8], at: usize) -> f32 {
        ByteOrder::Little.f32(data, at)
    }
}

use le::{f32_at, u16_at, u32_at};

/// Which way round a file's words are, from its own magic: `VEXX` on the PSP and
/// PS2, `XXEV` on the PS3 (the same exporter on a big-endian host). A file with
/// neither reads as [`ByteOrder::Little`]; [`has_magic`] is the "is this a
/// `.vex`" check.
#[must_use]
pub fn byte_order(data: &[u8]) -> ByteOrder {
    match data.get(12..16) {
        Some(m) if m == MAGIC_BE => ByteOrder::Big,
        _ => ByteOrder::Little,
    }
}

/// Format version, from the file header.
pub fn version(data: &[u8]) -> Result<u32> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    Ok(byte_order(data).u32(data, 0))
}

/// The class table for a file, read from its own version word.
///
/// What a decoder should call: [`classes::for_version`] with a version obtained
/// elsewhere risks pairing a table with a file it does not describe.
///
/// # Errors
///
/// [`Error::TooShort`] for a file with no header, [`Error::UnknownVersion`] for
/// a generation this project has never seen (never silently read as version 6).
pub fn classes_of(data: &[u8]) -> Result<classes::Classes> {
    let version = version(data)?;
    classes::for_version(version).ok_or(Error::UnknownVersion { version })
}

/// Whether the file carries the `VEXX` magic, in either spelling.
///
/// The magic sits at `+0x0c`, so a naive signature check misses it. `XXEV` is
/// the same magic on a big-endian host; [`byte_order`] says which.
#[must_use]
pub fn has_magic(data: &[u8]) -> bool {
    matches!(data.get(12..16), Some(m) if m == MAGIC || m == MAGIC_BE)
}

/// Byte length of the node tree, from the file header.
pub fn tree_len(data: &[u8]) -> Result<usize> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    Ok(byte_order(data).u32(data, 4) as usize)
}

/// Byte length of the embedded texture block, from the file header.
pub fn texture_len(data: &[u8]) -> Result<usize> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    Ok(byte_order(data).u32(data, 8) as usize)
}

/// Reads a NUL-terminated name out of a node header.
fn name_at(data: &[u8], at: usize, header_size: usize) -> Option<String> {
    if header_size < 0x20 {
        return None;
    }
    let start = at + 0x10;
    let end = (at + header_size).min(data.len());
    let bytes = data.get(start..end)?;
    let text = bytes.split(|&b| b == 0).next()?;
    if text.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(text).into_owned())
}

/// Reads a NUL-terminated string out of a payload, at a fixed offset.
fn cstr_at(payload: &[u8], at: usize) -> Option<String> {
    let bytes = payload.get(at..)?;
    let text = bytes.split(|&b| b == 0).next()?;
    if text.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(text).into_owned())
}

/// Walks the node tree, depth-first.
///
/// Stops at the first structurally impossible node rather than erroring: the
/// tree is followed by embedded texture data with no terminator.
pub fn nodes(data: &[u8]) -> Result<Vec<Node>> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }

    let order = byte_order(data);
    let mut out: Vec<Node> = Vec::new();
    // Remaining child counts track depth; a parallel stack of indices gives parents.
    let mut remaining: Vec<usize> = Vec::new();
    let mut remaining_parent: Vec<usize> = Vec::new();
    let mut at = FILE_HEADER_LEN;

    // Stop at the declared end of the tree rather than detecting where nodes stop.
    let end = (FILE_HEADER_LEN + tree_len(data)?).min(data.len());

    while at + 16 <= end {
        // Retire finished subtrees before reading the depth: a completed parent is
        // no longer an ancestor of the next node.
        while remaining.last() == Some(&0) {
            remaining.pop();
            remaining_parent.pop();
        }

        let header_size = order.u16(data, at + 4) as usize;
        let node = Node {
            class_id: order.u32(data, at),
            offset: at,
            header_size,
            data_size: order.u32(data, at + 8) as usize,
            child_count: usize::from(order.u16(data, at + 12)),
            unk_0x0e: order.u16(data, at + 14),
            name: name_at(data, at, header_size),
            depth: remaining.len(),
            parent: remaining_parent.last().copied(),
        };

        // A header smaller than the fields read, or a node past the tree end.
        if node.header_size < 16 || node.payload().end > end {
            break;
        }

        let children = node.child_count;
        let next = node.payload().end;
        out.push(node);

        if let Some(last) = remaining.last_mut() {
            *last -= 1;
        }
        if children > 0 {
            remaining.push(children);
            remaining_parent.push(out.len() - 1);
        }

        // A zero-length node would loop forever.
        if next <= at {
            break;
        }
        at = next;
    }

    Ok(out)
}

mod batch_flags;
pub use batch_flags::BlendClass;

mod mesh_header;
pub use mesh_header::{
    LAYER_DEFAULT, LAYER_EARLY, LAYER_EXHAUST, LAYER_REFLECTION_FIRST, LAYER_SCENE, Material,
    mesh_first_vertex_type, mesh_layer, mesh_materials,
};

mod attributes;
pub use attributes::{node_attributes, node_string_attribute};

mod anim_transform;
pub use anim_transform::{
    AnimChannel, AnimTransform, DEFAULT_LOOP_SECONDS, ROTATION_IS_QUATERNION, TRANSLATION_IS_FLOAT,
    anim_transform, anim_transform_of, anim_transforms,
};

mod textures;
pub use textures::{
    EmbeddedTexture, texture_asset_path, texture_row_bytes, texture_row_stride, textures,
};

/// One keyframe track of a mesh's texture-transform block: key times paired
/// with `(u, v)` values.
///
/// Times are `u16` 60 Hz frames (the block's `+0x0c` is `1/60` on every block
/// read); values are `s16` pairs in 1/256 units. From `TexAnim_EvalKeyframes`
/// (`0x08927034`); see
/// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`, "The values gap
/// is closed", and `docs/formats/vex.md`, "The texture-transform keyframe block".
#[derive(Debug, Clone, PartialEq)]
pub struct TexTransformTrack {
    /// Key times, 60 Hz frames, ascending.
    pub times: Vec<u16>,
    /// `(u, v)` at each key, in 1/256 units.
    pub values: Vec<(i16, i16)>,
}

impl TexTransformTrack {
    /// Evaluates the track at `t` frames as the engine does: clamp outside the
    /// keys, lerp between. Returns `(u, v)` in texture units.
    #[must_use]
    pub fn sample(&self, t: f32) -> (f32, f32) {
        self.sample_with(t, false)
    }

    /// [`sample`](Self::sample), with the block's step flag applied.
    ///
    /// `TexAnim_EvalKeyframes` snaps to a key instead of interpolating when the
    /// flag is set. It matters: `16_Track`'s flicker sequences are authored as
    /// key *pairs* one frame apart (`(3, 4)`, `(7, 8)`), and lerping across the
    /// gaps turns a hard flicker into a slow slide.
    ///
    /// **Which key it snaps to is a choice, not a read**: the decompilation does
    /// not settle the direction and this holds the **preceding** key, as those
    /// pairs argue. Confidence 60 on the direction alone, per
    /// `docs/reverse-engineering/confidence-rubric.md`; the rest is read at
    /// instruction level.
    #[must_use]
    pub fn sample_with(&self, t: f32, step: bool) -> (f32, f32) {
        let Some((&first, &last)) = self.times.first().zip(self.times.last()) else {
            return (0.0, 0.0);
        };
        let scale = |(u, v): (i16, i16)| (f32::from(u) / 256.0, f32::from(v) / 256.0);
        if t < f32::from(first) {
            return scale(self.values[0]);
        }
        if t >= f32::from(last) {
            return scale(self.values[self.values.len() - 1]);
        }
        let i = self
            .times
            .iter()
            .position(|&key| t < f32::from(key))
            .unwrap_or(self.times.len() - 1);
        let (u0, v0) = scale(self.values[i - 1]);
        if step {
            return (u0, v0);
        }
        let (t0, t1) = (f32::from(self.times[i - 1]), f32::from(self.times[i]));
        let frac = (t - t0) / (t1 - t0);
        let (u1, v1) = scale(self.values[i]);
        (u0 + (u1 - u0) * frac, v0 + (v1 - v0) * frac)
    }

    /// The last key time in frames: the span a looping animation like the boost
    /// plume's loops over.
    #[must_use]
    pub fn period(&self) -> f32 {
        self.times.last().copied().map_or(0.0, f32::from)
    }
}

/// A mesh's authored texture-transform animation: the scale and offset
/// keyframe tracks from the block after the material array.
#[derive(Debug, Clone, PartialEq)]
pub struct TexTransform {
    /// The `TEXOFFSET` track.
    pub offset: TexTransformTrack,
    /// The `TEXSCALE` track. A single key `(256, 256)` is the common
    /// "constant 1.0" case.
    pub scale: TexTransformTrack,
    /// Seconds per key-time unit, from the block's `+0x0c`; `1/60` on every
    /// block read.
    pub seconds_per_key: f32,
    /// The authored loop period in seconds, from the block's `+0x2c`.
    ///
    /// **Not the last key time**: `16_Track`'s flicker tracks end at frame 12,
    /// 18 or 24 while all three author a 50-frame loop. Sibling meshes carry the
    /// same steps at different key times and share one period, which is how the
    /// original interleaves their phase.
    pub loop_seconds: f32,
    /// Bit 0 of the *word* at `+0x2c` (whose float is
    /// [`loop_seconds`](Self::loop_seconds)). Set means snap to the preceding key;
    /// see [`TexTransformTrack::sample_with`].
    pub step: bool,
}

impl TexTransform {
    /// Evaluates both tracks at `seconds` as `TexAnim_UpdateTransform`
    /// (`0x08927204`) does: wrap by [`loop_seconds`](Self::loop_seconds), divide
    /// by [`seconds_per_key`](Self::seconds_per_key), sample. Returns
    /// `(scale, offset)` in texture units.
    ///
    /// An empty track evaluates to the engine's not-found default (scale
    /// `(1.0, 1.0)`, offset `(0.0, 0.0)`), so a block authoring only one of the
    /// two leaves the other alone.
    #[must_use]
    pub fn sample(&self, seconds: f32) -> ([f32; 2], [f32; 2]) {
        let period = if self.loop_seconds > 0.0 {
            self.loop_seconds
        } else {
            f32::MAX
        };
        let unit = if self.seconds_per_key > 0.0 {
            self.seconds_per_key
        } else {
            1.0 / 60.0
        };
        let t = (seconds % period) / unit;
        let scale = if self.scale.times.is_empty() {
            [1.0, 1.0]
        } else {
            let (u, v) = self.scale.sample_with(t, self.step);
            [u, v]
        };
        let offset = if self.offset.times.is_empty() {
            [0.0, 0.0]
        } else {
            let (u, v) = self.offset.sample_with(t, self.step);
            [u, v]
        };
        (scale, offset)
    }
}

/// The texture-transform keyframe block of **material 0** of one mesh payload,
/// if it carries any keys. A mesh authors one `0x40`-byte block per material
/// ([`mesh_tex_transforms`] returns all); most animated meshes have one.
#[must_use]
pub fn mesh_tex_transform(payload: &[u8]) -> Option<TexTransform> {
    mesh_tex_transform_at(payload, 0)
}

/// Every material's texture-transform block, in material order.
///
/// The blocks follow the material array at `+0x30 + material_count * 0x14`,
/// `0x40` bytes each (the runtime's `mesh+0x60 + material_index * 0x40`,
/// relocated). `None` means no track: the identity transform.
#[must_use]
pub fn mesh_tex_transforms(payload: &[u8]) -> Vec<Option<TexTransform>> {
    if payload.len() < 0x30 {
        return Vec::new();
    }
    let material_count = usize::from(u16_at(payload, 2));
    (0..material_count)
        .map(|i| mesh_tex_transform_at(payload, i))
        .collect()
}

/// One material's block; layout in [`mesh_tex_transforms`], field map in
/// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`.
///
/// `None` when the material lacks the `& 0x10` flag, the block or its key data
/// runs past the payload, or both tracks are empty.
///
/// **The flag test is the engine's own gate**: `Mesh_UpdateTextureTransforms`
/// (`0x0890e160`) evaluates only materials carrying it. It matters because
/// non-mesh payloads (a `Skycube` is a Mesh payload) parse this far, and
/// arbitrary bytes often read as a plausible block. Both predicates agree on
/// everything measured, per `crates/render/tests/authored_uv_ground_truth.rs`.
fn mesh_tex_transform_at(payload: &[u8], material_index: usize) -> Option<TexTransform> {
    if payload.len() < 0x30 {
        return None;
    }
    let material_count = usize::from(u16_at(payload, 2));
    if material_index >= material_count {
        return None;
    }
    // The engine's gate.
    let material = 0x30 + material_index * 0x14;
    if material + 2 > payload.len() || u16_at(payload, material) & 0x10 == 0 {
        return None;
    }
    let base = 0x30 + material_count * 0x14;
    let block = base + material_index * 0x40;
    if block + 0x30 > payload.len() {
        return None;
    }
    let offset_count = usize::from(u16_at(payload, block));
    let scale_count = usize::from(u16_at(payload, block + 2));
    if offset_count == 0 && scale_count == 0 {
        return None;
    }
    // `times`/`values` are relative to the **start of the block array**, not
    // their own block: indistinguishable on a single-material mesh, wrong on
    // `16_Track`'s two-material hologram panels (material 1 resolves to real keys
    // off the array base and noise off its own block).
    let track = |count: usize, times_rel: usize, values_rel: usize| {
        let times_at = base + u32_at(payload, block + times_rel) as usize;
        let values_at = base + u32_at(payload, block + values_rel) as usize;
        if times_at + count * 2 > payload.len() || values_at + count * 4 > payload.len() {
            return None;
        }
        Some(TexTransformTrack {
            times: (0..count)
                .map(|i| u16_at(payload, times_at + i * 2))
                .collect(),
            values: (0..count)
                .map(|i| {
                    let at = values_at + i * 4;
                    (u16_at(payload, at) as i16, u16_at(payload, at + 2) as i16)
                })
                .collect(),
        })
    };
    let loop_word = u32_at(payload, block + 0x2c);
    Some(TexTransform {
        offset: track(offset_count, 0x04, 0x10)?,
        scale: track(scale_count, 0x08, 0x14)?,
        seconds_per_key: f32::from_bits(u32_at(payload, block + 0x0c)),
        loop_seconds: f32::from_bits(loop_word),
        step: loop_word & 1 != 0,
    })
}

/// Decodes the batches of one mesh payload (the node's data). `batch_list`
/// selects list A (`0`) or B (`1`); a batch belongs to a list while the matching
/// `pass_mask` bit is set.
pub fn mesh_batches(payload: &[u8], batch_list: u8) -> Result<Vec<Batch>> {
    if payload.len() < 0x30 {
        return Err(Error::TooShort { got: payload.len() });
    }

    // File offsets from the payload start (the loader relocates them to pointers).
    let list_offset = u32_at(payload, if batch_list == 0 { 4 } else { 8 }) as usize;
    let terminator = if batch_list == 0 { 1u16 } else { 2 };

    let mut out = Vec::new();
    let mut at = list_offset;

    while at + 0x40 <= payload.len() {
        let pass_mask = u16_at(payload, at);
        if pass_mask & terminator == 0 {
            break;
        }

        let flags = payload[at + 3];
        let header_size = if flags & 0x40 != 0 { 0x80 } else { 0x40 };

        let use_alternate = u16_at(payload, at + 6) != 0;
        let vertex_count = usize::from(u16_at(payload, at + if use_alternate { 6 } else { 4 }));
        let primitive_type = payload[at + if use_alternate { 9 } else { 8 }];
        let vertex_type = u16_at(payload, at + 0x0a);
        let payload_size = usize::from(u16_at(payload, at + 0x0c));
        let alternate_offset = usize::from(u16_at(payload, at + 0x0e));
        let scale = f32_at(payload, at + 0x10);

        // A PS2 batch's payload is a VIF packet, with float bounds already in
        // position space. See `is_vif_batch`.
        let (vertices, bounds) = if is_vif_batch(vertex_type) {
            if at + 0x3c > payload.len() {
                return Err(Error::OutOfBounds {
                    what: "batch bounds",
                    end: at + 0x3c,
                    len: payload.len(),
                });
            }
            let corner = |off: usize| {
                [
                    f32_at(payload, at + off),
                    f32_at(payload, at + off + 4),
                    f32_at(payload, at + off + 8),
                ]
            };
            let vertices = vif_vertices(
                payload,
                at + header_size,
                payload_size,
                vertex_type,
                primitive_type,
            )?;
            (vertices, (corner(0x20), corner(0x30)))
        } else {
            let layout = VertexLayout::from_vertex_type(vertex_type)?;

            let base = at + header_size + if use_alternate { alternate_offset } else { 0 };
            let end = base + vertex_count * layout.stride;
            if end > payload.len() {
                return Err(Error::OutOfBounds {
                    what: "batch vertices",
                    end,
                    len: payload.len(),
                });
            }

            let mut vertices = Vec::with_capacity(vertex_count);
            for i in 0..vertex_count {
                vertices.push(decode_vertex(
                    payload,
                    base + i * layout.stride,
                    &layout,
                    scale,
                ));
            }

            let corner = |off: usize| {
                let mut v = [0.0f32; 3];
                for (i, c) in v.iter_mut().enumerate() {
                    let raw = i16::from_le_bytes([
                        payload[at + off + i * 2],
                        payload[at + off + i * 2 + 1],
                    ]);
                    *c = f32::from(raw) / POSITION_DIVISOR * scale;
                }
                v
            };
            (vertices, (corner(0x18), corner(0x20)))
        };

        out.push(Batch {
            pass_mask,
            material_index: payload[at + 2],
            primitive_type,
            vertex_type,
            scale,
            vertices,
            declared_vertex_count: u16_at(payload, at + 4),
            bounds,
            header_flags: flags,
        });

        // `payload_size` covers the vertex data only.
        let step = header_size + payload_size;
        if step == 0 {
            break;
        }
        at += step;
    }

    Ok(out)
}

fn decode_vertex(data: &[u8], at: usize, layout: &VertexLayout, scale: f32) -> Vertex {
    let p = at + layout.position;
    let component = |o: usize| f32::from(i16::from_le_bytes([data[o], data[o + 1]]));

    let position = [
        component(p) / POSITION_DIVISOR * scale,
        component(p + 2) / POSITION_DIVISOR * scale,
        component(p + 4) / POSITION_DIVISOR * scale,
    ];

    let normal = layout.normal.map(|o| {
        let n = |i: usize| f32::from(data[o + at + i] as i8) / 128.0;
        [n(0), n(1), n(2)]
    });

    let texcoord = layout.texcoord.map(|(o, format)| match format {
        TexcoordFormat::U8 => [
            f32::from(data[at + o]) / 128.0,
            f32::from(data[at + o + 1]) / 128.0,
        ],
        TexcoordFormat::F32 => [f32_at(data, at + o), f32_at(data, at + o + 4)],
    });

    let colour = layout.colour.map(|(o, format)| {
        let raw = match format.size() {
            2 => u32::from(u16_at(data, at + o)),
            _ => u32_at(data, at + o),
        };
        format.to_rgba(raw)
    });

    Vertex {
        position,
        normal,
        texcoord,
        colour,
    }
}

/// Bits 7-8 of a vertex type, which say how a position is stored.
pub const POSITION_BITS: u16 = 0x0180;

/// Bits 7-8 set to `3`: positions are 32-bit floats. Unreachable on PSP (the
/// stride calculator hard-codes `+ 6` for three `s16`), universal on PS2.
pub const POSITION_F32: u16 = 0x0180;

/// Whether a batch's vertices are a PS2 VIF packet rather than a PSP vertex
/// array.
///
/// Not a heuristic: every PS2 batch observed declares float positions and no PSP
/// batch can. The packet's own framing confirms it; see [`vif_vertices`].
#[must_use]
pub fn is_vif_batch(vertex_type: u16) -> bool {
    vertex_type & POSITION_BITS == POSITION_F32
}

/// Bytes of framing before a PS2 batch's DMA packet.
const VIF_REGION_HEADER: usize = 16;

/// Bytes of DMA tag at the head of the packet: one quadword.
const VIF_TAG_LEN: usize = 16;

/// VU1 address of the position array, in quadwords.
const VU_POSITION: u16 = 4;

/// VU1 address of the colour array.
const VU_COLOUR: u16 = 5;

/// VU1 address of the texture-coordinate array.
const VU_TEXCOORD: u16 = 6;

/// VU1 address of the normal array.
const VU_NORMAL: u16 = 7;

/// The colour value the GS treats as full intensity.
///
/// PS2 vertex colours are 0 to 128, not 0 to 255. Every byte in the two models
/// measured is 0 to 127; reading them as 0-255 halves the brightness and looks
/// like a lighting problem. See `docs/formats/vex.md`.
const PS2_COLOUR_ONE: u16 = 128;

/// Decodes the vertices of one PS2 batch, whose payload is a VIF packet.
///
/// `at` is the batch header's offset in the mesh payload and `payload_size` its
/// declared vertex-data length, as [`mesh_batches`] reads them.
///
/// # What the packet looks like
///
/// ```text
/// +0x00  u32   bytes of DMA packet that follow this header
/// +0x04  u32   vertex type again, matching the batch header's
/// +0x08  u32   pass mask again
/// +0x0c  u32   zero
/// +0x10  DMA tag quadword: qwc in the low 16 bits, then two VIF command words
/// +0x20  VIF command stream
/// ```
///
/// The stream is a run of chunks, each ending in `MSCNT`, each holding one
/// `UNPACK` per attribute at a fixed VU address: position 4, colour 5, texture
/// coordinates 6, normals 7. A chunk is one draw, so a strip longer than VU1
/// memory is split by **repeating two vertices**, which is why the decoded count
/// exceeds the batch header's by two per extra chunk.
///
/// # Why the chunks are concatenated
///
/// Correct only because **every chunk but the last has an even vertex count**,
/// on all 89,302 strip batches of the PS2 disc: winding alternates per triangle,
/// so the next chunk starts at an even global index, and the two repeated
/// vertices become zero-area triangles at the seam. An odd non-final chunk would
/// wind everything after it backwards, so it is refused; the day one turns up,
/// this needs per-chunk triangle generation.
///
/// # Errors
///
/// [`Error::Vif`] if the command stream does not walk; [`Error::Packet`] if the
/// framing does not close, an attribute array is missing or misshapen, the
/// normal array disagrees with the vertex type, or a split strip's chunk is odd.
pub fn vif_vertices(
    payload: &[u8],
    at: usize,
    payload_size: usize,
    vertex_type: u16,
    primitive_type: u8,
) -> Result<Vec<Vertex>> {
    let base = at;
    let packet = |what: &'static str| Error::Packet { what, at: base };

    if base + VIF_REGION_HEADER + VIF_TAG_LEN > payload.len() {
        return Err(packet("region header runs past the payload"));
    }
    let size = u32_at(payload, base) as usize;
    // Three exact framing checks: region length + 16 is the declared payload, the
    // DMA tag's quadword count spans the packet, the vertex type repeats.
    if size + VIF_REGION_HEADER != payload_size {
        return Err(packet("packet length disagrees with the batch header"));
    }
    if u32_at(payload, base + 4) != u32::from(vertex_type) {
        return Err(packet("packet vertex type disagrees with the batch header"));
    }
    let qwc = (u32_at(payload, base + VIF_REGION_HEADER) & 0xffff) as usize;
    if VIF_TAG_LEN + qwc * 16 != size {
        return Err(packet("DMA tag quadword count disagrees with the packet"));
    }

    // The stream starts at the tag quadword's upper half (the first two VIF
    // commands) and runs to the end of the packet.
    let start = base + VIF_REGION_HEADER + 8;
    let end = base + VIF_REGION_HEADER + size;
    if end > payload.len() {
        return Err(packet("packet runs past the payload"));
    }
    let codes = crate::vif::walk(&payload[start..end]).map_err(|error| Error::Vif {
        at: base,
        error: Box::new(error),
    })?;

    let want_normal = vertex_type & 0x60 == 0x20;

    let mut out = Vec::new();
    let mut chunk: [Option<crate::vif::Unpack>; 4] = [None, None, None, None];
    // Chunk lengths so far, for the parity check.
    let mut lengths: Vec<usize> = Vec::new();
    let slot = |address: u16| match address {
        VU_POSITION => Some(0usize),
        VU_COLOUR => Some(1),
        VU_TEXCOORD => Some(2),
        VU_NORMAL => Some(3),
        _ => None,
    };

    for code in codes {
        match code {
            crate::vif::Code::Unpack(unpack) => {
                if let Some(index) = slot(unpack.address) {
                    chunk[index] = Some(unpack);
                }
            }
            // A microprogram call is one draw: the boundary of the gathered arrays.
            crate::vif::Code::Mscnt | crate::vif::Code::Mscal(_)
                if chunk.iter().any(Option::is_some) =>
            {
                let before = out.len();
                emit_vif_chunk(payload, start, &chunk, want_normal, &mut out, &packet)?;
                lengths.push(out.len() - before);
                chunk = [None, None, None, None];
            }
            _ => {}
        }
    }
    if chunk.iter().any(Option::is_some) {
        let before = out.len();
        emit_vif_chunk(payload, start, &chunk, want_normal, &mut out, &packet)?;
        lengths.push(out.len() - before);
    }

    if primitive_type == PRIM_TRIANGLE_STRIP
        && lengths.iter().rev().skip(1).any(|length| length % 2 != 0)
    {
        return Err(packet("a split strip has a chunk with an odd vertex count"));
    }

    Ok(out)
}

/// Turns one chunk's attribute arrays into vertices.
fn emit_vif_chunk(
    payload: &[u8],
    start: usize,
    chunk: &[Option<crate::vif::Unpack>; 4],
    want_normal: bool,
    out: &mut Vec<Vertex>,
    packet: &impl Fn(&'static str) -> Error,
) -> Result<()> {
    use crate::vif::Format;

    let position = chunk[0]
        .as_ref()
        .ok_or_else(|| packet("a chunk with no position array"))?;
    if position.format != Format::Bits32 || !(3..=4).contains(&position.components) {
        return Err(packet("position array is not three or four floats"));
    }
    // **Only the normal bit carries over from the vertex type.** Across all 1,038
    // PS2 `.vex` files a normal array is present exactly when bits 5-6 say `s8
    // normal`, so a disagreement means a misread packet. Colour and texcoords
    // differ: **every** chunk carries both (including 4,378 batches whose type
    // declares no colour), the coordinates always two floats. See
    // `docs/formats/vex.md`.
    if chunk[3].is_some() != want_normal {
        return Err(packet("normal array disagrees with the vertex type"));
    }

    let count = position.count;
    let check = |slot: &Option<crate::vif::Unpack>,
                 components: usize,
                 format: Format,
                 what: &'static str|
     -> Result<Option<usize>> {
        match slot {
            None => Ok(None),
            Some(unpack) => {
                if unpack.count != count
                    || unpack.components != components
                    || unpack.format != format
                {
                    return Err(packet(what));
                }
                Ok(Some(start + unpack.data.start))
            }
        }
    };

    let colour = check(
        &chunk[1],
        4,
        Format::Bits8,
        "colour array has the wrong shape",
    )?;
    let texcoord = check(
        &chunk[2],
        2,
        Format::Bits32,
        "texture-coordinate array has the wrong shape",
    )?;
    let normal = check(
        &chunk[3],
        3,
        Format::Bits32,
        "normal array has the wrong shape",
    )?;
    let positions = start + position.data.start;
    let position_stride = 4 * position.components;

    for i in 0..count {
        let p = positions + i * position_stride;
        out.push(Vertex {
            position: [
                f32_at(payload, p),
                f32_at(payload, p + 4),
                f32_at(payload, p + 8),
            ],
            normal: normal.map(|o| {
                let n = o + i * 12;
                [
                    f32_at(payload, n),
                    f32_at(payload, n + 4),
                    f32_at(payload, n + 8),
                ]
            }),
            texcoord: texcoord.map(|o| {
                let t = o + i * 8;
                [f32_at(payload, t), f32_at(payload, t + 4)]
            }),
            colour: colour.map(|o| {
                let c = o + i * 4;
                let widen = |v: u8| -> u8 {
                    u8::try_from(u16::from(v) * 255 / PS2_COLOUR_ONE).unwrap_or(u8::MAX)
                };
                [
                    widen(payload[c]),
                    widen(payload[c + 1]),
                    widen(payload[c + 2]),
                    widen(payload[c + 3]),
                ]
            }),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests;

/// Which bytes of a `.vex` the node walk reaches.
///
/// **Narrower than `oag_rcs::rcsmodel::coverage`**, which asks whether a *field*
/// went unread. This asks whether a *region* goes unvisited. It does not
/// descend into a payload: most node classes are undecoded on purpose (see
/// `docs/formats/vex.md`), and per-class coverage would report that decision as
/// a defect on every file.
#[must_use]
pub fn coverage(data: &[u8]) -> oag_formats::coverage::Coverage {
    let mut seen = oag_formats::coverage::Coverage::new(data.len());
    seen.claim(0, FILE_HEADER_LEN, "the file header");
    let Ok(nodes) = nodes(data) else {
        return seen;
    };
    for node in &nodes {
        seen.claim(node.offset, node.header_size, "a node header");
        let payload = node.payload();
        seen.claim(payload.start, payload.len(), "a node payload");
    }
    // The header declares the texture block, so it is reached even though the walk
    // stops before it. Omitting it was this instrument's first false alarm (3.0 MB
    // over 666 files); check whether the format's own header accounts for a gap.
    if let (Ok(tree), Ok(textures)) = (tree_len(data), texture_len(data)) {
        seen.claim(
            FILE_HEADER_LEN + tree,
            textures,
            "the embedded texture block",
        );
    }
    seen
}
