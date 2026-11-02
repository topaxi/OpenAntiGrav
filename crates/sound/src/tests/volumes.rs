//! The four AUDIO rows: what each means as a number and which gain it reaches
//! (see [ADR-0027](../../../../../docs/architecture/adr/0027-three-mix-buses.md)
//! for why four and which two are ours).

use crate::sfx::CueTables;
use oag_audio::mixer::MUSIC_MASTER_TRIM;

use super::*;

#[test]
fn a_volume_round_trips_through_its_own_text() {
    for value in Volume::OFFERED {
        assert_eq!(value.to_string().parse::<Volume>(), Ok(value));
    }
}

#[test]
fn a_volume_outside_the_range_is_refused() {
    assert!("101".parse::<Volume>().is_err());
    assert!("-1".parse::<Volume>().is_err());
    assert_eq!("0".parse::<Volume>(), Ok(Volume(0)));
}

#[test]
fn full_volume_is_unattenuated() {
    assert_eq!(Volume::FULL.gain(), 1.0);
    assert_eq!(Volume::default(), Volume::FULL);
    assert_eq!(Volume(0).gain(), 0.0);
}

#[test]
fn every_settings_volume_reaches_the_bus_it_names() {
    // The four rows against the four gains in one pass: a row wired to the wrong
    // bus is inaudible by ear when three are at 100 (ADR-0027).
    let audio = Audio::open(
        &crate::settings::Settings {
            music_volume: Volume(25),
            sfx_volume: Volume(50),
            speech_volume: Volume(75),
            master_volume: Volume(90),
            ..Default::default()
        },
        None,
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    audio.output().with_mixer(|mixer| {
        // Music alone also carries `MUSIC_MASTER_TRIM`, the original's fixed
        // -7.13 dB on the music bus, independent of the slider.
        assert_eq!(mixer.bus_gain(Bus::Music), 0.25 * MUSIC_MASTER_TRIM);
        assert_eq!(mixer.bus_gain(Bus::Sfx), 0.5);
        assert_eq!(mixer.bus_gain(Bus::Speech), 0.75);
    });
}

#[test]
fn a_cue_is_a_voice_line_exactly_when_its_bank_is_the_speech_one() {
    // `Cue::bus` must stay derived from `Cue::bank`: a second hand-written list is
    // what ADR-0027 avoids.
    for cue in oag_gameplay::cue::Cue::ALL {
        let expected = if cue.bank() == sfx::BankName::Speech {
            Bus::Speech
        } else {
            Bus::Sfx
        };
        assert_eq!(cue.bus(), expected, "{cue:?} is on the wrong bus");
    }
    // The speech bank is not empty, so the loop above is not vacuous.
    assert!(
        oag_gameplay::cue::Cue::ALL
            .iter()
            .any(|cue| cue.bus() == Bus::Speech),
        "no cue reaches the speech bus at all"
    );
}

fn sfx_bus(audio: &Audio) -> f32 {
    audio.output().with_mixer(|mixer| mixer.bus_gain(Bus::Sfx))
}

/// The flyby's effects fade as the original's: silent from the first call, held silent until the
/// counter reads `0x28` (tick 20), then `0.02` more on each call to the slider, and the slider
/// alone from the countdown on. Fails if the fade is not on the Sfx bus, steps by another amount,
/// or starts early.
#[test]
fn the_flyby_effects_fade_is_silent_then_a_002_ramp_to_the_slider() {
    let mut audio = Audio::open(
        &crate::settings::Settings {
            sfx_volume: Volume(50),
            ..Default::default()
        },
        None,
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    assert_eq!(sfx_bus(&audio), 0.5, "no flyby: the slider alone");
    let mut curve = Vec::new();
    for tick in 0..=80u32 {
        if tick == 0 {
            audio.silence_sfx();
        }
        if tick == 20 {
            audio.fade_in_sfx(0.02);
        }
        curve.push(audio.sfx_fade_gain());
        assert_eq!(sfx_bus(&audio), 0.5 * audio.sfx_fade_gain(), "tick {tick}");
        audio.tick();
    }
    assert!(
        curve[..=20].iter().all(|&g| g == 0.0),
        "silent through tick 20"
    );
    for (tick, &gain) in curve.iter().enumerate().take(71).skip(21) {
        let want = (tick - 20) as f32 * 0.02;
        assert!((gain - want.min(1.0)).abs() < 1e-5, "tick {tick}: {gain}");
    }
    // The original adds the step in `f32` too, so the last call may land a hair under one and
    // the clamp take it on the next.
    assert!(curve[69] < 0.99, "still climbing at tick 69");
    assert!(curve[70] > 0.999, "full within a call of tick 70");
    assert!(
        curve[71..].iter().all(|&g| g == 1.0),
        "exactly full from tick 71"
    );
    audio.silence_sfx();
    audio.stop_race_sfx();
    assert_eq!(sfx_bus(&audio), 0.5, "leaving the race restores the slider");
}
