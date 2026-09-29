//! Wipeout 2048's node table and mesh objects: what places a submesh, and
//! what a `.rcsskeleton` beside the model animates.
//!
//! **Found by walking the file header rather than by pattern**, once the
//! sibling `.rcsskeleton`'s 32-bit node ids turned up verbatim inside the
//! model's CPU section - `altima`'s 165, contiguous and in the skeleton's own
//! order - and the header word that reaches them was traced back from there.
//! See `docs/formats/2048-animation.md`.
//!
//! ```text
//! file header (section B, from its own start):
//!   +0x10  u16   node count
//!   +0x12  u16   how many of the bind matrices below are written; the
//!                rest are all zeros. Equal to the node count on 35 of the
//!                49 skeleton-bearing files and on every craft; a
//!                `trackZone` writes 103 of `altima`'s 966
//!   +0x18  u32   offset of u32[node count]: `~crc32` of each node's full
//!                Maya path (`trackpart_animations:RiverBoat4`), the same
//!                hash Wipeout HD names everything by
//!   +0x1c  u32   offset of u32[node count]: each node's id - the value the
//!                `.rcsskeleton` and `.rcsanimclip` beside the model bind
//!                by, in an unidentified hash (see the doc page)
//!   +0x20  u32   offset of f32[16][node count]: each node's world matrix at
//!                bind, row-major with the translation in row 3, composed
//!                from the skeleton's own scale/rotation/translation and
//!                **without** its pivots - see [`Node::bind`]
//!   +0x28  u16   mesh object count
//!   +0x2a  u16   submesh count - every submesh record the relocation-pair
//!                search finds is reachable from exactly one mesh object,
//!                on all 49 skeleton-bearing files and the craft checked
//!   +0x2c  u32   offset of u32[mesh count]: each mesh object's offset
//!
//! one mesh object:
//!   +0x00  u32   `~crc32` of the shape's full path (`...:RiverBoat4Shape`)
//!   +0x04  u32   unread - unique per mesh, matches no hash tried
//!   +0x08  u16   node index into the table above, or 0xffff for a static
//!                mesh authored in world space
//!   +0x0a  u16   unread flags: 0x0101 or 0x0201 on a node-bound mesh,
//!                0x0101 (or 0x0306, 26 times) on a static one
//!   +0x10  u32   offset of the shape's name, NUL-terminated
//!   +0x14  u32   submesh count
//!   +0x1c  u32   offset of u32[submesh count]: each submesh object's offset;
//!                its record ([`super::SubMesh::record`]) starts
//!                [`super::MATERIAL_INDEX_BEFORE_RECORD`] bytes in, which is
//!                the same object the material index was found at the head of
//! ```
//!
//! **A node-bound mesh's vertices are in that node's space, not the
//! world's.** `super`'s module doc says no transform is composed onto a
//! circuit - true of the 1,023 meshes on `altima` with no node, and wrong
//! for the 130 with one, which drew a median 1,086 units from where the
//! original puts them until this table was read. The evidence is
//! cross-title: 2048's DLC re-ships Wipeout HD's circuits, and on Anulpha
//! Pass the vertex box of 73 of 78 node-bound meshes equals HD's own
//! authored box for the same mesh *in its `Anim Transform`'s space*, to the
//! float, and none of them equals it in world space.

use super::container::u32_at as u32_or_err;

/// Where the node count sits in the file header, on both widths.
const NODE_COUNT: usize = 0x10;
/// Where the count of written bind matrices sits, on both widths.
const BIND_COUNT: usize = 0x12;
/// A mesh object's node index when it has none.
pub const NO_NODE: u16 = 0xffff;
/// Where a mesh object's node index and flags sit, on both widths.
const MESH_NODE: usize = 0x08;
/// See [`MESH_NODE`].
const MESH_FLAGS: usize = 0x0a;

/// Where each field of the table sits, for one pointer width.
///
/// The two shipped shapes are [`Layout::VITA`] and [`Layout::PS4`]; a caller
/// picks by [`super::is_ps4`]. Every offset here is a place in the CPU
/// section, and every pointer they name is read `pointer` bytes wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Bytes per pointer: 4 on the Vita, 8 on the PS4.
    pub pointer: usize,
    /// File header: the offset of the node name-hash array.
    pub node_name_hashes: usize,
    /// File header: the offset of the node id array.
    pub node_ids: usize,
    /// File header: the offset of the node bind-matrix array.
    pub node_binds: usize,
    /// File header: the mesh object count (a `u16`).
    pub mesh_count: usize,
    /// File header: the offset of the mesh object offset table.
    pub mesh_table: usize,
    /// Mesh object: the offset of the shape's name.
    pub mesh_name: usize,
    /// Mesh object: the submesh count (a `u32`).
    pub mesh_submesh_count: usize,
    /// Mesh object: the offset of the submesh object offset table.
    pub mesh_submesh_list: usize,
    /// How far into a submesh object its record starts - the same
    /// [`super::MATERIAL_INDEX_BEFORE_RECORD`] (or its PS4 counterpart) the
    /// material index was found by, approached from the other side.
    pub record_in_submesh_object: usize,
}

impl Layout {
    /// Wipeout 2048 on the Vita, 32-bit pointers.
    pub const VITA: Self = Self {
        pointer: 4,
        node_name_hashes: 0x18,
        node_ids: 0x1c,
        node_binds: 0x20,
        mesh_count: 0x28,
        mesh_table: 0x2c,
        mesh_name: 0x10,
        mesh_submesh_count: 0x14,
        mesh_submesh_list: 0x1c,
        record_in_submesh_object: super::MATERIAL_INDEX_BEFORE_RECORD,
    };

    /// The PS4 Omega Collection, 64-bit pointers. See the module doc for the
    /// evidence and [`Layout::VITA`] for the shape it widens.
    pub const PS4: Self = Self {
        pointer: 8,
        node_name_hashes: 0x18,
        node_ids: 0x20,
        node_binds: 0x28,
        mesh_count: 0x38,
        mesh_table: 0x40,
        mesh_name: 0x18,
        mesh_submesh_count: 0x20,
        mesh_submesh_list: 0x28,
        record_in_submesh_object: super::PS4_MATERIAL_INDEX_BEFORE_RECORD,
    };

    /// The layout a file of this pointer width uses.
    #[must_use]
    pub const fn for_ps4(ps4: bool) -> Self {
        if ps4 { Self::PS4 } else { Self::VITA }
    }
}

/// One node of the model's own table.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// `~crc32` of the node's full Maya path.
    pub name_hash: u32,
    /// The id the `.rcsskeleton` and `.rcsanimclip` bind this node by.
    pub id: u32,
    /// The node's world matrix at bind, row-major with the translation in
    /// row 3 (`oag_vex::vex::transform`'s convention), or `None` for a node
    /// past the header's written count, whose slot is all zeros.
    ///
    /// Composed by the exporter from the skeleton's scale, rotation and
    /// translation alone - a node with a pivot lands somewhere else at
    /// runtime, so this is the file's own number and not where the node is
    /// drawn. See `docs/formats/2048-animation.md`, "The bind matrices leave
    /// the pivot out".
    pub bind: Option<[f32; 16]>,
}

/// One mesh object: a named shape, its node, and the submeshes under it.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshObject {
    /// The shape's full Maya path.
    pub name: String,
    /// `~crc32` of [`Self::name`], as the file carries it.
    pub name_hash: u32,
    /// Index into [`Scene::nodes`], or `None` for a static mesh.
    pub node: Option<usize>,
    /// The unread `+0x0a` word.
    pub flags: u16,
    /// Offsets, within the CPU section, of each submesh's record - the value
    /// [`super::SubMesh::record`] carries for the same submesh.
    pub submesh_records: Vec<usize>,
}

/// The node table and mesh objects of one model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    /// Every node, in table order.
    pub nodes: Vec<Node>,
    /// Every mesh object, in table order.
    pub meshes: Vec<MeshObject>,
}

impl Scene {
    /// The index in [`Self::nodes`] of the node with `id`, if any.
    #[must_use]
    pub fn node_by_id(&self, id: u32) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// The mesh object each submesh record belongs to: `(record, mesh
    /// index)`, sorted by record.
    #[must_use]
    pub fn mesh_by_record(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = self
            .meshes
            .iter()
            .enumerate()
            .flat_map(|(m, mesh)| mesh.submesh_records.iter().map(move |&r| (r, m)))
            .collect();
        out.sort_unstable();
        out
    }
}

/// Reads the node table and mesh objects out of the CPU section, in the
/// pointer width `layout` names.
///
/// `None` when the header does not check out - a count that runs past the
/// section, an offset that does not resolve, a 64-bit pointer with its high
/// half set - which is reported by the caller as "no node table" rather than
/// as a decode error, on the same terms [`super::material::read`] returns an
/// empty table.
#[must_use]
pub fn read(cpu: &[u8], layout: Layout) -> Option<Scene> {
    let node_count = usize::from(u16_at(cpu, NODE_COUNT)?);
    let bind_count = usize::from(u16_at(cpu, BIND_COUNT)?);
    let hashes_at = pointer_at(cpu, layout.node_name_hashes, layout)?;
    let ids_at = pointer_at(cpu, layout.node_ids, layout)?;
    let binds_at = pointer_at(cpu, layout.node_binds, layout)?;
    let mut nodes = Vec::with_capacity(node_count);
    for i in 0..node_count {
        let name_hash = u32_at(cpu, hashes_at.checked_add(i * 4)?)?;
        let id = u32_at(cpu, ids_at.checked_add(i * 4)?)?;
        let at = binds_at.checked_add(i * 64)?;
        let bytes = cpu.get(at..at + 64)?;
        let bind: [f32; 16] = std::array::from_fn(|k| {
            f32::from_le_bytes(bytes[k * 4..k * 4 + 4].try_into().expect("four bytes"))
        });
        let bind = (i < bind_count).then_some(bind);
        nodes.push(Node {
            name_hash,
            id,
            bind,
        });
    }

    let mesh_count = usize::from(u16_at(cpu, layout.mesh_count)?);
    let table = pointer_at(cpu, layout.mesh_table, layout)?;
    let mut meshes = Vec::with_capacity(mesh_count);
    for m in 0..mesh_count {
        let at = pointer_at(cpu, table.checked_add(m * layout.pointer)?, layout)?;
        let name_hash = u32_at(cpu, at)?;
        let node = u16_at(cpu, at + MESH_NODE)?;
        let flags = u16_at(cpu, at + MESH_FLAGS)?;
        let name = cstr_at(cpu, pointer_at(cpu, at + layout.mesh_name, layout)?)?;
        let submesh_count = u32_at(cpu, at + layout.mesh_submesh_count)? as usize;
        let list = pointer_at(cpu, at + layout.mesh_submesh_list, layout)?;
        let mut submesh_records = Vec::with_capacity(submesh_count.min(cpu.len() / 4));
        for s in 0..submesh_count {
            let object = pointer_at(cpu, list.checked_add(s * layout.pointer)?, layout)?;
            submesh_records.push(object.checked_add(layout.record_in_submesh_object)?);
        }
        let node = if node == NO_NODE {
            None
        } else {
            let index = usize::from(node);
            if index >= nodes.len() {
                return None;
            }
            Some(index)
        };
        meshes.push(MeshObject {
            name,
            name_hash,
            node,
            flags,
            submesh_records,
        });
    }
    Some(Scene { nodes, meshes })
}

/// An offset stored at `at`, `layout.pointer` bytes wide.
///
/// A 64-bit pointer whose high half is set is refused, not truncated: on the
/// Omega files every one of these is a small offset within the section.
fn pointer_at(cpu: &[u8], at: usize, layout: Layout) -> Option<usize> {
    if layout.pointer == 8 {
        let bytes = cpu.get(at..at.checked_add(8)?)?;
        let value = u64::from_le_bytes(bytes.try_into().expect("eight bytes"));
        usize::try_from(value).ok()
    } else {
        u32_at(cpu, at).map(|v| v as usize)
    }
}

fn u32_at(cpu: &[u8], at: usize) -> Option<u32> {
    u32_or_err(cpu, at, "node table").ok()
}

fn u16_at(cpu: &[u8], at: usize) -> Option<u16> {
    cpu.get(at..at + 2)
        .map(|b| u16::from_le_bytes(b.try_into().expect("two bytes")))
}

fn cstr_at(cpu: &[u8], at: usize) -> Option<String> {
    let bytes = cpu.get(at..)?;
    let end = bytes.iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

#[cfg(test)]
mod tests;
