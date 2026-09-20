//! The menu skins, against the discs they were read off.
//!
//! `menu_skin.rs` checks the two title packages against *each other* and runs in
//! CI. This one checks each against its own disc, so a number that was read
//! wrong - or a disc that turns out to say something else - fails here rather
//! than only looking odd on screen.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.

use std::path::{Path, PathBuf};

use oag_tables::fexml;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every `<Variable global="...">` the front end's root XML declares.
///
/// Both dialects, because Pulse's `Skin.xml` is `<code>`-shortened and Pure's is
/// plain - which is why this goes through `fexml::text` rather than reaching
/// straight for `expand`.
fn globals(image: &Path, title: &'static oag_title::Title) -> Vec<(String, String)> {
    let mut archives =
        oag_assets::Archives::open(&image.to_string_lossy(), title).expect("the archives open");
    let raw = archives
        .read_name(r"Data\Plugins\PI001\GUI\Skin.xml")
        .expect("every pressing carries the front-end root");
    let xml = fexml::text(&raw).expect("it is text");
    let root = fexml::parse(&xml);

    let mut out = Vec::new();
    collect(&root, &mut out);
    out
}

fn collect(node: &fexml::Node, out: &mut Vec<(String, String)>) {
    if node.name.eq_ignore_ascii_case("Variable")
        && let Some(name) = node.attr("global")
        && let Some(value) = node.value("String")
    {
        out.push((name.to_string(), value.to_string()));
    }
    for child in &node.children {
        collect(child, out);
    }
}

fn number(globals: &[(String, String)], key: &str) -> Option<f32> {
    globals
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .and_then(|(_, value)| value.parse().ok())
}

fn argb(globals: &[(String, String)], key: &str) -> Option<u32> {
    let raw = globals
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))?
        .1
        .clone();
    u32::from_str_radix(raw.trim_start_matches("0x").trim_start_matches("0X"), 16).ok()
}

/// Pulse's skin still says what Pulse's disc says.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_skin_matches_its_own_skin_xml() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let declared = globals(&path, oag_pulse::TITLE);
    let skin = oag_pulse::FRONT_END.menu.expect("Pulse authors a MenuSkin");

    assert_eq!(number(&declared, "MenuXOffset"), Some(skin.menu_x));
    assert_eq!(number(&declared, "MenuScale"), Some(skin.menu_scale));
    assert_eq!(number(&declared, "TitleXOffset"), Some(skin.title_x));
    assert_eq!(number(&declared, "TitleYOffset"), Some(skin.title_y));
    assert_eq!(number(&declared, "TitleScale"), Some(skin.title_scale));
    assert_eq!(argb(&declared, "TextColor"), skin.text);
    assert_eq!(argb(&declared, "TitleColor"), skin.title);
}

/// Pure's skin still says what Pure's disc says - **and still does not say what
/// its disc leaves out.**
///
/// The second half is the one that matters: Pure declares neither `TextColor`
/// nor `TitleColor`, and a future edit filling those in from Pulse would pass
/// every other check in this repository.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_skin_matches_its_own_skin_xml_and_its_silences() {
    let Some(path) = image("pure-psp-eu.chd") else {
        return;
    };
    let declared = globals(&path, oag_pure::TITLE);
    let skin = oag_pure::FRONT_END.menu.expect("Pure authors a MenuSkin");

    assert_eq!(number(&declared, "MenuXOffset"), Some(skin.menu_x));
    assert_eq!(number(&declared, "MenuScale"), Some(skin.menu_scale));
    assert_eq!(number(&declared, "TitleXOffset"), Some(skin.title_x));
    assert_eq!(number(&declared, "TitleYOffset"), Some(skin.title_y));
    assert_eq!(number(&declared, "TitleScale"), Some(skin.title_scale));

    assert!(
        argb(&declared, "TextColor").is_none(),
        "Pure's Skin.xml declares no TextColor; its skin's value is a measured \
         fallback, not a reading"
    );
    assert!(
        argb(&declared, "TitleColor").is_none(),
        "Pure's Skin.xml declares no TitleColor"
    );
}

/// Pulse's main menu still says the row geometry the skin carries.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_first_row_comes_off_its_main_menu_definition() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let raw = archives
        .read_name(r"Data\Plugins\PI001\GUI\MainMenu_Definition.xml")
        .expect("Pulse carries a main-menu definition");
    let xml = fexml::text(&raw).expect("it is text");
    let root = fexml::parse(&xml);

    let mut found = None;
    find_menu(&root, &mut found);
    let (y, font) = found.expect("the main menu has a Menu widget");

    let skin = oag_pulse::FRONT_END.menu.expect("Pulse authors a MenuSkin");
    assert_eq!(
        Some(y),
        skin.first_row_y,
        "the skin's first row is the widget's own y"
    );
    assert_eq!(
        Some(font.as_str()),
        skin.menu_font,
        "the skin's font role is the widget's own"
    );
}

/// The first `Menu` widget's `y` and `font`.
fn find_menu(node: &fexml::Node, out: &mut Option<(f32, String)>) {
    if out.is_some() {
        return;
    }
    if node.name.eq_ignore_ascii_case("Menu")
        && let Some(y) = node.value("y").and_then(|v| v.parse().ok())
        && let Some(font) = node.value("font")
    {
        *out = Some((y, font.to_string()));
        return;
    }
    for child in &node.children {
        find_menu(child, out);
    }
}

/// Every `<Entry idstring="...">` in the file, in document order - `Main
/// Menu`'s own seven rows, per `docs/formats/fe-menu-definitions.md`.
///
/// Nothing else in `MainMenu_Definition.xml` carries an `idstring`:
/// `TournamentLoad`'s own `<Redirect><Entry item="Mode" equals="..."
/// goto="...">` shares the `Entry` tag name but never an `idstring`
/// attribute, so this does not need to scope itself to the `Menu` widget to
/// avoid picking those up.
fn find_entries(node: &fexml::Node, out: &mut Vec<String>) {
    if node.name.eq_ignore_ascii_case("Entry")
        && let Some(id) = node.attr("idstring")
    {
        out.push(id.to_string());
    }
    for child in &node.children {
        find_entries(child, out);
    }
}

/// `entries.xml`'s own `<Entry ID="..." String="...">` table, flat rather
/// than the `Variable`/`Values` nesting [`collect`] reads off `Skin.xml`.
fn find_strings(node: &fexml::Node, out: &mut Vec<(String, String)>) {
    if node.name.eq_ignore_ascii_case("Entry")
        && let Some(id) = node.attr("ID")
        && let Some(value) = node.attr("String")
    {
        out.push((id.to_string(), value.to_string()));
    }
    for child in &node.children {
        find_strings(child, out);
    }
}

/// `Main Menu`'s own row order and wording, read off the USA disc's English
/// plugin - the primary source `docs/formats/fe-menu-definitions.md`'s own
/// table cites. Pins the two rows `assets/ui/menu.toml` was reworded to
/// match: `FE_RACE_CAM` first, `FE_RACEBOX` second.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_main_menu_row_order_and_text_come_off_the_disc() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");

    let raw = archives
        .read_name(r"Data\Plugins\PI001\GUI\MainMenu_Definition.xml")
        .expect("Pulse carries a main-menu definition");
    let xml = fexml::text(&raw).expect("it is text");
    let root = fexml::parse(&xml);
    let mut ids = Vec::new();
    find_entries(&root, &mut ids);
    assert_eq!(
        ids,
        vec![
            "FE_RACE_CAM",
            "FE_RACEBOX",
            "FE_MP",
            "FE_WIPEOUT_DOT_COM",
            "FE_PROFILE",
            "FE_OPT_PLUS",
            "FE_EXTRAS",
        ],
        "the seven rows, in the disc's own order"
    );

    let raw = archives
        .read_name(r"Data\Plugins\PI012\entries.xml")
        .expect("PI012 is the USA disc's English plugin");
    let xml = fexml::text(&raw).expect("it is text");
    let root = fexml::parse(&xml);
    let mut strings = Vec::new();
    find_strings(&root, &mut strings);
    let text = |id: &str| {
        strings
            .iter()
            .find(|(name, _)| name == id)
            .map(|(_, value)| value.as_str())
            .unwrap_or_else(|| panic!("entries.xml carries no {id:?}"))
    };

    assert_eq!(text("FE_RACE_CAM"), "RACE CAMPAIGN");
    assert_eq!(text("FE_RACEBOX"), "RACEBOX");
}
