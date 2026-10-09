//! The Mine's own `MINELAUNCH` cue, on a real disc and a real drop, out to a
//! WAV.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test mine_launch_audio_ground_truth --run-ignored all
//! ```
//!
//! Its own file rather than a test added to `mine_ground_truth.rs` or
//! `sfx_ground_truth.rs`: the first is the weapon's own mechanics (spread,
//! fuse, damage) on `crates/gameplay`'s side of the fence, and the second is
//! bank decoding in the abstract, off a `Banks` built with no `Race` at all.
//! This is the third thing - `audio::sfx::Cue::MineLaunch`'s wiring, end to
//! end through the composition root's own `Audio::race_tick`, on a real lap
//! under the real force law. Same standard `track_audio_ground_truth.rs` set
//! for the circuit's own ambience: a headless run through the null backend,
//! written out to 16-bit PCM, so "it should play" is a file rather than an
//! assertion about internal state alone.

use std::path::{Path, PathBuf};

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

/// Past the start-line countdown, the same reasoning `mine_ground_truth.rs`
/// gives at length: a stationary craft is not what a `MINELAUNCH` fired from a
/// moving one sounds like once positional audio pans and attenuates it.
const WARM_UP_TICKS: u64 = oag_race::COUNTDOWN_TICKS + 120;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// One button held down, tick after tick - `mine_ground_truth.rs`'s own
/// `held`, reproduced rather than shared: neither file exports test helpers to
/// the other, the same near-duplication `missile_ground_truth.rs` already
/// explains at length.
fn held(button: Button) -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(button.bit());
    buttons.begin_frame(button.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// Lays a whole cluster from a moving craft, mixes it through the real audio
/// path and writes the result to a `.wav` under `data/shots/`.
///
/// **What this proves that the unit tests in `race::tests::cues` cannot**: not
/// only that a `CueEvent` is queued, but that it survives `place()`'s range
/// gate, resolves against the real `weapons.bnk` waveform, and renders to
/// audible, non-silent PCM through the same mixer a real session uses. The
/// pickup is granted directly rather than found on a pad, the same way
/// `mine_ground_truth.rs`'s own tests do: this is about the cue the drop
/// raises, not about finding a Weapon Pad on the circuit.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn laying_a_mine_sounds_minelaunch_and_writes_it_out() {
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    let mut race = race::Race::start(loaded.setup);
    race.mine_stats().expect("the disc authors a Mine");

    let throttle = held(Button::Cross);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle));
    }
    let speed = race.sim.world.ships[0]
        .physics
        .body
        .linear_velocity
        .length();
    assert!(
        speed > 10.0,
        "the craft is barely moving at {speed:.1} units/s"
    );

    let wav = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/shots")
        .join("mine-launch.wav");
    // `Some(dump)` forces the null backend, which is what makes this runnable
    // headlessly - see `Audio::open`.
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(wav.clone()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );

    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.sim.world.ships[0]
        .pickup
        .begin_drop(oag_weapons::projectile::mine::CLUSTER);

    // Comfortably past the whole cluster: `DROP_INTERVAL` is a tenth of a
    // second apart, so `CLUSTER` charges are out well inside a second and a
    // half of racing.
    // **Peeked, not drained**: `Audio::race_tick` below does its own
    // `drain_cues`, which is what actually feeds the mixer. Draining here
    // first would starve that call and this test would measure a queue it had
    // already emptied rather than the real wiring.
    let mut launches = 0;
    for _ in 0..90 {
        race.tick(&PlayerInputs::single(throttle));
        launches += race
            .pending_cues()
            .iter()
            .filter(|e| e.cue == oag_sound::sfx::Cue::MineLaunch)
            .count();
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
    }
    audio.finish().expect("writing the dump");

    assert_eq!(
        launches,
        usize::from(oag_weapons::projectile::mine::CLUSTER),
        "the drop did not raise one MINELAUNCH per mine in the cluster"
    );

    let file = std::fs::read(&wav).expect("the dump");
    let pcm = &file[44..];
    let peak = pcm
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| i16::from_le_bytes(*c).unsigned_abs())
        .max()
        .expect("samples");
    assert!(peak > 0, "the dump is {} bytes of silence", pcm.len());
    println!(
        "wrote {} - {} frames, peak sample {peak}, {launches} MINELAUNCH cue(s) raised",
        wav.display(),
        pcm.len() / 4
    );
}
