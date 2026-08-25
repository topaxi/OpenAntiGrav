//! Decoding one surface record, and the chunk header that opens with one.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`, and it is the natural seam: everything here
//! is about the record at [`SURFACE_BASE`], which is the structure this module
//! learned about last and the one a reader is most likely to come looking for.

use crate::ByteOrder;

use super::{
    Error, LAYOUT_DESCRIBED, LAYOUT_INLINE, Layout, Mesh, Result, SUBMESH_BASE, SUBMESH_LEN,
    SURFACE_BASE, SURFACE_COUNT, SURFACE_LEN, SURFACE_TABLE, SubMesh, VertexDecl,
};

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
        let mut mesh = Self::parse_surface(data, at + SURFACE_BASE, layout, hash)?;

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
            if let Ok(surface) = Self::parse_surface(data, record, layout, hash) {
                mesh.extra_surfaces.push(surface);
            }
        }
        Ok(mesh)
    }

    /// Reads one surface record and the submesh descriptors after it.
    ///
    /// `at` is the record, not the chunk: `chunk + SURFACE_BASE` for the first
    /// and an entry of the chunk's surface table for the rest.
    fn parse_surface(data: &[u8], at: usize, layout: Layout, hash: u32) -> Result<Self> {
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
