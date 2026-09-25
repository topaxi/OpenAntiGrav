//! `oag_ui::screen::TagInput`, against `Data.wad` entry hash
//! `oag_pulse::hashes::TAG_INPUT_SCREENS` on the real disc.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! # Why `Create Profile Setup`, not `Name`/`Tag`
//!
//! The entry holds three `<TagInput>`s. `Name` and the first `Tag` (the
//! profile-entry screens - `length="10"`/`length="3"`) sit directly under an
//! **anonymous** `<Screen type="FE_Default">`, which `Screens::collect`
//! deliberately walks through without registering as a screen of its own -
//! see that function's own doc comment. `Create Profile Setup`'s own `Tag`
//! is the one instance under a *named* `Screen`, so it is the one this
//! project's public `Screens` API can reach, and the one this test checks.
//! `docs/formats/fexml.md`'s `TagInput` section records the other two
//! instances' own numbers, read the same way `oag_ui::tag_entry` reads them:
//! by walking `fexml::parse`'s tree directly rather than through `Screens`.
//!
//! # Why this also loads `Skin.xml`
//!
//! `Create Profile Setup`'s `TagInput` authors `x="FEGlobals->TitleXOffset"`
//! and `color="FEGlobals->TextColor"` - declared in the front-end root, not
//! in this entry. Parsing the entry alone resolves neither: `x` silently
//! becomes `0.0` and `color` falls back to opaque white, both wrong and both
//! looking like a passing test if this file only asserted its own read
//! back. This loads the real `Skin.xml` first and merges its globals in,
//! the same way `oag_ui::picker::slideshow` and every other consumer of a
//! screen file that references `FEGlobals->` has to.

use oag_tables::fexml;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("pulse-psp-usa.chd")
}

fn screens() -> Option<oag_ui::screen::Screens> {
    let path = image()?;
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");

    let skin_raw = archives
        .read_name(oag_pulse::names::FRONTEND_ROOT)
        .expect("every pressing carries the front-end root");
    let skin_xml = fexml::text(&skin_raw).expect("it is text");
    let skin_globals = oag_ui::screen::Screens::from_xml(&skin_xml).globals;

    let raw = archives
        .read_hash(oag_pulse::hashes::TAG_INPUT_SCREENS)
        .expect("the TagInput screens entry is on this disc");
    let xml = fexml::text(&raw).expect("it is text");
    let fallback: Vec<(&str, &str)> = skin_globals
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    Some(oag_ui::screen::Screens::from_xml_with_fallback_globals(
        &xml, &fallback,
    ))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn create_profile_setups_tag_resolves_off_the_real_disc() {
    let Some(screens) = screens() else { return };

    let screen = screens
        .by_name("Create Profile Setup")
        .expect("the entry names this screen");
    assert_eq!(screen.tag_inputs.len(), 1, "exactly one TagInput on it");

    let tag = &screen.tag_inputs[0];
    assert_eq!(tag.name, "Tag");
    assert_eq!(tag.length, 3, "a profile tag is three characters");
    assert_eq!(tag.y, 85.0);
    // Resolved through Skin.xml's own globals, not the defaults a missing
    // lookup would fall back to.
    assert_eq!(
        tag.x, 50.0,
        "FEGlobals->TitleXOffset, off the real Skin.xml"
    );
    assert_eq!(
        tag.color, 0xFF33_A6B9,
        "FEGlobals->TextColor, off the real Skin.xml"
    );
    // No `scale` attribute on this instance: `1.0` is the parser's own
    // default, not a number read off this widget.
    assert_eq!(tag.scale, 1.0);
    assert!(!tag.encrypt);
    assert_eq!(tag.allow_blank, None);
    assert_eq!(tag.focus.as_deref(), Some("true"));
}

/// The same entry hash resolves on the EU disc too, at a different index -
/// see `oag_pulse::hashes::TAG_INPUT_SCREENS`'s own doc for why this is
/// worth its own assertion rather than trusting the USA read.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_entry_hash_also_resolves_on_the_eu_disc() {
    let Some(path) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let raw = archives
        .read_hash(oag_pulse::hashes::TAG_INPUT_SCREENS)
        .expect("the same hash resolves on the EU pressing too");
    assert_eq!(raw.len(), 40_083);
}
