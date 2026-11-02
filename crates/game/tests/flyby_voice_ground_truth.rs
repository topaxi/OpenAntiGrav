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
        oag_game::sound::intro_voice(&mut audio, &mut race);
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
