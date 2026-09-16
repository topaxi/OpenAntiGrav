//! Byte coverage of every `.gxt` in Wipeout 2048's base package.
//!
//! **`#[ignore]`d and never run in CI.** Needs the decrypted Vita package
//! under `data/extracted/vita/PCSF00007`, which `gxt_ground_truth.rs`'s own
//! module doc explains how to get.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-texture --run-ignored all \
//!     -E 'binary(gxt_coverage_ground_truth)'
//! ```
//!
//! # Why this is expected to close exactly
//!
//! `Gxt::parse` already checks `header_offset == descriptors_end` and
//! `header_offset + header_size == data.len()` before it accepts a file (see
//! its own doc comment), so a `.gxt` this crate parses at all has no room for
//! a gap between the header, the descriptor table and the declared texel
//! span - `oag_texture::gxt::coverage` exists to make that fact checkable
//! from a sweep rather than only from `Gxt::parse`'s own internal checks, the
//! same role `oag_rcs::rcsmodel::coverage` plays for a format that is *not*
//! closed this tightly.

use std::path::{Path, PathBuf};

use oag_texture::gxt;

fn package() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_shipped_gxt_closes_exactly() {
    let Some(path) = package() else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut entries: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gxt"))
        .cloned()
        .collect();
    entries.sort();

    let (mut files, mut total, mut claimed) = (0usize, 0u64, 0u64);
    let mut worst: Option<(usize, String)> = None;
    for entry in &entries {
        let blob = archive
            .read_path(entry)
            .unwrap_or_else(|e| panic!("reading {entry}: {e}"));
        let seen = gxt::coverage(&blob);
        if seen.is_empty() {
            continue;
        }
        files += 1;
        total += seen.len() as u64;
        claimed += seen.claimed() as u64;
        let gap_len: usize = seen.gaps(1).iter().map(|g| g.len).sum();
        if gap_len > 0 && worst.as_ref().is_none_or(|(n, _)| gap_len > *n) {
            worst = Some((gap_len, seen.describe(1)));
        }
    }
    let pct = 100.0 * claimed as f64 / total.max(1) as f64;
    println!("gxt: {files} file(s), {claimed} of {total} byte(s) ({pct:.2}%)");
    if let Some((_, describe)) = &worst {
        println!("  worst: {describe}");
    }
    assert_eq!(files, 9910, "the corpus gxt_ground_truth.rs already pins");
    assert_eq!(
        claimed, total,
        "expected to close exactly, see the module doc"
    );
}
