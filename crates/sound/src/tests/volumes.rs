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
fn the_spatial_audio_setting_reaches_the_mixer_and_defaults_to_auto() {
    use oag_audio::{SpatialChoice, SpatialMode};
    let settings = crate::settings::Settings::default();
    assert_eq!(settings.spatial_audio, SpatialChoice::Auto);
    let audio = Audio::open(&settings, None, None, oag_audio::MIN_BUFFER, false);
    audio
        .output()
        .with_mixer(|mixer| assert_eq!(mixer.spatial_mode(), SpatialMode::RearCue));
    let original = crate::settings::Settings {
        spatial_audio: SpatialChoice::Original,
        ..Default::default()
    };
    audio.apply(&original);
    audio
        .output()
        .with_mixer(|mixer| assert_eq!(mixer.spatial_mode(), SpatialMode::Original));
    assert!(
        !audio.output().with_mixer(|mixer| mixer.has_binaural()),
        "the convolver is built only when headphones are asked for"
    );

    let headphones = crate::settings::Settings {
        spatial_audio: SpatialChoice::Headphones,
        ..Default::default()
    };
    audio.apply(&headphones);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !audio.output().with_mixer(|mixer| mixer.has_binaural()) {
        assert!(
            std::time::Instant::now() < deadline,
            "the sphere never landed"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
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

/// Pulse's effects group is squared at the commit (`FUN_089957c0`), so the bus reads the square of
/// the slider times the fade, through the flyby and after it. Fails if the bus stays linear.
#[test]
fn the_squared_law_puts_the_square_of_slider_times_fade_on_the_bus() {
    use oag_title::group_volume::GroupVolumeLaw;
    let mut audio = Audio::open(
        &crate::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    audio.set_effects_volume_law(GroupVolumeLaw::Squared);
    for tick in 0..=80u32 {
        if tick == 0 {
            audio.silence_sfx();
        }
        if tick == 20 {
            audio.fade_in_sfx(0.02);
        }
        let fade = audio.sfx_fade_gain();
        assert!(
            (sfx_bus(&audio) - fade * fade).abs() < 1e-6,
            "tick {tick}: {} against {}",
            sfx_bus(&audio),
            fade * fade
        );
        audio.tick();
    }
    assert_eq!(sfx_bus(&audio), 1.0);
    audio.apply(&crate::settings::Settings {
        sfx_volume: Volume(50),
        ..Default::default()
    });
    assert_eq!(sfx_bus(&audio), 0.25, "the slider at half is a quarter");
    audio.stop_race_sfx();
    assert_eq!(
        sfx_bus(&audio),
        0.5,
        "leaving the race returns the menus to linear"
    );
}

/// Pulse's Speech group goes through the same squaring helper as the effects group, so in a race
/// the speech bus reads the square of the Speech slider and in the menus the slider itself. Fails
/// if the speech bus stays linear in a race, or stays squared after leaving one.
#[test]
fn the_squared_speech_law_puts_the_square_of_the_slider_on_the_speech_bus() {
    use oag_title::group_volume::GroupVolumeLaw;
    fn speech_bus(audio: &Audio) -> f32 {
        audio
            .output()
            .with_mixer(|mixer| mixer.bus_gain(Bus::Speech))
    }
    let settings = crate::settings::Settings {
        speech_volume: Volume(50),
        ..Default::default()
    };
    let mut audio = Audio::open(&settings, None, None, oag_audio::MIN_BUFFER, false);
    assert_eq!(speech_bus(&audio), 0.5, "the menus are linear");
    audio.set_speech_volume_law(GroupVolumeLaw::Squared);
    assert_eq!(speech_bus(&audio), 0.25, "a Pulse race squares it");
    audio.apply(&settings);
    assert_eq!(speech_bus(&audio), 0.25, "a settings apply keeps the law");
    audio.apply(&crate::settings::Settings {
        speech_volume: Volume(30),
        ..Default::default()
    });
    assert!((speech_bus(&audio) - 0.09).abs() < 1e-6);
    audio.stop_race_sfx();
    assert_eq!(speech_bus(&audio), 0.3, "leaving the race restores linear");
}

/// Pulse's Speech group is the effects group: in a race the speech bus reads the square of the SFX
/// slider times the flyby fade, with the Speech slider as a linear extra; leaving restores the
/// menus' own slider. Fails if the speech bus ignores the SFX slider or the fade.
#[test]
fn the_speech_bus_follows_the_sfx_slider_and_the_flyby_fade_squared() {
    use oag_title::group_volume::GroupVolumeLaw;
    fn speech_bus(audio: &Audio) -> f32 {
        audio
            .output()
            .with_mixer(|mixer| mixer.bus_gain(Bus::Speech))
    }
    let settings = crate::settings::Settings {
        sfx_volume: Volume(50),
        ..Default::default()
    };
    let mut audio = Audio::open(&settings, None, None, oag_audio::MIN_BUFFER, false);
    assert_eq!(speech_bus(&audio), 1.0, "the menus use the Speech slider");
    audio.set_speech_volume_law(GroupVolumeLaw::Squared);
    audio.set_speech_follows_effects(true);
    assert_eq!(speech_bus(&audio), 0.25, "SFX 0.5, Speech 1.0, fade full");
    audio.silence_sfx();
    assert_eq!(speech_bus(&audio), 0.0);
    audio.fade_in_sfx(0.02);
    for _ in 0..10 {
        audio.tick();
        let fade = audio.sfx_fade_gain();
        let want = (0.5 * fade) * (0.5 * fade);
        assert!((speech_bus(&audio) - want).abs() < 1e-6);
    }
    audio.apply(&crate::settings::Settings {
        sfx_volume: Volume(50),
        speech_volume: Volume(50),
        ..Default::default()
    });
    let fade = audio.sfx_fade_gain();
    assert!((speech_bus(&audio) - 0.25 * fade * fade * 0.5).abs() < 1e-6);
    audio.stop_race_sfx();
    assert_eq!(
        speech_bus(&audio),
        0.5,
        "leaving restores the Speech slider"
    );
}

#[test]
fn auto_follows_the_output_and_a_remembered_choice_wins_for_its_device_only() {
    use std::sync::{Arc, Mutex};

    use oag_audio::{Route, RouteKind, RouteWatcher, SpatialChoice, SpatialMode};

    let now = Arc::new(Mutex::new(Route {
        device: Some("jack|speaker".into()),
        kind: RouteKind::Speakers,
    }));
    let probe = Arc::clone(&now);
    let mut settings = crate::settings::Settings::default();
    let mut audio = Audio::open(&settings, None, None, oag_audio::MIN_BUFFER, true);
    audio.watch_route(RouteWatcher::with_probe(
        Arc::new(move || probe.lock().expect("lock").clone()),
        1,
    ));
    let mode = |audio: &Audio| audio.output().with_mixer(|mixer| mixer.spatial_mode());
    let settle = |audio: &mut Audio, settings: &mut crate::settings::Settings| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !audio.poll_route(settings) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    };
    let set_route = |device: &str, kind| {
        *now.lock().expect("lock") = Route {
            device: Some(device.into()),
            kind,
        };
    };

    settle(&mut audio, &mut settings);
    assert_eq!(mode(&audio), SpatialMode::RearCue, "speakers: rear cue");

    set_route("jack|headphones", RouteKind::Headphones);
    settle(&mut audio, &mut settings);
    assert_eq!(mode(&audio), SpatialMode::Headphones, "headphones: HRTF");
    assert_eq!(settings.spatial_audio, SpatialChoice::Auto);

    // A pick on a USB codec that reports itself as speakers, then away and back.
    set_route("usb-codec", RouteKind::Speakers);
    settle(&mut audio, &mut settings);
    assert_eq!(mode(&audio), SpatialMode::RearCue);
    settings.choose_spatial(audio.route_device(), SpatialChoice::Headphones);
    audio.apply(&settings);
    assert_eq!(mode(&audio), SpatialMode::Headphones);

    set_route("jack|speaker", RouteKind::Speakers);
    settle(&mut audio, &mut settings);
    assert_eq!(mode(&audio), SpatialMode::RearCue, "the pick is not global");

    set_route("usb-codec", RouteKind::Speakers);
    settle(&mut audio, &mut settings);
    assert_eq!(
        mode(&audio),
        SpatialMode::Headphones,
        "the pick returns with its device"
    );

    set_route("mystery", RouteKind::Unknown);
    settle(&mut audio, &mut settings);
    assert_eq!(mode(&audio), SpatialMode::RearCue, "unknown: rear cue");
}

/// A title's master trim reaches the mixer in decibels; minus 6.02 dB halves the sum. Fails if
/// `Audio::set_master_trim_db` stops reaching the mixer.
#[test]
fn a_title_master_trim_in_decibels_halves_the_sum_at_minus_six() {
    use oag_audio::{Bus, Play, Sound};
    let mut audio = Audio::open(
        &crate::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    audio.set_master_trim_db(-6.0206);
    let level = audio.output().with_mixer(|mixer| {
        mixer.set_limiting(false);
        let sound = std::sync::Arc::new(Sound::new(vec![i16::MAX; 64], 1, 8).unwrap());
        mixer.play(Play::looping(sound, Bus::Sfx)).unwrap();
        let mut out = vec![0.0; 2 * 16];
        mixer.render(&mut out);
        out[8]
    });
    assert!((level - 0.5).abs() < 0.01, "level {level}");
}
