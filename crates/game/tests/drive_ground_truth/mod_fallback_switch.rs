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

/// A race of `mode` driven from the menus, quit, the mod switched off on the MODS page and the
/// recording watched, all in one session; `then` is what happens after WATCH.
fn watched_with_the_mod_off(tag: &str, mode: &str, then: &str) -> Option<Run> {
    let (image, root) = (pulse()?, official()?);
    let script = format!(
        "boot; select racebox; press cross; until page=race; set race.mode={mode}; \
         select launch_race; press cross; until screen=picker; press cross; \
         until screen.id=ship-select; press cross; until stage=race within 4000; \
         ticks 280; hold cross 120; escape; until page=pause within 100; \
         select pause_quit; press cross; until page=main within 600; \
         select mods; press cross; until page=mods; select {ID}; press square; ticks 5; \
         press circle; until page=main; \
         select replays; press cross; until page=replays; ticks 30; \
         select watch_replay; press cross; {then}"
    );
    let mods = root.parent().expect("the official mods");
    let run = drive_with(
        &image,
        tag,
        &script,
        None,
        &[("OAG_MODS", mods.to_str().expect("a UTF-8 path"))],
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    Some(run)
}

/// A Detonator race driven from the menus is recorded with the mod in its header, and watching
/// it after the MODS page has switched the mod off in the same session says which mod is
/// missing on the file's row.
#[test]
#[ignore = "needs a disc image in data/images/ and the official-mods submodule"]
fn a_detonator_replay_watched_with_its_mod_off_names_the_mod() {
    let Some(run) = watched_with_the_mod_off(
        "mods-replay",
        "detonator",
        "until page=replays within 900; ticks 5",
    ) else {
        return;
    };
    let auto = run.profile.join("data/oag/replays/auto");
    let file = std::fs::read_dir(&auto)
        .expect("the race was kept")
        .next()
        .expect("one recording")
        .expect("readable")
        .path();
    let replay = oag_replay::Replay::from_bytes(&std::fs::read(file).expect("the recording"))
        .expect("it reads back");
    assert!(
        replay
            .header
            .active_mods
            .iter()
            .any(|m| m.id == ID && !m.version.is_empty()),
        "{:?}",
        replay.header.active_mods
    );

    let why = (run.last().pointer("/screen/rows/0/values/0/unavailable"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("the row is not dimmed: {}\n{}", run.last(), run.stderr()));
    assert!(why.starts_with(&format!("missing {ID} ")), "{why}");
}

/// A recording that still loads with the mod off (a Time Trial needs none of its entries) plays,
/// and the viewer opens saying which mod is missing.
#[test]
#[ignore = "needs a disc image in data/images/ and the official-mods submodule"]
fn a_replay_that_still_loads_without_a_mod_says_so_as_the_viewer_opens() {
    let Some(run) = watched_with_the_mod_off(
        "mods-replay-plays",
        "time_trial",
        "until replay=playing within 4000; ticks 5",
    ) else {
        return;
    };
    let notice = text(run.last(), &["notice"])
        .unwrap_or_else(|| panic!("no notice: {}\n{}", run.last(), run.stderr()));
    assert!(
        notice.starts_with(&format!("mods differ from the recording: missing {ID} ")),
        "{notice}"
    );
}
