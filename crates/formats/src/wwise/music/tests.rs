use super::*;

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

/// A music node head: flags, no effects, parent, no properties, the four
/// opaque bytes, empty state chunk and RTPC list, the children and a meter.
fn node_bytes(parent: u32, children: &[u32], state_props: u32, rtpc: u16) -> Vec<u8> {
    let mut b = vec![0, 0, 0, 0];
    b.extend(le32(0));
    b.extend(le32(parent));
    b.extend([0, 0, 0]);
    b.extend([0xc0, 0, 0, 1]);
    b.extend(le32(state_props));
    b.extend(le32(0));
    b.extend(rtpc.to_le_bytes());
    b.extend(le32(children.len() as u32));
    for c in children {
        b.extend(le32(*c));
    }
    b.extend([0u8; 16]);
    b.extend(120f32.to_le_bytes());
    b.extend([4, 4, 0xff]);
    b.extend(le32(0));
    b
}

#[test]
fn a_segment_reads_to_its_last_byte_and_a_stray_byte_is_refused() {
    let mut b = node_bytes(7, &[1, 2, 3], 0, 0);
    b.extend(5000f64.to_le_bytes());
    b.extend(le32(1));
    b.extend(le32(9));
    b.extend(250f64.to_le_bytes());
    b.extend(le32(0));
    let seg = Segment::parse(Kind::MusicSegment, &b).expect("reads");
    assert_eq!(seg.node.children, [1, 2, 3]);
    assert_eq!((seg.node.parent, seg.duration), (Some(7), 5000.0));
    assert_eq!(seg.markers, [(9, 250.0)]);
    b.push(0);
    assert!(matches!(
        Segment::parse(Kind::MusicSegment, &b),
        Err(MusicError::TrailingBytes { .. })
    ));
}

#[test]
fn unmeasured_state_properties_and_rtpcs_are_refused_by_name() {
    let props = node_bytes(0, &[], 2, 0);
    assert_eq!(
        Segment::parse(Kind::MusicSegment, &props),
        Err(MusicError::StateProperties { count: 2 })
    );
    let rtpc = node_bytes(0, &[], 0, 3);
    assert_eq!(
        Segment::parse(Kind::MusicSegment, &rtpc),
        Err(MusicError::Rtpc { count: 3 })
    );
    assert_eq!(
        Segment::parse(Kind::MusicTrack, &[]),
        Err(MusicError::WrongKind(Kind::MusicTrack))
    );
}

#[test]
fn a_switch_tree_selects_by_state_and_a_dangling_range_is_refused() {
    let mut b = node_bytes(0, &[], 0, 0);
    b.extend(le32(0));
    b.push(1);
    b.extend(le32(1));
    b.extend(le32(0xAAAA));
    b.push(1);
    b.extend(le32(36));
    b.push(0);
    for (key, target) in [(0, 1 | 2 << 16), (11, 500), (22, 600)] {
        b.extend(le32(key));
        b.extend(le32(target));
        b.extend([50, 0, 100, 0]);
    }
    let switch = Switch::parse(Kind::MusicSwitch, &b).expect("reads");
    assert_eq!(switch.groups, [(0xAAAA, 1)]);
    assert_eq!(switch.select(&[22]), Some(600));
    assert_eq!(switch.select(&[33]), None);
    let mut bad = switch.clone();
    bad.tree[0].target = 2 | 5 << 16;
    assert_eq!(bad.leaves(), Err(MusicError::BadTree));
}

#[test]
fn a_playlist_whose_child_counts_do_not_add_up_is_refused() {
    let item = |segment: u32, children: u32| {
        let mut i = Vec::new();
        i.extend(le32(segment));
        i.extend(le32(1));
        i.extend(le32(children));
        i.extend(le32(0xffff_ffff));
        i.extend([1, 0, 0, 0, 0, 0]);
        i.extend(le32(50_000));
        i.extend([0, 0, 0, 0]);
        i
    };
    let mut ok = node_bytes(0, &[], 0, 0);
    ok.extend(le32(0));
    ok.extend(le32(2));
    ok.extend(item(0, 1));
    ok.extend(item(77, 0));
    let r = RanSeq::parse(Kind::MusicRanSeq, &ok).expect("reads");
    assert_eq!(r.segments(), [77]);

    let mut bad = node_bytes(0, &[], 0, 0);
    bad.extend(le32(0));
    bad.extend(le32(2));
    bad.extend(item(0, 2));
    bad.extend(item(77, 0));
    assert_eq!(
        RanSeq::parse(Kind::MusicRanSeq, &bad),
        Err(MusicError::BadTree)
    );
}

#[test]
fn the_name_hash_is_fnv1_over_the_lower_case_name() {
    use crate::wwise::name_hash;
    assert_eq!(name_hash("Menus"), 2_604_644_515);
    assert_eq!(name_hash("MENUS"), name_hash("menus"));
    assert_eq!(name_hash("Set_Music_Track_1__frontend"), 1_781_619_141);
}
