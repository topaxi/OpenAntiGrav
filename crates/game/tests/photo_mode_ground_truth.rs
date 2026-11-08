//! Photo mode against a real race: the simulation stands still through it, a saved picture is
//! the frame the player saw at its full size, and the pause menu offers it.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^photo_mode_ground_truth::/)'
//! ```
//!
//! Each test spawns the binary and drives it with a script (`--drive`), so it goes through the
//! same `Session` a window runs: the pause menu's row, the frame loop that does not tick the
//! race, the draw that saves the picture. What it reads is the game's own report (`--state-log`,
//! one JSON line per change, with the race's state hash and tick) and the files it wrote.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// A race on `image` as far as `race_prefix` goes, then `script`, in a profile of its own.
pub(crate) struct Run {
    pub(crate) status: std::process::ExitStatus,
    pub(crate) stderr: String,
    pub(crate) states: Vec<Value>,
    pub(crate) dir: PathBuf,
}

const TO_THE_RACE: &str = "boot; select racebox; press cross; until page=race; \
     select launch_race; press cross; until screen=picker; press cross; \
     until screen.id=ship-select; press cross; until stage=race within 4000; ticks 400; ";

/// [`TO_THE_RACE`] as a Single Race, eight craft; every race records every tick it plays (a rewind
/// plays the recording). Run it with `--autopilot` so the craft moves with nothing held on the pad: a held
/// button is read by the pad's own smoothing, which a visit's frames without a race tick do not
/// run down, and that is the pad and not the race.
const TO_THE_TIME_TRIAL: &str = "boot; select racebox; press cross; until page=race; \
     set race.mode=single_race; \
     select launch_race; press cross; until screen=picker; press cross; \
     until screen.id=ship-select; press cross; until stage=race within 4000; \
     ticks 400; ";

fn drive(image: &Path, tag: &str, script: &str, shots: &Path) -> Run {
    drive_with(image, tag, script, shots, &[])
}

/// [`drive`] with `args` added to the game's command line.
fn drive_with(image: &Path, tag: &str, script: &str, shots: &Path, args: &[&str]) -> Run {
    drive_binary(
        env!("CARGO_BIN_EXE_oag-game"),
        image,
        tag,
        script,
        shots,
        args,
    )
}

/// [`drive_with`] on the game binary `binary`: the module that names it is the one that spawns
/// the game, which is what puts it in the `gpu-vram` test group.
pub(crate) fn drive_binary(
    binary: &str,
    image: &Path,
    tag: &str,
    script: &str,
    shots: &Path,
    args: &[&str],
) -> Run {
    drive_binary_in(binary, image, tag, script, shots, args, false)
}

/// [`drive_with`] with no simulation mod mounted: a modded race records nothing (ADR-0087,
/// decision 7), so the rewind of a race that must play its recording is run this way.
fn drive_unmodded(image: &Path, tag: &str, script: &str, shots: &Path, args: &[&str]) -> Run {
    drive_binary_in(
        env!("CARGO_BIN_EXE_oag-game"),
        image,
        tag,
        script,
        shots,
        args,
        true,
    )
}

fn drive_binary_in(
    binary: &str,
    image: &Path,
    tag: &str,
    script: &str,
    shots: &Path,
    args: &[&str],
    unmodded: bool,
) -> Run {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("photo-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let log = dir.join("state.jsonl");
    let no_mods = dir.join("no-mods");
    if unmodded {
        std::fs::create_dir_all(&no_mods).expect("a scratch directory");
    }
    let mut command = std::process::Command::new(binary);
    if unmodded {
        command.env("OAG_MODS", &no_mods);
    }
    let output = command
        .arg(image)
        // One seed, or each launch deals its own weapons and the two races differ from tick 1.
        .args(["--no-audio", "--no-intro", "--seed", "7", "--drive"])
        .arg(script.replace("SHOTS", &shots.display().to_string()))
        .args(args)
        .arg("--state-log")
        .arg(&log)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("OAG_TRACKS", dir.join("no-tracks"))
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
    Run {
        status: output.status,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        states,
        dir,
    }
}

pub(crate) fn pulse() -> Option<PathBuf> {
    oag_testdata::image("pulse-psp-eu.chd")
}

/// Every `(simulation tick, hash)` the race reported, and the lens it was reported under.
fn ticks(run: &Run) -> Vec<(u64, String, String)> {
    run.states
        .iter()
        .filter_map(|state| {
            Some((
                state.get("sim_tick")?.as_u64()?,
                state.get("hash")?.as_str()?.to_string(),
                state.get("photo")?.as_str()?.to_string(),
            ))
        })
        .collect()
}

fn the_photos(run: &Run) -> Vec<PathBuf> {
    let dir = run.dir.join("data/oag/photos");
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| entries.filter_map(|e| Some(e.ok()?.path())).collect())
        .unwrap_or_default();
    // `oag-<title>-<seconds>-<n>.png`: by the two numbers, so `-10` follows `-9`.
    let key = |path: &PathBuf| {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let mut parts = stem.rsplit('-');
        let n: u64 = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let seconds: u64 = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        (seconds, n)
    };
    found.sort_by_key(key);
    found
}

/// Decodes a PNG to `(width, height, rgba)`.
fn decode(path: &Path) -> (u32, u32, Vec<u8>) {
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("a PNG");
    let mut pixels = vec![0; reader.output_buffer_size().expect("a buffer size")];
    let info = reader.next_frame(&mut pixels).expect("a frame");
    pixels.truncate(info.buffer_size());
    assert_eq!(info.color_type, png::ColorType::Rgba, "{}", path.display());
    (info.width, info.height, pixels)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_race_through_photo_mode_hashes_the_same_at_every_tick_as_one_without() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-hash-shots");
    let control = drive(
        &image,
        "hash-control",
        &format!("{TO_THE_RACE} ticks 500"),
        &shots,
    );
    assert!(control.status.success(), "{}", control.stderr);
    let photographed = drive(
        &image,
        "hash-photo",
        &format!(
            "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; press select; ticks 5; hold left 60; \
             hold select up 30; press square; ticks 5; \
             press start; hold right 30; hold down 1; ticks 2; hold right 30; hold down 1; ticks 2; \
             hold right 15; hold down 1; ticks 2; hold right 1; hold down 1; ticks 2; \
             hold right 15; ticks 10; \
             press start; press start; press cross; ticks 5; \
             screenshot SHOTS/during.png; press circle; until photo=none within 50; ticks 500; \
             screenshot SHOTS/end-photo.png"
        ),
        &shots,
    );
    assert!(photographed.status.success(), "{}", photographed.stderr);
    assert!(
        photographed.states.iter().any(|state| state
            .get("photo_fx")
            .and_then(Value::as_str)
            .is_some_and(|fx| {
                fx.contains("dof=on")
                    && !fx.contains("speed_blur=0 ")
                    && !fx.contains("exposure=33 ")
            })),
        "the run never moved the effects"
    );

    let held: BTreeMap<u64, String> = ticks(&control)
        .into_iter()
        .map(|(tick, hash, _)| (tick, hash))
        .collect();
    let seen = ticks(&photographed);
    assert!(
        seen.iter().any(|(_, _, lens)| lens == "close"),
        "the run never reached the close lens: {:?}",
        seen.iter()
            .map(|(_, _, lens)| lens.as_str())
            .collect::<Vec<_>>()
    );
    let mut compared = 0;
    for (tick, hash, _) in &seen {
        if let Some(control) = held.get(tick) {
            assert_eq!(
                hash, control,
                "tick {tick} hashes differently after photo mode"
            );
            compared += 1;
        }
    }
    assert!(compared > 400, "only {compared} ticks in common");
    // While the mode is open the race does not move: every report under a lens is one tick.
    let under: Vec<u64> = seen
        .iter()
        .filter(|(_, _, lens)| lens != "none")
        .map(|(tick, _, _)| *tick)
        .collect();
    assert!(!under.is_empty());
    assert!(
        under.windows(2).all(|pair| pair[0] == pair[1]),
        "the simulation ticked inside photo mode: {under:?}"
    );
    assert_eq!(the_photos(&photographed).len(), 1, "one shot, one file");
    // The effects went with the mode: after leaving, the picture is the one a visit that moved
    // nothing leaves, at the same tick.
    let plain = drive(
        &image,
        "hash-plain",
        &format!(
            "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; press select; ticks 5; hold left 60; \
             hold select up 30; press square; ticks 5; press cross; ticks 5; \
             screenshot SHOTS/during-plain.png; press circle; until photo=none within 50; \
             ticks 500; screenshot SHOTS/end-plain.png"
        ),
        &shots,
    );
    assert!(plain.status.success(), "{}", plain.stderr);
    let (left, right) = (
        decode(&shots.join("end-plain.png")),
        decode(&shots.join("end-photo.png")),
    );
    assert!(
        left.2 == right.2,
        "the picture after leaving carries what the mode did to it (differs by {})",
        difference(&left, &right, ALL)
    );
    a_visit_that_rewinds_leaves_every_live_tick_as_it_was(&image, &shots);
}

/// ADR-0087, decision 5: a visit that rewinds. On a Single Race, whose recording the rewind plays, the mode goes back
/// to the start of the race, forward again, takes a shot of the moment it stands at and goes
/// back once more before leaving. Every tick it reports (the rewound ones too, each a state the
/// recording replays) hashes as the same tick of a race that was never visited, and the race
/// resumes at the tick it left.
fn a_visit_that_rewinds_leaves_every_live_tick_as_it_was(image: &Path, shots: &Path) {
    let control = drive_with(
        image,
        "hash-control-tt",
        &format!("{TO_THE_TIME_TRIAL} ticks 600"),
        shots,
        &["--autopilot"],
    );
    assert!(control.status.success(), "{}", control.stderr);
    let rewound = drive_with(
        image,
        "hash-rewind",
        &format!(
            "{TO_THE_TIME_TRIAL} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; press start; \
             hold down 1; ticks 2; hold down 1; ticks 2; hold down 1; ticks 2; hold down 1; ticks 2; \
             hold left 90; ticks 5; hold right 20; ticks 5; press cross; ticks 5; \
             screenshot SHOTS/rewound.png; hold left 90; ticks 60; \
             escape; until photo=none within 50; ticks 600"
        ),
        shots,
        &["--autopilot"],
    );
    assert!(rewound.status.success(), "{}", rewound.stderr);
    let held: BTreeMap<u64, String> = ticks(&control)
        .into_iter()
        .map(|(tick, hash, _)| (tick, hash))
        .collect();
    let seen = ticks(&rewound);
    let mut compared = 0;
    for (tick, hash, _) in &seen {
        // The control's log starts after the first tick it steps, so the state before it (the
        // rewind goes back to it) has nothing to be compared with.
        if let Some(control) = held.get(tick) {
            assert_eq!(
                hash, control,
                "tick {tick} hashes differently after a rewind"
            );
            compared += 1;
        }
    }
    assert!(compared > 600, "only {compared} ticks compared");
    let under: Vec<u64> = seen
        .iter()
        .filter(|(_, _, lens)| lens != "none")
        .map(|(tick, _, _)| *tick)
        .collect();
    let live = *under.first().expect("the mode opened");
    let lowest = *under.iter().min().expect("the mode opened");
    assert!(
        under.iter().all(|tick| *tick <= live),
        "the visit went past the live tick {live}: {under:?}"
    );
    assert!(
        lowest + 200 < live,
        "the rewind never went back: lowest tick {lowest}, live {live}"
    );
    let after: Vec<u64> = seen
        .iter()
        .skip_while(|(_, _, lens)| lens == "none")
        .skip_while(|(_, _, lens)| lens != "none")
        .map(|(tick, _, _)| *tick)
        .collect();
    // The first line after the mode is the live tick again, or the tick after it when the
    // report for the last frame of the mode and the first tick of the race share a line.
    assert!(
        after
            .first()
            .is_some_and(|tick| (live..=live + 1).contains(tick)),
        "leaving puts the race back at the tick it left: {:?}",
        &after[..after.len().min(4)]
    );
    assert!(after.contains(&(live + 1)), "and it goes on from there");
    assert_eq!(the_photos(&rewound).len(), 1, "one shot, one file");
}

/// A moving race (an autopilot Single Race) that visits photo mode and leaves by `leave` hashes at
/// every tick after it as one that never visited: the button that left the mode is not a press
/// the race reads. Returns how many ticks it compared.
fn leaving_by(image: &Path, tag: &str, mode: &str, leave: &str) -> usize {
    let to_the_race = TO_THE_TIME_TRIAL.replace("single_race", mode);
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("photo-{tag}-shots"));
    let control = drive_with(
        image,
        &format!("{tag}-control"),
        &format!("{to_the_race} ticks 1000"),
        &shots,
        &["--autopilot"],
    );
    assert!(control.status.success(), "{}", control.stderr);
    let visited = drive_with(
        image,
        tag,
        &format!(
            "{to_the_race} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; ticks 15; {leave}; until photo=none within 50; \
             ticks 600"
        ),
        &shots,
        &["--autopilot"],
    );
    assert!(visited.status.success(), "{}", visited.stderr);
    let held: BTreeMap<u64, String> = ticks(&control)
        .into_iter()
        .map(|(tick, hash, _)| (tick, hash))
        .collect();
    let mut compared = 0;
    for (tick, hash, lens) in ticks(&visited) {
        if lens != "none" {
            continue;
        }
        if let Some(control) = held.get(&tick) {
            assert_eq!(
                hash, *control,
                "tick {tick} hashes differently after leaving by `{leave}`"
            );
            compared += 1;
        }
    }
    compared
}

/// The CIRCLE that leaves the mode is spent there: it does not reach the race as the absorb
/// press it is on a race that runs, whether it is tapped or held as a thumb holds it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn leaving_photo_mode_with_circle_hands_the_race_no_press() {
    let Some(image) = pulse() else {
        return;
    };
    for (tag, mode, leave) in [
        ("circle-tap", "single_race", "press circle"),
        ("circle-hold", "single_race", "hold circle 6"),
        ("circle-tap-tt", "time_trial", "press circle"),
        ("circle-hold-tt", "time_trial", "hold circle 6"),
    ] {
        let compared = leaving_by(&image, tag, mode, leave);
        assert!(compared > 400, "{tag}: only {compared} ticks compared");
    }
}

/// ADR-0088: over a live race the mode's PLAY row plays the race's own recording on from the tick the
/// rewind row was scrubbed to, to the live tick, and leaving puts the live race back to the bit:
/// every live tick hashes as the same tick of a race that never visited.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_live_race_played_from_a_scrub_and_left_hashes_as_it_was() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-play-shots");
    // The throttle is held so the craft moves and every hash is sensitive. No mod is mounted
    // (`drive_unmodded`): a modded race records nothing, and a rewind of a race that records
    // nothing stops at whole keyframes, so it would never play the recording
    // (`docs/ui/photo-mode.md`, "Not done, named").
    let to_the_throttle = TO_THE_TIME_TRIAL.replace("ticks 400; ", "hold cross 400; ");
    let control = drive_unmodded(
        &image,
        "play-control",
        &format!("{to_the_throttle} ticks 1000"),
        &shots,
        &[],
    );
    assert!(control.status.success(), "{}", control.stderr);
    let down = "hold down 1; ticks 2; ";
    let played = drive_unmodded(
        &image,
        "play",
        &format!(
            "{to_the_throttle} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; press start; {down}{down}{down}{down} \
             hold left 25; ticks 40; {down} {down} hold right 2; ticks 100; \
             screenshot SHOTS/playing.png; escape; until photo=none within 50; ticks 500"
        ),
        &shots,
        &[],
    );
    assert!(played.status.success(), "{}", played.stderr);
    let held: BTreeMap<u64, String> = ticks(&control)
        .into_iter()
        .map(|(tick, hash, _)| (tick, hash))
        .collect();
    let seen = ticks(&played);
    let mut compared = 0;
    for (tick, hash, _) in &seen {
        if let Some(control) = held.get(tick) {
            assert_eq!(
                hash, control,
                "tick {tick} hashes differently after playing"
            );
            compared += 1;
        }
    }
    assert!(compared > 500, "only {compared} ticks compared");
    let under: Vec<u64> = seen
        .iter()
        .filter(|(_, _, lens)| lens != "none")
        .map(|(tick, _, _)| *tick)
        .collect();
    let live = *under.first().expect("the mode opened");
    let lowest = *under.iter().min().expect("the mode opened");
    assert!(lowest + 100 < live, "the rewind never went back");
    // A landing whose pre-roll starts from the first keyframe does not hash as the live run did;
    // the scrub here stays out of it (`docs/ui/photo-mode.md`, "Not done, named").
    assert!(lowest > 130, "the scrub went to {lowest}");
    assert!(
        under.iter().all(|tick| *tick <= live),
        "past the live tick: {under:?}"
    );
    // From the lowest tick the race on screen only goes forward, a tick at a time.
    let from = under
        .iter()
        .position(|tick| *tick == lowest)
        .expect("a minimum");
    let forward = &under[from..];
    assert!(
        forward
            .windows(2)
            .all(|pair| pair[1] >= pair[0] && pair[1] - pair[0] <= 1),
        "PLAY plays tick by tick: {forward:?}"
    );
    assert!(
        *forward.last().expect("ticks") > lowest + 30,
        "PLAY played on from the scrub: {forward:?}"
    );
    assert!(
        played.states.iter().any(|state| state
            .get("photo_fx")
            .and_then(Value::as_str)
            .is_some_and(|fx| fx.contains("play=1"))),
        "the PLAY row never went on"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_saved_picture_is_the_frame_the_player_saw_at_its_full_size() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-pixels-shots");
    let _ = std::fs::remove_dir_all(&shots);
    // The camera is moved off the craft. The reference is a screenshot with the help hidden and the
    // HUD off. The first shot is taken with the help up, so the screen's text would show in it if
    // it were drawn into the picture; the second with the HUD row on, so the HUD would show in it.
    let run = drive(
        &image,
        "pixels",
        &format!(
            "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; press select; ticks 5; hold right 45; \
             press start; press start; ticks 10; screenshot SHOTS/bare.png; press start; ticks 5; \
             press cross; ticks 3; screenshot SHOTS/with-help.png; \
             press square; ticks 5; press cross; ticks 3; screenshot SHOTS/with-hud.png; \
             press circle; until photo=none within 50"
        ),
        &shots,
    );
    assert!(run.status.success(), "{}", run.stderr);
    let bare = decode(&shots.join("bare.png"));
    let with_help = decode(&shots.join("with-help.png"));
    assert_ne!(
        bare.2, with_help.2,
        "the help is on screen in the second frame"
    );
    let photos = the_photos(&run);
    assert_eq!(photos.len(), 2, "{photos:?}");
    let (plain, hud) = (decode(&photos[0]), decode(&photos[1]));
    assert_eq!(
        (plain.0, plain.1),
        (bare.0, bare.1),
        "the render's own size"
    );
    assert!(
        plain.0 >= 1280,
        "a picture at the size a player sees, not the PSP's 480"
    );
    assert_eq!(
        plain.2, bare.2,
        "with the help up, the picture saved is still the frame seen without it"
    );
    assert_ne!(
        hud.2, plain.2,
        "with the HUD row on the HUD is in the picture"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn escape_leaves_the_mode_and_the_next_one_opens_the_menus_and_the_race_still_runs() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-escape-shots");
    let run = drive(
        &image,
        "escape",
        &format!(
            "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; ticks 5; escape; expect stage=race photo=none; \
             ticks 60; escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; press circle; until photo=none within 50; ticks 60"
        ),
        &shots,
    );
    assert!(run.status.success(), "{}", run.stderr);
    let seen = ticks(&run);
    let last_under = seen
        .iter()
        .rposition(|(_, _, lens)| lens != "none")
        .unwrap();
    assert!(
        seen[last_under + 1..].len() >= 60,
        "the race runs again after the second mode: {} ticks",
        seen.len() - last_under - 1
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn select_on_race_end_photo_opens_the_mode_and_leaving_returns_to_it() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-finish-shots");
    // The player is flown for them, so the race reaches its line and `Race End Photo`.
    let run = drive_with(
        &image,
        "finish",
        &format!(
            "{TO_THE_RACE} ticks 8000; until endrace=photo within 2000; ticks 100; \
             press select; until photo=internal within 50; ticks 20; \
             press circle; until photo=none within 50; ticks 5; expect endrace=photo"
        ),
        &shots,
        &["--autopilot"],
    );
    assert!(
        run.status.success(),
        "{}\n{:?}",
        run.stderr,
        run.states.last()
    );
    let seen = ticks(&run);
    let under: Vec<u64> = seen
        .iter()
        .filter(|(_, _, lens)| lens != "none")
        .map(|(tick, _, _)| *tick)
        .collect();
    assert!(!under.is_empty(), "SELECT never opened the mode");
    assert!(
        under.windows(2).all(|pair| pair[0] == pair[1]),
        "the race ran on behind the mode: {under:?}"
    );
}

/// Every disc word a title's photo mode takes is in that disc's own string table: a mapped id the
/// table lacks would silently fall back to ours and leave the mapping untested.
#[test]
#[ignore = "needs disc images in data/images/"]
fn every_disc_word_a_title_maps_is_in_its_string_table() {
    use oag_title::photo::Row;
    let mut seen = 0;
    for name in [
        "pulse-psp-eu.chd",
        "hdfury-ps3-eu-dec.iso",
        "omega-ps4-eu.pkg",
    ] {
        let Some(image) = oag_testdata::image(name) else {
            continue;
        };
        let options = oag_game::boot::Options {
            language: None,
            source: image.display().to_string(),
            dlc: Vec::new(),
            leg: oag_ui::frontend::Leg::LogoFmv,
            movie: None,
            cache: std::env::temp_dir().join("oag-photo-strings-ground-truth"),
            audio_cache: oag_source::cache::default_audio_cache_dir(),
            extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
            no_video: true,
            refresh_video: false,
            prefer_av1_cache: false,
        };
        let (shell, _archives, title) =
            oag_game::boot::load_shell(&options).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        for row in [
            Row::Camera,
            Row::Hud,
            Row::Help,
            Row::Rotate,
            Row::Pan,
            Row::Reset,
            Row::Ship,
            Row::SpeedBlur,
            Row::Depth,
        ] {
            let Some(id) = title.photo.disc_id(row) else {
                continue;
            };
            let text = shell
                .strings
                .get(id)
                .unwrap_or_else(|| panic!("{name}: {id} is not in the string table"));
            assert!(!text.trim().is_empty(), "{name}: {id} is empty");
            println!("{name}: {row:?} = {id} = {text:?}");
            seen += 1;
        }
    }
    assert!(seen > 0 || oag_testdata::image("pulse-psp-eu.chd").is_none());
}

/// Mean of `f` over the pixels of the rectangle `(x0, y0, x1, y1)`, in fractions of the picture.
fn over(
    picture: &(u32, u32, Vec<u8>),
    (x0, y0, x1, y1): (f32, f32, f32, f32),
    f: impl Fn(&[u8]) -> f32,
) -> f32 {
    let (w, h, pixels) = picture;
    let (xs, xe) = ((x0 * *w as f32) as u32, (x1 * *w as f32) as u32);
    let (ys, ye) = ((y0 * *h as f32) as u32, (y1 * *h as f32) as u32);
    let mut sum = 0.0;
    let mut count = 0.0;
    for y in ys..ye {
        for x in xs..xe {
            let at = ((y * w + x) * 4) as usize;
            sum += f(&pixels[at..at + 4]);
            count += 1.0;
        }
    }
    sum / count
}

const ALL: (f32, f32, f32, f32) = (0.0, 0.0, 1.0, 1.0);

fn luma(p: &[u8]) -> f32 {
    0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2])
}

fn chroma(p: &[u8]) -> f32 {
    f32::from(p[..3].iter().max().copied().unwrap_or(0) - p[..3].iter().min().copied().unwrap_or(0))
}

/// Mean absolute difference between two pictures of one size over a region.
fn difference(
    a: &(u32, u32, Vec<u8>),
    b: &(u32, u32, Vec<u8>),
    region: (f32, f32, f32, f32),
) -> f32 {
    assert_eq!((a.0, a.1), (b.0, b.1));
    let (w, h) = (a.0, a.1);
    let mut diff = vec![0u8; a.2.len()];
    for (i, d) in diff.iter_mut().enumerate() {
        *d = a.2[i].abs_diff(b.2[i]);
    }
    over(&(w, h, diff), region, |p| {
        (f32::from(p[0]) + f32::from(p[1]) + f32::from(p[2])) / 3.0
    })
}

/// How much fine detail a region holds: the mean absolute step between neighbouring pixels.
fn detail(picture: &(u32, u32, Vec<u8>), region: (f32, f32, f32, f32)) -> f32 {
    let (w, h, pixels) = picture;
    let (xs, xe) = ((region.0 * *w as f32) as u32, (region.2 * *w as f32) as u32);
    let (ys, ye) = ((region.1 * *h as f32) as u32, (region.3 * *h as f32) as u32);
    let mut sum = 0.0;
    let mut count = 0.0;
    for y in ys..ye {
        for x in xs..xe - 1 {
            let at = ((y * w + x) * 4) as usize;
            let next = at + 4;
            sum += f32::from(pixels[at].abs_diff(pixels[next]))
                + f32::from(pixels[at + 1].abs_diff(pixels[next + 1]))
                + f32::from(pixels[at + 2].abs_diff(pixels[next + 2]));
            count += 3.0;
        }
    }
    sum / count
}

/// The edge of the picture, where the speed streak is, and the middle, where it is not.
const EDGE: (f32, f32, f32, f32) = (0.0, 0.0, 0.12, 1.0);
const MIDDLE: (f32, f32, f32, f32) = (0.40, 0.30, 0.60, 0.50);
/// The far field up the left of a `close` shot, and the craft's own body.
const FAR_FIELD: (f32, f32, f32, f32) = (0.02, 0.10, 0.38, 0.55);
const CRAFT: (f32, f32, f32, f32) = (0.44, 0.70, 0.56, 0.82);

/// Each picture control, moved in turn, changes the saved file in the direction it names, and put
/// back where it opened gives the file the mode saved before anything was touched, byte for byte.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn each_picture_control_changes_the_saved_shot_and_at_neutral_it_is_the_unprocessed_one() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-effects-shots");
    let _ = std::fs::remove_dir_all(&shots);
    // Rows: exposure, saturation, speed blur, depth of field. 72 ticks of a held push is 21 steps
    // (it fires on the first tick, again after 14 and then every 3), 69 is 20, 39 is 10 and 1 is
    // one. A shot after each move (the drive draws a frame only when it is asked for a screenshot, so each
    // shot is asked for one); the race stands still, so every file is one frame's picture.
    let run = drive(
        &image,
        "effects",
        &format!(
            "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; press select; ticks 20; press cross; ticks 2; screenshot SHOTS/frame-1.png; ticks 2; \
             press start; hold right 72; ticks 3; press cross; ticks 2; screenshot SHOTS/frame-2.png; ticks 2; \
             hold left 72; ticks 3; press cross; ticks 2; screenshot SHOTS/frame-3.png; ticks 2; \
             hold down 1; ticks 2; hold left 69; ticks 3; press cross; ticks 2; screenshot SHOTS/frame-4.png; ticks 2; \
             hold right 69; ticks 3; press cross; ticks 2; screenshot SHOTS/frame-5.png; ticks 2; \
             hold down 1; ticks 2; hold right 39; ticks 40; press cross; ticks 2; screenshot SHOTS/frame-6.png; ticks 2; \
             hold left 39; ticks 10; press cross; ticks 2; screenshot SHOTS/frame-7.png; ticks 2; \
             hold down 1; ticks 2; hold right 1; ticks 8; press cross; ticks 2; screenshot SHOTS/frame-8.png; ticks 2; \
             hold left 1; ticks 8; press cross; ticks 2; screenshot SHOTS/frame-9.png; ticks 2; \
             press circle; until photo=none within 50"
        ),
        &shots,
    );
    assert!(run.status.success(), "{}", run.stderr);
    let photos = the_photos(&run);
    assert_eq!(photos.len(), 9, "{photos:?}");
    let [
        neutral,
        bright,
        exposure_back,
        grey,
        saturation_back,
        speed,
        speed_back,
        dof,
        dof_back,
    ] = <[_; 9]>::try_from(photos.iter().map(|p| decode(p)).collect::<Vec<_>>())
        .unwrap_or_else(|_| unreachable!());

    // Back where it opened, the file is the unprocessed one.
    for (name, back) in [
        ("exposure", &exposure_back),
        ("saturation", &saturation_back),
        ("speed blur", &speed_back),
        ("depth of field", &dof_back),
    ] {
        assert!(
            back.2 == neutral.2,
            "{name} put back is not the picture the mode saved before it was touched"
        );
    }

    // Exposure up is brighter.
    let (before, after) = (over(&neutral, ALL, luma), over(&bright, ALL, luma));
    assert!(
        after > before + 3.0,
        "exposure up: mean luma {before} -> {after}"
    );
    // Saturation at zero is grey.
    assert!(
        over(&grey, ALL, chroma) < 0.1 * over(&neutral, ALL, chroma).max(1.0),
        "saturation 0 left colour in the picture"
    );
    // Speed blur lands at the edge of the picture and leaves the middle alone.
    assert!(
        difference(&neutral, &speed, EDGE) > 2.0,
        "no streak at the edge"
    );
    assert!(
        difference(&neutral, &speed, MIDDLE) < 1.0,
        "the speed blur reached the middle of the picture"
    );
    // Depth of field blurs the far field and leaves the craft sharp.
    let (sharp, soft) = (detail(&neutral, FAR_FIELD), detail(&dof, FAR_FIELD));
    assert!(soft < 0.75 * sharp, "far-field detail {sharp} -> {soft}");
    assert!(
        difference(&neutral, &dof, CRAFT) < 4.0,
        "the craft in focus changed: {}",
        difference(&neutral, &dof, CRAFT)
    );
}

/// The game's own motion blur is a setting the mode overrides: a saved shot is the same file
/// with the blur on `high` as with it `off`, a camera flown about the craft included.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_games_motion_blur_never_reaches_a_photo() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-blur-shots");
    let _ = std::fs::remove_dir_all(&shots);
    let script = format!(
        "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
         until photo=internal within 200; press select; ticks 5; hold left 40; hold up 1; \
         ticks 2; press cross; ticks 2; screenshot SHOTS/frame.png; ticks 2; \
         press circle; until photo=none within 50"
    );
    let mut files = Vec::new();
    for (tag, level) in [("blur-off", "off"), ("blur-high", "high")] {
        let run = drive_with(&image, tag, &script, &shots, &["--motion-blur", level]);
        assert!(run.status.success(), "{}", run.stderr);
        let photos = the_photos(&run);
        assert_eq!(photos.len(), 1, "{photos:?}");
        files.push(decode(&photos[0]));
    }
    assert!(
        files[0].2 == files[1].2,
        "the motion blur reached a photo (differs by {})",
        difference(&files[0], &files[1], ALL)
    );
}

/// HD's motion blur (ADR-0087, decision 5, stage 6): a shot of 120 frames of a race that has not
/// moved, over the recorded ticks before it. A track blur smears the scenery and leaves the craft
/// the camera rides sharp; a ship blur holds the camera and smears the craft; any input while the
/// frames are drawn abandons the shot and writes nothing.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_motion_blur_shot_smears_what_it_should_and_any_input_abandons_it() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-mblur-shots");
    let _ = std::fs::remove_dir_all(&shots);
    let to_the_race = "boot; select racebox; press cross; until page=race; \
         set race.mode=time_trial; select launch_race; press cross; until screen=picker; \
         press cross; until screen.id=ship-select; press cross; until stage=race within 4000; \
         ticks 300; hold cross 400; escape; until stage=menu; select photo_mode; press cross; \
         until photo=internal within 200; press select; ticks 5; hold left 8; ticks 3; ";
    let down = "hold down 1; ticks 2; ";
    // The plain shot, a track blur at full shutter, and a ship blur, all at the one tick.
    let run = drive(
        &image,
        "mblur",
        &format!(
            "{to_the_race} press cross; ticks 3; screenshot SHOTS/plain.png; press start; \
             {down}{down}{down}{down} hold right 1; ticks 2; {down} hold right 260; ticks 3; \
             press cross; ticks 3; until photo_job=none within 800; ticks 3; \
             hold up 1; ticks 3; hold right 1; ticks 3; press cross; ticks 3; \
             until photo_job=none within 800; ticks 3"
        ),
        &shots,
    );
    assert!(run.status.success(), "{}", run.stderr);
    let photos = the_photos(&run);
    assert_eq!(
        photos.len(),
        3,
        "a plain shot, a track blur and a ship blur: {photos:?}"
    );
    let [plain, track, ship] = [0, 1, 2].map(|i| decode(&photos[i]));
    assert_eq!((plain.0, plain.1), (track.0, track.1));
    // Where the craft's hull sits in the close lens turned by `hold left 8`, and a stretch of
    // scenery well clear of it. What a blur takes away is detail: a smear lowers the picture's
    // gradient along the way it moves.
    const HULL: (f32, f32, f32, f32) = (0.40, 0.68, 0.52, 0.78);
    const SCENERY: (f32, f32, f32, f32) = (0.0, 0.1, 0.3, 0.5);
    let kept =
        |blurred: &(u32, u32, Vec<u8>), region| detail(blurred, region) / detail(&plain, region);
    println!(
        "detail kept, track blur: hull {:.2} scenery {:.2}; ship blur: hull {:.2} scenery {:.2}",
        kept(&track, HULL),
        kept(&track, SCENERY),
        kept(&ship, HULL),
        kept(&ship, SCENERY)
    );
    assert!(
        kept(&track, SCENERY) < 0.6,
        "a track blur left the scenery sharp: {}",
        kept(&track, SCENERY)
    );
    assert!(
        kept(&track, HULL) > 0.8,
        "a track blur smeared the craft the camera rides: {}",
        kept(&track, HULL)
    );
    assert!(
        kept(&ship, HULL) < 0.8,
        "a ship blur left the craft sharp: {}",
        kept(&ship, HULL)
    );
    assert!(
        kept(&ship, SCENERY) > 0.8,
        "a ship blur smeared the scenery the camera holds: {}",
        kept(&ship, SCENERY)
    );
    // Any input abandons a shot under way, and the rest of the shot is not written.
    let abandoned = drive(
        &image,
        "mblur-abandon",
        &format!(
            "{to_the_race} press start; {down}{down}{down}{down} hold right 1; ticks 2; \
             {down} hold right 260; ticks 3; press cross; ticks 3; ticks 20; hold left 3; \
             ticks 3; until photo_job=none within 20; ticks 5"
        ),
        &shots,
    );
    assert!(abandoned.status.success(), "{}", abandoned.stderr);
    assert!(
        the_photos(&abandoned).is_empty(),
        "an abandoned shot wrote a file: {:?}",
        the_photos(&abandoned)
    );
    assert!(
        abandoned.states.iter().any(|state| state
            .get("photo_job")
            .and_then(Value::as_str)
            .is_some_and(|job| job != "none")),
        "the shot never started"
    );
}

/// One `key=value` of the drive's `photo_fx` line.
fn fx_field<'a>(fx: &'a str, key: &str) -> Option<&'a str> {
    fx.split(' ')
        .find_map(|part| part.strip_prefix(key)?.strip_prefix('='))
}

fn fx_values(run: &Run, key: &str) -> Vec<String> {
    run.states
        .iter()
        .filter_map(|state| state.get("photo_fx")?.as_str())
        .filter_map(|fx| fx_field(fx, key).map(str::to_string))
        .collect()
}

/// The recording a run left in `auto/`, whole.
fn the_recording(run: &Run) -> Vec<u8> {
    let dir = run.dir.join("data/oag/replays/auto");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|entries| entries.filter_map(|e| Some(e.ok()?.path())).collect())
        .unwrap_or_default();
    files.sort();
    assert_eq!(
        files.len(),
        1,
        "one race, one recording in {}",
        dir.display()
    );
    std::fs::read(&files[0]).expect("the recording")
}

/// ADR-0088: on the new keys photo mode's right stick orbits, its left stick pans and zooms, L1
/// and R1 change the craft, L2 and R2 scrub the REWIND row and never change the craft (under the
/// default trigger mode a pulled trigger also raises its shoulder's button in the snapshot), a
/// full pull of R2 is PLAY. None of it reaches the simulation: every live tick hashes as the same
/// tick of a race that never visited, and the recording is byte for byte the control's.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_right_stick_and_the_triggers_move_the_view_and_never_the_race() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-bindings-shots");
    let _ = std::fs::remove_dir_all(&shots);
    std::fs::create_dir_all(&shots).expect("a directory");
    let to_the_throttle = TO_THE_TIME_TRIAL.replace("ticks 400; ", "hold cross 700; ");
    let control = drive_unmodded(
        &image,
        "bindings-control",
        &format!("{to_the_throttle} ticks 1500"),
        &shots,
        &[],
    );
    assert!(control.status.success(), "{}", control.stderr);
    let visited = drive_unmodded(
        &image,
        "bindings",
        &format!(
            "{to_the_throttle} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; \
             press r; ticks 3; screenshot SHOTS/craft-1.png; press l; ticks 3; press r; ticks 3; press r; ticks 3; \
             press l; ticks 3; press l; ticks 3; \
             press select; ticks 5; rstick 1 -0.5 40; ticks 3; screenshot SHOTS/orbited.png; \
             lstick 1 1 20; ticks 3; \
             pull l2 0.8 8; ticks 3; pull r2 0.5 4; ticks 3; \
             pull r2 1.0 3; ticks 60; screenshot SHOTS/playing.png; pull l2 0.4 3; ticks 3; \
             escape; until photo=none within 50; ticks 500"
        ),
        &shots,
        &[],
    );
    assert!(visited.status.success(), "{}", visited.stderr);

    // L1 and R1 changed the craft and wrapped; the triggers never did.
    let subjects: Vec<u32> = {
        let mut seen: Vec<u32> = fx_values(&visited, "subject")
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        seen.dedup();
        seen
    };
    assert!(
        subjects.starts_with(&[0, 1, 0]) && subjects.len() >= 6,
        "L1 and R1 cycle the craft: {subjects:?}"
    );
    let after_the_cycling = subjects.len();
    let last = *subjects.last().expect("subjects");
    assert!(
        fx_values(&visited, "subject")
            .iter()
            .rev()
            .take_while(|s| s.parse() == Ok(last))
            .count()
            > 40,
        "{after_the_cycling} craft changes and then the triggers pulled: the subject stayed {last}"
    );

    // The right stick orbited, the left stick panned and zoomed.
    let orbit = fx_values(&visited, "orbit");
    let turned = orbit.iter().filter(|o| o.as_str() != "0.00,0.00").count();
    assert!(turned > 0, "the right stick never orbited: {orbit:?}");
    assert!(
        fx_values(&visited, "pan")
            .iter()
            .any(|p| !p.starts_with("0.00,")),
        "the left stick never panned"
    );
    assert!(
        fx_values(&visited, "fov")
            .iter()
            .any(|f| f.parse::<f32>().is_ok_and(|f| f < 65.0)),
        "the left stick never zoomed"
    );

    // L2 scrubbed back, R2 on, and a full pull played.
    let rewind: Vec<u64> = fx_values(&visited, "rewind")
        .iter()
        .filter_map(|r| r.parse().ok())
        .collect();
    let live = *rewind.first().expect("the mode opened");
    let lowest = *rewind.iter().min().expect("rewind ticks");
    // A landing under about 130 ticks starts from the first keyframe and does not hash as the live
    // run did (`docs/ui/photo-mode.md`, "Not done, named"): the scrub stays above it.
    assert!(lowest > 130, "the scrub went to {lowest}");
    assert!(lowest + 100 < live, "L2 never scrubbed back: {rewind:?}");
    assert!(
        fx_values(&visited, "play").iter().any(|p| p == "1"),
        "a full pull of R2 never turned PLAY on"
    );

    // Nothing of it reached the race.
    let held: BTreeMap<u64, String> = ticks(&control)
        .into_iter()
        .map(|(tick, hash, _)| (tick, hash))
        .collect();
    let mut compared = 0;
    for (tick, hash, _) in &ticks(&visited) {
        if let Some(control) = held.get(tick) {
            assert_eq!(hash, control, "tick {tick} hashes differently");
            compared += 1;
        }
    }
    assert!(compared > 500, "only {compared} ticks compared");
    // The two races ran different lengths: what both recorded is the same, input for input,
    // event for event and stored hash for stored hash.
    let (theirs, ours) = (
        oag_replay::Replay::from_bytes(&the_recording(&control)).expect("the control's recording"),
        oag_replay::Replay::from_bytes(&the_recording(&visited)).expect("the visit's recording"),
    );
    let common = theirs.ticks().min(ours.ticks());
    assert!(common > 1000, "only {common} ticks were recorded by both");
    assert_eq!(theirs.initial_hash, ours.initial_hash);
    for (slot, (a, b)) in theirs.inputs.iter().zip(&ours.inputs).enumerate() {
        assert!(
            a[..common as usize] == b[..common as usize],
            "slot {slot}'s recorded inputs differ"
        );
    }
    let before = |replay: &oag_replay::Replay| -> Vec<oag_replay::Event> {
        replay
            .events
            .iter()
            .copied()
            .filter(|e| e.tick < common)
            .collect()
    };
    assert_eq!(before(&theirs), before(&ours), "the recorded events differ");
    let hashes = (common / u64::from(theirs.header.hash_interval.max(1))) as usize;
    assert_eq!(
        theirs.hashes[..hashes],
        ours.hashes[..hashes],
        "the stored hashes differ"
    );
}
