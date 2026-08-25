//! Decoding one surface record, and the chunk header that opens with one.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`, and it is the natural seam: everything here
//! is about the record at [`SURFACE_BASE`], which is the structure this module
//! learned about last and the one a reader is most likely to come looking for.

use crate::ByteOrder;

use super::{
    Error, LAYOUT_DESCRIBED, LAYOUT_INLINE, Layout, Mesh, Result, SPACE_BYTE, SUBMESH_BASE,
    SUBMESH_LEN, SURFACE_BASE, SURFACE_COUNT, SURFACE_LEN, SURFACE_TABLE, SubMesh, VertexDecl,
};

/// Which space a chunk's dequantised positions are in, out of its
/// [`SPACE_BYTE`].
///
/// **The disc states this, and this project used to guess it.**
/// `oag_render::mesh::rcs::is_world_baked` decided the same question by
/// carrying a `.vex` node's authored box through its transform and asking
/// whether it landed within one world unit of the chunk's bias - a tolerance
/// invented here, with an essay attached. Measured over every `Mesh` node on
/// the disc whose chunk is in its own model, 11,450 of them, and judged by the
/// **bias**, which is independent of both readings (a world-baked chunk's bias
/// is a world position, a node-local one's is near zero):
///
/// - the byte agrees with the bias on **94.4 %**;
/// - the heuristic agrees with the byte on **43.2 %**, and on both directions
///   of disagreement the bias sides with the byte: of 5,724 chunks the byte
///   calls node-local and the heuristic called baked, 5,345 have a bias under
///   50 units and the median is **1.2**; of 775 the other way, only 27 are
///   under 50 units and the median is **544.3**.
///
/// Within `data/environments` the split is unambiguous: [`Self::World`]
/// chunks' `|bias|` has a median of 552.9 with **one** of 32,955 under a unit,
/// [`Self::Node`] chunks' a median of 2.7 with 35 % under a unit. Across the
/// disc every chunk of `data/ships` and `data/fe` is [`Self::Node`] and every
/// chunk of `data/pvsblocker` is [`Self::World`], and 8,769 of 8,773
/// [`Self::Node`] chunks are addressed by a `.vex` node.
///
/// Confidence 88 from the data. **The executable then confirmed the
/// mechanism**: `Scene_RefreshNodeMatrices` (`0x003fb330`) walks every chunk
/// and, *only* where this byte is `2`, follows the chunk's runtime block to a
/// linked scene node, refreshes it if its dirty bit is set, and copies four
/// 16-byte rows - a 4x4 matrix - into the head of the block. A `1` chunk is
/// skipped entirely. So the byte says **whether this chunk's world transform
/// is re-read from a node every frame**, which is the runtime face of the
/// same link `.vex` nodes make by hash. See
/// `docs/ghidra/functions/ps3-hdfury-eu/visibility.md`, "The chunk kind byte".
///
/// **It is genuinely three-way, which is why [`Self::Unknown`] exists rather
/// than a boolean**: the load-time switch tests `1`, then `2`, then falls
/// through to a third path. No chunk on this disc takes it.
///
/// **What it is not**: no arm of either switch reads the vertex declaration,
/// the index buffer, the stride, the surface count or anything under the
/// surface record - both only store command words into an emission cursor. So
/// it cannot be a triangle-list-versus-strip, an index-width or an
/// attribute-set selector, and none of this module's geometry decoding turns
/// on it. Confidence 82 for that negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// `1`: positions are already in world space, and no node transform
    /// applies. 33,088 chunks.
    World,
    /// `2`: positions are in the chunk's own space, and a `.vex` node places
    /// it. 8,773 chunks.
    Node,
    /// Neither value. None on this disc; kept so a future title's file is
    /// reported rather than silently read as one of the two.
    Unknown(u8),
}
impl Space {
    fn of(byte: u8) -> Self {
        match byte {
            1 => Self::World,
            2 => Self::Node,
            other => Self::Unknown(other),
        }
    }
}

impl Mesh {
    /// Reads one chunk: its hash and layout, then every surface it declares.
    ///
    /// **The returned `Mesh` is the chunk's first surface**, and the rest are
    /// in [`Self::extra_surfaces`]; [`Self::surfaces`] walks all of them. That
    /// is not a convenience - a chunk and a surface have the same record
    /// shape, so a surface *is* a `Mesh` with its own material, bias, scale,
    /// declaration and descriptors, and every reader written against a chunk
    /// works on one unchanged. See [`SURFACE_BASE`].
    pub(super) fn parse(data: &[u8], at: usize) -> Result<Self> {
        let end = at + SUBMESH_BASE;
        if end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a mesh chunk header",
                end,
                len: data.len(),
            });
        }
        let layout = match data[at + 0x06] {
            LAYOUT_DESCRIBED => Layout::Described,
            LAYOUT_INLINE => Layout::Inline,
            other => return Err(Error::UnknownChunkLayout { got: other, at }),
        };
        let hash = ByteOrder::Big.u32(data, at);
        let space = Space::of(data[at + SPACE_BYTE]);
        let mut mesh = Self::parse_surface(data, at + SURFACE_BASE, layout, space, hash)?;

        // **A surface that will not read is skipped, not fatal.** The first one
        // is the chunk itself and its failure is a real error, handled above;
        // a later one failing costs that surface's triangles and nothing else,
        // and refusing the whole model over it would lose the other 99 %.
        let count = usize::from(ByteOrder::Big.u16(data, at + SURFACE_COUNT));
        let table = ByteOrder::Big.u32(data, at + SURFACE_TABLE) as usize;
        for index in 1..count {
            let entry = table + index * 4;
            if entry + 4 > data.len() {
                break;
            }
            let record = ByteOrder::Big.u32(data, entry) as usize;
            if let Ok(surface) = Self::parse_surface(data, record, layout, space, hash) {
                mesh.extra_surfaces.push(surface);
            }
        }
        Ok(mesh)
    }

    /// Reads one surface record and the submesh descriptors after it.
    ///
    /// `at` is the record, not the chunk: `chunk + SURFACE_BASE` for the first
    /// and an entry of the chunk's surface table for the rest.
    fn parse_surface(
        data: &[u8],
        at: usize,
        layout: Layout,
        space: Space,
        hash: u32,
    ) -> Result<Self> {
        let end = at + SURFACE_LEN;
        if end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a surface record",
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
        let submeshes = match layout {
            Layout::Described => {
                let count = ByteOrder::Big.u32(data, at + 0x30) as usize;
                let last = at + SURFACE_LEN + count * SUBMESH_LEN;
                if last > data.len() {
                    return Err(Error::OutOfBounds {
                        what: "the submesh descriptors",
                        end: last,
                        len: data.len(),
                    });
                }
                (0..count)
                    .map(|i| {
                        let b = at + SURFACE_LEN + i * SUBMESH_LEN;
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
            Layout::Inline => {
                if at + 0x4e > data.len() {
                    return Err(Error::OutOfBounds {
                        what: "an inline chunk's buffer fields",
                        end: at + 0x4e,
                        len: data.len(),
                    });
                }
                vec![SubMesh {
                    format: std::array::from_fn(|k| data[at + SURFACE_LEN + k]),
                    vertex_count: ByteOrder::Big.u16(data, at + 0x4c) as usize,
                    vertex_offset: ByteOrder::Big.u32(data, at + 0x34) as usize,
                    index_count: ByteOrder::Big.u32(data, at + 0x38) as usize,
                    index_offset: ByteOrder::Big.u32(data, at + 0x3c) as usize,
                }]
            }
        };

        let decl = (layout == Layout::Described)
            .then(|| VertexDecl::parse(data, ByteOrder::Big.u32(data, at + 0x38) as usize))
            .flatten();

        Ok(Self {
            hash,
            bias: f3(0x10),
            scale: f3(0x20),
            material: ByteOrder::Big.u32(data, at),
            layout,
            space,
            submeshes,
            decl,
            extra_surfaces: Vec::new(),
        })
    }

    /// Every surface this chunk declares, this one first.
    ///
    /// **A quarter of the disc's chunks declare more than one**, and each names
    /// its own material - so a reader that walks only `self` paints part of the
    /// chunk with the wrong texture and never draws the rest at all. 9,891 of
    /// 41,861 chunks, measured by `crates/render/examples/hd_surfaces.rs`.
    pub fn surfaces(&self) -> impl Iterator<Item = &Self> {
        std::iter::once(self).chain(self.extra_surfaces.iter())
    }
}
