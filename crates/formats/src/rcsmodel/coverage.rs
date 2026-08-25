//! Which bytes of a `.rcsmodel` this crate actually reads.
//!
//! Its own file under the 1,000-line rule in `scripts/check-file-size.py`, and
//! a fair seam: this is an audit *of* the parser rather than part of it, and
//! nothing else in the module calls it.

use crate::ByteOrder;

use super::{
    HEADER_LEN, MATERIAL_LEN, Model, SUBMESH_LEN, SURFACE_BASE, SURFACE_COUNT, SURFACE_LEN,
    SURFACE_TABLE,
};

/// Which bytes of a `.rcsmodel` this module actually reads.
///
/// **The audit that would have caught the surface table.** Before it was read,
/// every chunk's `+0x10` count and `+0x18` offset table went unclaimed, and so
/// did the surface records and descriptors they name - a quarter of the disc's
/// chunks quietly losing their geometry with nothing to report it. A parser
/// cannot fail on a field it does not know about; this is how such a field
/// becomes visible.
///
/// Claims are made from the *parsed* model where possible - a submesh already
/// carries its buffer offsets - and from the raw header where the parser reads
/// an offset it does not keep. That duplication is the point rather than a
/// flaw: enumerating what is read is exactly the exercise that finds what is
/// not.
///
/// **A gap is a lead, not a bug.** Strings share a pool, records are padded,
/// and some sections are deliberately undecoded. See
/// `crates/formats/tests/coverage_ground_truth.rs`, which ratchets the total
/// rather than demanding zero.
#[must_use]
pub fn coverage(data: &[u8]) -> crate::coverage::Coverage {
    let mut seen = crate::coverage::Coverage::new(data.len());
    let Ok(model) = Model::parse(data) else {
        return seen;
    };
    let be32 = |at: usize| ByteOrder::Big.u32(data, at) as usize;
    seen.claim(0, HEADER_LEN, "the file header");
    let chunk_table = be32(0x20);
    seen.claim(
        chunk_table,
        model.meshes.len() * 4,
        "the chunk offset table",
    );
    let material_table = be32(0x30);
    seen.claim(
        material_table,
        model.materials.len() * 4,
        "the material offset table",
    );
    for index in 0..model.materials.len() {
        let at = be32(material_table + index * 4);
        seen.claim(at, MATERIAL_LEN, "a material record");
    }
    for (index, chunk) in model.meshes.iter().enumerate() {
        let at = be32(chunk_table + index * 4);
        seen.claim(at, SURFACE_BASE, "a chunk header");
        let surfaces = usize::from(ByteOrder::Big.u16(data, at + SURFACE_COUNT));
        let surface_table = be32(at + SURFACE_TABLE);
        seen.claim(surface_table, surfaces * 4, "a surface offset table");
        for (n, surface) in chunk.surfaces().enumerate() {
            let record = if n == 0 {
                at + SURFACE_BASE
            } else {
                be32(surface_table + n * 4)
            };
            seen.claim(record, SURFACE_LEN, "a surface record");
            seen.claim(
                record + SURFACE_LEN,
                surface.submeshes.len() * SUBMESH_LEN,
                "the submesh descriptors",
            );
            if let Some(decl) = &surface.decl {
                seen.claim(
                    be32(record + 0x38),
                    4 + decl.attributes.len() * 8,
                    "a vertex declaration",
                );
            }
            // The fallback chain a consumer actually uses: an inline chunk
            // declares no stride and its vertex buffer is still read.
            let stride = surface
                .declared_stride()
                .or_else(|| surface.solve_stride_without_a_box(data));
            for submesh in &surface.submeshes {
                if let Some(stride) = stride {
                    seen.claim(
                        submesh.vertex_offset,
                        submesh.vertex_count * stride,
                        "a vertex buffer",
                    );
                }
                seen.claim(
                    submesh.index_offset,
                    submesh.index_count * 2,
                    "an index buffer",
                );
            }
        }
    }
    seen
}
