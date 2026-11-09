//! A mod folder carrying a `[[page]]` loads end to end: the page is reached from its row,
//! a bad page leaves only its mod out, and `scripts/check-strings.py` holds a mod page to
//! its string ids (ADR-0082 decisions 1 and 6).

use std::path::{Path, PathBuf};
use std::process::Command;

use oag_game::mods::{self, Mods};
use oag_ui::menu::{BUILT_IN, Entry};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/mod-pages");

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A mods folder holding the fixture mods named in `names`.
fn folder(tag: &str, names: &[&str]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-mods-pages-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for name in names {
        copy(&Path::new(FIXTURES).join(name), &dir.join(name));
    }
    dir
}

fn build(dir: &Path) -> mods::Built {
    Mods::load_from(&[dir.to_path_buf()], &oag_game::settings::Mods::default())
        .unwrap()
        .build(
            BUILT_IN,
            None,
            Some(oag_ui::menu::For {
                title: oag_pulse::TITLE,
                updated: false,
            }),
        )
        .unwrap()
}

#[test]
fn a_mod_page_is_reached_from_its_row_in_the_built_tree() {
    let built = build(&folder("good", &["good"]));
    let at = built
        .definition
        .pages
        .iter()
        .position(|page| page.id == "test.good/shelf")
        .expect("the mod's page is in the tree under its namespaced id");
    let reached = built
        .definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .any(|entry| matches!(entry, Entry::Submenu { target, .. } if *target == at));
    assert!(reached, "its row opens the page");
    let listed = built.listing.iter().find(|m| m.id == "test.good").unwrap();
    assert!(listed.error.is_none(), "{:?}", listed.error);
    assert_eq!(built.definition.pages[at].title, "SHELF");
}

#[test]
fn a_bad_mod_page_leaves_out_its_mod_alone_and_names_the_mod_and_the_key() {
    let dir = folder("bad", &["good"]);
    let broken = dir.join("broken");
    copy(&Path::new(FIXTURES).join("good"), &broken);
    std::fs::write(
        broken.join("oag-mod.toml"),
        std::fs::read_to_string(broken.join("oag-mod.toml"))
            .unwrap()
            .replace("test.good", "test.broken"),
    )
    .unwrap();
    // The row that reached the page is gone: nothing opens it.
    let menu = std::fs::read_to_string(broken.join("menu.toml")).unwrap();
    let orphaned = menu.split("[[entry]]").next().unwrap();
    std::fs::write(broken.join("menu.toml"), orphaned).unwrap();

    let built = build(&dir);
    assert!(
        built
            .definition
            .pages
            .iter()
            .any(|page| page.id == "test.good/shelf"),
        "the good mod still loads"
    );
    assert!(
        !built
            .definition
            .pages
            .iter()
            .any(|page| page.id.starts_with("test.broken/")),
        "the broken mod adds nothing"
    );
    let line = built
        .report
        .iter()
        .find(|line| line.contains("left out") && line.contains("test.broken"))
        .unwrap_or_else(|| panic!("{:?}", built.report));
    assert!(line.contains("page \"shelf\""), "{line}");
    let listed = built
        .listing
        .iter()
        .find(|m| m.id == "test.broken")
        .unwrap();
    assert!(listed.error.is_some(), "listed with its error");
}

fn check_strings(fixture: &str) -> (bool, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new("python3")
        .arg(root.join("scripts/check-strings.py"))
        .arg("--mod")
        .arg(Path::new(FIXTURES).join(fixture))
        .current_dir(&root)
        .output()
        .expect("python3 runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn check_strings_is_red_on_a_mod_page_missing_its_text_and_green_with_it() {
    let (ok, said) = check_strings("good");
    assert!(ok, "{said}");
    let (ok, said) = check_strings("missing-string");
    assert!(!ok, "{said}");
    assert!(said.contains("TEST_PAGES_BACK"), "{said}");
}

fn setting_fixture(name: &str) -> mods::Built {
    build(&folder(name, &["good", name]))
}

#[test]
fn a_page_row_saving_under_its_own_mod_id_loads() {
    let built = setting_fixture("setting-own");
    let page = built
        .definition
        .pages
        .iter()
        .find(|page| page.id == "test.setting-own/shelf")
        .unwrap_or_else(|| panic!("{:?}", built.report));
    assert!(
        page.entries
            .iter()
            .any(|e| e.setting() == Some("mod.test.setting-own.pick"))
    );
}

#[test]
fn a_page_row_naming_another_mods_setting_drops_its_mod_alone_naming_mod_and_key() {
    let built = setting_fixture("setting-foreign");
    assert!(
        built
            .definition
            .pages
            .iter()
            .any(|page| page.id == "test.good/shelf"),
        "the other mod still loads"
    );
    assert!(
        !built
            .definition
            .pages
            .iter()
            .any(|page| page.id.starts_with("test.setting-foreign/")),
        "the offending mod adds nothing"
    );
    let line = built
        .report
        .iter()
        .find(|line| line.contains("left out") && line.contains("test.setting-foreign"))
        .unwrap_or_else(|| panic!("{:?}", built.report));
    assert!(line.contains("setting"), "{line}");
    assert!(line.contains("mod.test.good.pick"), "{line}");
}
