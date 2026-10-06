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
