//! The device half: a real `cpal` stream, or nothing at all.
//!
//! [`Output`] owns a [`Mixer`] behind a lock and, when there is a device, the
//! stream whose callback drains it. The composition root holds one of these;
//! no gameplay crate ever sees it.
//!
//! # Why a null backend is not a test fixture
//!
//! CI has no sound card, this project's own runs are headless, and
//! `--screenshot` captures must not depend on one either. [`Output::null`] is
//! therefore a first-class mode rather than something tests reach for -
//! [`Mixer`] still advances, [`Output::render_tick`] still produces samples,
//! and only the handoff to hardware is missing. That mirrors
//! `oag_input::Controls::without_pad`, which exists for exactly the same reason.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::{debug, error, warn};

use crate::mixer::{CHANNELS, Mixer};

/// The rate used when there is no device to ask.
///
/// 44,100 Hz because that is what every ATRAC3+ stream on the disc is; the PS2
/// PCM archives are 48,000 and get resampled, which is the direction that costs
/// nothing.
pub const DEFAULT_SAMPLE_RATE: u32 = 44_100;

/// The share of a buffer's own playing time the callback will spend waiting for
/// the mixer lock before giving the buffer up.
///
/// A quarter, so that even a callback which spends the whole budget still has
/// three quarters of the period left to render and convert in. Contention here
/// is measured in microseconds - the control side takes the lock to start,
/// stop or retune a voice and nothing else - so the budget is far more than a
/// collision costs, and matters only for the collision that happens because
/// the *holder* was preempted, which is the one this cannot win anyway.
const SPIN_FRACTION: f32 = 0.25;

/// The ceiling on that share, for a device that asks for very long buffers.
const SPIN_CAP: Duration = Duration::from_micros(500);

/// Spins between each yield while waiting for the lock.
///
/// **Not a pure spin.** The thread holding the mixer lock is, on a loaded
/// machine, the thread that just got preempted, and refusing to yield the core
/// is precisely what stops it finishing. Spinning is right for the microsecond
/// case and yielding is right for the preempted one, so this does both.
const SPINS_PER_YIELD: u32 = 64;

/// Frames a dropped buffer ramps over, either side of the gap.
///
/// About 1.5 ms at 44.1 kHz. A dropped buffer used to be filled with zeroes
/// outright, which puts a step discontinuity into the signal wherever the
/// waveform happened to be - and a step is a click, which is far more audible
/// than the millisecond of missing music around it. The mixer's playback
/// position does not advance across a miss (`render` never ran), so both edges
/// are discontinuities of amplitude alone and a short ramp removes them.
const DECLICK_FRAMES: usize = 64;

/// Frames between two reports of the dropped-buffer count, at 60 Hz about five
/// seconds. See [`Output::report_dropouts`].
const REPORT_EVERY: u32 = 300;

/// A mixer, and optionally the device draining it.
pub struct Output {
    mixer: Arc<Mutex<Mixer>>,
    /// Dropping this stops the stream, so it is held even though nothing reads
    /// it. `None` is the null backend.
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    device: Option<String>,
    /// Buffers the callback gave up on, shared with the callback itself.
    ///
    /// The mixer's own `starved` counter is the precedent: a failure the
    /// player can hear should be one a maintainer can count. Unlike `starved`
    /// this one is read by [`Output::report_dropouts`] rather than by tests
    /// alone, because it is the only way a machine that cannot open a window
    /// learns whether the machine that can is still dropping buffers.
    dropped: Arc<AtomicU64>,
    /// What [`Output::report_dropouts`] has already said, and how long ago.
    reported: AtomicU64,
    since_report: AtomicU32,
}

// `cpal::Stream` is deliberately not `Debug`, and the workspace warns on a
// missing one, so this is written out rather than derived.
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Output")
            .field("sample_rate", &self.sample_rate)
            .field("device", &self.device)
            .field("streaming", &self.stream.is_some())
            .field("dropped", &self.dropped_buffers())
            .finish()
    }
}

impl Output {
    /// A mixer with no device behind it.
    ///
    /// Never fails, which is the point: a machine with no sound card runs the
    /// game rather than refusing to start.
    #[must_use]
    pub fn null(sample_rate: u32) -> Self {
        let rate = if sample_rate == 0 {
            DEFAULT_SAMPLE_RATE
        } else {
            sample_rate
        };
        Self {
            mixer: Arc::new(Mutex::new(Mixer::new(rate))),
            stream: None,
            sample_rate: rate,
            device: None,
            dropped: Arc::new(AtomicU64::new(0)),
            reported: AtomicU64::new(0),
            since_report: AtomicU32::new(0),
        }
    }

    /// Opens the default output device.
    ///
    /// # Errors
    ///
    /// If there is no default device, its configuration cannot be read, or the
    /// stream cannot be built or started. Callers that would rather be silent
    /// than fail should use [`Output::open_or_null`].
    pub fn open() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no default audio output device")?;
        // cpal 0.18 replaced `Device::name` with a richer description; the
        // name is only used for messages, so an unreadable one is not fatal.
        let name = device
            .description()
            .map_or_else(|_| "<unnamed>".to_string(), |d| d.name().to_string());
        let supported = device
            .default_output_config()
            .with_context(|| format!("reading the default output config of {name}"))?;

        let config: cpal::StreamConfig = supported.config();
        // `cpal::SampleRate` is a plain `u32` alias as of 0.18, not a newtype.
        let sample_rate = config.sample_rate;

        let mixer = Arc::new(Mutex::new(Mixer::new(sample_rate)));
        let dropped = Arc::new(AtomicU64::new(0));

        // **The device's own sample format, not `f32`.** This built an `f32`
        // stream unconditionally until finding U1 of the 2026-08-18 review, and
        // a device whose default format is anything else - bare ALSA `hw:` is
        // routinely `i16` - failed at `build_output_stream` and, through
        // `open_or_null`, left the whole session silent on a working card. The
        // mixer still renders `f32` and the conversion happens on the way out;
        // `spread` is generic over the destination for that reason.
        let format = supported.sample_format();
        let stream = match format {
            cpal::SampleFormat::F32 => build::<f32>(&device, config, &mixer, &dropped),
            cpal::SampleFormat::F64 => build::<f64>(&device, config, &mixer, &dropped),
            cpal::SampleFormat::I8 => build::<i8>(&device, config, &mixer, &dropped),
            cpal::SampleFormat::I16 => build::<i16>(&device, config, &mixer, &dropped),
            cpal::SampleFormat::I32 => build::<i32>(&device, config, &mixer, &dropped),
            cpal::SampleFormat::I64 => build::<i64>(&device, config, &mixer, &dropped),
            cpal::SampleFormat::U8 => build::<u8>(&device, config, &mixer, &dropped),
            cpal::SampleFormat::U16 => build::<u16>(&device, config, &mixer, &dropped),
            cpal::SampleFormat::U32 => build::<u32>(&device, config, &mixer, &dropped),
            cpal::SampleFormat::U64 => build::<u64>(&device, config, &mixer, &dropped),
            // `SampleFormat` is `#[non_exhaustive]`, and the packed 24-bit and
            // DSD formats have no `FromSample<f32>` to convert through. Named
            // rather than silently silent, which is the failure this arm's
            // siblings exist to end.
            other => Err(anyhow::anyhow!("unsupported sample format {other}")),
        }
        .with_context(|| format!("building a {format} output stream on {name}"))?;

        Ok(Self {
            mixer,
            stream: Some(stream),
            sample_rate,
            device: Some(name),
            dropped,
            reported: AtomicU64::new(0),
            since_report: AtomicU32::new(0),
        })
    }

    /// Opens the default device, falling back to silence with a note on stdout.
    ///
    /// The same degradation the video path already takes when its decoder is
    /// missing: say what is absent, by name, and carry on.
    #[must_use]
    pub fn open_or_null() -> Self {
        match Self::open() {
            Ok(output) => output,
            Err(error) => {
                warn!("audio: no output device ({error:#}); running silent");
                Self::null(DEFAULT_SAMPLE_RATE)
            }
        }
    }

    /// The rate the mixer renders at.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The device's name, or `None` when running silent.
    #[must_use]
    pub fn device_name(&self) -> Option<&str> {
        self.device.as_deref()
    }

    /// Whether a real device is attached.
    #[must_use]
    pub fn is_streaming(&self) -> bool {
        self.stream.is_some()
    }

    /// Buffers the output callback gave up on rather than render.
    ///
    /// Each one is a gap of roughly a device buffer in every voice at once -
    /// ramped at both edges so it is a gap and not a click, but still audible
    /// as a hiccup in music. Non-zero means the mixer lock was held past the
    /// callback's whole waiting budget, which in practice means the thread
    /// holding it was preempted, which in practice means the machine is
    /// loaded.
    #[must_use]
    pub fn dropped_buffers(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Logs the dropped-buffer count when it grows, at most every
    /// [`REPORT_EVERY`] calls.
    ///
    /// **Called from the frame loop, never from the callback.** Formatting a
    /// message allocates, and the audio thread is the one place in this crate
    /// where an allocation is a dropout of its own; the callback therefore only
    /// ever bumps an atomic and this reads it back from a thread that can
    /// afford the sentence. Throttled by frames rather than by a clock because
    /// nothing in this crate reads one.
    pub fn report_dropouts(&self) {
        let dropped = self.dropped.load(Ordering::Relaxed);
        let reported = self.reported.load(Ordering::Relaxed);
        if dropped == reported {
            self.since_report.store(0, Ordering::Relaxed);
            return;
        }
        if self.since_report.fetch_add(1, Ordering::Relaxed) < REPORT_EVERY && reported != 0 {
            return;
        }
        self.since_report.store(0, Ordering::Relaxed);
        self.reported.store(dropped, Ordering::Relaxed);
        // The first one is a warning because it says something about the
        // machine the player is on; the rest are debug, because by then they
        // have been told and a race has better uses for the terminal.
        if reported == 0 {
            warn!(
                "audio: dropped {dropped} buffer(s) to mixer-lock contention - \
                 a short gap in every voice. Expected under heavy load."
            );
        } else {
            debug!("audio: {dropped} buffer(s) dropped to mixer-lock contention so far");
        }
    }

    /// Runs `f` against the mixer.
    ///
    /// This is how a tick starts, stops and retunes voices. It blocks against
    /// the audio callback, which only ever holds the lock for one buffer.
    ///
    /// # Panics
    ///
    /// If a previous call panicked while holding the lock.
    pub fn with_mixer<T>(&self, f: impl FnOnce(&mut Mixer) -> T) -> T {
        let mut mixer = self.mixer.lock().expect("the audio mixer lock is poisoned");
        f(&mut mixer)
    }

    /// Renders one tick's audio into `out`, for the offline dump.
    ///
    /// Only meaningful on a null backend: with a device attached the callback
    /// is already draining the mixer, and pulling from it here would race the
    /// hardware for the same samples. Returns the frames written, or zero if a
    /// device is attached.
    ///
    /// # Panics
    ///
    /// If a previous call panicked while holding the lock.
    pub fn render_tick(&self, tick_hz: u32, out: &mut Vec<f32>) -> usize {
        if self.stream.is_some() {
            return 0;
        }
        self.with_mixer(|mixer| mixer.render_tick(tick_hz, out))
    }
}

/// Builds and starts one output stream in the device's own sample format.
///
/// Generic over `T` so the fifteen-line callback is written once rather than
/// once per format; `cpal`'s own `FromSample` does the conversion, which is the
/// same integer scaling this crate's `Sound` decoding uses in the other
/// direction.
fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mixer: &Arc<Mutex<Mixer>>,
    dropped: &Arc<AtomicU64>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32> + Send + 'static,
{
    let device_channels = usize::from(config.channels).max(1);
    let callback_mixer = Arc::clone(mixer);
    let callback_dropped = Arc::clone(dropped);
    // Seconds per frame, so the waiting budget can be derived from the size of
    // the buffer actually handed over - a PipeWire quantum of 64 frames is
    // 1.45 ms and a 2,048-frame ALSA period is 46 ms, and a budget that suits
    // one is wrong for the other.
    let seconds_per_frame = 1.0 / f64::from(config.sample_rate.max(1)) as f32;
    // Rendered stereo, before it is spread over however many channels the
    // device actually has. Allocated once here rather than in the callback.
    let mut scratch: Vec<f32> = Vec::new();
    let mut declick = Declick::default();

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let frames = data.len() / device_channels;
            let budget = Duration::from_secs_f32(frames as f32 * seconds_per_frame * SPIN_FRACTION)
                .min(SPIN_CAP);

            if !render_stereo(&callback_mixer, &mut scratch, frames, budget, &mut declick) {
                callback_dropped.fetch_add(1, Ordering::Relaxed);
            }

            spread(&scratch, data, device_channels);
        },
        move |err| error!("audio: output stream error: {err}"),
        None,
    )?;
    stream.play()?;
    Ok(stream)
}

/// What the callback remembers from one buffer to the next, so that a dropped
/// one is a gap rather than a click. See [`DECLICK_FRAMES`].
#[derive(Debug, Default, Clone, Copy)]
struct Declick {
    /// The last stereo frame actually emitted, which a give-up ramps down from.
    tail: [f32; CHANNELS],
    /// Whether the previous buffer was given up on, so this one ramps back in.
    recovering: bool,
}

/// Fills `scratch` with `frames` stereo frames, returning whether the mixer
/// rendered them.
///
/// A `false` return means the lock was still held when the budget ran out and
/// the buffer is a declicked gap. Split out of the closure so that a machine
/// with no sound card can still test the contended path, which is the one that
/// only ever happens when nobody is watching.
fn render_stereo(
    mixer: &Mutex<Mixer>,
    scratch: &mut Vec<f32>,
    frames: usize,
    budget: Duration,
    declick: &mut Declick,
) -> bool {
    scratch.clear();
    scratch.resize(frames * CHANNELS, 0.0);

    let Some(mut guard) = acquire(mixer, budget) else {
        ramp_down(scratch, declick.tail);
        declick.tail = [0.0; CHANNELS];
        declick.recovering = true;
        return false;
    };

    guard.render(scratch);
    if declick.recovering {
        ramp_up(scratch);
        declick.recovering = false;
    }
    declick.tail = last_frame(scratch);
    true
}

/// Waits up to `budget` for the mixer lock without ever blocking on it.
///
/// **Not `lock`.** Blocking the audio thread on a tick that is mid-update
/// hands the device's deadline to the frame loop, and missing that deadline is
/// an underrun in the driver rather than a gap this code can shape. **Not a
/// bare `try_lock` either**, which is what this was: the control side holds the
/// lock for microseconds at a time, so almost every collision is one a short
/// wait wins outright, and giving up on the first attempt threw those away.
fn acquire<'a>(mixer: &'a Mutex<Mixer>, budget: Duration) -> Option<MutexGuard<'a, Mixer>> {
    // Before any clock is read, because the uncontended case is the common one
    // and it should cost exactly what it did before this function existed.
    match mixer.try_lock() {
        Ok(guard) => return Some(guard),
        // A poisoned lock never becomes unpoisoned, so waiting on it is waiting
        // forever, once per buffer.
        Err(TryLockError::Poisoned(_)) => return None,
        Err(TryLockError::WouldBlock) => {}
    }

    let deadline = Instant::now() + budget;
    loop {
        for _ in 0..SPINS_PER_YIELD {
            std::hint::spin_loop();
        }
        std::thread::yield_now();
        match mixer.try_lock() {
            Ok(guard) => return Some(guard),
            Err(TryLockError::Poisoned(_)) => return None,
            Err(TryLockError::WouldBlock) => {}
        }
        if Instant::now() >= deadline {
            return None;
        }
    }
}

/// The last stereo frame in a rendered buffer, or silence if there is none.
fn last_frame(stereo: &[f32]) -> [f32; CHANNELS] {
    let frames = stereo.len() / CHANNELS;
    if frames == 0 {
        return [0.0; CHANNELS];
    }
    let at = (frames - 1) * CHANNELS;
    [stereo[at], stereo[at + 1]]
}

/// Ramps a silent buffer down from `tail`, so a gap does not start with a step.
fn ramp_down(stereo: &mut [f32], tail: [f32; CHANNELS]) {
    let frames = stereo.len() / CHANNELS;
    let ramp = frames.min(DECLICK_FRAMES);
    for frame in 0..ramp {
        let gain = 1.0 - (frame + 1) as f32 / ramp as f32;
        let at = frame * CHANNELS;
        stereo[at] = tail[0] * gain;
        stereo[at + 1] = tail[1] * gain;
    }
}

/// Ramps a rendered buffer up from silence, the other edge of the same gap.
fn ramp_up(stereo: &mut [f32]) {
    let frames = stereo.len() / CHANNELS;
    let ramp = frames.min(DECLICK_FRAMES);
    for frame in 0..ramp {
        let gain = (frame + 1) as f32 / ramp as f32;
        let at = frame * CHANNELS;
        stereo[at] *= gain;
        stereo[at + 1] *= gain;
    }
}

/// Copies interleaved stereo into a buffer of `channels` channels, converting
/// to the device's sample type on the way.
///
/// Fewer than two channels take the left; more than two get silence in the
/// extras rather than a copy, because duplicating a stereo pair into surrounds
/// is a mix decision and not one this layer should be making quietly.
fn spread<T: cpal::FromSample<f32> + cpal::Sample>(stereo: &[f32], out: &mut [T], channels: usize) {
    let silence = T::from_sample_(0.0f32);
    if channels == CHANNELS {
        let n = out.len().min(stereo.len());
        for (slot, &sample) in out[..n].iter_mut().zip(&stereo[..n]) {
            *slot = T::from_sample_(sample);
        }
        out[n..].fill(silence);
        return;
    }
    out.fill(silence);
    for (frame, chunk) in out.chunks_exact_mut(channels).enumerate() {
        let at = frame * CHANNELS;
        if at + 1 >= stereo.len() {
            break;
        }
        if channels == 1 {
            chunk[0] = T::from_sample_((stereo[at] + stereo[at + 1]) * 0.5);
        } else {
            chunk[0] = T::from_sample_(stereo[at]);
            chunk[1] = T::from_sample_(stereo[at + 1]);
        }
    }
}

#[cfg(test)]
mod tests;
