//! `oag_ui::tag_entry::geometry`, against the real `Name`/`Tag` `<TagInput>`
//! screens on the disc - the two instances `crates/ui/tests/tag_input_ground_truth.rs`
//! cannot reach through `oag_ui::screen::Screens`, because both sit under an
//! anonymous `Screen`. See `oag_ui::tag_entry`'s own module doc.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.

use oag_tables::fexml;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("pulse-psp-usa.chd")
}

fn skin_globals(archives: &mut oag_assets::Archives) -> std::collections::HashMap<String, String> {
    let raw = archives
        .read_name(oag_pulse::names::FRONTEND_ROOT)
        .expect("every pressing carries the front-end root");
    let xml = fexml::text(&raw).expect("it is text");
    oag_ui::screen::Screens::from_xml(&xml).globals
}

fn tag_input_entry(archives: &mut oag_assets::Archives) -> String {
    let raw = archives
        .read_hash(oag_pulse::hashes::TAG_INPUT_SCREENS)
        .expect("the TagInput screens entry is on this disc");
    fexml::text(&raw).expect("it is text")
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_profile_name_screen_resolves_off_the_real_disc() {
    let Some(path) = image() else { return };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let globals = skin_globals(&mut archives);
    let xml = tag_input_entry(&mut archives);

    let geometry = oag_ui::tag_entry::geometry(&xml, &globals, "Name")
        .expect("the anonymous screen carrying Name is found");

    assert_eq!(geometry.tag_input.length, 10);
    assert_eq!(geometry.tag_input.y, 85.0);
    assert_eq!(geometry.tag_input.scale, 2.0);
    assert_eq!(geometry.tag_input.x, 50.0, "FEGlobals->TitleXOffset");
    assert_eq!(
        geometry.tag_input.color, 0xFF33_A6B9,
        "FEGlobals->TextColor"
    );

    assert_eq!(geometry.cells.len(), 10, "ten BGgradient cell tiles");
    for (index, cell) in geometry.cells.iter().enumerate() {
        assert_eq!(cell.x, 46.0 + 30.0 * index as f32);
        assert_eq!(cell.y, 88.0);
        assert_eq!(cell.width, Some(28.0));
        assert_eq!(cell.height, Some(25.0));
        assert_eq!(cell.color, 0x2fff_ffff);
    }

    let confirm = geometry.confirm.expect("a confirm label");
    assert_eq!(confirm.idstring.as_deref(), Some("FE_CONFIRM"));
    assert_eq!(confirm.x, 400.0);
    assert_eq!(confirm.y, 90.0);
    assert_eq!(geometry.bars.len(), 4, "two gradient bars, two rects each");

    let title = geometry.title.expect("a title label");
    assert_eq!(title.idstring.as_deref(), Some("PRO_ENTER_NAME"));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_profile_tag_screen_resolves_off_the_real_disc() {
    let Some(path) = image() else { return };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let globals = skin_globals(&mut archives);
    let xml = tag_input_entry(&mut archives);

    let geometry = oag_ui::tag_entry::geometry(&xml, &globals, "Tag")
        .expect("the anonymous screen carrying the profile Tag is found");

    // The *first* `Tag` in the document - `Create Profile Setup`'s own
    // instance is later in the file and under a named `Screen`, which
    // `find_container` never reaches because this one matches first.
    assert_eq!(geometry.tag_input.length, 3);
    assert_eq!(geometry.cells.len(), 3);
}
