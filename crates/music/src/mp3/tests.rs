//! Placeholder; filled in once the module compiles against symphonia's API.

use super::*;

/// Anything that is not an MPEG stream is a skip rather than a fault.
#[test]
fn a_blob_that_is_not_mpeg_is_not_described() {
    assert_eq!(describe(b"RIFF\0\0\0\0WAVE"), None);
    assert_eq!(describe(&[]), None);
}
