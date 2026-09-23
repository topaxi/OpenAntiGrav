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
            races_a_ghost(mode),
            matches!(mode, oag_race::Mode::TimeTrial | oag_race::Mode::SpeedLap),
            "{mode:?}"
        );
    }
}
