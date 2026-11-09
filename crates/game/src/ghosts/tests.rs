use oag_core::math::{Quat, Vec3};
use oag_replay::{GhostLap, Pose, Recorder};

use super::*;

fn key() -> Key {
    Key::new(
        "Wipeout Pulse",
        Some(r"Data\Environments\16_Track\track.vex"),
        "time_trial",
        "Venom",
    )
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-ghosts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn replay(key: &Key, lap_ticks: u32) -> Replay {
    let recorder = Recorder::new(header(key, "feisar", 1), 7);
    recorder.replay(Some(GhostLap {
        lap_ticks,
        poses: vec![
            Pose {
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            };
            lap_ticks as usize + 1
        ],
    }))
}

#[test]
fn a_key_becomes_a_safe_path_one_segment_per_part() {
    let path = path_in(Path::new("/root"), &key());
    assert_eq!(
        path,
        Path::new("/root/wipeout_pulse/time_trial/venom/data_environments_16_track_track.vex.oagr")
    );
    assert_eq!(segment(".."), "_", "no escaping the directory");
    assert_eq!(segment(""), "_");
}

#[test]
fn only_a_quicker_lap_replaces_a_stored_ghost() {
    let root = scratch("quicker");
    let key = key();
    let path = path_in(&root, &key);
    assert!(save_to(&path, &key, &replay(&key, 3_000)).expect("writes"));
    assert!(
        !save_to(&path, &key, &replay(&key, 3_100)).expect("reads"),
        "slower"
    );
    assert!(
        !save_to(&path, &key, &replay(&key, 3_000)).expect("reads"),
        "a tie"
    );
    assert!(save_to(&path, &key, &replay(&key, 2_900)).expect("writes"));
    let stored = load_from(&path, &key).expect("a ghost");
    assert_eq!(stored.ghost.map(|lap| lap.lap_ticks), Some(2_900));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_ghost_for_another_race_or_a_damaged_file_is_not_raced() {
    let root = scratch("wrong");
    let key = key();
    let path = path_in(&root, &key);
    let other = Key::new("Wipeout Pulse", Some("elsewhere"), "time_trial", "venom");
    assert!(save_to(&path, &other, &replay(&other, 3_000)).expect("writes"));
    assert!(load_from(&path, &key).is_none(), "another key's file");

    std::fs::write(&path, b"OAGR not really").expect("writes");
    assert!(load_from(&path, &key).is_none(), "a damaged file");
    assert!(
        save_to(&path, &key, &replay(&key, 3_000)).expect("writes"),
        "a damaged file is simply replaced"
    );
    assert!(load_from(&path, &key).is_some());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn only_time_trial_and_speed_lap_race_a_ghost() {
    for mode in oag_race::Mode::ALL {
        assert_eq!(
            mode.races_a_ghost(),
            matches!(mode, oag_race::Mode::TimeTrial | oag_race::Mode::SpeedLap),
            "{mode:?}"
        );
    }
}

#[test]
fn auto_load_off_races_no_stored_ghost_and_leaves_the_file_to_be_beaten() {
    let root = scratch("autoload");
    let key = key();
    let path = path_in(&root, &key);
    assert!(save_to(&path, &key, &replay(&key, 3_000)).expect("writes"));
    assert!(auto_load_from(&path, &key, true).is_some(), "on: loaded");
    assert!(
        auto_load_from(&path, &key, false).is_none(),
        "off: not loaded"
    );
    // The stored lap is still there to be beaten.
    assert!(save_to(&path, &key, &replay(&key, 2_900)).expect("writes"));
    assert!(!save_to(&path, &key, &replay(&key, 3_100)).expect("reads"));
}

mod old_and_foreign_headers {
    use std::sync::Mutex;

    use super::*;

    static LINES: Mutex<Vec<(log::Level, String)>> = Mutex::new(Vec::new());

    struct Capture;

    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            LINES
                .lock()
                .unwrap()
                .push((record.level(), record.args().to_string()));
        }
        fn flush(&self) {}
    }

    /// `bytes` with its `HEAD` text passed through `edit` and the checksum redone.
    fn with_head(bytes: &[u8], edit: impl FnOnce(&str) -> String) -> Vec<u8> {
        let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let text = edit(std::str::from_utf8(&bytes[16..16 + len]).unwrap());
        let mut out = bytes[..8].to_vec();
        out.extend_from_slice(b"HEAD");
        out.extend_from_slice(&(text.len() as u32).to_le_bytes());
        out.extend_from_slice(text.as_bytes());
        out.extend_from_slice(&bytes[16 + len..bytes.len() - 16]);
        let mut hasher = oag_core::StateHasher::new();
        hasher.write(&out);
        let sum = hasher.finish();
        out.extend_from_slice(b"END_");
        out.extend_from_slice(&8u32.to_le_bytes());
        out.extend_from_slice(&sum.to_le_bytes());
        out
    }

    fn stored(name: &str, edit: impl FnOnce(&str) -> String) -> (PathBuf, Key) {
        let root = scratch(name);
        let key = key();
        let path = path_in(&root, &key);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = with_head(&replay(&key, 3_000).to_bytes(), edit);
        std::fs::write(&path, bytes).unwrap();
        (path, key)
    }

    /// An old ghost file (no `toml-schema` line) is raced as before and the absence is a `debug`
    /// line; a file of another kind or a later major is ignored, naming what it is, at `warn`.
    /// Fails if the loader stops reading the header, or logs the old file louder.
    #[test]
    fn an_old_file_loads_at_debug_and_a_foreign_header_is_refused_by_name() {
        let _ = log::set_logger(Box::leak(Box::new(Capture)));
        log::set_max_level(log::LevelFilter::Debug);

        let (path, key) = stored("old", |text| {
            text.lines().skip(1).collect::<Vec<_>>().join("\n")
        });
        assert!(load_from(&path, &key).is_some(), "an old ghost still loads");
        let seen = LINES.lock().unwrap().clone();
        assert!(
            seen.iter()
                .any(|(level, line)| *level == log::Level::Debug && line.contains("no toml-schema")),
            "{seen:?}"
        );
        assert!(
            seen.iter().all(|(level, _)| *level > log::Level::Warn),
            "no warning for an old file: {seen:?}"
        );

        LINES.lock().unwrap().clear();
        let (path, key) = stored("later", |text| {
            text.replace("/v1/replay-header.tosd", "/v2/replay-header.tosd")
        });
        assert!(load_from(&path, &key).is_none());
        let seen = LINES.lock().unwrap().clone();
        assert!(
            seen.iter().any(|(level, line)| *level == log::Level::Warn
                && line.contains("replay-header schema v2 is not supported")),
            "{seen:?}"
        );

        LINES.lock().unwrap().clear();
        let (path, key) = stored("kind", |text| {
            text.replace("/v1/replay-header.tosd", "/v1/pilot.tosd")
        });
        assert!(load_from(&path, &key).is_none());
        let seen = LINES.lock().unwrap().clone();
        assert!(
            seen.iter()
                .any(|(_, line)| line.contains("names a \"pilot\" file")),
            "{seen:?}"
        );
    }
}
