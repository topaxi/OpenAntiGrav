//! What this table is asserted to be, without a disc.
//!
//! The archive-backed half is `crates/omega/tests/omega_title_ground_truth.rs`,
//! which re-reads `skin.xml` and the grid count off `data09.psarc` rather than
//! trusting the constants in `frontend.rs`.

use super::*;

/// Omega ships no archive name HD's or 2048's own candidates would match, so
/// there is nothing for a foreign-serial deny-list to rule out - a directory
/// source never reaches the serial check either way (see
/// [`crate::TITLE::foreign_serials`]'s own doc comment).
#[test]
fn foreign_serials_is_empty_on_the_same_terms_2048s_is() {
    assert!(TITLE.foreign_serials.is_empty());
}

/// Every archive is named exactly once across the three roles, and all nine
/// are there - the same "extra is a set, a missing row is nothing mounted"
/// check `oag_hd`'s own test runs.
#[test]
fn all_nine_archives_are_mounted_and_none_twice() {
    let mut named = TITLE.archive_names();
    assert_eq!(named.len(), 9, "{named:?}");
    named.sort();
    named.dedup();
    assert_eq!(named.len(), 9, "one archive is named in two roles");
    for name in &named {
        assert!(
            name.starts_with("uroot/data") && name.ends_with(".psarc"),
            "{name}"
        );
    }
}

/// `data09` - the archive carrying the complete front end - is the *bulk*
/// candidate, so it is searched before every other archive this title
/// mounts. This is the whole of what makes the patch-ahead-of-base design
/// work; a future edit that moved it into `extra` would compile fine and
/// silently stop winning collisions, which is exactly the failure this
/// assertion exists to catch.
#[test]
fn data09_is_the_bulk_candidate_not_an_extra_one() {
    assert_eq!(TITLE.archives.data, [(archives::DATA09, Platform::Ps4)]);
    assert!(
        !TITLE
            .archives
            .extra
            .iter()
            .any(|(name, _)| *name == archives::DATA09)
    );
}
