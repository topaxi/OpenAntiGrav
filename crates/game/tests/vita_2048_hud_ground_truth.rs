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

fn composed(archives: &mut oag_assets::Archives, root: &str) -> oag_hud::Composed {
    let mut read = |path: &str| archives.read_name(path).ok();
    oag_hud::compose(root, &mut read).unwrap_or_else(|| panic!("composing {root}"))
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
    let oag_title::hud::Sights::Concentric {
        seeking, locked, ..
    } = oag_2048::hud::ART.sights
    else {
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

/// `oag_2048::hud::ART::sights`'s `leach` set is authored too, and no layout
/// authors a `LeachBeamSight*LockedOn*` widget - the same data-only check
/// `oag_hd::hud::ART`'s own `leach` field rests on
/// (`crates/game/tests/lock_sight_ground_truth.rs`'s
/// `no_hd_layout_authors_a_leachbeam_lockedon_widget`), since no Vita3K
/// exists here to check a frame against.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_leachbeams_four_widgets_are_authored_and_have_no_lockedon_counterpart() {
    let Some(mut archives) = open() else {
        return;
    };
    let oag_title::hud::Sights::Concentric { leach, .. } = oag_2048::hud::ART.sights else {
        panic!("oag_2048::hud::ART::sights is no longer Concentric - update this test");
    };
    let names = leach.expect("oag_2048::hud::ART's leach set is None");

    let mut all_leach: BTreeMap<String, (String, [f32; 4], [f32; 4])> = BTreeMap::new();
    for &root in oag_2048::hud::ROOTS {
        for sprite in composed(&mut archives, root).layout.sprites {
            if sprite.name.starts_with("LeachBeamSight") {
                all_leach.entry(sprite.name.clone()).or_insert((
                    sprite.src.clone(),
                    sprite.rect,
                    sprite.color,
                ));
            }
        }
    }
    for (name, (src, rect, color)) in &all_leach {
        println!("2048 {name}: src={src:?} rect={rect:?} color={color:?}");
    }

    for name in names {
        assert!(
            all_leach.contains_key(name),
            "{name}: not authored by any of the 25 composed layouts"
        );
    }
    assert!(
        !all_leach.keys().any(|n| n.contains("LockedOn")),
        "a LeachBeamSight*LockedOn* widget exists after all: {all_leach:?}"
    );
}

/// The role-name fix `oag_title::HudArt::hud_font_role` exists for: this
/// title's own language plugins resolve `oag_2048::hud::ART::hud_font_role`
/// (`"2048HUD"`) to the real, decodable HUD face - not the 5x7 fallback, and
/// not a leftover entry from a plugin that never shipped one.
///
/// Regression pin for `crates/raceplay/src/hud.rs::hud_font` reading the
/// *title's own* role through `oag_title::Title::hud_art` rather than the
/// shared `oag_ui::language::roles::HUD` literal every other title's own
/// plugin happens to also spell. Before that change, `hud_font` asked every
/// source for the literal `"HUD"`; none of 2048's seventeen plugins fill it
/// except two leftovers - `korean` and `traditionalchinese`, each naming a
/// `Data\FE\Fonts\PulseHud.fnt`/`koreanHudSmall.fnt` this title's own archive
/// does not carry - so a first-match search across every loaded plugin
/// (`korean` sorts ahead of `english` in `oag_2048::frontend::FRONT_END`'s
/// alphabetised plugin list) picked up a dangling reference and fell back to
/// 5x7 regardless of which language the player chose.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_resolved_hud_font_is_2048_huds_own_face_not_a_leftover_or_the_fallback() {
    let Some(source) = source() else {
        return;
    };
    let mut report = Vec::new();
    let mut archives = oag_2048::open(&source.display().to_string()).expect("opening 2048");
    let plugins = oag_2048::TITLE
        .front_end
        .expect("oag_2048::TITLE.front_end is None - see ADR-0054")
        .language_plugins;
    let languages =
        oag_ui::language::load::load_languages(&mut archives, plugins, None, &mut report);
    assert!(
        !languages.is_empty(),
        "no language plugin resolved: {report:?}"
    );

    let role = oag_2048::hud::ART.hud_font_role;
    assert_eq!(role, "2048HUD", "oag_2048::hud::ART::hud_font_role moved");
    let name = languages
        .iter()
        .find_map(|language| language.font(role))
        .unwrap_or_else(|| panic!("no plugin fills the {role:?} role"))
        .to_string();
    assert_eq!(
        name, r"Data\XML\2048_hud\font\2048_hud.fnt",
        "resolved {role:?} to {name}, not this title's own HUD face"
    );
    let font = archives
        .read_font(&name)
        .unwrap_or_else(|e| panic!("{name} named but unreadable: {e}"));
    assert!(!font.glyphs.is_empty(), "{name} decoded to no glyphs");

    // The regression this axis closes: the shared literal every other
    // title's own plugin happens to also name is *not* this title's answer -
    // either no plugin fills it (a source whose plugins genuinely agree with
    // the other titles would fail this test, which is the point) or, as
    // measured here, a leftover entry a first-match search across every
    // loaded plugin could pick up ahead of the one the player chose.
    let shared_literal = oag_ui::language::roles::HUD;
    if let Some(leftover) = languages
        .iter()
        .find_map(|language| language.font(shared_literal))
    {
        assert_ne!(
            leftover, name,
            "{shared_literal:?} now resolves to the same file as {role:?} - \
             this test's premise (that they diverge) no longer holds"
        );
        assert!(
            archives.read_font(leftover).is_err(),
            "{shared_literal:?} resolved to {leftover}, which decodes - the \
             old first-match search would have drawn a real font rather than \
             exposing the mismatch this axis exists to fix"
        );
    }
}

/// **Measured, not merely chosen**: the skin this title actually plays
/// (`oag_2048::hud::skins::PLAYED`) never authors a `font="HUDSmall"` widget
/// anywhere across its seven roots, so `oag_title::HudArt::hud_small_font_role`
/// being `None` for 2048 costs nothing real - no widget the played HUD draws
/// will ever ask this build to resolve that role in the first place.
///
/// This is the check `oag_title::HudArt::hud_small_font_role`'s own doc
/// promises: the visible caption/value size split
/// (`LapTxt` at `scale=0.6` beside `Laps` at `scale=1.0`, both `font="HUD"`)
/// comes entirely from each widget's own authored `scale`, on a single font
/// file - not from a second, unresolved `.fnt` this build is failing to
/// find. `font="HUDSmall"` does exist in this archive, 261 widgets' worth,
/// but only in the three *unplayed* skins (`wo3_hud`, `2097_hud`, the bare
/// root) this title's own race-manager constructors never read - see
/// `oag_2048::hud::skins`' own doc comment for which skin is which.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_played_skins_layouts_author_no_hudsmall_widget_at_all() {
    let Some(mut archives) = open() else {
        return;
    };
    use oag_2048::hud::skins::played;
    let played_roots = [
        played::ARCADE,
        played::ELIMINATION,
        played::MP_TAG,
        played::SPEED_LAP,
        played::TIME_TRIAL,
        played::ZOMBIE,
        played::ZONE,
    ];
    let mut small_widgets = Vec::new();
    for root in played_roots {
        for label in composed(&mut archives, root).layout.labels {
            if label.font == oag_hud::Font::Small {
                small_widgets.push(format!("{root}: {}", label.name));
            }
        }
    }
    assert!(
        small_widgets.is_empty(),
        "the played skin does author font=\"HUDSmall\" after all: {small_widgets:?} - \
         oag_title::HudArt::hud_small_font_role's \"real gap\" doc comment needs \
         correcting, and hud_font's None fallback may now be visibly wrong"
    );
}
