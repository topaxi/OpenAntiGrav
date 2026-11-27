//! Switching a mod with fallback entries on the MODS page moves the RACE page's MODE row at
//! once (ADR-0100). The official `oag.detonator-pulse` is the mod: Pulse offers DETONATOR only
//! while its entries are mounted.
//!
//! Two switch paths rebuild the menus, and each has to mount what it rebuilt: a plain mod
//! rebuilds the tree, a texture pack reloads the whole shell.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Run, drive_with, pulse, text};

const ID: &str = "oag.detonator-pulse";

/// On, off and on again, reading the MODE row's values after each switch.
fn switch_twice(tag: &str, mods: &Path) -> Option<Vec<bool>> {
    let image = pulse()?;
    let race = "select racebox; press cross; until page=race; ticks 2; press circle; \
                until page=main";
    let toggle = format!(
        "select mods; press cross; until page=mods; select {ID}; press square; ticks 5; \
         press circle; until page=main"
    );
    let script = format!("boot; {race}; {toggle}; {race}; {toggle}; {race}");
    let run = drive_with(
        &image,
        tag,
        &script,
        None,
        &[("OAG_MODS", mods.to_str().expect("a UTF-8 path"))],
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    Some(detonator_offered(&run))
}

/// For each visit to the RACE page, whether its MODE row offered Detonator when it was left.
fn detonator_offered(run: &Run) -> Vec<bool> {
    let mut visits = Vec::new();
    let mut on_page = false;
    for state in &run.states {
        let offered = (text(state, &["page"]) == Some("race"))
            .then(|| mode_values(state))
            .flatten()
            .map(|values| values.iter().any(|v| v == "detonator"));
        match offered {
            Some(offered) if on_page => *visits.last_mut().expect("a visit") = offered,
            Some(offered) => visits.push(offered),
            None => {}
        }
        on_page = offered.is_some();
    }
    visits
}

fn mode_values(state: &Value) -> Option<Vec<String>> {
    let row = (state.pointer("/screen/rows")?.as_array()?.iter())
        .find(|row| text(row, &["setting"]) == Some("race.mode"))?;
    Some(
        (row["values"].as_array()?.iter())
            .filter_map(|value| Some(text(value, &["value"])?.to_string()))
            .collect(),
    )
}

fn official() -> Option<PathBuf> {
    let manifest = oag_testdata::exact("assets/mods/official/detonator-pulse/oag-mod.toml")?;
    Some(manifest.parent()?.to_path_buf())
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
#[ignore = "needs a disc image in data/images/ and the official-mods submodule"]
fn the_mods_page_switching_detonator_moves_the_mode_row_live() {
    let Some(root) = official() else {
        return;
    };
    let Some(offered) = switch_twice("mods-fallback", root.parent().expect("the official mods"))
    else {
        return;
    };
    assert_eq!(offered, [true, false, true], "on, off, on again");
}

#[test]
#[ignore = "needs a disc image in data/images/ and the official-mods submodule"]
fn a_mod_that_is_also_a_texture_pack_moves_the_mode_row_live() {
    let Some(root) = official() else {
        return;
    };
    let scratch = oag_testdata::scratch::dir("drive-mods-fallback-pack");
    let copy = scratch.join("mods/detonator-pulse");
    copy_tree(&root, &copy);
    let manifest = copy.join("oag-mod.toml");
    let mut text = std::fs::read_to_string(&manifest).unwrap();
    // A texture pack starts off unless its manifest says otherwise.
    text.push_str("default-enabled = true\n\n[texture-pack]\nformat = \"ppsspp\"\n");
    std::fs::write(&manifest, text).unwrap();
    std::fs::write(
        copy.join("textures.ini"),
        "[options]\nversion = 1\nhash = xxh64\n",
    )
    .unwrap();
    let Some(offered) = switch_twice("mods-fallback-pack-run", &scratch.join("mods")) else {
        return;
    };
    assert_eq!(offered, [true, false, true], "on, off, on again");
}
