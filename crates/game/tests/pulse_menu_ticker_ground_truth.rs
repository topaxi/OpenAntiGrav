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
//! (`oag_game::boot::screens::read_ticker`, `crate::app::menu_stage::footer`
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

/// A sanity check on the wiring, not a new measurement: with the disc's own `PI_News` items and a
/// fresh save (no statistic counted), the ticker's rotation is exactly the `TKR_NO*` tips - the
/// six per-mode time items each fall back to their "try it" tip - and it draws something inside
/// its own viewport at `elapsed = 0.0`. Drop the items from the layout and nothing plays.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_fresh_saves_ticker_plays_the_untried_mode_tips() {
    let Some((label, path)) = images().into_iter().next() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .unwrap_or_else(|e| panic!("{label}: the archives open: {e}"));
    let xml = front_end_root_xml(&mut archives);
    let screens = Screens::from_xml(&xml);
    let root = parse(&xml);
    let mut layout = TickerLayout::read(&root, &screens.globals)
        .unwrap_or_else(|| panic!("{label}: no ticker viewport to test against"));

    let mut opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .unwrap_or_else(|e| panic!("{label}: opening the source: {e}"));
    layout.news = oag_game::campaign::read_news(&mut opened.archives, opened.title);
    let plugins = opened
        .title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    let mut report = Vec::new();
    let languages =
        oag_ui::language::load::load_languages(&mut opened.archives, plugins, None, &mut report);
    // `oag_ui::language::load::load_strings` is the real path every other caller
    // (the live session, `capture::menu_page`) resolves `TKR_*` through - not a hand-rolled
    // read, which would leave this test checking its own parse of `language.entries`.
    let strings =
        oag_ui::language::load::load_strings(&mut opened.archives, &languages, None, &mut report);

    let store = oag_game::records::Store::default();
    let tips = oag_game::records::ticker_tips(Some(&layout), &strings, &store, 1);
    let untried: Vec<&str> = [
        "TKR_NOSR",
        "TKR_NOSL",
        "TKR_NOTT",
        "TKR_NOTOURN",
        "TKR_NOELIM",
        "TKR_NOZONE",
    ]
    .into_iter()
    .filter_map(|id| strings.get(id))
    .collect();
    assert_eq!(
        untried.len(),
        6,
        "{label}: the six tips are in the string table"
    );
    assert!(
        !tips.is_empty(),
        "{label}: a fresh save still shows the untried-mode tips"
    );
    assert!(
        tips.iter().all(|tip| untried.contains(&tip.as_str())),
        "{label}: nothing but those six is countable on a fresh save: {tips:?}"
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

/// **The `PI_News` list is the disc's, read, on every Pulse pressing present.** PSP (both
/// pressings carry the same file): 124 rows, draw weight 122, `TKR_FYISL` and `TKR_FYIZONE` at
/// weight 2, four `Unlockable` rows that are never drawn. PS2: 121 rows (no `TKR_FYISL`, `TKR_FYIZONE` or
/// concept-ship announcement, so three `Unlockable` rows). Every item's string is in the table.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_news_items_are_the_discs_list() {
    let pressings: Vec<(&str, PathBuf, usize, u32)> = [
        ("pulse-psp-eu", "data/images/pulse-psp-eu.chd", 124, 122),
        ("pulse-psp-usa", "data/images/pulse-psp-usa.chd", 124, 122),
        ("pulse-ps2-eu", "data/images/pulse-ps2-eu.chd", 121, 118),
    ]
    .into_iter()
    .filter_map(|(label, name, rows, weight)| {
        oag_testdata::image(name).map(|path| (label, path, rows, weight))
    })
    .collect();
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none() || !pressings.is_empty(),
        "OAG_REQUIRE_GAME_DATA is set but no Pulse image is present"
    );
    for (label, path, rows, weight) in pressings {
        let mut opened =
            oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
                .unwrap_or_else(|e| panic!("{label}: opening the source: {e}"));
        let feed = oag_game::campaign::read_news(&mut opened.archives, opened.title);
        assert_eq!(feed.items.len(), rows, "{label}: PI_News rows");
        let total: u32 = feed
            .items
            .iter()
            .map(oag_ui_screens::news::NewsItem::weight)
            .sum();
        assert_eq!(total, weight, "{label}: the draw's total weight");
        let unlockables = feed
            .items
            .iter()
            .filter(|item| item.class == oag_ui_screens::news::Class::Unlockable)
            .count();
        assert_eq!(
            unlockables,
            if rows == 124 { 4 } else { 3 },
            "{label}: unlock announcements"
        );
        let heavy: Vec<&str> = feed
            .items
            .iter()
            .filter(|item| item.weight() == 2)
            .map(|item| item.id.as_str())
            .collect();
        let expected: &[&str] = if rows == 124 {
            &["TKR_FYIZONE", "TKR_FYISL"]
        } else {
            &[]
        };
        assert_eq!(heavy, expected, "{label}: the weight-2 items");
        let plugins = opened
            .title
            .front_end
            .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
        let mut report = Vec::new();
        let languages = oag_ui::language::load::load_languages(
            &mut opened.archives,
            plugins,
            None,
            &mut report,
        );
        let strings = oag_ui::language::load::load_strings(
            &mut opened.archives,
            &languages,
            None,
            &mut report,
        );
        let missing: Vec<&str> = feed
            .items
            .iter()
            .filter(|item| strings.get(&item.id).is_none())
            .map(|item| item.id.as_str())
            .collect();
        assert!(
            missing.is_empty(),
            "{label}: items with no string: {missing:?}"
        );
    }
}

/// **The footer tag's glow is the disc's authored `RealGlow`, and the card titles author `pulse`.**
/// `NewsItemTagText` carries `RealGlow="64"` in Pulse's front-end root (the one widget authoring that
/// value) and `Grid Selection`'s `Title` (`GRID 1`) and `Cell
/// Selection`'s (`SINGLE RACE`) are `pulse="true"` Menu-font texts. Wipeout Pure's root authors no
/// glow on its tag. A reader that drops either attribute stamps nothing.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_footer_tag_and_card_titles_author_their_glow() {
    use oag_ui::frontend::Draw;
    use oag_ui_screens::picker::FaceScales;

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
        let legend = oag_ui_screens::campaign::footer::NavigationLegend::read(
            &parse(&xml),
            &screens.globals,
            &oag_ui::language::StringTable::default(),
        )
        .unwrap_or_else(|| panic!("{label}: Skin.xml authors a NavigationController"));
        let stamps: Vec<_> = legend
            .draw(&FaceScales::default())
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::GlowStamp { rect, value } => Some((rect, value)),
                _ => None,
            })
            .collect();
        assert_eq!(
            stamps,
            [([29.0, 234.0, 28.0, 14.0], 64)],
            "{label}: the tag's stamp"
        );

        let cell = archives
            .read_name(oag_game::campaign::SCREEN_ENTRY)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let cell = Screens::from_xml(&fexml::text(&cell).expect("text"));
        for (screen, string) in [
            ("Grid Selection", "GRID 1"),
            ("Cell Selection", "SINGLE RACE"),
        ] {
            let screen = cell
                .screens
                .iter()
                .find(|candidate| candidate.name == screen)
                .unwrap_or_else(|| panic!("{label}: {screen} is authored"));
            let title = screen
                .texts
                .iter()
                .find(|text| text.name.as_deref() == Some("Title"))
                .unwrap_or_else(|| panic!("{label}: {} has a Title", screen.name));
            assert!(
                title.pulse,
                "{label}: {string} title is authored pulse=\"true\""
            );
            assert_eq!(
                title.glow_stamp(0.0),
                220,
                "{label}: a pulsing title opens at the ladder's top"
            );
        }
    }
}

/// What Pulse authors for the glow it stamps on the title and the Main Menu, against the numbers
/// `oag_pulse::FRONT_END.menu.glow` carries (EDRAM frames of both screens on PPSSPP): the seven
/// `helptext` widgets author `RealGlow="128"` (the `help` reference), the title's
/// `BOOT_PRESS_START` is `pulse="true"`, and the end-of-race `tablehighlight` and the scrollbars
/// author `RealGlow="128"` onto a colour-only widget (a `Fill`). Dropping the read of any stamps
/// nothing.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_main_menu_help_title_text_and_fills_author_their_glow() {
    let images = images();
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none() || !images.is_empty(),
        "OAG_REQUIRE_GAME_DATA is set but neither Pulse PSP image is present"
    );
    let glow = oag_pulse::FRONT_END
        .menu
        .and_then(|menu| menu.glow)
        .expect("Pulse's menu skin carries its glow");
    for (label, path) in images {
        let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
            .unwrap_or_else(|e| panic!("{label}: the archives open: {e}"));
        let root = Screens::from_xml(&front_end_root_xml(&mut archives));
        let logo = root
            .screens
            .iter()
            .find(|screen| screen.name == "Show Logo")
            .unwrap_or_else(|| panic!("{label}: Skin.xml authors Show Logo"));
        let press = logo
            .texts
            .iter()
            .find(|text| text.idstring.as_deref() == Some("BOOT_PRESS_START"))
            .unwrap_or_else(|| panic!("{label}: Show Logo has BOOT_PRESS_START"));
        assert!(press.pulse, "{label}: BOOT_PRESS_START is pulse=\"true\"");
        assert_eq!(
            press.glow_stamp_at(1.0),
            Some(220),
            "{label}: the ladder's top"
        );

        // `Data.wad`'s `MainMenu_Definition.xml` and the end-of-race screens, found by what they
        // author rather than by name, since the entries are hash-addressed.
        let mut helps = 0;
        let mut highlight = None;
        let oag_assets::Container::Wad(wad) = &mut archives.data else {
            panic!("{label}: Pulse's Data archive is a WAD");
        };
        for index in 0..wad.directory().entries.len() {
            let Ok(head) = wad.peek(index, 64) else {
                continue;
            };
            if !fexml::is_fexml(&head) {
                continue;
            }
            let Ok(raw) = wad.read(index) else {
                continue;
            };
            if !fexml::is_fexml(&raw) {
                continue;
            }
            let Ok(text) = fexml::text(&raw) else {
                continue;
            };
            let screens = Screens::from_xml(&text);
            for screen in &screens.screens {
                for t in &screen.texts {
                    if screen.name == "Main Menu"
                        && t.idstring
                            .as_deref()
                            .is_some_and(|id| id.starts_with("FE_HELP_"))
                    {
                        assert_eq!(t.real_glow, glow.help, "{label}: {:?}", t.idstring);
                        helps += 1;
                    }
                }
                for fill in &screen.fills {
                    if screen.name == "EndRace Results" && fill.real_glow == 128 {
                        highlight = Some(fill.glow_stamp());
                    }
                }
            }
        }
        assert_eq!(helps, 7, "{label}: seven help lines author the stamp");
        assert!(
            matches!(
                highlight,
                Some(Some(oag_ui::frontend::Draw::GlowStamp { value: 128, .. }))
            ),
            "{label}: the end-of-race highlight stamps 128, got {highlight:?}"
        );
    }
}
