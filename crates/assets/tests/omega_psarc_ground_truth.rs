//! `omega-ps4-eu`'s five `dataNN.psarc` archives, on the corrected
//! extraction: the digest-matched path count is a container fact and a
//! ratchet, and so is the content of every entry whose extension carries a
//! magic.
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
//! # What this asserts
//!
//! [`docs/formats/psarc.md`](../../../docs/formats/psarc.md) measures the
//! archives on the **corrected extraction** (`PkgTool.Core` with the short
//! `Stream.Read` in `PFSCReader.ReadSector` fixed, 2026-09-27; promoted to
//! `data/extracted/ps4` 2026-09-29). Every number below is from that copy.
//! An earlier copy of this file ratcheted the short-read extraction's
//! numbers instead - about a seventh of each manifest's paths, thousands of
//! zeroed placeholder rows and one "corrupt" row per archive - and all of it
//! was the extractor's zero-padding, not a property of the archives. This
//! file therefore now fails on a short-read copy, on purpose: the numbers
//! cannot agree with it.
//!
//! What is asserted, per archive:
//!
//! 1. The digest-matched path count, and that it names **every** entry
//!    behind the manifest: no placeholder row, no orphan entry, no path
//!    without an entry. A regression here means `parse_manifest` or
//!    `match_paths_to_entries` broke - or the tree is not a whole extraction.
//! 2. Every entry reads without error.
//! 3. Every entry whose extension carries a magic (`.gnf`, `.vex`, the four
//!    `.rcs*` kinds) carries it, the one exception being a big-endian
//!    `.vex` (`XXEV`), which is real content in the other byte order and is
//!    counted, not skipped. A whole extraction has no other misses; a torn
//!    one has thousands.
//! 4. A small anchor set of known-real `.gnf`/`.vex` entries still decode on
//!    their own magic - the positive control.

use std::path::PathBuf;

use oag_assets::psarc::Archive;

fn archive_path(name: &str) -> Option<PathBuf> {
    oag_testdata::exact(&format!("data/extracted/ps4/omega-eu/uroot/{name}"))
}

fn open(name: &str) -> Option<Archive> {
    let path = archive_path(name)?;
    Some(Archive::open_file(&path).unwrap_or_else(|e| panic!("open {name}: {e}")))
}

/// One archive's expected shape, from `docs/formats/psarc.md`'s own
/// measurements on the corrected extraction.
struct Expected {
    archive: &'static str,
    /// Digest-matched paths, which is also every entry behind the manifest.
    matched_paths: usize,
    /// Entries whose extension carries a magic and whose bytes are the
    /// big-endian `.vex` (`XXEV`) rather than the little-endian one.
    big_endian_vex: usize,
}

/// The magic an extension's bytes open with, and where, for the formats that
/// carry one (`.png` included: 42 on `data00`, the only base archive with any). Little-endian `.vex` is `VEXX` at `+0x0c`; the other byte order
/// is checked separately. `.rcsmodel`, `.rcsskeleton` and `.rcsanimclip`
/// share one tag and `.rcsmaterial` has its own - see
/// `crates/assets/examples/psarc_oracle.rs`.
fn magic_of(extension: &str) -> Option<(usize, &'static [u8])> {
    match extension {
        "gnf" => Some((0x00, b"GNF ")),
        "png" => Some((0x00, b"\x89PNG\r\n\x1a\n")),
        "vex" => Some((0x0c, b"VEXX")),
        "rcsmodel" | "rcsskeleton" | "rcsanimclip" => Some((0x00, &[0xed, 0xad, 0x5c, 0xca])),
        "rcsmaterial" => Some((0x00, &[0xe5, 0xad, 0x5c, 0xca])),
        _ => None,
    }
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
    assert_eq!(
        archive.directory().entries.len(),
        expected.matched_paths + 1,
        "{}: every entry behind the manifest names a manifest path",
        expected.archive
    );

    let mut unexpected_errors = Vec::new();
    let mut wrong_magic = Vec::new();
    let mut big_endian_vex = 0;
    for path in archive.paths().to_vec() {
        let index = archive
            .index_of_path(&path)
            .expect("a listed path resolves");
        let bytes = match archive.read(index) {
            Ok(bytes) => bytes,
            Err(e) => {
                unexpected_errors.push(format!("{index} {path}: {e}"));
                continue;
            }
        };
        let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        let Some((at, magic)) = magic_of(&extension) else {
            continue;
        };
        let has = |at: usize, magic: &[u8]| bytes.get(at..at + magic.len()) == Some(magic);
        if has(at, magic) {
            continue;
        }
        if extension == "vex" && has(0x0c, b"XXEV") {
            big_endian_vex += 1;
        } else {
            wrong_magic.push(format!("{index} {path}"));
        }
    }
    assert!(
        unexpected_errors.is_empty(),
        "{}: unexpected read errors: {unexpected_errors:?}",
        expected.archive
    );
    assert!(
        wrong_magic.is_empty(),
        "{}: entries without their extension's magic: {wrong_magic:?}",
        expected.archive
    );
    assert_eq!(
        big_endian_vex, expected.big_endian_vex,
        "{}: big-endian .vex entries",
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
        matched_paths: 10_926,
        big_endian_vex: 6,
    });
}

#[test]
#[ignore]
fn data01_matches_its_manifest_and_reads_without_new_errors() {
    check(&Expected {
        archive: "data01.psarc",
        matched_paths: 4_641,
        big_endian_vex: 10,
    });
}

#[test]
#[ignore]
fn data02_matches_its_manifest_and_reads_without_new_errors() {
    check(&Expected {
        archive: "data02.psarc",
        matched_paths: 5_611,
        big_endian_vex: 8,
    });
}

#[test]
#[ignore]
fn data03_matches_its_manifest_and_reads_without_new_errors() {
    check(&Expected {
        archive: "data03.psarc",
        matched_paths: 661,
        big_endian_vex: 2,
    });
}

#[test]
#[ignore]
fn data04_matches_its_manifest_and_reads_without_new_errors() {
    check(&Expected {
        archive: "data04.psarc",
        matched_paths: 4_569,
        big_endian_vex: 0,
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
