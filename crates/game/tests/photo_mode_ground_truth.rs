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
struct Run {
    status: std::process::ExitStatus,
    stderr: String,
    states: Vec<Value>,
    dir: PathBuf,
}

const TO_THE_RACE: &str = "boot; select racebox; press cross; until page=race; \
     select launch_race; press cross; until screen=picker; press cross; \
     until screen.id=ship-select; press cross; until stage=race within 4000; ticks 400; ";

fn drive(image: &Path, tag: &str, script: &str, shots: &Path) -> Run {
    drive_with(image, tag, script, shots, &[])
}

/// [`drive`] with `args` added to the game's command line.
fn drive_with(image: &Path, tag: &str, script: &str, shots: &Path, args: &[&str]) -> Run {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("photo-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let log = dir.join("state.jsonl");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
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

fn pulse() -> Option<PathBuf> {
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
    found.sort();
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
             hold select up 30; press square; ticks 5; press cross; ticks 5; \
             screenshot SHOTS/during.png; press circle; until photo=none within 50; ticks 500"
        ),
        &shots,
    );
    assert!(photographed.status.success(), "{}", photographed.stderr);

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
             press start; ticks 10; screenshot SHOTS/bare.png; press start; ticks 5; \
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
