//! What the `track_col.col` reader in [`super`] is asserted to do.
//!
//! Its own file rather than an inline block: the rule in `CLAUDE.md` caps an
//! inline `#[cfg(test)] mod` at 200 lines and these are past it.

use super::*;

/// [`build_with`] in 2048's own 24-byte node layout.
fn build(surfaces: &[u8]) -> Vec<u8> {
    build_with(surfaces, NodeLayout::Wide)
}

/// Builds a file with one internal node, two leaves and `surfaces` triangles.
///
/// The geometry is a fan around the origin, which is enough for the reader:
/// nothing here checks that a triangle is well formed, only that every index
/// names something and every section closes.
fn build_with(surfaces: &[u8], layout: NodeLayout) -> Vec<u8> {
    let triangles = surfaces.len();
    let vertices = triangles + 2;
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(b"0001");

    // Two leaves under one root, splitting the leaf index array in half.
    let split = triangles / 2;
    out.extend_from_slice(SECTION_TAG);
    out.extend_from_slice(&(layout.len() as u32).to_le_bytes());
    out.extend_from_slice(&3u32.to_le_bytes());
    let node = |low: i32, high: i32, axis: u32, at: f32, count: usize, first: usize| {
        let mut n = Vec::new();
        n.extend_from_slice(&low.to_le_bytes());
        n.extend_from_slice(&high.to_le_bytes());
        match layout {
            NodeLayout::Wide => {
                n.extend_from_slice(&axis.to_le_bytes());
                n.extend_from_slice(&at.to_bits().to_le_bytes());
                n.extend_from_slice(&(0x000b_0000u32 | count as u32).to_le_bytes());
            }
            NodeLayout::Packed => {
                n.push(axis as u8);
                n.extend_from_slice(&(count as u16).to_le_bytes());
                n.extend_from_slice(&at.to_bits().to_le_bytes());
            }
        }
        n.extend_from_slice(&(first as u32).to_le_bytes());
        n
    };
    out.extend(node(1, 2, 1, 0.5, 0, 0));
    out.extend(node(-1, -1, u32::MAX, 0.0, split, 0));
    out.extend(node(-1, -1, u32::MAX, 0.0, triangles - split, split));

    out.extend_from_slice(SECTION_TAG);
    out.extend_from_slice(&(triangles as u32).to_le_bytes());
    for t in 0..triangles {
        out.extend_from_slice(&(t as u16).to_le_bytes());
    }

    out.extend_from_slice(SECTION_TAG);
    for v in [0.0f32, 1.0, 2.0, 10.0, 11.0, 12.0] {
        out.extend_from_slice(&v.to_bits().to_le_bytes());
    }

    out.extend_from_slice(SECTION_TAG);
    out.extend_from_slice(&(vertices as u16).to_le_bytes());
    for i in 0..vertices {
        for a in 0..3 {
            out.extend_from_slice(&((i * 3 + a) as f32).to_bits().to_le_bytes());
        }
    }
    out.extend_from_slice(&(TRIANGLE_LEN as u32).to_le_bytes());
    out.extend_from_slice(&(triangles as u16).to_le_bytes());
    for t in 0..triangles {
        for k in 0..3 {
            out.extend_from_slice(&((t + k) as u16).to_le_bytes());
        }
    }
    out.extend_from_slice(&(triangles as u16).to_le_bytes());
    out.extend_from_slice(surfaces);
    for v in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
        out.extend_from_slice(&v.to_bits().to_le_bytes());
    }

    out.extend_from_slice(SECTION_TAG);
    out
}

#[test]
fn it_reads_a_tree_a_leaf_array_and_a_soup() {
    let decoded = parse(&build(&[2, 4, 7, 2])).expect("the fixture parses");
    assert_eq!(decoded.nodes.len(), 3);
    assert_eq!(decoded.leaves.len(), 4);
    assert_eq!(decoded.mesh.triangles.len(), 4);
    assert_eq!(decoded.mesh.vertices.len(), 6);
    assert_eq!(decoded.mesh.surfaces, vec![2, 4, 7, 2]);
}

/// The Omega Collection's 19-byte node decodes to the same tree as 2048's
/// 24-byte one: the same children, axes, split positions, runs and leaf
/// starts, and no `unknown` half-word because the packed record has none.
#[test]
fn a_packed_node_decodes_to_the_same_tree_as_a_wide_one() {
    let wide = parse(&build(&[2, 4, 7, 2])).expect("the wide fixture parses");
    let packed =
        parse(&build_with(&[2, 4, 7, 2], NodeLayout::Packed)).expect("the packed fixture parses");
    assert_eq!(wide.layout, NodeLayout::Wide);
    assert_eq!(packed.layout, NodeLayout::Packed);
    assert_eq!(wide.nodes.len(), packed.nodes.len());
    for (w, p) in wide.nodes.iter().zip(&packed.nodes) {
        assert_eq!(
            (
                w.low,
                w.high,
                w.axis,
                w.split,
                w.triangle_count,
                w.first_leaf
            ),
            (
                p.low,
                p.high,
                p.axis,
                p.split,
                p.triangle_count,
                p.first_leaf
            )
        );
        assert_eq!(w.unknown, Some(0x000b));
        assert_eq!(p.unknown, None);
    }
    assert_eq!(wide.leaves, packed.leaves);
    assert_eq!(wide.mesh, packed.mesh);
}

/// The two strides are the only ones read. 20 is neither, and is refused by
/// name rather than walked as if it were 19 or 24.
#[test]
fn it_refuses_a_node_stride_that_is_neither_layout() {
    let mut broken = build(&[2, 4]);
    let at = broken
        .windows(4)
        .position(|w| w == (NODE_LEN as u32).to_le_bytes())
        .expect("the node stride field is in there");
    broken[at..at + 4].copy_from_slice(&20u32.to_le_bytes());
    assert_eq!(parse(&broken), Err(Error::BadNodeStride { stride: 20 }));
}

/// A packed file that lies about its node count runs off the array rather
/// than resynchronising on the next tag: the file is refused, not guessed at.
#[test]
fn a_packed_file_with_a_short_node_array_is_refused() {
    let file = build_with(&[2, 4], NodeLayout::Packed);
    // magic, version, tag and stride are 16 bytes; the node count follows.
    let at = 16;
    let mut broken = file.clone();
    broken[at..at + 4].copy_from_slice(&4u32.to_le_bytes());
    assert!(parse(&broken).is_err());
}

/// The root splits and the two below it hold triangles - the distinction the
/// child fix-up loop in `KdTree_Load` draws, and the only thing that separates
/// a node's `split` from its leaf run.
#[test]
fn a_node_is_a_split_or_a_run_of_triangles_and_never_both() {
    let decoded = parse(&build(&[2, 4, 7, 2])).expect("the fixture parses");
    let root = decoded.nodes[0];
    assert!(!root.is_leaf());
    assert_eq!(root.axis, Some(1));
    assert_eq!(root.triangle_count, 0);
    for leaf in &decoded.nodes[1..] {
        assert!(leaf.is_leaf());
        assert_eq!(leaf.axis, None, "a leaf splits on no axis");
        assert_eq!(leaf.triangle_count, 2);
    }
    // The runs tile the leaf array exactly, which is true of all 26 shipped
    // files as well - see `crates/vex/tests/kdcol_ground_truth.rs`.
    assert_eq!(decoded.nodes[1].first_leaf, 0);
    assert_eq!(decoded.nodes[2].first_leaf, 2);
}

/// The pair at the end of each section is a centre and a half-extent, not a
/// min and a max. Reading it the other way is plausible and wrong - see the
/// module docs for the six numbers that settle it on the real file.
#[test]
fn the_bounds_are_a_centre_and_a_half_extent() {
    let decoded = parse(&build(&[2])).expect("the fixture parses");
    assert_eq!(decoded.bounds.centre, [0.0, 1.0, 2.0]);
    assert_eq!(decoded.bounds.extent, [10.0, 11.0, 12.0]);
    assert_eq!(decoded.bounds.min(), [-10.0, -10.0, -10.0]);
    assert_eq!(decoded.bounds.max(), [10.0, 12.0, 14.0]);
}

/// The six bytes `TrackCollision_MeshFromNode` (`0x8126f800`) itself emits,
/// each against the `.vex` class that function reads them off.
#[test]
fn the_six_surface_bytes_the_executable_names_map_to_their_classes() {
    assert_eq!(class_of(2), Some(SurfaceKind::Floor));
    assert_eq!(class_of(3), Some(SurfaceKind::MagFloor));
    assert_eq!(class_of(4), Some(SurfaceKind::Wall));
    assert_eq!(class_of(5), Some(SurfaceKind::TrackWall));
    assert_eq!(class_of(6), Some(SurfaceKind::TrackWall));
    assert_eq!(class_of(7), Some(SurfaceKind::Reset));
}

/// `10`, `11` and `12` are placed by measurement rather than by the executable,
/// and the two routes are deliberately separate functions so a caller can tell
/// them apart. See [`super::is_measured_floor`] for the evidence and its
/// confidence.
#[test]
fn the_measured_floors_are_not_claimed_to_come_from_a_class() {
    for byte in [10u8, 11, 12] {
        assert_eq!(class_of(byte), None, "byte {byte} names no .vex class");
        assert!(is_measured_floor(byte));
        assert_eq!(surface_kind(byte), Some(SurfaceKind::Floor));
    }
    // `15` is what the executable's own default arm emits, and no shipped file
    // carries one: unplaced, so a caller reports it rather than colliding.
    assert_eq!(surface_kind(15), None);
    assert_eq!(surface_kind(0), None);
}

/// Triangles come out grouped by kind in [`SurfaceKind::ALL`] order, because a
/// collider's index is its identity in the physics world.
#[test]
fn nodes_come_out_in_a_fixed_order_with_their_own_vertices() {
    let decoded = parse(&build(&[7, 2, 4, 2])).expect("the fixture parses");
    let (nodes, unplaced) = collision_nodes(&decoded);
    assert!(unplaced.is_empty());
    let kinds: Vec<SurfaceKind> = nodes.iter().map(|n| n.kind).collect();
    assert_eq!(
        kinds,
        vec![SurfaceKind::Floor, SurfaceKind::Wall, SurfaceKind::Reset],
        "SurfaceKind::ALL's order, not the order the triangles appear in"
    );
    let floor = &nodes[0].geometry.meshes[0];
    assert_eq!(floor.triangles.len(), 2);
    // Only the vertices its own triangles name, remapped, and a neutral scalar
    // for each - the container carries no scalar chunk.
    assert!(floor.vertices.len() <= 6);
    assert_eq!(floor.vertex_scalars.len(), floor.vertices.len());
    assert!(floor.vertex_scalars.iter().all(|s| *s == 1.0));
    for triangle in &floor.triangles {
        for &index in triangle {
            assert!(usize::from(index) < floor.vertices.len());
        }
    }
}

/// A byte nothing places is counted and reported rather than dropped in
/// silence: a circuit missing collision should say which surface it lost.
#[test]
fn an_unplaced_surface_byte_is_reported_with_its_count() {
    let decoded = parse(&build(&[2, 99, 99, 15])).expect("the fixture parses");
    let (nodes, unplaced) = collision_nodes(&decoded);
    assert_eq!(unplaced, vec![(15, 1), (99, 2)]);
    assert_eq!(nodes.len(), 1);
}

#[test]
fn it_refuses_a_wrong_magic() {
    let mut file = build(&[2]);
    file[0] = b'x';
    assert!(matches!(parse(&file), Err(Error::BadMagic { .. })));
}

/// The version is four ASCII hex digits and the loader requires it to read as
/// one - `sscanf`-ed against `"kdtr%04x"`, not compared as bytes.
#[test]
fn it_refuses_a_version_other_than_one() {
    let mut file = build(&[2]);
    file[4..8].copy_from_slice(b"0002");
    assert!(matches!(parse(&file), Err(Error::BadVersion { .. })));
    let mut file = build(&[2]);
    file[4..8].copy_from_slice(b"zzzz");
    assert!(matches!(parse(&file), Err(Error::BadVersion { .. })));
}

/// A stride the original could not load is refused rather than walked: it reads
/// `stride * count` bytes into a buffer it sized at `6 * count`.
#[test]
fn it_refuses_a_triangle_stride_the_original_could_not_load() {
    let file = build(&[2, 4]);
    let at = file
        .windows(4)
        .position(|w| w == (TRIANGLE_LEN as u32).to_le_bytes())
        .expect("the stride field is in there");
    let mut broken = file.clone();
    broken[at..at + 4].copy_from_slice(&8u32.to_le_bytes());
    assert!(matches!(parse(&broken), Err(Error::BadStride { .. })));
}

#[test]
fn it_refuses_a_truncated_file() {
    let file = build(&[2, 4, 7]);
    for cut in [4, 12, 40, file.len() - 1] {
        assert!(parse(&file[..cut]).is_err(), "{cut} bytes should not parse");
    }
}

#[test]
fn it_refuses_bytes_after_the_last_section_tag() {
    let mut file = build(&[2]);
    file.push(0);
    assert!(matches!(
        parse(&file),
        Err(Error::TrailingBytes { extra: 1 })
    ));
}

/// A `.col` sits beside its `track.vex` and is found by name, the same idiom
/// the `.rcsmodel` pairing uses.
#[test]
fn the_sibling_name_is_the_track_beside_it() {
    assert_eq!(
        sibling_name(r"Data\art\published\environments\altima\track.vex").as_deref(),
        Some(r"Data\art\published\environments\altima\track_col.col")
    );
    assert_eq!(
        sibling_name("/data/environments/zone_1/Track.vex").as_deref(),
        Some("/data/environments/zone_1/Track_col.col")
    );
    assert_eq!(sibling_name("Data/Ships/Feisar/Ship.vex"), None);
    assert_eq!(sibling_name("track.vex"), None);
}

/// Omega's reversed circuits: `track_reversed.vex` beside
/// `track_col_reversed.col`, the `_col` before the `_reversed`. Measured on
/// all twelve of `data01`/`data02`'s reversed circuits.
#[test]
fn a_reversed_track_pairs_with_the_reversed_collision() {
    assert_eq!(
        sibling_name("Data/environments/tech_de_ra/track_reversed.vex").as_deref(),
        Some("Data/environments/tech_de_ra/track_col_reversed.col")
    );
    assert_eq!(
        sibling_name(r"Data\environments\tech_de_ra\Track_Reversed.vex").as_deref(),
        Some(r"Data\environments\tech_de_ra\Track_col_Reversed.col")
    );
}

/// Only the track itself pairs: the other files a circuit directory holds
/// that end in `_reversed.vex` are not collision-bearing.
#[test]
fn a_reversed_file_that_is_not_the_track_has_no_sibling() {
    for name in [
        "Data/environments/tech_de_ra/start_grid_reversed.vex",
        "Data/environments/tech_de_ra/padReplacement_reversed.vex",
        "Data/environments/tech_de_ra/track_overlay_reversed.vex",
        "Data/environments/tech_de_ra/_reversed.vex",
        "Data/environments/tech_de_ra/track_reversedx.vex",
    ] {
        assert_eq!(sibling_name(name), None, "{name}");
    }
}
