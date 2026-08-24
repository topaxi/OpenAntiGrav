//! What the mixer in [`super`] is asserted to do: voice lifetime and slot reuse,
//! playheads and seeking, bus gains, and the determinism of a rendered tick.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `mixer.rs`: the tests are 267 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

fn tone(frames: usize, channels: u16, rate: u32) -> Arc<Sound> {
    let samples = (0..frames * channels as usize)
        .map(|i| i16::try_from(i % 1000).unwrap_or(0))
        .collect();
    Arc::new(Sound::new(samples, channels, rate).expect("sound"))
}

#[test]
fn a_sound_needs_whole_frames() {
    assert!(Sound::new(vec![0; 3], 2, 44_100).is_err());
    assert!(Sound::new(vec![0; 4], 2, 44_100).is_ok());
    assert!(Sound::new(vec![0; 4], 0, 44_100).is_err());
    assert!(Sound::new(vec![0; 4], 2, 0).is_err());
}

#[test]
fn rendering_with_nothing_playing_writes_silence_not_a_short_buffer() {
    let mut mixer = Mixer::new(44_100);
    let mut out = vec![1.0f32; 64];
    mixer.render(&mut out);
    assert_eq!(out.len(), 64, "render must fill the whole buffer");
    assert!(out.iter().all(|s| *s == 0.0), "expected silence");
}

#[test]
fn a_one_shot_frees_its_slot_when_it_ends() {
    let mut mixer = Mixer::new(44_100);
    let id = mixer
        .play(Play::once(tone(4, 2, 44_100), Bus::Sfx))
        .expect("a free slot");
    assert_eq!(mixer.active_voices(), 1);

    let mut out = vec![0.0f32; 64 * CHANNELS];
    mixer.render(&mut out);

    assert_eq!(mixer.active_voices(), 0, "the voice should have finished");
    assert!(!mixer.is_playing(id), "the handle should be stale");
    assert_eq!(mixer.position(id), None, "and report no playhead");
}

/// The playhead is in the **source's** seconds, so a caller pacing a movie
/// against it does not have to know the device's rate. Rendered at 44.1 kHz
/// against a 22.05 kHz source, so the two would disagree by a factor of two
/// if the resampling step were left in.
#[test]
fn a_playhead_is_measured_in_the_sources_own_seconds() {
    let mut mixer = Mixer::new(44_100);
    let id = mixer
        .play(Play::looping(tone(22_050, 1, 22_050), Bus::Music))
        .expect("a free slot");
    assert_eq!(mixer.position(id), Some(0.0), "nothing rendered yet");

    // 4,410 output frames is a tenth of a second at 44.1 kHz, and a tenth
    // of a second of a 22.05 kHz source is 2,205 of its frames.
    let mut out = vec![0.0f32; 4_410 * CHANNELS];
    mixer.render(&mut out);
    let seconds = mixer.position(id).expect("still sounding");
    assert!(
        (seconds - 0.1).abs() < 1e-9,
        "expected a tenth of a second, got {seconds}"
    );
}

/// [`Mixer::seek`] and [`Mixer::position`] are each other's inverse, and the
/// unit they agree in is the **source's** seconds. Two sounds at different
/// rates, because that is the case the whole signature exists for: carrying
/// a playhead from a 48 kHz track to a 44.1 kHz one has to land at the same
/// moment of the music, not at the same frame number.
#[test]
fn a_seek_lands_where_the_playhead_then_reports() {
    for rate in [22_050, 44_100, 48_000] {
        let mut mixer = Mixer::new(44_100);
        let id = mixer
            .play(Play::looping(tone(rate as usize * 4, 2, rate), Bus::Music))
            .expect("a free slot");

        assert!(mixer.seek(id, 2.5), "the voice is sounding");
        let seconds = mixer.position(id).expect("still sounding");
        assert!(
            (seconds - 2.5).abs() < 1e-9,
            "at {rate} Hz: expected 2.5 s, got {seconds}"
        );
    }
}

/// Past the end of a **looping** sound the seek wraps, the same rule
/// `render` applies when the playhead runs off one. Landing at four times
/// the length of a one-second bed has to be the start of it, not silence
/// for ever.
#[test]
fn a_seek_past_the_end_of_a_loop_wraps_into_it() {
    let mut mixer = Mixer::new(44_100);
    let id = mixer
        .play(Play::looping(tone(44_100, 2, 44_100), Bus::Music))
        .expect("a free slot");

    assert!(mixer.seek(id, 4.25));
    let seconds = mixer.position(id).expect("still sounding");
    assert!(
        (seconds - 0.25).abs() < 1e-9,
        "expected 0.25 s, got {seconds}"
    );

    // A negative or non-finite request is the start rather than a refusal:
    // a voice parked at a NaN position renders nothing and says nothing.
    assert!(mixer.seek(id, -3.0));
    assert_eq!(mixer.position(id), Some(0.0));
    assert!(mixer.seek(id, f64::NAN));
    assert_eq!(mixer.position(id), Some(0.0));
}

#[test]
fn seeking_a_stale_handle_does_nothing() {
    let mut mixer = Mixer::new(44_100);
    let id = mixer
        .play(Play::once(tone(4, 2, 44_100), Bus::Sfx))
        .expect("a free slot");
    let mut out = vec![0.0f32; 64 * CHANNELS];
    mixer.render(&mut out);
    assert!(!mixer.seek(id, 1.0), "the voice has already finished");
}

#[test]
fn a_looping_voice_keeps_going_past_its_end() {
    let mut mixer = Mixer::new(44_100);
    let id = mixer
        .play(Play::looping(tone(4, 2, 44_100), Bus::Sfx))
        .expect("a free slot");
    let mut out = vec![0.0f32; 1024 * CHANNELS];
    mixer.render(&mut out);
    assert!(mixer.is_playing(id), "a looping voice must not free itself");
}

#[test]
fn a_stale_handle_cannot_steer_the_voice_that_took_its_slot() {
    let mut mixer = Mixer::new(44_100);
    let old = mixer
        .play(Play::once(tone(2, 2, 44_100), Bus::Sfx))
        .expect("a free slot");
    // Run it out so the slot is recycled.
    let mut out = vec![0.0f32; 64 * CHANNELS];
    mixer.render(&mut out);
    let new = mixer
        .play(Play::looping(tone(64, 2, 44_100), Bus::Sfx))
        .expect("a free slot");

    assert!(!mixer.is_playing(old));
    mixer.stop(old);
    assert!(
        mixer.is_playing(new),
        "stopping a stale handle must not stop whoever took the slot"
    );
}

#[test]
fn the_pool_refuses_rather_than_steals_when_it_is_full() {
    let mut mixer = Mixer::new(44_100);
    for _ in 0..MAX_VOICES {
        assert!(
            mixer
                .play(Play::looping(tone(64, 2, 44_100), Bus::Sfx))
                .is_some()
        );
    }
    assert_eq!(mixer.active_voices(), MAX_VOICES);
    assert!(
        mixer
            .play(Play::once(tone(4, 2, 44_100), Bus::Sfx))
            .is_none()
    );
    assert_eq!(mixer.starved(), 1);
    assert_eq!(
        mixer.active_voices(),
        MAX_VOICES,
        "a refused start must not have displaced anyone"
    );
}

#[test]
fn a_bus_gain_only_moves_its_own_bus() {
    let sound = tone(1024, 2, 44_100);
    let mut mixer = Mixer::new(44_100);
    mixer.play(Play::looping(sound.clone(), Bus::Music));
    mixer.set_bus_gain(Bus::Sfx, 0.0);
    let mut out = vec![0.0f32; 128 * CHANNELS];
    mixer.render(&mut out);
    let music_energy: f32 = out.iter().map(|s| s.abs()).sum();
    assert!(music_energy > 0.0, "silencing SFX must not silence music");

    mixer.stop_all();
    mixer.play(Play::looping(sound, Bus::Sfx));
    mixer.render(&mut out);
    let sfx_energy: f32 = out.iter().map(|s| s.abs()).sum();
    assert_eq!(sfx_energy, 0.0, "the SFX bus was set to zero");
}

#[test]
fn stopping_a_bus_leaves_the_other_one_sounding() {
    let sound = tone(1024, 2, 44_100);
    let mut mixer = Mixer::new(44_100);
    let music = mixer
        .play(Play::looping(sound.clone(), Bus::Music))
        .expect("slot");
    let sfx = mixer.play(Play::looping(sound, Bus::Sfx)).expect("slot");
    mixer.stop_bus(Bus::Sfx);
    assert!(mixer.is_playing(music));
    assert!(!mixer.is_playing(sfx));
}

#[test]
fn a_tick_renders_the_same_frame_count_every_time() {
    let mut mixer = Mixer::new(48_000);
    let mut out = Vec::new();
    for _ in 0..60 {
        assert_eq!(mixer.render_tick(60, &mut out), 800);
    }
    assert_eq!(
        out.len(),
        48_000 * CHANNELS,
        "sixty ticks at 60 Hz is one second of audio"
    );
}

#[test]
fn the_same_control_sequence_renders_the_same_samples_twice() {
    let render = || {
        let mut mixer = Mixer::new(44_100);
        mixer.play(Play::looping(tone(777, 1, 48_000), Bus::Music));
        let mut out = Vec::new();
        for tick in 0..30 {
            if tick == 10 {
                mixer.play(Play::once(tone(300, 2, 22_050), Bus::Sfx));
            }
            mixer.render_tick(60, &mut out);
        }
        out
    };
    assert_eq!(render(), render(), "the mixer must not read a clock");
}

#[test]
fn a_mono_source_arrives_on_both_channels() {
    let mut mixer = Mixer::new(44_100);
    mixer.play(Play::looping(tone(512, 1, 44_100), Bus::Sfx));
    let mut out = vec![0.0f32; 64 * CHANNELS];
    mixer.render(&mut out);
    for pair in out.as_chunks::<CHANNELS>().0 {
        assert_eq!(pair[0], pair[1], "mono must duplicate, not pan");
    }
}

#[test]
fn resampling_a_faster_source_consumes_it_faster() {
    // 88.2 kHz into a 44.1 kHz mixer is two source frames per output
    // frame, so a 200-frame sound lasts 100 output frames.
    let mut mixer = Mixer::new(44_100);
    let id = mixer
        .play(Play::once(tone(200, 2, 88_200), Bus::Sfx))
        .expect("slot");
    let mut out = vec![0.0f32; 99 * CHANNELS];
    mixer.render(&mut out);
    assert!(mixer.is_playing(id), "99 frames in, it should still sound");
    mixer.render(&mut out);
    assert!(!mixer.is_playing(id), "198 frames in, it should be done");
}

/// Finding U6's guard: a voice that could never advance is refused, and the
/// refusal is visible rather than silent.
#[test]
fn a_zero_pitch_voice_is_refused_rather_than_parked() {
    let mut mixer = Mixer::new(44_100);
    let before = mixer.starved();

    for pitch in [0.0, -1.0, f32::NAN] {
        let mut play = Play::once(tone(2048, 2, 44_100), Bus::Sfx);
        play.pitch = pitch;
        assert!(
            mixer.play(play).is_none(),
            "pitch {pitch} would never advance the playhead"
        );
    }
    assert_eq!(
        mixer.starved(),
        before + 3,
        "a refusal that does not count is a slot lost silently"
    );

    // And an ordinary pitch still plays, so the guard refuses the broken case
    // rather than the quiet one.
    let mut slow = Play::once(tone(2048, 2, 44_100), Bus::Sfx);
    slow.pitch = 0.01;
    assert!(mixer.play(slow).is_some());
}

/// Finding U7's guard: a sample rate that does not divide the tick rate must
/// not lose frames, or the `--dump-audio` WAV drifts behind the simulation.
#[test]
fn a_non_divisible_sample_rate_averages_out_over_a_second() {
    let mut mixer = Mixer::new(44_100);
    let mut out = Vec::new();
    let mut frames = 0;
    for _ in 0..64 {
        frames += mixer.render_tick(64, &mut out);
    }
    assert_eq!(
        frames, 44_100,
        "a second of ticks must be a second of audio"
    );
    assert_eq!(out.len(), 44_100 * CHANNELS);
}

#[test]
fn a_voice_with_no_pan_reaches_both_channels_unchanged() {
    // The default, and the reason `Play::pan` is an `Option`: music and movie
    // audio must not be pulled down 3 dB by a law that was never applied to
    // them. A regression here is silent - everything just gets quieter.
    let mut mixer = Mixer::new(8);
    let sound = Arc::new(Sound::new(vec![i16::MAX; 4], 1, 8).unwrap());
    mixer.play(Play::once(sound, Bus::Sfx)).unwrap();
    let mut out = vec![0.0; 2 * CHANNELS];
    mixer.render(&mut out);
    assert!((out[0] - 1.0).abs() < 1e-3);
    assert!((out[1] - 1.0).abs() < 1e-3);
}

#[test]
fn panning_hard_left_empties_the_right_channel() {
    let mut mixer = Mixer::new(8);
    let sound = Arc::new(Sound::new(vec![i16::MAX; 4], 1, 8).unwrap());
    mixer
        .play(Play {
            pan: Some(-1.0),
            ..Play::once(sound, Bus::Sfx)
        })
        .unwrap();
    let mut out = vec![0.0; 2 * CHANNELS];
    mixer.render(&mut out);
    assert!((out[0] - 1.0).abs() < 1e-3, "left was {}", out[0]);
    assert!(out[1].abs() < 1e-6, "right was {}", out[1]);
}

#[test]
fn a_centred_pan_is_quieter_than_no_pan_at_all() {
    // Not a bug: the original's own table gives a centred source 0.7071 in each
    // channel. This test exists so that "fixing" it to 1.0 fails loudly.
    let mut mixer = Mixer::new(8);
    let sound = Arc::new(Sound::new(vec![i16::MAX; 4], 1, 8).unwrap());
    mixer
        .play(Play {
            pan: Some(0.0),
            ..Play::once(sound, Bus::Sfx)
        })
        .unwrap();
    let mut out = vec![0.0; 2 * CHANNELS];
    mixer.render(&mut out);
    assert!((out[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);
    assert!((out[1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);
}

#[test]
fn a_live_voice_can_be_moved_across_the_field_and_back_to_none() {
    // The per-tick half: the original recomputes an emitter's angle every frame
    // and pushes it at the sounding instance, so an overtaking craft crosses
    // rather than jumping at its next cue.
    let mut mixer = Mixer::new(8);
    let sound = Arc::new(Sound::new(vec![i16::MAX; 64], 1, 8).unwrap());
    let id = mixer
        .play(Play {
            pan: Some(-1.0),
            ..Play::looping(sound, Bus::Sfx)
        })
        .unwrap();
    let mut out = vec![0.0; CHANNELS];
    mixer.render(&mut out);
    assert!(out[1].abs() < 1e-6);

    mixer.set_pan(id, Some(1.0));
    mixer.render(&mut out);
    assert!(out[0].abs() < 1e-6, "left did not empty: {}", out[0]);
    assert!((out[1] - 1.0).abs() < 1e-3);

    mixer.set_pan(id, None);
    mixer.render(&mut out);
    assert!(
        (out[0] - 1.0).abs() < 1e-3,
        "taking the position away should \
        restore both channels, not leave the last pan in place"
    );
}
