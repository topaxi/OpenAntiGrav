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

/// Bytes of file header before the node tree.
pub const FILE_HEADER_LEN: usize = 16;

/// File magic, at `+0x0c` of the header.
pub const MAGIC: &[u8; 4] = b"VEXX";

pub mod classes {
    //! Class IDs, keyed by the file's own version word.
    //!
    //! # Why the version and not the title
    //!
    //! A class ID is a **table index, not a stable enum**: the numbering shifted
    //! wholesale between format versions, so `0x125` means `Mesh` in a version-6
    //! file and something else entirely in a version-4 one. Keying on which disc
    //! a file came off would be both wrong and unnecessary, and
    //! [ADR-0022](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md)
    //! says why: the decoder reads the artifact and picks a table.
    //!
    //! **This is not a guess dressed up as a design.** The Pulse PSP disc ships
    //! its own version-4 file - `Data\Defaults\Skycube.vex`, in an otherwise
    //! version-6 archive - and its class IDs are the *older* numbering, not
    //! Pulse's: `0x0ee` world, `0x373` texture, `0x378` shape, measured in
    //! `crates/formats/tests/skycube_ground_truth.rs`. Its `0x373` is exactly
    //! the `Texture` ID `docs/formats/pure-status.md` measured across 156 Pure
    //! files. So a version-4 file uses version-4 numbering whichever disc it
    //! came from, which is the whole claim this module rests on, and it was
    //! testable before any Pure work started.
    //!
    //! # The older tables are deliberately incomplete
    //!
    //! [`V6`] is fully recovered. [`V4`] carries only what has been measured,
    //! and [`V3`] almost nothing - a `None` here means "not recovered", never
    //! "absent from the format", and per `CLAUDE.md`'s naming rules an
    //! unrecovered ID gets no entry rather than a plausible one. `docs/formats/pure-status.md`
    //! costs out what filling them in would take.

    /// One format version's class-ID assignments.
    ///
    /// `None` means the ID has not been recovered for that version. A decoder
    /// that needs one and finds `None` should report that it cannot read the
    /// file, not fall back to another version's numbering: the two tables share
    /// no values, so a fallback would silently match nothing or, worse, match
    /// the wrong node type.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Classes {
        /// The format version this table belongs to.
        pub version: u32,
        pub mesh: Option<u32>,
        pub texture: Option<u32>,
        pub transform: Option<u32>,
        pub wo_track: Option<u32>,
        pub section: Option<u32>,
        pub skycube: Option<u32>,
        pub fogcube: Option<u32>,
        pub speedup_pad: Option<u32>,
        pub weapon_pad: Option<u32>,
        pub start_position: Option<u32>,
        pub floor_collision: Option<u32>,
        pub wall_collision: Option<u32>,
        pub reset_collision: Option<u32>,
        pub mag_floor_collision: Option<u32>,
        pub cage_collision: Option<u32>,
    }

    /// The table used by every file Pulse ships except one.
    ///
    /// Recovered in full; see each `CLASS_*` constant's own doc comment for the
    /// evidence behind it.
    pub const V6: Classes = Classes {
        version: 6,
        mesh: Some(super::CLASS_MESH),
        texture: Some(super::CLASS_TEXTURE),
        transform: Some(super::CLASS_TRANSFORM),
        wo_track: Some(super::CLASS_WO_TRACK),
        section: Some(super::CLASS_SECTION),
        skycube: Some(super::CLASS_SKYCUBE),
        fogcube: Some(super::CLASS_FOGCUBE),
        speedup_pad: Some(super::CLASS_SPEEDUP_PAD),
        weapon_pad: Some(super::CLASS_WEAPON_PAD),
        start_position: Some(super::CLASS_START_POSITION),
        floor_collision: Some(super::CLASS_FLOOR_COLLISION),
        wall_collision: Some(super::CLASS_WALL_COLLISION),
        reset_collision: Some(super::CLASS_RESET_COLLISION),
        mag_floor_collision: Some(super::CLASS_MAG_FLOOR_COLLISION),
        cage_collision: Some(super::CLASS_CAGE_COLLISION),
    };

    /// The table Pure's 156 version-4 files use, and Pulse's one legacy
    /// `Skycube.vex`.
    ///
    /// `texture` is corroborated twice and independently: `docs/formats/pure-status.md`
    /// measured `0x373` across 156 Pure files by closing
    /// `sum(clut_size + texel_size)` against each file's declared texture-block
    /// length, and `skycube_ground_truth.rs` reads the same `0x373` out of a
    /// file on the *Pulse* disc.
    ///
    /// `mesh`, `transform` and `wo_track` come from `pure-status.md` alone, at
    /// confidences 92, 90 and 94.
    pub const V4: Classes = Classes {
        version: 4,
        mesh: Some(0x11e),
        texture: Some(0x373),
        transform: Some(0x6d),
        wo_track: Some(0x36d),
        section: None,
        skycube: None,
        fogcube: None,
        speedup_pad: None,
        weapon_pad: None,
        start_position: None,
        floor_collision: None,
        wall_collision: None,
        reset_collision: None,
        mag_floor_collision: None,
        cage_collision: None,
    };

    /// The table Pure's 15 version-3 files use.
    ///
    /// Nothing is recovered. Version 3 also **shortens the batch header** - it
    /// carries no `alternate` pair - so a v3 file needs more than a class table
    /// before it decodes; `docs/formats/pure-status.md` records that walk at
    /// confidence 65.
    pub const V3: Classes = Classes {
        version: 3,
        mesh: None,
        texture: None,
        transform: None,
        wo_track: None,
        section: None,
        skycube: None,
        fogcube: None,
        speedup_pad: None,
        weapon_pad: None,
        start_position: None,
        floor_collision: None,
        wall_collision: None,
        reset_collision: None,
        mag_floor_collision: None,
        cage_collision: None,
    };

    /// The class table for a file's version word, or `None` for a version this
    /// project has never seen.
    ///
    /// `None` is not a decode failure to swallow: it means the file is a format
    /// generation nobody has looked at, and saying so is more useful than
    /// guessing at [`V6`].
    #[must_use]
    pub fn for_version(version: u32) -> Option<Classes> {
        match version {
            6 => Some(V6),
            4 => Some(V4),
            3 => Some(V3),
            _ => None,
        }
    }
}

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

/// Model-space matrices of every node of `class_id` that carries a 4x4 payload.
///
/// The locator classes - `Engine Flare`, `Ship Muzzle`, `Ship Collision Fx`,
/// `cannon_flash`, `Start Position` - all store a **64-byte payload in exactly
/// the layout [`transform`] reads**: row-major, translation in row 3. That is not
/// assumed here; [`crate::track::start_position`] already decodes `Start
/// Position` that way, and its reading is corroborated against the running game
/// to within 1.12 degrees of heading (see `docs/formats/track.md`).
///
/// [`world_transforms`] deliberately treats every non-`Transform` class as the
/// identity, because a track's assembly must not depend on guessing at payloads
/// it does not decode. This function is the opposite trade, taken explicitly for
/// one class at a time, and it composes with the parent chain the same way.
///
/// Nodes whose payload is too short to be a matrix are skipped rather than
/// defaulted to the identity: a locator at the origin and a locator that failed
/// to decode should not look the same to a caller.
pub fn class_world_transforms(data: &[u8], nodes: &[Node], class_id: u32) -> Vec<[f32; 16]> {
    let chain = world_transforms(data, nodes);
    nodes
        .iter()
        .filter(|node| node.class_id == class_id)
        .filter_map(|node| {
            let local = transform(data.get(node.payload())?)?;
            let parent = node
                .parent
                .and_then(|p| chain.get(p).copied())
                .unwrap_or(IDENTITY);
            Some(multiply(&local, &parent))
        })
        .collect()
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

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn f32_at(data: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(data, at))
}

/// Format version, from the file header.
pub fn version(data: &[u8]) -> Result<u32> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    Ok(u32_at(data, 0))
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

/// Whether the file carries the `VEXX` magic.
///
/// The magic sits at `+0x0c`, not at the start, so a naive signature check
/// misses it.
#[must_use]
pub fn has_magic(data: &[u8]) -> bool {
    data.len() >= FILE_HEADER_LEN && &data[12..16] == MAGIC
}

/// Byte length of the node tree, from the file header.
pub fn tree_len(data: &[u8]) -> Result<usize> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    Ok(u32_at(data, 4) as usize)
}

/// Byte length of the embedded texture block, from the file header.
pub fn texture_len(data: &[u8]) -> Result<usize> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    Ok(u32_at(data, 8) as usize)
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

        let header_size = u16_at(data, at + 4) as usize;
        let node = Node {
            class_id: u32_at(data, at),
            offset: at,
            header_size,
            data_size: u32_at(data, at + 8) as usize,
            child_count: usize::from(u16_at(data, at + 12)),
            unk_0x0e: u16_at(data, at + 14),
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

/// A `Transform` node's matrix, or `None` if the payload is not one.
///
/// **Row-major, translation in row 3**, which is the row-vector convention:
/// `v' = v * M`. Rows 0 to 2 are an orthonormal basis in every node checked, and
/// row 3 ends in `1.0`.
///
/// A `Transform` with an empty payload is the identity, which is how 57 of
/// `01_Track`'s 715 transforms are stored.
///
/// The convention is corroborated outside this format: the `Start Position` bind
/// forces **row 1** to `(0, 1, 0)` when it re-orthonormalises a grid slot, so row
/// 1 is the up axis and `+y` is world up. See `docs/formats/track.md`.
#[must_use]
pub fn transform(payload: &[u8]) -> Option<[f32; 16]> {
    if payload.is_empty() {
        return Some(IDENTITY);
    }
    if payload.len() < 64 {
        return None;
    }
    let mut m = [0.0f32; 16];
    for (i, cell) in m.iter_mut().enumerate() {
        *cell = f32_at(payload, i * 4);
    }
    Some(m)
}

/// The identity, in the same row-major layout as [`transform`].
pub const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0, //
];

/// Multiplies two row-major matrices: the result applies `a` then `b`.
#[must_use]
pub fn multiply(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut out = [0.0f32; 16];
    for row in 0..4 {
        for col in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[row * 4 + k] * b[k * 4 + col];
            }
            out[row * 4 + col] = sum;
        }
    }
    out
}

/// Applies a row-major matrix to a point, with an implicit `w` of 1.
#[must_use]
pub fn transform_point(m: &[f32; 16], p: [f32; 3]) -> [f32; 3] {
    [
        p[0] * m[0] + p[1] * m[4] + p[2] * m[8] + m[12],
        p[0] * m[1] + p[1] * m[5] + p[2] * m[9] + m[13],
        p[0] * m[2] + p[1] * m[6] + p[2] * m[10] + m[14],
    ]
}

/// World matrix of every node, composed down the tree.
///
/// One entry per node of [`nodes`], in the same order. A node's matrix is the
/// product of every `Transform` matrix on its ancestor chain, itself included, so
/// a `Mesh` can be placed with a single lookup.
///
/// This is what makes a whole track assemblable: mesh vertices are in the local
/// space of whichever transform encloses them, and on `01_Track` a mesh sits
/// under up to 25 nested transforms.
pub fn world_transforms(data: &[u8], nodes: &[Node]) -> Vec<[f32; 16]> {
    let mut out: Vec<[f32; 16]> = Vec::with_capacity(nodes.len());
    for node in nodes {
        let parent = node
            .parent
            .and_then(|p| out.get(p).copied())
            .unwrap_or(IDENTITY);

        let local = if node.class_id == CLASS_TRANSFORM {
            data.get(node.payload())
                .and_then(transform)
                .unwrap_or(IDENTITY)
        } else {
            IDENTITY
        };

        out.push(multiply(&local, &parent));
    }
    out
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

    for node in nodes(data)?
        .into_iter()
        .filter(|n| n.class_id == CLASS_TEXTURE)
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
        let texels = &data[at + clut_size..end];
        let row = texture_row_bytes(width, bits_per_pixel);
        let stride = texture_row_stride(width, bits_per_pixel);

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
mod tests {

    /// A minimal mesh payload carrying the boost plume's own keyframe block -
    /// the bytes as they sit on disc (`docs/formats/vex.md`, "The
    /// texture-transform keyframe block"), so the parser and the sampler are
    /// covered without a disc image; `boost_plume_ground_truth.rs` pins the
    /// same values against every team's real file.
    #[test]
    fn tex_transform_block_parses_and_samples_like_the_engine() {
        let mut payload = vec![0u8; 0x30];
        payload[2] = 1; // material_count
        payload.extend([0u8; 0x14]); // one material
        payload[0x30] = 0x10; // ...carrying the engine's animate-me flag
        let block = payload.len(); // 0x44
        payload.extend([0u8; 0x30]); // the block itself
        let w =
            |p: &mut Vec<u8>, at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        payload[block..block + 2].copy_from_slice(&2u16.to_le_bytes()); // offset keys
        payload[block + 2..block + 4].copy_from_slice(&1u16.to_le_bytes()); // scale keys
        // key data appended after the block, offsets relative to the block
        let scale_times = payload.len() - block;
        payload.extend(90u16.to_le_bytes());
        let scale_values = payload.len() - block;
        payload.extend(256u16.to_le_bytes());
        payload.extend(256u16.to_le_bytes());
        let offset_times = payload.len() - block;
        payload.extend(1u16.to_le_bytes());
        payload.extend(90u16.to_le_bytes());
        let offset_values = payload.len() - block;
        for v in [2i16, 0, 253, 0] {
            payload.extend(v.to_le_bytes());
        }
        w(&mut payload, block + 0x04, offset_times as u32);
        w(&mut payload, block + 0x08, scale_times as u32);
        w(&mut payload, block + 0x10, offset_values as u32);
        w(&mut payload, block + 0x14, scale_values as u32);

        let transform = super::mesh_tex_transform(&payload).expect("block parses");
        assert_eq!(transform.offset.times, vec![1, 90]);
        assert_eq!(transform.offset.values, vec![(2, 0), (253, 0)]);
        assert_eq!(transform.scale.sample(45.0), (1.0, 1.0));
        // Clamp below the first key and past the last, lerp between - the
        // engine evaluator's own behaviour (`TexAnim_EvalKeyframes`).
        assert_eq!(transform.offset.sample(0.0), (2.0 / 256.0, 0.0));
        assert_eq!(transform.offset.sample(90.0), (253.0 / 256.0, 0.0));
        assert_eq!(transform.offset.sample(500.0), (253.0 / 256.0, 0.0));
        let (u, v) = transform.offset.sample(1.0 + 89.0 / 2.0);
        assert!((u - (2.0 + 251.0 / 2.0) / 256.0).abs() < 1e-6);
        assert_eq!(v, 0.0);
        assert_eq!(transform.offset.period(), 90.0);
    }

    /// Two materials, two blocks, and the second one's key offsets resolving
    /// off the **array base** rather than off its own block.
    ///
    /// Shaped after `16_Track`'s hologram panels, whose two materials scroll V
    /// one tile per 60 and per 120 frames. Read off the wrong base, material 1
    /// decodes to noise (key times in the tens of thousands) rather than
    /// failing, which is why this is pinned rather than left to the
    /// ground-truth test.
    #[test]
    fn a_second_materials_block_resolves_its_keys_off_the_array_base() {
        let mut payload = vec![0u8; 0x30];
        payload[2] = 2; // material_count
        payload.extend([0u8; 0x28]); // two materials
        payload[0x30] = 0x10; // ...both carrying the animate-me flag
        payload[0x44] = 0x10;
        let base = payload.len();
        payload.extend([0u8; 0x80]); // two blocks
        let w =
            |p: &mut Vec<u8>, at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());

        // One shared pool of key data after both blocks. Each block's own
        // times/values are two keys of V scroll over `frames`.
        let author = |p: &mut Vec<u8>, block: usize, frames: u16, loop_word: u32| {
            p[block..block + 2].copy_from_slice(&2u16.to_le_bytes()); // offset keys
            p[block + 2..block + 4].copy_from_slice(&1u16.to_le_bytes()); // scale keys
            let scale_times = p.len() - base;
            p.extend(frames.to_le_bytes());
            let scale_values = p.len() - base;
            p.extend(256u16.to_le_bytes());
            p.extend(256u16.to_le_bytes());
            let offset_times = p.len() - base;
            p.extend(0u16.to_le_bytes());
            p.extend(frames.to_le_bytes());
            let offset_values = p.len() - base;
            for v in [0i16, 0, 0, 256] {
                p.extend(v.to_le_bytes());
            }
            w(p, block + 0x04, offset_times as u32);
            w(p, block + 0x08, scale_times as u32);
            w(p, block + 0x0c, (1.0f32 / 60.0).to_bits());
            w(p, block + 0x10, offset_values as u32);
            w(p, block + 0x14, scale_values as u32);
            w(p, block + 0x2c, loop_word);
        };
        author(&mut payload, base, 60, 1.0f32.to_bits());
        author(&mut payload, base + 0x40, 120, 2.0f32.to_bits());

        let blocks = super::mesh_tex_transforms(&payload);
        assert_eq!(blocks.len(), 2);
        let first = blocks[0].as_ref().expect("material 0 authors a track");
        let second = blocks[1].as_ref().expect("material 1 authors a track");
        assert_eq!(first.offset.times, vec![0, 60]);
        assert_eq!(second.offset.times, vec![0, 120]);
        assert_eq!(second.offset.values, vec![(0, 0), (0, 256)]);
        assert_eq!(first.loop_seconds, 1.0);
        assert_eq!(second.loop_seconds, 2.0);
        // `mesh_tex_transform` is material 0's block, unchanged.
        assert_eq!(super::mesh_tex_transform(&payload).as_ref(), Some(first));
    }

    /// The loop period is the block's own `+0x2c`, not the last key time, and
    /// bit 0 of that word snaps the sample to the preceding key.
    ///
    /// Shaped after `16_Track`'s flicker sequences: keys in pairs one frame
    /// apart, ending at frame 12, over an authored 50-frame loop. Reading the
    /// period as `period()` would run them at four times speed and destroy the
    /// phase interleave with their siblings.
    #[test]
    fn the_loop_period_and_step_flag_come_from_the_block_not_the_keys() {
        let mut payload = vec![0u8; 0x30];
        payload[2] = 1;
        payload.extend([0u8; 0x14]);
        payload[0x30] = 0x10;
        let block = payload.len();
        payload.extend([0u8; 0x40]);
        let w =
            |p: &mut Vec<u8>, at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        payload[block..block + 2].copy_from_slice(&4u16.to_le_bytes());
        let offset_times = payload.len() - block;
        for t in [3u16, 4, 7, 8] {
            payload.extend(t.to_le_bytes());
        }
        let offset_values = payload.len() - block;
        for v in [0i16, 0, 0, -64, 0, -64, 0, -128] {
            payload.extend(v.to_le_bytes());
        }
        w(&mut payload, block + 0x04, offset_times as u32);
        w(&mut payload, block + 0x10, offset_values as u32);
        w(&mut payload, block + 0x0c, (1.0f32 / 60.0).to_bits());
        // 0.8333 s with bit 0 set: the float's low mantissa bit *is* the flag.
        w(&mut payload, block + 0x2c, (50.0f32 / 60.0).to_bits() | 1);

        let t = super::mesh_tex_transform(&payload).expect("block parses");
        assert!(t.step);
        assert!((t.loop_seconds - 50.0 / 60.0).abs() < 1e-6);
        assert_eq!(t.offset.period(), 8.0, "the keys end long before the loop");

        // Stepped: frame 5 sits between keys 4 and 7 and holds key 4's value
        // rather than sliding towards key 7's.
        let (_, offset) = t.sample(5.0 / 60.0);
        assert_eq!(offset, [0.0, -64.0 / 256.0]);
        // Past the last key it clamps, all the way to the wrap.
        let (_, late) = t.sample(45.0 / 60.0);
        assert_eq!(late, [0.0, -128.0 / 256.0]);
        // And the wrap is the block's period, not the last key time: one loop
        // on lands back at the start, where 8 frames on would not.
        let (_, wrapped) = t.sample(50.0 / 60.0);
        assert_eq!(wrapped, t.sample(0.0).1);
        // An absent scale track evaluates to the engine's identity default.
        assert_eq!(t.sample(5.0 / 60.0).0, [1.0, 1.0]);
    }

    /// A material without the `& 0x10` flag has no transform, whatever the
    /// bytes where its block would sit happen to say.
    ///
    /// The engine's own gate (`Mesh_UpdateTextureTransforms` evaluates only
    /// flagged materials), and the thing that keeps a payload which is not
    /// really an animated mesh from picking up an animation: arbitrary bytes
    /// read as a plausible block often enough that "the counts are non-zero"
    /// is not a safe predicate on its own.
    #[test]
    fn a_material_without_the_flag_has_no_transform_however_the_bytes_read() {
        let mut payload = vec![0u8; 0x30];
        payload[2] = 1;
        payload.extend([0u8; 0x14]);
        let block = payload.len();
        payload.extend([0u8; 0x40]);
        // A block that would parse: two offset keys pointing at real data.
        payload[block..block + 2].copy_from_slice(&2u16.to_le_bytes());
        let times = payload.len() - block;
        payload.extend(0u16.to_le_bytes());
        payload.extend(60u16.to_le_bytes());
        let values = payload.len() - block;
        for v in [0i16, 0, 0, 256] {
            payload.extend(v.to_le_bytes());
        }
        let w =
            |p: &mut Vec<u8>, at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        w(&mut payload, block + 0x04, times as u32);
        w(&mut payload, block + 0x10, values as u32);

        assert_eq!(super::mesh_tex_transform(&payload), None, "flag is clear");
        payload[0x30] = 0x10;
        assert!(
            super::mesh_tex_transform(&payload).is_some(),
            "the same bytes with the flag set do parse - so the flag is what \
             decided it, not a malformed block"
        );
    }

    /// An empty block - both counts zero - is the engine's identity default,
    /// not a parse failure, and a payload too short for a block is `None` too.
    #[test]
    fn tex_transform_block_absent_or_empty_is_none() {
        let mut payload = vec![0u8; 0x30];
        payload[2] = 1;
        payload.extend([0u8; 0x14]);
        payload.extend([0u8; 0x30]); // block present, counts 0/0
        assert_eq!(super::mesh_tex_transform(&payload), None);
        assert_eq!(super::mesh_tex_transform(&payload[..0x50]), None);
        assert_eq!(super::mesh_tex_transform(&[0u8; 0x10]), None);
    }

    #[test]
    fn a_version_word_selects_a_table_and_an_unknown_one_selects_none() {
        assert_eq!(classes::for_version(6), Some(classes::V6));
        assert_eq!(classes::for_version(4), Some(classes::V4));
        assert_eq!(classes::for_version(3), Some(classes::V3));
        // Not a decode failure to swallow: a version nobody has looked at
        // should say so rather than be read as version 6.
        assert_eq!(classes::for_version(5), None);
        assert_eq!(classes::for_version(7), None);
    }

    /// The two numberings share no value, which is why falling back from one to
    /// the other would match nothing rather than mismatch loudly.
    #[test]
    fn the_two_recovered_tables_have_no_id_in_common() {
        let v6 = [
            classes::V6.mesh,
            classes::V6.texture,
            classes::V6.transform,
            classes::V6.wo_track,
        ];
        let v4 = [
            classes::V4.mesh,
            classes::V4.texture,
            classes::V4.transform,
            classes::V4.wo_track,
        ];
        for id in v6.into_iter().flatten() {
            assert!(
                !v4.contains(&Some(id)),
                "{id:#x} appears in both numberings"
            );
        }
    }

    /// The version-6 table is the `CLASS_*` constants, not a second copy of
    /// them that can drift.
    #[test]
    fn the_version_6_table_is_the_documented_constants() {
        assert_eq!(classes::V6.mesh, Some(CLASS_MESH));
        assert_eq!(classes::V6.texture, Some(CLASS_TEXTURE));
        assert_eq!(classes::V6.transform, Some(CLASS_TRANSFORM));
        assert_eq!(classes::V6.wo_track, Some(CLASS_WO_TRACK));
    }
    use super::*;

    /// Builds a `.vex` file from `(class_id, child_count, payload_len)` nodes,
    /// in the depth-first pre-order the format stores them in.
    fn build_tree(nodes: &[(u32, usize, usize)]) -> Vec<u8> {
        let mut tree = Vec::new();
        for &(class_id, children, payload_len) in nodes {
            tree.extend(class_id.to_le_bytes());
            tree.extend(0x20u16.to_le_bytes()); // header_size
            tree.extend(0u16.to_le_bytes()); // padding to +0x08
            tree.extend((payload_len as u32).to_le_bytes());
            tree.extend((children as u32).to_le_bytes());
            tree.extend([0u8; 0x10]); // the name area, left empty
            tree.extend(std::iter::repeat_n(0u8, payload_len));
        }

        let mut out = Vec::new();
        out.extend(6u32.to_le_bytes());
        out.extend((tree.len() as u32).to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(MAGIC);
        out.extend(tree);
        out
    }

    #[test]
    fn walks_a_flat_tree() {
        let data = build_tree(&[(1, 0, 0), (2, 0, 16), (3, 0, 0)]);
        let nodes = nodes(&data).expect("walk");
        assert_eq!(nodes.len(), 3);
        assert_eq!(
            nodes.iter().map(|n| n.class_id).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert!(nodes.iter().all(|n| n.depth == 0));
        assert_eq!(nodes[1].data_size, 16);
        assert_eq!(nodes[1].payload().len(), 16);
    }

    /// A parent whose subtree is finished must stop counting as an ancestor.
    ///
    /// `root -> a -> b`, then a sibling of `root`. The sibling is at depth 0, and
    /// reporting it at depth 2 is what an implementation does if it retires
    /// finished subtrees after reading the depth instead of before. Nothing reads
    /// `depth` yet, which is the only reason this was survivable.
    #[test]
    fn depth_returns_to_zero_after_a_completed_subtree() {
        let data = build_tree(&[(1, 1, 0), (2, 1, 0), (3, 0, 0), (4, 0, 0)]);
        let depths: Vec<usize> = nodes(&data)
            .expect("walk")
            .iter()
            .map(|n| n.depth)
            .collect();
        assert_eq!(depths, [0, 1, 2, 0]);
    }

    #[test]
    fn depth_tracks_siblings_at_every_level() {
        // root
        //   a
        //     a1
        //     a2
        //   b
        let data = build_tree(&[(1, 2, 0), (2, 2, 0), (3, 0, 0), (4, 0, 0), (5, 0, 0)]);
        let depths: Vec<usize> = nodes(&data)
            .expect("walk")
            .iter()
            .map(|n| n.depth)
            .collect();
        assert_eq!(depths, [0, 1, 2, 2, 1]);
    }

    #[test]
    fn stops_at_a_node_that_runs_past_the_tree() {
        let mut data = build_tree(&[(1, 0, 0), (2, 0, 0)]);
        // Claim a payload far larger than the file for the second node.
        let second = FILE_HEADER_LEN + 0x20;
        data[second + 8..second + 12].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        let nodes = nodes(&data).expect("walk");
        assert_eq!(nodes.len(), 1, "the impossible node must not be reported");
    }

    #[test]
    fn stops_at_a_header_too_small_to_be_one() {
        let mut data = build_tree(&[(1, 0, 0), (2, 0, 0)]);
        let second = FILE_HEADER_LEN + 0x20;
        data[second + 4..second + 6].copy_from_slice(&8u16.to_le_bytes());
        assert_eq!(nodes(&data).expect("walk").len(), 1);
    }

    #[test]
    fn a_truncated_file_is_refused_rather_than_walked() {
        assert!(matches!(nodes(&[]), Err(Error::TooShort { .. })));
        assert!(matches!(nodes(&[0u8; 8]), Err(Error::TooShort { .. })));
    }

    /// A tree length larger than the file must not read past the end.
    #[test]
    fn a_lying_tree_length_is_clamped_to_the_file() {
        let mut data = build_tree(&[(1, 0, 0)]);
        data[4..8].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        let nodes = nodes(&data).expect("walk");
        assert_eq!(nodes.len(), 1);
    }

    /// The check that pins `child_count` to 16 bits: in a pre-order tree with
    /// immediate child counts, the counts sum to one less than the node count.
    /// The ground-truth test runs the same assertion against real files.
    #[test]
    fn the_child_counts_sum_to_one_root() {
        let data = build_tree(&[(1, 2, 0), (2, 2, 0), (3, 0, 0), (4, 0, 0), (5, 0, 0)]);
        let nodes = nodes(&data).expect("walk");
        let children: usize = nodes.iter().map(|n| n.child_count).sum();
        assert_eq!(nodes.len() - children, 1);
    }

    /// Reading `child_count` as a `u32` swallows the `u16` at `+0x0e`, which is
    /// non-zero on a few dozen nodes per real file. This is what that looks like.
    #[test]
    fn the_word_after_child_count_is_not_part_of_it() {
        let mut data = build_tree(&[(1, 1, 0), (2, 0, 0)]);
        let root = FILE_HEADER_LEN;
        data[root + 14..root + 16].copy_from_slice(&24u16.to_le_bytes());

        let nodes = nodes(&data).expect("walk");
        assert_eq!(nodes[0].child_count, 1, "a u32 read would give 1_572_865");
        assert_eq!(nodes[0].unk_0x0e, 24);
        assert_eq!(nodes[1].depth, 1, "and the depth stack would be corrupt");
        assert_eq!(nodes[1].parent, Some(0));
    }

    #[test]
    fn parents_follow_the_tree() {
        // root -> a -> a1, a2; root -> b
        let data = build_tree(&[(1, 2, 0), (2, 2, 0), (3, 0, 0), (4, 0, 0), (5, 0, 0)]);
        let nodes = nodes(&data).expect("walk");
        let parents: Vec<Option<usize>> = nodes.iter().map(|n| n.parent).collect();
        assert_eq!(parents, [None, Some(0), Some(1), Some(1), Some(0)]);
    }

    #[test]
    fn an_empty_transform_payload_is_the_identity() {
        assert_eq!(transform(&[]), Some(IDENTITY));
        assert_eq!(transform(&[0u8; 32]), None, "too short to be a matrix");
    }

    #[test]
    fn multiply_composes_in_row_vector_order() {
        let mut translate = IDENTITY;
        translate[12] = 10.0;
        let mut scale = IDENTITY;
        scale[0] = 2.0;
        scale[5] = 2.0;
        scale[10] = 2.0;

        // Scale first, then translate: the translation is not scaled.
        let m = multiply(&scale, &translate);
        assert_eq!(transform_point(&m, [1.0, 0.0, 0.0]), [12.0, 0.0, 0.0]);
        // Translate first, then scale: it is.
        let m = multiply(&translate, &scale);
        assert_eq!(transform_point(&m, [1.0, 0.0, 0.0]), [22.0, 0.0, 0.0]);
    }

    #[test]
    fn world_transforms_compose_down_the_chain() {
        // A transform tree three deep, each translating by 1 on x, with a mesh
        // at the bottom.
        let data = {
            let mut nodes = Vec::new();
            for _ in 0..3 {
                nodes.push((CLASS_TRANSFORM, 1usize, 64usize));
            }
            nodes.push((CLASS_MESH, 0, 0));
            let mut data = build_tree(&nodes);
            // Fill each transform payload with a translate-by-one matrix.
            let mut at = FILE_HEADER_LEN;
            for _ in 0..3 {
                let payload = at + 0x20;
                for (i, v) in IDENTITY.iter().enumerate() {
                    data[payload + i * 4..payload + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
                }
                data[payload + 12 * 4..payload + 12 * 4 + 4].copy_from_slice(&1.0f32.to_le_bytes());
                at = payload + 64;
            }
            data
        };

        let nodes = nodes(&data).expect("walk");
        assert_eq!(nodes.len(), 4);
        let world = world_transforms(&data, &nodes);
        assert_eq!(transform_point(&world[0], [0.0; 3]), [1.0, 0.0, 0.0]);
        assert_eq!(transform_point(&world[1], [0.0; 3]), [2.0, 0.0, 0.0]);
        assert_eq!(transform_point(&world[2], [0.0; 3]), [3.0, 0.0, 0.0]);
        // The mesh inherits its parent chain without contributing.
        assert_eq!(transform_point(&world[3], [0.0; 3]), [3.0, 0.0, 0.0]);
    }

    /// A texture this build cannot decode must hold its place, because materials
    /// name a texture by its ordinal. Dropping it renumbers every later one, and
    /// the symptom is a model wearing the wrong skins rather than an error.
    #[test]
    fn an_undecodable_texture_keeps_its_slot() {
        // Three Texture nodes, the middle one at an unsupported 16bpp. Each
        // carries a 16-byte payload declaring a 4-byte palette and 4 texels.
        let mut tree: Vec<u8> = Vec::new();
        for bpp in [8u8, 16, 8] {
            tree.extend(CLASS_TEXTURE.to_le_bytes());
            tree.extend(0x20u16.to_le_bytes());
            tree.extend(0u16.to_le_bytes());
            tree.extend(0x10u32.to_le_bytes()); // payload length
            tree.extend(0u32.to_le_bytes()); // no children
            tree.extend([0u8; 0x10]); // name area
            // payload: 1x4 pixels, the given depth, 4-byte clut, 4 texels
            tree.extend(1u16.to_le_bytes());
            tree.extend(4u16.to_le_bytes());
            tree.push(bpp);
            tree.push(1); // mip count
            tree.extend([0u8, 0]);
            tree.extend(4u32.to_le_bytes());
            tree.extend(4u32.to_le_bytes());
        }

        let mut data = Vec::new();
        data.extend(6u32.to_le_bytes());
        data.extend((tree.len() as u32).to_le_bytes());
        data.extend((3u32 * 8).to_le_bytes());
        data.extend(MAGIC);
        data.extend(tree);
        // Three palette-and-texel blocks, distinguishable by their first byte.
        for tag in [0x11u8, 0x22, 0x33] {
            data.extend([tag, 0, 0, 255]); // one palette entry
            data.extend([0u8; 4]); // texels
        }

        let slots = textures(&data).expect("textures");
        assert_eq!(slots.len(), 3, "one slot per Texture node");
        assert!(slots[1].is_none(), "16bpp is not decodable here");

        // The third texture must have read *its own* block, which only happens
        // if the undecodable one still advanced the cursor.
        let third = slots[2].as_ref().expect("third texture");
        assert_eq!(third.palette[0], [0x33, 0, 0, 255]);
        let first = slots[0].as_ref().expect("first texture");
        assert_eq!(first.palette[0], [0x11, 0, 0, 255]);
    }

    /// A track's `Texture` nodes use the short 32-byte header with the name
    /// field zeroed, so the node name is `None` and the only name the texture
    /// has is the runtime asset path at payload `+0x38`. Reading the header
    /// alone is why every track texture used to come back unnamed, and why the
    /// blink-light heuristic could never match track geometry.
    #[test]
    fn a_texture_names_itself_from_the_payload_when_the_header_does_not() {
        let path = br"Data\Environments\07_Track\Textures\07_Pulse_light_BLEND_GLOW.TGA";
        // 0x38 for the fixed fields, the path, and its NUL.
        let mut payload = vec![0u8; 0x38 + path.len() + 1];
        payload[0..2].copy_from_slice(&1u16.to_le_bytes()); // width
        payload[2..4].copy_from_slice(&4u16.to_le_bytes()); // height
        payload[4] = 8; // bits per pixel
        payload[5] = 1; // mip count
        payload[8..12].copy_from_slice(&4u32.to_le_bytes()); // clut_size
        payload[12..16].copy_from_slice(&4u32.to_le_bytes()); // texel_size
        payload[0x38..0x38 + path.len()].copy_from_slice(path);

        let mut tree: Vec<u8> = Vec::new();
        tree.extend(CLASS_TEXTURE.to_le_bytes());
        // The short header: no name area at all, which is the whole point.
        tree.extend(0x10u16.to_le_bytes());
        tree.extend(0u16.to_le_bytes());
        tree.extend((payload.len() as u32).to_le_bytes());
        tree.extend(0u32.to_le_bytes()); // no children
        tree.extend(&payload);

        let mut data = Vec::new();
        data.extend(6u32.to_le_bytes());
        data.extend((tree.len() as u32).to_le_bytes());
        data.extend(8u32.to_le_bytes()); // clut + texels
        data.extend(MAGIC);
        data.extend(tree);
        data.extend([0x44u8, 0, 0, 255]); // one palette entry
        data.extend([0u8; 4]); // texels

        let slots = textures(&data).expect("textures");
        let texture = slots[0].as_ref().expect("a decodable texture");
        assert_eq!(texture.name, None, "the header carries no name");
        assert_eq!(
            texture.asset_path.as_deref(),
            Some(std::str::from_utf8(path).expect("ascii")),
            "the payload does"
        );
    }

    /// PS2's `.vex` scenes declare a zero-length texture block: `Texture`
    /// nodes carry real dimensions and non-zero `clut_size`/`texel_size`
    /// fields, but no palette or texel bytes follow the tree at all. Reading
    /// them as if the bytes were there walks past the end of the file, which
    /// is what broke on a real PS2 disc image (`Data\Ships\Feisar\Ship.vex`,
    /// see HANDOVER.md). The header's own declared texture length is the
    /// signal to trust instead of the node's.
    #[test]
    fn a_zero_length_texture_block_returns_none_for_every_slot() {
        let mut tree: Vec<u8> = Vec::new();
        tree.extend(CLASS_TEXTURE.to_le_bytes());
        tree.extend(0x20u16.to_le_bytes());
        tree.extend(0u16.to_le_bytes());
        tree.extend(0x10u32.to_le_bytes()); // payload length
        tree.extend(0u32.to_le_bytes()); // no children
        tree.extend([0u8; 0x10]); // name area
        // A payload that looks exactly like a real, decodable 8bpp texture:
        // this is the point. Only the header's texture_len says otherwise.
        tree.extend(64u16.to_le_bytes());
        tree.extend(64u16.to_le_bytes());
        tree.push(8);
        tree.push(1);
        tree.extend([0u8, 0]);
        tree.extend(1024u32.to_le_bytes()); // clut_size
        tree.extend(4096u32.to_le_bytes()); // texel_size

        let mut data = Vec::new();
        data.extend(6u32.to_le_bytes());
        data.extend((tree.len() as u32).to_le_bytes());
        data.extend(0u32.to_le_bytes()); // texture block length: none
        data.extend(MAGIC);
        data.extend(tree);
        // No palette or texel bytes follow, matching the zero-length header.

        let slots = textures(&data).expect("a zero-length block is not an error");
        assert_eq!(slots.len(), 1, "one slot per Texture node");
        assert!(
            slots[0].is_none(),
            "no bytes to read, whatever the node claims"
        );
    }

    #[test]
    fn a_truncated_material_keeps_its_slot() {
        // material_count says two, but the payload only holds one.
        let mut payload = vec![0u8; 0x30 + 0x14];
        payload[2..4].copy_from_slice(&2u16.to_le_bytes());
        payload[0x30 + 4..0x30 + 8].copy_from_slice(&7u32.to_le_bytes());

        let materials = mesh_materials(&payload);
        assert_eq!(
            materials,
            vec![
                Some(Material {
                    flags: 0,
                    texture: 7,
                    second_texture: 0,
                }),
                None,
            ]
        );
    }

    /// All three fields come out of their documented offsets, and the tail the
    /// entry does not use is not mistaken for one of them.
    #[test]
    fn a_material_reads_flags_and_both_texture_indices() {
        let mut payload = vec![0u8; 0x30 + 0x14];
        payload[2..4].copy_from_slice(&1u16.to_le_bytes());
        payload[0x30..0x30 + 2].copy_from_slice(&0x2001u16.to_le_bytes());
        payload[0x30 + 4..0x30 + 8].copy_from_slice(&12u32.to_le_bytes());
        payload[0x30 + 8..0x30 + 12].copy_from_slice(&34u32.to_le_bytes());

        assert_eq!(
            mesh_materials(&payload),
            vec![Some(Material {
                flags: 0x2001,
                texture: 12,
                second_texture: 34,
            })]
        );
    }

    /// Every combination the game's stride calculator can reach.
    /// Values from the recovered layout table; see `docs/formats/vex.md`.
    #[test]
    fn derives_every_reachable_layout() {
        let cases: &[(u16, usize, usize)] = &[
            (0x100, 6, 0),
            (0x101, 8, 2),
            (0x103, 16, 8),
            (0x11c, 12, 4),
            (0x11d, 16, 8),
            (0x11f, 20, 12),
            (0x120, 10, 4),
            (0x121, 12, 6),
            (0x123, 20, 12),
            (0x13c, 16, 8),
            (0x13d, 20, 12),
            (0x13f, 24, 16),
            // 16-bit colour. `0x139` and `0x13b` are both observed in the
            // shipped tracks and both validated by the bounding-box check; the
            // three colour-only rows are the same rule applied, not sightings.
            (0x110, 8, 2),   // BGR5650
            (0x114, 8, 2),   // ABGR5551
            (0x118, 8, 2),   // ABGR4444
            (0x139, 14, 8),  // u8 texcoord, ABGR4444, s8 normal
            (0x13b, 20, 14), // f32 texcoord, ABGR4444, s8 normal
        ];

        for &(vertex_type, stride, position) in cases {
            let layout = VertexLayout::from_vertex_type(vertex_type)
                .unwrap_or_else(|e| panic!("{vertex_type:#06x}: {e}"));
            assert_eq!(layout.stride, stride, "stride for {vertex_type:#06x}");
            assert_eq!(layout.position, position, "position for {vertex_type:#06x}");
        }
    }

    #[test]
    fn identifies_present_components() {
        let full = VertexLayout::from_vertex_type(0x13f).unwrap();
        assert_eq!(full.texcoord, Some((0, TexcoordFormat::F32)));
        assert_eq!(full.colour, Some((8, ColourFormat::Abgr8888)));
        assert_eq!(full.normal, Some(12));
        assert_eq!(full.position, 16);

        let bare = VertexLayout::from_vertex_type(0x100).unwrap();
        assert_eq!(bare.texcoord, None);
        assert_eq!(bare.colour, None);
        assert_eq!(bare.normal, None);
    }

    /// 0x13b is the type that made a whole track refuse to decode: f32
    /// texcoords, ABGR4444 colour, s8 normals, s16 position.
    #[test]
    fn decodes_the_track_vertex_type() {
        let layout = VertexLayout::from_vertex_type(0x13b).expect("0x13b");
        assert_eq!(layout.texcoord, Some((0, TexcoordFormat::F32)));
        assert_eq!(layout.colour, Some((8, ColourFormat::Abgr4444)));
        assert_eq!(layout.normal, Some(10));
        assert_eq!(layout.position, 14);
        assert_eq!(layout.stride, 20);
    }

    /// Widening a 4-bit or 5-bit channel by shifting leaves white looking grey.
    #[test]
    fn sixteen_bit_colour_widens_to_full_range() {
        assert_eq!(ColourFormat::Abgr4444.to_rgba(0xffff), [255, 255, 255, 255]);
        assert_eq!(ColourFormat::Abgr4444.to_rgba(0x0000), [0, 0, 0, 0]);
        assert_eq!(ColourFormat::Abgr5551.to_rgba(0xffff), [255, 255, 255, 255]);
        assert_eq!(ColourFormat::Bgr5650.to_rgba(0xffff), [255, 255, 255, 255]);
        // Alpha is one bit in 5551, so it is either off or fully on.
        assert_eq!(ColourFormat::Abgr5551.to_rgba(0x7fff)[3], 0);
        // Channel order: red is the low bits in every ABGR format.
        assert_eq!(ColourFormat::Abgr4444.to_rgba(0x000f), [255, 0, 0, 0]);
        assert_eq!(ColourFormat::Abgr8888.to_rgba(0x0000_00ff), [255, 0, 0, 0]);
    }

    #[test]
    fn rejects_undefined_colour_formats() {
        // Bits 2-4 of 1, 2 and 3 are not GU colour formats.
        for vertex_type in [0x104u16, 0x108, 0x10c] {
            assert!(
                matches!(
                    VertexLayout::from_vertex_type(vertex_type),
                    Err(Error::UnsupportedVertexType { .. })
                ),
                "{vertex_type:#06x} should be refused"
            );
        }
    }

    #[test]
    fn rejects_u16_texcoords() {
        // The game's own calculator falls through this case without advancing
        // the offset. Guessing would shift every following field.
        assert!(matches!(
            VertexLayout::from_vertex_type(0x102),
            Err(Error::UnsupportedVertexType { .. })
        ));
    }

    #[test]
    fn rejects_non_s16_positions() {
        // The `+ 6` in the stride formula only holds for s16 positions.
        for vertex_type in [0x000, 0x080, 0x180] {
            assert!(
                matches!(
                    VertexLayout::from_vertex_type(vertex_type),
                    Err(Error::UnsupportedVertexType { .. })
                ),
                "{vertex_type:#06x} should be refused"
            );
        }
    }

    #[test]
    fn rejects_weights_and_morphs() {
        assert!(VertexLayout::from_vertex_type(0x0500).is_err());
    }

    #[test]
    fn scales_positions_into_model_units() {
        let layout = VertexLayout::from_vertex_type(0x100).unwrap();
        let mut data = vec![0u8; 16];
        // Full-scale positive, full-scale negative, zero.
        data[0..2].copy_from_slice(&i16::MAX.to_le_bytes());
        data[2..4].copy_from_slice(&(-32768i16).to_le_bytes());
        data[4..6].copy_from_slice(&0i16.to_le_bytes());

        let v = decode_vertex(&data, 0, &layout, 100.0);
        assert!((v.position[0] - 99.997).abs() < 0.01, "{:?}", v.position);
        assert!((v.position[1] + 100.0).abs() < 0.001, "{:?}", v.position);
        assert_eq!(v.position[2], 0.0);
    }

    #[test]
    fn a_missing_scale_would_be_obvious() {
        // Guards the mistake this format invites: forgetting the scale leaves
        // every model in the unit cube.
        let layout = VertexLayout::from_vertex_type(0x100).unwrap();
        let mut data = vec![0u8; 16];
        data[0..2].copy_from_slice(&16384i16.to_le_bytes());

        let scaled = decode_vertex(&data, 0, &layout, 250.0);
        assert!((scaled.position[0] - 125.0).abs() < 0.001);
    }

    /// Builds a PS2 batch: a 0x40 header, then the region header, DMA tag and
    /// VIF command stream a real one carries.
    ///
    /// Hand-built rather than captured, because a real batch is game data and
    /// cannot be committed. The shape is the one
    /// `crates/formats/tests/vex_ps2_ground_truth.rs` checks against every model
    /// on the disc; this is what keeps the same paths covered in CI, where there
    /// is no disc.
    fn ps2_batch(
        vertex_type: u16,
        primitive_type: u8,
        chunks: &[Vec<[f32; 3]>],
        with_normal: bool,
    ) -> Vec<u8> {
        let code = |command: u8, num: u8, immediate: u16| -> [u8; 4] {
            (u32::from(command) << 24 | u32::from(num) << 16 | u32::from(immediate)).to_le_bytes()
        };

        let mut stream = Vec::new();
        for positions in chunks {
            let count = u8::try_from(positions.len()).expect("fixture chunk fits in NUM");
            // The setup quadword, whose low 15 bits are the chunk's count.
            stream.extend(code(0x01, 0, 0x0101));
            stream.extend(code(0x6c, 1, 0xc000));
            stream.extend(u32::from(0x8000 | u16::from(count)).to_le_bytes());
            stream.extend([0u8; 12]);
            stream.extend(code(0x01, 0, 0x0104));
            // Colour, then texture coordinates, then position, then normal:
            // the order a real packet uses.
            stream.extend(code(0x6e, count, 0xc005));
            for i in 0..positions.len() {
                stream.extend([64, 64, 64, u8::try_from(i % 128).unwrap()]);
            }
            stream.extend(code(0x74, count, 0xc006));
            for (i, _) in positions.iter().enumerate() {
                stream.extend((i as f32).to_le_bytes());
                stream.extend((-(i as f32)).to_le_bytes());
            }
            stream.extend(code(0x78, count, 0xc004));
            for p in positions {
                for c in p {
                    stream.extend(c.to_le_bytes());
                }
            }
            if with_normal {
                stream.extend(code(0x78, count, 0xc007));
                for _ in positions {
                    stream.extend(0.0f32.to_le_bytes());
                    stream.extend(1.0f32.to_le_bytes());
                    stream.extend(0.0f32.to_le_bytes());
                }
            }
            stream.extend(code(0x17, 0, 0));
        }
        // The packet is the tag quadword plus the stream, which the tag's two
        // command words are already part of.
        while (stream.len() + 8) % 16 != 0 {
            stream.extend(code(0x00, 0, 0));
        }
        let packet_len = 16 + stream.len() - 8;
        let qwc = (packet_len - 16) / 16;

        let declared: usize = chunks.iter().map(Vec::len).sum::<usize>()
            - 2 * chunks.len().saturating_sub(1)
                * usize::from(primitive_type == PRIM_TRIANGLE_STRIP);

        let mut header = vec![0u8; 0x40];
        header[0..2].copy_from_slice(&1u16.to_le_bytes()); // pass_mask, list A
        header[4..6].copy_from_slice(&(declared as u16).to_le_bytes());
        header[8] = primitive_type;
        header[0x0a..0x0c].copy_from_slice(&vertex_type.to_le_bytes());
        header[0x0c..0x0e].copy_from_slice(&((16 + packet_len) as u16).to_le_bytes());
        header[0x10..0x14].copy_from_slice(&1.0f32.to_le_bytes());
        // The f32 bounding box, wide enough for the fixture's positions.
        for i in 0..3 {
            header[0x20 + i * 4..0x24 + i * 4].copy_from_slice(&(-1000.0f32).to_le_bytes());
            header[0x30 + i * 4..0x34 + i * 4].copy_from_slice(&1000.0f32.to_le_bytes());
        }

        let mut out = header;
        out.extend((packet_len as u32).to_le_bytes());
        out.extend(u32::from(vertex_type).to_le_bytes());
        out.extend(1u32.to_le_bytes()); // pass mask again
        out.extend(0u32.to_le_bytes());
        out.extend((0x6000_0000u32 | qwc as u32).to_le_bytes()); // DMA tag
        out.extend(0u32.to_le_bytes());
        out.extend(stream);
        out
    }

    /// Wraps batches into a mesh payload: the header, then batch list A.
    fn ps2_mesh(batches: &[Vec<u8>]) -> Vec<u8> {
        let mut payload = vec![0u8; 0x30];
        payload[4..8].copy_from_slice(&0x30u32.to_le_bytes()); // list A offset
        payload[8..12].copy_from_slice(&0x30u32.to_le_bytes());
        for batch in batches {
            payload.extend(batch);
        }
        // A terminator: a batch header whose pass mask has neither list bit.
        payload.extend(vec![0u8; 0x40]);
        payload
    }

    #[test]
    fn a_vif_batch_is_selected_by_its_position_width() {
        // The PSP's twelve types are not VIF batches; the PS2's eight are.
        for psp in [0x100u16, 0x121, 0x139, 0x13d, 0x13f] {
            assert!(!is_vif_batch(psp), "{psp:#06x}");
        }
        for ps2 in [0x181u16, 0x183, 0x199, 0x19b, 0x1a1, 0x1a3, 0x1b9, 0x1bb] {
            assert!(is_vif_batch(ps2), "{ps2:#06x}");
        }
    }

    #[test]
    fn decodes_a_ps2_batch_into_vertices() {
        let positions = vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        let payload = ps2_mesh(&[ps2_batch(
            0x1b9,
            PRIM_TRIANGLES,
            std::slice::from_ref(&positions),
            true,
        )]);
        let batches = mesh_batches(&payload, 0).expect("decodes");
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];

        assert_eq!(batch.vertex_type, 0x1b9);
        assert_eq!(batch.vertices.len(), 3);
        assert_eq!(batch.declared_vertex_count, 3);
        for (vertex, expected) in batch.vertices.iter().zip(&positions) {
            assert_eq!(vertex.position, *expected);
        }
        // The bounding box is the f32 pair at +0x20 and +0x30, not the PSP's
        // s16 pair scaled: reading the wrong one gives zeros here.
        assert_eq!(batch.bounds, ([-1000.0; 3], [1000.0; 3]));
        assert_eq!(batch.vertices[1].texcoord, Some([1.0, -1.0]));
        assert_eq!(batch.vertices[0].normal, Some([0.0, 1.0, 0.0]));
        // 64 of 128 is half intensity, which widens to half of 255.
        assert_eq!(batch.vertices[0].colour, Some([127, 127, 127, 0]));
    }

    /// A strip split across chunks repeats two vertices, so the decoded count
    /// exceeds the declared one. Concatenating them is what makes the repeats
    /// zero-area triangles rather than a hole.
    #[test]
    fn a_split_strip_keeps_both_chunks() {
        let first: Vec<[f32; 3]> = (0..6).map(|i| [i as f32, 0.0, 0.0]).collect();
        let second: Vec<[f32; 3]> = (4..9).map(|i| [i as f32, 0.0, 0.0]).collect();
        let payload = ps2_mesh(&[ps2_batch(
            0x199,
            PRIM_TRIANGLE_STRIP,
            &[first.clone(), second.clone()],
            false,
        )]);
        let batch = &mesh_batches(&payload, 0).expect("decodes")[0];

        assert_eq!(batch.vertices.len(), first.len() + second.len());
        assert_eq!(
            usize::from(batch.declared_vertex_count),
            first.len() + second.len() - 2,
            "the header counts the strip, not the repeats"
        );
        assert_eq!(batch.vertices[6].position, [4.0, 0.0, 0.0]);
        assert!(
            batch.vertices[0].normal.is_none(),
            "0x199 declares no normal, so none may be decoded"
        );
    }

    #[test]
    fn a_packet_whose_length_disagrees_with_the_batch_is_refused() {
        let mut payload = ps2_mesh(&[ps2_batch(
            0x1b9,
            PRIM_TRIANGLES,
            &[vec![[0.0, 0.0, 0.0]; 3]],
            true,
        )]);
        // Shorten the packet's own declared length by one quadword.
        let size = u32_at(&payload, 0x30 + 0x40) - 16;
        payload[0x30 + 0x40..0x30 + 0x44].copy_from_slice(&size.to_le_bytes());
        assert!(matches!(
            mesh_batches(&payload, 0),
            Err(Error::Packet { .. })
        ));
    }

    #[test]
    fn a_missing_normal_array_is_refused_when_the_type_declares_one() {
        // The one attribute whose presence the vertex type really does predict.
        let payload = ps2_mesh(&[ps2_batch(
            0x1b9,
            PRIM_TRIANGLES,
            &[vec![[0.0, 0.0, 0.0]; 3]],
            false,
        )]);
        assert!(matches!(
            mesh_batches(&payload, 0),
            Err(Error::Packet { .. })
        ));
    }

    #[test]
    fn colour_and_texcoords_are_taken_as_found_whatever_the_type_says() {
        // 0x181 declares no colour at all, and every real PS2 chunk carries one
        // anyway. Refusing it here would reject 930 batches on the disc.
        let payload = ps2_mesh(&[ps2_batch(
            0x181,
            PRIM_TRIANGLES,
            &[vec![[0.0, 0.0, 0.0]; 3]],
            false,
        )]);
        let batch = &mesh_batches(&payload, 0).expect("decodes")[0];
        assert!(batch.vertices[0].colour.is_some());
        assert!(batch.vertices[0].texcoord.is_some());
    }

    #[test]
    fn rejects_a_short_file() {
        assert_eq!(version(&[0u8; 4]), Err(Error::TooShort { got: 4 }));
        assert_eq!(nodes(&[0u8; 4]), Err(Error::TooShort { got: 4 }));
    }

    fn batch_with(primitive_type: u8, vertex_count: usize) -> Batch {
        Batch {
            pass_mask: 1,
            material_index: 0,
            primitive_type,
            vertex_type: 0x100,
            scale: 1.0,
            vertices: vec![
                Vertex {
                    position: [0.0; 3],
                    normal: None,
                    texcoord: None,
                    colour: None,
                };
                vertex_count
            ],
            declared_vertex_count: u16::try_from(vertex_count).unwrap_or(u16::MAX),
            bounds: ([0.0; 3], [0.0; 3]),
            header_flags: 0,
        }
    }

    #[test]
    fn expands_a_triangle_list() {
        let t = batch_with(PRIM_TRIANGLES, 9).triangles();
        assert_eq!(t, vec![[0, 1, 2], [3, 4, 5], [6, 7, 8]]);
    }

    #[test]
    fn a_triangle_list_ignores_a_trailing_partial_triangle() {
        assert_eq!(batch_with(PRIM_TRIANGLES, 8).triangles().len(), 2);
    }

    #[test]
    fn expands_a_strip_with_alternating_winding() {
        // Getting the flip wrong makes every other face point inwards, which
        // with culling on riddles the model with holes.
        let t = batch_with(PRIM_TRIANGLE_STRIP, 5).triangles();
        assert_eq!(t, vec![[0, 1, 2], [2, 1, 3], [2, 3, 4]]);
    }

    #[test]
    fn a_strip_shorter_than_a_triangle_yields_nothing() {
        assert!(batch_with(PRIM_TRIANGLE_STRIP, 2).triangles().is_empty());
        assert!(batch_with(PRIM_TRIANGLE_STRIP, 0).triangles().is_empty());
    }

    #[test]
    fn unsupported_primitives_yield_nothing_rather_than_garbage() {
        for prim in [0u8, 1, 2, 5, 6] {
            assert!(batch_with(prim, 12).triangles().is_empty(), "prim {prim}");
        }
    }

    #[test]
    fn classifies_pass_masks() {
        let batch = |pass_mask| Batch {
            pass_mask,
            material_index: 0,
            primitive_type: 3,
            vertex_type: 0x100,
            scale: 1.0,
            vertices: Vec::new(),
            declared_vertex_count: 0,
            bounds: ([0.0; 3], [0.0; 3]),
            header_flags: 0,
        };
        assert!(batch(0x0101).is_transparent());
        assert!(!batch(0x0021).is_transparent());
        assert!(batch(0x0801).is_alpha_tested());
        // `0x20` set is the `Gu_Disable(GU_CULL_FACE)` branch - two-sided - so
        // the batch that carries the bit is the one that is *not* culled.
        assert!(!batch(0x0021).is_culled());
        assert!(batch(0x0001).is_culled());

        // The three bits inside `0x0700`, and the branch order that separates
        // them: `Gfx_BuildBatchStateList` tests `0x100` first, then `0x200`,
        // then `0x400`, so a batch carrying more than one takes the earliest.
        assert_eq!(batch(0x0100).blend_class(), Some(BlendClass::AlphaOver));
        assert_eq!(batch(0x0200).blend_class(), Some(BlendClass::Additive));
        assert_eq!(batch(0x0400).blend_class(), Some(BlendClass::None));
        assert_eq!(batch(0x0300).blend_class(), Some(BlendClass::AlphaOver));
        assert_eq!(batch(0x0600).blend_class(), Some(BlendClass::Additive));
        assert_eq!(batch(0x0021).blend_class(), None);
        // The boost plume's own measured mask, all 32 batches across 8 teams.
        assert_eq!(batch(0x1232).blend_class(), Some(BlendClass::Additive));
    }

    #[test]
    fn decodes_the_additive_blend_flag() {
        let batch = |header_flags| Batch {
            pass_mask: 1,
            material_index: 0,
            primitive_type: 3,
            vertex_type: 0x100,
            scale: 1.0,
            vertices: Vec::new(),
            declared_vertex_count: 0,
            bounds: ([0.0; 3], [0.0; 3]),
            header_flags,
        };
        assert!(batch(0x00).is_additive_blend());
        assert!(!batch(0x10).is_additive_blend());
        // The extended-header bit (0x40) is independent of the blend bit.
        assert!(batch(0x40).is_additive_blend());
        assert!(!batch(0x50).is_additive_blend());
    }
}
