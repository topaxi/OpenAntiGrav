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
            .args([
                "--no-audio",
                "--no-soundtrack",
                "--no-effects",
                "--no-intro",
                "--seed",
                "7",
                "--drive",
            ])
            .arg(script.replace("DIR", &self.dir.display().to_string()))
            .args(args)
            .arg("--state-log")
            .arg(&log)
            .env("XDG_CONFIG_HOME", self.dir.join("config"))
            .env("XDG_DATA_HOME", self.dir.join("data"))
            .env("XDG_STATE_HOME", self.dir.join("state"))
            .env("OAG_TRACKS", self.dir.join("no-tracks"))
            // No simulation mod: these races are the unmodded ones whatever mods the checkout has.
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
        // The race music's place moves wherever music plays, a replay being watched included
        // (`docs/architecture/race-music-resume.md`); it is not a record.
        out.retain(|path, _| path.file_name().is_none_or(|name| name != "music.toml"));
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
             press select; until photo=free within 50; ticks 20; \
             {to_the_rewind_row} hold left 30; ticks 5; screenshot DIR/shots/photo-scrubbed.png; \
             press circle; until photo=none within 50; ticks 40; \
             screenshot DIR/shots/viewer-after.png; \
             press select; until photo=free within 50; ticks 5; \
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

/// Leaving a recording lands on REPLAYS, and backing out of that is the front end's main page: the
/// title's pause screen (its RESTART RACE and QUIT RACE rows) is for a race that can be resumed,
/// and a watched one is never parked. Keyboard, pad and pointer each leave the viewer.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
fn leaving_a_recording_never_lands_on_the_pause_screen() {
    let Some(image) = pulse() else {
        return;
    };
    let profile = Profile::new("leave");
    let (status, _) = profile.drive(
        &image,
        &format!("{TO_THE_RACE} ticks 280; hold cross 600; quit"),
        &[],
    );
    assert!(status.success());
    let watch = "select watch_replay; press cross; until replay=playing within 4000; ticks 60; ";
    // The EXIT chip of the viewer's bar, at the window the drive opens (1440x816).
    let exit_chip = "mouse 1093 41; ticks 2; click; ";
    let back_out =
        "until page=replays; ticks 10; press circle; until page=main within 50; ticks 10; ";
    let (status, states) = profile.drive(
        &image,
        &format!(
            "{TO_THE_LIST} ticks 30; {watch} escape; {back_out} \
             select replays; press cross; until page=replays; ticks 10; {watch} press circle; {back_out} \
             select replays; press cross; until page=replays; ticks 10; {watch} {exit_chip} {back_out} quit"
        ),
        &[],
    );
    assert!(status.success());
    let mut pages: Vec<&str> = states
        .iter()
        .filter(|state| text(state, "stage") == Some("menu"))
        .filter_map(|state| text(state, "page"))
        .collect();
    pages.dedup();
    assert!(
        !pages.contains(&"pause"),
        "the pause screen was shown for a watched recording: {pages:?}"
    );
    let landed = pages.iter().filter(|page| **page == "main").count();
    assert_eq!(
        landed, 4,
        "the front end, then each of three ways out: {pages:?}"
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
            "--no-soundtrack",
            "--no-effects",
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
    // With a pose track the file would open in POSE VIEW instead of being refused.
    replay.pose_track = None;
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

/// HD's `EndRace Menu` offers WATCH REPLAY under VIEW RESULTS AGAIN (`docs/ui/replays.md`,
/// "The end-of-race row"): a time trial flown to its end, the pointer on the row and a click, the
/// race just finished plays in the viewer and leaving it lands on REPLAYS.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU"]
fn hd_end_of_race_menu_watches_the_race_just_finished() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let profile = Profile::new("hd-endrace-watch");
    let shots = profile.dir.join("shots");
    std::fs::create_dir_all(&shots).expect("a directory");
    // The fourth block's middle, at the window the drive opens (1440x816).
    let row = "mouse 476 307; ticks 2; click; ticks 2; ";
    let (status, states) = profile.drive(
        &image,
        &format!(
            "boot; select racebox; press cross; until page=race; set race.mode=time_trial; \
             select launch_race; press cross; until screen=picker; press cross; \
             until screen.id=ship-select; press cross; until stage=race within 4000; ticks 300; \
             until endrace=other within 20000; ticks 100; press cross; ticks 100; \
             screenshot DIR/shots/menu.png; {row}until replay=playing within 4000; ticks 60; \
             screenshot DIR/shots/playing.png; escape; until page=replays within 400; quit"
        ),
        &["--autopilot"],
    );
    assert!(status.success());
    assert_eq!(profile.replays().len(), 1, "the race just flown, in auto/");
    let last = states.last().expect("state lines");
    assert_eq!(text(last, "page"), Some("replays"));
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
        "boot; until stage=menu within 3000; select campaign; press cross; until screen=campaign; \
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
    let mut race = oag_raceplay::replay::start_recorded(
        loaded.setup,
        loaded.view,
        loaded.modules,
        &replay.header,
    )
    .expect("starts");
    oag_replay::Verifier::run(replay, race.core.sim.state_hash(), |events, inputs| {
        for event in events {
            race.core.sim.apply_event(event).expect("applies");
        }
        race.tick(inputs);
        race.core.sim.drain_cues();
        race.core.sim.state_hash()
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

impl Profile {
    /// A screenshot of a `--race` run on `image` with `args`, headless, written to `name.png` in
    /// this profile. `mods` leaves the checkout's default mods on (the examples' simulation
    /// modules run); otherwise none does.
    fn shoot(&self, image: &Path, args: &[&str], mods: bool, name: &str) -> PathBuf {
        let shot = self.dir.join(format!("{name}.png"));
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
        command
            .arg(image)
            .args([
                "--no-audio",
                "--no-soundtrack",
                "--no-effects",
                "--race",
                "--size",
                "320x180",
                "--screenshot",
            ])
            .arg(&shot)
            .args(args)
            .env("XDG_CONFIG_HOME", self.dir.join("config"))
            .env("XDG_DATA_HOME", self.dir.join("data"))
            .env("XDG_STATE_HOME", self.dir.join("state"))
            .env("OAG_TRACKS", self.dir.join("no-tracks"))
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY");
        if !mods {
            command.env("OAG_MODS", self.dir.join("no-mods"));
        }
        let output = command.output().expect("running oag-game");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        shot
    }
}

/// `png` without its `oaGs` chunk: the text says there is a stream, the file has none.
fn without_stream(png: &[u8]) -> Vec<u8> {
    let mut out = png[..8].to_vec();
    let mut at = 8;
    while at < png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().expect("a length")) as usize;
        let end = at + 12 + len;
        if &png[at + 4..at + 8] != b"oaGs" {
            out.extend_from_slice(&png[at..end]);
        }
        at = end;
    }
    out
}

fn notices(states: &[Value]) -> Vec<&str> {
    let mut seen: Vec<&str> = states
        .iter()
        .filter_map(|state| text(state, "notice"))
        .collect();
    seen.dedup();
    seen
}

const SHOT_ARGS: [&str; 7] = [
    "--mode",
    "single_race",
    "--class",
    "venom",
    "--hold",
    "cross",
    "--ticks",
];

/// A screenshot dropped on the main menu opens the viewer on its race, paused on the shot's tick
/// with the state hash the shot was taken at; every file that cannot open says why over the menu
/// and leaves it where it was; and a drop on a page below the main menu does nothing.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
fn a_dropped_screenshot_opens_the_viewer_on_its_tick_and_a_refused_one_says_why() {
    let Some(image) = pulse() else {
        return;
    };
    let profile = Profile::new("shot-drop");
    let tick = "300";
    let args: Vec<&str> = SHOT_ARGS.iter().copied().chain([tick]).collect();
    let plain = profile.shoot(&image, &args, false, "plain");
    let modded_args: Vec<&str> = args
        .iter()
        .copied()
        .chain(["--give", "extension"])
        .collect();
    let modded = profile.shoot(&image, &modded_args, true, "modded");
    let bytes = std::fs::read(&plain).expect("the shot");
    let found = oag_game::shot_meta::read(&bytes).expect("it reads back");
    let record = found.record.expect("a --race shot carries its stream");
    assert_eq!(record.tick, 300);
    let wanted_hash = format!("{:016x}", record.hash);
    // The same race without its stream, and a file that is not a PNG at all.
    std::fs::write(profile.dir.join("no-stream.png"), without_stream(&bytes)).expect("writing it");
    std::fs::write(profile.dir.join("not-a-shot.txt"), "hello").expect("writing it");
    // The same run as a replay file, which plays from the start where the shot opens paused.
    std::fs::write(profile.dir.join("cut.oagr"), record.replay.to_bytes()).expect("writing it");
    let modded_record = oag_game::shot_meta::read(&std::fs::read(&modded).expect("the shot"))
        .expect("it reads back")
        .record
        .expect("a stream");
    let modded_names: Vec<String> = modded_record
        .replay
        .header
        .mods
        .iter()
        .map(|used| used.id.clone())
        .collect();

    let shots = profile.dir.join("shots");
    std::fs::create_dir_all(&shots).expect("a directory");
    // Each refused drop is followed by `key`, which dismisses its notice (and is spent by it).
    let refused = |file: &str| {
        format!("drop DIR/{file}; ticks 3; screenshot DIR/shots/{file}.notice.png; key; ticks 3; ")
    };
    let mut script = format!(
        "boot; {}{}",
        refused("not-a-shot.txt"),
        refused("no-stream.png")
    );
    if !modded_names.is_empty() {
        script += &refused("modded.png");
    }
    // A page below the main menu ignores the drop.
    script += "select racebox; press cross; until page=race; drop DIR/plain.png; ticks 30; \
               expect stage=menu; expect page=race; press circle; until page=main within 100; ticks 5; ";
    // The main menu opens it; CROSS plays it from the start; CIRCLE leaves for the main menu.
    script += "drop DIR/plain.png; until stage=race within 4000; until replay=ended within 3000; \
               ticks 10; screenshot DIR/shots/at-the-shot.png; press cross; until replay=playing within 100; \
               ticks 30; press circle; until stage=menu within 400; expect page=main; \
               drop DIR/cut.oagr; until stage=race within 4000; until replay=playing within 3000; \
               press circle; until stage=menu within 400; expect page=main; quit";
    let (status, states) = profile.drive(&image, &script, &[]);
    assert!(status.success());

    let seen = notices(&states);
    let wants = if modded_names.is_empty() { 2 } else { 3 };
    assert_eq!(seen.len(), wants, "{seen:?}");
    assert!(seen[0].contains("NEITHER A REPLAY NOR"), "{}", seen[0]);
    assert!(seen[1].contains("INPUT STREAM"), "{}", seen[1]);
    if let Some(name) = modded_names.first() {
        assert!(
            seen[2].contains(name.as_str()),
            "the module is named: {}",
            seen[2]
        );
    }
    let at_shot = states
        .iter()
        .rfind(|state| text(state, "replay") == Some("ended"))
        .expect("the viewer opened");
    assert_eq!(
        text(at_shot, "replay_at"),
        Some("300/300/300"),
        "paused on the shot's tick"
    );
    assert_eq!(
        text(at_shot, "hash"),
        Some(wanted_hash.as_str()),
        "the race on screen is the shot's, to the bit"
    );
    assert!(
        states
            .iter()
            .any(|state| text(state, "replay") == Some("playing")),
        "CROSS plays it from the start"
    );
    let from_file = states
        .iter()
        .rfind(|state| text(state, "replay") == Some("playing"))
        .expect("a replay file dropped on the main menu plays");
    assert!(
        text(from_file, "replay_at")
            .is_some_and(|at| at.ends_with("/300/300") || at.contains("/300/")),
        "{from_file:?}"
    );
}

/// A drop in a race that is being driven is ignored, and one in an attract demo ends the demo and
/// opens the shot's viewer.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
fn a_dropped_screenshot_is_ignored_in_a_race_and_taken_in_a_demo() {
    let Some(image) = pulse() else {
        return;
    };
    let profile = Profile::new("shot-drop-race");
    let args: Vec<&str> = SHOT_ARGS.iter().copied().chain(["200"]).collect();
    let plain = profile.shoot(&image, &args, false, "plain");
    std::fs::write(profile.dir.join("not-a-shot.txt"), "hello").expect("writing it");
    let (status, states) = profile.drive(
        &image,
        &format!("{TO_THE_RACE} drop DIR/plain.png; ticks 60; expect stage=race; quit"),
        &[],
    );
    assert!(status.success());
    assert!(
        states
            .iter()
            .all(|state| text(state, "replay").is_none() && text(state, "notice").is_none()),
        "a race being driven never saw the drop"
    );
    let (status, states) = profile.drive(
        &image,
        "until demo=running within 8000; drop DIR/not-a-shot.txt; ticks 3; expect demo=running; \
         drop DIR/plain.png; until replay=ended within 4000; ticks 5; quit",
        &["--demo"],
    );
    assert!(status.success());
    // A refused drop leaves the demo running, with the band over it.
    let refused = states
        .iter()
        .find(|state| text(state, "notice").is_some())
        .expect("the refusal is shown over the demo");
    assert_eq!(text(refused, "demo"), Some("running"));
    assert!(text(refused, "notice").is_some_and(|n| n.contains("NEITHER A REPLAY NOR")));
    let at_shot = states
        .iter()
        .rfind(|state| text(state, "replay") == Some("ended"))
        .expect("the demo gave way to the viewer");
    assert_eq!(text(at_shot, "replay_at"), Some("200/200/200"));
    assert!(text(at_shot, "demo").is_none(), "no demo behind the viewer");
    let _ = plain;
}
