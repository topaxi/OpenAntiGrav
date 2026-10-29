//! The MODS page: reachable from OPTIONS on every title, listing each mod built-in first,
//! marking a mod with no licence, and listing one that did not load dimmed with its error.

use std::path::{Path, PathBuf};

use oag_game::mods::{self, Mods};
use oag_ui::language::StringTable;
use oag_ui::menu::{BUILT_IN, Menu, Value, ValueSource};

const MANIFEST: &str = r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "test.garage"
name = "Garage"
version = "0.3.0"
authors = ["A. Modder"]
license = "CC0-1.0"
description = "One extra row."
"#;

const MENU: &str = r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod-menu.tosd", version = "1.0.0" }

[[entry]]
id = "garage"
parent = "options"
kind = "submenu"
label = "GARAGE"
target = "records"
"#;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-mods-page-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn folder(root: &Path, manifest: &str, menu: &str) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(root.join("oag-mod.toml"), manifest).unwrap();
    std::fs::write(root.join("menu.toml"), menu).unwrap();
}

fn build(base: &Path, title: &'static oag_title::Title) -> mods::Built {
    Mods::load_from(&[base.to_path_buf()], &Default::default())
        .unwrap()
        .build(
            BUILT_IN,
            None,
            Some(oag_ui::menu::For {
                title,
                updated: false,
            }),
        )
        .unwrap()
}

/// Garage (licensed), Plain (no licence), Wrong (a row whose parent does not exist, left
/// out at build) and a file that is not a zip.
fn proof_mods() -> PathBuf {
    let dir = scratch("proof");
    folder(&dir.join("garage"), MANIFEST, MENU);
    folder(
        &dir.join("plain"),
        &MANIFEST
            .replace("test.garage", "test.plain")
            .replace("Garage", "Plain")
            .replace("license = \"CC0-1.0\"\n", ""),
        &MENU.replace("garage", "plain"),
    );
    folder(
        &dir.join("wrong"),
        &MANIFEST.replace("test.garage", "test.wrong"),
        &MENU
            .replace("garage", "wrong")
            .replace("\"options\"", "\"nowhere\""),
    );
    std::fs::write(dir.join("broken.oagmod"), b"not a zip").unwrap();
    dir
}

fn ids(built: &mods::Built) -> Vec<&str> {
    built.listing.iter().map(|m| m.id.as_str()).collect()
}

#[test]
fn every_title_reaches_the_mods_page_from_options() {
    let none = scratch("none");
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
        oag_2048::TITLE,
    ] {
        let built = build(&none, title);
        let main = &built.definition.pages[built.definition.root];
        let at = main
            .row_with_id("mods")
            .unwrap_or_else(|| panic!("{}: no MODS row", title.name));
        assert_eq!(
            main.meta[at].parent.as_deref(),
            Some("options"),
            "{}",
            title.name
        );
        assert!(
            built.definition.page(mods::PAGE).is_some(),
            "{}: no MODS page",
            title.name
        );
    }
}

#[test]
fn the_list_has_the_built_in_first_then_each_mod_then_what_did_not_load() {
    let built = build(&proof_mods(), oag_pulse::TITLE);
    assert_eq!(
        ids(&built),
        [
            "oag.extras",
            "test.garage",
            "test.plain",
            "test.wrong",
            "broken.oagmod"
        ]
    );
    let first = &built.listing[0];
    assert!(first.builtin && first.error.is_none());
    assert!(first.rows.iter().any(|r| r == "mods"), "{:?}", first.rows);
    assert_eq!(built.listing[1].rows, ["garage"]);
}

#[test]
fn a_mod_with_no_license_says_so_in_capitals() {
    let built = build(&proof_mods(), oag_pulse::TITLE);
    let strings = StringTable::default();
    let table = built.listing[2].table(&strings);
    assert!(
        table
            .iter()
            .any(|(label, value)| label == "LICENSE" && value == "NO LICENSE STATED"),
        "{table:?}"
    );
    let garage = built.listing[1].table(&strings);
    assert!(
        garage
            .iter()
            .any(|(label, value)| label == "LICENSE" && value == "CC0-1.0"),
        "{garage:?}"
    );
    assert!(
        garage
            .iter()
            .any(|(l, v)| l == "AUTHORS" && v == "A. Modder")
    );
}

#[test]
fn a_mod_that_did_not_load_is_listed_dimmed_with_its_error() {
    let built = build(&proof_mods(), oag_pulse::TITLE);
    let strings = StringTable::default();
    let choices = mods::choices(&built.listing, &strings);
    assert!(choices[0].unavailable.is_none() && choices[1].unavailable.is_none());
    let wrong = &choices[3];
    assert_eq!(wrong.value, "test.wrong");
    assert!(
        wrong
            .unavailable
            .as_deref()
            .is_some_and(|e| e.contains("left out") && e.contains("nowhere")),
        "{wrong:?}"
    );
    let broken = &choices[4];
    assert_eq!(broken.label, "broken.oagmod");
    assert!(broken.unavailable.is_some(), "{broken:?}");
    assert!(built.listing[4].table(&strings).is_empty());
}

#[test]
fn the_page_row_steps_through_the_list_and_the_table_follows_it() {
    let built = build(&proof_mods(), oag_pulse::TITLE);
    let strings = StringTable::default();
    let mut model = Menu::new(built.definition);
    assert_eq!(
        model.supply(ValueSource::Mods, &mods::choices(&built.listing, &strings)),
        1,
        "one row reads the list"
    );
    assert!(model.open(mods::PAGE));
    let table = |model: &Menu| mods::table_for(model, &built.listing, &strings).unwrap();
    assert!(table(&model).iter().any(|(_, v)| v == "oag.extras"));
    assert!(model.seed(mods::SELECTED, &Value::Text("test.plain".into())));
    assert!(table(&model).iter().any(|(_, v)| v == "NO LICENSE STATED"));
    assert!(
        mods::table_for(
            &Menu::new(build(&proof_mods(), oag_pulse::TITLE).definition),
            &built.listing,
            &strings
        )
        .is_none()
    );
}
