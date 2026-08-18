//! `.rcsmodel`: where Wipeout HD keeps the geometry that used to be in the
//! `.vex`.
//!
//! On the PSP and PS2 a [`Mesh`](crate::vex) node's payload *is* its geometry.
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
//! is in no field of the file - see [`TexcoordFormat`] and
//! [`Mesh::texcoord_format`]. Reading every one as a half left 290 of a
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
//! +0x04  u32   end of the directory / first byte of chunk data
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

use crate::ByteOrder;

pub mod material;
mod stride;

pub use material::{Blend, Factor, Material, Transparency};

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
const SUBMESH_BASE: usize = 0x60;

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
/// **Byte `+0x06` of a chunk.** The `u32` it sits in reads `00 nn LL kk`: `nn`
/// counts chunks (and is `0xff` on many), `LL` is this, and `kk` takes the
/// values `01` and `02` for a reason nothing here has distinguished. See
/// [`LAYOUT_INLINE`].
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
/// **These three and nothing else, and that is a measurement rather than an
/// optimisation.** They are 6 bytes of position plus 8, 12 or 16 of attributes,
/// and they are the only widths an authored bounding box has ever settled on -
/// across all 89 meshes of Assegai, its LOD1 and Talon's Junction.
///
/// Searching every even width from 6 to 64 instead, as this did first, is
/// strictly worse on a circuit: 814 of `talons_junction`'s 983 chunks still
/// choose one of these three, and the other 169 choose a width no measurement
/// supports and decode to spikes radiating out of the level. A width outside
/// this set is not evidence of a fourth format; it is the search finding
/// nothing and picking the least bad noise. Skipping the chunk and saying so is
/// the honest answer, and it is what the callers do.
pub const STRIDES: &[usize] = &[14, 18, 22];

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

/// How many vertices [`Mesh::texcoord_format`] looks at before deciding.
///
/// The two groups are not marginal - the half reading is usable on 95 %+ of one
/// and under 50 % of the other - so a small sample settles it, and a submesh
/// under this many vertices is read whole.
const TEXCOORD_FORMAT_SAMPLE: usize = 64;

/// How many whole tiles of its texture a coordinate may span before the half
/// reading is judged not to be one.
///
/// A circuit tiles a road texture tens of times and never thousands; the values
/// this rejects are `65504` and exact powers of two up to it, which is what a
/// half decodes to when its bits are really a `u16`.
const TEXCOORD_PLAUSIBLE_TILES: f32 = 8.0;

/// The two types a `.rcsmodel` writes a texture coordinate in.
///
/// **Both are measured, and the split is real rather than one type misread.**
/// The discriminator is texel density: the spread of `log(uv area / world
/// area)` within a chunk, which is consistent for a correct mapping because
/// artists map at a consistent density and is not for a wrong one. Over
/// Talon's Junction's stride-18 chunks, `Half` scores **0.610** against
/// `Unorm16`'s 1.164 on the group where halves read plausibly, and `Unorm16`
/// scores **1.839** against `Half`'s 8.170 on the group where they do not. Each
/// group is best explained by a different type, on a metric that cannot be
/// gamed by scale - which is what says there are two.
///
/// Selected per submesh by [`Mesh::texcoord_format`]. 285 of Talon's Junction's
/// 337 stride-18 submeshes are `Half` and 52 are `Unorm16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexcoordFormat {
    /// Two big-endian IEEE halves - see [`unpack_half`].
    Half,
    /// Two big-endian `u16`s over `0..=1`, RSX's `CELL_GCM_VERTEX_U16N` shape.
    Unorm16,
}

impl TexcoordFormat {
    /// One 16-bit field as a coordinate.
    #[must_use]
    pub fn decode(self, bits: u16) -> f32 {
        match self {
            Self::Half => unpack_half(bits),
            Self::Unorm16 => f32::from(bits) / f32::from(u16::MAX),
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
    /// The draw calls this mesh is split into.
    pub submeshes: Vec<SubMesh>,
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
}

impl Mesh {
    /// Reads one chunk header and its submesh descriptors.
    fn parse(data: &[u8], at: usize) -> Result<Self> {
        let end = at + SUBMESH_BASE;
        if end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a mesh chunk header",
                end,
                len: data.len(),
            });
        }
        let f3 = |off: usize| {
            [
                ByteOrder::Big.f32(data, at + off),
                ByteOrder::Big.f32(data, at + off + 4),
                ByteOrder::Big.f32(data, at + off + 8),
            ]
        };
        let submeshes = match data[at + 0x06] {
            LAYOUT_DESCRIBED => {
                let count = ByteOrder::Big.u32(data, at + 0x50) as usize;
                let last = at + SUBMESH_BASE + count * SUBMESH_LEN;
                if last > data.len() {
                    return Err(Error::OutOfBounds {
                        what: "the submesh descriptors",
                        end: last,
                        len: data.len(),
                    });
                }
                (0..count)
                    .map(|i| {
                        let b = at + SUBMESH_BASE + i * SUBMESH_LEN;
                        SubMesh {
                            format: std::array::from_fn(|k| data[b + k]),
                            vertex_count: ByteOrder::Big.u16(data, b + 0x08) as usize,
                            vertex_offset: ByteOrder::Big.u32(data, b + 0x18) as usize,
                            index_count: ByteOrder::Big.u16(data, b + 0x0a) as usize,
                            index_offset: ByteOrder::Big.u32(data, b + 0x10) as usize,
                        }
                    })
                    .collect()
            }
            LAYOUT_INLINE => {
                if at + 0x6e > data.len() {
                    return Err(Error::OutOfBounds {
                        what: "an inline chunk's buffer fields",
                        end: at + 0x6e,
                        len: data.len(),
                    });
                }
                vec![SubMesh {
                    format: std::array::from_fn(|k| data[at + SUBMESH_BASE + k]),
                    vertex_count: ByteOrder::Big.u16(data, at + 0x6c) as usize,
                    vertex_offset: ByteOrder::Big.u32(data, at + 0x54) as usize,
                    index_count: ByteOrder::Big.u32(data, at + 0x58) as usize,
                    index_offset: ByteOrder::Big.u32(data, at + 0x5c) as usize,
                }]
            }
            other => {
                return Err(Error::UnknownChunkLayout { got: other, at });
            }
        };

        Ok(Self {
            hash: ByteOrder::Big.u32(data, at),
            bias: f3(0x30),
            scale: f3(0x40),
            material: ByteOrder::Big.u32(data, at + 0x20),
            submeshes,
        })
    }

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

    /// The texture coordinates, one pair per vertex.
    ///
    /// # The last four bytes of a vertex, on every stride
    ///
    /// Two big-endian IEEE halves - see [`unpack_half`]. Located by elimination
    /// rather than decoded from anything that names them: the position, the
    /// normal and (at stride 22) the tangent account for every other field, and
    /// what is left reads as a pair in a texture-coordinate range.
    ///
    /// **How strong that is, measured**: on Assegai 99.9 % of 24,848 stride-22
    /// vertices and 96.9 % of its stride-18 ones fall in the unit square, with
    /// 0.02 % non-finite. A circuit is looser, as tiling makes it - 79.6 % of
    /// Talon's Junction's 525,944 stride-18 vertices in the unit square, 83.2 %
    /// within +/-8, and 1.68 % non-finite, which is a real residue and not
    /// rounding.
    ///
    /// **What confirms it is a picture, and that is now possible.** Until
    /// `oag_formats::gtf` was read there was nothing to check a UV against, so
    /// this went unread and the code refused to call it one. There is an oracle
    /// now: a texture sampled through these coordinates either lands on the
    /// surface it belongs to or streaks visibly.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfBounds`], as [`Self::positions`].
    pub fn texcoords(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Vec<[f32; 2]>> {
        let end = submesh.vertex_offset + stride * submesh.vertex_count.saturating_sub(1) + stride;
        if submesh.vertex_count > 0 && end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a vertex buffer's texture coordinates",
                end,
                len: data.len(),
            });
        }
        let format = self.texcoord_format(data, submesh, stride);
        Ok((0..submesh.vertex_count)
            .map(|k| {
                let at = submesh.vertex_offset + k * stride + stride - TEXCOORD_LEN;
                std::array::from_fn(|i| format.decode(ByteOrder::Big.u16(data, at + i * 2)))
            })
            .collect())
    }

    /// Which of the two types this submesh's texture coordinate is written in.
    ///
    /// **Recovered from the content, because it is in no field of the file.**
    /// Every byte of the chunk header (`+0x00`..`+0x60`) and every byte of the
    /// `0x80`-byte submesh descriptor was swept against the two groups on
    /// Talon's Junction and **none separates them**; neither does the material,
    /// the stride, or the `83 XX` descriptor byte. What does separate them is
    /// what the bytes decode to, which is what this reads.
    ///
    /// A [`TexcoordFormat::Half`] submesh read as halves gives finite
    /// coordinates in a texture-coordinate range; a [`TexcoordFormat::Unorm16`]
    /// one read the same way gives infinities and values up to `65504`, the
    /// largest finite half - **290 of Talon's Junction's 1,112 draw calls** span
    /// over 100 tiles of their texture for exactly this reason. So the test is
    /// whether the half reading is mostly usable.
    ///
    /// See `docs/formats/rcsmodel.md` for the evidence that these are two
    /// *types* rather than one type misread.
    #[must_use]
    pub fn texcoord_format(&self, data: &[u8], submesh: &SubMesh, stride: usize) -> TexcoordFormat {
        let mut usable = 0usize;
        let mut seen = 0usize;
        for k in (0..submesh.vertex_count).take(TEXCOORD_FORMAT_SAMPLE) {
            let at = submesh.vertex_offset + k * stride + stride - TEXCOORD_LEN;
            if at + TEXCOORD_LEN > data.len() {
                break;
            }
            seen += 1;
            let pair: [f32; 2] =
                std::array::from_fn(|i| unpack_half(ByteOrder::Big.u16(data, at + i * 2)));
            if pair
                .iter()
                .all(|c| c.is_finite() && c.abs() <= TEXCOORD_PLAUSIBLE_TILES)
            {
                usable += 1;
            }
        }
        // An empty or unreadable submesh keeps the type the disc uses on five
        // sixths of its geometry, which is also the one every earlier reading
        // assumed.
        if seen == 0 {
            return TexcoordFormat::Half;
        }
        if usable * 2 >= seen {
            TexcoordFormat::Half
        } else {
            TexcoordFormat::Unorm16
        }
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
