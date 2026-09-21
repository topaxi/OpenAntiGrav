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
//! **`decode` untiles nothing - see `docs/formats/gnf.md`'s tiling
//! section for why.** Every valid `.gnf` `docs/formats/gnf.md`'s census
//! found (1,407 across all nine archives) declares `TileMode(13)`
//! (`Thin_1DThin` - GFD-Studio's own `TileMode.cs` enum), micro-tiled, so
//! every one of them is expected to fail `decode` with
//! [`oag_texture::gnf::Error::Tiled`] - naming that outright rather than a
//! floor on how many decode successfully is what keeps this test
//! meaningful (and failing loudly) the day someone lands the untiler.
//!
//! What this corpus sweep actually is a fact about:
//!
//! 1. `Texture::decode` never panics on any real, `GNF `-valid entry across
//!    the whole corpus - a decode-time analogue of
//!    `omega_gnf_ground_truth.rs`'s own parse-time sweep.
//! 2. Every entry's [`oag_texture::gnf::Error`] is one of the three
//!    `decode` can raise, and a tiled entry's [`Error::Tiled`] names the
//!    *exact* `tile_mode` byte the descriptor itself declares - not just
//!    "an error happened".
//! 3. `data08.psarc`'s `Data/fe/` subtree specifically (the front-end
//!    images and fonts) - named in the task brief as the set to sweep in
//!    full - gets the same treatment, isolated from the rest so a reader
//!    can see this specific, user-visible asset class is covered.

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

/// Runs `decode` over every `.gnf` entry `paths` names in `archive`,
/// asserting it never panics and, when it errs, that the error is one this
/// module's own contract promises - [`Error::Tiled`]'s own `tile_mode`
/// checked against the descriptor's, not just "an error came back".
/// Returns `(decoded, tiled, other_error, unparsed)` counts - `unparsed` is
/// the still-open "garbage"/"all-zero" population `docs/formats/psarc.md`
/// measures, already covered for panics by `omega_gnf_ground_truth.rs`, not
/// this function's own concern beyond counting it.
fn decode_all(
    archive: &mut Archive,
    paths: &[String],
    label: &str,
) -> (usize, usize, usize, usize) {
    let (mut decoded, mut tiled, mut other, mut unparsed) = (0, 0, 0, 0);
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
            Err(gnf::Error::Tiled { tile_mode }) => {
                assert_eq!(
                    tile_mode, texture.tile_mode.0,
                    "{label}: {path}: Error::Tiled named the wrong tile_mode"
                );
                tiled += 1;
            }
            Err(gnf::Error::UnsupportedFormat { .. } | gnf::Error::DataOutOfBounds { .. }) => {
                other += 1;
            }
            Err(other_err) => panic!("{label}: {path}: unexpected error: {other_err}"),
        }
    }
    (decoded, tiled, other, unparsed)
}

fn check_archive(open: impl FnOnce() -> Option<Archive>, name: &str) {
    let Some(mut archive) = open() else {
        return;
    };
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
        .cloned()
        .collect();
    if paths.is_empty() {
        return;
    }
    let (decoded, tiled, other, unparsed) = decode_all(&mut archive, &paths, name);
    println!(
        "{name}: {decoded} decoded, {tiled} tiled (refused by name), {other} other clean errors, {unparsed} unparsed, out of {} entries",
        paths.len()
    );
    assert!(
        decoded + tiled + other > 0,
        "{name}: not one of {} .gnf entries reached decode()",
        paths.len()
    );
}

#[test]
#[ignore]
fn data00_gnf_entries_decode_or_are_named_tiled() {
    check_archive(|| open_base(BASE_ARCHIVES[0]), BASE_ARCHIVES[0]);
}

#[test]
#[ignore]
fn data01_gnf_entries_decode_or_are_named_tiled() {
    check_archive(|| open_base(BASE_ARCHIVES[1]), BASE_ARCHIVES[1]);
}

#[test]
#[ignore]
fn data02_gnf_entries_decode_or_are_named_tiled() {
    check_archive(|| open_base(BASE_ARCHIVES[2]), BASE_ARCHIVES[2]);
}

#[test]
#[ignore]
fn data03_gnf_entries_decode_or_are_named_tiled() {
    check_archive(|| open_base(BASE_ARCHIVES[3]), BASE_ARCHIVES[3]);
}

#[test]
#[ignore]
fn data04_gnf_entries_decode_or_are_named_tiled() {
    check_archive(|| open_base(BASE_ARCHIVES[4]), BASE_ARCHIVES[4]);
}

#[test]
#[ignore]
fn patch_data05_gnf_entries_decode_or_are_named_tiled() {
    check_archive(|| open_patch(PATCH_ARCHIVES[0]), PATCH_ARCHIVES[0]);
}

#[test]
#[ignore]
fn patch_data07_gnf_entries_decode_or_are_named_tiled() {
    check_archive(|| open_patch(PATCH_ARCHIVES[1]), PATCH_ARCHIVES[1]);
}

#[test]
#[ignore]
fn patch_data09_gnf_entries_decode_or_are_named_tiled() {
    check_archive(|| open_patch(PATCH_ARCHIVES[3]), PATCH_ARCHIVES[3]);
}

/// `data08.psarc`'s `Data/fe/` subtree specifically - the front-end images
/// and fonts the task brief named directly (`Data/fe/images/*.gnf`,
/// `Data/fe/fonts/*.gnf`) - swept in full rather than sampled, since it is
/// small enough to (312 `.gnf` entries in the whole archive, per
/// `docs/formats/gnf.md`'s census) and is exactly the asset class a reader
/// would check first.
#[test]
#[ignore]
fn data08_fe_gnf_entries_decode_or_are_named_tiled() {
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
    let (decoded, tiled, other, unparsed) =
        decode_all(&mut archive, &paths, "data08.psarc Data/fe/");
    println!(
        "data08.psarc Data/fe/: {decoded} decoded, {tiled} tiled (refused by name), {other} other clean errors, {unparsed} unparsed, out of {} entries",
        paths.len()
    );
    assert_eq!(
        decoded + tiled + other + unparsed,
        paths.len(),
        "data08.psarc Data/fe/: not every entry reached an accounted-for outcome"
    );
    assert!(
        decoded + tiled + other > 0,
        "data08.psarc Data/fe/: not one of {} entries reached decode()",
        paths.len()
    );
}
