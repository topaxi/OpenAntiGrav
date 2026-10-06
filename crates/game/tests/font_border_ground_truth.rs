//! `borderExtendPixels` per font role, off every title's own language
//! definitions, and what an extended glyph quad samples.
//!
//! **`#[ignore]`d and never run in CI**: it needs game content.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(font_border_ground_truth)'
//! ```
//!
//! The census half answers the question an extended quad raises: how much of
//! the authored border can a glyph draw before its quad reaches a neighbour?
//! `oag_ui::font::Atlas::with_border_extend` cuts each glyph to half its gap,
//! and the HUD faces on every title are asserted to get all of it.

use std::path::{Path, PathBuf};

use oag_ui::language::{Language, load::load_languages};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn present(rel: &str) -> Option<PathBuf> {
    let path = root().join(rel);
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

/// The English plugin's slots that author a border, as `(role, px)`.
fn authored(languages: &[Language]) -> Vec<(String, u32)> {
    languages
        .iter()
        .find(|l| l.name == "English")
        .map(|l| l.font_borders.clone())
        .unwrap_or_default()
}

/// `(glyphs, smallest reach, largest reach)` the atlas gives `font` at `border`.
fn reach(font: &oag_texture::fnt::Font, border: u32) -> (usize, u32, u32) {
    let atlas = oag_ui::font::Atlas::from_font(font).with_border_extend(border);
    let reaches: Vec<u32> = font
        .glyphs
        .iter()
        .filter(|g| g.width > 0 && g.height > 0)
        .filter_map(|g| char::from_u32(u32::from(g.codepoint)))
        .filter_map(|c| atlas.cell(c))
        .map(|cell| cell.extend)
        .collect();
    (
        reaches.len(),
        reaches.iter().copied().min().unwrap_or(0),
        reaches.iter().copied().max().unwrap_or(0),
    )
}

fn census(label: &str, archives: &mut oag_assets::Archives, languages: &[Language]) {
    let english = languages.iter().find(|l| l.name == "English");
    for (role, px) in authored(languages) {
        let Some(name) = english.and_then(|l| l.font(&role)) else {
            continue;
        };
        let Ok(font) = archives.read_font(name) else {
            println!("{label}: {role} -> {name} unreadable");
            continue;
        };
        let (n, lo, hi) = reach(&font, px);
        println!(
            "CENSUS {label}: {role:<12} authored {px:>2}  {name}  {n} glyphs, drawn reach {lo}..{hi}"
        );
        if role == "HUD" || role == "HUDSmall" {
            assert_eq!(
                lo, px,
                "{label}: {role} is packed too tight for its authored border"
            );
        }
    }
}

fn open_image(rel: &str) -> Option<(oag_assets::Archives, Vec<Language>)> {
    let path = present(rel)?;
    let mut report = Vec::new();
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let plugins = opened
        .title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    let mut archives = opened.archives;
    let languages = load_languages(&mut archives, plugins, None, &mut report);
    Some((archives, languages))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_psp_authors_five_for_the_hud_and_three_for_the_captions() {
    for rel in [
        "data/images/pulse-psp-eu.chd",
        "data/images/pulse-psp-usa.chd",
        "data/images/pulse-ps2-eu.chd",
    ] {
        let Some((mut archives, languages)) = open_image(rel) else {
            continue;
        };
        for language in &languages {
            if language.fonts.is_empty() {
                continue;
            }
            assert_eq!(language.border_extend("HUD"), 5, "{rel} {}", language.name);
            assert_eq!(
                language.border_extend("HUDSmall"),
                3,
                "{rel} {}",
                language.name
            );
        }
        census(rel, &mut archives, &languages);
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_other_titles_border_census() {
    for rel in [
        "data/images/pure-psp-eu.chd",
        "data/images/hdfury-ps3-eu-dec.iso",
    ] {
        if let Some((mut archives, languages)) = open_image(rel) {
            census(rel, &mut archives, &languages);
        }
    }
    if let Some(path) = present("data/extracted/vita/PCSF00007/base") {
        let mut archives = oag_2048::open(&path.display().to_string()).expect("2048");
        let plugins = oag_2048::TITLE
            .front_end
            .expect("front end")
            .language_plugins;
        let languages = load_languages(&mut archives, plugins, None, &mut Vec::new());
        census("2048", &mut archives, &languages);
    }
    if let Some(path) = present("data/extracted/ps4") {
        let mut archives = oag_omega::open(&path.display().to_string()).expect("omega");
        let plugins = oag_omega::TITLE
            .front_end
            .expect("front end")
            .language_plugins;
        let languages = load_languages(&mut archives, plugins, None, &mut Vec::new());
        census("omega", &mut archives, &languages);
    }
}
