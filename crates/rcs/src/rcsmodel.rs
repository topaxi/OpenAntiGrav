//! `.rcsmodel`: where Wipeout HD keeps the geometry that used to be in the
//! `.vex`.
//!
//! On the PSP and PS2 a `oag_vex::vex`'s `Mesh` node's payload *is* its geometry.
//! On the PS3 that payload is a bounding-box pair and a 32-bit word, and the
//! vertices live in a `.rcsmodel` beside the `.vex` - 643 files and 686 MiB on
//! the HD disc, against 72 MiB of `.vex`. Nothing above this module could draw
//! an HD circuit or an HD craft until it was read.
//!
//! # What is decoded here, and what is not
//!
//! **Positions, triangle indices, vertex normals, texture coordinates and the
//! material table.** The normals are a packed 11:11:10 signed triple at `+6` of
//! every vertex, on all three strides - see [`unpack_normal`] and
//! [`Mesh::normals`]. The texture coordinate is the **last four** bytes of a
//! vertex, two big-endian halves, on every stride - [`Mesh::texcoords`]. The
//! material table names the `.gtf` each surface paints with and says which
//! surfaces are see-through - [`material`] and [`Blend`].
//!
//! What remains undecoded in a vertex is the four bytes at **`+0x0a`**, which
//! are a *tangent* on stride 22 and something else on stride 18. That split
//! follows the **stride**, not the `83 XX` descriptor byte, which was the
//! obvious hypothesis and is measured false; see [`SubMesh::format`].
//!
//! **The last four bytes are a coordinate in one of two types**, and which one
//! is in the file, in the chunk's own [`VertexDecl`] - see [`TexcoordFormat`]
//! and [`Mesh::texcoords`]. Reading every one as a half left 290 of a
//! circuit's 1,112 draw calls spanning over 100 tiles of their texture; reading
//! each submesh in its own type leaves 84.
//!
//! # The `.vex` is not optional
//!
//! This format cannot be decoded on its own, and that is a property of the
//! format rather than of this implementation:
//!
//! - **A chunk is addressed by hash.** The 32-bit word at a `Mesh` node's
//!   `+0x30` is the chunk's own first word. Without the `.vex` you have
//!   geometry with no idea which node - and therefore which world transform -
//!   it belongs to.
//! - **The vertex stride is not in the file.** See [`Mesh::solve_stride`]: the
//!   tightest oracle for it is the authored bounding box the `.vex` node
//!   carries, the same box `docs/formats/hd-status.md` measured `min <= max` on
//!   across 1,638 of 1,638 nodes. A chunk no node references has no box, and
//!   [`Mesh::solve_stride_without_a_box`] is what those go through.
//!
//! So the entry point is [`Model::parse`] for the container and
//! [`Mesh::solve_stride`] per node, with the box supplied by the caller.
//!
//! # Layout
//!
//! Big-endian throughout. Unlike the `.vex`, **there is no magic and no byte
//! order to sniff** - the version word `0x000a0000` is the only signature, and
//! it is checked rather than assumed.
//!
//! ```text
//! +0x00  u32   version, 0x000a0000 on every file on the disc
//! +0x04  u32   offset of the relocation table `{count, offsets[]}`, which
//!              sits exactly where the directory ends - see [`render_block`]
//! +0x08  u32   0xffffffff
//! +0x1c  u32   mesh count
//! +0x20  u32   offset of the mesh offset table: `count` big-endian u32s
//! +0x24  u32   offset of the file-wide bounds block
//! +0x28  u32   offset of the string pool (material and texture paths)
//! +0x2c  u32   material count
//! +0x30  u32   offset of the material offset table
//! ```
//!
//! One mesh chunk, at an offset the table gives:
//!
//! ```text
//! +0x00  u32     hash, matching the `.vex` Mesh node's own +0x30 word
//! +0x08  u32     offset of the chunk's 0x40-byte render-block record, whose
//!                +0x06 halfword is [`Mesh::render_flags`] - see [`render_block`]
//! +0x20  u32     index into the material table - see [`material`]
//! +0x30  f32[3]  position bias, in the node's own space
//! +0x40  f32[3]  position scale - 1/128 on every file measured
//! +0x50  u32     submesh count
//! +0x60  ...     submesh descriptors, 0x80 bytes each
//! ```
//!
//! One submesh descriptor:
//!
//! ```text
//! +0x00  u8[8]   vertex-format descriptor, `83 XX 10 10 10 10 10 00`.
//!                **Neither the stride nor the field set** - see `SubMesh::format`.
//! +0x08  u16     vertex count
//! +0x0a  u16     index count, divisible by 3 on 50,873 of 50,873 submeshes
//! +0x10  u32     index buffer offset: `count` big-endian u16s
//! +0x18  u32     vertex buffer offset
//! ```
//!
//! Full evidence, with confidence scores, on
//! [`rcsmodel.md`](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/rcsmodel.md).

use oag_formats::ByteOrder;

mod coverage;
mod inline_uv;
pub mod material;
pub mod psp2;
mod render_block;
mod stride;
mod surface;

pub use coverage::coverage;
pub use render_block::{RENDER_BLOCK_LEN, RENDER_TRACK};
pub use surface::Space;
pub mod vertex_decl;

pub use material::{Blend, Factor, Material, Transparency};
pub use vertex_decl::{Attribute, VertexDecl};

/// The version word every `.rcsmodel` on the HD disc opens with.
///
/// Checked rather than assumed, because this container carries no magic: a
/// caller that hands it the wrong blob would otherwise read a count out of
/// whatever was there and walk the file on it.
pub const VERSION: u32 = 0x000a_0000;

/// Bytes of header before the offsets this module reads.
const HEADER_LEN: usize = 0x40;

/// Bytes per submesh descriptor.
const SUBMESH_LEN: usize = 0x80;

/// First submesh descriptor, relative to its chunk.
///
/// Which is `SURFACE_BASE + SURFACE_LEN`, because the descriptors belong to
/// the *surface* rather than to the chunk. See [`SURFACE_BASE`].
const SUBMESH_BASE: usize = 0x60;

/// The byte that says which space a chunk's positions are in.
///
/// **The `kk` of the `00 nn LL kk` word at `+0x04`**, which this module carried
/// for months as "takes the values `01` and `02` for a reason nothing here has
/// distinguished". The engine branches on it, and the disc says what it means:
/// see [`Space`].
const SPACE_BYTE: usize = 0x07;

/// Where a chunk's first surface record starts, relative to the chunk.
///
/// **A chunk header is 0x20 bytes and then a surface record**, and this is the
/// finding that turned a quarter of the disc's geometry from missing into
/// drawn. Every field this module used to call a chunk field - the material at
/// `+0x20`, the bias at `+0x30`, the scale at `+0x40`, the submesh count at
/// `+0x50`, the vertex declaration at `+0x58` and the descriptors at `+0x60` -
/// is at `+0x00`, `+0x10`, `+0x20`, `+0x30`, `+0x38` and `+0x40` of a record
/// that recurs elsewhere in the file for every surface past the first.
/// Confirmed structurally: the first entry of every chunk's surface table
/// points at `chunk + 0x20` on **all 41,861 chunks on the disc**.
///
/// See `docs/formats/rcsmodel.md`, "A chunk names one material here and the
/// engine reads several".
const SURFACE_BASE: usize = 0x20;

/// Bytes of one surface record, before its own submesh descriptors.
const SURFACE_LEN: usize = 0x40;

/// A chunk's surface count, relative to the chunk. A `u16`.
const SURFACE_COUNT: usize = 0x10;

/// A chunk's table of surface-record offsets, relative to the chunk.
const SURFACE_TABLE: usize = 0x18;

/// Bytes of a material record this module reads.
///
/// Not the record's own length, which varies from 96 to 768 bytes across the
/// disc: it is how far in the last field [`Material`] decodes reaches.
const MATERIAL_LEN: usize = 0x18;

/// Bytes of position in a vertex: three big-endian `i16`s.
const POSITION_LEN: usize = 6;

/// Bytes of texture coordinate at the **end** of a vertex: two big-endian
/// halves.
///
/// Measured from the end rather than the start because that is what holds
/// across all three strides - see [`Mesh::texcoords`].
const TEXCOORD_LEN: usize = 4;

/// A chunk whose submeshes are a table of `0x80`-byte descriptors at `+0x60`.
///
/// **Byte `+0x06` of a chunk.** The `u32` it sits in reads `00 nn LL kk`: `LL`
/// is this and `kk` is [`Space`]. **`nn` is read by nothing**: the one
/// instruction in the whole geometry path that loads `+0x05` is an unrolled
/// byte-by-byte block copy that carries `+0x04` through `+0x08` alike, so it
/// is not a semantic read - which closes the old guess that it counts chunks,
/// as a clean negative at confidence 85. See [`LAYOUT_INLINE`].
const LAYOUT_DESCRIBED: u8 = 0x05;

/// A chunk that names one buffer pair in its own header and has no descriptor
/// table at all.
///
/// **This is what 224 of the disc's 643 models are made of**, and reading them
/// as [`LAYOUT_DESCRIBED`] is what used to fail: the word at `+0x50` is a
/// file-wide pointer there, not a submesh count, so it read as hundreds and put
/// the descriptors past the end of the file. The fields instead are
///
/// ```text
/// +0x54  u32   vertex buffer offset
/// +0x58  u32   index count
/// +0x5c  u32   index buffer offset
/// +0x6c  u16   vertex count
/// ```
///
/// with exactly one submesh per chunk. Confidence 88: it is settled by the
/// index-range invariant, which holds on every submesh of all 643 files once
/// this layout is read - and would not survive a wrong field on any of them.
const LAYOUT_INLINE: u8 = 0x01;

/// The strides the searches will consider, in bytes.
///
/// **The set the disc's own vertex declarations carry, and nothing else.** A
/// declaration states the stride outright (`vertex_decl`), so it is the oracle
/// rather than a guess: over all 643 models the declarations carry exactly
/// seven widths - 18 on 38,060 chunks, 14 on 12,616, 22 on 9,423, 10 on 941,
/// 26 on 755, 38 on 208 and 34 on 23.
///
/// **This was `[14, 18, 22]`, which is three of the seven.** Those came from
/// the widths an authored bounding box had settled on back when no declaration
/// was read; when `vertex_decl` was decoded it showed four more and this
/// constant was not revisited. The cost was structural - a chunk of one of the
/// missing four *with no declaration of its own* could not be solved by any
/// search, because the answer was not on the ballot. Widening it took the
/// surfaces no rule can decode from 371 to 115 and the disc's read coverage
/// from 96.06% to 97.69%.
///
/// Searching every even width from 6 to 64 instead, as this did first, is
/// still strictly worse: 814 of `talons_junction`'s 983 chunks choose one of
/// the original three anyway, and the rest choose a width no measurement
/// supports and decode to spikes radiating out of the level. **The rule is not
/// "search wider", it is "search exactly what the data declares".**
///
/// Pinned by `crates/formats/tests/rcsmodel_stride_ground_truth.rs`, which
/// asserts that every declared width is one the searches may answer.
pub const STRIDES: &[usize] = &[10, 14, 18, 22, 26, 34, 38];

/// How much of a chunk must decode to a unit normal for a stride to win.
///
/// See [`Mesh::solve_stride_by_normals`]. A wrong stride scores about 0.3 by
/// chance, and the right one 0.9 or better on most chunks; this sits below the
/// cluster the right answers form and well above the noise.
const NORMAL_UNIT_BAR: f32 = 0.8;

/// How far the winning stride must beat the runner-up on that fraction.
const NORMAL_MARGIN_BAR: f32 = 0.3;

/// Fewest vertices the normal rule will judge a chunk on.
///
/// **The bars above are fractions, and a fraction of eight is not evidence.**
/// Talon's Junction's furthest-flung chunks are eight-vertex cards where 7 of 8
/// unit reads clear a 0.8 bar by luck; requiring a real sample is what keeps
/// them out. See [`Mesh::solve_stride_by_normals`].
const NORMAL_MIN_VERTICES: usize = 32;

/// Byte offset of the packed vertex normal, immediately after the position.
///
/// **The same on all three strides**, which is what says the widths are one
/// layout with optional fields rather than three formats. See
/// [`Mesh::normals`].
pub const NORMAL_OFFSET: usize = POSITION_LEN;

/// Turns the 32-bit word at [`NORMAL_OFFSET`] into a unit vector.
///
/// **11 bits of x, 11 of y, 10 of z**, each signed two's-complement, packed
/// low-to-high in a big-endian `u32`. The odd split is what the recovery turned
/// on: a planar submesh whose faces all point along `+x` carries the constant
/// word `0x000003ff`, which is `x = 1023` and nothing else only under an
/// 11-bit low field, and one pointing along `-y` carries `0x00200800`, which is
/// `y = -1023` and nothing else only under an 11-bit field starting at bit 11.
///
/// The result is a unit vector on **99.9 %** of 23,608 vertices, and no other
/// reading of any offset in the vertex exceeds 51 %.
#[must_use]
pub fn unpack_normal(word: u32) -> [f32; 3] {
    let signed = |raw: u32, bits: u32| {
        let mask = (1 << bits) - 1;
        let half = 1 << (bits - 1);
        let v = (raw & mask) as i32;
        let v = if v >= half { v - (1 << bits) } else { v };
        v as f32 / (half - 1) as f32
    };
    [
        signed(word, 11),
        signed(word >> 11, 11),
        signed(word >> 22, 10),
    ]
}

/// Turns a big-endian IEEE half into an `f32`.
///
/// Written out rather than pulled in: Rust's own `f16` is unstable, and the two
/// places this project needs one (here and a `.gtf` descriptor) do not justify a
/// dependency. Subnormals and zero are handled by the `exp == 0` arm, and
/// infinities and NaN come out as themselves so a caller can *see* a field that
/// is not a half rather than have it silently clamped - which is what
/// [`Mesh::texcoords`]' non-finite count is measuring.
#[must_use]
pub fn unpack_half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f32::from(bits & 0x03ff) / 1024.0;
    sign * match exponent {
        0 => fraction * SUBNORMAL_SCALE,
        31 if fraction == 0.0 => f32::INFINITY,
        31 => f32::NAN,
        _ => (1.0 + fraction) * exp2(exponent - 15),
    }
}

/// `2^-14`, the smallest normal half, written as a literal because
/// `f32::powi` is a platform transcendental this crate's rules keep out.
const SUBNORMAL_SCALE: f32 = 6.103_515_6e-5;

/// `2^n` for the exponent range a half can hold, by bit pattern.
///
/// Exact, and no `powf` - the exponent of an `f32` is bits 23..31 biased by 127,
/// and a half's `1..30` maps inside that range with room to spare.
fn exp2(n: i32) -> f32 {
    f32::from_bits(((n + 127) as u32) << 23)
}

/// The two types a `.rcsmodel` writes a texture coordinate in.
///
/// **The file says which**, in the type byte of the chunk's own
/// [`VertexDecl`] - `0x23` for a pair of halves on 54,120 attributes disc-wide
/// and `0x22` for a pair of `f32` on 230. See
/// [`vertex_decl`] and `docs/formats/rcsmodel.md`.
///
/// This used to be selected per submesh by sniffing what the bytes decoded to,
/// with `Unorm16` as its second member. That reading is retired: what the sniff
/// was separating is not a second coordinate type but a **different
/// attribute** - a four-byte `tangent` or `colorSet1` in the last four bytes of
/// a vertex whose coordinate is elsewhere, read as a coordinate because the
/// reader assumed the coordinate came last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexcoordFormat {
    /// Two big-endian IEEE halves - see [`unpack_half`]. Declared `0x23`.
    Half,
    /// Two big-endian `f32`. Declared `0x22`.
    Float,
}

impl TexcoordFormat {
    /// The type a declared attribute is written in, or `None` for one that is
    /// not a two-component coordinate.
    #[must_use]
    pub fn of(attribute: &Attribute) -> Option<Self> {
        if attribute.components != 2 {
            return None;
        }
        match attribute.rsx_type {
            vertex_decl::RSX_HALF => Some(Self::Half),
            vertex_decl::RSX_FLOAT => Some(Self::Float),
            _ => None,
        }
    }

    /// Bytes one coordinate pair occupies.
    #[must_use]
    pub fn width(self) -> usize {
        match self {
            Self::Half => 4,
            Self::Float => 8,
        }
    }

    /// One pair, at `at`.
    #[must_use]
    pub fn decode(self, data: &[u8], at: usize) -> [f32; 2] {
        match self {
            Self::Half => {
                std::array::from_fn(|i| unpack_half(ByteOrder::Big.u16(data, at + i * 2)))
            }
            Self::Float => std::array::from_fn(|i| ByteOrder::Big.f32(data, at + i * 4)),
        }
    }
}

/// Everything that can go wrong reading one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not enough bytes for the header.
    TooShort {
        /// What was supplied.
        got: usize,
    },
    /// The version word is not [`VERSION`].
    BadVersion {
        /// What the file opened with.
        got: u32,
    },
    /// A table or buffer the header points at is not inside the file.
    OutOfBounds {
        /// Which structure was being read.
        what: &'static str,
        /// The byte it wanted.
        end: usize,
        /// The file length.
        len: usize,
    },
    /// A chunk's `+0x04` word is neither of the two known layouts.
    ///
    /// Reported rather than assumed, because the two differ in where the
    /// submesh buffers are named and reading one as the other produces
    /// plausible-looking nonsense - see [`LAYOUT_INLINE`].
    UnknownChunkLayout {
        /// The byte at `+0x06`.
        got: u8,
        /// Where the chunk starts.
        at: usize,
    },
    /// The chunk declares no texture coordinate at all.
    ///
    /// 984 of the disc's chunks. Answered rather than substituted for: a
    /// caller that draws these untextured is showing an absence, and one that
    /// invents a coordinate is showing a wrong picture. See
    /// [`VertexDecl::diffuse_texcoord`].
    NoTexcoord,
    /// An index buffer names a vertex the submesh does not have.
    ///
    /// A real corruption check rather than a formality: it is the invariant
    /// that says the index buffer offset and count were read correctly, and it
    /// holds on all 1,274 submeshes of the three models measured.
    IndexOutOfRange {
        /// The largest index found.
        index: usize,
        /// How many vertices the submesh declares.
        vertices: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "a .rcsmodel needs {HEADER_LEN} bytes, got {got}"),
            Self::BadVersion { got } => {
                write!(f, "version {got:#010x}, expected {VERSION:#010x}")
            }
            Self::OutOfBounds { what, end, len } => {
                write!(f, "{what} ends at {end} but the file is {len} bytes")
            }
            Self::NoTexcoord => {
                write!(f, "the chunk declares no texture coordinate")
            }
            Self::IndexOutOfRange { index, vertices } => {
                write!(f, "an index names vertex {index} of {vertices}")
            }
            Self::UnknownChunkLayout { got, at } => write!(
                f,
                "the chunk at {at:#x} declares layout {got:#04x}, \
                 which is neither {LAYOUT_DESCRIBED:#04x} nor {LAYOUT_INLINE:#04x}"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Shorthand for this module's results.
pub type Result<T> = std::result::Result<T, Error>;

/// An axis-aligned box, as a `(min, max)` pair.
///
/// The shape a `.vex` `Mesh` node authors, and the oracle
/// [`Mesh::solve_stride`] reads the vertex stride out of.
pub type Bounds = ([f32; 3], [f32; 3]);

/// A `.rcsmodel`, parsed as far as its mesh directory.
///
/// Holds no vertex data: the buffers are read straight out of the caller's
/// slice on demand, so opening a 24 MiB circuit costs a table walk.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// One entry per chunk the offset table names, in table order.
    pub meshes: Vec<Mesh>,
    /// One entry per material the header's own table names, in table order.
    ///
    /// [`Mesh::material`] indexes this. See [`material`] for what a record
    /// holds and for why nothing draws through it yet.
    pub materials: Vec<Material>,
}

/// One mesh chunk: the geometry belonging to one `.vex` `Mesh` node.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// The word that ties this chunk to a `.vex` node's payload `+0x30`.
    pub hash: u32,
    /// Added to every dequantised position.
    pub bias: [f32; 3],
    /// Multiplies every position component. `1/128` on every file measured.
    pub scale: [f32; 3],
    /// Index into [`Model::materials`], from the chunk's `+0x20`.
    ///
    /// **Per chunk, not per submesh** - the `0x80`-byte submesh descriptor
    /// carries no field whose values all fall below the file's material count,
    /// and the chunk's does on every chunk of every model measured. Confirmed by
    /// name rather than by range alone: on Assegai `WindscreenShape` resolves
    /// through it to `glass_texture_n`, `cockpit_screenShape` to `screen_test`,
    /// `FlashybitsShape` to `emissive_bloom` and `ShipShape` to
    /// `diffuse_with_specular_from_alpha_n_vcol`.
    ///
    /// Out of range on a file whose material table this reader could not walk;
    /// [`Model::material_of`] answers `None` there rather than panicking.
    pub material: u32,
    /// Which shape this chunk's header takes, out of its `+0x06` byte.
    pub layout: Layout,
    /// The draw calls this mesh is split into.
    pub submeshes: Vec<SubMesh>,
    /// What the chunk says its vertices are made of, out of the word at
    /// `+0x58`.
    ///
    /// `Some` on every [`LAYOUT_DESCRIBED`] chunk of every model on the disc,
    /// and `None` on a [`LAYOUT_INLINE`] one, where that word is the index
    /// count instead. See [`vertex_decl`] - it is the file stating the stride
    /// and the offset of each attribute, both of which this module used to
    /// solve for.
    pub decl: Option<VertexDecl>,
    /// Which space this chunk's positions are in, out of its [`SPACE_BYTE`].
    ///
    /// Shared by every surface of a chunk, like [`Self::layout`] - the byte is
    /// in the chunk header, outside the surface record.
    pub space: Space,
    /// The authored flags halfword of the chunk's render-block record, the
    /// `+0x06` of the 0x40-byte record the header's `+0x08` word names.
    ///
    /// **Not the layout byte**, which is also a `+0x06` - of the chunk
    /// header, not of this record. Bit 0 is [`RENDER_TRACK`]; see
    /// [`Self::is_track`] and [`render_block`] for the rest. Zero on the
    /// 35,913 chunks that author nothing, and on a chunk whose word is zero.
    /// Shared by every surface of a chunk, like [`Self::space`].
    pub render_flags: u16,
    /// The chunk's surfaces past this one, each a `Mesh` in its own right.
    ///
    /// Empty on the 76 % of chunks that declare a single surface, and on every
    /// element of this list itself - a surface has no surfaces. Walk it through
    /// [`Self::surfaces`] rather than directly, so the first one is not missed.
    pub extra_surfaces: Vec<Mesh>,
}

/// Which of the two shapes a chunk's header takes, out of its `+0x06` byte.
///
/// **The field that decides what several later words mean**, and the reason
/// this is on [`Mesh`] rather than kept private: `+0x58` is a pointer to a
/// [`vertex_decl::VertexDecl`] on one and an index count on the other, so a
/// reader that does not know which it is holding reads a declaration out of
/// noise. See [`LAYOUT_DESCRIBED`] and [`LAYOUT_INLINE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// [`LAYOUT_DESCRIBED`]: a table of `0x80`-byte submesh descriptors at
    /// `+0x60`. 39,372 of the disc's 41,861 chunks.
    Described,
    /// [`LAYOUT_INLINE`]: one buffer pair named in the chunk header itself.
    /// 2,489 chunks.
    Inline,
}

/// One draw call's worth of geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubMesh {
    /// The eight-byte word a descriptor opens with, `83 XX 10 10 10 10 10 00`.
    ///
    /// **It is not the stride and it is not the field set either**, which are
    /// the two things it looks like it ought to be and the two that have been
    /// measured. `XX` runs `07` to `0d`; `docs/formats/rcsmodel.md` has the
    /// cross-tabulations. Kept because it is the only part of the descriptor
    /// that varies and is not accounted for, so the next reading starts here.
    pub format: [u8; 8],
    /// How many vertices its buffer holds.
    pub vertex_count: usize,
    /// Byte offset of the vertex buffer, from the start of the file.
    pub vertex_offset: usize,
    /// How many indices: always a multiple of three.
    pub index_count: usize,
    /// Byte offset of the index buffer, from the start of the file.
    pub index_offset: usize,
}

/// Reads `count` big-endian `u32`s at `at`, or says what fell off the end.
fn table(data: &[u8], at: usize, count: usize, what: &'static str) -> Result<Vec<u32>> {
    let end = at + count * 4;
    if end > data.len() {
        return Err(Error::OutOfBounds {
            what,
            end,
            len: data.len(),
        });
    }
    Ok((0..count)
        .map(|i| ByteOrder::Big.u32(data, at + i * 4))
        .collect())
}

impl Model {
    /// Reads the header and the mesh directory.
    ///
    /// # Errors
    ///
    /// [`Error::TooShort`] and [`Error::BadVersion`] for a blob that is not one
    /// of these, and [`Error::OutOfBounds`] for a table that leaves the file -
    /// which is what a wrongly-read count looks like.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        let version = ByteOrder::Big.u32(data, 0);
        if version != VERSION {
            return Err(Error::BadVersion { got: version });
        }

        let count = ByteOrder::Big.u32(data, 0x1c) as usize;
        let offsets = table(
            data,
            ByteOrder::Big.u32(data, 0x20) as usize,
            count,
            "the mesh offset table",
        )?;

        let meshes = offsets
            .into_iter()
            .map(|at| Mesh::parse(data, at as usize))
            .collect::<Result<Vec<_>>>()?;

        let materials = table(
            data,
            ByteOrder::Big.u32(data, 0x30) as usize,
            ByteOrder::Big.u32(data, 0x2c) as usize,
            "the material offset table",
        )?
        .into_iter()
        .map(|at| {
            let at = at as usize;
            Material::parse(data, at).ok_or(Error::OutOfBounds {
                what: "a material record",
                end: at + MATERIAL_LEN,
                len: data.len(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

        Ok(Self { meshes, materials })
    }

    /// The material a chunk points at, or `None` when its index is out of
    /// range.
    #[must_use]
    pub fn material_of(&self, mesh: &Mesh) -> Option<&Material> {
        self.materials.get(mesh.material as usize)
    }

    /// The chunk carrying a `.vex` `Mesh` node's `+0x30` word, if this file has
    /// one.
    ///
    /// **Linear, and that is measured rather than lazy**: the largest model on
    /// the disc has 983 chunks, and a race resolves each node once at load. A
    /// map would be worth building if something looked a chunk up per frame,
    /// and nothing does.
    #[must_use]
    pub fn mesh(&self, hash: u32) -> Option<&Mesh> {
        self.meshes.iter().find(|mesh| mesh.hash == hash)
    }

    /// Where the chunk with this hash sits in [`Self::meshes`].
    ///
    /// File order is the index `track.pvs` addresses a chunk by - see
    /// [`crate::hd_pvs`] - so a caller that has a hash and needs a PVS bit
    /// needs this rather than the chunk itself.
    #[must_use]
    pub fn mesh_index(&self, hash: u32) -> Option<usize> {
        self.meshes.iter().position(|mesh| mesh.hash == hash)
    }
}

impl Mesh {
    /// Dequantises one submesh's positions at a given stride.
    ///
    /// The positions are in the `.vex` node's own space, so a caller still
    /// applies that node's world transform - exactly as it does for a PSP mesh.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfBounds`] when the buffer leaves the file, which is what a
    /// stride that is too large looks like.
    pub fn positions(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Vec<[f32; 3]>> {
        let end =
            submesh.vertex_offset + stride * submesh.vertex_count.saturating_sub(1) + POSITION_LEN;
        if submesh.vertex_count > 0 && end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a vertex buffer",
                end,
                len: data.len(),
            });
        }
        Ok((0..submesh.vertex_count)
            .map(|k| {
                let at = submesh.vertex_offset + k * stride;
                std::array::from_fn(|i| {
                    let q = f32::from(ByteOrder::Big.i16(data, at + i * 2));
                    self.bias[i] + q * self.scale[i]
                })
            })
            .collect())
    }

    /// The authored vertex normals, one per vertex, in the mesh's own space.
    ///
    /// # How this was found, and how much of the vertex it settles
    ///
    /// The attribute bytes after a position were undecoded until 2026-08-17,
    /// and the thing that decoded them is a **planar submesh**: one whose faces
    /// all point the same way, so the encoding of that direction is whatever is
    /// constant across its vertex records and can be read off rather than
    /// searched for. Two of Talon's Junction's roads supplied the two axes that
    /// pin the bit split - see [`unpack_normal`].
    ///
    /// Checked against geometry the file does not state: the **area-weighted
    /// average of the faces touching each vertex**, over every mesh of Assegai
    /// more than a unit across. 82.3 % of 23,241 vertices land within 18
    /// degrees, where the best reading of any other offset reaches 14.6 %. The
    /// remaining 18 % are what a hard edge looks like - the exporter splits the
    /// vertex and authors a normal the smooth average does not have, which is
    /// the whole reason a model stores normals instead of computing them.
    ///
    /// The rest of a vertex is **not** decoded here, and only one part of it is
    /// even identified. The last four bytes read as two `f16` in a
    /// texture-coordinate range on every stride, but **a texture coordinate has
    /// no oracle** until `.gtf` is read, so that is located rather than
    /// confirmed and nothing consumes it. The four bytes at `+10` are a *unit*
    /// field perpendicular to this normal on **every** stride-22 submesh - a
    /// tangent, median `|dot|` 0.007 to 0.008 over 78,440 vertices - and on
    /// every stride-18 one they are not, at 0.52 to 0.57. What they hold in
    /// general is unrecovered; the measurement is in
    /// `the_field_after_the_normal_follows_the_stride_and_not_the_descriptor_byte`.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfBounds`] if the buffer runs past the file, as
    /// [`Self::positions`].
    pub fn normals(&self, data: &[u8], submesh: &SubMesh, stride: usize) -> Result<Vec<[f32; 3]>> {
        let end = submesh.vertex_offset
            + stride * submesh.vertex_count.saturating_sub(1)
            + NORMAL_OFFSET
            + 4;
        if submesh.vertex_count > 0 && end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a vertex buffer's normals",
                end,
                len: data.len(),
            });
        }
        Ok((0..submesh.vertex_count)
            .map(|k| {
                let at = submesh.vertex_offset + k * stride + NORMAL_OFFSET;
                unpack_normal(ByteOrder::Big.u32(data, at))
            })
            .collect())
    }

    /// Bytes per vertex, as the chunk's own declaration states it.
    ///
    /// **Prefer this over [`Self::solve_stride`] and its siblings**, which fit
    /// the stride to the authored bounding box because this block had not been
    /// read: the declaration agrees with the search on 35,983 of the 35,990
    /// chunks the search settles, and settles 3,382 more that it does not. The
    /// searches remain for [`Layout::Inline`] chunks, which declare nothing.
    #[must_use]
    pub fn declared_stride(&self) -> Option<usize> {
        self.decl.as_ref().map(|decl| decl.stride)
    }

    /// The texture coordinates a diffuse texture is sampled through, one pair
    /// per vertex.
    ///
    /// # Where they are is declared, not assumed
    ///
    /// The chunk's [`VertexDecl`] gives the attribute's byte offset and its
    /// type, and [`VertexDecl::diffuse_texcoord`] picks which of the several a
    /// vertex may declare. That matters because **the last four bytes of a
    /// vertex are usually not it**: on the commonest stride-18 layout they are
    /// `lightmapUV`, whose coordinates are atlas-packed, and painting a diffuse
    /// texture through them smears it into streaks.
    ///
    /// A [`Layout::Inline`] chunk declares nothing, so there the last four
    /// bytes are read as two halves - the reading this method used everywhere
    /// before the declaration was found, kept for the 2,489 chunks that still
    /// have no better answer.
    ///
    /// # Errors
    ///
    /// [`Error::NoTexcoord`] for a chunk that declares no texture coordinate at
    /// all, which 984 of the disc's do - answered rather than substituted for,
    /// so a caller draws an untextured surface instead of a wrongly-mapped one.
    /// [`Error::OutOfBounds`] as [`Self::positions`].
    pub fn texcoords(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Vec<[f32; 2]>> {
        let (offset, format) = match &self.decl {
            Some(decl) => {
                let attribute = decl.diffuse_texcoord().ok_or(Error::NoTexcoord)?;
                let format = TexcoordFormat::of(attribute).ok_or(Error::NoTexcoord)?;
                (usize::from(attribute.offset), format)
            }
            None => (stride.saturating_sub(TEXCOORD_LEN), TexcoordFormat::Half),
        };
        let coords = self.coords_at(data, submesh, stride, offset, format)?;
        if self.decl.is_none() && stride == inline_uv::STRIDE {
            return Ok(self.inline_uv_before_colour(data, submesh, stride, coords));
        }
        Ok(coords)
    }

    /// The chunk's second diffuse coordinate set (`Uv2`), one pair per vertex.
    ///
    /// # Errors
    ///
    /// [`Error::NoTexcoord`] where the declaration names none, and
    /// [`Error::OutOfBounds`] as [`Self::positions`].
    pub fn second_texcoords(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Vec<[f32; 2]>> {
        let decl = self.decl.as_ref().ok_or(Error::NoTexcoord)?;
        let attribute = decl.second_texcoord().ok_or(Error::NoTexcoord)?;
        let format = TexcoordFormat::of(attribute).ok_or(Error::NoTexcoord)?;
        self.coords_at(data, submesh, stride, usize::from(attribute.offset), format)
    }

    /// The coordinates a **lightmap** is sampled through, one pair per vertex.
    ///
    /// The declaration names this attribute `lightmapUV` outright - see
    /// [`VertexDecl::lightmap_texcoord`] - so unlike [`Self::texcoords`] there
    /// is no rule here, only a lookup. A chunk that declares none answers
    /// [`Error::NoTexcoord`], which is every chunk whose material carries no
    /// lightmap and is the majority of the disc.
    ///
    /// # Errors
    ///
    /// [`Error::NoTexcoord`] and [`Error::OutOfBounds`], as [`Self::texcoords`].
    pub fn lightmap_texcoords(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Vec<[f32; 2]>> {
        let decl = self.decl.as_ref().ok_or(Error::NoTexcoord)?;
        let attribute = decl.lightmap_texcoord().ok_or(Error::NoTexcoord)?;
        let format = TexcoordFormat::of(attribute).ok_or(Error::NoTexcoord)?;
        self.coords_at(data, submesh, stride, usize::from(attribute.offset), format)
    }

    /// One submesh's **baked per-vertex light**, and the sun-occlusion mask
    /// beside it, or an error if the declaration carries no colour set.
    ///
    /// `[r, g, b, mask]`. The colour is HD's `f[TC1]`: the term the fragment
    /// program *adds* to the lightmap contribution before multiplying the
    /// albedo - see `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The
    /// lit track material". The vertex program moves it across unchanged
    /// (`MOV o[TC1].xyz, v[N].xyzx`), so the three colour bytes normalise and
    /// nothing else happens to them.
    ///
    /// **No shared exponent is applied, and the reason is a read.** HD's
    /// vertex programs do carry an RGBE form - `v.xyz * exp2(v.w * 255 - 128)`,
    /// whose `(255, 128)` are unanimous across all 6,946 blocks that use it -
    /// but that form reads attribute `0x868f8229`, and **no `.rcsmodel` on the
    /// disc declares it** (0 of 123). The attribute the models actually carry
    /// is a plain colour set.
    ///
    /// **The fourth byte is the sun-occlusion mask**, settled 2026-08-20: 41 %
    /// zero and 52 % full across Talon's Junction's 240,400 lit vertices, and
    /// the vertex program routes `colourSet.w` to a spare interpolator the
    /// fragment program reads in two places - gating the sun term and gating
    /// the specular, exactly where a lightmapped variant of the same material
    /// uses `lightmap.a` instead. See
    /// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The sun is real and
    /// it is masked". Confidence 86.
    pub fn vertex_light(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Vec<[f32; 4]>> {
        let decl = self.decl.as_ref().ok_or(Error::NoTexcoord)?;
        let attribute = decl.vertex_colour().ok_or(Error::NoTexcoord)?;
        let offset = usize::from(attribute.offset);
        let end =
            submesh.vertex_offset + stride * submesh.vertex_count.saturating_sub(1) + offset + 4;
        if submesh.vertex_count > 0 && end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a vertex buffer's colour set",
                end,
                len: data.len(),
            });
        }
        Ok((0..submesh.vertex_count)
            .map(|k| {
                let at = submesh.vertex_offset + k * stride + offset;
                [
                    f32::from(data[at]) / 255.0,
                    f32::from(data[at + 1]) / 255.0,
                    f32::from(data[at + 2]) / 255.0,
                    f32::from(data[at + 3]) / 255.0,
                ]
            })
            .collect())
    }

    /// One submesh's coordinate pairs at a byte offset within the vertex.
    ///
    /// Shared by [`Self::texcoords`] and [`Self::lightmap_texcoords`], which
    /// differ only in which attribute they pick.
    fn coords_at(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
        offset: usize,
        format: TexcoordFormat,
    ) -> Result<Vec<[f32; 2]>> {
        let end = submesh.vertex_offset
            + stride * submesh.vertex_count.saturating_sub(1)
            + offset
            + format.width();
        if submesh.vertex_count > 0 && end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a vertex buffer's texture coordinates",
                end,
                len: data.len(),
            });
        }
        Ok((0..submesh.vertex_count)
            .map(|k| format.decode(data, submesh.vertex_offset + k * stride + offset))
            .collect())
    }

    /// Reads one submesh's triangle indices.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfBounds`] when the buffer leaves the file, and
    /// [`Error::IndexOutOfRange`] when an index names a vertex the submesh does
    /// not declare - the check that says the offset and count were right.
    pub fn indices(&self, data: &[u8], submesh: &SubMesh) -> Result<Vec<u16>> {
        let end = submesh.index_offset + submesh.index_count * 2;
        if end > data.len() {
            return Err(Error::OutOfBounds {
                what: "an index buffer",
                end,
                len: data.len(),
            });
        }
        let indices: Vec<u16> = (0..submesh.index_count)
            .map(|k| ByteOrder::Big.u16(data, submesh.index_offset + k * 2))
            .collect();
        if let Some(&max) = indices.iter().max()
            && usize::from(max) >= submesh.vertex_count
        {
            return Err(Error::IndexOutOfRange {
                index: usize::from(max),
                vertices: submesh.vertex_count,
            });
        }
        Ok(indices)
    }
}

#[cfg(test)]
mod tests;
