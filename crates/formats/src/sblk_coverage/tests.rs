use super::*;

#[test]
fn a_blob_that_does_not_parse_claims_nothing() {
    let seen = coverage(&[0u8; 8]);
    assert_eq!(seen.claimed(), 0);
}

#[test]
fn an_empty_blob_claims_nothing_rather_than_panicking() {
    let seen = coverage(&[]);
    assert_eq!(seen.claimed(), 0);
}
