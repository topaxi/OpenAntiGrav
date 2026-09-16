//! `omega-ps4-eu`'s five `dataNN.psarc` archives: the digest-matched path
//! count is a container fact and a ratchet; whether an *individual* entry's
//! content is real is not (see below), so this file does not assert a
//! content rate.
//!
//! **`#[ignore]`d and never run in CI.** They need
//! `data/extracted/ps4/omega-eu/uroot/`, which this project does not ship -
//! see `docs/reverse-engineering/source-images.md`'s Omega Collection
//! section and `data/README.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this asserts, and what it deliberately does not
//!
//! [`docs/formats/psarc.md`](../../../docs/formats/psarc.md)'s "Block data
//! location" section measures that a large fraction of this family's real
//! entries read back as either all-zero or "garbage" (present bytes that do
//! not match their extension's own magic) through the crate's own,
//! unmodified reader - **and that this is not a reader bug**: the same
//! split reproduces across two independent extractions of two different
//! `.pkg` files, `entry.offset` spans each archive's declared length
//! exactly, and a small sample rules out a constant per-entry offset error.
//! A test asserting a real-content-rate *floor* would therefore be asserting
//! a fact about this one local dump, not about `oag_formats::psarc` or
//! `oag_assets::psarc::Archive` - it would break the day someone re-extracts
//! the `.pkg` and gets a differently-provisioned copy, with no reader
//! change at fault. See that page for the measured numbers themselves.
//!
//! What *is* a fact about the reader, and does belong in a ratchet:
//!
//! 1. The digest-matched path count per archive (`docs/formats/psarc.md`'s
//!    "Manifest delimiter and entry/path correspondence" table) - a
//!    regression here means `parse_manifest`/`match_paths_to_entries`
//!    broke, not that the dump changed.
//! 2. Every digest-matched entry either reads without error, or fails with
//!    exactly the one documented corrupt row per archive
//!    (`docs/formats/psarc.md`'s "A single corrupt row per archive"
//!    section) - anything else is a new, unexplained read failure.
//! 3. A small anchor set of known-real `.gnf`/`.vex` entries still decode on
//!    their own magic - the positive control `docs/formats/psarc.md`'s
//!    "valid" bucket numbers rest on.

use std::path::PathBuf;

use oag_assets::psarc::Archive;

fn archive_path(name: &str) -> Option<PathBuf> {
    oag_testdata::exact(&format!("data/extracted/ps4/omega-eu/uroot/{name}"))
}

fn open(name: &str) -> Option<Archive> {
    let path = archive_path(name)?;
    Some(Archive::open_file(&path).unwrap_or_else(|e| panic!("open {name}: {e}")))
}

/// One archive's expected shape: digest-matched path count, and the single
/// known corrupt directory index (real digest, unreadable geometry), if any.
/// Numbers from `docs/formats/psarc.md`'s own measurements.
struct Expected {
    archive: &'static str,
    matched_paths: usize,
    corrupt_index: Option<usize>,
}

fn check(expected: &Expected) {
    let Some(mut archive) = open(expected.archive) else {
        return;
    };

    assert_eq!(
        archive.paths().len(),
        expected.matched_paths,
        "{}: digest-matched path count",
        expected.archive
    );

    let mut unexpected_errors = Vec::new();
    // Walk by directory index directly - `paths()` order does not carry the
    // index, and re-deriving it via `index_of_path` would re-run the same
    // digest match `paths()` already did once.
    let indices: Vec<usize> = (0..archive.directory().entries.len())
        .filter(|&i| archive.directory().entries[i].digest != [0u8; 16])
        .collect();
    for index in indices {
        if let Err(e) = archive.read(index) {
            if Some(index) == expected.corrupt_index {
                continue;
            }
            unexpected_errors.push(format!("{index}: {e}"));
        }
    }
    assert!(
        unexpected_errors.is_empty(),
        "{}: unexpected read errors: {unexpected_errors:?}",
        expected.archive
    );
}

/// A path known to decode on its own magic, for the positive control - see
/// module docs, point 3.
fn assert_decodes_as_gnf(archive: &mut Archive, path: &str) {
    let bytes = archive
        .read_path(path)
        .unwrap_or_else(|e| panic!("{path}: {e}"));
    assert_eq!(&bytes[0..4], b"GNF ", "{path}: expected GNF magic at +0x00");
}

fn assert_decodes_as_vex(archive: &mut Archive, path: &str) {
    let bytes = archive
        .read_path(path)
        .unwrap_or_else(|e| panic!("{path}: {e}"));
    assert!(bytes.len() >= 0x10, "{path}: too short for a VEXX header");
    assert_eq!(
        &bytes[0x0c..0x10],
        b"VEXX",
        "{path}: expected VEXX magic at +0x0c"
    );
}

#[test]
#[ignore]
fn data00_matches_its_manifest_and_reads_without_new_errors() {
    check(&Expected {
        archive: "data00.psarc",
        matched_paths: 1_492,
        corrupt_index: Some(9042),
    });
}

#[test]
#[ignore]
fn data01_matches_its_manifest_and_reads_without_new_errors() {
    check(&Expected {
        archive: "data01.psarc",
        matched_paths: 891,
        corrupt_index: None,
    });
}

#[test]
#[ignore]
fn data02_matches_its_manifest_and_reads_without_new_errors() {
    check(&Expected {
        archive: "data02.psarc",
        matched_paths: 923,
        corrupt_index: Some(4676),
    });
}

#[test]
#[ignore]
fn data03_matches_its_manifest_and_reads_without_new_errors() {
    check(&Expected {
        archive: "data03.psarc",
        matched_paths: 327,
        corrupt_index: None,
    });
}

#[test]
#[ignore]
fn data04_matches_its_manifest_and_reads_without_new_errors() {
    check(&Expected {
        archive: "data04.psarc",
        matched_paths: 837,
        corrupt_index: Some(318),
    });
}

/// The positive control: a handful of entries `docs/formats/psarc.md`'s
/// "valid" bucket already names, still decoding on their own magic through
/// the unmodified reader. Kept separate from the per-archive checks above so
/// a magic regression (as opposed to a manifest/read regression) names
/// itself precisely.
#[test]
#[ignore]
fn known_real_entries_still_decode_on_their_own_magic() {
    let Some(mut archive) = open("data03.psarc") else {
        return;
    };
    assert_decodes_as_gnf(
        &mut archive,
        "Data/art/published/hdships/harimau/Livery2/Holographic_02_GLOW.gnf",
    );
    assert_decodes_as_vex(
        &mut archive,
        "Data/art/published/hdships/auricom/Ship_LOD.vex",
    );
}
