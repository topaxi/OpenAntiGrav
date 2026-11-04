//! The MODS page's ENABLED row: a mod turned off adds nothing and stays listed, the
//! built-in cannot be turned off, and the row follows the MOD row.

use std::path::{Path, PathBuf};

use oag_game::mods::{self, Mods};
use oag_game::settings::Mods as Switches;
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

/// Garage, a mod that adds one row, and a file that is not a zip.
fn base(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-mods-switch-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let garage = dir.join("garage");
    std::fs::create_dir_all(&garage).unwrap();
    std::fs::write(garage.join("oag-mod.toml"), MANIFEST).unwrap();
    std::fs::write(garage.join("menu.toml"), MENU).unwrap();
    std::fs::write(dir.join("broken.oagmod"), b"not a zip").unwrap();
    dir
}

fn off(ids: &[&str]) -> Switches {
    Switches {
        disabled: ids.iter().map(|id| (*id).to_string()).collect(),
        ..Switches::default()
    }
}

fn load(base: &Path, switches: &Switches) -> Mods {
    Mods::load_from(&[base.to_path_buf()], switches).unwrap()
}

fn build(mods: &Mods) -> mods::Built {
    mods.build(
        BUILT_IN,
        None,
        Some(oag_ui::menu::For {
            title: oag_pulse::TITLE,
            updated: false,
        }),
    )
    .unwrap()
}

fn has_row(built: &mods::Built, id: &str) -> bool {
    built.definition.pages[built.definition.root]
        .row_with_id(id)
        .is_some()
}

#[test]
fn a_mod_turned_off_adds_no_row_and_stays_listed_to_be_turned_back_on() {
    let dir = base("off");
    let on = build(&load(&dir, &Switches::default()));
    assert!(has_row(&on, "garage"), "{:?}", on.listing);
    let turned_off = load(&dir, &off(&["test.garage"]));
    let built = build(&turned_off);
    assert!(!has_row(&built, "garage"), "its row is gone");
    assert!(!turned_off.has_player_mods());
    assert_eq!(
        turned_off.player().count(),
        0,
        "no campaign, pilot or track reads it"
    );
    let garage = built
        .listing
        .iter()
        .find(|m| m.id == "test.garage")
        .unwrap();
    assert_eq!(garage.enabled, Some(false));
    assert!(garage.rows.is_empty() && garage.error.is_none());
    let strings = StringTable::default();
    assert!(
        garage
            .table(&strings)
            .iter()
            .any(|(label, value)| label == "ADDS" && value == "NOTHING, TURNED OFF"),
        "{:?}",
        garage.table(&strings)
    );
    assert!(
        built
            .report
            .iter()
            .any(|line| line.contains("test.garage") && line.contains("turned off")),
        "{:?}",
        built.report
    );
    let order: Vec<&str> = built.listing.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        order,
        ["oag.extras", "test.garage", "broken.oagmod"],
        "the extras, the mods, what failed"
    );
}

#[test]
fn the_built_in_cannot_be_turned_off() {
    let dir = base("builtin");
    let built = build(&load(&dir, &off(&[oag_mod::extras::ID])));
    assert!(
        has_row(&built, "mods"),
        "the MODS row is the built-in's; it stays"
    );
    assert_eq!(built.listing[0].enabled, Some(true));
    let mut switches = Switches::default();
    assert!(!mods::switch(
        &mut switches,
        oag_mod::extras::ID,
        true,
        false
    ));
    assert_eq!(switches, Switches::default());
    let (choices, value) = mods::switch_for(&built.listing[0], &StringTable::default());
    assert_eq!(value, "on");
    assert_eq!(choices.len(), 1, "one value: nothing to step to");
    assert_eq!(choices[0].label, "ALWAYS ON", "the one value says why");
    assert!(
        choices[0].unavailable.is_none(),
        "no note to draw over the detail lines"
    );
}

#[test]
fn the_switch_turns_a_mod_off_and_on_again_in_the_set() {
    let mut switches = Switches::default();
    assert!(mods::switch(&mut switches, "test.garage", true, false));
    assert!(
        !mods::switch(&mut switches, "test.garage", true, false),
        "already off"
    );
    assert_eq!(switches, off(&["test.garage"]));
    assert!(mods::switch(&mut switches, "test.garage", true, true));
    assert_eq!(switches, Switches::default());
}

#[test]
fn a_mod_that_ships_off_is_off_until_the_player_turns_it_on() {
    let dir = base("default-off");
    let manifest = dir.join("garage/oag-mod.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        text.replace("[mod]\n", "[mod]\ndefault-enabled = false\n"),
    )
    .unwrap();

    let none = Switches::default();
    let shipped = load(&dir, &none);
    assert!(
        shipped.is_disabled("test.garage"),
        "default-enabled = false"
    );
    assert!(!has_row(&build(&shipped), "garage"));
    let listing = build(&shipped).listing;
    let entry = listing.iter().find(|m| m.id == "test.garage").unwrap();
    assert_eq!(entry.enabled, Some(false), "the MODS page shows it off");
    assert!(!entry.default_enabled);

    let mut switches = none;
    assert!(mods::switch(&mut switches, "test.garage", false, true));
    assert!(switches.enabled.contains("test.garage") && switches.disabled.is_empty());
    let on = load(&dir, &switches);
    assert!(!on.is_disabled("test.garage"));
    assert!(has_row(&build(&on), "garage"), "the player's switch wins");

    assert!(mods::switch(&mut switches, "test.garage", false, false));
    assert_eq!(
        switches,
        Switches::default(),
        "back at the default, nothing stored"
    );
    assert!(load(&dir, &switches).is_disabled("test.garage"));
}

#[test]
fn the_enabled_row_follows_the_mod_row() {
    let dir = base("row");
    let built = build(&load(&dir, &off(&["test.garage"])));
    let strings = StringTable::default();
    let mut model = Menu::new(built.definition.clone());
    model.supply(ValueSource::Mods, &mods::choices(&built.listing, &strings));
    assert!(model.open(mods::PAGE));
    let enabled = |model: &Menu| -> Vec<String> {
        let entry = model
            .page()
            .entries
            .iter()
            .find(|entry| entry.setting() == Some(mods::ENABLED))
            .expect("the MODS page has an ENABLED row");
        let Some(Value::Text(value)) = entry.chosen() else {
            panic!("ENABLED holds nothing");
        };
        let oag_ui::menu::Entry::Choice { values, .. } = entry else {
            panic!("ENABLED is a choice");
        };
        let mut out = vec![value];
        out.extend(values.iter().map(|v| v.value.clone()));
        out
    };
    model.seed(mods::SELECTED, &Value::Text("test.garage".into()));
    mods::resupply_switch(&mut model, &built.listing, &strings);
    assert_eq!(
        enabled(&model),
        ["off", "on", "off"],
        "held off, offered both"
    );
    model.seed(mods::SELECTED, &Value::Text("oag.extras".into()));
    mods::resupply_switch(&mut model, &built.listing, &strings);
    assert_eq!(enabled(&model), ["on", "on"]);
    model.seed(mods::SELECTED, &Value::Text("broken.oagmod".into()));
    mods::resupply_switch(&mut model, &built.listing, &strings);
    assert_eq!(
        enabled(&model),
        ["off", "off"],
        "never opened: nothing to switch"
    );
}

/// A mod `folder` (id `test.<folder>`) with the manifest `extra` appended and `files` beside it.
fn fingerprint_mod(dir: &Path, folder: &str, extra: &str, files: &[&str]) {
    let path = dir.join(folder);
    std::fs::create_dir_all(&path).unwrap();
    let text = MANIFEST
        .replace("test.garage", &format!("test.{folder}"))
        .replace("Garage", folder)
        + extra;
    std::fs::write(path.join("oag-mod.toml"), text).unwrap();
    for file in files {
        let target = path.join(file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, b"x").unwrap();
    }
}

fn fingerprint_of(dir: &Path) -> u64 {
    load(dir, &Switches::default()).content_fingerprint(&Default::default())
}

fn empty_base(name: &str) -> PathBuf {
    let empty = std::env::temp_dir().join(format!("oag-mods-switch-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(&empty).unwrap();
    empty
}

#[test]
fn a_page_or_string_only_mod_leaves_the_lan_fingerprint_alone() {
    let dir = base("fingerprint-pages");
    let none = fingerprint_of(&empty_base("fingerprint-pages-none"));
    assert_eq!(none, oag_net::discovery::NO_MODS);
    assert_eq!(fingerprint_of(&dir), none, "a row-only mod");
    fingerprint_mod(&dir, "words", "", &["strings/english.toml"]);
    fingerprint_mod(&dir, "pilots", "", &["pilots/ace.toml"]);
    assert_eq!(fingerprint_of(&dir), none, "strings and pilots too");
}

#[test]
fn a_simulation_track_or_campaign_mod_changes_the_lan_fingerprint() {
    let none = oag_net::discovery::NO_MODS;
    let simulation = "\n[extension]\nsimulation = true\napi = 1\nstate_bytes = 64\n\n[extension.rules]\nmodule = \"ext/rules.wasm\"\n";
    let cases: [(&str, &str, &[&str]); 3] = [
        ("sim", simulation, &["ext/rules.wasm"]),
        ("track", "", &["tracks/lagoon.track.toml"]),
        ("campaign", "", &["campaigns/run.toml"]),
    ];
    for (folder, extra, files) in cases {
        let dir = base(&format!("fingerprint-{folder}"));
        fingerprint_mod(&dir, folder, extra, files);
        assert_ne!(fingerprint_of(&dir), none, "a {folder} mod changes a race");
        let off =
            load(&dir, &off(&[&format!("test.{folder}")])).content_fingerprint(&Default::default());
        assert_eq!(off, none, "and does not when it is off");
    }
}

fn channel_mod(dir: &Path, folder: &str, id: &str, channels: &str) {
    let path = dir.join(folder);
    std::fs::create_dir_all(&path).unwrap();
    let text = MANIFEST
        .replace("test.garage", id)
        .replace("Garage", folder)
        + "\n[channels]\n"
        + channels;
    std::fs::write(path.join("oag-mod.toml"), text).unwrap();
}

#[test]
fn a_channel_nobody_loaded_sends_is_warned_about_by_name_on_the_listener() {
    let dir = base("channels");
    channel_mod(
        &dir,
        "ear",
        "test.ear",
        "listens = [\"test.mouth/hello\"]\n",
    );
    channel_mod(
        &dir,
        "mouth",
        "test.mouth",
        "publishes = [\"test.mouth/hello\"]\n",
    );
    let strings = StringTable::default();
    let lines = |built: &mods::Built, id: &str| {
        built
            .listing
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .table(&strings)
    };
    let both = build(&load(&dir, &Switches::default()));
    let ear = lines(&both, "test.ear");
    assert!(
        ear.contains(&("HEARS".into(), "test.mouth/hello".into())),
        "{ear:?}"
    );
    assert!(ear.iter().all(|(l, _)| l != "NO ONE SENDS"), "{ear:?}");
    assert!(lines(&both, "test.mouth").contains(&("SENDS".into(), "test.mouth/hello".into())));
    let silent = build(&load(&dir, &off(&["test.mouth"])));
    assert!(
        lines(&silent, "test.ear").contains(&("NO ONE SENDS".into(), "test.mouth/hello".into())),
        "{:?}",
        lines(&silent, "test.ear")
    );
}

#[test]
fn a_flat_list_front_end_with_four_lines_keeps_the_no_one_sends_warning() {
    let dir = base("narrow");
    channel_mod(
        &dir,
        "ear",
        "test.ear",
        "listens = [\"test.mouth/hello\"]\n",
    );
    let strings = StringTable::default();
    let built = build(&load(&dir, &Switches::default()));
    let ear = built.listing.iter().find(|m| m.id == "test.ear").unwrap();
    let lines = ear.table_within(&strings, 4);
    assert!(lines.len() <= 4, "{lines:?}");
    assert!(
        lines.contains(&("NO ONE SENDS".into(), "test.mouth/hello".into())),
        "{lines:?}"
    );
    assert!(lines.iter().any(|(l, _)| l == "ADDS"), "{lines:?}");
}

/// A simulation mod's `sim` pick is in the LAN fingerprint, so two machines on different picks
/// are refused as having different mods; its `view` pick is not, so a Deck on Low and a desktop
/// on High still race together.
#[test]
fn a_sim_pick_changes_the_lan_fingerprint_and_a_view_pick_does_not() {
    let simulation = "\n[extension]\nsimulation = true\napi = 1\nstate_bytes = 64\n\n[extension.rules]\nmodule = \"ext/rules.wasm\"\n";
    let dir = base("fingerprint-picks");
    fingerprint_mod(&dir, "picks", simulation, &["ext/rules.wasm"]);
    std::fs::write(
        dir.join("picks/menu.toml"),
        r#"
[[page]]
id = "p"
title = "P"
[[page.entry]]
kind = "choice"
label = "RULES"
setting = "mod.test.picks.rules"
values = ["a", "b"]
default = "a"
[[page.entry]]
kind = "choice"
label = "LOOK"
setting = "mod.test.picks.look"
values = ["low", "high"]
default = "high"
scope = "view"
"#,
    )
    .unwrap();
    let mods = load(&dir, &Switches::default());
    let picks = |saved: &[(&str, &str)]| {
        let mut table = oag_game::settings::mod_settings::ModSettings::new();
        for (key, value) in saved {
            oag_game::settings::mod_settings::set(
                &mut table,
                &format!("mod.test.picks.{key}"),
                value,
            );
        }
        mods.content_fingerprint(&table)
    };
    let rest = picks(&[]);
    assert_ne!(
        picks(&[("rules", "b")]),
        rest,
        "a sim pick is part of the identity"
    );
    assert_eq!(picks(&[("look", "low")]), rest, "a view pick is not");
}
