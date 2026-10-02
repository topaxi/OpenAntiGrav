//! `Race End Photo` against the disc it is read off: `InGame_Definition.xml`'s
//! screen of that name, the one the original sits on between the flag and
//! `EndRace Results`.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! What this pins that a hand-written fixture cannot: that the screen is found
//! by the generic screen reader at all (the file nests it among a hundred
//! others), that the legend is exactly two text widgets, and where the disc
//! puts them. `PhotoMessage`'s and `ProceedMessage`'s placement, font, colour
//! and string ids are read off the file, never typed into this crate.

use oag_tables::fexml;

fn screens() -> Option<oag_ui::screen::Screens> {
    let path = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let raw = archives
        .read_name(oag_pulse::names::INGAME_DEFINITION)
        .expect("every Pulse pressing carries InGame_Definition.xml");
    let xml = fexml::text(&raw).expect("it is text");
    Some(oag_ui::screen::Screens::from_xml(&xml))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_legend_is_two_stats_font_texts_at_the_authored_places() {
    let Some(screens) = screens() else { return };
    let screen = screens
        .by_name(oag_ui::endrace::photo::SCREEN)
        .expect("Race End Photo is on InGame_Definition.xml");
    let text = |name: &str| {
        screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name} is not a text widget on Race End Photo"))
    };
    let proceed = text("ProceedMessage");
    assert_eq!(proceed.idstring.as_deref(), Some("FE_PRESS_TO_CONT"));
    assert!(
        proceed.font.eq_ignore_ascii_case("stats"),
        "{}",
        proceed.font
    );
    assert_eq!((proceed.x, proceed.y), (20.0, 242.0));
    let photo = text("PhotoMessage");
    assert_eq!(photo.idstring.as_deref(), Some("ER_PRESS_SELECT_PHOTO"));
    assert!(photo.font.eq_ignore_ascii_case("stats"), "{}", photo.font);
    assert_eq!((photo.x, photo.y), (20.0, 212.0));
    assert_eq!(screen.texts.len(), 2, "no third text on this screen");
}
