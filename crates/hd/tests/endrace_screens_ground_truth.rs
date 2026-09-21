//! Wipeout HD/Fury's own `EndRace_Definition.xml`, read off the real disc -
//! the evidence `docs/formats/hd-endrace-screens.md` is written from, pinned
//! so a future change to `oag_assets`' precedence, `oag_ui::screen`'s
//! `"block"` parsing, or `oag_hd::endrace`'s own constants notices if it
//! stops agreeing with the disc.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```

use std::path::PathBuf;

use oag_assets::Archives;
use oag_ui::endrace::Layout;
use oag_ui::language::StringTable;
use oag_ui::picker::FaceScales;
use oag_ui::screen::Screens;

const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn opened() -> Option<Archives> {
    let path = image()?;
    Some(oag_hd::open(path.to_str().expect("utf-8 path")).expect("the disc opens as HD"))
}

/// `Archives::read_name`'s own precedence reaches `DATA02`'s copy - `data`
/// (`DATA00`, no copy) then `fe` (`DATA02`) - matching
/// `oag_title::FrontEnd::endrace_entry`'s own doc and
/// `docs/formats/hd-endrace-screens.md`'s own "This build's own mount
/// order" claim. Sizes match that page's own inventory table exactly.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_precedence_resolved_copy_is_data02_by_size() {
    let Some(mut archives) = opened() else {
        return;
    };
    let blob = archives
        .read_name(oag_hd::endrace::SCREEN_ENTRY)
        .expect("EndRace_Definition.xml reads");
    assert_eq!(blob.len(), 30_110, "DATA02's own copy, by size");
}

/// Every one of the five archives that carry `endrace_definition.xml`
/// parses as plain UTF-8 (no `oag_tables::fexml` dictionary), at the size
/// `docs/formats/hd-endrace-screens.md`'s own inventory table states, and
/// `DATA00`/`DATA01` carry no copy at all.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn five_archives_carry_the_file_at_its_own_documented_size() {
    let Some(mut archives) = opened() else {
        return;
    };
    let mut expect_size = |archive: &str, size: usize| {
        let matches = archives.read_every_name(oag_hd::endrace::SCREEN_ENTRY);
        let (_, blob) = matches
            .iter()
            .find(|(label, _)| label.ends_with(archive))
            .unwrap_or_else(|| panic!("{archive} should carry {}", oag_hd::endrace::SCREEN_ENTRY));
        assert_eq!(blob.len(), size, "{archive}'s own copy");
        // Plain UTF-8, not fexml-shortened - the file does not open with a
        // `<code>` dictionary the way every PSP screen file does.
        let text = String::from_utf8(blob.clone()).expect("plain UTF-8");
        assert!(
            !text.trim_start_matches('\u{feff}').starts_with("<code"),
            "{archive}'s own copy should not be dictionary-shortened"
        );
    };
    expect_size(oag_hd::archives::DATA02, 30_110);
    expect_size(oag_hd::archives::DATA03, 30_623);
    expect_size(oag_hd::archives::DATA04, 31_053);
    expect_size(oag_hd::archives::DATA05, 42_406);
    expect_size(oag_hd::archives::DATA06, 41_190);

    let matches = archives.read_every_name(oag_hd::endrace::SCREEN_ENTRY);
    for archive in [oag_hd::archives::DATA00, oag_hd::archives::DATA01] {
        assert!(
            !matches.iter().any(|(label, _)| label.ends_with(archive)),
            "{archive} should carry no copy of this file at all"
        );
    }
}

/// `DATA02`'s copy: three screens (`Results`/`Rewards`/`Menu`, no
/// `Podium`), `EndRace Results`' own grid is four columns by ten rows and
/// only the first two columns are captioned - the shape
/// `oag_ui::endrace::hd` draws off, read here independent of that crate so
/// a change to either notices the other.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn data02_authors_three_screens_and_a_four_by_ten_grid() {
    let Some(mut archives) = opened() else {
        return;
    };
    let blob = archives
        .read_name(oag_hd::endrace::SCREEN_ENTRY)
        .expect("reads");
    let xml = String::from_utf8(blob).expect("plain UTF-8");
    let screens = Screens::from_xml(&xml);

    for name in ["EndRace Results", "EndRace Rewards", "EndRace Menu"] {
        assert!(
            screens.by_name(name).is_some(),
            "{name} should be on DATA02"
        );
    }
    assert!(
        screens.by_name("EndRace Podium").is_none(),
        "DATA02 should carry no Podium screen"
    );

    let results = Layout::read_authored(
        &screens,
        "EndRace Results",
        &StringTable::default(),
        FaceScales::default(),
        oag_hd::endrace::AUTHORED_GRID,
        oag_hd::endrace::AUTHORED_GRID,
    )
    .expect("EndRace Results is on this screen");

    // Both halves numeric - the same requirement
    // `oag_ui::endrace::hd::grid_slot` makes, which is what excludes
    // `Grid0.h`/`Grid1.h`/`Grid2.h` (the still-unexplained white-ink
    // variant, see `docs/formats/hd-endrace-screens.md`) from the 4x10
    // shape proper.
    let grid_texts: Vec<&str> = results
        .screen
        .texts
        .iter()
        .filter_map(|text| text.name.as_deref())
        .filter(|name| {
            name.strip_prefix("Grid").is_some_and(|rest| {
                let mut parts = rest.split('.');
                let col = parts.next().unwrap_or_default();
                let row = parts.next().unwrap_or_default();
                !col.is_empty()
                    && !row.is_empty()
                    && col.chars().all(|c| c.is_ascii_digit())
                    && row.chars().all(|c| c.is_ascii_digit())
            })
        })
        .collect();
    assert_eq!(
        grid_texts.len(),
        40,
        "four columns (Grid0..Grid3) by ten rows (.0..0.9) - {grid_texts:?}"
    );

    let head1 = results
        .screen
        .texts
        .iter()
        .find(|text| text.name.as_deref() == Some("GridHead1"))
        .expect("GridHead1 is a <Block>, collected as a Text by oag_ui::screen's \"block\" arm");
    let head2 = results
        .screen
        .texts
        .iter()
        .find(|text| text.name.as_deref() == Some("GridHead2"))
        .expect("GridHead2 likewise");
    assert_eq!(head1.idstring.as_deref(), Some("IG_HUD_POS"));
    assert_eq!(head2.idstring.as_deref(), Some("IG_HUD_TIME"));
    assert!(head2.x > head1.x, "TIME sits to the right of POS");
}

/// `EndRace Menu`'s eight `<Block>` options, by name and idstring - the
/// table `oag_ui::endrace::MenuOption::hd_block_name`/`idstring` is written
/// against. Five of the eight idstrings are byte-identical to
/// `MenuOption::idstring`'s own Pulse-derived table; this test is what
/// backs that claim in `docs/formats/hd-endrace-screens.md`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn menu_blocks_match_the_names_and_idstrings_menuoption_expects() {
    let Some(mut archives) = opened() else {
        return;
    };
    let blob = archives
        .read_name(oag_hd::endrace::SCREEN_ENTRY)
        .expect("reads");
    let xml = String::from_utf8(blob).expect("plain UTF-8");
    let screens = Screens::from_xml(&xml);
    let layout = Layout::read_authored(
        &screens,
        "EndRace Menu",
        &StringTable::default(),
        FaceScales::default(),
        oag_hd::endrace::AUTHORED_GRID,
        oag_hd::endrace::AUTHORED_GRID,
    )
    .expect("EndRace Menu is on this screen");

    let expect = |name: &str, idstring: &str| {
        let text = layout
            .screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name} should be a <Block> on EndRace Menu"));
        assert_eq!(
            text.idstring.as_deref(),
            Some(idstring),
            "{name}'s own idstring"
        );
    };
    expect("next_race", "ER_NEXT_RACE");
    expect("race_again", "ER_RACE_AGAIN");
    expect("return_to_grid", "ER_RETURN_GRID");
    expect("return_to_menu", "ER_RETURN_MENU");
    expect("quit_tournament", "ER_QUIT_TOUR");
    expect("return_to_lobby", "IG_PAUSE_QUIT");
    expect("view_again", "ER_VIEW_AGAIN");
    expect("view_MP_again", "ER_VIEW_AGAIN");

    // Every implemented MenuOption's own idstring is on the screen and
    // resolves to a real Block - the direct check that
    // `oag_ui::endrace::hd::hd_menu_draw_list`'s own `find_text` lookup by
    // `hd_block_name()` finds something on the real disc, not only on the
    // crate's own miniature fixture.
    for option in [
        oag_ui::endrace::MenuOption::NextRace,
        oag_ui::endrace::MenuOption::RaceAgain,
        oag_ui::endrace::MenuOption::ReturnToGrid,
        oag_ui::endrace::MenuOption::ReturnToMenu,
        oag_ui::endrace::MenuOption::ViewResultsAgain,
    ] {
        let block = layout
            .screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(option.hd_block_name()))
            .unwrap_or_else(|| panic!("{:?}'s own Block should be on the real disc", option));
        assert_eq!(block.idstring.as_deref(), Some(option.idstring()));
    }
}

/// HD's own English string table (`DATA02`'s
/// `Languages\English\entries.xml`) names a finishing position
/// `ER_{n}PLACE`, not Pulse's `ER_{n}STP` - the measured divergence
/// `oag_ui::endrace::hd::hd_headline_text` branches on. See that
/// function's own doc.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn hds_own_english_table_names_a_place_with_the_place_suffix_not_pulses_stp() {
    let Some(mut archives) = opened() else {
        return;
    };
    let blob = archives
        .read_name(r"Data\Plugins\Languages\English\entries.xml")
        .expect("English entries.xml reads");
    let xml = String::from_utf8(blob)
        .expect("plain UTF-8")
        .to_ascii_lowercase();

    for place in 1..=8 {
        assert!(
            xml.contains(&format!("id=\"er_{place}place\"")),
            "ER_{place}PLACE should be in HD's own English table"
        );
    }
    assert!(
        !xml.contains("id=\"er_1stp\""),
        "ER_1STP (Pulse's own idstring) should not be in HD's own table"
    );

    // The three headline idstrings `hd_headline_text` reuses unchanged
    // (`TimeTrial`/`SpeedLap`/`NoPosition`) really are shared, verbatim,
    // between the two titles - the corroboration
    // `docs/formats/hd-endrace-screens.md` cites for why `Position` alone
    // needed its own branch rather than the whole table being HD-specific.
    for shared in ["er_tt_com", "er_sl_com", "er_ship_des"] {
        assert!(
            xml.contains(&format!("id=\"{shared}\"")),
            "{shared} should be shared with Pulse's own idstring table"
        );
    }
}
