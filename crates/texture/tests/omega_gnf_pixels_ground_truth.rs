//! `oag_texture::gnf::Texture::decode` against every real `.gnf` this
//! project can extract from all nine Omega Collection archives (five base
//! `omega-eu` + four patch `omega-eu-patch`).
//!
//! **`#[ignore]`d and never run in CI.** Needs
//! `data/extracted/ps4/omega-eu/uroot/` and
//! `data/extracted/ps4/omega-eu-patch/uroot/`, which this project does not
//! ship - see `docs/reverse-engineering/source-images.md`'s Omega
//! Collection section.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this asserts, and what it deliberately does not
//!
//! **`decode` untiles `TileMode(13)` (`Thin_1DThin`) BC7**; the evidence is
//! `docs/formats/gnf.md`'s "Tiling" section. A base-level-clean entry decodes;
//! one whose base level has a block with no valid BC7 mode bit (the PSARC-level
//! missing/garbage-content population of `docs/formats/psarc.md`'s "Block data
//! location") is refused with [`oag_texture::gnf::Error::CorruptBlocks`].
//! `crates/texture/examples/gnf_frontend_census.rs` measures the split on the
//! front end's sprite sheet.
//!
//! What the sweep asserts:
//!
//! 1. `Texture::decode` never panics on any `GNF `-valid entry in the corpus, the
//!    decode-time analogue of `omega_gnf_ground_truth.rs`'s parse-time sweep.
//! 2. Every entry's [`oag_texture::gnf::Error`] is one `decode` can raise here
//!    (`Tiled` has not triggered since every entry declares 8 or 13;
//!    `CorruptBlocks`, `UnsupportedFormat`, `DataOutOfBounds`), and a `Tiled`
//!    error names the *exact* `tile_mode` the descriptor declares.
//! 3. `data08.psarc`'s `Data/fe/` subtree (front-end images and fonts) is swept in
//!    full and isolated, so this user-visible asset class is visibly covered.

use std::path::PathBuf;

use oag_assets::psarc::Archive;
use oag_texture::gnf::{self, Texture};

const BASE_ARCHIVES: &[&str] = &[
    "data00.psarc",
    "data01.psarc",
    "data02.psarc",
    "data03.psarc",
    "data04.psarc",
];

const PATCH_ARCHIVES: &[&str] = &[
    "data05.psarc",
    "data07.psarc",
    "data08.psarc",
    "data09.psarc",
];

fn open_base(name: &str) -> Option<Archive> {
    let path: PathBuf = oag_testdata::exact(&format!("data/extracted/ps4/omega-eu/uroot/{name}"))?;
    Some(Archive::open_file(&path).unwrap_or_else(|e| panic!("open {name}: {e}")))
}

fn open_patch(name: &str) -> Option<Archive> {
    let path: PathBuf =
        oag_testdata::exact(&format!("data/extracted/ps4/omega-eu-patch/uroot/{name}"))?;
    Some(Archive::open_file(&path).unwrap_or_else(|e| panic!("open {name}: {e}")))
}

/// Runs `decode` over every `.gnf` entry `paths` names in `archive`, asserting it
/// never panics and that any error is one the contract promises
/// ([`Error::Tiled`]'s `tile_mode` checked against the descriptor's). Returns
/// `(decoded, corrupt, other_error, unparsed)`; `unparsed` is the "garbage"/
/// "all-zero" population of `docs/formats/psarc.md`, already covered for panics by
/// `omega_gnf_ground_truth.rs`, and `corrupt` is [`Error::CorruptBlocks`].
fn decode_all(
    archive: &mut Archive,
    paths: &[String],
    label: &str,
) -> (usize, usize, usize, usize) {
    let (mut decoded, mut corrupt, mut other, mut unparsed) = (0, 0, 0, 0);
    for path in paths {
        let bytes = archive
            .read_path(path)
            .unwrap_or_else(|e| panic!("{label}: {path}: read failed: {e}"));
        let Ok(texture) = Texture::parse(&bytes) else {
            unparsed += 1;
            continue;
        };
        match texture.decode(&bytes) {
            Ok(rgba) => {
                assert_eq!(
                    rgba.len(),
                    (texture.width * texture.height) as usize,
                    "{label}: {path}: wrong pixel count"
                );
                decoded += 1;
            }
            Err(gnf::Error::CorruptBlocks { .. }) => {
                corrupt += 1;
            }
            Err(gnf::Error::Tiled { tile_mode }) => {
                assert_eq!(
                    tile_mode, texture.tile_mode.0,
                    "{label}: {path}: Error::Tiled named the wrong tile_mode"
                );
                other += 1;
            }
            Err(gnf::Error::UnsupportedFormat { .. } | gnf::Error::DataOutOfBounds { .. }) => {
                other += 1;
            }
            Err(other_err) => panic!("{label}: {path}: unexpected error: {other_err}"),
        }
    }
    (decoded, corrupt, other, unparsed)
}

/// `slice` of `slices`: entry `i` of the archive's `.gnf` list belongs to slice
/// `i % slices`, so the slices cover it with nothing left over. The sweep's
/// claim is per entry, so any partition asserts the same things; the one
/// whole-archive guard (some entry reached `decode`) now holds per slice,
/// which is stricter, and the three biggest archives were 153 s, 134 s and
/// 124 s on one core under load on 2026-10-06.
fn check_archive(
    open: impl FnOnce() -> Option<Archive>,
    name: &str,
    (slice, slices): (usize, usize),
) {
    let Some(mut archive) = open() else {
        return;
    };
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
        .skip(slice)
        .step_by(slices)
        .cloned()
        .collect();
    if paths.is_empty() {
        return;
    }
    let (decoded, corrupt, other, unparsed) = decode_all(&mut archive, &paths, name);
    println!(
        "{name}: {decoded} decoded, {corrupt} refused (corrupt base level), {other} other clean errors, {unparsed} unparsed, out of {} entries",
        paths.len()
    );
    assert!(
        decoded + corrupt + other > 0,
        "{name}: not one of {} .gnf entries reached decode()",
        paths.len()
    );
}

#[test]
#[ignore]
fn data00_slice_0_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_base(BASE_ARCHIVES[0]), BASE_ARCHIVES[0], (0, 2));
}

#[test]
#[ignore]
fn data00_slice_1_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_base(BASE_ARCHIVES[0]), BASE_ARCHIVES[0], (1, 2));
}

#[test]
#[ignore]
fn data01_slice_0_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_base(BASE_ARCHIVES[1]), BASE_ARCHIVES[1], (0, 2));
}

#[test]
#[ignore]
fn data01_slice_1_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_base(BASE_ARCHIVES[1]), BASE_ARCHIVES[1], (1, 2));
}

#[test]
#[ignore]
fn data02_slice_0_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_base(BASE_ARCHIVES[2]), BASE_ARCHIVES[2], (0, 2));
}

#[test]
#[ignore]
fn data02_slice_1_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_base(BASE_ARCHIVES[2]), BASE_ARCHIVES[2], (1, 2));
}

#[test]
#[ignore]
fn data03_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_base(BASE_ARCHIVES[3]), BASE_ARCHIVES[3], (0, 1));
}

#[test]
#[ignore]
fn data04_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_base(BASE_ARCHIVES[4]), BASE_ARCHIVES[4], (0, 1));
}

#[test]
#[ignore]
fn patch_data05_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_patch(PATCH_ARCHIVES[0]), PATCH_ARCHIVES[0], (0, 1));
}

#[test]
#[ignore]
fn patch_data07_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_patch(PATCH_ARCHIVES[1]), PATCH_ARCHIVES[1], (0, 1));
}

#[test]
#[ignore]
fn patch_data09_gnf_entries_decode_or_refuse_by_name() {
    check_archive(|| open_patch(PATCH_ARCHIVES[3]), PATCH_ARCHIVES[3], (0, 1));
}

/// `data08.psarc`'s `Data/fe/` subtree (`Data/fe/images/*.gnf`,
/// `Data/fe/fonts/*.gnf`), swept in full (312 `.gnf` entries in the archive, per
/// `docs/formats/gnf.md`'s census). Asserts `decoded > 0` outright, not just
/// `decoded + corrupt + other > 0` as the whole-archive sweeps do:
/// `crates/texture/examples/gnf_frontend_census.rs` measured 219 of this subtree
/// drawing clean, so a drop to zero real pictures is what this catches.
#[test]
#[ignore]
fn data08_fe_gnf_entries_decode_or_refuse_by_name() {
    let Some(mut archive) = open_patch("data08.psarc") else {
        return;
    };
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
        .filter(|p| p.to_ascii_lowercase().contains("/fe/"))
        .cloned()
        .collect();
    assert!(
        !paths.is_empty(),
        "data08.psarc: no Data/fe/*.gnf entries at all"
    );
    let (decoded, corrupt, other, unparsed) =
        decode_all(&mut archive, &paths, "data08.psarc Data/fe/");
    println!(
        "data08.psarc Data/fe/: {decoded} decoded, {corrupt} refused (corrupt base level), {other} other clean errors, {unparsed} unparsed, out of {} entries",
        paths.len()
    );
    assert_eq!(
        decoded + corrupt + other + unparsed,
        paths.len(),
        "data08.psarc Data/fe/: not every entry reached an accounted-for outcome"
    );
    assert!(
        decoded > 0,
        "data08.psarc Data/fe/: not one real front-end image decoded"
    );
}
