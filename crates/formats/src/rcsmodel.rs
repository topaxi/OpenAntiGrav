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
//! **Positions, triangle indices and vertex normals.** The normals are a packed
//! 11:11:10 signed triple at `+6` of every vertex, on all three strides - see
//! [`unpack_normal`] and [`Mesh::normals`].
//!
//! The rest of a vertex is 4 to 12 further bytes and is **not** decoded. What is
//! known about them:
//!
//! - The **last four** read as two `f16` in a texture-coordinate range on every
//!   stride. That is where a UV would be, and there is no way to check it until
//!   the `.gtf` textures are read - so nothing consumes it and the code does not
//!   call it one.
//! - The four at **`+0x0a`** are a *tangent* on stride 22 and something else on
//!   stride 18. That split follows the **stride**, not the `83 XX` descriptor
//!   byte, which was the obvious hypothesis and is measured false; see
//!   [`SubMesh::format`].
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

/// Bytes of position in a vertex: three big-endian `i16`s.
const POSITION_LEN: usize = 6;

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
        Ok(Self { meshes })
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

    /// Recovers the vertex stride from the bounding box the `.vex` node
    /// authors, or `None` when no single stride explains it.
    ///
    /// # Why this is a search and not a field read
    ///
    /// **The stride is not in the file, as far as anyone has found.** The eight
    /// bytes at a descriptor's `+0x00` look like a vertex-format word and the
    /// byte at `+0x01` does vary with it - but it takes values `0x07` through
    /// `0x0d` against strides of 14, 18 and 22, and one value of it (`0x08`)
    /// appears with all three. No byte or `u16` anywhere in the 0x80-byte
    /// descriptor equals the stride on more than one of 38 solved submeshes.
    /// So this reads it out of the data by consequence instead.
    ///
    /// # Why the box is a sound oracle
    ///
    /// The `.vex` node carries an authored min/max pair whose `min <= max`
    /// holds on 1,638 of 1,638 nodes, and it is the mesh's *tight* box: with
    /// the true stride the dequantised points touch all six faces to within a
    /// quantisation step. A wrong stride reads position bits out of the
    /// attributes that follow, which lands outside the box almost immediately.
    ///
    /// **The test is tightness, not containment**, and the difference matters:
    /// requiring only "inside" admits a stride that is a divisor of the true
    /// one, which walks a subset of the vertices and stays inside by
    /// construction. Requiring the union to fill the box removed every
    /// ambiguity across all 89 meshes measured - 0 ambiguous, in three models
    /// including the largest circuit on the disc.
    ///
    /// `bounds` is the node's `(min, max)`. `tolerance` is how far a face may
    /// be missed by; two quantisation steps is what the callers use.
    #[must_use]
    pub fn solve_stride(&self, data: &[u8], bounds: Bounds, tolerance: f32) -> Option<usize> {
        let (min, max) = bounds;
        let populated = self.submeshes.iter().filter(|s| s.vertex_count > 0).count();
        let mut found = None;
        for &stride in STRIDES {
            let Some(((lo, hi), fitted)) = self.extent(data, stride, bounds) else {
                continue;
            };
            // **The majority rule.** `extent` skips a submesh that does not fit,
            // so without this a stride could qualify by fitting one submesh out
            // of nineteen and skipping the rest. Half is enough: the case this
            // exists for is one stray submesh, not a coin toss.
            if fitted * 2 < populated {
                continue;
            }
            let tight = (0..3).all(|i| {
                (lo[i] - min[i]).abs() <= tolerance && (hi[i] - max[i]).abs() <= tolerance
            });
            if !tight {
                continue;
            }
            if found.is_some() {
                // Two strides explain the same box. Never seen on the disc, and
                // reported as unknown rather than resolved by preference.
                return None;
            }
            found = Some(stride);
        }
        found
    }

    /// Recovers the vertex stride with no bounding box to check against, by
    /// taking the one whose decoded positions are most compact.
    ///
    /// # Why a circuit needs this and a craft does not
    ///
    /// **Wipeout HD's road is not in the `.vex`.** All 126 `Mesh` nodes of
    /// `talons_junction/track.vex` are props - blimps, girders, sky traffic -
    /// and the circuit itself is among the **904 of 983** chunks no node
    /// references at all, drawn from the visibility set instead. Those chunks
    /// have no authored box, so [`Self::solve_stride`] has nothing to ask.
    ///
    /// # Why the most compact reading is the right one
    ///
    /// A position is a quantised `i16`; the attribute bytes after it are
    /// normalised across the whole `i16` range. So a wrong stride reads
    /// attributes as positions and spreads them over the full +/-32768 - two
    /// orders of magnitude wider than a real mesh, which occupies a tile.
    /// Taking the minimum is therefore not a heuristic dressed as a rule, it is
    /// reading the one interpretation that is not noise.
    ///
    /// **Checked against the oracle it replaces**: on the 78 meshes of Assegai
    /// and Talon's Junction where an authored box settles the stride, this
    /// picks the same value on **77**. The one disagreement is a mesh where the
    /// box admitted 36 and this picks 18 - half of it, so the box was matching
    /// every second vertex and this is the better answer rather than a worse
    /// one.
    ///
    /// # The winner has to be decisive
    ///
    /// Smallest-wins alone is not enough: on a circuit the three widths often
    /// produce spans within a few per cent of each other, and picking one of
    /// those by a hair decodes to spikes radiating out of the level. So the
    /// winner must be **at most half** the runner-up - a relative test with no
    /// threshold to tune, and the reading either stands out from the noise or
    /// there is no answer.
    ///
    /// Validated on the same oracle: all **70 of 70** of `talons_junction`'s
    /// box-labelled chunks clear it, the worst at 0.35 and the median at 0.02.
    /// It keeps 718 of the circuit's 983 chunks; the rest draw nothing.
    ///
    /// `None` for a chunk with no readable vertex buffer, or none whose reading
    /// stands out.
    #[must_use]
    pub fn solve_stride_by_extent(&self, data: &[u8]) -> Option<usize> {
        let mut spans: Vec<(usize, i32)> = STRIDES
            .iter()
            .filter_map(|&stride| Some((stride, self.quantised_span(data, stride, None)?)))
            .collect();
        spans.sort_by_key(|&(_, span)| span);
        let [(stride, best), (_, runner_up), ..] = spans[..] else {
            return None;
        };
        (i64::from(best) * 2 <= i64::from(runner_up)).then_some(stride)
    }

    /// Recovers the vertex stride from where the file **puts** its buffers,
    /// rather than from what they decode to.
    ///
    /// A mesh's vertex buffers are packed back to back, so the distance from one
    /// submesh's buffer to the next one's, over the first one's vertex count, is
    /// the stride - arithmetic on two numbers the file states outright, with no
    /// oracle and nothing decoded. Every consecutive pair votes and the majority
    /// wins.
    ///
    /// # Why this is the rule to prefer
    ///
    /// It is **structural where the other two are not**. [`Self::solve_stride`]
    /// needs a box the `.vex` authors, and [`Self::solve_stride_by_extent`] is a
    /// statistic about what the bytes look like once read. This is neither: it
    /// is the layout the exporter wrote.
    ///
    /// Measured against both:
    ///
    /// - **18 of 18 agreement with the authored box**, 0 disagreements, over
    ///   every mesh of Assegai and Talon's Junction that has a box and more than
    ///   one submesh.
    /// - **95 of 95 agreement with the compactness rule** on `talons_junction`'s
    ///   chunks where both decide. Two independent rules - one about layout, one
    ///   about content - agreeing exactly is a far better argument for the
    ///   compactness rule than the compactness rule can make for itself.
    /// - It decides **16 chunks the compactness rule cannot**, and the
    ///   compactness rule decides 623 this one cannot: a chunk with a single
    ///   submesh has no step to measure, and most of a circuit's chunks are
    ///   single-submesh. They are complements, not alternatives.
    ///
    /// # The rounding, and what is unexplained
    ///
    /// A step is usually exactly `vertex_count * stride`, but not always: the
    /// residual `step - vertex_count * stride` is 0 on 38 of 46 measured pairs
    /// and otherwise -16, -32 or -96 - always negative, always a multiple of 16.
    /// **Why the next buffer starts before the previous one's declared length
    /// ends is unrecovered**, and this rounds rather than modelling it, because a
    /// correction nobody can justify is worse than a rounding everybody can see.
    /// The error is at most 96 bytes over at least 33 vertices, well inside the
    /// 2-byte gaps between the three widths.
    ///
    /// `None` for a mesh with fewer than two submeshes, or one whose pairs do
    /// not agree on a width [`STRIDES`] carries.
    #[must_use]
    pub fn solve_stride_by_layout(&self) -> Option<usize> {
        let mut by_offset: Vec<&SubMesh> = self.submeshes.iter().collect();
        by_offset.sort_unstable_by_key(|sub| sub.vertex_offset);

        let mut votes = vec![0usize; STRIDES.len()];
        for pair in by_offset.windows(2) {
            let [first, next] = pair else { continue };
            if first.vertex_count == 0 || next.vertex_offset <= first.vertex_offset {
                continue;
            }
            let step = next.vertex_offset - first.vertex_offset;
            // Round to nearest without leaving integer arithmetic: the +/- 96
            // residual above means the quotient is not exact.
            let stride = (2 * step + first.vertex_count) / (2 * first.vertex_count);
            if let Some(slot) = STRIDES.iter().position(|&s| s == stride) {
                votes[slot] += 1;
            }
        }
        let (slot, &best) = votes.iter().enumerate().max_by_key(|&(_, n)| n)?;
        if best == 0 {
            return None;
        }
        // A tie is two readings with equal support, and there is no principle
        // here that breaks one - so it is unknown, the same answer the other two
        // rules give when nothing stands out.
        let tied = votes.iter().filter(|&&n| n == best).count() > 1;
        (!tied).then(|| STRIDES[slot])
    }

    /// The stride of a chunk **no `.vex` node references**, which is most of a
    /// circuit.
    ///
    /// Asks [`Self::solve_stride_by_layout`] first because it is structural, and
    /// falls back to [`Self::solve_stride_by_extent`] because the layout rule
    /// says nothing at all about a single-submesh chunk. On `talons_junction`
    /// the two together decide **734 of 983** chunks where the compactness rule
    /// alone decided 718, and they never disagree - 95 of 95 where both answer.
    #[must_use]
    pub fn solve_stride_without_a_box(&self, data: &[u8]) -> Option<usize> {
        self.solve_stride_by_layout()
            .or_else(|| self.solve_stride_by_extent(data))
    }

    /// The widest axis span of the raw `i16` positions at `stride`, or `None`
    /// if no submesh could be read or the span passed `ceiling`.
    ///
    /// Measured before the bias and scale are applied, so it compares strides
    /// on one chunk without a multiply per vertex. `ceiling` stops a read that
    /// has already exceeded a span the caller has no use for.
    fn quantised_span(&self, data: &[u8], stride: usize, ceiling: Option<i32>) -> Option<i32> {
        let mut lo = [i32::MAX; 3];
        let mut hi = [i32::MIN; 3];
        let mut any = false;
        for submesh in &self.submeshes {
            if submesh.vertex_count == 0 {
                continue;
            }
            let end = submesh.vertex_offset + stride * (submesh.vertex_count - 1) + POSITION_LEN;
            if end > data.len() {
                continue;
            }
            any = true;
            for k in 0..submesh.vertex_count {
                let at = submesh.vertex_offset + k * stride;
                for i in 0..3 {
                    let v = i32::from(ByteOrder::Big.i16(data, at + i * 2));
                    lo[i] = lo[i].min(v);
                    hi[i] = hi[i].max(v);
                }
                if let Some(ceiling) = ceiling
                    && (0..3).any(|i| hi[i] - lo[i] >= ceiling)
                {
                    return None;
                }
            }
        }
        any.then(|| (0..3).map(|i| hi[i] - lo[i]).max().unwrap_or(0))
    }

    /// Whether one submesh's positions all land inside `bounds` at `stride`.
    ///
    /// **A caller that draws has to ask this too, not just [`solve_stride`].**
    /// The stride search tolerates a submesh that does not fit - see
    /// [`Mesh::extent`] - so the stride it returns is right for the mesh and
    /// wrong for that one submesh, whose positions then come out of the
    /// attribute bytes and scatter across the world. Assegai's hull has exactly
    /// one such submesh and it stretched the ship's own bounding sphere from 7
    /// units to 130, which framed the craft as a speck.
    ///
    /// [`solve_stride`]: Mesh::solve_stride
    #[must_use]
    pub fn submesh_fits(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
        bounds: Bounds,
    ) -> bool {
        let (min, max) = bounds;
        let slack = 1e-2;
        self.positions(data, submesh, stride).is_ok_and(|points| {
            points
                .iter()
                .all(|p| (0..3).all(|i| p[i] >= min[i] - slack && p[i] <= max[i] + slack))
        })
    }

    /// The box the submeshes that *fit* inside `bounds` at `stride` occupy, and
    /// how many of them there were.
    ///
    /// # A submesh that does not fit is skipped, not fatal
    ///
    /// **Measured, and it is the difference between drawing a craft and drawing
    /// its airbrakes.** Assegai's hull is one `Mesh` node of 19 submeshes; 18 of
    /// them fit at stride 22 and exactly one - `sub13` - fits at no stride at
    /// all. Requiring every submesh to fit therefore rejected 22 for the whole
    /// node and the hull vanished, while the 18 that do fit reconstruct it
    /// exactly. Why that one submesh reads differently is unrecovered.
    ///
    /// The guard against a stride surviving by skipping almost everything is
    /// [`Mesh::solve_stride`]'s majority rule, which is why the count comes back
    /// with the box rather than being swallowed here.
    fn extent(&self, data: &[u8], stride: usize, bounds: Bounds) -> Option<(Bounds, usize)> {
        let (min, max) = bounds;
        // A hair of slack, because the authored box is stored as `f32` and the
        // positions are reconstructed from a quantised integer.
        let slack = 1e-2;
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut fitted = 0;
        for submesh in &self.submeshes {
            if submesh.vertex_count == 0 {
                continue;
            }
            let Ok(points) = self.positions(data, submesh, stride) else {
                continue;
            };
            // Bails on the first stray point, which is what keeps the search
            // over `STRIDES` cheap: a wrong stride usually leaves the box within
            // a few vertices.
            if points
                .iter()
                .any(|p| (0..3).any(|i| p[i] < min[i] - slack || p[i] > max[i] + slack))
            {
                continue;
            }
            fitted += 1;
            for point in points {
                for i in 0..3 {
                    lo[i] = lo[i].min(point[i]);
                    hi[i] = hi[i].max(point[i]);
                }
            }
        }
        (fitted > 0).then_some(((lo, hi), fitted))
    }
}

#[cfg(test)]
mod tests;
