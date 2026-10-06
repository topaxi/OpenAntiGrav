//! The four AUDIO rows: what each means as a number and which gain it reaches
//! (see [ADR-0027](../../../../../docs/architecture/adr/0027-three-mix-buses.md)
//! for why four and which two are ours).

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
    for cue in sfx::Cue::ALL {
        let expected = if cue.bank() == sfx::BankName::Speech {
            Bus::Speech
        } else {
            Bus::Sfx
        };
        assert_eq!(cue.bus(), expected, "{cue:?} is on the wrong bus");
    }
    // The speech bank is not empty, so the loop above is not vacuous.
    assert!(
        sfx::Cue::ALL.iter().any(|cue| cue.bus() == Bus::Speech),
        "no cue reaches the speech bus at all"
    );
}
