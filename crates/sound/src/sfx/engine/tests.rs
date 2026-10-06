//! The engine law, checked with no hardware and no disc. The cue layer's tests
//! are in `sfx/tests.rs`; the law moved out with [`Engine`].

use super::*;
use crate::sfx::Loaded;
use oag_audio::Sound;
use std::sync::Arc;

/// A listener at the origin and a craft position that changes nothing: the craft
/// is on top of the listener, so the law under test alone moves the gain.
const LISTENER: oag_audio::Listener = oag_audio::Listener {
    position: [0.0; 3],
    right: [1.0, 0.0, 0.0],
};
const HEARD: [f32; 3] = [0.0; 3];

/// A `Loaded` holding `count` distinguishable one-frame sounds.
fn loaded(count: usize, looping: bool) -> Loaded {
    Loaded {
        waveforms: (0..count)
            .map(|i| {
                (
                    Arc::new(Sound::new(vec![i as i16 + 1], 1, 44_100).expect("sound")),
                    looping,
                )
            })
            .collect(),
    }
}

fn banks(entries: &[(Cue, usize, bool)]) -> Banks {
    Banks {
        sounds: entries
            .iter()
            .map(|&(cue, count, looping)| (cue, loaded(count, looping)))
            .collect(),
        report: Vec::new(),
        ..Default::default()
    }
}

#[test]
fn the_engine_note_is_spread_per_craft() {
    let mut rng = Rng::new(11);
    let notes: Vec<f32> = (0..8).map(|_| Engine::new(&mut rng).base).collect();
    for note in &notes {
        assert!(
            (ENGINE_BASE - ENGINE_SPREAD..=ENGINE_BASE + ENGINE_SPREAD).contains(note),
            "{note} is outside the recovered spread"
        );
    }
    // Eight craft on one note would be one craft eight times over.
    assert!(notes.windows(2).any(|w| w[0] != w[1]));
}

#[test]
fn the_engine_snaps_on_its_first_tick_and_lags_after() {
    let mut mixer = Mixer::new(44_100);
    let banks = banks(&[(Cue::Engine, 1, true)]);
    let mut rng = Rng::new(3);
    let mut engine = Engine::new(&mut rng);
    let base = engine.base;

    engine.tick(
        &mut mixer,
        &banks,
        &mut rng,
        100.0,
        true,
        HEARD,
        &LISTENER,
        false,
        true,
        1.0 / 60.0,
    );
    // The rising edge snaps: else the note sweeps up from `lag`'s start over the
    // first second of every race.
    assert!((engine.lag - (base + 100.0 * ENGINE_PITCH_PER_KMH)).abs() < 1e-3);

    let before = engine.lag;
    engine.tick(
        &mut mixer,
        &banks,
        &mut rng,
        300.0,
        true,
        HEARD,
        &LISTENER,
        false,
        true,
        1.0 / 60.0,
    );
    let target = base + 300.0 * ENGINE_PITCH_PER_KMH;
    // One step of a 1% chase, not a jump to the new target.
    assert!((engine.lag - (before + (target - before) * ENGINE_LAG_RATE)).abs() < 1e-3);
    assert!(engine.lag < target);
}

#[test]
fn the_engine_holds_one_voice_across_ticks() {
    let mut mixer = Mixer::new(44_100);
    let banks = banks(&[(Cue::Engine, 1, true)]);
    let mut rng = Rng::new(5);
    let mut engine = Engine::new(&mut rng);

    for _ in 0..120 {
        engine.tick(
            &mut mixer,
            &banks,
            &mut rng,
            200.0,
            true,
            HEARD,
            &LISTENER,
            false,
            true,
            1.0 / 60.0,
        );
    }
    assert_eq!(mixer.active_voices(), 1, "the engine re-triggered");
    assert_eq!(mixer.starved(), 0);

    engine.stop(&mut mixer);
    assert_eq!(mixer.active_voices(), 0);
}

#[test]
fn a_non_looping_engine_bank_is_refused_rather_than_retriggered() {
    let mut mixer = Mixer::new(44_100);
    // A bank whose `~ENGINE` descriptors do not set the loop flag: playing it as a
    // one-shot would restart it every time it ended, sixty times a second once
    // shorter than a tick.
    let banks = banks(&[(Cue::Engine, 1, false)]);
    let mut rng = Rng::new(5);
    let mut engine = Engine::new(&mut rng);

    for _ in 0..10 {
        engine.tick(
            &mut mixer,
            &banks,
            &mut rng,
            200.0,
            true,
            HEARD,
            &LISTENER,
            false,
            true,
            1.0 / 60.0,
        );
    }
    assert_eq!(mixer.active_voices(), 0);
}

#[test]
fn intensity_rises_to_full_and_stops_there() {
    let mut mixer = Mixer::new(44_100);
    let banks = banks(&[(Cue::Engine, 1, true)]);
    let mut rng = Rng::new(9);
    let mut engine = Engine::new(&mut rng);
    for _ in 0..600 {
        engine.tick(
            &mut mixer,
            &banks,
            &mut rng,
            200.0,
            true,
            HEARD,
            &LISTENER,
            false,
            true,
            1.0 / 60.0,
        );
    }
    assert!((engine.intensity - 1.0).abs() < 1e-6);
}

#[test]
fn a_stopped_engine_winds_down_to_its_own_note_and_stays_there() {
    let mut mixer = Mixer::new(44_100);
    let banks = banks(&[(Cue::Engine, 1, true)]);
    let mut rng = Rng::new(13);
    let mut engine = Engine::new(&mut rng);
    for _ in 0..600 {
        engine.tick(
            &mut mixer,
            &banks,
            &mut rng,
            250.0,
            true,
            HEARD,
            &LISTENER,
            false,
            true,
            1.0 / 60.0,
        );
    }
    let running = engine.pitch;
    assert!(running > engine.base);

    // Twice as fast down as up, so a finished race is quiet in about two seconds.
    // Stepped past that: the fall is 1/120 a tick against intensity 1.0, so
    // landing on zero at tick 120 would assert float accumulation, not the law.
    for _ in 0..180 {
        engine.tick(
            &mut mixer,
            &banks,
            &mut rng,
            250.0,
            false,
            HEARD,
            &LISTENER,
            false,
            true,
            1.0 / 60.0,
        );
    }
    assert!(engine.intensity <= 0.0);
    assert!(engine.pitch < running);
    // The spin-down stops at the craft's base note, not downward for the session.
    assert!(
        engine.pitch >= engine.base - ENGINE_SPINDOWN,
        "wound down past the base note"
    );
    // The chase state is untouched, so a restart resumes from where the engine
    // was, not the floor.
    assert!(engine.lag > engine.base);
}

/// The composition root's bug: the finished-race arm steps nothing, so a held
/// voice not advanced and released would loop at racing pitch under the results
/// table until the player backed out.
#[test]
fn a_finished_race_releases_the_engine_rather_than_leaving_it_humming() {
    let mut mixer = Mixer::new(44_100);
    let banks = banks(&[(Cue::Engine, 1, true)]);
    let mut rng = Rng::new(17);
    let mut engine = Engine::new(&mut rng);
    for _ in 0..300 {
        engine.tick(
            &mut mixer,
            &banks,
            &mut rng,
            250.0,
            true,
            HEARD,
            &LISTENER,
            false,
            true,
            1.0 / 60.0,
        );
    }
    assert_eq!(mixer.active_voices(), 1);

    // The recovered law floors at 0.4, so intensity zero is still a 40 % drone;
    // the voice is released instead (`Engine::tick`).
    for _ in 0..300 {
        engine.tick(
            &mut mixer,
            &banks,
            &mut rng,
            250.0,
            false,
            HEARD,
            &LISTENER,
            false,
            true,
            1.0 / 60.0,
        );
    }
    assert_eq!(mixer.active_voices(), 0, "the engine kept sounding");

    // It stays released: the finished arm keeps calling this every tick.
    for _ in 0..300 {
        engine.tick(
            &mut mixer,
            &banks,
            &mut rng,
            250.0,
            false,
            HEARD,
            &LISTENER,
            false,
            true,
            1.0 / 60.0,
        );
    }
    assert_eq!(mixer.active_voices(), 0, "the engine restarted itself");
}

/// `SoundInstance_UpdateSpatial`'s doppler term: an engine closing on the ear
/// plays sharp, one receding plays flat, a cut frame holds the note alone.
#[test]
fn a_closing_engine_is_sharp_and_a_receding_one_flat_and_a_cut_is_neither() {
    let mut mixer = Mixer::new(44_100);
    let banks = banks(&[(Cue::Engine, 1, true)]);
    let mut rng = Rng::new(5);
    let mut engine = Engine::new(&mut rng);
    let dt = 1.0 / 60.0;
    let mut tick = |engine: &mut Engine, mixer: &mut Mixer, at: f32, enabled: bool| {
        engine.tick(
            mixer,
            &banks,
            &mut rng,
            100.0,
            true,
            [0.0, 0.0, at],
            &LISTENER,
            false,
            enabled,
            dt,
        );
        mixer
            .pitch(engine.voice.expect("the engine holds a voice"))
            .unwrap()
    };
    // Settle the note itself over a second, so the lag is not what moves.
    for _ in 0..120 {
        tick(&mut engine, &mut mixer, 40.0, true);
    }
    let held = tick(&mut engine, &mut mixer, 40.0, true);
    // Ten units closer in a frame: 600 units a second, `2^(600 * 0.0005)`.
    let closing = tick(&mut engine, &mut mixer, 30.0, true);
    assert!(
        (closing / held - 2f32.powf(600.0 * oag_audio::spatial::DOPPLER_SCALE)).abs() < 1e-3,
        "closing {closing} against held {held}"
    );
    let receding = tick(&mut engine, &mut mixer, 40.0, true);
    assert!(receding < held, "receding {receding} against held {held}");
    // A cut frame plays the bare note whatever the distance did.
    let cut = tick(&mut engine, &mut mixer, 10.0, false);
    assert!((cut - held).abs() < 1e-3, "cut {cut} against held {held}");
}
