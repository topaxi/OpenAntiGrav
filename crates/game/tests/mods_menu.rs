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
    let mods = Mods::load_from(&[base.to_path_buf()]).unwrap();
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
            "REMIX",
            "SPLIT SCREEN",
            "OPTIONS",
            "RECORDS",
            "MODS",
            "QUIT"
        ]
    );
    assert_eq!(
        order(oag_hd::TITLE),
        [
            "RACE CAMPAIGN",
            "RACEBOX",
            "REMIX",
            "SPLIT SCREEN",
            "OPTIONS",
            "MODS",
            "QUIT",
            "RECORDS"
        ]
    );
}
