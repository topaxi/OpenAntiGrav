//! Which `.fnt` each font *role* resolves to, on every disc present.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(font_roles_ground_truth)'
//! ```
//!
//! # What this is for
//!
//! `oag_game::boot::load_font` used to name `Data\FE\Fonts\pulse_text.fnt`
//! outright. That is exactly what Pulse's own `PI008` resolves the `Default`
//! role to, so it was right on Pulse, right on the PS2 port, and drew the
//! *whole* of Pure's front end in the 5x7 fallback - Pure resolves the same role
//! to `FX300ANG.fnt`. Of the eight `<Font>` slots each title fills in, the two
//! discs have exactly one filename in common (`small.fnt`, and not for the body
//! face). A hardcoded name that happens to match one disc is indistinguishable
//! from a resolved one until a second disc turns up, which is what this file is:
//! the resolution run over every image present, asserting the titles disagree.
//!
//! The assertions are deliberately **not** a table of filenames. Spelling Pure's
//! `.fnt` names here would be the same mistake one layer up - and shipped
//! content in the repository besides. What is asserted is the *shape*: every
//! disc names a `Default` face, that face loads, and the two titles' answers are
//! not the same string.

use std::path::{Path, PathBuf};

use oag_game::language::{Language, roles};

/// Which title an image is, for the tests that compare two of them.
///
/// A three-way enum rather than an `is_pure` flag because Wipeout HD is now in
/// the list and is neither: it resolves its own eight slots off its own plugins,
/// and the Pulse-against-Pure comparisons below are about those two discs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Pulse,
    Pure,
    Hd,
}

/// Every image present, as `(label, path, family)`.
///
/// A missing image is skipped rather than failed, the way the other ground-truth
/// files here do it, unless `OAG_REQUIRE_GAME_DATA` says the caller expects them.
fn images() -> Vec<(&'static str, PathBuf, Family)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for (label, name, family) in [
        (
            "pulse-psp-eu",
            "data/images/pulse-psp-eu.chd",
            Family::Pulse,
        ),
        (
            "pulse-psp-usa",
            "data/images/pulse-psp-usa.chd",
            Family::Pulse,
        ),
        (
            "pulse-ps2-eu",
            "data/images/pulse-ps2-eu.chd",
            Family::Pulse,
        ),
        ("pure-psp-eu", "data/images/pure-psp-eu.chd", Family::Pure),
        ("pure-psp-usa", "data/images/pure-psp-usa.chd", Family::Pure),
        // The decrypted PS3 image, and it only earns a place here now that
        // `oag_formats::fnt` reads a big-endian header: before that every one of
        // HD's 33 fonts failed the magic check and this would have asserted the
        // fallback rather than the face.
        (
            "hdfury-ps3-eu",
            "data/images/hdfury-ps3-eu-dec.iso",
            Family::Hd,
        ),
    ] {
        let path = root.join(name);
        if path.exists() {
            found.push((label, path, family));
        } else {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            println!("skipping: {} not present", path.display());
        }
    }
    found
}

/// The language plugins one source carries, and its archives.
fn languages_of(image: &Path) -> (oag_assets::Archives, Vec<Language>) {
    let mut report = Vec::new();
    let opened = oag_game::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
        .expect("opening the source");
    // The plugin list is the title's own - numbered on the PSP discs, named on
    // Wipeout HD - so it is taken from the source that was just identified
    // rather than from a constant this test would have to keep in step.
    let plugins = opened
        .title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    let mut archives = opened.archives;
    let languages = oag_game::boot::load_languages(&mut archives, plugins, &mut report);
    (archives, languages)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_disc_names_a_body_face_and_it_loads() {
    for (label, image, _) in images() {
        let (mut archives, languages) = languages_of(&image);
        assert!(
            !languages.is_empty(),
            "{label}: no language plugin resolved, so no font role can"
        );
        let name = languages
            .iter()
            .find_map(|language| language.font(roles::DEFAULT))
            .unwrap_or_else(|| panic!("{label}: no plugin fills in the {:?} slot", roles::DEFAULT))
            .to_string();
        let font = archives
            .read_font(&name)
            .unwrap_or_else(|e| panic!("{label}: {name} named but unreadable: {e}"));
        // Not a glyph count: the discs differ there, and the point is only that
        // the resolved name is a real, decodable face rather than a string that
        // happened to hash to something.
        assert!(
            !font.glyphs.is_empty(),
            "{label}: {name} decoded to no glyphs"
        );
        assert!(font.line_height > 0, "{label}: {name} has no line height");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_two_titles_do_not_name_the_same_body_face() {
    // The regression guard for the hardcode this file exists for. If a change
    // ever puts one title's filename back in the code, both sides of this go to
    // the same string and it fails naming both.
    let found = images();
    let face = |want: Family| -> Option<(String, String)> {
        found.iter().find(|(_, _, f)| *f == want).map(|(l, i, _)| {
            let (_, languages) = languages_of(i);
            let name = languages
                .iter()
                .find_map(|language| language.font(roles::DEFAULT))
                .unwrap_or_else(|| panic!("{l}: no {:?} slot", roles::DEFAULT))
                .to_string();
            ((*l).to_string(), name)
        })
    };
    let (Some((pulse_label, pulse_face)), Some((pure_label, pure_face))) =
        (face(Family::Pulse), face(Family::Pure))
    else {
        println!("skipping: needs one Pulse image and one Pure image");
        return;
    };
    assert_ne!(
        pulse_face,
        pure_face,
        "{pulse_label} and {pure_label} resolve {:?} to the same file, which is what \
         a hardcoded filename would also produce - re-check `boot::load_font`",
        roles::DEFAULT
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_two_titles_fill_the_same_slots_but_for_menu_against_scroll() {
    // The measurement `oag_pure::frontend::MENU_SKIN`'s `menu_font: None` rests
    // on. **An earlier pass got this wrong in the cheap direction**: reading one
    // plugin through a truncating pager showed Pure filling four slots to Pulse's
    // eight, and the conclusion drawn - "Pure declares half as many" - was an
    // artefact of the reading. Both titles fill in eight. The real difference is
    // one slot: Pulse has `Menu`, Pure has `Scroll`, and neither has the other's.
    //
    // Asserted as set arithmetic rather than as a list of role names, so it stays
    // a statement about the *difference* the code depends on.
    let found = images();
    let slots = |want: Family| -> Option<Vec<String>> {
        found.iter().find(|(_, _, f)| *f == want).map(|(_, i, _)| {
            let (_, languages) = languages_of(i);
            let mut names: Vec<String> = languages
                .iter()
                .flat_map(|l| l.fonts.iter().map(|(role, _)| role.to_ascii_lowercase()))
                .collect();
            names.sort();
            names.dedup();
            names
        })
    };
    let (Some(pulse_slots), Some(pure_slots)) = (slots(Family::Pulse), slots(Family::Pure)) else {
        println!("skipping: needs one Pulse image and one Pure image");
        return;
    };
    let only_pulse: Vec<_> = pulse_slots
        .iter()
        .filter(|role| !pure_slots.contains(role))
        .collect();
    let only_pure: Vec<_> = pure_slots
        .iter()
        .filter(|role| !pulse_slots.contains(role))
        .collect();
    assert_eq!(
        only_pulse,
        ["menu"],
        "the one slot Pulse fills and Pure does not; got Pulse {pulse_slots:?} \
         against Pure {pure_slots:?}"
    );
    assert_eq!(
        only_pure,
        ["scroll"],
        "and the one Pure fills and Pulse does not; got Pulse {pulse_slots:?} \
         against Pure {pure_slots:?}"
    );
    assert!(
        !pure_slots.iter().any(|role| role == "menu"),
        "restated as the thing the code reads: Pure names no Menu role, which is \
         why its MenuSkin::menu_font is None"
    );
}
