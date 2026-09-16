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

/// Where the node count sits in the file header.
const NODE_COUNT: usize = 0x10;
/// Where the count of written bind matrices sits.
const BIND_COUNT: usize = 0x12;
/// Where the offset of the node name-hash array sits.
const NODE_NAME_HASHES: usize = 0x18;
/// Where the offset of the node id array sits.
const NODE_IDS: usize = 0x1c;
/// Where the offset of the node bind-matrix array sits.
const NODE_BINDS: usize = 0x20;
/// Where the mesh object count sits.
const MESH_COUNT: usize = 0x28;
/// Where the offset of the mesh object offset table sits.
const MESH_TABLE: usize = 0x2c;
/// A mesh object's node index when it has none.
pub const NO_NODE: u16 = 0xffff;
/// How far into a submesh object its record starts - the same
/// [`super::MATERIAL_INDEX_BEFORE_RECORD`] the material index was found by,
/// approached from the other side.
const RECORD_IN_SUBMESH_OBJECT: usize = super::MATERIAL_INDEX_BEFORE_RECORD;

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

/// Reads the node table and mesh objects out of the CPU section.
///
/// `None` when the header does not check out - a count that runs past the
/// section, an offset that does not resolve - which is reported by the
/// caller as "no node table" rather than as a decode error, on the same terms
/// [`super::material::read`] returns an empty table.
#[must_use]
pub fn read(cpu: &[u8]) -> Option<Scene> {
    let node_count = usize::from(u16_at(cpu, NODE_COUNT)?);
    let bind_count = usize::from(u16_at(cpu, BIND_COUNT)?);
    let hashes_at = u32_at(cpu, NODE_NAME_HASHES)? as usize;
    let ids_at = u32_at(cpu, NODE_IDS)? as usize;
    let binds_at = u32_at(cpu, NODE_BINDS)? as usize;
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

    let mesh_count = usize::from(u16_at(cpu, MESH_COUNT)?);
    let table = u32_at(cpu, MESH_TABLE)? as usize;
    let mut meshes = Vec::with_capacity(mesh_count);
    for m in 0..mesh_count {
        let at = u32_at(cpu, table.checked_add(m * 4)?)? as usize;
        let name_hash = u32_at(cpu, at)?;
        let node = u16_at(cpu, at + 0x08)?;
        let flags = u16_at(cpu, at + 0x0a)?;
        let name = cstr_at(cpu, u32_at(cpu, at + 0x10)? as usize)?;
        let submesh_count = u32_at(cpu, at + 0x14)? as usize;
        let list = u32_at(cpu, at + 0x1c)? as usize;
        let mut submesh_records = Vec::with_capacity(submesh_count.min(cpu.len() / 4));
        for s in 0..submesh_count {
            let object = u32_at(cpu, list.checked_add(s * 4)?)? as usize;
            submesh_records.push(object.checked_add(RECORD_IN_SUBMESH_OBJECT)?);
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
