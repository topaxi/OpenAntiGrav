//! What [`super`] is asserted to do, on a model built by hand: the node
//! table reads, and [`super::super::parse`] links every submesh record back
//! to its mesh object and node.

use super::super::{
    DESCRIPTOR_BASE, DESCRIPTOR_LEN, INDEX_POINTER, KNOWN_BUFFER_POINTER_GAPS, MAGIC,
    PS4_BUFFER_POINTER_GAP, PS4_HEADER_WORD, RELOCATION_LEN, parse,
};
use super::*;
use crate::rcsskeleton::tests::Section;

/// One mesh object to author: its name, node, and how many vertices each
/// of its submeshes has (each submesh is one triangle over them).
struct Shape {
    name: &'static str,
    node: Option<u16>,
    submeshes: Vec<usize>,
}

/// Writes `value` at `at` in the pointer width `layout` names.
fn put_pointer(s: &mut Section, at: usize, value: usize, layout: Layout) {
    s.0[at..at + layout.pointer].copy_from_slice(&(value as u64).to_le_bytes()[..layout.pointer]);
}

/// Appends `n` zero bytes and returns where they start.
fn reserve(s: &mut Section, n: usize) -> usize {
    let at = s.0.len();
    s.0.resize(at + n, 0);
    at
}

/// An image with `nodes` nodes (ids `100 + i`, name hashes `200 + i`, bind
/// translation `i` in x) and these mesh objects, whose submesh records sit
/// in a GPU section exactly as the disc packs them, in `layout`'s pointer
/// width. Every field goes where `layout` says, so the same builder writes the
/// Vita's file and the PS4's.
fn image(layout: Layout, nodes: usize, shapes: &[Shape]) -> Vec<u8> {
    let ps4 = layout.pointer == 8;
    let gap = if ps4 {
        PS4_BUFFER_POINTER_GAP
    } else {
        KNOWN_BUFFER_POINTER_GAPS[0]
    };
    let mut s = Section(vec![0u8; 0x60]);
    s.0[NODE_COUNT..NODE_COUNT + 2].copy_from_slice(&(nodes as u16).to_le_bytes());
    // All but the last node get a written bind matrix.
    s.0[BIND_COUNT..BIND_COUNT + 2]
        .copy_from_slice(&(nodes.saturating_sub(1) as u16).to_le_bytes());
    let hashes_at = s.0.len();
    for i in 0..nodes {
        s.u32(200 + i as u32);
    }
    let ids_at = s.0.len();
    for i in 0..nodes {
        s.u32(100 + i as u32);
    }
    let binds_at = s.0.len();
    for i in 0..nodes {
        let mut m = crate::rcsskeleton::IDENTITY;
        m[12] = i as f32;
        s.f32s(&m);
    }
    put_pointer(&mut s, layout.node_name_hashes, hashes_at, layout);
    put_pointer(&mut s, layout.node_ids, ids_at, layout);
    put_pointer(&mut s, layout.node_binds, binds_at, layout);

    let mut gpu = Vec::new();
    let mut relocations = Vec::new();
    let submesh_count: usize = shapes.iter().map(|sh| sh.submeshes.len()).sum();
    let mesh_count = layout.mesh_count;
    s.0[mesh_count..mesh_count + 2].copy_from_slice(&(shapes.len() as u16).to_le_bytes());
    s.0[mesh_count + 2..mesh_count + 4].copy_from_slice(&(submesh_count as u16).to_le_bytes());
    let table_at = reserve(&mut s, shapes.len() * layout.pointer);
    put_pointer(&mut s, layout.mesh_table, table_at, layout);
    for (m, shape) in shapes.iter().enumerate() {
        let object_at = reserve(&mut s, 0x30);
        s.0[object_at..object_at + 4].copy_from_slice(&(0xaaaa_0000 + m as u32).to_le_bytes());
        s.0[object_at + 4..object_at + 8].copy_from_slice(&(0xbbbb_0000 + m as u32).to_le_bytes());
        let node = shape.node.unwrap_or(NO_NODE);
        s.0[object_at + MESH_NODE..object_at + MESH_NODE + 2].copy_from_slice(&node.to_le_bytes());
        s.0[object_at + MESH_FLAGS..object_at + MESH_FLAGS + 2]
            .copy_from_slice(&0x0101u16.to_le_bytes());
        let count_at = object_at + layout.mesh_submesh_count;
        s.0[count_at..count_at + 4].copy_from_slice(&(shape.submeshes.len() as u32).to_le_bytes());
        put_pointer(&mut s, table_at + m * layout.pointer, object_at, layout);

        let name_at = s.0.len();
        s.0.extend_from_slice(shape.name.as_bytes());
        s.0.push(0);
        put_pointer(&mut s, object_at + layout.mesh_name, name_at, layout);
        let list_at = reserve(&mut s, shape.submeshes.len() * layout.pointer);
        put_pointer(
            &mut s,
            object_at + layout.mesh_submesh_list,
            list_at,
            layout,
        );
        for (k, &vertex_count) in shape.submeshes.iter().enumerate() {
            let submesh_object = s.0.len();
            put_pointer(&mut s, list_at + k * layout.pointer, submesh_object, layout);
            // The material index and the words beside it, then the record.
            let head = reserve(&mut s, layout.record_in_submesh_object);
            s.0[head + 8..head + 12].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
            let record = reserve(&mut s, INDEX_POINTER + gap + layout.pointer);
            assert_eq!(record - submesh_object, layout.record_in_submesh_object);
            s.0[record..record + 4].copy_from_slice(&3u32.to_le_bytes());
            s.0[record + 4..record + 8].copy_from_slice(&(vertex_count as u32).to_le_bytes());
            let index_at = gpu.len();
            for k in 0..3u16 {
                gpu.extend_from_slice(&(k % vertex_count as u16).to_le_bytes());
            }
            while gpu.len() % 4 != 0 {
                gpu.push(0);
            }
            let vertex_at = gpu.len();
            for v in 0..vertex_count {
                for a in 0..3 {
                    gpu.extend_from_slice(&((v * 3 + a) as f32).to_le_bytes());
                }
                gpu.extend_from_slice(&[0u8; 4]);
            }
            relocations.push((record + INDEX_POINTER) as u32);
            relocations.push((record + INDEX_POINTER + gap) as u32);
            put_pointer(&mut s, record + INDEX_POINTER, index_at, layout);
            put_pointer(&mut s, record + INDEX_POINTER + gap, vertex_at, layout);
        }
    }
    let cpu = s.0;

    let header_len = DESCRIPTOR_BASE + 2 * DESCRIPTOR_LEN + relocations.len() * RELOCATION_LEN;
    let mut out = vec![0u8; header_len];
    out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    if ps4 {
        out[4..8].copy_from_slice(&PS4_HEADER_WORD.to_le_bytes());
    }
    out[0x08..0x0c].copy_from_slice(&2u32.to_le_bytes());
    out[0x0c..0x10].copy_from_slice(&(header_len as u32).to_le_bytes());
    out[0x24..0x28].copy_from_slice(&(cpu.len() as u32).to_le_bytes());
    out[0x44..0x48].copy_from_slice(&(gpu.len() as u32).to_le_bytes());
    out[0x4c..0x50].copy_from_slice(&(relocations.len() as u32).to_le_bytes());
    let table = DESCRIPTOR_BASE + 2 * DESCRIPTOR_LEN;
    for (i, &offset) in relocations.iter().enumerate() {
        let at = table + i * RELOCATION_LEN;
        out[at..at + 4].copy_from_slice(&offset.to_le_bytes());
    }
    out.extend_from_slice(&cpu);
    out.extend_from_slice(&gpu);
    out
}

fn reads_the_node_table_and_links_every_submesh(layout: Layout) {
    let file = image(
        layout,
        3,
        &[
            Shape {
                name: "ns:staticShape",
                node: None,
                submeshes: vec![4],
            },
            Shape {
                name: "ns:boatShape",
                node: Some(2),
                submeshes: vec![3, 5],
            },
        ],
    );
    let model = parse(&file).expect("parses");
    assert_eq!(model.scene.nodes.len(), 3);
    assert_eq!(model.scene.nodes[2].id, 102);
    assert_eq!(model.scene.nodes[2].name_hash, 202);
    assert_eq!(model.scene.nodes[1].bind.map(|m| m[12]), Some(1.0));
    assert_eq!(model.scene.nodes[2].bind, None, "past the written count");
    assert_eq!(model.scene.node_by_id(101), Some(1));
    assert_eq!(model.scene.meshes.len(), 2);
    assert_eq!(model.scene.meshes[1].name, "ns:boatShape");
    assert_eq!(model.scene.meshes[1].node, Some(2));
    assert_eq!(model.scene.meshes[0].node, None);
    assert_eq!(model.scene.meshes[1].submesh_records.len(), 2);

    assert_eq!(model.submeshes.len(), 3);
    let by_mesh: Vec<(Option<usize>, Option<usize>, usize)> = model
        .submeshes
        .iter()
        .map(|s| (s.mesh, s.node, s.positions.len()))
        .collect();
    assert_eq!(
        by_mesh,
        vec![
            (Some(0), None, 4),
            (Some(1), Some(2), 3),
            (Some(1), Some(2), 5)
        ]
    );
}

#[test]
fn reads_the_node_table_and_links_every_submesh_to_its_mesh_and_node() {
    reads_the_node_table_and_links_every_submesh(Layout::VITA);
}

/// The same table in the PS4's pointer width: every offset a `u64`, the
/// fields in the places [`Layout::PS4`] names, and [`parse`] choosing it off
/// the header word alone.
#[test]
fn reads_the_ps4_node_table_and_links_every_submesh_to_its_mesh_and_node() {
    reads_the_node_table_and_links_every_submesh(Layout::PS4);
}

/// The PS4 layout is the Vita's with widened offsets, so reading a PS4 file
/// through the Vita layout is a refusal or a wrong table and never a right
/// one - the reason [`Layout::for_ps4`] exists and is keyed on
/// [`super::super::is_ps4`].
#[test]
fn the_vita_layout_does_not_read_a_ps4_table() {
    let file = image(
        Layout::PS4,
        3,
        &[Shape {
            name: "ns:boatShape",
            node: Some(2),
            submeshes: vec![3],
        }],
    );
    let cpu_start = u32::from_le_bytes(file[0x0c..0x10].try_into().unwrap()) as usize;
    let cpu = &file[cpu_start..];
    let right = read(cpu, Layout::PS4).expect("the PS4 layout reads it");
    assert_eq!(right.meshes.len(), 1);
    assert_ne!(read(cpu, Layout::VITA), Some(right));
}

fn a_node_index_past_the_table_refuses_the_whole_table(layout: Layout) {
    let file = image(
        layout,
        1,
        &[Shape {
            name: "x",
            node: Some(7),
            submeshes: vec![3],
        }],
    );
    let model = parse(&file).expect("the geometry still parses");
    assert!(model.scene.nodes.is_empty());
    assert!(
        model
            .submeshes
            .iter()
            .all(|s| s.mesh.is_none() && s.node.is_none())
    );
}

#[test]
fn a_node_index_past_the_table_refuses_the_whole_table_on_both_widths() {
    a_node_index_past_the_table_refuses_the_whole_table(Layout::VITA);
    a_node_index_past_the_table_refuses_the_whole_table(Layout::PS4);
}

/// A 64-bit pointer with its high half set is not an offset into a section.
#[test]
fn a_ps4_pointer_with_a_high_half_is_refused_not_truncated() {
    let file = image(
        Layout::PS4,
        1,
        &[Shape {
            name: "x",
            node: None,
            submeshes: vec![3],
        }],
    );
    let cpu_start = u32::from_le_bytes(file[0x0c..0x10].try_into().unwrap()) as usize;
    let mut cpu = file[cpu_start..].to_vec();
    let at = Layout::PS4.mesh_table + 4;
    cpu[at..at + 4].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(read(&cpu, Layout::PS4), None);
}
