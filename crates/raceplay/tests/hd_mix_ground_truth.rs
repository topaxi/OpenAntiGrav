//! Wipeout HD's authored mix, read off the real disc.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay hd_mix_ground_truth:: --run-ignored all
//! ```
//!
//! Every number asserted here was also read live off RPCS3's sound system
//! (`g_sound_system + 0x6a0 + 0xb8 * state`, `docs/formats/hd-xfx.md` "The
//! authored mix"), so the file's Stereo rows and the emulator agree.

use std::path::Path;

use oag_sound::hd_mix::{Maps, State};

fn maps() -> Option<Maps> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        return None;
    }
    let mut opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    Some(Maps::load(&mut opened.archives).expect("GlobalAudioConfig.xml has rows"))
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_rows_are_the_ones_read_live_off_the_emulator() {
    let Some(maps) = maps() else { return };
    let near = |a: f32, b: f32| (a - b).abs() < 1e-5;
    let front = maps.row(State::FrontEnd).expect("front end");
    assert!(near(front[0], 0.30), "front end music {}", front[0]);
    let countdown = maps.row(State::Countdown).expect("countdown");
    assert!(
        near(countdown[0], 0.0) && near(countdown[8], 0.0),
        "music and user8 are silent on the grid"
    );
    assert!(
        near(countdown[7], 0.5),
        "engines on the grid {}",
        countdown[7]
    );
    let race = maps.row(State::RaceNormal).expect("race");
    assert!(near(race[0], 0.65) && near(race[7], 0.68) && near(race[8], 0.8));
}

/// The emulator's live row for the critical-energy state read `music 0.45`
/// and `user7 0.7`, which are the `DATA00` copy of the file; `DATA01` ships
/// `0.5` and `0.8`. The port must read the copy the original reads.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_copy_read_is_the_one_the_original_loads() {
    let Some(maps) = maps() else { return };
    let critical = maps.row(State::CriticalEnergy).expect("critical energy");
    assert!((critical[0] - 0.45).abs() < 1e-5, "music {}", critical[0]);
    assert!((critical[7] - 0.7).abs() < 1e-5, "user7 {}", critical[7]);
}

/// The rear low-pass's value is HD's own file's, the stereo child; the surround child is above the
/// open end and filters nothing. Pulse, Pure, 2048 and Omega name no layout, so they play none.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_rear_low_pass_is_the_stereo_child_of_the_file() {
    let Some(maps) = maps() else { return };
    let rear = maps.rear();
    assert_eq!(rear.get("Stereo"), Some(14_000.0));
    assert_eq!(rear.get("Surround"), Some(24_000.0));
    let layout = oag_hd::TITLE.rear_occlusion.expect("HD names a layout");
    assert_eq!(rear.get(layout.value), Some(14_000.0));
    assert!(oag_pulse::TITLE.rear_occlusion.is_none());
    assert!(oag_pure::TITLE.rear_occlusion.is_none());
    assert!(oag_omega::TITLE.rear_occlusion.is_none());
    assert!(oag_2048::TITLE.rear_occlusion.is_none());
}

/// Every archive's copy of the file (`DATA00`, `DATA01`) carries the same stereo value, so the
/// copy the original loads is not a choice that matters for the rear low-pass.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_copy_of_the_file_carries_the_same_rear_value() {
    let Some(path) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let copies = opened
        .archives
        .read_every_name(r"Data\sound\GlobalAudioConfig.xml");
    assert!(copies.len() >= 2, "{} copies", copies.len());
    for (label, blob) in copies {
        println!("rear occlusion copy in {label}");
        let rear = Maps::parse(&String::from_utf8_lossy(&blob)).rear();
        assert_eq!(rear.get("Stereo"), Some(14_000.0), "{label}");
    }
}
