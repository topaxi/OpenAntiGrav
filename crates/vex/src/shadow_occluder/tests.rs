//! What [`super::Occluder`] is asserted to do, on a payload built here.
//!
//! The disc-wide numbers this module's docs quote are pinned by
//! `crates/vex/tests/shadow_occluder_ground_truth.rs`, which needs a disc
//! image. What is here is the arithmetic that has to hold whatever the disc
//! says: the strides, the sentinel, and the silhouette walk.

use super::*;

/// A tetrahedron in the shipped layout: four triangles over four vertices,
/// wound and cross-linked the way `shadow_mineShape` is.
///
/// Built rather than copied out of a file - no game content lands in this
/// repository, per ADR-0006 - but the *shape* is that node's: one face
/// pointing down, three up-and-outward, each edge naming the face across it.
fn tetrahedron() -> Vec<u8> {
    // (0,0,0), (1,0,0), (0,0,1) on the floor, (0,1,0) above them.
    let vertices: [[f32; 3]; 4] = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
    ];
    // Each face: its normal, its three vertices wound counter-clockwise about
    // that normal, and the face across each of its three edges.
    let faces: [([f32; 3], [u16; 3], [u16; 3]); 4] = [
        ([0.0, -1.0, 0.0], [0, 1, 2], [2, 3, 1]),
        ([-1.0, 0.0, 0.0], [0, 2, 3], [0, 3, 2]),
        ([0.0, 0.0, -1.0], [0, 3, 1], [1, 3, 0]),
        ([0.577_35, 0.577_35, 0.577_35], [1, 3, 2], [2, 1, 0]),
    ];

    let mut out = vec![0u8; HEADER_LEN];
    out[0..2].copy_from_slice(&(faces.len() as u16).to_le_bytes());
    out[2..4].copy_from_slice(&(vertices.len() as u16).to_le_bytes());
    for (axis, value) in [0.0f32, 0.0, 0.0].iter().enumerate() {
        out[0x0c + axis * 4..0x10 + axis * 4].copy_from_slice(&value.to_le_bytes());
    }
    for (axis, value) in [1.0f32, 1.0, 1.0].iter().enumerate() {
        out[0x18 + axis * 4..0x1c + axis * 4].copy_from_slice(&value.to_le_bytes());
    }

    for (normal, index, neighbour) in faces {
        let mut record = vec![0u8; FACE_LEN];
        for axis in 0..3 {
            record[axis * 4..axis * 4 + 4].copy_from_slice(&normal[axis].to_le_bytes());
        }
        record[0x0c..0x10].copy_from_slice(&3u32.to_le_bytes());
        for slot in 0..3 {
            record[0x10 + slot * 2..0x12 + slot * 2]
                .copy_from_slice(&neighbour[slot].to_le_bytes());
            record[0x18 + slot * 2..0x1a + slot * 2].copy_from_slice(&index[slot].to_le_bytes());
        }
        // The two the shipped records carry on a triangle: no face across the
        // fourth edge, and the fourth index repeating the first.
        record[0x16..0x18].copy_from_slice(&NO_NEIGHBOUR.to_le_bytes());
        record[0x1e..0x20].copy_from_slice(&index[0].to_le_bytes());
        out.extend_from_slice(&record);
    }

    for vertex in vertices {
        let mut record = vec![0u8; VERTEX_LEN];
        record[0..4].copy_from_slice(&1.0f32.to_le_bytes());
        for axis in 0..3 {
            record[4 + axis * 4..8 + axis * 4].copy_from_slice(&vertex[axis].to_le_bytes());
        }
        out.extend_from_slice(&record);
    }
    out
}

#[test]
fn a_payload_that_does_not_close_is_refused() {
    let payload = tetrahedron();
    assert!(Occluder::parse(&payload, ByteOrder::Little).is_some());
    // One byte short, one byte long, and a truncated header: the length
    // closure is the whole validation, so it has to actually bite.
    assert!(Occluder::parse(&payload[..payload.len() - 1], ByteOrder::Little).is_none());
    let mut longer = payload.clone();
    longer.push(0);
    assert!(Occluder::parse(&longer, ByteOrder::Little).is_none());
    assert!(Occluder::parse(&payload[..8], ByteOrder::Little).is_none());
}

#[test]
fn the_records_decode_at_their_own_strides() {
    let occluder = Occluder::parse(&tetrahedron(), ByteOrder::Little).expect("closes");
    assert_eq!(occluder.faces.len(), 4);
    assert_eq!(occluder.vertices.len(), 4);
    assert_eq!(occluder.bounds, ([0.0; 3], [1.0; 3]));
    assert_eq!(occluder.vertices[3], [0.0, 1.0, 0.0]);
    let floor = &occluder.faces[0];
    assert_eq!(floor.normal, [0.0, -1.0, 0.0]);
    assert_eq!(floor.count, 3);
    assert_eq!(floor.indices(), [0, 1, 2]);
    // The `w` column is dropped, so an index means the same thing here as in
    // the file: vertex 2 is the third record, not the third float.
    assert_eq!(
        occluder.vertices[usize::from(floor.vertices[2])],
        [0.0, 0.0, 1.0]
    );
}

#[test]
fn a_triangles_fourth_edge_has_no_face_across_it() {
    let occluder = Occluder::parse(&tetrahedron(), ByteOrder::Little).expect("closes");
    for face in &occluder.faces {
        assert_eq!(face.neighbours[3], NO_NEIGHBOUR);
        assert_eq!(face.vertices[3], face.vertices[0], "the loop closes");
        // `neighbour` wraps on the face's own count, so edge 3 of a triangle
        // is edge 0 rather than the sentinel slot.
        assert_eq!(face.neighbour(3), face.neighbour(0));
    }
}

#[test]
fn adjacency_is_reciprocal() {
    // The property the disc closes on over 14,328 edges, asserted here on the
    // four the fixture has - so a stride that moved would fail without a disc.
    let occluder = Occluder::parse(&tetrahedron(), ByteOrder::Little).expect("closes");
    for (index, face) in occluder.faces.iter().enumerate() {
        for edge in 0..usize::from(face.count) {
            let across = face.neighbour(edge).expect("a closed hull");
            let other = &occluder.faces[usize::from(across)];
            let want = (
                face.vertices[edge],
                face.vertices[(edge + 1) % usize::from(face.count)],
            );
            let owns = (0..usize::from(other.count)).any(|k| {
                let have = (
                    other.vertices[k],
                    other.vertices[(k + 1) % usize::from(other.count)],
                );
                have == (want.1, want.0) || have == want
            });
            assert!(
                owns,
                "face {index} edge {edge} names {across}, which does not own it"
            );
        }
    }
}

#[test]
fn every_face_winds_with_its_own_normal() {
    let occluder = Occluder::parse(&tetrahedron(), ByteOrder::Little).expect("closes");
    for (index, face) in occluder.faces.iter().enumerate() {
        assert!(
            face.winding(&occluder.vertices) > 0.0,
            "face {index} winds against its declared normal"
        );
    }
}

#[test]
fn the_silhouette_is_the_ring_between_what_faces_the_light_and_what_does_not() {
    let occluder = Occluder::parse(&tetrahedron(), ByteOrder::Little).expect("closes");
    // Straight down: only the floor face faces it, so its own three edges are
    // the silhouette - the ring around the one front-facing face.
    let edges = occluder.silhouette([0.0, -1.0, 0.0]);
    assert_eq!(edges.len(), 3, "{edges:?}");
    let floor = &occluder.faces[0];
    for edge in 0..3 {
        let want = (floor.vertices[edge], floor.vertices[(edge + 1) % 3]);
        assert!(edges.contains(&want), "{want:?} missing from {edges:?}");
    }
}

#[test]
fn a_direction_nothing_faces_has_no_silhouette() {
    let occluder = Occluder::parse(&tetrahedron(), ByteOrder::Little).expect("closes");
    // Every edge of a closed hull is shared by two faces, so a direction where
    // *every* face is front-facing cancels every edge. The fixture has no such
    // direction, but the empty case has to be reachable rather than a panic:
    // the degenerate zero direction faces nothing at all.
    assert!(occluder.silhouette([0.0, 0.0, 0.0]).is_empty());
}

#[test]
fn an_open_hull_keeps_its_boundary_edges() {
    // Two faces sharing one edge, six boundary edges between them - the shape
    // `Data.wad#242` and `#244` actually carry. A boundary edge has nothing on
    // the other side to cancel it, so it is on the silhouette whenever its own
    // face is.
    let mut payload = tetrahedron();
    // Drop to one face by rewriting the count and truncating - the closure
    // check is what keeps this honest.
    payload[0..2].copy_from_slice(&1u16.to_le_bytes());
    payload.drain(HEADER_LEN + FACE_LEN..HEADER_LEN + FACE_LEN * 4);
    let occluder = Occluder::parse(&payload, ByteOrder::Little).expect("closes");
    assert_eq!(occluder.faces.len(), 1);
    // Its three neighbours name faces that no longer exist; `silhouette` looks
    // them up rather than trusting the index, so nothing panics and every edge
    // survives.
    let edges = occluder.silhouette([0.0, -1.0, 0.0]);
    assert_eq!(edges.len(), 3, "{edges:?}");
}

#[test]
fn the_silhouette_chains_into_one_closed_ring() {
    let occluder = Occluder::parse(&tetrahedron(), ByteOrder::Little).expect("closes");
    let rings = occluder.silhouette_loops([0.0, -1.0, 0.0]);
    assert_eq!(rings.len(), 1, "{rings:?}");
    assert!(rings[0].closed);
    // The floor face's own winding, which is what makes a ring fannable
    // without reordering it first.
    assert_eq!(rings[0].vertices, vec![0, 1, 2]);
    assert_eq!(occluder.outline([0.0, -1.0, 0.0]), vec![vec![0, 1, 2]]);
}

#[test]
fn a_tetrahedron_outlines_three_edges_from_every_direction() {
    // A convex hull's silhouette is one ring from any direction that is not
    // exactly edge-on, and a tetrahedron's is always three edges. Six
    // directions rather than one: a walk that only works for the direction it
    // was written against is not a silhouette walk.
    let occluder = Occluder::parse(&tetrahedron(), ByteOrder::Little).expect("closes");
    for direction in [
        [0.097_589_54, -0.975_895_4, 0.195_179_08],
        [0.0, -1.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.577_35, 0.577_35, 0.577_35],
    ] {
        let rings = occluder.outline(direction);
        assert_eq!(rings.len(), 1, "{direction:?}: {rings:?}");
        assert_eq!(rings[0].len(), 3, "{direction:?}: {rings:?}");
    }
}
