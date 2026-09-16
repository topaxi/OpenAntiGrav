use super::*;

#[test]
fn a_non_sysp_blob_claims_nothing() {
    let seen = coverage(&[0u8; 16]);
    assert_eq!(seen.claimed(), 0);
}

#[test]
fn too_short_a_blob_claims_nothing_rather_than_panicking() {
    let seen = coverage(&[0u8; 4]);
    assert_eq!(seen.claimed(), 0);
}
