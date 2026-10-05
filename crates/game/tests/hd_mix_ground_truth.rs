//! Wipeout HD's authored mix, read off the real disc.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_mix_ground_truth --run-ignored all
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
        oag_game::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
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
