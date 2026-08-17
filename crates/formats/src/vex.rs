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
//! # `child_count` is 16 bits, and the arithmetic says so
//!
//! Reading it as a `u32` works on most nodes and fails on a few dozen, because
//! `+0x0e` is usually zero. On `01_Track` 54 of 2,071 nodes have something there,
//! and for those a `u32` read gives a child count of 1,572,865, which corrupts
//! every depth after it.
//!
//! The check that settles it: in a pre-order tree with immediate child counts,
//! the counts sum to one less than the node count. As a `u16` that holds exactly
//! on every file tried, ship models and tracks alike. As a `u32` the sum comes
//! out at 111 million for 2,071 nodes.
//!
//! See `docs/formats/vex.md` for the class-ID table and the evidence.
//!
//! # Geometry is pre-batched GE draw calls
//!
//! Meshes are not portable vertex and index buffers. They are batches of PSP
//! Graphics Engine draw calls, **never indexed**, with vertices stored inline
//! after each batch header. This module decodes them into ordinary vertex
//! arrays.
//!
//! Positions are **always** three `s16`, scaled by a per-batch `f32`:
//!
//! ```text
//! position = s16 / 32768.0 * scale
//! ```
//!
//! Missing that scale is the classic failure here: every model comes out a
//! uniform wrong size, which looks like a units problem rather than a decoding
//! bug.

use std::fmt;

use crate::ByteOrder;

/// Bytes of file header before the node tree.
pub const FILE_HEADER_LEN: usize = 16;

/// File magic, at `+0x0c` of the header, as the PSP and PS2 spell it.
pub const MAGIC: &[u8; 4] = b"VEXX";

/// The same magic on the PS3, written by the same exporter on a big-endian
/// host. It is the whole of how a file declares its [`ByteOrder`]; see
/// [`byte_order`].
pub const MAGIC_BE: &[u8; 4] = b"XXEV";

pub mod classes;
mod matrix;

pub use matrix::{
    IDENTITY, class_world_transforms, multiply, transform, transform_point, world_transforms,
};

/// Class ID of a `Mesh` node.
///
/// Version 6 only; [`classes::for_version`] is what a decoder should ask.
pub const CLASS_MESH: u32 = 0x125;

/// Class ID of a `Texture` node.
///
/// Version 6 only; [`classes::for_version`] is what a decoder should ask.
pub const CLASS_TEXTURE: u32 = 0x3c1;

/// Class ID of a `Floor Collision` node.
///
/// One of the five collision classes, all registered with the same vtable by
/// `Collision_RegisterNodeClasses` (`0x08934d44`). Their payloads are decoded by
/// [`collision`](crate::collision), which also documents what is inferred rather
/// than observed about them.
pub const CLASS_FLOOR_COLLISION: u32 = 0x3b9;

/// Class ID of a `Wall Collision` node.
pub const CLASS_WALL_COLLISION: u32 = 0x3ba;

/// Class ID of a `Reset Collision` node.
pub const CLASS_RESET_COLLISION: u32 = 0x3cd;

/// Class ID of a `Mag Floor Collision` node: the magstrip surface.
pub const CLASS_MAG_FLOOR_COLLISION: u32 = 0x3e6;

/// Class ID of a `Cage Collision` node, which the loader parses and then skips.
pub const CLASS_CAGE_COLLISION: u32 = 0x3e7;

/// Class ID of a `WO Track` node: the AI spline graph, decoded by
/// [`track::parse`](crate::track::parse).
pub const CLASS_WO_TRACK: u32 = 0x3bb;

/// Class ID of a `Start Position` node, decoded by
/// [`track::start_position`](crate::track::start_position).
///
/// Exactly one per track file, on all 40 of the PSP disc's.
pub const CLASS_START_POSITION: u32 = 0x3bc;

/// Class ID of a `section` node: the authored visibility partition, decoded by
/// [`pvs::TrackPvs`](crate::pvs::TrackPvs).
///
/// **Not the lap structure**, despite sitting beside `Start Position` and the
/// pads in the class table. A `section` carries a potentially-visible-set
/// bitmask and a bounding box and nothing else; the track path is
/// [`CLASS_WO_TRACK`].
pub const CLASS_SECTION: u32 = 0x3c9;

/// Class ID of a `Transform` node.
///
/// 715 of `01_Track`'s 2,071 nodes. Its payload is a 4x4 matrix, or nothing at
/// all when the transform is the identity.
pub const CLASS_TRANSFORM: u32 = 0x6e;

/// Class ID of a `LodGroup` node: an authored level of detail.
///
/// Its payload's `child_count` (`u32` at `+0x50`) and, when that is `2`, a
/// switch-distance `f32` right after are real authoring-time data - but the
/// original PSP binary never reads either. See `docs/formats/vex.md`,
/// "`LodGroup`: authored, but never switched at runtime".
pub const CLASS_LOD_GROUP: u32 = 0x2ee;

/// Class ID of an `Engine Flare` node: the ship's exhaust.
///
/// Its runtime handler is read at instruction level in
/// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`: an additive camera-facing quad
/// at the nozzle, plus the `~ENGINE` sound and the `<Team>boost.vex` model.
pub const CLASS_ENGINE_FLARE: u32 = 0x3bf;

/// Class ID of a `ParticleSystem` node.
///
/// The `.pob` it names parses through [`crate::pob`], emitter tree and all.
pub const CLASS_PARTICLE_SYSTEM: u32 = 0x3c4;

/// Class ID of a `Trail` node: a position-history ribbon.
///
/// Distinct from [`CLASS_ENGINE_FLARE`], which carries no history at all. The
/// ribbon's ring buffer and draw are in `exhaust.md`.
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
/// [`mesh_batches`] decode it unchanged - same header words, same bounding-box
/// pair at `+0x10`/`+0x20`, same stride-`0x14` material array at `+0x30`. That
/// is measured rather than assumed: see
/// `crates/formats/tests/skycube_ground_truth.rs`, which checks it against all
/// 40 sky nodes on the PSP disc.
///
/// Every track file authors exactly one, parented to the world node, with its
/// geometry inline. Materials run 1, 5 or 6 - six being a full cube, five the
/// same cube without the face nobody sees, and one the Zone variants.
/// `Data\Defaults\Skycube.vex` also exists on the disc but is a **version-4**
/// file in a version-6 archive, so no shipped track can reference it; treat it
/// as a legacy fallback, not as the thing to load.
pub const CLASS_SKYCUBE: u32 = 0x3c6;

/// Class ID of a `fogCube` node: a track's fog volume and parameters.
///
/// 128 bytes on every track that has one - a 64-byte row-major 4x4 in the same
/// convention [`CLASS_TRANSFORM`] uses, then two `{rgb, 0, near, far}` sets of
/// six floats, then eight bytes not yet read. 36 of the 40 track files author
/// one, so a loader must handle its absence.
pub const CLASS_FOGCUBE: u32 = 0x3d3;

/// Class ID of an `AmbientLight` node: a flat colour added everywhere.
///
/// A 16-byte payload, `{r, g, b, intensity}` as four `f32`, decoded by
/// [`lighting::ambient_lights`](crate::lighting::ambient_lights). Placement
/// comes from the node's own transform chain, the same way a pad's does -
/// the payload itself carries no matrix. See `docs/formats/lighting.md`.
pub const CLASS_AMBIENT_LIGHT: u32 = 0x12c;

/// Class ID of a `DirectionalLight` node: a parallel light with no position.
///
/// Same 16-byte `{r, g, b, intensity}` payload shape as [`CLASS_AMBIENT_LIGHT`],
/// decoded by
/// [`lighting::directional_lights`](crate::lighting::directional_lights).
pub const CLASS_DIRECTIONAL_LIGHT: u32 = 0x131;

/// Class ID of a `PointLight` node: a light that falls off with distance.
///
/// A 32-byte payload: `{r, g, b, range}` as four `f32`, then four `u32`
/// observed as `{1, 0, 0, 0}` on every shipped sample and otherwise undecoded.
/// Decoded by [`lighting::point_lights`](crate::lighting::point_lights).
pub const CLASS_POINT_LIGHT: u32 = 0x132;

/// Class ID of a `Dynamic Point Light` node: presumably a moving light source
/// (ships included), per the class name.
///
/// **No parser here.** Authored zero times on any of the 40 PSP track files in
/// an earlier track-only census; the full-disc sweep in
/// `crates/formats/tests/lighting_ground_truth.rs` rechecks that across all of
/// `Data.wad` rather than assuming it holds off-track too.
pub const CLASS_DYNAMIC_POINT_LIGHT: u32 = 0x3c2;

/// Class ID of a `Speedup Pad` node: a boost pad on the track surface.
///
/// **Its payload is a [`CLASS_MESH`] payload**, for the same reason
/// [`CLASS_SKYCUBE`]'s is: the pad's bind handler (`0x089264f4`) calls the
/// `Mesh` bind (`0x0890e998`) first and only then reads its own fields. So a pad
/// carries its own geometry, and the bounding-box pair at `+0x10`/`+0x20` is
/// both the mesh's bounds and the pad's trigger volume. Confidence 85; see
/// [`pads`](crate::pads) and `docs/formats/pads.md`.
///
/// `01_Track` authors nine, each an instance of one shared payload under a
/// different [`CLASS_TRANSFORM`] parent, so a pad's placement is entirely in its
/// transform chain and its volume is entirely in local space.
pub const CLASS_SPEEDUP_PAD: u32 = 0x3bd;

/// Class ID of a `Weapon Pad` node: a pickup pad on the track surface.
///
/// Shares [`CLASS_SPEEDUP_PAD`]'s base vtable, so the payload is decoded the
/// same way. Nothing consumes one yet - there is no pickup system - but the
/// geometry decodes and is asserted against the disc alongside the speedup pads.
pub const CLASS_WEAPON_PAD: u32 = 0x3be;

/// The `.vex` class-ID to name table, as the shipped executable carries it.
///
/// Read out of `BOOT.BIN` at `0x08ab2370` - stride 12, `{u32 id, char *name,
/// ptr}`, names at `0x08a84d40`, terminated by `id == -1`. The third field is a
/// runtime slot that `Vex_RegisterClass` fills at boot, **not** a shared vtable.
/// `docs/ghidra/functions/psp-pulse-usa/exhaust.md` carries the evidence and the
/// confidence score (95, because ten of these IDs were already in this file from
/// unrelated evidence and every one agrees).
///
/// These are names, not content: a class name describes the format, which is what
/// ADR-0006 permits. Nothing here is a tuning value.
///
/// **Not exhaustive.** The read stopped at `0x08ab26a0` without reaching the
/// terminator, so the generic Maya classes with small sequential ids
/// (`0 Invalid`, `1 Base`, `2 Name`, …) are only partly covered and the table's
/// extent is unknown. `0x3e3` genuinely has no entry, and the table does contain
/// out-of-order ids (`0x3d0`, `0x3e9`, `0x3eb`), so a gap proves nothing.
const CLASS_NAMES: &[(u32, &str)] = &[
    (0x06e, "Transform"),
    (0x0f4, "World"),
    (0x0f7, "Camera"),
    (0x123, "NurbsSurface"),
    (0x125, "Mesh"),
    (0x12c, "AmbientLight"),
    (0x131, "DirectionalLight"),
    (0x132, "PointLight"),
    (0x2ee, "LodGroup"),
    (0x3b9, "Floor Collision"),
    (0x3ba, "Wall Collision"),
    (0x3bb, "WO Track"),
    (0x3bc, "Start Position"),
    (0x3bd, "Speedup Pad"),
    (0x3be, "Weapon Pad"),
    (0x3bf, "Engine Flare"),
    (0x3c0, "Anim Transform"),
    (0x3c1, "Texture"),
    (0x3c2, "Dynamic Point Light"),
    (0x3c3, "Dynamic Shadow Occluder"),
    (0x3c4, "ParticleSystem"),
    (0x3c5, "Airbrake"),
    (0x3c6, "Skycube"),
    (0x3c7, "Quake"),
    (0x3c8, "Trail"),
    (0x3c9, "section"),
    (0x3ca, "gate"),
    (0x3cb, "shadow"),
    (0x3cc, "speaker"),
    (0x3cd, "Reset Collision"),
    (0x3ce, "wospot"),
    (0x3cf, "wopoint"),
    (0x3d0, "Ship Collision Fx"),
    (0x3d3, "fogCube"),
    (0x3d4, "MeshNode_Ghost"),
    (0x3d5, "sea"),
    (0x3d6, "seaweed"),
    (0x3d7, "seareflect"),
    (0x3d8, "cloudCube"),
    (0x3d9, "cloudGroup"),
    (0x3da, "weatherPos"),
    (0x3db, "Unused 1"),
    (0x3dc, "animationTrigger"),
    (0x3dd, "gridCamera"),
    (0x3de, "lensflare"),
    (0x3df, "textureBlob"),
    (0x3e0, "blob"),
    (0x3e1, "sound"),
    (0x3e2, "Ship Muzzle"),
    (0x3e4, "exitglow"),
    (0x3e5, "engine_fire"),
    (0x3e6, "Mag Floor Collision"),
    (0x3e7, "Cage Collision"),
    (0x3e9, "soundcone"),
    (0x3eb, "cannon_flash"),
];

/// The shipped name for a class ID, or `None` if the ID is not in
/// [`CLASS_NAMES`].
///
/// `None` means "not in the part of the table that was read", not "invalid" -
/// see [`CLASS_NAMES`] on the missing extent.
#[must_use]
pub fn class_name(class_id: u32) -> Option<&'static str> {
    CLASS_NAMES
        .iter()
        .find(|(id, _)| *id == class_id)
        .map(|(_, name)| *name)
}

/// Every node of one class, in tree order.
///
/// Exists because callers were all hand-rolling
/// `nodes(data).iter().filter(|n| n.class_id == ...)`, and a class filter is the
/// first thing anything reading a `.vex` wants.
pub fn nodes_by_class(nodes: &[Node], class_id: u32) -> impl Iterator<Item = &Node> {
    nodes.iter().filter(move |n| n.class_id == class_id)
}

/// Divisor for `s16` positions and the `f32` scale.
///
/// The game's own offline direction uses 32767; 32768 matches the hardware
/// exactly and the 0.003% difference is irrelevant.
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
    /// Deliberately an error rather than a fall back to version 6: the
    /// numberings share no id, so reading an unknown generation with the wrong
    /// table finds nothing and looks exactly like an empty file.
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
    /// A vertex type this build does not handle.
    ///
    /// The reachable set is small and fully enumerated; anything else means
    /// either an unexplored asset class or a decoding error, and both are worth
    /// hearing about loudly.
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
    /// Separate from [`Self::Vif`] because these are the checks that decide
    /// whether the packet was read as the right *kind* of thing at all: the
    /// three lengths that have to close, and the attribute set having to agree
    /// with the vertex type.
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

/// How a vertex colour is packed.
///
/// The GU's four colour formats. Pulse's ship models use only
/// [`Abgr8888`](ColourFormat::Abgr8888); its **tracks** use
/// [`Abgr4444`](ColourFormat::Abgr4444), which is why a track model refused to
/// decode until this existed.
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
    /// The 16-bit formats are widened by **bit replication**, not by shifting:
    /// 5 bits of `0x1f` has to become `0xff`, and `0x1f << 3` gives `0xf8`. Get
    /// that wrong and every bright surface comes out slightly dark, which is
    /// hard to see and easy to leave in.
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
    /// rule. For every reachable combination the two agree, but reproducing the
    /// game's version means a disagreement would show up as an error here
    /// rather than as silently shifted vertices.
    pub fn from_vertex_type(vertex_type: u16) -> Result<Self> {
        // Position is always three s16: the stride calculation hard-codes a
        // `+ 6`, so bits 7-8 are always 2.
        if vertex_type & 0x0180 != 0x0100 {
            return Err(Error::UnsupportedVertexType { vertex_type });
        }
        // Weights, indices, morphs and the transform-2D bit are never handled.
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
            // u16 texcoords fall through the game's own branch without
            // advancing the offset, which is either a pruned case or a latent
            // bug. Either way, refuse rather than guess.
            _ => return Err(Error::UnsupportedVertexType { vertex_type }),
        };

        // Bits 2-4 are the GU colour format. 1 to 3 are not defined by the
        // hardware, so they are refused rather than guessed at.
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

/// Which blend equation a transparent batch asks for.
///
/// Recovered from `Gfx_BuildBatchStateList`; see [`Batch::blend_class`] for the
/// bits and the branch order. Not a quality setting and not a choice - the
/// batch's own `pass_mask` picks one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendClass {
    /// `pass_mask & 0x100`: ordinary alpha blend, `SrcAlpha` over
    /// `OneMinusSrcAlpha`.
    AlphaOver,
    /// `pass_mask & 0x200`: additive and source-alpha weighted, `SrcAlpha`
    /// plus `One`. The boost plume's, and the engine flare's.
    Additive,
    /// `pass_mask & 0x400`: sorted with the transparent batches but drawn
    /// unblended.
    None,
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
    /// **Not always `vertices.len()`.** On PSP the two agree except where a batch
    /// uses its alternate block, which is counted at `+0x06` instead. On PS2 this
    /// is the count of the draw the batch *describes*, while `vertices` holds
    /// what its VIF packet unpacks: a strip split across chunks unpacks two extra
    /// per boundary to continue itself. Kept because the relationship between the
    /// two is an invariant worth being able to check from outside; see
    /// [`vif_vertices`].
    pub declared_vertex_count: u16,
    /// The batch's own bounding box in model units.
    ///
    /// Every decoded vertex must fall inside it, which is the standing check on
    /// the vertex layout and the scale factor: get either wrong and the positions
    /// leave the box at once.
    ///
    /// On PSP it is stored as `s16` at `+0x18` and `+0x20`, in vertex space, and
    /// is scaled here the same way positions are. On PS2 it is `f32` at `+0x20`
    /// and `+0x30`, already in the space the positions are in.
    pub bounds: ([f32; 3], [f32; 3]),
    /// Byte 3 of the on-disk batch header, read the same way on PSP and PS2
    /// (both share this header layout). Bit `0x40` selects the
    /// extended/alternate header block already accounted for by
    /// [`declared_vertex_count`]'s own doc comment; bit `0x10` is decoded by
    /// [`is_additive_blend`](Self::is_additive_blend), confirmed against the
    /// PSP draw path only. The remaining bits are otherwise undecoded here.
    ///
    /// [`declared_vertex_count`]: Self::declared_vertex_count
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
    /// Most batches are strips, so a renderer that only understands triangle
    /// lists needs this. Strips alternate winding every triangle, and getting
    /// that wrong makes every other face point inwards, which with back-face
    /// culling on produces a model full of holes.
    ///
    /// Returns an empty list for primitive types other than triangles and
    /// strips; points, lines and sprites are not geometry a mesh viewer can
    /// use, and none appear in the models examined.
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
                    // Flip winding on odd triangles so all faces agree.
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

    /// Whether this batch is transparent and must be drawn after opaque ones.
    ///
    /// The `0x0700` mask is confirmed operationally: it is the exact test
    /// `Gfx_BuildBatchStateList` branches on. Which of the three bits is set
    /// decides *how* it blends - see [`Batch::blend_class`].
    #[must_use]
    pub fn is_transparent(&self) -> bool {
        self.pass_mask & 0x0700 != 0
    }

    /// Which blend equation a transparent batch asks for, or `None` when it is
    /// not transparent at all.
    ///
    /// The three bits inside `is_transparent`'s `0x0700` are not
    /// interchangeable, and the reimplementation drew all of them with one
    /// equation until this was recovered. From `Gfx_BuildBatchStateList`, whose
    /// branch order this method reproduces - `0x100` wins over `0x200`, which
    /// wins over `0x400`:
    ///
    /// | Bit | The original programs |
    /// | --- | --- |
    /// | `0x100` | `GU_ADD`, `GU_SRC_ALPHA` / `GU_ONE_MINUS_SRC_ALPHA`, colour test off |
    /// | `0x200` | `GU_ADD`, `GU_SRC_ALPHA` / `GU_FIX 0xffffff`, colour test on |
    /// | `0x400` | `Gu_Disable(GU_BLEND)` - in the transparent class, not blended |
    ///
    /// Only the blend equation and the colour test differ between them; the
    /// depth-write disable, the alpha test and the stencil setup are emitted
    /// outside the nest and are common to all three.
    ///
    /// See `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`, "Three
    /// `pass_mask` bits decoded, inside the `0x0700` transparent class".
    #[must_use]
    pub fn blend_class(&self) -> Option<BlendClass> {
        if self.pass_mask & 0x0100 != 0 {
            Some(BlendClass::AlphaOver)
        } else if self.pass_mask & 0x0200 != 0 {
            Some(BlendClass::Additive)
        } else if self.pass_mask & 0x0400 != 0 {
            Some(BlendClass::None)
        } else {
            None
        }
    }

    /// Whether this batch is alpha-tested rather than blended.
    #[must_use]
    pub fn is_alpha_tested(&self) -> bool {
        self.pass_mask & 0x0800 != 0
    }

    /// Whether back-face culling is enabled. Set means two-sided.
    ///
    /// **The sense of the bit is the opposite of what it looks like, and this
    /// method used to have it backwards.** `Mesh_SetBatchDrawState` reads
    /// `if ((pass_mask & 0x20) == 0) { Gu_Enable(5) } else { Gu_Disable(5) }`,
    /// state index `5` being `GU_CULL_FACE` - so the bit **set** *disables*
    /// culling. Read at instruction level, confidence 90; see
    /// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`, "The plume is
    /// two-sided".
    ///
    /// Nothing in this workspace consumes this yet - every `mesh_render`
    /// pipeline sets `cull_mode: None` deliberately, because strip winding is
    /// reconstructed rather than read from the file and culling would turn a
    /// winding mistake into missing geometry. So the inversion never reached a
    /// picture. It is corrected here because the first consumer would have
    /// culled exactly the surfaces the original draws two-sided, and on the
    /// boost plume - all 32 of whose batches carry the bit - that means every
    /// batch.
    #[must_use]
    pub fn is_culled(&self) -> bool {
        self.pass_mask & 0x0020 == 0
    }

    /// Whether the PSP draw path's shared per-batch state setup
    /// (`Mesh_SetBatchDrawState`) takes its pure-additive blend branch
    /// (`Gu_BlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX 0xffffff)`, i.e.
    /// `dst + src`) rather than its "replace" branch (`GU_FIX 0xffffff,
    /// GU_FIX 0`, i.e. `src` alone, discarding `dst`). Selected by
    /// `header_flags & 0x10`, clear for additive.
    ///
    /// Confidence 85 for what this method decodes: the branch selection and
    /// blend-equation decode are confirmed against a live Ghidra decompile.
    /// That number is not a claim about any specific batch's real value -
    /// whether a given model's batches actually read additive, and whether
    /// `GU_BLEND` is actually *enabled* when they do, are separate,
    /// lower-confidence claims. See
    /// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.
    #[must_use]
    pub fn is_additive_blend(&self) -> bool {
        self.header_flags & 0x10 == 0
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
    /// The `u16` at `+0x0e`, whose meaning is not established.
    ///
    /// Zero on all but a few dozen nodes per file. Where it is set the values
    /// are small and round (24, 36, 48, 56, 68, 368) and look like byte counts,
    /// but nothing has been traced to them. Kept because it is the field that
    /// made `child_count` look 32 bits wide.
    pub unk_0x0e: u16,
    /// Node name from the header, when it carries one.
    pub name: Option<String>,
    /// Depth in the tree, zero for the root.
    pub depth: usize,
    /// Index of this node's parent in the vector [`nodes`] returned.
    ///
    /// `None` for the root. Composing a mesh's world matrix means walking this
    /// chain and multiplying the [`Transform`](CLASS_TRANSFORM) matrices on it.
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
/// The file header and the node walk go through [`byte_order`] instead, because
/// a PS3 `.vex` has those and this project reads them. What it does **not** have
/// is geometry: a PS3 `Mesh` node is a bounding-box pair and a reference into a
/// `.rcsmodel`, so every batch, vertex and embedded-texture decoder below runs
/// on little-endian files by construction. Threading an order through those
/// would be untestable - no big-endian file reaches them.
mod le {
    use crate::ByteOrder;

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

/// Which way round a file's words are, from its own magic.
///
/// `VEXX` on the PSP and PS2, `XXEV` on the PS3 - the same four bytes, written
/// by the same exporter on a big-endian host. So the file says which it is, and
/// nothing here has to ask what console it came from. A file with neither
/// spelling reads as [`ByteOrder::Little`]; [`has_magic`] is the check for "is
/// this a `.vex`" and this is not it.
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
/// This is what a decoder should call. Asking [`classes::for_version`] with a
/// version obtained some other way reintroduces the one mistake the table
/// exists to prevent: pairing a table with a file it does not describe.
///
/// # Errors
///
/// [`Error::TooShort`] for a file with no header, and
/// [`Error::UnknownVersion`] for a format generation this project has never
/// seen - which is deliberately not silently read as version 6.
pub fn classes_of(data: &[u8]) -> Result<classes::Classes> {
    let version = version(data)?;
    classes::for_version(version).ok_or(Error::UnknownVersion { version })
}

/// Whether the file carries the `VEXX` magic, in either spelling.
///
/// The magic sits at `+0x0c`, not at the start, so a naive signature check
/// misses it. `XXEV` counts: it is the same magic on a big-endian host, and a
/// caller that needs to know which asks [`byte_order`].
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
    // Names live after the fixed fields; a minimal header has none.
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
/// Stops at the first structurally impossible node rather than erroring, since
/// the tree is followed by embedded texture data with no explicit terminator.
pub fn nodes(data: &[u8]) -> Result<Vec<Node>> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }

    let order = byte_order(data);
    let mut out: Vec<Node> = Vec::new();
    // Children follow their parent, so a stack of remaining counts tracks depth,
    // and a parallel stack of their indices gives each node its parent.
    let mut remaining: Vec<usize> = Vec::new();
    let mut remaining_parent: Vec<usize> = Vec::new();
    let mut at = FILE_HEADER_LEN;

    // The tree is followed by embedded textures, so stop at its declared end
    // rather than trying to detect where node data stops looking like nodes.
    let end = (FILE_HEADER_LEN + tree_len(data)?).min(data.len());

    while at + 16 <= end {
        // Retire finished subtrees *before* reading the depth, not after. A
        // parent whose children are all consumed is no longer an ancestor, and
        // deferring the pop reports the first node after a completed subtree at
        // the depth of that subtree rather than its own.
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

        // A header smaller than the fields already read, or a node running past
        // the tree, means the structure is not what we think it is.
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

        // Guard against a zero-length node, which would loop forever.
        if next <= at {
            break;
        }
        at = next;
    }

    Ok(out)
}

/// A texture embedded in the file.
#[derive(Debug, Clone)]
pub struct EmbeddedTexture {
    /// Original asset path, from the node name.
    ///
    /// This is the **authoring** path on the artists' machine, e.g.
    /// `Z:/WipeoutPSP/X2/Data/Ships/Feisar/Textures/engine_general.tga`, and it
    /// is only present when the node header is long enough to carry a name.
    /// Track `Texture` nodes have a 32-byte header with the name field zeroed,
    /// so on a track this is `None` for every texture and
    /// [`asset_path`](Self::asset_path) is the only name available.
    pub name: Option<String>,
    /// Runtime asset path, from payload `+0x38`, e.g.
    /// `Data\Ships\Feisar\Textures\engine_general.tga`.
    ///
    /// Present on ships *and* tracks, which is what makes it the field to match
    /// a texture by. Backslash-separated and in the game's own spelling, so it
    /// is also what [`crate::wad::hash_name`] would take.
    pub asset_path: Option<String>,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// 4 or 8.
    pub bits_per_pixel: u8,
    /// Number of mip levels present. Only the base level is decoded.
    pub mip_count: u8,
    /// Palette, RGBA8888.
    pub palette: Vec<[u8; 4]>,
    /// Base-level pixel indices, one per pixel.
    pub indices: Vec<u8>,
}

impl EmbeddedTexture {
    /// Expands the base level to RGBA8888.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.indices.len() * 4);
        for &i in &self.indices {
            let c = self
                .palette
                .get(i as usize)
                .copied()
                .unwrap_or([255, 0, 255, 255]);
            out.extend_from_slice(&c);
        }
        out
    }
}

/// Offset of the runtime asset path inside a `Texture` node's payload.
///
/// The two pointer fields at `+0x10` and `+0x14` are zero at rest and patched at
/// load, so the path is the last field of the header that survives on disc.
const TEXTURE_ASSET_PATH: usize = 0x38;

/// The runtime asset path out of a `Texture` node's payload.
///
/// Separate from [`textures`] because it needs only the node, not the embedded
/// pixel block, so it also answers "what does this file reference" on a PS2
/// scene whose texture block is empty.
#[must_use]
pub fn texture_asset_path(payload: &[u8]) -> Option<String> {
    cstr_at(payload, TEXTURE_ASSET_PATH)
}

/// Extracts the textures appended after the node tree.
///
/// Their data is not pointed at from anywhere: the header's pointer fields are
/// zero at rest and patched at load. Instead each texture's palette and texels
/// are packed back to back in node order, starting immediately after the tree.
///
/// That the sizes add up exactly to the header's declared texture length is
/// what confirms the packing, and this function checks it.
///
/// # Position is the identity
///
/// Materials name a texture by its **ordinal among the `Texture` nodes**, so the
/// returned vector is positional and one entry per node, including nodes this
/// build cannot decode. Those come back as `None`.
///
/// Dropping them instead would silently renumber every later texture, which is
/// not a decoding error that shows up as an error: it shows up as a model wearing
/// the wrong skins. Use `.iter().flatten()` for a plain list.
pub fn textures(data: &[u8]) -> Result<Vec<Option<EmbeddedTexture>>> {
    let block = FILE_HEADER_LEN + tree_len(data)?;
    let mut at = block;
    let mut out = Vec::new();

    // A zero-length texture block is not a malformed file: PS2 `.vex` scenes
    // carry `Texture` nodes with real dimensions but no palette/texel bytes
    // after the tree at all, unlike PSP's. Every node's declared clut/texel
    // size would otherwise be read as if the bytes were there and walk past
    // the end of the file. Where the actual pixels live on PS2 is not yet
    // known; every texture in the model comes back `None` until it is.
    let embedded = texture_len(data)? > 0;

    // **The `Texture` id from this file's version**, not version 6's `0x3c1`.
    // A version-4 file numbers it `0x373`, so the constant matched nothing and
    // every material resolved to no texture at all - which draws a whole track
    // white rather than failing, and is exactly what a Pure race looked like.
    let classes = classes_of(data)?;
    let texture_class = classes.texture;
    for node in nodes(data)?
        .into_iter()
        .filter(|n| Some(n.class_id) == texture_class)
    {
        if !embedded {
            out.push(None);
            continue;
        }

        let p = &data[node.payload()];
        if p.len() < 0x10 {
            out.push(None);
            continue;
        }

        let width = u16_at(p, 0);
        let height = u16_at(p, 2);
        let bits_per_pixel = p[4];
        let mip_count = p[5];
        let clut_size = u32_at(p, 8) as usize;
        let texel_size = u32_at(p, 12) as usize;

        let end = at + clut_size + texel_size;
        if end > data.len() {
            return Err(Error::OutOfBounds {
                what: "embedded texture",
                end,
                len: data.len(),
            });
        }
        if !matches!(bits_per_pixel, 4 | 8) {
            // The data is still there and still has to be stepped over, so the
            // *next* texture stays correctly positioned in the block. Only this
            // one is unavailable.
            at = end;
            out.push(None);
            continue;
        }

        let palette: Vec<[u8; 4]> = data[at..at + clut_size]
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect();

        // Only the base level; mips follow it and are not needed for viewing.
        let pixels = usize::from(width) * usize::from(height);
        let row = texture_row_bytes(width, bits_per_pixel);
        let stride = texture_row_stride(width, bits_per_pixel);
        // **The pre-swizzle flag, read on version 4 and below only.**
        //
        // Bit 0 of the flags byte at `+0x06` means the texels are already in the
        // GE's 16-byte by 8-row block order, so a literal read comes out
        // scrambled - which is what every Pure model texture looked like, the
        // flag being `0x61` there against Pulse's `0xe4`.
        //
        // **Gated on the version rather than read unconditionally**, and that is
        // not caution for its own sake: a corpus sweep found bit 0 set on 88 of
        // Pulse PSP's 5,375 `Texture` nodes and 120 of the PS2 pressing's 8,972,
        // on ship liveries and effects rather than on the font atlases the
        // original claim was about. So reading it on version 6 would change what
        // those 88 decode to, and no ground-truth screenshot covers the one model
        // that changed - the regression would pass `just test-data`. Whether
        // those nodes really are swizzled is an open question with its own row on
        // `docs/formats/pure-status.md`; this change deliberately does not
        // settle it, and version 4 is the generation where the evidence is
        // unambiguous.
        let swizzled = matches!(classes.version, 0..=4)
            && p.get(6)
                .is_some_and(|flags| flags & crate::texture::FLAG_SWIZZLED != 0);
        let linear;
        let texels = if swizzled {
            linear =
                crate::texture::unswizzle(&data[at + clut_size..end], stride, usize::from(height));
            &linear[..]
        } else {
            &data[at + clut_size..end]
        };

        let mut indices = Vec::with_capacity(pixels);
        for y in 0..usize::from(height) {
            let Some(line) = texels.get(y * stride..y * stride + row) else {
                break;
            };
            if bits_per_pixel == 8 {
                indices.extend_from_slice(line);
            } else {
                for &b in line {
                    indices.push(b & 0x0f);
                    indices.push(b >> 4);
                }
            }
        }
        // `to_rgba` promises exactly `width * height` texels and its consumers
        // index against those dimensions, so a block too short to hold every
        // row (which no Pulse PSP texture is - see the closure argument in
        // `texture_row_stride` - but a file sized by some other rule could be)
        // is padded out rather than handed on short. The same call caps a
        // 4-bit odd width, where the last byte carries a pixel past the end.
        indices.resize(pixels, 0);

        out.push(Some(EmbeddedTexture {
            name: node.name,
            asset_path: cstr_at(p, TEXTURE_ASSET_PATH),
            width,
            height,
            bits_per_pixel,
            mip_count,
            palette,
            indices,
        }));
        at = end;
    }

    Ok(out)
}

/// How many bytes of one texture row hold actual picture, before padding.
#[must_use]
pub fn texture_row_bytes(width: u16, bits_per_pixel: u8) -> usize {
    (usize::from(width) * usize::from(bits_per_pixel)).div_ceil(8)
}

/// The stride between texture rows: [`texture_row_bytes`] rounded up to 16.
///
/// **The GE's texture buffer width is in units of 16 bytes**, so a row narrower
/// than that is padded out and the next row starts on the next 16-byte
/// boundary, not immediately after the picture. A decoder that reads rows back
/// to back is right only where the picture already fills the stride - which is
/// every 4-bit texture 32 pixels or wider and every 8-bit one 16 or wider, i.e.
/// nearly all of them, which is exactly why this went unnoticed.
///
/// **Confidence 90.** Measured rather than assumed: every `Texture` node
/// declares its own total texel-block size at payload `+0x0c`, and summing this
/// stride over each texture's declared mip levels reproduces that number for
/// **135 of 135** textures on `16_Track`. The unpadded formula reproduces
/// **22** of them - it agrees only where the padding is a no-op. See
/// `crates/formats/tests/texture_stride_ground_truth.rs`, which re-measures it
/// across circuits and ships, and `docs/formats/vex.md`.
///
/// What this is *not*: swizzling. A swizzled PSP texture is reordered into
/// 16-byte by 8-row blocks, and reading it row-wise would corrupt every texture
/// wider than the block rather than only the narrow ones. Every 64-pixel-wide
/// 4-bit texture on the disc decodes correctly read row-wise, so this data is
/// linear with padded rows.
#[must_use]
pub fn texture_row_stride(width: u16, bits_per_pixel: u8) -> usize {
    texture_row_bytes(width, bits_per_pixel).next_multiple_of(16)
}

/// One material of a mesh, from the stride-`0x14` array at `+0x30`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Material {
    /// The `u16` at `+0x00`: render state, not an animation switch.
    ///
    /// Surveyed across every material of all 12 PSP circuits. It is richly
    /// varied (19 distinct values on `07_Track` alone) and correlates with the
    /// artists' own naming, but it does **not** separate animated surfaces from
    /// static ones - `flicker1nonalpha_GLOW` and the plainly static
    /// `hub_banner_GLOW` both carry `0x91`. Two bits are legible:
    ///
    /// - `0x0080` accompanies the `_GLOW`/additive naming convention.
    /// - `0x2000` lands on exactly the `*_shinemap` textures, which is
    ///   independent corroboration of the "extra pass" reading of the same bit
    ///   in a batch's `pass_mask`, and of [`Material::second_texture`].
    ///
    /// Nothing consumes it yet. Recorded so the census is reproducible from the
    /// parser rather than from a one-off script. See `docs/formats/vex.md`.
    pub flags: u16,
    /// The `u32` at `+0x04`: an index into the model's texture array, from
    /// [`textures`].
    pub texture: u32,
    /// The `u32` at `+0x08`: the second texture of the `0x2000` extra pass.
    ///
    /// Zero on every material of `07_Track`, so the pass it belongs to is not
    /// exercised there and this stays unread by the renderer.
    pub second_texture: u32,
}

/// Materials of one mesh payload.
///
/// Stride 0x14, starting at `+0x30`.
///
/// Positional for the same reason as [`textures`]: a batch selects a material by
/// index, so a material that runs past the payload has to come back as `None`
/// rather than shorten the list and renumber the ones after it.
///
/// The remaining `+0x0c..0x14` is **proven zero** on every material of every
/// PSP circuit, which is what rules out an authored per-surface UV scroll rate
/// and forces texture-keyed animation instead.
#[must_use]
pub fn mesh_materials(payload: &[u8]) -> Vec<Option<Material>> {
    if payload.len() < 0x30 {
        return Vec::new();
    }
    let count = usize::from(u16_at(payload, 2));
    (0..count)
        .map(|i| {
            let at = 0x30 + i * 0x14;
            (at + 0x0c <= payload.len()).then(|| Material {
                flags: u16_at(payload, at),
                texture: u32_at(payload, at + 4),
                second_texture: u32_at(payload, at + 8),
            })
        })
        .collect()
}

/// One keyframe track of a mesh's texture-transform block: key times paired
/// with `(u, v)` values.
///
/// Times are `u16`s in 60 Hz frames (the block's `+0x0c` is `1/60` on every
/// block read); values are `s16` pairs in 1/256 units, so `256` is `1.0`.
/// Recovered from `TexAnim_EvalKeyframes` (`0x08927034`) - see
/// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`, "The values
/// gap is closed", and `docs/formats/vex.md`, "The texture-transform keyframe
/// block".
#[derive(Debug, Clone, PartialEq)]
pub struct TexTransformTrack {
    /// Key times, 60 Hz frames, ascending.
    pub times: Vec<u16>,
    /// `(u, v)` at each key, in 1/256 units.
    pub values: Vec<(i16, i16)>,
}

impl TexTransformTrack {
    /// Evaluates the track at `t` frames, the way the engine's evaluator does:
    /// clamp to the first key below `times[0]`, to the last key past the end,
    /// linear interpolation between keys otherwise. Returns `(u, v)` in
    /// texture units (the 1/256 scaling applied).
    #[must_use]
    pub fn sample(&self, t: f32) -> (f32, f32) {
        self.sample_with(t, false)
    }

    /// [`sample`](Self::sample), with the block's step flag applied.
    ///
    /// `TexAnim_EvalKeyframes` takes the flag as an argument and, when it is
    /// set, snaps to a key instead of interpolating between two. It is not a
    /// detail: the flicker sequences on `16_Track` are authored as key *pairs*
    /// one frame apart (`(3, 4)`, `(7, 8)`, ...), and lerping across the gaps
    /// between pairs turns a hard flicker into a slow slide.
    ///
    /// **Which key it snaps to is a choice, not a read.** The decompilation
    /// says "snaps to the key" without settling the direction, and this holds
    /// the **preceding** one. That is what those key pairs argue for - hold a
    /// value, then jump to the next - and holding the *following* key instead
    /// would shift every stepped surface one segment early rather than change
    /// what it looks like. Confidence 60 on the direction alone, per
    /// `docs/reverse-engineering/confidence-rubric.md`; everything else here
    /// is read at instruction level.
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

    /// The last key time, in frames - the span the engine's per-model clock
    /// loops over for a looping animation like the boost plume's.
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
    /// Seconds per key-time unit, from the block's `+0x0c`. `1/60` on every
    /// block read so far, which is what makes key times 60 Hz frames.
    pub seconds_per_key: f32,
    /// The authored loop period in seconds, from the block's `+0x2c`.
    ///
    /// **Not the last key time**, and reading it as such is wrong by a factor
    /// of four on `16_Track`'s flicker sequences: their tracks end at frame
    /// 12, 18 or 24 while all three author a 50-frame loop. That difference is
    /// the whole point of the field - sibling meshes carry the same steps at
    /// different key times and share one period, which is how the original
    /// interleaves their phase.
    pub loop_seconds: f32,
    /// Bit 0 of the *word* at `+0x2c`, whose float value is
    /// [`loop_seconds`](Self::loop_seconds). Set means snap to the preceding
    /// key rather than interpolate - see
    /// [`TexTransformTrack::sample_with`].
    pub step: bool,
}

impl TexTransform {
    /// Evaluates both tracks at `seconds`, the way `TexAnim_UpdateTransform`
    /// (`0x08927204`) does: wrap the time by
    /// [`loop_seconds`](Self::loop_seconds), divide by
    /// [`seconds_per_key`](Self::seconds_per_key) to reach key-time units, and
    /// sample. Returns `(scale, offset)`, both in texture units.
    ///
    /// An empty track evaluates to the engine's own not-found default -
    /// `(1.0, 1.0)` for the scale, `(0.0, 0.0)` for the offset - rather than to
    /// zero, so a block that authors only one of the two leaves the other
    /// alone.
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

/// The texture-transform keyframe block of one mesh payload, if it carries
/// any keys.
///
/// This is the block of **material 0**. A mesh authors one `0x40`-byte block
/// per material; [`mesh_tex_transforms`] returns all of them. Kept as its own
/// entry point because most animated meshes have exactly one material, and
/// every caller that predates the per-material reading wants this one.
#[must_use]
pub fn mesh_tex_transform(payload: &[u8]) -> Option<TexTransform> {
    mesh_tex_transform_at(payload, 0)
}

/// Every material's texture-transform block, in material order.
///
/// The blocks sit immediately after the material array, at
/// `+0x30 + material_count * 0x14`, `0x40` bytes each - the same array the
/// runtime reaches as `mesh+0x60 + material_index * 0x40`, relocated in place.
/// An entry is `None` when that material authors no track at all, which is the
/// identity transform per the engine's own default.
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

/// One material's block. See [`mesh_tex_transforms`] for the layout, and
/// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md` for the field
/// map.
///
/// Returns `None` when the material does not carry the `& 0x10` flag, when the
/// block (or any key data it points at) runs past the payload, or when both
/// tracks are empty.
///
/// **The flag test is the engine's own gate**, not belt-and-braces:
/// `Mesh_UpdateTextureTransforms` (`0x0890e160`) walks the materials and
/// evaluates only those carrying it. It matters because a `.vex` payload that
/// is not a mesh at all still parses this far - a `Skycube` payload *is* a
/// Mesh payload - and arbitrary bytes read as a plausible block often enough
/// to matter. Both predicates agree on everything measured; see
/// `crates/render/tests/authored_uv_ground_truth.rs`, which asserts that
/// rather than assuming it.
fn mesh_tex_transform_at(payload: &[u8], material_index: usize) -> Option<TexTransform> {
    if payload.len() < 0x30 {
        return None;
    }
    let material_count = usize::from(u16_at(payload, 2));
    if material_index >= material_count {
        return None;
    }
    // The engine's gate: `& 0x10` on the material's first `u16`.
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
    // The `times`/`values` fields are relative to the **start of the block
    // array**, not to the block that holds them. Indistinguishable on a
    // single-material mesh, and wrong on `16_Track`'s two-material hologram
    // panels: material 1's fields resolve to real key data off the array base
    // and to noise off its own block.
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
    // The `times`/`values` fields are relative to the block, so a per-material
    // block's key data is reached from that block's own base, not the first's.
    let loop_word = u32_at(payload, block + 0x2c);
    Some(TexTransform {
        offset: track(offset_count, 0x04, 0x10)?,
        scale: track(scale_count, 0x08, 0x14)?,
        seconds_per_key: f32::from_bits(u32_at(payload, block + 0x0c)),
        loop_seconds: f32::from_bits(loop_word),
        step: loop_word & 1 != 0,
    })
}

/// Decodes the batches of one mesh payload.
///
/// `payload` is the mesh node's data. `batch_list` selects list A (`0`) or list
/// B (`1`); a batch belongs to a list while the corresponding `pass_mask` bit
/// is set.
pub fn mesh_batches(payload: &[u8], batch_list: u8) -> Result<Vec<Batch>> {
    if payload.len() < 0x30 {
        return Err(Error::TooShort { got: payload.len() });
    }

    // The loader relocates these into pointers; in the file they are offsets
    // from the start of the mesh payload.
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

        // A PS2 batch's payload is a VIF packet rather than an interleaved
        // vertex array, and carries its bounding box as floats in the space the
        // positions are already in. See `is_vif_batch`.
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

        // `payload_size` covers the vertex data only, so a batch is its header
        // plus that.
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

/// Bits 7-8 set to `3`: positions are 32-bit floats.
///
/// Unreachable on PSP, where the game's own stride calculator hard-codes a `+ 6`
/// for three `s16`, and universal on PS2, where the VU works in floats.
pub const POSITION_F32: u16 = 0x0180;

/// Whether a batch's vertices are a PS2 VIF packet rather than a PSP vertex
/// array.
///
/// The discriminator is the position width, and it is not a heuristic: every PS2
/// batch observed declares 32-bit float positions and no PSP batch can, because
/// the PSP loader's stride calculator adds a hard-coded 6 bytes for three `s16`.
/// The reading is then confirmed structurally by the packet itself - see
/// [`vif_vertices`] for the three framing checks it has to pass.
#[must_use]
pub fn is_vif_batch(vertex_type: u16) -> bool {
    vertex_type & POSITION_BITS == POSITION_F32
}

/// Bytes of framing before a PS2 batch's DMA packet.
const VIF_REGION_HEADER: usize = 16;

/// Bytes of DMA tag at the head of the packet, which is one quadword: 8 bytes of
/// tag and two VIF command words in the upper half.
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
/// PS2 vertex colours are 0 to 128, not 0 to 255: 128 is 1.0 through the
/// texture-modulate path. Every colour byte in the two models measured is 0 to
/// 127, so nothing here saturates in practice, and reading them as 0-255 would
/// make every model exactly half as bright - which reads as a lighting problem
/// rather than a decoding one. See `docs/formats/vex.md`.
const PS2_COLOUR_ONE: u16 = 128;

/// Decodes the vertices of one PS2 batch, whose payload is a VIF packet.
///
/// `at` is the batch header's offset in the mesh payload and `payload_size` its
/// declared vertex-data length, both as [`mesh_batches`] reads them.
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
/// The stream is a run of chunks, each ending in `MSCNT`, and each holding one
/// `UNPACK` per attribute at a fixed VU address: position at 4, colour at 5,
/// texture coordinates at 6, normals at 7. A chunk is one draw, so a strip
/// longer than VU1 memory allows is split across several - by **repeating two
/// vertices**, which is why the decoded vertex count exceeds the batch header's
/// by two per extra chunk.
///
/// # Why the chunks are concatenated
///
/// Returning one vertex list, rather than the chunks separately, is only correct
/// because of a second property: **every chunk but the last has an even vertex
/// count**, on all 89,302 strip batches of the PS2 disc. A strip's winding
/// alternates per triangle, so the first triangle of the next chunk starts at an
/// even global index and hardware order and concatenated order agree; the two
/// repeated vertices become zero-area triangles at the seam. An odd non-final
/// chunk would wind every triangle after it backwards - inside-out geometry
/// wherever the batch is culled - so it is refused here rather than drawn, and
/// the day one turns up is the day this needs per-chunk triangle generation.
///
/// # Errors
///
/// [`Error::Vif`] if the command stream does not walk, and [`Error::Packet`] if
/// the framing does not close, an attribute array is missing or has the wrong
/// shape, the normal array disagrees with the vertex type, or a split strip's
/// chunk has an odd vertex count.
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
    // Three independent framing checks, and all three are exact: the region
    // header's length plus its own 16 bytes is the batch's declared payload, the
    // DMA tag's quadword count spans the packet, and the vertex type is repeated.
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

    // The command stream starts at the tag quadword's upper half, which holds
    // the first two VIF commands, and runs to the end of the packet.
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
    // Lengths of the chunks emitted so far, for the parity check below.
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
            // A microprogram call is one draw, so it is the boundary the
            // attribute arrays gathered so far belong to.
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
    // `.vex` files on the PS2 disc, a normal array is present exactly when bits
    // 5-6 say `s8 normal` and absent otherwise, so a disagreement there means the
    // packet was misread and is an error. Colour and texture coordinates are a
    // different matter: **every** PS2 chunk carries both, including the 4,378
    // batches whose type declares no colour at all, and the coordinates are
    // always two floats whatever the type's texture-coordinate bits say. See
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
