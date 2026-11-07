//! The replay viewer in the game: a race recorded by default, watched, and a build mismatch shown.
//! ADR-0087, stage 5 of decision 10. `--drive`, no window.
//!
//! - **A race is recorded** by the session that hands it over: a file in `auto/` after a race
//!   that ran long enough, and none after a race a `--race` run built (decision 1: a race a test,
//!   a trace or a capture builds records nothing unless it asks).
//! - **Watching writes nothing it must not**: `records.toml`, `ghosts/` and `auto/` are what they
//!   were before the viewer opened (a watched race is not a record, a ghost or a replay).
//! - **A mismatch is shown**: one tick of the recorded input is changed and the file written
//!   back, so the first stored hash after it disagrees. The viewer plays the part that agrees,
//!   stops at its last second (`replay=stopped`), and the banner is on screen.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game replay_viewer_ground_truth:: --run-ignored all
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

struct Profile {
    dir: PathBuf,
}

impl Profile {
    fn new(tag: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("replay-viewer-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("no-mods")).expect("a scratch directory");
        eprintln!("profile: {}", dir.display());
        Self { dir }
    }

    fn auto(&self) -> PathBuf {
        self.dir.join("data/oag/replays/auto")
    }

    /// Runs the game on `image` with `script` and no window, in this profile, with `args`.
    fn drive(
        &self,
        image: &Path,
        script: &str,
        args: &[&str],
    ) -> (std::process::ExitStatus, Vec<Value>) {
        let log = self.dir.join("state.jsonl");
        let _ = std::fs::remove_file(&log);
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
            .arg(image)
            .args(["--no-audio", "--no-intro", "--seed", "7", "--drive"])
            .arg(script.replace("DIR", &self.dir.display().to_string()))
            .args(args)
            .arg("--state-log")
            .arg(&log)
            .env("XDG_CONFIG_HOME", self.dir.join("config"))
            .env("XDG_DATA_HOME", self.dir.join("data"))
            .env("XDG_STATE_HOME", self.dir.join("state"))
            .env("OAG_TRACKS", self.dir.join("no-tracks"))
            // No simulation mod: a modded race records nothing (decision 7).
            .env("OAG_MODS", self.dir.join("no-mods"))
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .output()
            .expect("running oag-game");
        let states = std::fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).expect("a state line is JSON"))
            .collect();
        if !output.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        (output.status, states)
    }

    /// Every file under `config/` and `data/oag/ghosts`, by path, with its bytes: what a watched
    /// race must leave alone.
    fn kept(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut out = BTreeMap::new();
        for root in [self.dir.join("config"), self.dir.join("data/oag/ghosts")] {
            collect(&root, &mut out);
        }
        out
    }

    fn replays(&self) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(self.auto())
            .map(|dir| dir.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        found.sort();
        found
    }
}

fn collect(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if let Ok(bytes) = std::fs::read(&path) {
            out.insert(path, bytes);
        }
    }
}

fn pulse() -> Option<PathBuf> {
    oag_testdata::image("pulse-psp-eu.chd")
}

const TO_THE_RACE: &str = "boot; select racebox; press cross; until page=race; \
     select launch_race; press cross; until screen=picker; press cross; \
     until screen.id=ship-select; press cross; until stage=race within 4000; ";

const TO_THE_LIST: &str = "boot; select replays; press cross; until page=replays; ";

fn text<'a>(state: &'a Value, key: &str) -> Option<&'a str> {
    state.get(key).and_then(Value::as_str)
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn a_recorded_race_is_watched_and_a_mismatch_is_shown() {
    let Some(image) = pulse() else {
        return;
    };
    let profile = Profile::new("mismatch");

    // 1. A race driven from the menus is recorded by the session: throttle for 24 s.
    let (status, _) = profile.drive(
        &image,
        &format!("{TO_THE_RACE} ticks 280; hold cross 1500; quit"),
        &[],
    );
    assert!(status.success());
    let recorded = profile.replays();
    assert_eq!(
        recorded.len(),
        1,
        "one race, one file in auto/: {recorded:?}"
    );
    let original = std::fs::read(&recorded[0]).expect("the recording");
    let replay = oag_replay::Replay::from_bytes(&original).expect("it reads back");
    assert!(replay.ticks() >= 1_700, "{} ticks", replay.ticks());
    assert_eq!(
        replay.header.ticks,
        replay.ticks(),
        "the length is in the header the list reads"
    );

    // 2. One tick of the throttle is flipped, and the file written back as the newest.
    let mut perturbed = replay.clone();
    let tick = 1_500;
    let held = perturbed.inputs[0][tick]
        .buttons
        .is_held(oag_gameplay::input::Button::Cross);
    let mut buttons = oag_gameplay::input::Input::new();
    let mask = if held {
        0
    } else {
        oag_gameplay::input::Button::Cross.bit()
    };
    buttons.begin_frame(mask);
    buttons.begin_frame(mask);
    perturbed.inputs[0][tick].buttons = buttons;
    perturbed.inputs[0][tick].stick_y = 0.9;
    let name = recorded[0]
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .replace("-1.oagr", "-9.oagr");
    let name = name.replacen("pulse-", "pulse-9", 1);
    std::fs::write(profile.auto().join(name), perturbed.to_bytes()).expect("writing it back");
    let before = profile.kept();
    let files_before = profile.replays();
    assert_eq!(files_before.len(), 2);

    // 3. The list shows the newest first; WATCH plays to the last agreeing second and stops.
    let shots = profile.dir.join("shots");
    std::fs::create_dir_all(&shots).expect("a directory");
    let (status, states) = profile.drive(
        &image,
        &format!(
            "{TO_THE_LIST} ticks 30; screenshot DIR/shots/list.png; select watch_replay; \
             press cross; until replay=playing within 4000; ticks 60; screenshot DIR/shots/playing.png; \
             until replay=stopped within 4000; ticks 10; screenshot DIR/shots/banner.png; \
             press left; ticks 5; screenshot DIR/shots/seek-back.png; quit"
        ),
        &[],
    );
    assert!(status.success());
    let stopped = states
        .iter()
        .find(|state| text(state, "replay") == Some("stopped"))
        .expect("the viewer stopped where the build stops matching");
    let at = text(stopped, "replay_at").expect("where");
    let parts: Vec<u64> = at.split('/').filter_map(|part| part.parse().ok()).collect();
    assert_eq!(parts.len(), 3, "{at}");
    assert_eq!(
        parts[0], parts[2],
        "it stopped at the end of what is built: {at}"
    );
    assert_eq!(
        parts[2], 1_500,
        "the last stored hash before tick {tick} agrees: {at}"
    );

    // 4. Watching changed nothing it must not.
    assert_eq!(
        profile.kept(),
        before,
        "records.toml, settings and ghosts are as they were"
    );
    assert_eq!(
        profile.replays(),
        files_before,
        "the viewer's race is not recorded"
    );
}

/// A race a `--race` run builds records nothing unless it asks (decision 1).
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn a_race_a_capture_builds_leaves_no_replay() {
    let Some(image) = pulse() else {
        return;
    };
    let profile = Profile::new("capture");
    let shot = profile.dir.join("shot.png");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args([
            "--no-audio",
            "--race",
            "--mode",
            "single_race",
            "--size",
            "320x200",
            "--ticks",
            "400",
        ])
        .arg("--screenshot")
        .arg(&shot)
        .env("XDG_CONFIG_HOME", profile.dir.join("config"))
        .env("XDG_DATA_HOME", profile.dir.join("data"))
        .env("XDG_STATE_HOME", profile.dir.join("state"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .expect("running oag-game");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(shot.exists());
    assert!(
        !profile.dir.join("data/oag/replays").exists(),
        "a capture records nothing: {:?}",
        profile.replays()
    );
}

/// A recording whose rebuild is not the recorded race is refused by name, in the list, and never
/// played wrong: the tick-zero hash is the judge (decision 7).
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn a_recording_this_build_cannot_rebuild_is_refused_by_name() {
    let Some(image) = pulse() else {
        return;
    };
    let profile = Profile::new("refused");
    let (status, _) = profile.drive(&image, &format!("{TO_THE_RACE} ticks 400; quit"), &[]);
    assert!(status.success());
    let files = profile.replays();
    assert_eq!(files.len(), 1, "{files:?}");
    let mut replay =
        oag_replay::Replay::from_bytes(&std::fs::read(&files[0]).expect("read")).expect("parse");
    // The same file asking for another race: the rebuild is not the recorded one.
    replay.header.setup.as_mut().expect("a setup").mode = "zone".to_string();
    std::fs::write(&files[0], replay.to_bytes()).expect("write");
    let shots = profile.dir.join("shots");
    std::fs::create_dir_all(&shots).expect("a directory");
    let (status, states) = profile.drive(
        &image,
        &format!(
            "{TO_THE_LIST} ticks 20; select watch_replay; press cross; ticks 600; \
             screenshot DIR/shots/refused.png; quit"
        ),
        &[],
    );
    assert!(status.success());
    assert!(
        states.iter().all(|state| text(state, "replay").is_none()),
        "the viewer never opened on a race that is not the recorded one"
    );
    let last = states.last().expect("a state");
    assert_eq!(
        text(last, "page"),
        Some("replays"),
        "back on the list it came from"
    );
    let row = &last["screen"]["rows"][0];
    let reason = row["values"][0]["unavailable"].as_str().unwrap_or_default();
    assert!(
        reason.starts_with("NOT THE RACE THAT WAS RECORDED"),
        "the row says why: {row}"
    );
}
