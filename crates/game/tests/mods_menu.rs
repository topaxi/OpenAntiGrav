//! A mod's menu rows reach the built tree, whichever way the mod is packaged, and a
//! mod that does not fit is left out and named without stopping the build.

use std::io::Write;
use std::path::{Path, PathBuf};

use oag_game::mods::Mods;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

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
after = "records"
kind = "submenu"
label = "GARAGE"
string_id = "TEST_GARAGE"
target = "records"
"#;

const STRINGS: &str =
    "[strings]\nTEST_GARAGE = { text = \"GARAGE (from the mod)\", human = true }\n";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-mods-menu-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn folder(root: &Path, manifest: &str, menu: &str) {
    std::fs::create_dir_all(root.join("strings")).unwrap();
    std::fs::write(root.join("oag-mod.toml"), manifest).unwrap();
    std::fs::write(root.join("menu.toml"), menu).unwrap();
    std::fs::write(root.join("strings/english.toml"), STRINGS).unwrap();
}

fn zipped(path: &Path, method: CompressionMethod) {
    let mut writer = ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = SimpleFileOptions::default().compression_method(method);
    for (name, text) in [
        ("oag-mod.toml", MANIFEST),
        ("menu.toml", MENU),
        ("strings/english.toml", STRINGS),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(text.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
}

/// The main page's labels in drawn order for a mod search over `base`.
fn labels(base: &Path, title: &'static oag_title::Title) -> (Vec<String>, Vec<String>) {
    let mods = Mods::load_from(&[base.to_path_buf()], &Default::default()).unwrap();
    let built = mods
        .build(
            oag_ui::menu::BUILT_IN,
            None,
            Some(oag_ui::menu::For {
                title,
                updated: false,
            }),
        )
        .unwrap();
    let page = &built.definition.pages[built.definition.root];
    (
        page.entries.iter().map(|e| e.label().to_string()).collect(),
        built.report,
    )
}

#[test]
fn a_folder_a_deflate_zip_and_a_zstd_oagmod_each_add_a_row_after_records() {
    let base = scratch("kinds");
    let source = base.join("source");
    folder(&source, MANIFEST, MENU);

    for (kind, set_up) in [
        (
            "folder",
            Box::new(|dir: &Path| folder(&dir.join("garage"), MANIFEST, MENU))
                as Box<dyn Fn(&Path)>,
        ),
        (
            "zip, deflate",
            Box::new(|dir: &Path| zipped(&dir.join("garage.zip"), CompressionMethod::Deflated)),
        ),
        (
            "oagmod, zstd",
            Box::new(|dir: &Path| {
                oag_mod::pack::pack(&source, &dir.join("garage.oagmod")).unwrap();
            }),
        ),
    ] {
        let dir = scratch(kind.split(',').next().unwrap());
        set_up(&dir);
        let (labels, report) = labels(&dir, oag_pulse::TITLE);
        let at = labels.iter().position(|l| l == "RECORDS").unwrap();
        assert_eq!(
            labels[at + 1],
            "GARAGE (from the mod)",
            "{kind}: {labels:?}"
        );
        assert!(
            report
                .iter()
                .any(|l| l.contains("test.garage 0.3.0") && l.contains("garage")),
            "{kind}: the load report names the mod, its version and its row: {report:?}"
        );
    }
}

#[test]
fn a_mod_with_no_license_loads_and_is_marked() {
    let dir = scratch("nolicense");
    folder(
        &dir.join("garage"),
        &MANIFEST.replace("license = \"CC0-1.0\"\n", ""),
        MENU,
    );
    let (_, report) = labels(&dir, oag_pulse::TITLE);
    assert!(
        report
            .iter()
            .any(|l| l.contains("test.garage") && l.contains("no license stated")),
        "{report:?}"
    );
}

#[test]
fn a_mod_whose_row_does_not_fit_is_left_out_and_named() {
    let dir = scratch("broken");
    folder(
        &dir.join("garage"),
        MANIFEST,
        &MENU.replace("parent = \"options\"", "parent = \"nowhere\""),
    );
    let (labels, report) = labels(&dir, oag_pulse::TITLE);
    assert!(!labels.iter().any(|l| l.contains("GARAGE")), "{labels:?}");
    assert!(
        report
            .iter()
            .any(|l| l.contains("test.garage") && l.contains("parent") && l.contains("left out")),
        "{report:?}"
    );
}

#[test]
fn the_extras_alone_give_each_title_its_own_main_menu() {
    let none = scratch("none");
    let order = |title| labels(&none, title).0;
    assert_eq!(
        order(oag_pulse::TITLE),
        [
            "RACE CAMPAIGN",
            "RACEBOX",
            "MULTIPLAYER",
            "OPTIONS",
            "RECORDS",
            "MODS",
            "QUIT",
            "PHOTO MODE"
        ]
    );
    assert_eq!(
        order(oag_hd::TITLE),
        [
            "RACE CAMPAIGN",
            "RACEBOX",
            "MULTIPLAYER",
            "OPTIONS",
            "MODS",
            "QUIT",
            "PHOTO MODE",
            "RECORDS"
        ]
    );
}

/// The pause menu has a PHOTO MODE row on every title, and the row is on the page only while
/// the session offers it (a race with one view held behind the menus).
#[test]
fn the_pause_menu_offers_photo_mode_on_all_five_titles_and_only_when_offered() {
    let none = scratch("photo");
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
        oag_2048::TITLE,
    ] {
        let mods = Mods::load_from(std::slice::from_ref(&none), &Default::default()).unwrap();
        let built = mods
            .build(
                oag_ui::menu::BUILT_IN,
                None,
                Some(oag_ui::menu::For {
                    title,
                    updated: false,
                }),
            )
            .unwrap();
        let mut menu = oag_ui::menu::Menu::new(built.definition);
        let has = |menu: &oag_ui::menu::Menu| {
            menu.page()
                .entries
                .iter()
                .any(|e| e.label() == "PHOTO MODE")
        };
        assert!(!has(&menu), "{}: off the page until offered", title.name);
        menu.offer(oag_ui::menu::Action::PhotoMode, true);
        assert!(has(&menu), "{}: on the page when offered", title.name);
        menu.offer(oag_ui::menu::Action::PhotoMode, false);
        assert!(!has(&menu), "{}: gone when not", title.name);
    }
}

const FRENCH: &str = "[strings]\nTEST_GARAGE = { text = \"GARAGE (du mod)\", human = true }\n";

/// The rebuild a MODS switch or a LANGUAGE pick runs, over a search path of `base`: the
/// main page's labels.
fn rebuilt_labels(base: &Path, disabled: &[&str], language: Option<&str>) -> Vec<String> {
    let disabled = disabled.iter().map(|id| (*id).to_string()).collect();
    let (definition, _, _) = oag_game::menu_tree::rebuilt(
        &[base.to_path_buf()],
        &disabled,
        language,
        oag_pulse::TITLE,
        false,
        &oag_ui::strings::project_table(language),
    )
    .unwrap();
    definition.pages[definition.root]
        .entries
        .iter()
        .map(|e| e.label().to_string())
        .collect()
}

#[test]
fn a_rebuild_without_a_disabled_mod_drops_its_row_and_with_it_restores_the_row() {
    let dir = scratch("rebuild-switch");
    folder(&dir.join("garage"), MANIFEST, MENU);
    let on = rebuilt_labels(&dir, &[], None);
    assert!(on.iter().any(|l| l == "GARAGE (from the mod)"), "{on:?}");
    let off = rebuilt_labels(&dir, &["test.garage"], None);
    assert!(!off.iter().any(|l| l.contains("GARAGE")), "{off:?}");
    assert_eq!(rebuilt_labels(&dir, &[], None), on);
}

#[test]
fn a_rebuild_for_another_language_gives_a_mod_row_that_languages_text() {
    let dir = scratch("rebuild-language");
    folder(&dir.join("garage"), MANIFEST, MENU);
    std::fs::write(dir.join("garage/strings/french.toml"), FRENCH).unwrap();
    let english = rebuilt_labels(&dir, &[], Some("English"));
    let french = rebuilt_labels(&dir, &[], Some("French"));
    assert!(
        english.iter().any(|l| l == "GARAGE (from the mod)"),
        "{english:?}"
    );
    assert!(french.iter().any(|l| l == "GARAGE (du mod)"), "{french:?}");
    assert!(
        !french.iter().any(|l| l == "GARAGE (from the mod)"),
        "{french:?}"
    );
}
