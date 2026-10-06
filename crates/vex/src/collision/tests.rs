//! What the collision-chunk reader in [`super`] is asserted to do: the triangle
//! soup it builds and the chunks it refuses.

use super::*;
use crate::vex::classes::V6;

/// [`parse_chunks`] on a fixture, every one of which is written little-endian.
fn parse_chunks_le(payload: &[u8]) -> Result<CollisionGeometry> {
    parse_chunks(payload, ByteOrder::Little)
}

/// Encodes one chunk: `{u32 kind, u16 stride, u16 count, body}`.
fn chunk_with_stride(kind: u32, stride: u16, count: usize, body: &[u8]) -> Vec<u8> {
    let mut out = kind.to_le_bytes().to_vec();
    out.extend(stride.to_le_bytes());
    out.extend((count as u16).to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// Encodes one chunk with the stride its type implies, as real files do.
fn chunk(kind: u32, count: usize, body: &[u8]) -> Vec<u8> {
    let stride = chunk_stride(kind).unwrap_or(0) as u16;
    chunk_with_stride(kind, stride, count, body)
}

fn vertex_chunk(vertices: &[[f32; 3]]) -> Vec<u8> {
    let mut body = Vec::new();
    for v in vertices {
        for c in v {
            body.extend(c.to_le_bytes());
        }
    }
    chunk(CHUNK_VERTICES, vertices.len(), &body)
}

fn triangle_chunk(triangles: &[[u16; 3]]) -> Vec<u8> {
    let mut body = Vec::new();
    for t in triangles {
        for i in t {
            body.extend(i.to_le_bytes());
        }
    }
    chunk(CHUNK_TRIANGLES, triangles.len(), &body)
}

fn scalar_chunk(scalars: &[f32]) -> Vec<u8> {
    let mut body = Vec::new();
    for s in scalars {
        body.extend(s.to_le_bytes());
    }
    chunk(CHUNK_VERTEX_SCALARS, scalars.len(), &body)
}

/// Builds a payload from already-encoded chunks, one list per object.
fn build(version: u32, objects: &[Vec<Vec<u8>>]) -> Vec<u8> {
    let mut out = version.to_le_bytes().to_vec();
    out.extend((objects.len() as u32).to_le_bytes());
    for chunks in objects {
        out.extend((chunks.len() as u32).to_le_bytes());
        for c in chunks {
            out.extend_from_slice(c);
        }
    }
    out
}

/// A quad in the shape real objects take: chunk 1, then 3, then 2.
fn quad() -> Vec<Vec<u8>> {
    vec![
        vertex_chunk(&QUAD_VERTICES),
        scalar_chunk(&QUAD_SCALARS),
        triangle_chunk(&QUAD_TRIANGLES),
    ]
}

const QUAD_VERTICES: [[f32; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [10.0, 0.0, 0.0],
    [10.0, 0.0, 20.0],
    [0.0, -1.5, 20.0],
];
const QUAD_TRIANGLES: [[u16; 3]; 2] = [[0, 1, 2], [0, 2, 3]];
const QUAD_SCALARS: [f32; 4] = [0.25, 0.5, 0.75, 1.0];

#[test]
fn a_well_formed_object_decodes_to_its_triangle_soup() {
    let payload = build(HEADER_WORD, &[quad()]);
    let geometry = parse_chunks_le(&payload).expect("parse");

    assert_eq!(geometry.version, HEADER_WORD);
    assert_eq!(geometry.meshes.len(), 1);
    let mesh = &geometry.meshes[0];
    assert_eq!(mesh.vertices, QUAD_VERTICES);
    assert_eq!(mesh.triangles, QUAD_TRIANGLES);
    assert_eq!(mesh.vertex_scalars, QUAD_SCALARS);
    assert!(mesh.has_vertex_scalars());
    assert_eq!(geometry.vertex_count(), 4);
    assert_eq!(geometry.triangle_count(), 2);
    assert_eq!(
        mesh.triangle(0),
        Some([QUAD_VERTICES[0], QUAD_VERTICES[1], QUAD_VERTICES[2]])
    );
    assert_eq!(mesh.triangle(2), None);
    assert_eq!(mesh.bounds(), Some(([0.0, -1.5, 0.0], [10.0, 0.0, 20.0])));
    // The strides real files write, recovered rather than assumed.
    assert_eq!(
        mesh.chunks.iter().map(|c| c.stride).collect::<Vec<_>>(),
        [
            VERTEX_STRIDE as u16,
            SCALAR_STRIDE as u16,
            TRIANGLE_STRIDE as u16
        ]
    );
}

/// A correct parse accounts for every byte (the synthetic version of
/// `collision_ground_truth.rs`'s assertion against 319 real nodes).
#[test]
fn the_decoded_structure_accounts_for_every_byte() {
    let cases: Vec<Vec<Vec<Vec<u8>>>> = vec![
        vec![quad()],
        vec![quad(), quad()],
        // Both the scalar and triangle chunks are optional to the walk.
        vec![vec![vertex_chunk(&QUAD_VERTICES)]],
        vec![vec![
            vertex_chunk(&QUAD_VERTICES),
            triangle_chunk(&QUAD_TRIANGLES),
        ]],
        // An empty object, which the format permits: zero chunks.
        vec![vec![]],
        vec![],
    ];

    for objects in cases {
        let payload = build(HEADER_WORD, &objects);
        let geometry = parse_chunks_le(&payload).expect("parse");
        assert_eq!(
            geometry.encoded_len(),
            payload.len(),
            "{} objects do not account for the payload",
            objects.len()
        );
    }
}

/// A node payload is padded to 16 bytes with zeros, so the walk stops short of
/// its declared length; `padded_len` is the check, enforced by `from_vex`.
#[test]
fn the_payload_closes_after_its_alignment_padding() {
    let payload = build(HEADER_WORD, &[quad()]);
    let geometry = parse_chunks_le(&payload).expect("parse");
    // The fixture happens to be 16-byte aligned already.
    assert_eq!(geometry.encoded_len() % PAYLOAD_ALIGN, 0);
    assert_eq!(geometry.padded_len(), geometry.encoded_len());

    // An odd triangle count is what leaves real payloads needing padding.
    let odd = build(
        HEADER_WORD,
        &[vec![
            vertex_chunk(&QUAD_VERTICES),
            scalar_chunk(&QUAD_SCALARS),
            triangle_chunk(&[[0, 1, 2]]),
        ]],
    );
    let geometry = parse_chunks_le(&odd).expect("parse");
    assert_eq!(geometry.encoded_len(), odd.len());
    assert!(geometry.padded_len() > geometry.encoded_len());
    assert_eq!(geometry.padded_len() % PAYLOAD_ALIGN, 0);
}

/// Chunks are located by type, not position: nothing in the format says shipped
/// order (1, 3, 2) must hold, and positional reads would transpose an array.
#[test]
fn chunks_may_appear_in_any_order() {
    let ordered = build(
        HEADER_WORD,
        &[vec![
            triangle_chunk(&QUAD_TRIANGLES),
            vertex_chunk(&QUAD_VERTICES),
            scalar_chunk(&QUAD_SCALARS),
        ]],
    );
    let mesh = &parse_chunks_le(&ordered).expect("parse").meshes[0];
    assert_eq!(mesh.vertices, QUAD_VERTICES);
    assert_eq!(mesh.triangles, QUAD_TRIANGLES);
    assert_eq!(mesh.vertex_scalars, QUAD_SCALARS);
    // The order found is retained: a ground-truth run reports it.
    assert_eq!(
        mesh.chunks.iter().map(|c| c.kind).collect::<Vec<_>>(),
        [CHUNK_TRIANGLES, CHUNK_VERTICES, CHUNK_VERTEX_SCALARS]
    );
}

/// Every prefix of a valid payload must fail, not panic.
#[test]
fn a_truncated_payload_is_refused_rather_than_read_past() {
    let payload = build(HEADER_WORD, &[quad(), quad()]);
    for len in 0..payload.len() {
        assert!(
            parse_chunks_le(&payload[..len]).is_err(),
            "truncating to {len} of {} bytes should not parse",
            payload.len()
        );
    }
}

#[test]
fn an_out_of_range_triangle_index_is_rejected() {
    let payload = build(
        HEADER_WORD,
        &[vec![
            vertex_chunk(&QUAD_VERTICES),
            triangle_chunk(&[[0, 1, 2], [0, 4, 3]]),
        ]],
    );
    assert_eq!(
        parse_chunks_le(&payload),
        Err(Error::BadTriangleIndex {
            object: 0,
            triangle: 1,
            index: 4,
            vertex_count: 4,
        })
    );
}

#[test]
fn an_absent_scalar_chunk_yields_neutral_scalars() {
    let payload = build(
        HEADER_WORD,
        &[vec![
            vertex_chunk(&QUAD_VERTICES),
            triangle_chunk(&QUAD_TRIANGLES),
        ]],
    );
    let mesh = &parse_chunks_le(&payload).expect("parse").meshes[0];
    assert!(!mesh.has_vertex_scalars());
    assert_eq!(mesh.vertex_scalars, vec![DEFAULT_VERTEX_SCALAR; 4]);
    // The original's averaging function returns this for a missing array.
    assert_eq!(mesh.avg_vertex_scalar(0), Some(DEFAULT_VERTEX_SCALAR));
}

/// Chunk 3 being per-vertex was a prediction that held on every shipped object.
/// A per-*triangle*-sized chunk is still refused, with both counts in the error.
#[test]
fn a_scalar_chunk_that_is_not_one_per_vertex_is_refused() {
    let payload = build(
        HEADER_WORD,
        &[vec![
            vertex_chunk(&QUAD_VERTICES),
            triangle_chunk(&QUAD_TRIANGLES),
            scalar_chunk(&[0.5, 0.5]),
        ]],
    );
    assert_eq!(
        parse_chunks_le(&payload),
        Err(Error::ScalarCountMismatch {
            object: 0,
            scalars: 2,
            vertices: 4,
            triangles: 2,
        })
    );
}

#[test]
fn averaging_a_triangles_scalars_is_the_mean_of_its_corners() {
    let mesh = &parse_chunks_le(&build(HEADER_WORD, &[quad()]))
        .expect("parse")
        .meshes[0];
    // Triangle 0 is vertices 0, 1, 2: 0.25, 0.5, 0.75.
    assert_eq!(mesh.avg_vertex_scalar(0), Some(0.5));
    // Triangle 1 is vertices 0, 2, 3: 0.25, 0.75, 1.0.
    let avg = mesh.avg_vertex_scalar(1).expect("triangle 1");
    assert!((avg - 2.0 / 3.0).abs() < 1e-6, "{avg}");
    assert_eq!(mesh.avg_vertex_scalar(2), None);
}

/// A fourth chunk type is fatal: its contents would be unknown, and quietly
/// dropping data is worse than stopping.
#[test]
fn an_unknown_chunk_type_is_fatal_rather_than_skipped() {
    let payload = build(
        HEADER_WORD,
        &[vec![
            vertex_chunk(&QUAD_VERTICES),
            chunk_with_stride(4, 4, 2, &[0u8; 8]),
            triangle_chunk(&QUAD_TRIANGLES),
        ]],
    );
    assert_eq!(
        parse_chunks_le(&payload),
        Err(Error::UnknownChunk { object: 0, kind: 4 })
    );
    assert_eq!(chunk_stride(4), None);
}

/// The chunk header's stride field is the walk's alignment check: a mismatch
/// means the cursor is not where the parser thinks.
#[test]
fn a_chunk_whose_declared_stride_contradicts_its_type_is_refused() {
    let payload = build(
        HEADER_WORD,
        &[vec![chunk_with_stride(CHUNK_VERTICES, 16, 1, &[0u8; 12])]],
    );
    assert_eq!(
        parse_chunks_le(&payload),
        Err(Error::StrideMismatch {
            object: 0,
            kind: CHUNK_VERTICES,
            declared: 16,
            expected: VERTEX_STRIDE,
        })
    );
}

#[test]
fn a_repeated_chunk_type_is_refused_rather_than_resolved() {
    let payload = build(
        HEADER_WORD,
        &[vec![
            vertex_chunk(&QUAD_VERTICES),
            vertex_chunk(&QUAD_VERTICES),
        ]],
    );
    assert_eq!(
        parse_chunks_le(&payload),
        Err(Error::DuplicateChunk {
            object: 0,
            kind: CHUNK_VERTICES,
        })
    );
}

/// A hostile object count must not reach a `Vec` reservation.
#[test]
fn an_impossible_object_count_is_refused_before_allocating() {
    let mut payload = build(HEADER_WORD, &[quad()]);
    payload[4..8].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    assert!(matches!(
        parse_chunks_le(&payload),
        Err(Error::OutOfBounds { .. })
    ));
}

#[test]
fn an_impossible_chunk_count_is_refused_before_allocating() {
    let mut payload = build(HEADER_WORD, &[quad()]);
    payload[8..12].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    assert!(matches!(
        parse_chunks_le(&payload),
        Err(Error::OutOfBounds { .. })
    ));
}

/// A chunk whose declared count needs more bytes than the payload holds.
#[test]
fn a_chunk_count_larger_than_the_payload_is_refused() {
    let mut payload = build(HEADER_WORD, &[vec![vertex_chunk(&QUAD_VERTICES)]]);
    // The count sits at +0x06 of the chunk header, after the two u32 file
    // fields and the u32 chunk count.
    let count_at = HEADER_LEN + OBJECT_HEADER_LEN + 6;
    payload[count_at..count_at + 2].copy_from_slice(&0xffffu16.to_le_bytes());
    assert!(matches!(
        parse_chunks_le(&payload),
        Err(Error::OutOfBounds {
            what: "chunk payload",
            ..
        })
    ));
}

#[test]
fn a_payload_shorter_than_its_header_is_refused() {
    assert_eq!(parse_chunks_le(&[]), Err(Error::TooShort { got: 0 }));
    assert_eq!(parse_chunks_le(&[0u8; 7]), Err(Error::TooShort { got: 7 }));
}

#[test]
fn trailing_bytes_are_tolerated_but_visible() {
    let mut payload = build(HEADER_WORD, &[quad()]);
    let exact = payload.len();
    payload.extend([0u8; 32]);
    let geometry = parse_chunks_le(&payload).expect("parse");
    assert_eq!(geometry.encoded_len(), exact);
    assert_ne!(geometry.padded_len(), payload.len());
}

/// The five class IDs, and the fact that all five decode with one parser.
#[test]
fn every_surface_kind_round_trips_through_its_class_id() {
    for kind in SurfaceKind::ALL {
        let id = kind.class_id(V6).expect("version 6 has every collision id");
        assert_eq!(SurfaceKind::from_class_id(id, V6), Some(kind));
    }
    assert_eq!(SurfaceKind::from_class_id(vex::CLASS_MESH, V6), None);
    assert_eq!(SurfaceKind::from_class_id(0, V6), None);

    // The surface-type enum the loader stores, from the class table.
    assert_eq!(SurfaceKind::Wall.surface_type(), Some(0));
    assert_eq!(SurfaceKind::Floor.surface_type(), Some(1));
    assert_eq!(SurfaceKind::Reset.surface_type(), Some(2));
    assert_eq!(SurfaceKind::MagFloor.surface_type(), Some(3));
    // Cage never reaches a collider object, so it has no surface type.
    assert_eq!(SurfaceKind::Cage.surface_type(), None);
}

/// The `-1.0` in the file is a sentinel: averaged against a wall's `0.05` it
/// gives `-0.475` and would add tangential velocity on every contact.
#[test]
fn a_floor_is_frictionless_whatever_it_touches() {
    assert_eq!(SurfaceKind::Floor.friction(), None);
    assert_eq!(SurfaceKind::MagFloor.friction(), None);
    assert_eq!(SurfaceKind::Reset.friction(), None);
    assert_eq!(SurfaceKind::Wall.friction(), Some(WALL_FRICTION));

    let floor = SurfaceKind::Floor.friction();
    let wall = SurfaceKind::Wall.friction();
    assert_eq!(combine_friction(floor, wall), 0.0);
    assert_eq!(combine_friction(wall, floor), 0.0);
    assert_eq!(combine_friction(floor, floor), 0.0);
    // Two walls are the only bouncing contact, and they average.
    assert_eq!(combine_friction(wall, wall), WALL_FRICTION);
    assert_eq!(combine_friction(Some(0.1), Some(0.3)), 0.2);
}

/// Builds a `.vex` file from `(class_id, payload)` nodes, all at the root.
fn build_vex(nodes: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut tree = Vec::new();
    for (class_id, payload) in nodes {
        tree.extend(class_id.to_le_bytes());
        tree.extend(0x20u16.to_le_bytes()); // header_size
        tree.extend(0u16.to_le_bytes()); // padding to +0x08
        tree.extend((payload.len() as u32).to_le_bytes());
        tree.extend(0u32.to_le_bytes()); // no children
        tree.extend([0u8; 0x10]); // the name area, left empty
        tree.extend_from_slice(payload);
    }

    let mut out = Vec::new();
    out.extend(6u32.to_le_bytes());
    out.extend((tree.len() as u32).to_le_bytes());
    out.extend(0u32.to_le_bytes()); // no embedded textures
    out.extend(vex::MAGIC);
    out.extend(tree);
    out
}

/// A node payload as the file stores it: the walk's bytes, zero-padded to 16.
fn padded(payload: &[u8]) -> Vec<u8> {
    let mut out = payload.to_vec();
    out.resize(payload.len().next_multiple_of(PAYLOAD_ALIGN), 0);
    out
}

#[test]
fn collision_nodes_are_found_in_a_vex_tree() {
    let payload = padded(&build(HEADER_WORD, &[quad()]));
    let file = build_vex(&[
        (vex::CLASS_MESH, vec![0u8; 0x30]),
        (vex::CLASS_FLOOR_COLLISION, payload.clone()),
        (vex::CLASS_TRANSFORM, Vec::new()),
        (vex::CLASS_WALL_COLLISION, payload.clone()),
        (vex::CLASS_CAGE_COLLISION, payload),
    ]);

    let found = from_vex(&file).expect("collision nodes");
    assert_eq!(
        found.iter().map(|n| n.kind).collect::<Vec<_>>(),
        [SurfaceKind::Floor, SurfaceKind::Wall, SurfaceKind::Cage],
        "a Cage node is reported, not skipped"
    );
    // Node indices are into the whole tree, for world-transform lookup.
    assert_eq!(
        found.iter().map(|n| n.node_index).collect::<Vec<_>>(),
        [1, 3, 4]
    );
    for node in &found {
        assert_eq!(node.geometry.vertex_count(), 4);
        assert_eq!(node.geometry.triangle_count(), 2);
    }
}

/// A dropped object looks like a wrong structure size and the payload then
/// fails to close; `from_vex` enforces it and the ground-truth test runs it over
/// real nodes.
#[test]
fn a_node_whose_walk_does_not_close_is_refused() {
    // Declare two objects and supply one.
    let mut payload = build(HEADER_WORD, &[quad(), quad()]);
    payload.truncate(HEADER_LEN + build(HEADER_WORD, &[quad()]).len() - HEADER_LEN);
    payload[4..8].copy_from_slice(&2u32.to_le_bytes());
    // Padded to a boundary, so only the object walk can detect it.
    let file = build_vex(&[(vex::CLASS_FLOOR_COLLISION, padded(&payload))]);
    assert!(from_vex(&file).is_err());

    // And a payload that parses but stops well short of its node's length.
    let short = build(HEADER_WORD, &[quad()]);
    let mut over = short.clone();
    over.resize(short.len() + PAYLOAD_ALIGN * 2, 0);
    let file = build_vex(&[(vex::CLASS_FLOOR_COLLISION, over)]);
    assert!(matches!(
        from_vex(&file),
        Err(Error::PayloadNotAccountedFor { node: 0, .. })
    ));
}

#[test]
fn a_vex_file_without_collision_nodes_yields_nothing() {
    let file = build_vex(&[(vex::CLASS_MESH, vec![0u8; 0x30])]);
    assert!(from_vex(&file).expect("walk").is_empty());
}

#[test]
fn a_vex_walk_failure_is_propagated_rather_than_swallowed() {
    assert!(matches!(
        from_vex(&[0u8; 4]),
        Err(Error::Vex(vex::Error::TooShort { .. }))
    ));
}

/// A node whose payload is not a collision payload must fail loudly: the class
/// IDs are the one part a wrong reading would not otherwise show.
#[test]
fn a_node_payload_that_is_not_collision_data_is_refused() {
    let file = build_vex(&[(vex::CLASS_FLOOR_COLLISION, vec![0xff; 0x20])]);
    assert!(from_vex(&file).is_err());
}

/// The same object, written both ways round, decodes to the same geometry.
///
/// The payload cannot say which way round it is (`0xffffffff` is a palindrome,
/// chunk kinds 1, 2, 3 are not), so the order comes from the containing `.vex`
/// magic in `from_vex`. One hand-built object is all this claim needs.
#[test]
fn a_big_endian_object_decodes_to_the_same_geometry_as_its_little_endian_twin() {
    let mut be: Vec<u8> = Vec::new();
    be.extend(HEADER_WORD.to_be_bytes());
    be.extend(1u32.to_be_bytes()); // one object
    be.extend(3u32.to_be_bytes()); // three chunks, in the shipped order 1, 3, 2

    let mut chunk = |kind: u32, count: usize, body: &[u8]| {
        be.extend(kind.to_be_bytes());
        be.extend((chunk_stride(kind).unwrap() as u16).to_be_bytes());
        be.extend((count as u16).to_be_bytes());
        be.extend_from_slice(body);
    };

    let mut vertices = Vec::new();
    for v in &QUAD_VERTICES {
        for c in v {
            vertices.extend(c.to_be_bytes());
        }
    }
    chunk(CHUNK_VERTICES, QUAD_VERTICES.len(), &vertices);

    let mut scalars = Vec::new();
    for s in &QUAD_SCALARS {
        scalars.extend(s.to_be_bytes());
    }
    chunk(CHUNK_VERTEX_SCALARS, QUAD_SCALARS.len(), &scalars);

    let mut triangles = Vec::new();
    for t in &QUAD_TRIANGLES {
        for i in t {
            triangles.extend(i.to_be_bytes());
        }
    }
    chunk(CHUNK_TRIANGLES, QUAD_TRIANGLES.len(), &triangles);

    let le = build(HEADER_WORD, &[quad()]);
    assert_eq!(be.len(), le.len(), "the same layout, only byte-swapped");

    let from_be = parse_chunks(&be, ByteOrder::Big).expect("the big-endian payload");
    let from_le = parse_chunks_le(&le).expect("the little-endian payload");
    assert_eq!(from_be, from_le);
    assert_eq!(from_be.meshes[0].vertices, QUAD_VERTICES.to_vec());

    // The wrong order does not quietly decode: the stride word catches it, which
    // is why the parser checks a field the game's loader ignores.
    assert!(
        parse_chunks(&be, ByteOrder::Little).is_err(),
        "a big-endian payload read little-endian must not produce geometry"
    );
}
