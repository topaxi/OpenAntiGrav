//! Composes Wipeout 2048's own 25 HUD layouts and checks what came out.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted Vita package
//! extracted with `oag-unpack`, which this project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md` for how
//! `data/extracted/vita/PCSF00007` gets populated.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(vita_2048_hud_ground_truth)'
//! ```
//!
//! # This file stops at the layout, not the pixels
//!
//! `crates/game/tests/hd_hud_ground_truth.rs` has a check
//! (`every_sprite_s_source_rectangle_fits_inside_the_texture_it_names`) that
//! decodes every texture a layout names and checks the composed sprite
//! rectangles fit inside it. `oag_texture::gxt` can do the equivalent now -
//! see `crates/formats/tests/gxt_ground_truth.rs`, which decodes real HUD
//! textures and renders them - but composing this title's rectangles against
//! decoded `.gxt` dimensions is not written here yet. This file stops at
//! "every reference resolves to a shipped entry", which is as far as
//! [`oag_2048::hud::texture_entry`]'s own evidence goes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base");

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

fn open() -> Option<oag_assets::Archives> {
    let source = source()?;
    oag_2048::open(&source.display().to_string()).ok()
}

fn composed(archives: &mut oag_assets::Archives, root: &str) -> oag_game::hud::Composed {
    let mut read = |path: &str| archives.read_name(path).ok();
    oag_game::hud::compose(root, &mut read).unwrap_or_else(|| panic!("composing {root}"))
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_package_ships_the_25_roots_the_title_carries_plus_one_split_screen() {
    let Some(source) = source() else {
        return;
    };
    let psarc_path = source.join("PSP2/data.psarc");
    let archive = oag_assets::psarc::Archive::open_file(&psarc_path)
        .unwrap_or_else(|e| panic!("opening {}: {e}", psarc_path.display()));

    let mut found: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with("_hud.xml"))
        .cloned()
        .collect();
    found.sort();

    assert_eq!(found.len(), 26, "shipped *_HUD.xml documents: {found:?}");

    let split_screen: Vec<&String> = found
        .iter()
        .filter(|p| p.to_ascii_lowercase().contains("splitscreen"))
        .collect();
    assert_eq!(
        split_screen,
        vec!["data/xml/SplitScreenZone_hud/Zone_HUD.xml"],
        "expected exactly the one split-screen Zone HUD, left out of ROOTS"
    );

    let mut named: Vec<String> = oag_2048::hud::ROOTS
        .iter()
        .map(|p| p.replace('\\', "/").to_ascii_lowercase())
        .collect();
    named.sort();
    let mut shipped_non_split: Vec<String> = found
        .iter()
        .filter(|p| !p.to_ascii_lowercase().contains("splitscreen"))
        .map(|p| p.to_ascii_lowercase())
        .collect();
    shipped_non_split.sort();
    assert_eq!(
        named, shipped_non_split,
        "oag_2048::hud::ROOTS and the archive's own manifest disagree"
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_root_composes_with_nothing_missing_and_nothing_skipped() {
    let Some(mut archives) = open() else {
        return;
    };
    for &root in oag_2048::hud::ROOTS {
        let out = composed(&mut archives, root);
        assert!(
            out.missing.is_empty(),
            "{root}: includes that did not resolve: {:?}",
            out.missing
        );
        assert!(
            out.layout.skipped.is_empty(),
            "{root}: widgets the parser did not model: {:?}",
            out.layout.skipped
        );
    }
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_texture_reference_resolves_once_the_extension_rule_applies() {
    let Some(mut archives) = open() else {
        return;
    };
    let mut uses: BTreeMap<String, usize> = BTreeMap::new();
    for &root in oag_2048::hud::ROOTS {
        for sprite in composed(&mut archives, root).layout.sprites {
            *uses.entry(sprite.src).or_default() += 1;
        }
    }

    assert_eq!(uses.len(), 19, "distinct texture references: {uses:?}");

    for (reference, count) in &uses {
        let entry = oag_2048::hud::texture_entry(reference);
        assert!(
            archives.locate(&entry).is_some(),
            "{reference} ({count} sprites) resolves to {entry}, which is not on the disc"
        );
    }
}

/// Every layout that authors a reticle at all names it as `<Image>` sprites,
/// never `<Mode3D><Model>` widgets - the evidence
/// `oag_2048::hud::ART::sights` rests on.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn no_layout_authors_a_mode3d_reticle() {
    let Some(mut archives) = open() else {
        return;
    };
    for &root in oag_2048::hud::ROOTS {
        let out = composed(&mut archives, root);
        assert!(
            out.layout.models.is_empty(),
            "{root}: unexpected Mode3D models: {:?}",
            out.layout.models
        );
    }
}

/// Every `MissileSight*`/`LeachBeamSight*` widget `oag_2048::hud::ART::sights`
/// names is authored by at least one layout, on the same terms
/// `hd_hud_ground_truth.rs` checks `oag_hd::hud::ALWAYS_ON` against.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_named_sight_widget_is_authored_by_at_least_one_layout() {
    let Some(mut archives) = open() else {
        return;
    };
    let oag_title::hud::Sights::Concentric { seeking, locked } = oag_2048::hud::ART.sights else {
        panic!("oag_2048::hud::ART::sights is no longer Concentric - update this test");
    };

    let mut authored: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for &root in oag_2048::hud::ROOTS {
        for sprite in composed(&mut archives, root).layout.sprites {
            authored.insert(sprite.name);
        }
    }

    for name in seeking.iter().chain(*locked) {
        assert!(
            authored.contains(*name),
            "{name}: not authored by any of the 25 composed layouts"
        );
    }
}
