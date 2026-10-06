//! What the authored potentially-visible-set reader in [`super`] is asserted to
//! do: the section masks it decodes, and the payloads it refuses.

use super::*;
use crate::track::{AiTrack, Junction, Path, SplinePoint};

fn point(section_id: u8) -> SplinePoint {
    SplinePoint {
        pos: [0.0; 3],
        tangent: [0.0, 0.0, 1.0],
        down: [0.0, -1.0, 0.0],
        lateral: [1.0, 0.0, 0.0],
        progress: 0.0,
        half_width_left: 1.0,
        half_width_right: 1.0,
        ai_bound_left: 1.0,
        ai_bound_right: 1.0,
        racing_line: 0.0,
        section_id,
        flags: 0,
        light_scale: [0xff; 4],
    }
}

fn path(section_ids: &[u8]) -> Path {
    Path {
        points: section_ids.iter().copied().map(point).collect(),
        max_spacing: 1.0,
        entry: None,
        exit: None,
    }
}

/// Builds a `section` payload by hand, in the layout the module documents.
fn payload(index: u8, lo: u32, hi: u32, bounds: Option<([f32; 3], [f32; 3])>) -> Vec<u8> {
    let mut out = vec![index, u8::from(bounds.is_some()), 0, 0, 0, 0, 0, 0];
    out.extend(lo.to_le_bytes());
    out.extend(hi.to_le_bytes());
    if let Some((min, max)) = bounds {
        for v in min {
            out.extend(v.to_le_bytes());
        }
        out.extend(0f32.to_le_bytes());
        for v in max {
            out.extend(v.to_le_bytes());
        }
        out.extend(0f32.to_le_bytes());
    }
    out
}

/// A `Node` standing in for one the tree walker produced (the walk is `vex`'s
/// business; the payload reading is under test here).
fn node(offset: usize, len: usize) -> Node {
    Node {
        class_id: vex::CLASS_SECTION,
        offset,
        header_size: 0,
        data_size: len,
        child_count: 0,
        unk_0x0e: 0,
        name: None,
        depth: 0,
        parent: None,
    }
}

/// A minimal `.vex` file header, so a fixture's `data` is a file:
/// [`TrackPvs::from_nodes`] reads the class table out of the version word.
fn vex_header() -> Vec<u8> {
    let mut header = vec![0u8; vex::FILE_HEADER_LEN];
    header[0..4].copy_from_slice(&6u32.to_le_bytes());
    header[0x0c..0x10].copy_from_slice(vex::MAGIC);
    header
}

/// The same header written the way the PS3 exporter writes it.
fn vex_header_be() -> Vec<u8> {
    let mut header = vec![0u8; vex::FILE_HEADER_LEN];
    header[0..4].copy_from_slice(&6u32.to_be_bytes());
    header[0x0c..0x10].copy_from_slice(vex::MAGIC_BE);
    header
}

/// One `section` payload with the whole mask as a single big-endian `u64`.
fn payload_be(index: u8, mask: u64) -> Vec<u8> {
    let mut out = vec![index, 0, 0, 0, 0, 0, 0, 0];
    out.extend(mask.to_be_bytes());
    out
}

fn pvs_from(payloads: &[Vec<u8>]) -> Result<TrackPvs> {
    let mut data = vex_header();
    let mut nodes = Vec::new();
    for p in payloads {
        nodes.push(node(data.len(), p.len()));
        data.extend_from_slice(p);
    }
    TrackPvs::from_nodes(&data, &nodes)
}

/// [`pvs_from`] against a file whose magic says big-endian.
fn pvs_from_be(payloads: &[Vec<u8>]) -> Result<TrackPvs> {
    let mut data = vex_header_be();
    let mut nodes = Vec::new();
    for p in payloads {
        nodes.push(node(data.len(), p.len()));
        data.extend_from_slice(p);
    }
    TrackPvs::from_nodes(&data, &nodes)
}

fn tree_node(class_id: u32, offset: usize, len: usize, parent: Option<usize>) -> Node {
    Node {
        class_id,
        offset,
        header_size: 0,
        data_size: len,
        child_count: 0,
        unk_0x0e: 0,
        name: None,
        depth: 0,
        parent,
    }
}

/// A section node governs its parent's whole subtree, and nothing above
/// or beside it.
#[test]
fn a_section_governs_its_parents_subtree() {
    let section = payload(7, 0, 0, None);
    let mut data = vex_header();
    data.extend_from_slice(&section);
    // 0 root
    // +-- 1 group transform
    // |   +-- 2 section id 7
    // |   +-- 3 mesh
    // |   +-- 4 transform
    // |       +-- 5 mesh
    // +-- 6 mesh, in no group
    let nodes = vec![
        tree_node(0x6e, 0, 0, None),
        tree_node(0x6e, 0, 0, Some(0)),
        tree_node(
            vex::CLASS_SECTION,
            vex::FILE_HEADER_LEN,
            section.len(),
            Some(1),
        ),
        tree_node(vex::CLASS_MESH, 0, 0, Some(1)),
        tree_node(0x6e, 0, 0, Some(1)),
        tree_node(vex::CLASS_MESH, 0, 0, Some(4)),
        tree_node(vex::CLASS_MESH, 0, 0, Some(0)),
    ];
    let governing = governing_sections(&data, &nodes).expect("derive");
    assert_eq!(
        governing,
        vec![None, Some(7), Some(7), Some(7), Some(7), Some(7), None]
    );
}

/// The mask is one 64-bit field, and on a little-endian file that is exactly
/// the two words joined low word first.
#[test]
fn the_mask_is_one_64_bit_field_low_word_first_on_a_little_endian_file() {
    let pvs = pvs_from(&[payload(0, 0x0000_0002, 0x0000_0001, None)]).expect("parse");
    assert_eq!(pvs.visible_from(0), 0x0000_0001_0000_0002 | 1);
}

/// The same mask on a big-endian file, and **not** the two words swapped in
/// place.
///
/// The module's one silent-garbage trap: a mechanical `from_le_bytes` ->
/// `from_be_bytes` sweep over a `lo`/`hi` pair raises nothing. Measured over
/// Wipeout HD's 24 circuits, that reading names an undeclared section on 55.6 %
/// of set bits against 22.2 % for the correct one. See
/// `docs/formats/hd-status.md#the-pvs-mask-and-the-byte-order-trap`.
#[test]
fn a_big_endian_mask_is_one_field_and_not_two_words_swapped_in_place() {
    let mask = 0x0000_0001_0000_0002u64;
    let pvs = pvs_from_be(&[payload_be(0, mask)]).expect("parse");
    assert_eq!(pvs.visible_from(0), mask | 1);

    // What the wrong reading would produce, spelled out so the two visibly differ.
    let swapped_in_place = mask.rotate_left(32);
    assert_ne!(
        pvs.visible_from(0),
        swapped_in_place | 1,
        "the halves must not come out the other way round"
    );
}

/// The original ORs a section's own bit in at load, so a mask is never zero
/// and a section always sees itself.
#[test]
fn a_section_always_sees_itself() {
    let pvs = pvs_from(&[payload(5, 0, 0, None)]).expect("parse");
    assert_eq!(pvs.visible_from(5), 1 << 5);
}

/// `09_Track` reversed has 44 sections and a maximum id of 45. Nothing may
/// assume `id < count`.
#[test]
fn ids_may_be_sparse() {
    let pvs = pvs_from(&[payload(0, 0, 0, None), payload(45, 0, 0, None)]).expect("parse");
    assert_eq!(pvs.len(), 2);
    assert!(pvs.declares(45), "a sparse id must survive the parse");
    assert!(!pvs.declares(1), "an id no node declared is not declared");
    assert_eq!(pvs.ids().collect::<Vec<_>>(), vec![0, 45]);
}

/// The whole safety property: anything the track does not know about is
/// visible, never hidden.
#[test]
fn an_unknown_id_sees_everything() {
    let pvs = pvs_from(&[payload(0, 0, 0, None)]).expect("parse");
    assert_eq!(pvs.visible_from(1), ALL_VISIBLE, "an undeclared id");
    assert_eq!(pvs.visible_from(63), ALL_VISIBLE, "the last legal id");
    assert_eq!(pvs.visible_from(200), ALL_VISIBLE, "past the cap entirely");
    assert_eq!(
        TrackPvs::empty().visible_from(0),
        ALL_VISIBLE,
        "no sections at all"
    );
}

/// Not knowing where the camera is must mean drawing everything.
#[test]
fn a_union_over_no_ids_sees_everything() {
    let pvs = pvs_from(&[payload(0, 0, 0, None)]).expect("parse");
    assert_eq!(pvs.visible_from_any([]), ALL_VISIBLE);
}

#[test]
fn a_union_is_the_or_of_its_parts() {
    let pvs = pvs_from(&[payload(0, 0b0100, 0, None), payload(1, 0b1000, 0, None)]).expect("parse");
    assert_eq!(pvs.visible_from_any([0, 1]), 0b1111);
}

#[test]
fn the_optional_bounding_box_is_read_when_the_flag_is_set() {
    let with = payload(0, 0, 0, Some(([-1.0, -2.0, -3.0], [4.0, 5.0, 6.0])));
    let pvs = pvs_from(&[with, payload(1, 0, 0, None)]).expect("parse");
    assert_eq!(
        pvs.bounds_of(0),
        Some(Aabb {
            min: [-1.0, -2.0, -3.0],
            max: [4.0, 5.0, 6.0]
        })
    );
    assert_eq!(pvs.bounds_of(1), None, "has_bounds was clear");
}

/// The fourth float of each corner is alignment padding, not a coordinate:
/// a box read three-wide would pick up the pad as the next axis.
#[test]
fn a_corner_is_three_coordinates_and_a_pad() {
    let with = payload(0, 0, 0, Some(([1.0, 2.0, 3.0], [7.0, 8.0, 9.0])));
    let pvs = pvs_from(&[with]).expect("parse");
    let bounds = pvs.bounds_of(0).expect("bounds");
    assert_eq!(bounds.min, [1.0, 2.0, 3.0]);
    assert_eq!(
        bounds.max,
        [7.0, 8.0, 9.0],
        "the max corner starts at +0x20, not +0x1c"
    );
}

#[test]
fn a_point_outside_every_box_locates_nowhere() {
    let with = payload(3, 0, 0, Some(([0.0; 3], [1.0, 1.0, 1.0])));
    let pvs = pvs_from(&[with]).expect("parse");
    assert_eq!(pvs.section_at([0.5, 0.5, 0.5]), Some(3));
    assert_eq!(pvs.section_at([9.0, 9.0, 9.0]), None);
}

#[test]
fn an_index_past_the_cap_is_refused() {
    let err = pvs_from(&[payload(64, 0, 0, None)]).expect_err("must refuse");
    assert_eq!(err, Error::IndexOutOfRange { node: 0, index: 64 });
}

/// Shipped tracks author one id several times over with agreeing masks, so this
/// fixes the resolution for a case the data does not produce, the safe way: draw
/// more.
#[test]
fn two_nodes_claiming_one_index_are_unioned() {
    let pvs = pvs_from(&[
        payload(2, 0b0001_0000, 0, Some(([0.0; 3], [1.0, 1.0, 1.0]))),
        payload(2, 0b0010_0000, 0, Some(([-5.0; 3], [0.5, 0.5, 0.5]))),
    ])
    .expect("parse");
    assert_eq!(pvs.len(), 1, "one id, however many nodes claim it");
    assert_eq!(
        pvs.visible_from(2),
        0b0011_0100,
        "both masks, and its own bit"
    );
    assert_eq!(
        pvs.bounds_of(2),
        Some(Aabb {
            min: [-5.0; 3],
            max: [1.0, 1.0, 1.0]
        }),
        "the boxes union rather than one winning"
    );
}

/// A mask may name a section the file does not declare (one bit in fifty on
/// shipped data). It must survive the parse: filtering would hide a real
/// misparse behind a clean result.
#[test]
fn a_mask_may_name_an_undeclared_section() {
    let pvs = pvs_from(&[payload(0, 1 << 9, 0, None)]).expect("parse");
    assert!(pvs.visible_from(0) & (1 << 9) != 0);
    assert!(!pvs.declares(9));
}

#[test]
fn a_truncated_payload_is_refused_rather_than_read_short() {
    let short = payload(0, 0, 0, None)[..12].to_vec();
    let err = pvs_from(&[short]).expect_err("must refuse");
    assert!(matches!(err, Error::TooShort { .. }), "got {err:?}");

    let mut claims_bounds = payload(0, 0, 0, Some(([0.0; 3], [1.0; 3])));
    claims_bounds.truncate(FIXED_LEN + 8);
    let err = pvs_from(&[claims_bounds]).expect_err("must refuse");
    assert!(matches!(err, Error::TooShort { .. }), "got {err:?}");
}

/// Adjacency comes from the spline, because ids are neither dense nor
/// ordered along the track. Here 7 sits between 0 and 3 on the path, so it
/// neighbours both, while 0 and 3 do not touch at one hop.
#[test]
fn adjacency_follows_the_spline_and_not_the_numbering() {
    let track = AiTrack {
        version: 0x105,
        paths: vec![path(&[0, 0, 7, 7, 3, 3])],
        junctions: Vec::new(),
    };
    let one = SectionAdjacency::from_track(&track, 1);
    assert_eq!(one.near(7), (1 << 7) | 1 | (1 << 3));
    assert_eq!(one.near(0), 1 | (1 << 7), "0 does not touch 3 in one hop");

    let two = SectionAdjacency::from_track(&track, 2);
    assert_eq!(two.near(0), 1 | (1 << 7) | (1 << 3), "two hops reaches 3");
}

/// A ring track built from several paths must not have a seam the padding
/// fails to cross.
#[test]
fn adjacency_crosses_a_junction() {
    let track = AiTrack {
        version: 0x105,
        paths: vec![path(&[0, 1]), path(&[2, 3])],
        junctions: vec![Junction {
            prev: [Some(0), None],
            next: [Some(1), None],
        }],
    };
    let one = SectionAdjacency::from_track(&track, 1);
    assert!(
        one.near(1) & (1 << 2) != 0,
        "the join between paths is an edge"
    );
}

#[test]
fn zero_hops_pads_to_nothing_and_a_capped_id_pads_to_nothing() {
    let track = AiTrack {
        version: 0x105,
        paths: vec![path(&[0, 1])],
        junctions: Vec::new(),
    };
    assert_eq!(SectionAdjacency::from_track(&track, 0).near(0), 1);
    assert_eq!(SectionAdjacency::none().near(200), 0);
}

#[test]
fn set_bits_walks_a_mask_in_order() {
    assert_eq!(set_bits(0b1001).collect::<Vec<_>>(), vec![0, 3]);
    assert_eq!(set_bits(ALL_VISIBLE).count(), MAX_SECTIONS);
    assert_eq!(set_bits(0).next(), None);
}
