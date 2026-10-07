//! `Output`'s tests, out of line because the inline-test ceiling is 200 lines
//! and the gap-covering path needs more than the format conversions did.

use super::*;
use crate::mixer::{Bus, Play, Sound};
use crate::spectrum::Spectrum;

fn tone() -> Arc<Sound> {
    let samples = (0..2048).map(|i| ((i % 64) * 400) as i16).collect();
    Arc::new(Sound::new(samples, 2, 44_100).expect("sound"))
}

#[test]
fn a_null_output_still_mixes() {
    let output = Output::null(44_100);
    assert!(!output.is_streaming());
    assert_eq!(output.device_name(), None);

    output.with_mixer(|mixer| {
        mixer.play(Play::looping(tone(), Bus::Music));
    });

    let mut out = Vec::new();
    assert_eq!(output.render_tick(60, &mut out), 735);
    assert!(
        out.iter().any(|s| *s != 0.0),
        "a null output must still produce samples"
    );
}

#[test]
fn a_zero_sample_rate_falls_back_rather_than_dividing_by_zero() {
    let output = Output::null(0);
    assert_eq!(output.sample_rate(), DEFAULT_SAMPLE_RATE);
}

#[test]
fn stereo_spreads_verbatim() {
    let stereo = [1.0, -1.0, 0.5, -0.5];
    let mut out = [0.0; 4];
    spread(&stereo, &mut out, 2);
    assert_eq!(out, stereo);
}

#[test]
fn mono_spreads_as_the_average_of_the_pair() {
    let stereo = [1.0, 0.0, 0.5, 0.5];
    let mut out = [0.0; 2];
    spread(&stereo, &mut out, 1);
    assert_eq!(out, [0.5, 0.5]);
}

/// Finding U1's own guard, at the layer a machine with no sound card can
/// still check: the mixer renders `f32` and the device may want something
/// else, so `spread` has to convert rather than only copy. `i16` and `u16`
/// because they are the two formats a bare ALSA `hw:` device actually
/// offers, and the ones whose absence made a working card silent.
///
/// **Not a check of the device path**, which needs hardware this project's
/// runs do not have. What it pins is that full scale stays full scale and
/// silence stays silence through the conversion, in both signed and
/// unsigned conventions - `u16`'s origin is `1 << 15`, not zero, which is
/// the half of this that a copy would get wrong without erroring.
#[test]
fn a_device_that_wants_integers_gets_converted_samples() {
    let stereo = [1.0, -1.0, 0.0, 0.0];

    let mut signed = [0i16; 4];
    spread(&stereo, &mut signed, 2);
    assert_eq!(signed[0], i16::MAX);
    assert_eq!(signed[1], i16::MIN);
    assert_eq!(&signed[2..], &[0, 0]);

    let mut unsigned = [0u16; 4];
    spread(&stereo, &mut unsigned, 2);
    assert_eq!(unsigned[0], u16::MAX);
    assert_eq!(unsigned[1], u16::MIN);
    assert_eq!(&unsigned[2..], &[1 << 15, 1 << 15]);

    // And the padding past the rendered frames is the *format's* silence,
    // not a zero bit pattern - which for `u16` is the mid-point.
    let mut short = [7u16; 6];
    spread(&stereo, &mut short, 2);
    assert!(
        short[4..].iter().all(|s| *s == 1 << 15),
        "unsigned padding must be the format's origin, not zero"
    );
}

#[test]
fn extra_channels_are_left_silent_rather_than_filled_with_a_guess() {
    let stereo = [1.0, -1.0];
    let mut out = [9.0; 6];
    spread(&stereo, &mut out, 6);
    assert_eq!(out[0], 1.0);
    assert_eq!(out[1], -1.0);
    assert!(
        out[2..].iter().all(|s| *s == 0.0),
        "surrounds must not be invented here"
    );
}

/// A gap that begins mid-buffer ramps the signal it did get down to zero, so
/// the silence after it does not start with a step.
#[test]
fn a_buffer_the_ring_could_not_fill_ramps_down_into_the_gap() {
    let mut scratch = vec![0.8f32; 128 * CHANNELS];
    fade_out(&mut scratch);

    assert!(scratch[0] > 0.79, "the body of what arrived is untouched");
    let end = 128 - 1;
    assert!(
        scratch[end * CHANNELS].abs() < 0.02,
        "and it ends at silence"
    );
    for frame in (128 - DECLICK_FRAMES + 1)..128 {
        assert!(
            scratch[frame * CHANNELS] < scratch[(frame - 1) * CHANNELS],
            "frame {frame} did not fall"
        );
    }
}

/// And a gap that begins on a buffer boundary - nothing arrived at all - decays
/// from where the previous buffer ended instead, because that step is real too.
#[test]
fn a_gap_that_starts_at_a_boundary_decays_from_the_last_frame() {
    let mut scratch = vec![0.0f32; 128 * CHANNELS];
    ramp_from(&mut scratch, [1.0, -1.0]);

    assert!(scratch[0] > 0.9, "the ramp starts next to the last sample");
    assert!(scratch[1] < -0.9);
    for frame in 1..DECLICK_FRAMES {
        assert!(scratch[frame * CHANNELS] < scratch[(frame - 1) * CHANNELS]);
    }
    assert!(
        scratch[DECLICK_FRAMES * CHANNELS..]
            .iter()
            .all(|s| *s == 0.0),
        "past the ramp the gap is silent"
    );
}

/// The other edge: the first buffer after a gap starts at silence too.
#[test]
fn the_buffer_after_a_gap_ramps_back_in() {
    let mut scratch = vec![1.0f32; 256 * CHANNELS];
    ramp_up(&mut scratch);

    assert!(scratch[0] < 0.1, "resuming must not step up from silence");
    for frame in 1..DECLICK_FRAMES {
        assert!(scratch[frame * CHANNELS] > scratch[(frame - 1) * CHANNELS]);
    }
    assert!(
        scratch[DECLICK_FRAMES * CHANNELS..]
            .iter()
            .all(|s| *s == 1.0),
        "past the ramp the signal is untouched"
    );
}

/// A buffer shorter than the ramp still reaches silence rather than stopping
/// part-way down it - the case a 32-frame PipeWire quantum actually hits.
#[test]
fn a_buffer_shorter_than_the_ramp_still_reaches_silence() {
    let mut scratch = vec![1.0f32; 8 * CHANNELS];
    fade_out(&mut scratch);
    assert_eq!(scratch[7 * CHANNELS], 0.0);

    let mut scratch = vec![0.0f32; 8 * CHANNELS];
    ramp_from(&mut scratch, [1.0, 1.0]);
    assert_eq!(scratch[7 * CHANNELS], 0.0);

    let mut scratch = vec![1.0f32; 8 * CHANNELS];
    ramp_up(&mut scratch);
    assert_eq!(scratch[7 * CHANNELS], 1.0);
}

/// The tail is read off what was played, so a gap ramps from where the music
/// actually was rather than from an assumed full scale.
#[test]
fn the_tail_follows_the_last_played_frame() {
    let played = [0.9, -0.9, 0.25, -0.25];
    assert_eq!(last_frame(&played), [0.25, -0.25]);
    assert_eq!(last_frame(&[]), [0.0, 0.0]);
}

/// The depth a caller names is a **ceiling** on how late a cue is heard, so the
/// thread holds the ring at the target and the ring is one chunk larger so a
/// write always fits.
#[test]
fn the_target_is_the_latency_and_the_ring_is_a_chunk_larger() {
    let target = Duration::from_millis(60);
    let held = render::target_samples(48_000, target) / CHANNELS;
    assert_eq!(held, 2_880, "60 ms of 48 kHz");
    assert!(
        render::ring_capacity(48_000, target) > render::target_samples(48_000, target),
        "the ring has room to write past the target"
    );
}

/// The render thread fills the ring and stops when its handle drops.
///
/// **The one test of the new shape a machine with no sound card can run.**
/// There is no device here and none is needed: the thread renders a real mixer
/// into a real ring, which is every part of it except the copy out.
#[test]
fn the_render_thread_fills_the_ring_and_stops_when_dropped() {
    let mixer = Arc::new(Mutex::new(Mixer::new(44_100)));
    mixer
        .lock()
        .unwrap()
        .play(Play::looping(tone(), Bus::Music))
        .expect("a voice");

    let target = render::target_samples(44_100, Duration::from_millis(60));
    let (producer, mut consumer) =
        rtrb::RingBuffer::new(render::ring_capacity(44_100, Duration::from_millis(60)));
    let ahead = render::Ahead::spawn(
        Arc::clone(&mixer),
        producer,
        target,
        44_100,
        Arc::new(Spectrum::new()),
    );

    // It polls, so give it a few passes to fill rather than assuming one.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while consumer.slots() < target / 2 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let held = consumer.slots();
    assert!(
        held >= target / 2,
        "the ring held {held} sample(s) after five seconds"
    );
    // And it stops at the target rather than filling to the brim, because the
    // occupancy is the latency.
    assert!(
        held <= target,
        "{held} sample(s) is past the {target} it was told to hold"
    );

    let mut out = vec![0.0f32; 2_048];
    let (_, unfilled) = consumer.pop_partial_slice(&mut out);
    assert!(unfilled.is_empty(), "and it hands out what it rendered");
    assert!(out.iter().any(|s| *s != 0.0), "which is audio, not zeroes");

    drop(ahead);
    // Dropping joins, so by here nothing is writing: the count cannot move.
    let settled = consumer.slots();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(consumer.slots(), settled, "the thread is still rendering");
}

/// The counters a loaded machine is diagnosed with. A null output has no
/// callback, so this is the floor rather than the behaviour under contention.
#[test]
fn a_silent_output_has_seen_nothing() {
    let output = Output::null(44_100);
    assert_eq!(output.health().dropped_buffers(), 0);
    assert_eq!(output.health().jumps(), 0);
    assert_eq!(output.health().late_callbacks(), 0);
    // Reports nothing, and is not due to on the first frame either.
    output.report_health();
}

/// A jump is a step between two adjacent buffers, and the threshold is what
/// keeps ordinary loud audio from reading as one.
#[test]
fn a_step_between_buffers_is_counted_and_a_small_one_is_not() {
    let health = Health::default();
    health.scan(&[0.55, 0.45], [0.5, 0.5]);
    assert_eq!(health.jumps(), 0, "ordinary movement is not a click");

    health.scan(&[-0.8, 0.0], [0.8, 0.0]);
    assert_eq!(health.jumps(), 1, "a step across full scale is");
}

/// The blind spot this had until 2026-08-31: a voice ends wherever its playhead
/// runs out, which is almost never the first frame of a buffer.
#[test]
fn a_step_in_the_middle_of_a_buffer_is_counted_too() {
    let health = Health::default();
    let mut buffer = vec![0.7f32; 64 * CHANNELS];
    for slot in &mut buffer[40 * CHANNELS..] {
        *slot = 0.0;
    }
    health.scan(&buffer, [0.7, 0.7]);
    assert_eq!(health.jumps(), 1, "a cut at frame 40 is still a cut");
}

/// Late callbacks keep the worst overshoot, because the count alone does not
/// say whether the machine missed by a millisecond or by a buffer.
#[test]
fn a_late_callback_is_counted_with_its_worst_overshoot() {
    let health = Health::default();
    health.late(1_000);
    health.late(9_000);
    health.late(2_000);
    assert_eq!(health.late_callbacks(), 3);
}

/// The tap fills once and is taken once, which is what makes writing it from
/// the frame loop safe to call sixty times a second.
#[test]
fn a_tap_fills_to_its_capacity_and_is_taken_exactly_once() {
    let tap = Tap::new(8);
    assert!(!tap.is_full());

    tap.push(&[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(tap.recorded(), 4);
    assert!(!tap.is_full());

    // Past the end it keeps what fits and drops the rest rather than growing,
    // because growing is allocating and this runs on the audio thread.
    tap.push(&[5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
    assert_eq!(tap.recorded(), 8);
    assert!(tap.is_full());

    assert_eq!(
        tap.take().expect("the first take"),
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
    );
    assert!(tap.take().is_none(), "a recording is written once");
    assert!(!tap.is_full());
}

/// A device that advertises `f32` and refuses it (AAudio on a Galaxy S24) must
/// get `i16` tried next, and a device whose default is `i16` must not be asked
/// for it twice.
#[test]
fn a_refused_default_format_falls_back_to_i16_then_f32() {
    use cpal::SampleFormat::{F32, I16, U16};
    assert_eq!(formats_to_try(F32), vec![F32, I16]);
    assert_eq!(formats_to_try(I16), vec![I16, F32]);
    assert_eq!(formats_to_try(U16), vec![U16, I16, F32]);
}
