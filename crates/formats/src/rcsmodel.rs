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
//! **Positions and triangle indices, and nothing else.** That is enough to draw
//! the shape of a thing, which is what this exists for. Each vertex is followed
//! by 8 to 16 further bytes - normals, texture coordinates, colours, tangents -
//! and none of them is read: their layout is not recovered, and a normal read
//! from the wrong offset lights a model wrongly rather than visibly failing.
//! The `.gtf` textures those coordinates would address are a separate undecoded
//! container anyway, so nothing downstream could use them yet.
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
//! - **The vertex stride is not in the file.** See [`solve_stride`]: it is
//!   recovered from the authored bounding box the `.vex` node carries, which is
//!   the same box `docs/formats/hd-status.md` measured `min <= max` on across
//!   1,638 of 1,638 nodes.
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
//!                **Not the stride** - see `solve_stride`.
//! +0x08  u16     vertex count
//! +0x0a  u16     index count, divisible by 3 on 1,274 of 1,274 submeshes
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

/// The strides [`Mesh::solve_stride`] will consider, in bytes.
///
/// **14, 18 and 22 are the only values measured**, and they are 6 bytes of
/// position plus 8, 12 or 16 of attributes - so the range below is wider than
/// what the disc uses, deliberately, and steps by 2 because a `.rcsmodel`
/// vertex is `i16`-aligned on every file read.
const STRIDES: std::ops::RangeInclusive<usize> = 6..=64;

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
        }
    }
}

impl std::error::Error for Error {}

/// Shorthand for this module's results.
pub type Result<T> = std::result::Result<T, Error>;

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
        let count = ByteOrder::Big.u32(data, at + 0x50) as usize;
        let last = at + SUBMESH_BASE + count * SUBMESH_LEN;
        if last > data.len() {
            return Err(Error::OutOfBounds {
                what: "the submesh descriptors",
                end: last,
                len: data.len(),
            });
        }

        let submeshes = (0..count)
            .map(|i| {
                let b = at + SUBMESH_BASE + i * SUBMESH_LEN;
                SubMesh {
                    vertex_count: ByteOrder::Big.u16(data, b + 0x08) as usize,
                    vertex_offset: ByteOrder::Big.u32(data, b + 0x18) as usize,
                    index_count: ByteOrder::Big.u16(data, b + 0x0a) as usize,
                    index_offset: ByteOrder::Big.u32(data, b + 0x10) as usize,
                }
            })
            .collect();

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
    pub fn solve_stride(
        &self,
        data: &[u8],
        bounds: ([f32; 3], [f32; 3]),
        tolerance: f32,
    ) -> Option<usize> {
        let (min, max) = bounds;
        let mut found = None;
        for stride in STRIDES.step_by(2) {
            let Some((lo, hi)) = self.extent(data, stride, bounds) else {
                continue;
            };
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

    /// The bounding box of every submesh's positions at `stride`, or `None` if
    /// any point falls outside `bounds`.
    ///
    /// Bails on the first stray point, which is what keeps the search over
    /// [`STRIDES`] cheap: a wrong stride usually leaves the box within a few
    /// vertices.
    fn extent(
        &self,
        data: &[u8],
        stride: usize,
        bounds: ([f32; 3], [f32; 3]),
    ) -> Option<([f32; 3], [f32; 3])> {
        let (min, max) = bounds;
        // A hair of slack, because the authored box is stored as `f32` and the
        // positions are reconstructed from a quantised integer.
        let slack = 1e-2;
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut any = false;
        for submesh in &self.submeshes {
            if submesh.vertex_count == 0 {
                continue;
            }
            let points = self.positions(data, submesh, stride).ok()?;
            for point in points {
                any = true;
                for i in 0..3 {
                    if point[i] < min[i] - slack || point[i] > max[i] + slack {
                        return None;
                    }
                    lo[i] = lo[i].min(point[i]);
                    hi[i] = hi[i].max(point[i]);
                }
            }
        }
        any.then_some((lo, hi))
    }
}

#[cfg(test)]
mod tests;
