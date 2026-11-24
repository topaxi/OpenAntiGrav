//! `--no-audio`: the forced-null backend `--dump-audio` uses, without a sample
//! buffer to write. Split out under the 200-line module rule
//! (`scripts/check-file-size.py`).

use super::*;

#[test]
fn no_audio_forces_the_null_backend() {
    let audio = Audio::open(
        &crate::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        true,
    );
    assert!(
        !audio.output().is_streaming(),
        "--no-audio must never open a device, whether or not one is attached"
    );
}

#[test]
fn no_audio_and_no_dump_leaves_no_playhead_to_pace_a_movie_against() {
    // The clock rule of `a_mixer_that_is_never_advanced_offers_no_clock`, reached
    // through `--no-audio`: with nothing streaming or dumped, a movie's sound is
    // never a clock, so `Frontend::advance_movie` uses the fixed timestep, the
    // path that finishes a movie leg at the same tick on every machine (see
    // `Audio::movie_playhead` and ADR-0019).
    let mut audio = Audio::open(
        &crate::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        true,
    );
    let sound = Sound::new(vec![0i16; 44_100 * 2], 2, 44_100).expect("a sound");
    assert!(audio.start_movie(sound), "a free voice");
    assert_eq!(
        audio.movie_playhead(),
        None,
        "--no-audio must land on the same tick-clocked fallback as no device at all"
    );
}

/// A null output plays its voices on unheard: a one-shot ends when its sound does, so a race's
/// effects free their slots and the race song is never refused for want of one (without it the
/// slots filled, the song never started and the now-playing display stayed blank).
#[test]
fn no_audio_retires_a_voice_when_its_sound_ends_and_frees_its_slot() {
    let mut audio = Audio::open(
        &crate::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        true,
    );
    let short = Arc::new(Sound::new(vec![0i16; 4_410 * 2], 2, 44_100).expect("a sound"));
    let ids: Vec<_> = (0..oag_audio::mixer::MAX_VOICES)
        .map(|_| {
            audio
                .output()
                .with_mixer(|mixer| mixer.play(Play::once(Arc::clone(&short), Bus::Sfx)))
                .expect("a free slot")
        })
        .collect();
    for _ in 0..12 {
        audio.tick();
    }
    assert!(
        ids.iter()
            .all(|&id| !audio.output().with_mixer(|mixer| mixer.is_playing(id))),
        "every 0.1 s voice has ended after 0.2 s of ticks"
    );
    let song = Arc::new(Sound::new(vec![0i16; 44_100 * 2], 2, 44_100).expect("a sound"));
    assert!(
        audio
            .output()
            .with_mixer(|mixer| mixer.play(Play::once(song, Bus::Music)))
            .is_some(),
        "the next voice finds a slot"
    );
}
