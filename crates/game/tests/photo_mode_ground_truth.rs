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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("photo-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let log = dir.join("state.jsonl");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        // One seed, or each launch deals its own weapons and the two races differ from tick 1.
        .args(["--no-audio", "--no-intro", "--seed", "7", "--drive"])
        .arg(script.replace("SHOTS", &shots.display().to_string()))
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
    let run = drive(
        &image,
        "pixels",
        &format!(
            "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
             until photo=internal within 200; press select; ticks 5; hold right 45; \
             press start; ticks 10; screenshot SHOTS/seen.png; press cross; ticks 5; \
             screenshot SHOTS/after.png; press circle; until photo=none within 50"
        ),
        &shots,
    );
    assert!(run.status.success(), "{}", run.stderr);
    let seen = decode(&shots.join("seen.png"));
    let after = decode(&shots.join("after.png"));
    let photos = the_photos(&run);
    assert_eq!(photos.len(), 1, "{photos:?}");
    let saved = decode(&photos[0]);
    assert_eq!(
        (saved.0, saved.1),
        (seen.0, seen.1),
        "the render's own size"
    );
    assert!(
        saved.0 >= 1280,
        "a picture at the size a player sees, not the PSP's 480"
    );
    assert_eq!(
        saved.2, seen.2,
        "the picture saved is the frame seen with the help hidden"
    );
    assert_eq!(seen.2, after.2, "and a shot does not move the camera");
}
