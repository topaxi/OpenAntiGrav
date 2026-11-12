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
    drive_args(image, tag, script, tracks, env, &[])
}

/// [`drive_with`] with `args` added to the game's command line.
fn drive_args(
    image: &Path,
    tag: &str,
    script: &str,
    tracks: Option<&Path>,
    env: &[(&str, &str)],
    args: &[&str],
) -> Run {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("drive-{tag}"));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let log = scratch.join("state.jsonl");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--no-audio", "--no-intro", "--drive", script])
        .args(args)
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
    // A scripted run must not open the host's gamepad: a pad in use there would steer it.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("gamepad:"),
        "--drive opened a gamepad:\n{stderr}"
    );
    let states = std::fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("a state line is JSON"))
        .collect();
    Run { output, states }
}

/// Whether the official `oag.remix` mod is checked out (`assets/mods/official`), which the
/// game finds on its default search path from the repository root; a test of its page
/// skips without it.
fn official_remix() -> bool {
    let found = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/mods/official/remix/oag-mod.toml")
        .is_file();
    if !found {
        eprintln!("skipped: the official-mods submodule is not checked out");
    }
    found
}

fn pulse() -> Option<PathBuf> {
    oag_testdata::image("pulse-psp-eu.chd")
}

fn hd() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

fn text<'a>(state: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(state, |at, key| at.get(key))
        .and_then(Value::as_str)
}

/// The frame that opens a selection screen waits on no disc read
/// (`docs/rendering/frame-thread-cpu.md`, "A picker's opening read"): Track Select, Ship Select
/// and the way back each open on a read that landed on a worker before the tap, also after a
/// change of mode asked for another. Fails on a build whose opening frame starts the read
/// itself, which logs that it waited.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_selection_screen_opens_on_a_read_that_has_landed() {
    let Some(image) = pulse() else {
        return;
    };
    let run = drive_with(
        &image,
        "opening-reads",
        "boot; select racebox; press cross; until page=race; \
         until opening_reads=landed; set race.mode=speed_lap; \
         until opening_reads=landed; select launch_race; press cross; \
         until screen.id=track-select; until opening_reads=landed; press cross; \
         until screen.id=ship-select; until opening_reads=landed; press circle; \
         until screen.id=track-select",
        None,
        &[("RUST_LOG", "oag_game=debug")],
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let stderr = run.stderr();
    assert!(
        !stderr.contains("opens waiting for its read"),
        "an opening frame waited:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("opens on a read that has landed").count(),
        3,
        "Track Select, Ship Select and Track Select again:\n{stderr}"
    );
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

/// HD's Racebox TARGET row (its `Eliminations` slider, `200` to `600` by `20`) reaches the race:
/// an Eliminator launched with 400 ends on 400 points, not the slider's minimum. Fails on a build
/// that drops the row's pick on the way to the loader.
#[test]
#[ignore = "needs an HD disc image in data/images/"]
fn the_race_box_target_row_sets_what_an_hd_eliminator_ends_on() {
    let Some(image) = hd() else {
        return;
    };
    let run = drive(
        &image,
        "hd-target",
        "boot; select racebox; press cross; until page=race; \
         set race.mode=eliminator; set race.score_target=400; \
         select launch_race; press cross; \
         until screen=picker; press cross; \
         until screen.id=ship-select; press cross; \
         until stage=race within 3000; ticks 5",
        None,
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let race = &run.last()["race"];
    assert_eq!(text(run.last(), &["race", "mode"]), Some("eliminator"));
    assert_eq!(race["target"].as_u64(), Some(400), "{race}");
}

/// A Race Box race runs the example simulation mods in a build with the `extensions`
/// feature, and none without it: the launch through the menus attaches what `--race` does.
/// With the feature, the energy turbo `--give extension` hands over is held and not spent.
/// (2026-10-29: a Race Box race played on `just play`, a build without the feature, ran
/// no module while the log listed the mods as loaded.)
#[test]
#[ignore = "needs a disc image in data/images/ and the assets/mods/examples submodule"]
fn a_race_box_race_runs_the_simulation_mods_only_with_the_extensions_feature() {
    let Some(image) = pulse() else {
        return;
    };
    if oag_testdata::exact("assets/mods/examples/energy-turbo/oag-mod.toml").is_none() {
        return;
    }
    let run = drive_args(
        &image,
        "racebox-mods",
        "boot; select racebox; press cross; until page=race; \
         set race.mode=single_race; select launch_race; press cross; \
         until screen=picker; select 16_Track; press cross; \
         until screen.id=ship-select; select Feisar; press cross; \
         until stage=race within 3000; ticks 300",
        None,
        &[],
        &["--give", "extension"],
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let race = &run.last()["race"];
    let mods: Vec<&str> = race["mods"]
        .as_array()
        .map(|mods| mods.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if cfg!(feature = "extensions") {
        assert_eq!(mods, ["oag.energy-turbo", "oag.wall-penalty"], "{race}");
        assert_eq!(text(race, &["pickup"]), Some("Extension"), "{race}");
    } else {
        assert!(mods.is_empty(), "{race}");
        assert!(
            run.stderr().contains(
                "its simulation module does not run, this build has no `extensions` feature"
            ),
            "a build without the feature says so"
        );
    }
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

/// A run that ends on the loading screen waits for the race load: its texture sink is inside the
/// driver, and exiting under it crashes NVIDIA's Vulkan driver (`docs/tools/web.md`, "Exit and GPU
/// workers"). On any GPU the line names the worker, so this needs no crash to fail; the load is
/// held up (a debug-build hook) so the script always ends first.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_process_waits_for_the_race_load_before_it_exits() {
    let Some(image) = pulse() else {
        return;
    };
    let run = drive_with(
        &image,
        "exit-in-load",
        "boot; select racebox; press cross; until page=race; \
         set race.mode=speed_lap; select launch_race; press cross; \
         until screen=picker; select 03_Track; press cross; \
         until screen.id=ship-select; select Feisar; press cross; \
         until stage=loading within 3000",
        None,
        &[("OAG_TEST_RACE_LOAD_DELAY_MS", "2000")],
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    assert!(
        run.stderr()
            .lines()
            .any(|line| line.contains("waiting for GPU workers before exit")
                && line.contains("\"race-load\"")),
        "the exit did not wait for the race load:\n{}",
        run.stderr()
    );
}

/// A load that is still running when the process starts to leave stops at its next stage instead
/// of finishing: the load is held at each stage (a debug-build hook,
/// `OAG_TEST_RACE_LOAD_STAGE_DELAY_MS`), the script ends during the first hold, and the
/// game says it waited for `race-load` but never reports the later stages. Dropping the checks
/// from `stages::reach` lets the load run to `CraftsBuilt`, three holds later.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_race_load_stops_at_its_next_stage_when_the_process_exits() {
    let Some(image) = pulse() else {
        return;
    };
    let run = drive_with(
        &image,
        "exit-stops-load",
        "boot; select racebox; press cross; until page=race; \
         set race.mode=speed_lap; select launch_race; press cross; \
         until screen=picker; select 03_Track; press cross; \
         until screen.id=ship-select; select Feisar; press cross; \
         until stage=loading within 3000",
        None,
        &[
            ("OAG_TEST_RACE_LOAD_STAGE_DELAY_MS", "1500"),
            ("RUST_LOG", "oag_raceplay=debug,info"),
        ],
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let stderr = run.stderr();
    assert!(
        stderr
            .lines()
            .any(|line| line.contains("waiting for GPU workers before exit")
                && line.contains("\"race-load\"")),
        "the exit did not wait for the race load:\n{stderr}"
    );
    assert!(
        stderr.contains("race load reached TrackRead"),
        "the load never started:\n{stderr}"
    );
    assert!(
        !stderr.contains("race load reached TrackBuilt"),
        "the load ran on after the process started to exit:\n{stderr}"
    );
    assert!(
        !stderr.contains("the process is exiting"),
        "a stopped load is not a failure a player reads:\n{stderr}"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn race_remix_launches_a_track_file_from_its_custom_title() {
    let Some(image) = pulse() else {
        return;
    };
    if !official_remix() {
        return;
    }
    let tracks = Path::new(env!("CARGO_MANIFEST_DIR")).join("../trackfile/tests/fixtures");
    let run = drive(
        &image,
        "remix-file",
        "boot; select remix; press cross; until page=oag.remix/remix; \
         set mod.oag.remix.track_title=CUSTOM; set mod.oag.remix.track=oval.track.toml; \
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
    if !official_remix() {
        return;
    }
    let tracks = Path::new(env!("CARGO_MANIFEST_DIR")).join("../trackfile/tests/fixtures");
    let run = drive(
        &image,
        "remix-file-zone",
        "boot; select remix; press cross; until page=oag.remix/remix; \
         set mod.oag.remix.mode=zone; \
         set mod.oag.remix.track_title=CUSTOM; set mod.oag.remix.track=oval.track.toml; \
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
        "boot; select racebox; press cross; until page=race; select nonesuch within 30",
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
            rows.iter().find(|row| {
                row.get("setting").and_then(Value::as_str) == Some("mod.oag.remix.track")
            })
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

/// The labels of RACE CAMPAIGN's picker each time it was on screen, in order, one entry per
/// distinct listing. On Pulse the page always exists (its NEW CAMPAIGN row), so what a mod adds
/// or takes away is its own rows.
fn campaign_picker_listings(run: &Run) -> Vec<Vec<String>> {
    let mut seen: Vec<Vec<String>> = Vec::new();
    for state in run
        .states
        .iter()
        .filter(|s| text(s, &["page"]) == Some("campaigns"))
    {
        let Some(rows) = state.pointer("/screen/rows").and_then(Value::as_array) else {
            continue;
        };
        let labels: Vec<String> = rows
            .iter()
            .filter_map(|row| row.get("label").and_then(Value::as_str))
            .map(str::to_string)
            .collect();
        if !labels.is_empty() && seen.last() != Some(&labels) {
            seen.push(labels);
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
    if !official_remix() {
        return;
    }
    // Hillside is a track-only mod; NIGHT RUN (example-campaign) races its track.
    let run = drive(
        &image,
        "mods-switch",
        "boot; select campaign; press cross; until page=campaigns; ticks 5; press circle; \
         until page=main; \
         select remix; press cross; until page=oag.remix/remix; \
         set mod.oag.remix.track_title=CUSTOM; ticks 120; press circle; until page=main; \
         select mods; press cross; until page=mods; \
         set mods.selected=oag.hillside; set mods.enabled=off; expect page=mods; \
         press circle; until page=main; \
         select campaign; press cross; until screen=campaign; ticks 5; press circle; \
         until screen!=campaign; until page=main; \
         select remix; press cross; until page=oag.remix/remix; ticks 60; press circle; \
         until page=main; \
         select mods; press cross; until page=mods; \
         set mods.selected=oag.hillside; set mods.enabled=on; \
         press circle; until page=main; \
         select campaign; press cross; until page=campaigns; ticks 5; press circle; \
         until page=main; ticks 5",
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
    let listings = campaign_picker_listings(&run);
    let has_night_run = |labels: &Vec<String>| labels.iter().any(|l| l == "NIGHT RUN");
    // With Hillside off NIGHT RUN is the only mod campaign refused, so there is no picker:
    // RACE CAMPAIGN opens the disc's campaign in one press, and the picker is back once it is on.
    let page = |s: &Value| text(s, &["page"]) == Some("campaigns");
    let picker_visits = run
        .states
        .windows(2)
        .filter(|pair| !page(&pair[0]) && page(&pair[1]))
        .count();
    assert_eq!(picker_visits, 2, "{listings:?}");
    assert!(
        !listings.is_empty() && listings.iter().all(has_night_run),
        "NIGHT RUN's row is listed whenever the picker is: {listings:?}"
    );
    assert!(
        run.states
            .iter()
            .any(|s| text(s, &["screen", "kind"]) == Some("campaign")),
        "with Hillside off RACE CAMPAIGN goes straight to the disc's campaign, no picker"
    );
    assert!(
        listings
            .iter()
            .all(|l| l.iter().any(|label| label == "NEW CAMPAIGN")),
        "the editor's row stays: {listings:?}"
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

/// The SCREEN SIZE row, stepped the way a pad steps it, reaches the settings file and the menus'
/// own picture: it opens on Pulse's 100, a step to 90 saves `[graphics] hud_safe_area`, and the
/// reported row reads 90. Dropping the session's `graphics.hud_safe_area` arm leaves the file
/// without the key.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_screen_size_row_saves_hud_safe_area() {
    let Some(image) = pulse() else {
        return;
    };
    let run = drive(
        &image,
        "screen-size",
        "boot; select options; press cross; until page=options; \
         select display; press cross; until page=display; \
         set graphics.hud_safe_area=90; ticks 5; expect page=display",
        None,
    );
    assert!(run.output.status.success(), "{}", run.stderr());
    let value = |state: &Value| {
        state["screen"]["rows"]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row["setting"] == "graphics.hud_safe_area")
            })
            .map(|row| row["value"].as_str().unwrap_or_default().to_string())
    };
    let seen: Vec<String> = run.states.iter().filter_map(value).collect();
    assert_eq!(
        seen.first().map(String::as_str),
        Some("100"),
        "opens on Pulse's own default"
    );
    assert_eq!(
        seen.last().map(String::as_str),
        Some("90"),
        "and ends where the pad left it"
    );
    let saved = std::fs::read_to_string(
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("drive-screen-size/config/oag/settings.toml"),
    )
    .expect("the run saved its settings");
    let saved: toml::Table = saved.parse().unwrap();
    assert_eq!(
        saved["graphics"]["hud_safe_area"].as_integer(),
        Some(90),
        "the step wrote the setting: {saved}"
    );
}

/// The rightmost pixel of the HUD's cyan shield bar in the lower right of a frame, which is
/// where the bar's end sits. Pulse's `speed` bar draws pure cyan there at every size.
fn shield_bar_right_edge(path: &Path) -> u32 {
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("a PNG");
    let mut pixels = vec![0; reader.output_buffer_size().expect("a buffer size")];
    let info = reader.next_frame(&mut pixels).expect("a frame");
    let channels = info.color_type.samples();
    let (width, height) = (info.width as usize, info.height as usize);
    let mut right = 0;
    for y in height * 3 / 4..height {
        for x in width * 11 / 20..width {
            let at = (y * width + x) * channels;
            let (r, g, b) = (pixels[at], pixels[at + 1], pixels[at + 2]);
            if r < 60 && g > 200 && b > 200 {
                right = right.max(x as u32);
            }
        }
    }
    right
}

/// A race parked behind the menus takes the SCREEN SIZE row the moment it resumes: the same race,
/// paused, through OPTIONS, DISPLAY, back and resumed, draws its HUD pulled in from the screen's
/// edge. Dropping the per-frame `Overlay::set_safe_area` leaves the resumed HUD at the scale the
/// race loaded with.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_race_resumed_from_the_pause_menu_draws_its_hud_at_the_new_screen_size() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("drive-screen-size-live");
    let _ = std::fs::remove_dir_all(&shots);
    std::fs::create_dir_all(&shots).expect("a screenshot directory");
    let (before, after) = (shots.join("before.png"), shots.join("after.png"));
    let run = drive(
        &image,
        "screen-size-live",
        &format!(
            "boot; select racebox; press cross; until page=race; select launch_race; press cross; \
             until screen=picker; press cross; until screen.id=ship-select; press cross; \
             until stage=race within 4000; ticks 300; screenshot {}; \
             press start; until stage=menu within 300; ticks 20; \
             select pause_options; press cross; until page=options; \
             select display; press cross; until page=display; \
             set graphics.hud_safe_area=90; press circle; until page=options; \
             press circle; until page=pause; press start; until stage=race within 300; \
             ticks 30; screenshot {}",
            before.display(),
            after.display()
        ),
        None,
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let (was, now) = (
        shield_bar_right_edge(&before),
        shield_bar_right_edge(&after),
    );
    assert!(
        was > 0 && now + 30 < was,
        "the shield bar's end moved in from {was} to {now}"
    );
}

/// The MUSIC page's rows reach the settings file: the booted disc's own row
/// saves a switch into `[audio] music_switches`, and turning it back on saves that too.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_music_page_saves_its_switches() {
    let Some(image) = pulse() else {
        return;
    };
    let booted = "disc:wipeout-pulse:psp";
    let run = drive(
        &image,
        "music-switch",
        &format!(
            "boot; select options; press cross; until page=options; \
             select audio; press cross; until page=audio; \
             select music; press cross; until page=music; ticks 30; \
             set audio.music_pool.{booted}=off; ticks 5; expect page=music; \
             press circle; until page=audio; ticks 5"
        ),
        None,
    );
    assert!(run.output.status.success(), "{}", run.stderr());
    let saved = std::fs::read_to_string(
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("drive-music-switch/config/oag/settings.toml"),
    )
    .expect("the run saved its settings");
    let saved: toml::Table = saved.parse().unwrap();
    assert_eq!(
        saved["audio"]["music_switches"][booted].as_bool(),
        Some(false),
        "the booted disc's switch is saved off: {saved}"
    );
}

/// A texture pack switched on the MODS page changes the front end's faces at once, and
/// switched off puts the disc's back: the faces are built with the shell, so the switch
/// reloads it. The loader report names every face drawn from the pack.
#[test]
#[ignore = "needs a disc image in data/images/ and the Pulse pack (just fetch-texture-packs)"]
fn the_mods_page_turns_a_texture_pack_on_and_off_and_the_front_ends_faces_follow_live() {
    let Some(image) = pulse() else {
        return;
    };
    let id = "ppsspp.wipeout-pulse-upscale-version";
    let pack = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/texture-packs/WipEout-Pulse-Upscale-Version/textures.ini");
    if !pack.is_file() {
        eprintln!(
            "skipped: {} is absent (just fetch-texture-packs)",
            pack.display()
        );
        return;
    }
    let run = drive_with(
        &image,
        "mods-texture-pack",
        &format!(
            "boot; ticks 5; select mods; press cross; until page=mods; \
             set mods.selected={id}; set mods.enabled=on; ticks 5; expect page=mods; \
             set mods.enabled=off; ticks 5; expect page=mods; press circle; until page=main"
        ),
        None,
        &[("RUST_LOG", "warn,oag_game=info,oag_raceplay=debug")],
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let stderr = run.stderr();
    let turned_on = stderr
        .find(&format!("mod {id}: turned on"))
        .unwrap_or_else(|| panic!("the pack was never turned on:\n{stderr}"));
    let turned_off = stderr
        .find(&format!("mod {id}: turned off"))
        .unwrap_or_else(|| panic!("the pack was never turned off:\n{stderr}"));
    let from_pack = |text: &str| {
        text.lines()
            .filter(|line| line.contains(".fnt") && line.contains("from the texture pack"))
            .count()
    };
    let art_from_pack = |text: &str| {
        text.lines()
            .filter(|line| line.contains(".mip") && line.contains("from the texture pack, for"))
            .count()
    };
    assert_eq!(
        from_pack(&stderr[..turned_on]),
        0,
        "the boot is the disc's own:\n{stderr}"
    );
    assert!(
        from_pack(&stderr[turned_on..turned_off]) >= 1,
        "turning the pack on redraws the faces from it:\n{stderr}"
    );
    assert_eq!(
        from_pack(&stderr[turned_off..]),
        0,
        "turning it off builds the disc's faces again:\n{stderr}"
    );
    assert_eq!(
        art_from_pack(&stderr[..turned_on]),
        0,
        "the boot's sheet is the disc's:\n{stderr}"
    );
    assert!(
        art_from_pack(&stderr[turned_on..turned_off]) >= 1,
        "turning the pack on rebuilds the sprite sheet from it:\n{stderr}"
    );
    assert_eq!(
        art_from_pack(&stderr[turned_off..]),
        0,
        "turning it off builds the disc's sheet again:\n{stderr}"
    );
}
