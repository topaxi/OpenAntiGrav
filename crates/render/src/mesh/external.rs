//! Whether a `.vex` file's render geometry is in the file or beside it.
//!
//! Its own module because the test grew a second half when Wipeout 2048
//! arrived, and because [`super`] is at this project's 1,000-line ceiling.

use oag_formats::vex;

/// Whether this `.vex`'s render geometry lives outside the file.
///
/// True for a PS3 `.vex`, and that is the whole of the test: **Wipeout HD moved
/// its meshes into a `.rcsmodel` beside each `.vex`** - 643 of them, 686 MiB,
/// undecoded - and left the node tree, the transforms, the spline, the collision
/// soup and the pad volumes exactly where they were. So an HD model parses
/// perfectly and yields no triangles, which is indistinguishable from a decoder
/// bug unless something says so.
///
/// Byte order used to be the whole of the test, on the reasoning that a
/// big-endian `.vex` is a PS3 one and no PSP or PS2 pressing ships one. **That
/// is still true and is no longer sufficient**: Wipeout 2048 exports the same
/// way on a little-endian console, so a byte-order test answers `false` for a
/// file whose geometry is every bit as external as HD's, and the `.vex` then
/// goes to a batch decoder that fails on the first header with
/// `unsupported GU vertex type 0x0000`.
///
/// So the second half asks the file rather than the console: **a mesh batch
/// declaring vertex type `0` declares no vertex format**, which is what an
/// exporter writes when the vertices are somewhere else. Checked on the first
/// mesh node the tree carries, because the export decision is per file.
///
/// The byte-order half is kept ahead of it rather than replaced by it: an HD
/// `.vex` answers `true` on either test, and keeping the cheap one first means
/// this cannot start disagreeing with the twenty-eight circuits already
/// measured through it.
#[must_use]
pub fn geometry_is_external(data: &[u8]) -> bool {
    if !vex::has_magic(data) {
        return false;
    }
    if vex::byte_order(data) == oag_formats::ByteOrder::Big {
        return true;
    }
    let Ok(nodes) = vex::nodes(data) else {
        return false;
    };
    let Ok(classes) = vex::classes_of(data) else {
        return false;
    };
    let Some(mesh) = classes.mesh else {
        return false;
    };
    nodes
        .iter()
        .find(|node| node.class_id == mesh)
        .and_then(|node| vex::mesh_first_vertex_type(&data[node.payload()]))
        == Some(0)
}
