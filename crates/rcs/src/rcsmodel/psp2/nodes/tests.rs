//! What [`super`] is asserted to do, on a model built by hand: the node
//! table reads, and [`super::super::parse`] links every submesh record back
//! to its mesh object and node.

use super::super::{
    DESCRIPTOR_BASE, DESCRIPTOR_LEN, INDEX_POINTER, MAGIC, RELOCATION_LEN, VERTEX_POINTER, parse,
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

/// An image with `nodes` nodes (ids `100 + i`, name hashes `200 + i`, bind
/// translation `i` in x) and these mesh objects, whose submesh records sit
/// in a GPU section exactly as the disc packs them.
fn image(nodes: usize, shapes: &[Shape]) -> Vec<u8> {
    let mut s = Section(vec![0u8; 0x60]);
    s.0[0x10..0x12].copy_from_slice(&(nodes as u16).to_le_bytes());
    // All but the last node get a written bind matrix.
    s.0[0x12..0x14].copy_from_slice(&(nodes.saturating_sub(1) as u16).to_le_bytes());
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
    s.patch(NODE_NAME_HASHES, hashes_at as u32);
    s.patch(NODE_IDS, ids_at as u32);
    s.patch(NODE_BINDS, binds_at as u32);

    let mut gpu = Vec::new();
    let mut relocations = Vec::new();
    let submesh_count: usize = shapes.iter().map(|sh| sh.submeshes.len()).sum();
    s.0[MESH_COUNT..MESH_COUNT + 2].copy_from_slice(&(shapes.len() as u16).to_le_bytes());
    s.0[MESH_COUNT + 2..MESH_COUNT + 4].copy_from_slice(&(submesh_count as u16).to_le_bytes());
    let table_at = s.0.len();
    let mut table_sites = Vec::new();
    for _ in shapes {
        table_sites.push(s.u32(0));
    }
    s.patch(MESH_TABLE, table_at as u32);
    for (m, shape) in shapes.iter().enumerate() {
        let object_at = s.0.len();
        s.u32(0xaaaa_0000 + m as u32);
        s.u32(0xbbbb_0000 + m as u32);
        let node = shape.node.unwrap_or(NO_NODE);
        s.0.extend_from_slice(&node.to_le_bytes());
        s.0.extend_from_slice(&0x0101u16.to_le_bytes());
        s.u32(0);
        let name_site = s.u32(0);
        s.u32(shape.submeshes.len() as u32);
        s.u32(0);
        let list_site = s.u32(0);
        let name_at = s.0.len();
        s.0.extend_from_slice(shape.name.as_bytes());
        s.0.push(0);
        while !s.0.len().is_multiple_of(4) {
            s.0.push(0);
        }
        s.patch(name_site, name_at as u32);
        let list_at = s.0.len();
        let mut object_sites = Vec::new();
        for _ in &shape.submeshes {
            object_sites.push(s.u32(0));
        }
        s.patch(list_site, list_at as u32);
        for (k, &vertex_count) in shape.submeshes.iter().enumerate() {
            let submesh_object = s.0.len();
            s.patch(object_sites[k], submesh_object as u32);
            // The material index and the words beside it.
            s.u32(0);
            s.u32(0);
            s.u32(0xffff_ffff);
            s.u32(0);
            s.u32(0);
            s.u32(2);
            let record = s.0.len();
            assert_eq!(record - submesh_object, RECORD_IN_SUBMESH_OBJECT);
            s.u32(3);
            s.u32(vertex_count as u32);
            s.u32(0);
            s.u32(0);
            let index_at = gpu.len() as u32;
            for k in 0..3u16 {
                gpu.extend_from_slice(&(k % vertex_count as u16).to_le_bytes());
            }
            while gpu.len() % 4 != 0 {
                gpu.push(0);
            }
            let vertex_at = gpu.len() as u32;
            for v in 0..vertex_count {
                for a in 0..3 {
                    gpu.extend_from_slice(&((v * 3 + a) as f32).to_le_bytes());
                }
                gpu.extend_from_slice(&[0u8; 4]);
            }
            relocations.push((record + INDEX_POINTER) as u32);
            relocations.push((record + VERTEX_POINTER) as u32);
            let index_site = s.u32(index_at);
            assert_eq!(index_site, record + INDEX_POINTER);
            while s.0.len() < record + VERTEX_POINTER {
                s.u32(0);
            }
            let vertex_site = s.u32(vertex_at);
            assert_eq!(vertex_site, record + VERTEX_POINTER);
        }
        s.patch(table_sites[m], object_at as u32);
    }
    let cpu = s.0;

    let header_len = DESCRIPTOR_BASE + 2 * DESCRIPTOR_LEN + relocations.len() * RELOCATION_LEN;
    let mut out = vec![0u8; header_len];
    out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
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

#[test]
fn reads_the_node_table_and_links_every_submesh_to_its_mesh_and_node() {
    let file = image(
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
fn a_node_index_past_the_table_refuses_the_whole_table() {
    let file = image(
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
