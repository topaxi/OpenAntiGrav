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
//! are on the race the game says it loaded, so a launch that dropped the picked track, the
//! picked craft or the track file reports something else and fails.

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

    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }
}

/// Runs the game on `image` with `script` and no window, in a profile of its own.
fn drive(image: &Path, tag: &str, script: &str, tracks: Option<&Path>) -> Run {
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
         set race.mode=speed_lap; select start; press cross; \
         until screen=picker; select 03_Track; press cross; \
         until screen.id=ship-select; select Feisar; press cross; \
         until stage=race within 3000; ticks 30",
        None,
    );
    assert!(run.output.status.success(), "{}", run.stderr());
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
         select start; press cross; until stage=race within 3000; ticks 30",
        Some(&tracks),
    );
    assert!(run.output.status.success(), "{}", run.stderr());
    let last = run.last();
    assert_eq!(text(last, &["stage"]), Some("race"));
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
