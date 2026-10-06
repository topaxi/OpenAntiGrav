use super::*;

/// A minimal mesh-shaped payload: header, `count` empty materials, both
/// batch lists absent (offsets point past the payload so the terminator
/// check fails at once).
fn minimal_payload(material_count: u16) -> Vec<u8> {
    let materials_len = usize::from(material_count) * MATERIAL_STRIDE;
    let mut out = vec![0u8; HEADER_LEN + materials_len];
    out[2..4].copy_from_slice(&material_count.to_le_bytes());
    // Both batch-list offsets point at the payload's own end, which is
    // short of a full header, so the walk claims nothing and stops.
    let end = out.len() as u32;
    out[4..8].copy_from_slice(&end.to_le_bytes());
    out[8..12].copy_from_slice(&end.to_le_bytes());
    out
}

/// A payload whose list B offset is `0` (`Skycube`'s shape on every real sky),
/// with a mesh-flags word carrying the list-B terminator bit and header `+0x0c`
/// set so a misread as a batch's `payload_size` would step past the material
/// array into a trailing region that is an honest gap. A walk not guarding a zero
/// list offset claims into it; this pins that it does not.
#[test]
fn a_zero_list_offset_is_not_walked_even_when_its_bit_would_pass() {
    let materials_end = HEADER_LEN + MATERIAL_STRIDE;
    let mut payload = vec![0u8; materials_end + 0x40];
    payload[2..4].copy_from_slice(&1u16.to_le_bytes()); // one material
    // Mesh flags word (`+0x00`) carries bit 1, list B's terminator bit.
    payload[0..2].copy_from_slice(&0x0002u16.to_le_bytes());
    // List A's own offset, at `+0x04`: out of range, so it never walks.
    let end = payload.len() as u32;
    payload[4..8].copy_from_slice(&end.to_le_bytes());
    // List B's own offset, at `+0x08`, is the documented `0`.
    payload[8..12].copy_from_slice(&0u32.to_le_bytes());
    // Header `+0x0c`: read as a batch's `payload_size` at offset 0, it would extend
    // a fabricated batch 0x40 bytes into the trailing region, a real undecoded gap.
    payload[0x0c..0x0e].copy_from_slice(&0x40u16.to_le_bytes());

    let seen = coverage(&payload);
    assert_eq!(
        seen.claimed(),
        materials_end,
        "the trailing region must stay an honest gap, not a fabricated batch claim"
    );
    let gaps = seen.gaps(1);
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].at, materials_end);
}

#[test]
fn a_header_only_payload_has_no_gap() {
    let payload = minimal_payload(2);
    let seen = coverage(&payload);
    assert_eq!(seen.gaps(1), Vec::new());
}

#[test]
fn too_short_a_payload_claims_nothing_rather_than_panicking() {
    let seen = coverage(&[0u8; 4]);
    assert_eq!(seen.claimed(), 0);
}

#[test]
fn fogcube_claims_the_whole_128_bytes() {
    let seen = fogcube_coverage(&[0u8; 128]);
    assert_eq!(seen.claimed(), 128);
    assert_eq!(seen.gaps(1), Vec::new());
}

/// Builds the exact shape `skycube_extra_block` recognises: a lead-in whose
/// first word is the records length, `count` self-indexed `0x40`-byte
/// records, then zero padding to `total_len`.
fn extra_block(count: usize, total_len: usize) -> Vec<u8> {
    let records_len = count * TEX_TRANSFORM_STRIDE;
    let mut out = vec![0u8; total_len];
    out[0..4].copy_from_slice(&(records_len as u32).to_le_bytes());
    for index in 0..count {
        let record = 0x10 + index * TEX_TRANSFORM_STRIDE;
        out[record + 0x20..record + 0x24].copy_from_slice(&(index as u32).to_le_bytes());
    }
    out
}

#[test]
fn the_skycube_extra_block_closes_on_the_measured_shape() {
    // `06_Track`'s own numbers: 6 records, 0x1a0 total.
    let block = extra_block(6, 0x1a0);
    let mut payload = vec![0u8; 0x30];
    payload.extend_from_slice(&block);
    let geometry_at = (0x30 + block.len()) as u32;
    payload[4..8].copy_from_slice(&geometry_at.to_le_bytes());

    let found = skycube_extra_block(&payload, 0x30).expect("recognised");
    assert_eq!(found.at, 0x30);
    assert_eq!(found.len, 0x1a0);
    assert_eq!(found.record_count, 6);
}

#[test]
fn a_non_zero_trailing_pad_refuses_the_shape() {
    let mut block = extra_block(6, 0x1a0);
    *block.last_mut().unwrap() = 1;
    let mut payload = vec![0u8; 0x30];
    payload.extend_from_slice(&block);
    let geometry_at = (0x30 + block.len()) as u32;
    payload[4..8].copy_from_slice(&geometry_at.to_le_bytes());

    assert!(skycube_extra_block(&payload, 0x30).is_none());
}

#[test]
fn a_record_whose_self_index_is_wrong_refuses_the_shape() {
    let mut block = extra_block(6, 0x1a0);
    // Corrupt record 3's self-index.
    let record = 0x10 + 3 * TEX_TRANSFORM_STRIDE;
    block[record + 0x20..record + 0x24].copy_from_slice(&99u32.to_le_bytes());
    let mut payload = vec![0u8; 0x30];
    payload.extend_from_slice(&block);
    let geometry_at = (0x30 + block.len()) as u32;
    payload[4..8].copy_from_slice(&geometry_at.to_le_bytes());

    assert!(skycube_extra_block(&payload, 0x30).is_none());
}

#[test]
fn no_gap_at_all_finds_no_block() {
    let payload = minimal_payload(6);
    assert!(skycube_extra_block(&payload, HEADER_LEN + 6 * MATERIAL_STRIDE).is_none());
}
