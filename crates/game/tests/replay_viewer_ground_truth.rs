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
//!   goes on in POSE VIEW on the recorded poses (`replay=pose_view`, then `pose_ended`), and the
//!   banner is on screen.
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
    oag_testdata::image("pulse-psp-usa.chd")
}

const TO_THE_RACE: &str = "boot; select racebox; press cross; until page=race; \
     select launch_race; press cross; until screen=picker; press cross; \
     until screen.id=ship-select; press cross; until stage=race within 4000; ";

/// [`TO_THE_RACE`] as a Single Race, so there are craft to follow.
const TO_THE_FIELD: &str = "boot; select racebox; press cross; until page=race; \
     set race.mode=single_race; \
     select launch_race; press cross; until screen=picker; press cross; \
     until screen.id=ship-select; press cross; until stage=race within 4000; ";

const TO_THE_LIST: &str = "boot; select replays; press cross; until page=replays; ";

fn text<'a>(state: &'a Value, key: &str) -> Option<&'a str> {
    state.get(key).and_then(Value::as_str)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
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
             until replay=pose_view within 4000; ticks 30; screenshot DIR/shots/pose-view.png; \
             until replay=pose_ended within 4000; ticks 10; screenshot DIR/shots/pose-end.png; \
             press left; ticks 5; screenshot DIR/shots/seek-back.png; quit"
        ),
        &[],
    );
    assert!(status.success());
    // The build stops matching at tick 1500: the viewer plays what agrees to there, then the
    // recorded poses to the end of the recording (the track is the whole run's).
    let first = states
        .iter()
        .find(|state| text(state, "replay") == Some("pose_view"))
        .expect("the viewer went on past the last agreeing second in POSE VIEW");
    let at: Vec<u64> = text(first, "replay_at")
        .expect("where")
        .split('/')
        .filter_map(|part| part.parse().ok())
        .collect();
    assert_eq!(at.len(), 3);
    assert_eq!(
        at[2], 1_500,
        "the last stored hash before tick {tick} agrees"
    );
    assert!(at[0] >= at[2], "pose view begins where what agrees ends");
    let last = states
        .iter()
        .rfind(|state| text(state, "replay") == Some("pose_ended"))
        .expect("and stops at the end of the pose track");
    let end: Vec<u64> = text(last, "replay_at")
        .expect("where")
        .split('/')
        .filter_map(|part| part.parse().ok())
        .collect();
    assert_eq!(end[0], end[1], "the track runs to the end of the recording");

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

/// One visit to photo mode over the viewer: the viewer's tick on the line before it, the ticks
/// the lines inside it report, and the lines after it.
struct Visit {
    before: (String, u64),
    inside: Vec<u64>,
    after: Vec<(String, u64)>,
}

/// The viewer's word and tick on a state line, when it is reporting one.
fn viewer_at(state: &Value) -> Option<(String, u64)> {
    let tick = text(state, "replay_at")?.split('/').next()?.parse().ok()?;
    Some((text(state, "replay")?.to_string(), tick))
}

/// Every run of lines with photo mode open, in order.
fn visits(states: &[Value]) -> Vec<Visit> {
    let open = |state: &Value| text(state, "photo").is_some_and(|lens| lens != "none");
    let mut found = Vec::new();
    let mut index = 0;
    while index < states.len() {
        if !open(&states[index]) || index == 0 {
            index += 1;
            continue;
        }
        let Some(before) = states[..index].iter().rev().find_map(viewer_at) else {
            index += 1;
            continue;
        };
        let end = states[index..]
            .iter()
            .position(|state| !open(state))
            .map_or(states.len(), |n| index + n);
        found.push(Visit {
            before,
            inside: states[index..end]
                .iter()
                .filter_map(viewer_at)
                .map(|(_, at)| at)
                .collect(),
            after: states[end..]
                .iter()
                .filter_map(viewer_at)
                .take(40)
                .collect(),
        });
        index = end;
    }
    found
}

/// ADR-0088: the viewer opens photo mode at the tick on screen; the mode's rewind row is a seek
/// of the viewer, so leaving continues the replay from the tick it was scrubbed to; and the mode's
/// PLAY row plays the recording on from the scrub.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
fn photo_mode_and_the_viewer_switch_both_ways_at_the_tick_the_player_chose() {
    let Some(image) = pulse() else {
        return;
    };
    let profile = Profile::new("photo-switch");
    let (status, _) = profile.drive(
        &image,
        &format!("{TO_THE_RACE} ticks 280; hold cross 1500; quit"),
        &[],
    );
    assert!(status.success());
    let shots = profile.dir.join("shots");
    std::fs::create_dir_all(&shots).expect("a directory");
    let down = "hold down 1; ticks 2; ";
    let to_the_rewind_row = format!("press start; {down}{down}{down}{down}");
    let (status, states) = profile.drive(
        &image,
        &format!(
            "{TO_THE_LIST} ticks 30; select watch_replay; press cross; \
             until replay=playing within 4000; ticks 400; screenshot DIR/shots/viewer.png; \
             press select; until photo=internal within 50; ticks 20; \
             {to_the_rewind_row} hold left 30; ticks 5; screenshot DIR/shots/photo-scrubbed.png; \
             press circle; until photo=none within 50; ticks 40; \
             screenshot DIR/shots/viewer-after.png; \
             press select; until photo=internal within 50; ticks 5; \
             {to_the_rewind_row} hold left 30; ticks 20; {down} {down} hold right 1; ticks 120; \
             press circle; until photo=none within 50; ticks 30; quit"
        ),
        &[],
    );
    assert!(status.success());
    let visits = visits(&states);
    assert_eq!(visits.len(), 2, "two visits to photo mode");

    // The viewer opens the mode at the tick on screen, and stands still under it until scrubbed.
    let first = &visits[0];
    assert_eq!(
        first.inside[0], first.before.1,
        "opened where the viewer stood"
    );
    assert_eq!(first.before.0, "playing");
    // The scrub: the row went back, and the viewer was sought there.
    let scrubbed = *first.inside.iter().min().expect("ticks");
    assert!(
        scrubbed + 100 < first.before.1,
        "the scrub went back from {} to {scrubbed}",
        first.before.1
    );
    assert_eq!(
        *first.inside.last().expect("ticks"),
        scrubbed,
        "it stood where it was scrubbed to"
    );
    // Leaving continues the replay from that tick, playing as it was.
    let resumed = &first.after;
    assert_eq!(resumed[0].1, scrubbed, "leaving continues at the scrub");
    assert!(
        resumed.iter().any(|(word, _)| word == "playing"),
        "and it plays on: {resumed:?}"
    );
    assert!(
        resumed.last().expect("ticks").1 > scrubbed + 10,
        "the playhead moved on from the scrub: {resumed:?}"
    );

    // The second visit scrubs back and plays from there: ticks under the mode go forward.
    let second = &visits[1];
    let back = *second.inside.iter().min().expect("ticks");
    let at = second
        .inside
        .iter()
        .position(|&tick| tick == back)
        .expect("a minimum");
    let played = &second.inside[at..];
    assert!(
        played.last().expect("ticks") > &(back + 30),
        "PLAY played on from the scrub at {back}: {played:?}"
    );
    assert!(
        played.windows(2).all(|pair| pair[1] >= pair[0]),
        "and only forward: {played:?}"
    );
}

/// A race a `--race` run builds records nothing unless it asks (decision 1).
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
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
/// The script waits for the load to start and to end rather than for a number of ticks: a load
/// on a busy machine takes as long as it takes.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
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
            "{TO_THE_LIST} ticks 20; select watch_replay; press cross; \
             until stage=loading within 600; until stage=menu page=replays within 600; \
             ticks 20; screenshot DIR/shots/refused.png; quit"
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

/// A campaign race recorded from the menus, as WATCH REPLAY at its end opens it, rebuilds from
/// its header and reproduces every stored hash; without its `campaign_skill` the field flies at
/// another tuning and the file stops matching within seconds (`docs/ui/replays.md`, "What the
/// header needed").
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
fn a_campaign_race_recorded_from_the_menus_reproduces() {
    let Some(image) = pulse() else {
        return;
    };
    let profile = Profile::new("campaign");
    // The campaign's first cell (a Single Race against the original driver), throttle for 25 s.
    let (status, states) = profile.drive(
        &image,
        "boot; until stage=menu within 3000; select campaign; press cross; until page=campaigns; \
         select campaign-original; press cross; until screen=campaign; \
         ticks 10; press cross; ticks 60; press cross; ticks 60; press cross; \
         until stage=race within 4000; ticks 280; hold cross 1500; quit",
        &[],
    );
    assert!(status.success());
    assert!(
        states.iter().any(|s| s["race"]["mode"] == "single_race"),
        "the cell raced"
    );
    let files = profile.replays();
    assert_eq!(files.len(), 1, "{files:?}");
    let replay =
        oag_replay::Replay::from_bytes(&std::fs::read(&files[0]).expect("read")).expect("parse");
    let options = &replay.header.options;
    assert_eq!(options.get("ai").map(String::as_str), Some("original"));
    assert!(
        options.contains_key("campaign_skill"),
        "the cell's scale is recorded: {options:?}"
    );
    assert_eq!(
        verify(&image, &replay),
        Ok(replay.ticks()),
        "the recording reproduces to its last tick"
    );

    let mut stripped = replay.clone();
    stripped.header.options.remove("campaign_skill");
    let desync = verify(&image, &stripped).expect_err("a rebuild without the cell's scale");
    eprintln!("without campaign_skill: desync at tick {}", desync.tick);
}

/// Rebuilds `replay` from its header over `image`, as WATCH does, and checks every stored hash.
pub(crate) fn verify(image: &Path, replay: &oag_replay::Replay) -> Result<u64, oag_replay::Desync> {
    let mut options =
        oag_raceplay::Options::for_replay(&replay.header, &image.display().to_string())
            .expect("the header rebuilds");
    // The packs the game mounted from the repository's `data/dlc`, as WATCH mounts the session's.
    options.dlc = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/dlc")];
    let loaded = oag_raceplay::load(&options).expect("loads");
    let mut race =
        oag_raceplay::replay::start_recorded(loaded.setup, &replay.header).expect("starts");
    oag_replay::Verifier::run(replay, race.sim.state_hash(), |events, inputs| {
        for event in events {
            race.apply_event(event).expect("applies");
        }
        race.tick(inputs);
        race.drain_cues();
        race.sim.state_hash()
    })
}

/// ADR-0088's binding plan over the viewer: R1 and L1 follow the next and previous craft
/// (wrapping), SQUARE hides the HUD, L2 and R2 scrub the playhead back and on and never change
/// the craft (under the default trigger mode a pulled trigger also raises its shoulder's button
/// in the snapshot), the right stick jogs it. None of it is the race's: the recording on disk is
/// not touched, and a tick that both runs reach hashes the same with and without the input.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
fn the_viewer_follows_craft_scrubs_and_hides_its_hud_on_the_new_keys() {
    let Some(image) = pulse() else {
        return;
    };
    let profile = Profile::new("bindings");
    let (status, _) = profile.drive(
        &image,
        &format!("{TO_THE_FIELD} ticks 280; hold cross 1500; quit"),
        &[],
    );
    assert!(status.success());
    let shots = profile.dir.join("shots");
    std::fs::create_dir_all(&shots).expect("a directory");
    let recorded = profile.replays();
    let bytes: Vec<Vec<u8>> = recorded
        .iter()
        .map(|path| std::fs::read(path).expect("the recording"))
        .collect();

    // The control: the replay played on, no input.
    let (status, control) = profile.drive(
        &image,
        &format!(
            "{TO_THE_LIST} ticks 30; select watch_replay; press cross; \
             until replay=playing within 4000; ticks 700; quit"
        ),
        &[],
    );
    assert!(status.success());

    let (status, states) = profile.drive(
        &image,
        &format!(
            "{TO_THE_LIST} ticks 30; select watch_replay; press cross; \
             until replay=playing within 4000; ticks 400; press cross; \
             until replay=paused within 50; ticks 5; screenshot DIR/shots/craft-0.png; \
             press r; ticks 5; screenshot DIR/shots/craft-1.png; press l; ticks 3; press l; ticks 3; \
             screenshot DIR/shots/craft-last.png; press r; ticks 3; \
             press square; ticks 5; screenshot DIR/shots/hud-off.png; press square; ticks 3; \
             pull l2 0.8 8; ticks 3; screenshot DIR/shots/scrubbed-back.png; \
             pull r2 0.8 4; ticks 3; rstick 1 0 6; ticks 3; rstick 0 1 6; ticks 3; \
             press cross; until replay=playing within 50; ticks 100; quit"
        ),
        &[],
    );
    assert!(status.success());

    // The craft: 0, then R1 to 1, L1 back to 0 and again to the last, R1 back to 0, and no
    // trigger or stick changed it after.
    let mut follows: Vec<u32> = states
        .iter()
        .filter_map(|state| text(state, "replay_follow")?.parse().ok())
        .collect();
    follows.dedup();
    assert!(
        follows.starts_with(&[0, 1, 0]) && follows.len() == 5 && follows[3] > 1 && follows[4] == 0,
        "L1 and R1 follow the previous and next craft, wrapping: {follows:?}"
    );

    // The HUD went off and on.
    let mut huds: Vec<&str> = states
        .iter()
        .filter_map(|state| text(state, "replay_hud"))
        .collect();
    huds.dedup();
    assert_eq!(huds, ["on", "off", "on"], "SQUARE turns the HUD off and on");

    // The playhead: L2 went back, R2 on, the right stick's horizontal axis on, its vertical axis
    // nowhere.
    let paused = states
        .iter()
        .position(|state| text(state, "replay") == Some("paused"))
        .expect("the viewer paused");
    let ats: Vec<u64> = states[paused..]
        .iter()
        .filter_map(viewer_at)
        .map(|(_, at)| at)
        .collect();
    let (start, lowest) = (ats[0], *ats.iter().min().expect("ticks"));
    assert!(
        lowest + 80 < start,
        "L2 scrubbed back from {start} to {lowest}: {ats:?}"
    );
    let back = ats.iter().position(|&at| at == lowest).expect("a minimum");
    let after = &ats[back..];
    assert!(
        after.iter().any(|&at| at > lowest + 40),
        "R2 scrubbed on: {after:?}"
    );

    // Nothing of it was the race's. The files are as they were, and a tick both runs reach has
    // the hash it has without the input.
    assert_eq!(
        profile
            .replays()
            .iter()
            .map(|path| std::fs::read(path).expect("the recording"))
            .collect::<Vec<_>>(),
        bytes,
        "watching with the new keys wrote to the recordings"
    );
    let held: BTreeMap<u64, String> = control
        .iter()
        .filter_map(|state| {
            Some((
                state.get("sim_tick")?.as_u64()?,
                text(state, "hash")?.to_string(),
            ))
        })
        .collect();
    let mut compared = 0;
    for state in &states {
        if let (Some(tick), Some(hash)) = (
            state.get("sim_tick").and_then(Value::as_u64),
            text(state, "hash"),
        ) && let Some(control) = held.get(&tick)
        {
            assert_eq!(hash, control, "tick {tick} hashes differently");
            compared += 1;
        }
    }
    assert!(compared > 300, "only {compared} ticks in common");
}
