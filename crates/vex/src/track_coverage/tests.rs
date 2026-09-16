use super::*;

/// A minimal `section` payload with no bounding box and a short name.
fn section_payload(has_bounds: bool, mask: u64, name: &str) -> Vec<u8> {
    let mut out = vec![0u8; SECTION_FIXED_LEN];
    out[0] = 0;
    out[1] = u8::from(has_bounds);
    out[8..16].copy_from_slice(&mask.to_le_bytes());
    if has_bounds {
        out.extend_from_slice(&[0u8; SECTION_BOUNDS_LEN]);
    }
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out
}

#[test]
fn a_section_with_no_bounds_leaves_pad6_unclaimed() {
    let payload = section_payload(false, 0xff, "moa");
    let seen = section_coverage(&payload);
    let gaps = seen.gaps(1);
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    assert_eq!((gaps[0].at, gaps[0].len), (2, 6));
}

#[test]
fn a_section_with_bounds_still_leaves_only_pad6_unclaimed() {
    let payload = section_payload(true, 0xff, "moa");
    let seen = section_coverage(&payload);
    let gaps = seen.gaps(1);
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    assert_eq!((gaps[0].at, gaps[0].len), (2, 6));
}

#[test]
fn too_short_a_payload_claims_nothing() {
    let seen = section_coverage(&[0u8; 4]);
    assert_eq!(seen.claimed(), 0);
}

#[test]
fn a_wo_track_payload_that_does_not_parse_claims_nothing() {
    let seen = wo_track_coverage(&[0u8; 8]);
    assert_eq!(seen.claimed(), 0);
}

#[test]
fn a_collision_payload_that_does_not_parse_claims_nothing() {
    let seen = collision_coverage(&[0u8; 4], ByteOrder::Little);
    assert_eq!(seen.claimed(), 0);
}
