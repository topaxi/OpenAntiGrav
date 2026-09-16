use super::*;
use oag_formats::ByteOrder;

fn le_u32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}
fn le_f32(v: f32) -> [u8; 4] {
    v.to_le_bytes()
}

/// A 16-byte `cloudCube` payload, byte for byte the shape every one of the ten
/// shipped instances is.
fn shipped_cloud_cube_payload() -> Vec<u8> {
    let mut p = vec![0u8; CLOUD_CUBE_PAYLOAD_LEN];
    p[0..4].copy_from_slice(&le_u32(2));
    p[4..8].copy_from_slice(&le_f32(1.0));
    p
}

#[test]
fn a_cloud_cube_payload_is_a_type_and_a_scale() {
    let (kind, scale) =
        cloud_cube(&shipped_cloud_cube_payload(), ByteOrder::Little).expect("decode");
    assert_eq!(kind, 2);
    assert_eq!(scale, 1.0);
}

#[test]
fn a_short_payload_does_not_decode() {
    assert_eq!(cloud_cube(&[0u8; 8], ByteOrder::Little), None);
}

/// Builds a node header carrying a named-attribute list, the same shape
/// `CloudGroup_Init` walks: `+0x06` the first entry's offset, `+0x0e` the
/// list's byte length, then `{u16, u8 value_offset, u16 stride, name...}`
/// entries terminated by `stride == 0`.
fn header_with_attributes(entries: &[(&str, f32)]) -> Vec<u8> {
    let first_entry_at = 0x10;
    let mut header = vec![0u8; first_entry_at];
    let mut at = first_entry_at;
    for (name, value) in entries {
        // Name sits at `entry + 4`, NUL-terminated; the value goes right after
        // it, 4-byte aligned so `value_offset` (a `u8`) stays exact.
        let name_len = name.len() + 1;
        let value_offset = (4 + name_len).div_ceil(4) * 4;
        let stride = value_offset + 4;
        header.resize(at + stride, 0);
        header[at] = 0; // unused, not read by this module
        header[at + 1] = value_offset as u8;
        header[at + 2..at + 4].copy_from_slice(&(stride as u16).to_le_bytes());
        header[at + 4..at + 4 + name.len()].copy_from_slice(name.as_bytes());
        let value_at = at + value_offset;
        header[value_at..value_at + 4].copy_from_slice(&value.to_le_bytes());
        at += stride;
    }
    // Terminator: an entry whose stride is zero.
    header.resize(at + 4, 0);
    let list_len = (at + 4 - first_entry_at) as u16;
    header[6..8].copy_from_slice(&(first_entry_at as u16).to_le_bytes());
    header[0xe..0x10].copy_from_slice(&list_len.to_le_bytes());
    header
}

fn group_node(offset: usize, header: Vec<u8>, parent: Option<usize>) -> (Node, Vec<u8>) {
    let unk_0x0e = u16::from_le_bytes([header[0xe], header[0xf]]);
    let node = Node {
        class_id: CLASS_CLOUD_GROUP,
        offset,
        header_size: header.len(),
        data_size: 0,
        child_count: 1,
        unk_0x0e,
        name: None,
        depth: 0,
        parent,
    };
    (node, header)
}

#[test]
fn cloud_attributes_reads_every_named_field_and_defaults_the_rest() {
    let header = header_with_attributes(&[("SpriteRadius", 4.0), ("Overlap", 0.65)]);
    let mut data = vec![0u8; header.len()];
    data.copy_from_slice(&header);
    let (node, _) = group_node(0, header, None);

    let attrs = CloudAttributes::from_node(&data, &node);
    assert_eq!(attrs.sprite_radius, 4.0);
    assert_eq!(attrs.overlap, 0.65);
    // Every name not in the list reads as zero, the same fallback
    // `CloudGroup_Init` itself takes.
    assert_eq!(attrs.hi_colour, [0.0, 0.0, 0.0]);
    assert_eq!(attrs.seed, 0.0);
    assert_eq!(attrs.midpoint, 0.0);
}

/// A `cloudGroup` nested inside another must compose both matrices - the
/// arithmetic check that `class_world_transforms` alone would fail here,
/// because it reads a nested `cloudGroup`'s parent out of the stock
/// `world_transforms` chain, which treats `cloudGroup` as the identity.
#[test]
fn a_nested_cloud_group_composes_both_matrices() {
    fn transform_payload(translate: [f32; 3]) -> Vec<u8> {
        let mut m = IDENTITY;
        m[12] = translate[0];
        m[13] = translate[1];
        m[14] = translate[2];
        m.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    // world(0) -> outer cloudGroup(1, +10 on x) -> inner cloudGroup(2, +1 on y)
    //   -> Transform(3, +1 on z) -> cloudCube(4)
    let outer_payload = transform_payload([10.0, 0.0, 0.0]);
    let inner_payload = transform_payload([0.0, 1.0, 0.0]);
    let transform_payload_bytes = transform_payload([0.0, 0.0, 1.0]);
    let cube_payload = shipped_cloud_cube_payload();

    let mut data = Vec::new();
    let world_off = data.len();
    data.extend_from_slice(&[0u8; 0x10]);
    let outer_off = data.len();
    data.extend_from_slice(&outer_payload);
    let inner_off = data.len();
    data.extend_from_slice(&inner_payload);
    let transform_off = data.len();
    data.extend_from_slice(&transform_payload_bytes);
    let cube_off = data.len();
    data.extend_from_slice(&cube_payload);

    let nodes = vec![
        Node {
            class_id: vex::CLASS_MESH, // arbitrary non-matrix class for the root
            offset: world_off,
            header_size: 0x10,
            data_size: 0,
            child_count: 1,
            unk_0x0e: 0,
            name: None,
            depth: 0,
            parent: None,
        },
        Node {
            class_id: CLASS_CLOUD_GROUP,
            offset: outer_off,
            header_size: 0,
            data_size: 64,
            child_count: 1,
            unk_0x0e: 0,
            name: None,
            depth: 1,
            parent: Some(0),
        },
        Node {
            class_id: CLASS_CLOUD_GROUP,
            offset: inner_off,
            header_size: 0,
            data_size: 64,
            child_count: 1,
            unk_0x0e: 0,
            name: None,
            depth: 2,
            parent: Some(1),
        },
        Node {
            class_id: vex::CLASS_TRANSFORM,
            offset: transform_off,
            header_size: 0,
            data_size: 64,
            child_count: 1,
            unk_0x0e: 0,
            name: None,
            depth: 3,
            parent: Some(2),
        },
        Node {
            class_id: CLASS_CLOUD_CUBE,
            offset: cube_off,
            header_size: 0,
            data_size: CLOUD_CUBE_PAYLOAD_LEN,
            child_count: 0,
            unk_0x0e: 0,
            name: None,
            depth: 4,
            parent: Some(3),
        },
    ];

    let world = cloud_world_transforms(&data, &nodes);
    let leaf = world[4];
    assert_eq!([leaf[12], leaf[13], leaf[14]], [10.0, 1.0, 1.0]);

    let found = clouds(&data, &nodes);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0.world_position, [10.0, 1.0, 1.0]);
    assert_eq!(found[0].0.kind, 2);
    assert_eq!(found[0].0.scale, 1.0);
}

/// A `cloudCube` with no `cloudGroup` ancestor at all is not a shape any
/// shipped file authors; [`clouds`] must not invent parameters for it.
#[test]
fn a_cloud_cube_with_no_owning_group_is_skipped() {
    let data = vec![0u8; 0x10 + CLOUD_CUBE_PAYLOAD_LEN];
    let nodes = vec![
        Node {
            class_id: vex::CLASS_MESH,
            offset: 0,
            header_size: 0x10,
            data_size: 0,
            child_count: 1,
            unk_0x0e: 0,
            name: None,
            depth: 0,
            parent: None,
        },
        Node {
            class_id: CLASS_CLOUD_CUBE,
            offset: 0x10,
            header_size: 0,
            data_size: CLOUD_CUBE_PAYLOAD_LEN,
            child_count: 0,
            unk_0x0e: 0,
            name: None,
            depth: 1,
            parent: Some(0),
        },
    ];
    assert_eq!(clouds(&data, &nodes), Vec::new());
}
