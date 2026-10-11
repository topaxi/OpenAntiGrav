//! Test scaffolding for the disc-backed ground truths: finds the player's Unity
//! content and hands a reader the bytes of a bundle.
//!
//! Not part of the library. A `.wtrk` wraps a UnityFS bundle at an offset; the
//! wrapper is a different layer's business, so the test just finds the
//! signature and passes everything from there on.

#![allow(dead_code)]

use std::path::PathBuf;

/// The Workshop items directory, or `None` (skipping, or failing under
/// `OAG_REQUIRE_GAME_DATA`).
pub fn workshop() -> Option<PathBuf> {
    oag_testdata::exact("data/unity/workshop-473770")
}

/// The game install directory.
pub fn install() -> Option<PathBuf> {
    oag_testdata::exact("data/unity/ballisticng")
}

/// One item file of the Workshop census.
pub struct Item {
    pub id: String,
    pub path: PathBuf,
    pub ext: String,
}

/// Every file of every Workshop item, sorted by id.
pub fn items(root: &std::path::Path) -> Vec<Item> {
    let mut out = Vec::new();
    let mut dirs: Vec<_> = std::fs::read_dir(root)
        .expect("workshop dir")
        .map(|e| e.expect("entry").path())
        .collect();
    dirs.sort();
    for d in dirs {
        for e in std::fs::read_dir(&d).expect("item dir") {
            let path = e.expect("entry").path();
            let ext = path
                .extension()
                .map(|x| x.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            out.push(Item {
                id: d.file_name().unwrap().to_string_lossy().into_owned(),
                path,
                ext,
            });
        }
    }
    out
}

/// The bytes of an item from the `UnityFS` signature on, or `None` when the
/// file holds no bundle (the legacy `.shp`).
pub fn bundle_bytes(path: &std::path::Path) -> Option<Vec<u8>> {
    let all = std::fs::read(path).expect("read item");
    let at = all.windows(8).position(|w| w == b"UnityFS\0")?;
    Some(all[at..].to_vec())
}
