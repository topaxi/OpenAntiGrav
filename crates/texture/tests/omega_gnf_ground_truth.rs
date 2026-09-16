//! `oag_texture::gnf::Texture::parse` against every real `.gnf` this project
//! can extract from `omega-ps4-eu`'s five `dataNN.psarc` archives.
//!
//! **`#[ignore]`d and never run in CI.** Needs
//! `data/extracted/ps4/omega-eu/uroot/`, which this project does not ship -
//! see `docs/reverse-engineering/source-images.md`'s Omega Collection
//! section.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this asserts, and what it deliberately does not
//!
//! Not a "real content" rate: `docs/formats/psarc.md`'s "Block data
//! location" section (and `crates/assets/tests/omega_psarc_ground_truth.rs`'s
//! own module docs) already establish that whether a given entry's bytes are
//! present at all is a property of this local dump, not of the reader, so a
//! floor on how many `.gnf` entries decode with valid magic would break the
//! day someone re-extracts the `.pkg`. What *is* a fact about the parser:
//!
//! 1. Every entry parses to either a valid `GNF ` header with a sane
//!    descriptor, or a clean [`oag_texture::gnf::Error`] - never a panic -
//!    over the whole real corpus, several hundred files per archive.
//! 2. At least one entry per archive decodes with the expected magic and
//!    plausible dimensions - the positive control that would catch a
//!    regression in the header/descriptor bit math specifically.
//! 3. A curated known-real entry (the one `docs/formats/psarc.md` names
//!    directly) decodes to the exact fields this project hand-verified
//!    against the raw bytes: BC7/sRGB, 128x64, tiled.

use std::path::PathBuf;

use oag_assets::psarc::Archive;
use oag_texture::gnf::{self, SurfaceFormat, Texture};

const ARCHIVES: &[&str] = &[
    "data00.psarc",
    "data01.psarc",
    "data02.psarc",
    "data03.psarc",
    "data04.psarc",
];

fn open(name: &str) -> Option<Archive> {
    let path: PathBuf = oag_testdata::exact(&format!("data/extracted/ps4/omega-eu/uroot/{name}"))?;
    Some(Archive::open_file(&path).unwrap_or_else(|e| panic!("open {name}: {e}")))
}

fn check_archive(name: &str) {
    let Some(mut archive) = open(name) else {
        return;
    };
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
        .cloned()
        .collect();
    assert!(!paths.is_empty(), "{name}: no .gnf entries at all");

    let mut valid = 0usize;
    for path in &paths {
        let bytes = archive
            .read_path(path)
            .unwrap_or_else(|e| panic!("{name}: {path}: read failed: {e}"));
        match Texture::parse(&bytes) {
            Ok(texture) => {
                valid += 1;
                assert!(texture.width > 0, "{name}: {path}: zero width");
                assert!(texture.height > 0, "{name}: {path}: zero height");
            }
            Err(gnf::Error::BadMagic { .. } | gnf::Error::TooShort { .. }) => {
                // Not a real GNF header at this offset - the still-open
                // "garbage"/"all-zero" population `docs/formats/psarc.md`
                // measures. Not this test's concern; it must not panic,
                // and it did not.
            }
            Err(other) => panic!("{name}: {path}: unexpected error: {other}"),
        }
    }
    assert!(
        valid > 0,
        "{name}: not one of {} .gnf entries decoded",
        paths.len()
    );
}

#[test]
#[ignore]
fn data00_gnf_entries_parse_or_fail_cleanly() {
    check_archive(ARCHIVES[0]);
}

#[test]
#[ignore]
fn data01_gnf_entries_parse_or_fail_cleanly() {
    check_archive(ARCHIVES[1]);
}

#[test]
#[ignore]
fn data02_gnf_entries_parse_or_fail_cleanly() {
    check_archive(ARCHIVES[2]);
}

#[test]
#[ignore]
fn data03_gnf_entries_parse_or_fail_cleanly() {
    check_archive(ARCHIVES[3]);
}

#[test]
#[ignore]
fn data04_gnf_entries_parse_or_fail_cleanly() {
    check_archive(ARCHIVES[4]);
}

#[test]
#[ignore]
fn the_known_real_holographic_glow_decodes_exactly() {
    let Some(mut archive) = open("data03.psarc") else {
        return;
    };
    let bytes = archive
        .read_path("Data/art/published/hdships/harimau/Livery2/Holographic_02_GLOW.gnf")
        .expect("reads");
    let texture = Texture::parse(&bytes).expect("parses");
    assert_eq!(texture.surface_format, SurfaceFormat::Bc7);
    assert_eq!(texture.width, 128);
    assert_eq!(texture.height, 64);
    assert!(!texture.is_linear(), "this sample is genuinely tiled");
}
