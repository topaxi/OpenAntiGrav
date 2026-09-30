//! The three EndRace screens against the disc they are read off:
//! `EndRace_Definition.xml`'s widget inventory, checked against
//! `docs/formats/endrace-screens.md`'s own table.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! # What this pins that a hand-written fixture cannot
//!
//! `oag_ui::endrace`'s own unit tests parse a miniature of this file, and a
//! miniature can only test what its author remembered to include. This
//! reads the real one and checks the two facts `oag_ui::endrace::draw`
//! actually depends on: which widgets a colour-only `Image` (`tablehighlight`,
//! `tablebg1`..`8`) collects as - a [`oag_ui::screen::Fill`], not an
//! `Image`, because it carries no `src` - and that the ones with real
//! textures (`MedalImg`/`loyaltybar`/`boostimg`/`perfectlap1`) land in
//! [`oag_ui::screen::Screen::images`] where the draw code's own name-based
//! gates expect them. Both were read wrong once during this screen's own
//! drawing pass - see `docs/ui/endrace-screens.md`.

use std::path::PathBuf;

use oag_tables::fexml;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn screens() -> Option<oag_ui::screen::Screens> {
    let path = image()?;
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let raw = archives
        .read_name(r"Data\Plugins\PI001\GUI\EndRace_Definition.xml")
        .expect("every pressing carries the three EndRace screens");
    let xml = fexml::text(&raw).expect("it is text");
    Some(oag_ui::screen::Screens::from_xml(&xml))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn all_three_screens_are_present_and_endrace_menu_carries_a_menu_widget() {
    let Some(screens) = screens() else { return };
    for name in ["EndRace Results", "EndRace Rewards", "EndRace Menu"] {
        assert!(screens.by_name(name).is_some(), "{name} did not resolve");
    }
    let menu = screens.by_name("EndRace Menu").unwrap();
    assert!(
        menu.menu.is_some(),
        "Endrace Options should read as a Menu widget"
    );
}

/// `tablehighlight` and every `tablebg{n}` carry `Color`/`Color1..4` and no
/// `src` - they collect as [`oag_ui::screen::Fill`]s, not `Image`s. A draw
/// that gates on `screen.images` by these names (as `results_draw_list`
/// used to) silently never fires.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_table_row_backgrounds_and_the_totals_highlight_are_fills_not_images() {
    let Some(screens) = screens() else { return };
    let results = screens.by_name("EndRace Results").unwrap();
    let fill_names: Vec<&str> = results
        .fills
        .iter()
        .filter_map(|fill| fill.name.as_deref())
        .collect();
    assert!(fill_names.contains(&"tablehighlight"), "{fill_names:?}");
    for n in 1..=8 {
        let name = format!("tablebg{n}");
        assert!(
            fill_names.contains(&name.as_str()),
            "tablebg{n} should be a Fill: {fill_names:?}"
        );
    }
    let image_names: Vec<&str> = results
        .images
        .iter()
        .filter_map(|image| image.name.as_deref())
        .collect();
    assert!(
        !image_names.contains(&"tablehighlight"),
        "tablehighlight must not also collect as an Image"
    );
}

/// The widgets `results_draw_list`/`rewards_draw_list` gate on by name and
/// expect to find in `screen.images` - each carries a real `src`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_real_icon_widgets_collect_as_images() {
    let Some(screens) = screens() else { return };
    let results = screens.by_name("EndRace Results").unwrap();
    let results_images: Vec<&str> = results
        .images
        .iter()
        .filter_map(|image| image.name.as_deref())
        .collect();
    for name in ["boostimg", "perfectlap1"] {
        assert!(
            results_images.contains(&name),
            "{name} should collect as an Image: {results_images:?}"
        );
    }

    let rewards = screens.by_name("EndRace Rewards").unwrap();
    let rewards_images: Vec<&str> = rewards
        .images
        .iter()
        .filter_map(|image| image.name.as_deref())
        .collect();
    for name in ["MedalImg", "LoyaltyImg", "loyaltybg", "loyaltybar"] {
        assert!(
            rewards_images.contains(&name),
            "{name} should collect as an Image: {rewards_images:?}"
        );
    }
}

/// `oag_ui::endrace`'s table walk gates a row's background by the `y` of the
/// anonymous widgets nested in each `tablebg{n}`, which the screen reader collects
/// flat: a `hex_bg.mip` tile and two fading rules. This counts them on the real
/// file, with the bands written out again here rather than shared: **every row owns
/// exactly one tile and two rules, and no other unnamed widget sits in the grid's
/// 92..253 span**, so a widget in a band is one of the row's own.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_table_row_nests_one_tile_and_two_rules_and_nothing_else_sits_in_its_band() {
    let Some(screens) = screens() else { return };
    let results = screens.by_name("EndRace Results").unwrap();
    // Row 1 owns [92, 113); row i > 1 owns [93 + 20 (i - 1), 113 + 20 (i - 1)).
    let band = |y: f32| -> Option<usize> {
        (1..=8).find(|&row| {
            let top = 92.0 + 20.0 * (row - 1) as f32 + if row == 1 { 0.0 } else { 1.0 };
            (top..top + if row == 1 { 21.0 } else { 20.0 }).contains(&y)
        })
    };
    let mut tiles = [0; 8];
    for image in results.images.iter().filter(|image| image.name.is_none()) {
        if let Some(row) = band(image.y) {
            tiles[row - 1] += 1;
        }
    }
    let mut rules = [0; 8];
    for fill in results.fills.iter().filter(|fill| fill.name.is_none()) {
        if let Some(row) = band(fill.y) {
            rules[row - 1] += 1;
        }
    }
    // `zonetopline`'s own right half: an unnamed rule nested at the same `y` (91) as the
    // named left half, which `table_layers` must hide with it.
    let top_rules = results
        .fills
        .iter()
        .filter(|fill| fill.name.is_none() && (fill.y - 91.0).abs() < 0.01)
        .count();
    assert_eq!(top_rules, 1, "one nested half beside zonetopline");
    assert_eq!(tiles, [1; 8], "one hex_bg tile per row");
    assert_eq!(rules, [2; 8], "two fading rules per row");

    // The wrapper fill carries its own `y` without the tag's `OffsetY` - the reader's
    // quirk `oag_ui::endrace` compensates for - so a row's `tablebg` fill reads 0 or 1.
    for row in 1..=8 {
        let name = format!("tablebg{row}");
        let fill = results
            .fills
            .iter()
            .find(|fill| fill.name.as_deref() == Some(name.as_str()))
            .unwrap_or_else(|| panic!("{name} is a fill"));
        assert!(fill.y <= 1.0, "{name} is at {}, not at its row", fill.y);
    }
}
