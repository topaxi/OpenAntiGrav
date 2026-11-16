//! Pulse's pre-race flyby voice-over, through the audio, against the real disc.
//!
//! **`#[ignore]`d and never run in CI**; needs `data/images/` and `ffmpeg`. Run with
//! `just test-data`.
//!
//! `RaceMode_UpdateIntro` starts the circuit's `PreRaceDialogue` clip as its lock counter reaches
//! zero and stops the music player when the flyby ends. The claim: on a null audio backend the
//! clip is sounding from the tick the flyby raises its start until the tick it raises its stop,
//! and the race song parked under the countdown's hold is not disturbed by either.

use oag_game::sound::GameLibrary;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input, InputSnapshot};

const IMAGE: &str = "data/images/pulse-psp-usa.chd";

fn cross() -> PlayerInputs {
    let mut buttons = Input::EMPTY;
    buttons.begin_frame(Button::Cross.bit());
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

/// Plays the flyby of `16_Track` on a null backend and returns whether the voice-over sounds
/// after each flyby tick, and whether the held race song was sounding at the end.
fn flyby(held: bool) -> Option<(Vec<bool>, bool)> {
    let source = oag_testdata::image(IMAGE)?.display().to_string();
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: source.clone(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("flyby-voice-audio");
    let discs = oag_sound::MusicDiscs::survey(&source, &GameLibrary);
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        true,
    );
    oag_game::sound::hold_race_song(&mut audio, loaded.title);
    audio.start_race_music(&discs, oag_sound::MusicSource::Auto, &cache);
    audio.settle_music();
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(race.begin_intro());
    oag_game::sound::prepare_intro_voice(&mut audio, &discs, &race);
    audio.settle_pre_race_voice();
    let mut sounding = Vec::new();
    while race.in_intro() {
        race.tick_intro(&if held { cross() } else { PlayerInputs::none() });
        oag_game::sound::intro_cues(&mut audio, &mut race);
        oag_game::sound::flyby_tick(&mut audio, &mut race);
        audio.tick();
        sounding.push(audio.pre_race_voice_playing());
        assert!(sounding.len() < 3000, "the flyby never ended");
    }
    Some((sounding, audio.playhead_race().is_some()))
}

/// Fails if the flyby raises no voice calls, forwards them to nothing, or starts the clip off its
/// tick; and if the voice-over's stop reaches the held race song.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and ffmpeg"]
fn the_voice_over_sounds_from_the_counters_zero_until_the_flyby_ends() {
    let lock = oag_pulse::pre_race::PRE_RACE.lock_ticks.value as usize;
    let Some((sounding, song)) = flyby(false) else {
        return;
    };
    assert!(sounding[..lock].iter().all(|&on| !on), "before zero");
    assert!(
        sounding[lock..sounding.len() - 1].iter().all(|&on| on),
        "from the counter's zero until the end"
    );
    assert!(!sounding[sounding.len() - 1], "stopped on the ending tick");
    assert!(!song, "the held race song is still parked");
}

/// A held Cross ends the flyby on the tick the counter reaches zero, where the original starts and
/// stops the voice in one call: the clip never sounds.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and ffmpeg"]
fn a_skipped_flyby_never_sounds_its_voice_over() {
    let lock = oag_pulse::pre_race::PRE_RACE.lock_ticks.value as usize;
    let Some((sounding, _)) = flyby(true) else {
        return;
    };
    assert_eq!(sounding.len(), lock + 1);
    assert!(sounding.iter().all(|&on| !on));
}

/// The effects bus gain after each audio step of a `16_Track` flyby, and then of 30 more steps once
/// it is over, with the flyby's calls and the race's sounds forwarded as the session does.
fn effects_gain(held: bool) -> Option<Vec<f32>> {
    use oag_audio::Bus;
    let source = oag_testdata::image(IMAGE)?.display().to_string();
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source,
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        true,
    );
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(race.begin_intro());
    let mut gains = Vec::new();
    let step = |audio: &mut oag_sound::Audio, gains: &mut Vec<f32>| {
        audio.tick();
        gains.push(audio.output().with_mixer(|mixer| mixer.bus_gain(Bus::Sfx)));
    };
    while race.in_intro() {
        race.tick_intro(&if held { cross() } else { PlayerInputs::none() });
        oag_game::sound::intro_cues(&mut audio, &mut race);
        oag_game::sound::flyby_tick(&mut audio, &mut race);
        step(&mut audio, &mut gains);
        assert!(gains.len() < 3000, "the flyby never ended");
    }
    for _ in 0..30 {
        step(&mut audio, &mut gains);
    }
    Some(gains)
}

/// `RaceMode_UpdateIntro` drops the effects to silence in its first call and fades them in from the
/// counter's `0x28` (tick 20) by `0.02` a call. Fails if the effects bus is not silent through tick
/// 19, does not climb by that step from tick 20 or is not full 50 calls on, whether the flyby runs
/// out or a held Cross ends it at tick 60, mid-ramp: the ramp finishes in the countdown.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_effects_are_silent_through_tick_19_then_fade_in_by_002_a_call() {
    for held in [false, true] {
        let Some(gains) = effects_gain(held) else {
            return;
        };
        assert!(
            gains[..20].iter().all(|&g| g == 0.0),
            "silent first 20 calls, held {held}"
        );
        for (call, &gain) in gains[20..70].iter().enumerate() {
            let want = (call + 1) as f32 * 0.02;
            assert!(
                (gain - want).abs() < 1e-4,
                "held {held}: call {call}: {gain} against {want}"
            );
        }
        assert!(
            gains[69..].iter().all(|&g| g > 0.999),
            "full from call 69 on, held {held}"
        );
    }
}

/// Per flyby tick of `03_Track` (the whole flyby, no skip): the effects bus gain, the circuit's
/// emitters sounding, and the craft engines audible (open above zero gain: out of range an engine holds its voice at
/// zero, as the original's commit volume 0).
fn flyby_mix() -> Option<Vec<(f32, usize, usize)>> {
    use oag_audio::Bus;
    let source = oag_testdata::image(IMAGE)?.display().to_string();
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source,
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(r"Data\Environments\03_Track\track.vex".to_string()),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        true,
    );
    oag_game::sound::hold_race_song(&mut audio, loaded.title);
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(race.begin_intro());
    let mut mix = Vec::new();
    while race.in_intro() {
        race.tick_intro(&PlayerInputs::none());
        oag_game::sound::intro_cues(&mut audio, &mut race);
        oag_game::sound::flyby_tick(&mut audio, &mut race);
        audio.tick();
        let gain = audio.output().with_mixer(|mixer| mixer.bus_gain(Bus::Sfx));
        mix.push((gain, audio.ambient_voices(), audio.audible_engines()));
        assert!(mix.len() < 3000, "the flyby never ended");
    }
    Some(mix)
}

/// Under Pulse's flyby the effects fade quiets the circuit's ambience and the gain is the square
/// of the fade. Fails if the circuit's first emitter does not sound near the original's tick 393 (the flyby once took
/// none), if a craft engine is audible before about tick 1,150, where the original's grid comes into earshot, or if the bus gain is the fade rather than its square.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_flyby_plays_the_circuit_ambience_under_the_squared_fade_and_no_engines_early() {
    let Some(mix) = flyby_mix() else {
        return;
    };
    for (tick, &(gain, _, _)) in mix.iter().enumerate().take(70) {
        let fade = if tick < 20 {
            0.0
        } else {
            ((tick - 19) as f32 * 0.02).min(1.0)
        };
        assert!(
            (gain - fade * fade).abs() < 1e-4,
            "tick {tick}: bus {gain} against fade squared {}",
            fade * fade
        );
    }
    let first_ambient = mix.iter().position(|&(_, ambient, _)| ambient > 0);
    eprintln!(
        "first ambience tick {first_ambient:?}, flyby ticks {}",
        mix.len()
    );
    // The original's first emitter-placed loop, `moather` cue 0 `NEON_RING`, starts at tick 393;
    // its tick-0 `~AMB` loop is not among the placed emitters in range and nothing stands in for it.
    assert!(
        first_ambient.is_some_and(|tick| (380..=400).contains(&tick)),
        "the circuit's first emitter sounds near tick 393, got {first_ambient:?}"
    );
    for tick in [673, 1033, 1200, 1500] {
        eprintln!(
            "tick {tick}: ambient {} engines audible {}",
            mix[tick].1, mix[tick].2
        );
    }
    let first_engine = mix.iter().position(|&(_, _, other)| other > 0);
    eprintln!("first audible engine tick {first_engine:?}");
    // The original's first non-zero engine volume is at about tick 1,150.
    assert!(
        first_engine.is_some_and(|tick| (1100..=1200).contains(&tick)),
        "the first audible engine is near tick 1,150, got {first_engine:?}"
    );
}
