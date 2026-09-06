//! What a circuit's own authored sound emitters do over a real lap.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test track_audio_ground_truth \
//!   --run-ignored all --no-capture
//! ```
//!
//! # What this is for
//!
//! `crates/formats/tests/sound_emitter_ground_truth.rs` proves the *decode*
//! against all 33 circuit files. This is the layer above, and it answers a
//! question nothing had answered: **how many of a circuit's emitters are inside
//! their own radius at the same time**, on a real lap, against the recovered
//! radius law. `oag_audio::mixer::MAX_VOICES` is 32 and `01_Track` authors 86
//! emitters, so "the budget question is real" was a prediction until this ran.

use std::path::{Path, PathBuf};

use oag_game::audio::sfx::{TrackEmitters, listener_of};
use oag_game::race;

/// `01_Track`, the circuit the thread names: 86 `sound` nodes and no cone.
const TRACK: &str = r"Data\Environments\01_Track\track.vex";

/// `14_Track`, the densest circuit on the disc: 97 `sound` nodes and 9 cones.
///
/// Measured as well as `01_Track` because a budget read off the *second* most
/// crowded circuit is not a budget. See `track-sound-emitters.md`'s census.
const DENSEST: &str = r"Data\Environments\14_Track\track.vex";

/// How long a measured lap is allowed to take before the run is called off.
///
/// Three minutes of simulated time. A Venom lap of Vineta K is well under one,
/// and a run that reaches this has gone wrong in a way a longer cap would hide.
const TICK_CAP: usize = 60 * 180;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// One circuit, loaded, with the player flown by the racing line.
fn lap(image: &Path, track: &str) -> Option<(race::Race, TrackEmitters)> {
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track.to_string()),
        // The whole grid, because the listener is the camera and the camera is
        // the player's - a solo run and a full grid put the ears in the same
        // places, but a full grid is what a player actually races.
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    // Parsed by `race::load` itself, off the same `.vex` it builds the circuit
    // from - so this measures what a real race carries, not a second decode.
    let emitters = loaded.setup.track_emitters.clone();
    let mut race = race::Race::start(loaded.setup);
    // The player is flown by their own racing line, so the ears go all the way
    // round rather than into the first wall. See `Race::set_autopilot`.
    race.set_autopilot(true);
    Some((race, emitters))
}

/// Flies one lap and reports how many emitters were in range on each tick.
///
/// Returns the peak, because that is the number a voice budget is set from.
/// Everything else is printed: a bound that only ever appears inside an
/// `assert!` is a number nobody reads, and this one is the input to a design
/// decision - `oag_audio::mixer::MAX_VOICES` is 32.
fn measure(name: &str, race: &mut race::Race, emitters: &TrackEmitters) -> usize {
    let mut histogram = vec![0usize; emitters.omni.len() + 1];
    let mut ever = vec![false; emitters.omni.len()];
    let mut ticks = 0usize;
    while ticks < TICK_CAP && race.world.race.laps_completed() < 1 {
        race.tick(&oag_gameplay::InputSnapshot::default());
        let listener = listener_of(race);
        #[expect(clippy::cast_precision_loss, reason = "a tick count, well under 2^24")]
        let frame = ticks as f32;
        let mut live = 0;
        for (at, _) in emitters.placed(&listener, frame) {
            ever[at] = true;
            live += 1;
        }
        histogram[live] += 1;
        ticks += 1;
    }
    assert!(
        ticks < TICK_CAP,
        "{name}: three minutes of autopilot and the lap never closed"
    );

    let peak = histogram
        .iter()
        .rposition(|&n| n > 0)
        .expect("at least one tick");
    let floor = histogram
        .iter()
        .position(|&n| n > 0)
        .expect("the same tick");
    let total: usize = histogram.iter().enumerate().map(|(n, c)| n * c).sum();
    #[expect(clippy::cast_precision_loss, reason = "counts, not money")]
    let mean = total as f32 / ticks as f32;
    let heard = ever.iter().filter(|&&e| e).count();

    println!(
        "{name}, one lap of {ticks} ticks: {floor}..={peak} emitters in range per tick, \
         mean {mean:.1}; {heard} of {} were in range at some point",
        emitters.omni.len()
    );
    for (count, ticks_at) in histogram.iter().enumerate() {
        if *ticks_at > 0 {
            println!("  {count:>3} in range: {ticks_at} tick(s)");
        }
    }
    peak
}

/// How many emitters are in range on each tick of one lap of `01_Track`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_lap_of_vineta_k_says_how_many_emitters_want_a_voice_at_once() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let Some((mut race, emitters)) = lap(&path, TRACK) else {
        return;
    };
    for line in &emitters.report {
        println!("{line}");
    }
    assert_eq!(
        emitters.omni.len(),
        86,
        "01_Track authors 86 omnidirectional emitters"
    );
    assert_eq!(emitters.cones, 0, "01_Track authors no cone");

    let peak = measure("01_Track", &mut race, &emitters);

    // The claim, and the reason the number was worth measuring: a circuit's
    // ambience fits in the voice pool with room for the race's own cues. If
    // this ever fails, the design question the thread raised is live again and
    // `Mixer::starved` is where it will show.
    assert!(
        peak < oag_audio::mixer::MAX_VOICES,
        "{peak} emitters want a voice at once and the pool holds {}",
        oag_audio::mixer::MAX_VOICES
    );
}

/// The same measurement on the circuit that authors the most.
///
/// `01_Track` alone would leave the budget resting on the circuit the thread
/// happened to name. `14_Track` is the disc's densest, and its nine cones are
/// the largest set this port deliberately does not play, so the gap between
/// what is authored and what sounds is at its widest here.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_densest_circuit_on_the_disc_fits_in_the_pool_too() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let Some((mut race, emitters)) = lap(&path, DENSEST) else {
        return;
    };
    for line in &emitters.report {
        println!("{line}");
    }
    assert_eq!(emitters.omni.len(), 97, "14_Track authors 97 of them");
    assert_eq!(emitters.cones, 9, "and nine cones this port does not play");

    let peak = measure("14_Track", &mut race, &emitters);
    assert!(
        peak < oag_audio::mixer::MAX_VOICES,
        "{peak} emitters want a voice at once and the pool holds {}",
        oag_audio::mixer::MAX_VOICES
    );
}
