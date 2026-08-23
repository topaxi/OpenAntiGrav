//! Hardware-free tests for the cue layer.
//!
//! Nothing here touches a disc: [`Banks`] is built by hand so the law in
//! [`Engine`] and the choice in [`Banks::pick`] can be checked without game
//! content. The disc-backed half is
//! `crates/game/tests/sfx_ground_truth.rs`.

use super::*;

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
    }
}

#[test]
fn a_cue_with_one_waveform_never_consults_the_generator() {
    let banks = banks(&[(Cue::SpeedupPad, 1, false)]);
    let mut rng = Rng::new(1);
    let before = rng.snapshot();
    let (sound, looping) = banks.pick(Cue::SpeedupPad, &mut rng).expect("loaded");
    assert_eq!(sound.frames(), 1);
    assert!(!looping);
    // Drawing a number for a choice of one would make every other cue's stream
    // depend on how many alternates this one happens to have.
    assert_eq!(rng.snapshot(), before);
}

#[test]
fn a_cue_with_alternates_reaches_all_of_them() {
    let banks = banks(&[(Cue::Collision, 15, false)]);
    // Every alternate is one frame long, so they are told apart by identity
    // rather than by content.
    let mut rng = Rng::new(7);
    let mut pointers = std::collections::BTreeSet::new();
    for _ in 0..600 {
        let (sound, _) = banks.pick(Cue::Collision, &mut rng).expect("loaded");
        pointers.insert(Arc::as_ptr(&sound) as usize);
    }
    assert_eq!(pointers.len(), 15, "some alternate is never chosen");
}

#[test]
fn an_unloaded_cue_is_silent_rather_than_substituted() {
    let banks = banks(&[(Cue::SpeedupPad, 1, false)]);
    let mut rng = Rng::new(1);
    assert!(banks.pick(Cue::Collision, &mut rng).is_none());
    assert!(banks.pick(Cue::Engine, &mut rng).is_none());
}

#[test]
fn every_cue_names_a_bank_and_a_string() {
    for cue in Cue::ALL {
        assert!(!cue.name().is_empty());
        assert!(cue.bank().entry(false).starts_with(r"Data\Sound\"));
        assert!(cue.bank().entry(true).starts_with(r"Data\Sound\"));
    }
    // Zone moves the ship bank and nothing else, because `SHIP_ZM` is the only
    // bank with a Zone counterpart on either disc.
    assert_eq!(BankName::Ship.entry(false), r"Data\Sound\ship.bnk");
    assert_eq!(BankName::Ship.entry(true), r"Data\Sound\ship_zone.bnk");
    assert_eq!(BankName::Hud.entry(true), BankName::Hud.entry(false));
    assert_eq!(
        BankName::Weapons.entry(true),
        BankName::Weapons.entry(false)
    );
    // The two cues this port holds a handle to, and only those two: a held cue
    // must not also be fired as a one-shot from the drain loop.
    let held: Vec<Cue> = Cue::ALL.into_iter().filter(|c| c.held()).collect();
    assert_eq!(held, vec![Cue::Engine, Cue::Shield]);
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
    // Eight craft drawing the same note would be one craft eight times over,
    // which is the thing the spread exists to prevent.
    assert!(notes.windows(2).any(|w| w[0] != w[1]));
}

#[test]
fn the_engine_snaps_on_its_first_tick_and_lags_after() {
    let mut mixer = Mixer::new(44_100);
    let banks = banks(&[(Cue::Engine, 1, true)]);
    let mut rng = Rng::new(3);
    let mut engine = Engine::new(&mut rng);
    let base = engine.base;

    engine.tick(&mut mixer, &banks, &mut rng, 100.0, true, 1.0 / 60.0);
    // The rising edge is a snap: without it the note sweeps up from wherever
    // `lag` started over the first second of every race.
    assert!((engine.lag - (base + 100.0 * ENGINE_PITCH_PER_KMH)).abs() < 1e-3);

    let before = engine.lag;
    engine.tick(&mut mixer, &banks, &mut rng, 300.0, true, 1.0 / 60.0);
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
        engine.tick(&mut mixer, &banks, &mut rng, 200.0, true, 1.0 / 60.0);
    }
    assert_eq!(mixer.active_voices(), 1, "the engine re-triggered");
    assert_eq!(mixer.starved(), 0);

    engine.stop(&mut mixer);
    assert_eq!(mixer.active_voices(), 0);
}

#[test]
fn a_non_looping_engine_bank_is_refused_rather_than_retriggered() {
    let mut mixer = Mixer::new(44_100);
    // A bank whose `~ENGINE` descriptors do not set the loop flag. Playing it
    // as a one-shot would restart it every time it ended, sixty times a second
    // once it is shorter than a tick.
    let banks = banks(&[(Cue::Engine, 1, false)]);
    let mut rng = Rng::new(5);
    let mut engine = Engine::new(&mut rng);

    for _ in 0..10 {
        engine.tick(&mut mixer, &banks, &mut rng, 200.0, true, 1.0 / 60.0);
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
        engine.tick(&mut mixer, &banks, &mut rng, 200.0, true, 1.0 / 60.0);
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
        engine.tick(&mut mixer, &banks, &mut rng, 250.0, true, 1.0 / 60.0);
    }
    let running = engine.pitch;
    assert!(running > engine.base);

    // Twice as fast down as up, so a race that has just ended is quiet in
    // about two seconds rather than four. Stepped past that here: the fall is
    // 1/120 a tick against an intensity of exactly 1.0, so landing on zero at
    // tick 120 would be an assertion about float accumulation rather than
    // about the law.
    for _ in 0..180 {
        engine.tick(&mut mixer, &banks, &mut rng, 250.0, false, 1.0 / 60.0);
    }
    assert!(engine.intensity <= 0.0);
    assert!(engine.pitch < running);
    // The spin-down stops at the craft's own base note rather than running
    // away downward for the rest of the session.
    assert!(
        engine.pitch >= engine.base - ENGINE_SPINDOWN,
        "wound down past the base note"
    );
    // The chase state is untouched, so a restart resumes from where the engine
    // was rather than from the floor.
    assert!(engine.lag > engine.base);
}

/// The bug this guards is the one the composition root actually had: the
/// finished-race arm of the frame loop steps nothing, so if the held voice were
/// not advanced *and* released it would loop at racing pitch under the results
/// table until the player backed out to the menus.
#[test]
fn a_finished_race_releases_the_engine_rather_than_leaving_it_humming() {
    let mut mixer = Mixer::new(44_100);
    let banks = banks(&[(Cue::Engine, 1, true)]);
    let mut rng = Rng::new(17);
    let mut engine = Engine::new(&mut rng);
    for _ in 0..300 {
        engine.tick(&mut mixer, &banks, &mut rng, 250.0, true, 1.0 / 60.0);
    }
    assert_eq!(mixer.active_voices(), 1);

    // The recovered volume law floors at 0.4, so "intensity zero" is still a
    // 40 % drone. The voice is released instead - see `Engine::tick`.
    for _ in 0..300 {
        engine.tick(&mut mixer, &banks, &mut rng, 250.0, false, 1.0 / 60.0);
    }
    assert_eq!(mixer.active_voices(), 0, "the engine kept sounding");

    // And it stays released: the finished arm goes on calling this every tick
    // for as long as the results table is up.
    for _ in 0..300 {
        engine.tick(&mut mixer, &banks, &mut rng, 250.0, false, 1.0 / 60.0);
    }
    assert_eq!(mixer.active_voices(), 0, "the engine restarted itself");
}

#[test]
fn a_bank_that_mixes_looping_and_one_shot_waveforms_keeps_them_apart() {
    // `hud.bnk`'s `~BLOWUP` is one looping waveform of two and
    // `~AIRBRAKE_MONO` one of three, so the flag cannot be collapsed to the
    // cue. Nothing wired today is mixed, which is exactly why this is a test
    // rather than something the next reader would notice.
    let mixed = Loaded {
        waveforms: vec![
            (
                Arc::new(Sound::new(vec![1], 1, 44_100).expect("sound")),
                false,
            ),
            (
                Arc::new(Sound::new(vec![2], 1, 44_100).expect("sound")),
                true,
            ),
        ],
    };
    let banks = Banks {
        sounds: [(Cue::Collision, mixed)].into_iter().collect(),
        report: Vec::new(),
    };

    let mut rng = Rng::new(23);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200 {
        let (_, looping) = banks.pick(Cue::Collision, &mut rng).expect("loaded");
        seen.insert(looping);
    }
    assert_eq!(seen.len(), 2, "the per-waveform flag was collapsed");
}
