//! The footer's scrolling tip ticker's own layout against the disc it is
//! read off - both PSP pressings, EU preferred where only one is present.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! # Why this is a test of its own
//!
//! `oag_ui_screens::campaign::footer::TickerLayout::read` already had a fixture-based
//! unit test (`crates/ui-screens/src/campaign/footer/tests.rs`) against a miniature
//! hand-written `Skin.xml`, and `oag_game::campaign::load`'s own
//! `read_footer` reads the ticker for the Race Campaign's screens - but
//! nothing before this pinned the *real* disc's own `TextInfoIsAlwaysLast`
//! viewport, the one the ordinary menu pages' own footer now reads too
//! (`oag_game::boot::screens::read_ticker`, `crate::main::menu_stage::footer`
//! in the binary). `docs/ui/campaign-screens.md`'s 2026-09-21 pass measured
//! `OffsetX="85" OffsetY="235" width="370" height="32"` off `pulse-psp-usa`;
//! this is the test that keeps that number honest against both pressings
//! rather than only the one that was open at the time.
use std::path::PathBuf;

use oag_tables::fexml;
use oag_ui::screen::{Screens, parse};
use oag_ui_screens::campaign::footer::TickerLayout;

/// `(label, image)` for each Pulse PSP pressing present - EU first, per this
/// project's own default preference between the two when both are on disk.
fn images() -> Vec<(&'static str, PathBuf)> {
    [
        ("pulse-psp-eu", "data/images/pulse-psp-eu.chd"),
        ("pulse-psp-usa", "data/images/pulse-psp-usa.chd"),
    ]
    .into_iter()
    .filter_map(|(label, name)| oag_testdata::image(name).map(|path| (label, path)))
    .collect()
}

/// The front-end root's own text, off `archives` - the same
/// `fexml::text`-over-`read_name` step `pulse_frame_ground_truth.rs` and
/// `oag_game::campaign::read_footer` both take.
fn front_end_root_xml(archives: &mut oag_assets::Archives) -> String {
    let raw = archives
        .read_name(oag_pulse::names::FRONTEND_ROOT)
        .expect("Pulse carries a front-end root");
    fexml::text(&raw).expect("it is text")
}

/// **The ticker's own viewport, measured off the real disc, on every Pulse
/// pressing present.** A missing `data/images/` skips honestly (see
/// `CLAUDE.md`'s own note on `data/` in a sandbox); `OAG_REQUIRE_GAME_DATA=1`
/// turns that into a failure instead, the same contract every other
/// ground-truth test here follows.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_footer_authors_the_ticker_viewport_docs_measured() {
    let images = images();
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none() || !images.is_empty(),
        "OAG_REQUIRE_GAME_DATA is set but neither Pulse PSP image is present"
    );
    for (label, path) in images {
        let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
            .unwrap_or_else(|e| panic!("{label}: the archives open: {e}"));
        let xml = front_end_root_xml(&mut archives);
        let screens = Screens::from_xml(&xml);
        let root = parse(&xml);

        let layout = TickerLayout::read(&root, &screens.globals).unwrap_or_else(|| {
            panic!("{label}: Skin.xml authors no TextInfoIsAlwaysLast viewport")
        });

        assert_eq!(
            layout.viewport,
            [85.0, 235.0, 370.0, 32.0],
            "{label}: the ticker's own viewport moved from what \
             docs/ui/campaign-screens.md measured on pulse-psp-usa"
        );
    }
}

/// A sanity check on the wiring, not a new measurement: given the two tips
/// `oag_game::records::ticker_tips` finds on a fresh save (`TKR_NOTOURN`/
/// `TKR_NOHH` are unconditional - see that function's own doc), the ticker
/// actually draws something inside its own viewport at `elapsed = 0.0`. This
/// is what a player sees on the very first frame a menu with no save data
/// yet opens on - not a still with nothing in the footer at all.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_fresh_saves_ticker_tips_actually_draw() {
    let Some((label, path)) = images().into_iter().next() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .unwrap_or_else(|e| panic!("{label}: the archives open: {e}"));
    let xml = front_end_root_xml(&mut archives);
    let screens = Screens::from_xml(&xml);
    let root = parse(&xml);
    let layout = TickerLayout::read(&root, &screens.globals)
        .unwrap_or_else(|| panic!("{label}: no ticker viewport to test against"));

    let mut opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .unwrap_or_else(|e| panic!("{label}: opening the source: {e}"));
    let plugins = opened
        .title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    let mut report = Vec::new();
    let languages =
        oag_ui::language::load::load_languages(&mut opened.archives, plugins, None, &mut report);
    // `oag_ui::language::load::load_strings` is the real path every other caller
    // (the live session, `capture::menu_page`) resolves `TKR_NO*` through -
    // not a hand-rolled read, which would leave this test checking its own
    // parse of `language.entries` (an archive *path*, not decoded text)
    // rather than the wiring this thread actually built.
    let strings =
        oag_ui::language::load::load_strings(&mut opened.archives, &languages, None, &mut report);

    let tips = oag_game::records::ticker_tips(&strings, &oag_game::records::Store::default());
    assert!(
        !tips.is_empty(),
        "{label}: a fresh save should still show TKR_NOTOURN/TKR_NOHH"
    );

    let measure = |text: &str| f32::from(u16::try_from(text.chars().count()).unwrap_or(0)) * 8.0;
    let draw = oag_ui_screens::campaign::footer::ticker_draw(
        &layout,
        0.0,
        &tips,
        &oag_ui_screens::picker::FaceScales::default(),
        &measure,
    );
    assert!(
        draw.is_some(),
        "{label}: the ticker should draw something at elapsed = 0.0 with real tips in hand"
    );
}
