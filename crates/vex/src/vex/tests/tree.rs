//! The node tree: the class table a version word selects, the walk over the
//! nodes, and the world transforms composed down the chain.

use crate::vex::*;
use oag_formats::ByteOrder;

#[test]
fn a_version_word_selects_a_table_and_an_unknown_one_selects_none() {
    assert_eq!(classes::for_version(6), Some(classes::V6));
    assert_eq!(classes::for_version(4), Some(classes::V4));
    assert_eq!(classes::for_version(3), Some(classes::V3));
    // Not a decode failure to swallow: a version nobody has looked at
    // should say so rather than be read as version 6.
    assert_eq!(classes::for_version(5), None);
    assert_eq!(classes::for_version(7), None);
}

/// The two numberings share no value, which is why falling back from one to
/// the other would match nothing rather than mismatch loudly.
#[test]
fn the_two_recovered_tables_have_no_id_in_common() {
    let v6 = [
        classes::V6.mesh,
        classes::V6.texture,
        classes::V6.transform,
        classes::V6.wo_track,
    ];
    let v4 = [
        classes::V4.mesh,
        classes::V4.texture,
        classes::V4.transform,
        classes::V4.wo_track,
    ];
    for id in v6.into_iter().flatten() {
        assert!(
            !v4.contains(&Some(id)),
            "{id:#x} appears in both numberings"
        );
    }
}

/// The version-6 table is the `CLASS_*` constants, not a second copy of
/// them that can drift.
#[test]
fn the_version_6_table_is_the_documented_constants() {
    assert_eq!(classes::V6.mesh, Some(CLASS_MESH));
    assert_eq!(classes::V6.texture, Some(CLASS_TEXTURE));
    assert_eq!(classes::V6.transform, Some(CLASS_TRANSFORM));
    assert_eq!(classes::V6.wo_track, Some(CLASS_WO_TRACK));
}

/// Builds a `.vex` file from `(class_id, child_count, payload_len)` nodes,
/// in the depth-first pre-order the format stores them in.
fn build_tree(nodes: &[(u32, usize, usize)]) -> Vec<u8> {
    build_tree_in(nodes, ByteOrder::Little)
}

/// The same file, written the way the console `order` names would write it.
///
/// The same file, written the way the console `order` names would write it.
///
/// The PS3 export is the same layout with a different byte order and a reversed
/// magic, which `docs/formats/hd-status.md` measures over 40 shipped files.
fn build_tree_in(nodes: &[(u32, usize, usize)], order: ByteOrder) -> Vec<u8> {
    let u16 = |v: u16| -> Vec<u8> {
        match order {
            ByteOrder::Little => v.to_le_bytes().to_vec(),
            ByteOrder::Big => v.to_be_bytes().to_vec(),
        }
    };
    let u32 = |v: u32| -> Vec<u8> {
        match order {
            ByteOrder::Little => v.to_le_bytes().to_vec(),
            ByteOrder::Big => v.to_be_bytes().to_vec(),
        }
    };

    let mut tree = Vec::new();
    for &(class_id, children, payload_len) in nodes {
        tree.extend(u32(class_id));
        tree.extend(u16(0x20)); // header_size
        tree.extend(u16(0)); // padding to +0x08
        tree.extend(u32(payload_len as u32));
        // A `u16` count and the `u16` at `+0x0e`, not one `u32`: writing it
        // as a word looks identical little-endian and is zero big-endian.
        tree.extend(u16(children as u16));
        tree.extend(u16(0));
        tree.extend([0u8; 0x10]); // the name area, left empty
        tree.extend(std::iter::repeat_n(0u8, payload_len));
    }

    let mut out = Vec::new();
    out.extend(u32(6));
    out.extend(u32(tree.len() as u32));
    out.extend(u32(0));
    out.extend(match order {
        ByteOrder::Little => MAGIC,
        ByteOrder::Big => MAGIC_BE,
    });
    out.extend(tree);
    out
}

/// The same tree, written both ways round, walks to the same nodes.
///
/// The same tree, written both ways round, walks to the same nodes.
///
/// A file declares its order in its own magic (`oag_formats::byte_order`). The
/// negative half matters as much: read the big-endian file little-endian and the
/// header alone is nonsense.
#[test]
fn a_big_endian_file_walks_to_the_same_tree_as_its_little_endian_twin() {
    let shape = [(1, 1, 0), (2, 0, 16), (3, 0, 0)];
    let le = build_tree(&shape);
    let be = build_tree_in(&shape, ByteOrder::Big);

    assert_eq!(byte_order(&le), ByteOrder::Little);
    assert_eq!(byte_order(&be), ByteOrder::Big);
    assert!(has_magic(&le) && has_magic(&be), "both spellings are .vex");
    assert_eq!(version(&le).unwrap(), 6);
    assert_eq!(
        version(&be).unwrap(),
        6,
        "the version word follows the magic"
    );

    let walked = |data: &[u8]| -> Vec<(u32, usize, usize, usize)> {
        nodes(data)
            .expect("walk")
            .iter()
            .map(|n| (n.class_id, n.child_count, n.data_size, n.depth))
            .collect()
    };
    assert_eq!(walked(&le), walked(&be));
    assert_eq!(walked(&be), [(1, 1, 0, 0), (2, 0, 16, 1), (3, 0, 0, 0)]);

    // And what the swap is worth: the same bytes with the magic left alone
    // read as a version nobody has a table for, rather than as version 6.
    let mut mislabelled = be.clone();
    mislabelled[12..16].copy_from_slice(MAGIC);
    assert_eq!(version(&mislabelled).unwrap(), 0x0600_0000);
}

#[test]
fn walks_a_flat_tree() {
    let data = build_tree(&[(1, 0, 0), (2, 0, 16), (3, 0, 0)]);
    let nodes = nodes(&data).expect("walk");
    assert_eq!(nodes.len(), 3);
    assert_eq!(
        nodes.iter().map(|n| n.class_id).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(nodes.iter().all(|n| n.depth == 0));
    assert_eq!(nodes[1].data_size, 16);
    assert_eq!(nodes[1].payload().len(), 16);
}

/// A parent whose subtree is finished must stop counting as an ancestor.
///
/// A parent whose subtree is finished must stop counting as an ancestor.
///
/// `root -> a -> b`, then a sibling of `root` at depth 0 (not 2, as results if
/// finished subtrees retire after reading the depth instead of before). Nothing
/// reads `depth` yet, the only reason the bug was survivable.
#[test]
fn depth_returns_to_zero_after_a_completed_subtree() {
    let data = build_tree(&[(1, 1, 0), (2, 1, 0), (3, 0, 0), (4, 0, 0)]);
    let depths: Vec<usize> = nodes(&data)
        .expect("walk")
        .iter()
        .map(|n| n.depth)
        .collect();
    assert_eq!(depths, [0, 1, 2, 0]);
}

#[test]
fn depth_tracks_siblings_at_every_level() {
    // root
    //   a
    //     a1
    //     a2
    //   b
    let data = build_tree(&[(1, 2, 0), (2, 2, 0), (3, 0, 0), (4, 0, 0), (5, 0, 0)]);
    let depths: Vec<usize> = nodes(&data)
        .expect("walk")
        .iter()
        .map(|n| n.depth)
        .collect();
    assert_eq!(depths, [0, 1, 2, 2, 1]);
}

#[test]
fn stops_at_a_node_that_runs_past_the_tree() {
    let mut data = build_tree(&[(1, 0, 0), (2, 0, 0)]);
    // Claim a payload far larger than the file for the second node.
    let second = FILE_HEADER_LEN + 0x20;
    data[second + 8..second + 12].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    let nodes = nodes(&data).expect("walk");
    assert_eq!(nodes.len(), 1, "the impossible node must not be reported");
}

#[test]
fn stops_at_a_header_too_small_to_be_one() {
    let mut data = build_tree(&[(1, 0, 0), (2, 0, 0)]);
    let second = FILE_HEADER_LEN + 0x20;
    data[second + 4..second + 6].copy_from_slice(&8u16.to_le_bytes());
    assert_eq!(nodes(&data).expect("walk").len(), 1);
}

#[test]
fn a_truncated_file_is_refused_rather_than_walked() {
    assert!(matches!(nodes(&[]), Err(Error::TooShort { .. })));
    assert!(matches!(nodes(&[0u8; 8]), Err(Error::TooShort { .. })));
}

/// A tree length larger than the file must not read past the end.
#[test]
fn a_lying_tree_length_is_clamped_to_the_file() {
    let mut data = build_tree(&[(1, 0, 0)]);
    data[4..8].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    let nodes = nodes(&data).expect("walk");
    assert_eq!(nodes.len(), 1);
}

/// The check that pins `child_count` to 16 bits: in a pre-order tree with
/// immediate child counts, the counts sum to one less than the node count.
/// The ground-truth test runs the same assertion against real files.
#[test]
fn the_child_counts_sum_to_one_root() {
    let data = build_tree(&[(1, 2, 0), (2, 2, 0), (3, 0, 0), (4, 0, 0), (5, 0, 0)]);
    let nodes = nodes(&data).expect("walk");
    let children: usize = nodes.iter().map(|n| n.child_count).sum();
    assert_eq!(nodes.len() - children, 1);
}

/// Reading `child_count` as a `u32` swallows the `u16` at `+0x0e`, which is
/// non-zero on a few dozen nodes per real file. This is what that looks like.
#[test]
fn the_word_after_child_count_is_not_part_of_it() {
    let mut data = build_tree(&[(1, 1, 0), (2, 0, 0)]);
    let root = FILE_HEADER_LEN;
    data[root + 14..root + 16].copy_from_slice(&24u16.to_le_bytes());

    let nodes = nodes(&data).expect("walk");
    assert_eq!(nodes[0].child_count, 1, "a u32 read would give 1_572_865");
    assert_eq!(nodes[0].unk_0x0e, 24);
    assert_eq!(nodes[1].depth, 1, "and the depth stack would be corrupt");
    assert_eq!(nodes[1].parent, Some(0));
}

#[test]
fn parents_follow_the_tree() {
    // root -> a -> a1, a2; root -> b
    let data = build_tree(&[(1, 2, 0), (2, 2, 0), (3, 0, 0), (4, 0, 0), (5, 0, 0)]);
    let nodes = nodes(&data).expect("walk");
    let parents: Vec<Option<usize>> = nodes.iter().map(|n| n.parent).collect();
    assert_eq!(parents, [None, Some(0), Some(1), Some(1), Some(0)]);
}

#[test]
fn an_empty_transform_payload_is_the_identity() {
    assert_eq!(transform(&[], ByteOrder::Little), Some(IDENTITY));
    assert_eq!(
        transform(&[0u8; 32], ByteOrder::Little),
        None,
        "too short to be a matrix"
    );
}

#[test]
fn multiply_composes_in_row_vector_order() {
    let mut translate = IDENTITY;
    translate[12] = 10.0;
    let mut scale = IDENTITY;
    scale[0] = 2.0;
    scale[5] = 2.0;
    scale[10] = 2.0;

    // Scale first, then translate: the translation is not scaled.
    let m = multiply(&scale, &translate);
    assert_eq!(transform_point(&m, [1.0, 0.0, 0.0]), [12.0, 0.0, 0.0]);
    // Translate first, then scale: it is.
    let m = multiply(&translate, &scale);
    assert_eq!(transform_point(&m, [1.0, 0.0, 0.0]), [22.0, 0.0, 0.0]);
}

#[test]
fn world_transforms_compose_down_the_chain() {
    // A transform tree three deep, each translating by 1 on x, with a mesh
    // at the bottom.
    let data = {
        let mut nodes = Vec::new();
        for _ in 0..3 {
            nodes.push((CLASS_TRANSFORM, 1usize, 64usize));
        }
        nodes.push((CLASS_MESH, 0, 0));
        let mut data = build_tree(&nodes);
        // Fill each transform payload with a translate-by-one matrix.
        let mut at = FILE_HEADER_LEN;
        for _ in 0..3 {
            let payload = at + 0x20;
            for (i, v) in IDENTITY.iter().enumerate() {
                data[payload + i * 4..payload + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            data[payload + 12 * 4..payload + 12 * 4 + 4].copy_from_slice(&1.0f32.to_le_bytes());
            at = payload + 64;
        }
        data
    };

    let nodes = nodes(&data).expect("walk");
    assert_eq!(nodes.len(), 4);
    let world = world_transforms(&data, &nodes);
    assert_eq!(transform_point(&world[0], [0.0; 3]), [1.0, 0.0, 0.0]);
    assert_eq!(transform_point(&world[1], [0.0; 3]), [2.0, 0.0, 0.0]);
    assert_eq!(transform_point(&world[2], [0.0; 3]), [3.0, 0.0, 0.0]);
    // The mesh inherits its parent chain without contributing.
    assert_eq!(transform_point(&world[3], [0.0; 3]), [3.0, 0.0, 0.0]);
}
