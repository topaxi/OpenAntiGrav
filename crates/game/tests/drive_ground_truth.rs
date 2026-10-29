//! `oag-game --drive` takes our own menus into a race with no window, and the race that loads is
//! the one the script selected.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^drive_ground_truth::/)'
//! ```
//!
//! Each test spawns the binary, so it goes through the same `Session` a window runs: the race
//! box's own `launch_race`, the loading stage and the race build. What it reads is the game's
//! own report, one JSON line per change (`--state-log`); no picture is parsed. The assertions
//! are on the race the game says the loader was handed once `stage=race`, so a launch that dropped
//! the picked track, the picked craft or the track file reports something else and fails.

use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;

struct Run {
    output: Output,
    states: Vec<Value>,
}

impl Run {
    fn last(&self) -> &Value {
        self.states.last().expect("the game reported no state")
    }

    /// Why the run did not succeed: its exit status and the lines that are not driver log noise.
    fn why_not_success(&self) -> String {
        format!("{:?}\n{}", self.output.status, self.stderr())
    }

    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }
}

/// Runs the game on `image` with `script` and no window, in a profile of its own.
fn drive(image: &Path, tag: &str, script: &str, tracks: Option<&Path>) -> Run {
    drive_with(image, tag, script, tracks, &[])
}

/// [`drive`] with `env` added to the game's environment.
fn drive_with(
    image: &Path,
    tag: &str,
    script: &str,
    tracks: Option<&Path>,
    env: &[(&str, &str)],
) -> Run {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("drive-{tag}"));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let log = scratch.join("state.jsonl");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--no-audio", "--no-intro", "--drive", script])
        .arg("--state-log")
        .arg(&log)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        // RACE REMIX surveys every disc image on the search path, which starts at `data/images`
        // relative to where the game runs.
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        // No display: the run must not need one.
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY");
    command.envs(env.iter().copied());
    match tracks {
        Some(dir) => command.env("OAG_TRACKS", dir),
        None => command.env("OAG_TRACKS", scratch.join("no-tracks")),
    };
    let output = command.output().expect("running oag-game");
    let states = std::fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("a state line is JSON"))
        .collect();
    Run { output, states }
}

fn pulse() -> Option<PathBuf> {
    oag_testdata::image("pulse-psp-eu.chd")
}

fn text<'a>(state: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(state, |at, key| at.get(key))
        .and_then(Value::as_str)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_race_box_launches_the_track_and_craft_it_was_told_to_pick() {
    let Some(image) = pulse() else {
        return;
    };
    let run = drive(
        &image,
        "racebox",
        "boot; select racebox; press cross; until page=race; \
         set race.mode=speed_lap; select launch_race; press cross; \
         until screen=picker; select 03_Track; press cross; \
         until screen.id=ship-select; select Feisar; press cross; \
         until stage=race within 3000; ticks 30",
        None,
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let last = run.last();
    assert_eq!(text(last, &["stage"]), Some("race"));
    assert_eq!(text(last, &["race", "title"]), Some("Wipeout Pulse"));
    assert_eq!(
        text(last, &["race", "track"]),
        Some(r"Data\Environments\03_Track\track.vex"),
        "Moa Therma White was picked"
    );
    assert_eq!(text(last, &["race", "team"]), Some("Feisar"));
    assert_eq!(text(last, &["race", "mode"]), Some("speed_lap"));
    // It went through the loading stage, as a window's launch does, and showed both pickers.
    let stages: Vec<&str> = run
        .states
        .iter()
        .filter_map(|state| text(state, &["stage"]))
        .collect();
    assert!(
        stages.windows(2).any(|pair| pair == ["loading", "race"]),
        "{stages:?}"
    );
    let screens: Vec<&str> = run
        .states
        .iter()
        .filter_map(|state| text(state, &["screen", "id"]))
        .collect();
    assert!(
        screens.contains(&"track-select") && screens.contains(&"ship-select"),
        "{screens:?}"
    );
}

/// The process does not exit under a GPU worker: the race's EndRace prefetch is held up for three
/// seconds (a debug-build hook, `OAG_TEST_PREFETCH_DELAY_MS`), the script ends first, and the
/// game says it waited for the worker and exits 0. Dropping `gpu_workers::finish` from `main`
/// removes the line and, on a machine slow enough, the exit status with it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_process_waits_for_a_running_gpu_worker_before_it_exits() {
    let Some(image) = pulse() else {
        return;
    };
    let race = "boot; select racebox; press cross; until page=race; \
         set race.mode=speed_lap; select launch_race; press cross; \
         until screen=picker; select 03_Track; press cross; \
         until screen.id=ship-select; select Feisar; press cross; \
         until stage=race within 3000; ticks 5";
    let slow = [("OAG_TEST_PREFETCH_DELAY_MS", "3000")];
    for (tag, script) in [
        ("exit-in-race", race.to_string()),
        ("exit-after-escape", format!("{race}; escape; ticks 5")),
    ] {
        let run = drive_with(&image, tag, &script, None, &slow);
        assert!(
            run.output.status.success(),
            "{tag}: {}",
            run.why_not_success()
        );
        assert!(
            run.stderr().contains("waiting for GPU workers before exit"),
            "{tag}: the exit did not wait for the prefetch:\n{}",
            run.stderr()
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn race_remix_launches_a_track_file_from_its_custom_title() {
    let Some(image) = pulse() else {
        return;
    };
    let tracks = Path::new(env!("CARGO_MANIFEST_DIR")).join("../trackfile/tests/fixtures");
    let run = drive(
        &image,
        "remix-file",
        "boot; select remix; press cross; until page=remix; \
         set remix.track_title=CUSTOM; set remix.track=oval.track.toml; \
         select launch_remix; press cross; until stage=race within 3000; ticks 30",
        Some(&tracks),
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let last = run.last();
    assert_eq!(text(last, &["stage"]), Some("race"));
    assert!(
        text(last, &["race", "track_file"]).is_some_and(|path| path.ends_with("oval.track.toml")),
        "the picked track file is the one raced: {last}"
    );
}

/// MODE Zone on RACE REMIX's CUSTOM title races the file's loop as the Zone circuit:
/// the launch used to log "a custom track has no Zone circuit" and stay on the page.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn race_remix_launches_a_track_file_in_zone_mode() {
    let Some(image) = pulse() else {
        return;
    };
    let tracks = Path::new(env!("CARGO_MANIFEST_DIR")).join("../trackfile/tests/fixtures");
    let run = drive(
        &image,
        "remix-file-zone",
        "boot; select remix; press cross; until page=remix; \
         set race.mode=zone; \
         set remix.track_title=CUSTOM; set remix.track=oval.track.toml; \
         select launch_remix; press cross; until stage=race within 3000; ticks 30",
        Some(&tracks),
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let last = run.last();
    assert_eq!(text(last, &["stage"]), Some("race"));
    assert_eq!(text(last, &["race", "mode"]), Some("zone"));
    assert!(
        text(last, &["race", "track_file"]).is_some_and(|path| path.ends_with("oval.track.toml")),
        "the picked track file is the one raced: {last}"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_row_that_does_not_exist_and_a_wait_that_never_ends_fail_with_the_state() {
    let Some(image) = pulse() else {
        return;
    };
    let missing = drive(
        &image,
        "no-row",
        "boot; select racebox; press cross; until page=race; select nonesuch",
        None,
    );
    assert!(!missing.output.status.success());
    let why = missing.stderr();
    assert!(
        why.contains("no row \"nonesuch\"") && why.contains("race.mode"),
        "{why}"
    );
    assert!(
        why.contains("last state:") && why.contains("\"page\":\"race\""),
        "{why}"
    );

    let stuck = drive(
        &image,
        "timeout",
        "boot; until page=nowhere within 40",
        None,
    );
    assert!(!stuck.output.status.success());
    let why = stuck.stderr();
    assert!(
        why.contains("timed out after 40 ticks") && why.contains("\"page\":\"main\""),
        "{why}"
    );
}

/// The labels of the remix TRACK row in every state that shows it, in order.
fn remix_tracks(run: &Run) -> Vec<Vec<String>> {
    let mut seen: Vec<Vec<String>> = Vec::new();
    for state in &run.states {
        let rows = state.pointer("/screen/rows").and_then(Value::as_array);
        let Some(track) = rows.and_then(|rows| {
            rows.iter()
                .find(|row| row.get("setting").and_then(Value::as_str) == Some("remix.track"))
        }) else {
            continue;
        };
        let labels: Vec<String> = (track.get("values").and_then(Value::as_array))
            .into_iter()
            .flatten()
            .filter_map(|value| value.get("label").and_then(Value::as_str))
            .map(str::to_string)
            .collect();
        if seen.last() != Some(&labels) {
            seen.push(labels);
        }
    }
    seen
}

/// What RACE CAMPAIGN is on the main page, in order: `submenu` while a mod campaign puts
/// the picker behind it, `action` (the disc's campaign) with none.
fn campaign_row_kinds(run: &Run) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for state in run
        .states
        .iter()
        .filter(|s| text(s, &["page"]) == Some("main"))
    {
        let rows = state.pointer("/screen/rows").and_then(Value::as_array);
        let kind = rows
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row.get("id").and_then(Value::as_str) == Some("campaign"))
            })
            .and_then(|row| row.get("kind").and_then(Value::as_str));
        if let Some(kind) = kind
            && seen.last().map(String::as_str) != Some(kind)
        {
            seen.push(kind.to_string());
        }
    }
    seen
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_mods_page_turns_a_mod_off_and_on_and_what_it_adds_goes_and_comes_back_live() {
    let Some(image) = pulse() else {
        return;
    };
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/mods/examples");
    if !examples.join("hillside/oag-mod.toml").is_file() {
        eprintln!("skipped: the mod-examples submodule is not checked out");
        return;
    }
    // Hillside is a track-only mod; NIGHT RUN (example-campaign) races its track.
    let run = drive(
        &image,
        "mods-switch",
        "boot; select remix; press cross; until page=remix; \
         set remix.track_title=CUSTOM; ticks 120; press circle; until page=main; \
         select mods; press cross; until page=mods; \
         set mods.selected=oag.hillside; set mods.enabled=off; expect page=mods; \
         press circle; until page=main; \
         select remix; press cross; until page=remix; ticks 60; press circle; until page=main; \
         select mods; press cross; until page=mods; \
         set mods.selected=oag.hillside; set mods.enabled=on; \
         press circle; until page=main; ticks 5",
        None,
    );
    assert!(run.output.status.success(), "{}", run.stderr());
    let tracks = remix_tracks(&run);
    let has = |labels: &[String], name: &str| labels.iter().any(|l| l.contains(name));
    let custom: Vec<&Vec<String>> = tracks.iter().filter(|labels| has(labels, "OCTA")).collect();
    assert!(
        custom.first().is_some_and(|l| has(l, "HILLSIDE")),
        "CUSTOM lists Hillside's track while it is on: {tracks:?}"
    );
    assert!(
        custom.last().is_some_and(|l| !has(l, "HILLSIDE")),
        "and not once it is off, the campaign's own track kept: {tracks:?}"
    );
    assert_eq!(
        campaign_row_kinds(&run),
        ["submenu", "action", "submenu"],
        "NIGHT RUN's picker goes with Hillside off (its event races Hillside) and comes back"
    );
    let why = run.stderr();
    assert!(
        why.contains("\"oag.hillside\" is turned off on the MODS page"),
        "the campaign is refused by name, saying why: {why}"
    );
    let saved = std::fs::read_to_string(
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("drive-mods-switch/config/oag/settings.toml"),
    )
    .expect("the run saved its settings");
    let saved: toml::Table = saved.parse().unwrap();
    assert_eq!(
        saved["mods"]["disabled"].as_array().map(Vec::len),
        Some(0),
        "turned back on, nothing is left off"
    );
}
