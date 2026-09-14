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
