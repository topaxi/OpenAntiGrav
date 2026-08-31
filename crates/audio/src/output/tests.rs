//! `Output`'s tests, out of line because the inline-test ceiling is 200 lines
//! and the contended callback path needs more than the format conversions did.

use super::*;
use crate::mixer::{Bus, Play, Sound};

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

/// The uncontended path is unchanged: the mixer renders and nothing is counted.
#[test]
fn an_uncontended_buffer_is_rendered() {
    let mixer = Mutex::new(Mixer::new(44_100));
    mixer
        .lock()
        .unwrap()
        .play(Play::looping(tone(), Bus::Music))
        .expect("a voice");

    let mut scratch = Vec::new();
    let mut declick = Declick::default();
    assert!(render_stereo(
        &mixer,
        &mut scratch,
        256,
        Duration::from_micros(100),
        &mut declick
    ));
    assert_eq!(scratch.len(), 256 * CHANNELS);
    assert!(scratch.iter().any(|s| *s != 0.0));
    assert!(!declick.recovering);
}

/// The point of the waiting budget: a lock held for less than it is a late
/// buffer, not a lost one.
///
/// The budget here is two seconds against a 2 ms hold, which is nothing like a
/// real one. That is deliberate: the assertion is about the *mechanism* - that
/// the callback waits at all - and a margin that wide survives Windows, whose
/// default timer resolution is ~15.6 ms and turns a 2 ms sleep into a 15 ms
/// one. The behaviour this guards against, a single `try_lock`, returns false
/// in microseconds no matter how generous the budget is.
#[test]
fn a_briefly_held_lock_is_waited_out_rather_than_dropped() {
    let mixer = Arc::new(Mutex::new(Mixer::new(44_100)));
    mixer
        .lock()
        .unwrap()
        .play(Play::looping(tone(), Bus::Music))
        .expect("a voice");

    let held = Arc::clone(&mixer);
    let (tx, rx) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let guard = held.lock().unwrap();
        tx.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(2));
        drop(guard);
    });
    rx.recv().expect("the holder to have taken the lock");

    let mut scratch = Vec::new();
    let mut declick = Declick::default();
    let rendered = render_stereo(
        &mixer,
        &mut scratch,
        256,
        Duration::from_secs(2),
        &mut declick,
    );
    holder.join().unwrap();

    assert!(
        rendered,
        "a hold well inside the budget must not cost a buffer"
    );
    assert!(scratch.iter().any(|s| *s != 0.0));
}

/// And past the budget it gives up rather than blocking - the property that
/// keeps the device's deadline out of the frame loop's hands.
#[test]
fn a_lock_held_past_the_budget_gives_up() {
    let mixer = Mutex::new(Mixer::new(44_100));
    let guard = mixer.lock().expect("the lock");

    let mut scratch = Vec::new();
    let mut declick = Declick::default();
    let rendered = render_stereo(
        &mixer,
        &mut scratch,
        256,
        Duration::from_micros(200),
        &mut declick,
    );
    drop(guard);

    assert!(!rendered);
    assert_eq!(scratch.len(), 256 * CHANNELS);
    assert!(declick.recovering, "the next buffer has to ramp back in");
}

/// The declick itself, and the reason this is not a `fill(0.0)`: a gap that
/// starts with a step from full scale to zero is a click, which is louder than
/// the millisecond it replaces.
#[test]
fn a_dropped_buffer_ramps_down_from_the_last_sample_rather_than_cutting() {
    let mut scratch = vec![0.0; 256 * CHANNELS];
    ramp_down(&mut scratch, [1.0, -1.0]);

    assert!(
        scratch[0] > 0.9,
        "the ramp has to start next to the last sample, not at zero"
    );
    assert!(scratch[1] < -0.9);
    // Monotone down over the ramp, then silence for the rest of the buffer.
    for frame in 1..DECLICK_FRAMES {
        assert!(scratch[frame * CHANNELS] < scratch[(frame - 1) * CHANNELS]);
    }
    assert!(
        scratch[DECLICK_FRAMES * CHANNELS..]
            .iter()
            .all(|s| *s == 0.0),
        "past the ramp a dropped buffer is silent"
    );
}

/// The other edge: the first buffer after a gap starts at silence too.
#[test]
fn the_buffer_after_a_drop_ramps_back_in() {
    let mut scratch = vec![1.0; 256 * CHANNELS];
    ramp_up(&mut scratch);

    assert!(scratch[0] < 0.1, "resuming must not step up from silence");
    for frame in 1..DECLICK_FRAMES {
        assert!(scratch[frame * CHANNELS] > scratch[(frame - 1) * CHANNELS]);
    }
    assert!(
        scratch[DECLICK_FRAMES * CHANNELS..]
            .iter()
            .all(|s| *s == 1.0),
        "past the ramp the rendered signal is untouched"
    );
}

/// A buffer shorter than the ramp still ends at silence rather than part-way
/// down it - the case a 32-frame PipeWire quantum actually hits.
#[test]
fn a_buffer_shorter_than_the_ramp_still_reaches_silence() {
    let mut scratch = vec![0.0; 8 * CHANNELS];
    ramp_down(&mut scratch, [1.0, 1.0]);
    assert_eq!(scratch[7 * CHANNELS], 0.0);
    assert_eq!(scratch[7 * CHANNELS + 1], 0.0);

    let mut scratch = vec![1.0; 8 * CHANNELS];
    ramp_up(&mut scratch);
    assert_eq!(scratch[7 * CHANNELS], 1.0);
}

/// The tail is read off what was rendered, so the ramp starts where the music
/// actually was rather than at an assumed full scale.
#[test]
fn the_tail_follows_the_last_rendered_frame() {
    let rendered = [0.9, -0.9, 0.25, -0.25];
    assert_eq!(last_frame(&rendered), [0.25, -0.25]);
    assert_eq!(last_frame(&[]), [0.0, 0.0]);
}

/// A poisoned lock gives up at once instead of spending a budget per buffer on
/// a lock that will never be free again.
#[test]
fn a_poisoned_lock_is_not_waited_on() {
    let mixer = Arc::new(Mutex::new(Mixer::new(44_100)));
    let poison = Arc::clone(&mixer);
    let _ = std::thread::spawn(move || {
        let _guard = poison.lock().unwrap();
        panic!("poisoning the mixer on purpose");
    })
    .join();

    // Ten seconds against a one-second assertion, for the same reason the test
    // above is generous: correct behaviour returns in microseconds and the
    // behaviour being guarded against takes the whole budget, so the margin can
    // be as wide as a loaded CI machine needs.
    let started = std::time::Instant::now();
    assert!(acquire(&mixer, Duration::from_secs(10)).is_none());
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "a poisoned lock must not be waited out"
    );
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

/// The buffer this asks a device for: the target, clamped into what it offers.
#[test]
fn the_requested_buffer_is_forty_milliseconds_of_the_devices_own_rate() {
    let range = cpal::SupportedBufferSize::Range { min: 32, max: 8192 };
    assert_eq!(
        requested_buffer_size(&range, 48_000, MIN_BUFFER),
        cpal::BufferSize::Fixed(1920)
    );
    assert_eq!(
        requested_buffer_size(&range, 44_100, MIN_BUFFER),
        cpal::BufferSize::Fixed(1764)
    );
}

/// A device that cannot reach 40 ms gets its own ceiling rather than a request
/// it would refuse to build a stream for.
#[test]
fn a_narrow_range_is_clamped_rather_than_overrun() {
    let small = cpal::SupportedBufferSize::Range { min: 64, max: 256 };
    assert_eq!(
        requested_buffer_size(&small, 48_000, MIN_BUFFER),
        cpal::BufferSize::Fixed(256)
    );
    let large = cpal::SupportedBufferSize::Range {
        min: 4096,
        max: 8192,
    };
    assert_eq!(
        requested_buffer_size(&large, 48_000, MIN_BUFFER),
        cpal::BufferSize::Fixed(4096)
    );
}

/// And a device that will not say keeps its own default: a fixed size guessed
/// against an unknown range is how a stream fails to build at all.
/// A caller with a slower loop asks for more, which is the whole point of the
/// target being a parameter: 40 ms is two and a half frames at 60 Hz and one
/// and a fifth at 30.
#[test]
fn a_slower_loop_asks_for_a_bigger_buffer() {
    let range = cpal::SupportedBufferSize::Range { min: 32, max: 8192 };
    let thirty_hz = Duration::from_millis(83);
    let cpal::BufferSize::Fixed(frames) = requested_buffer_size(&range, 48_000, thirty_hz) else {
        panic!("a stated range gets a fixed size");
    };
    // 83 ms of 48 kHz, give or take what a single-precision second costs.
    assert!(
        (3_980..=3_990).contains(&frames),
        "{frames} frames is not 83 ms"
    );
}

#[test]
fn an_unknown_range_is_left_alone() {
    assert_eq!(
        requested_buffer_size(&cpal::SupportedBufferSize::Unknown, 48_000, MIN_BUFFER),
        cpal::BufferSize::Default
    );
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
