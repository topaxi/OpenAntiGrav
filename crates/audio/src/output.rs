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

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::{error, warn};

use crate::mixer::{CHANNELS, Mixer};
use crate::spectrum::Spectrum;

mod health;
mod render;
mod tap;
pub use health::Health;
pub use tap::Tap;

/// What `--tap-audio` asks for: how much to record, and where to put it.
#[derive(Debug, Clone)]
pub struct TapSpec {
    /// Seconds of output to keep. The recording starts when the stream does.
    pub seconds: f32,
    pub path: std::path::PathBuf,
}

/// The rate used when there is no device to ask.
///
/// 44,100 Hz because that is what every ATRAC3+ stream on the disc is; the PS2
/// PCM archives are 48,000 and get resampled, which is the direction that costs
/// nothing.
pub const DEFAULT_SAMPLE_RATE: u32 = 44_100;

/// The least audio this project renders ahead of the device.
///
/// **60 ms, and it is a real number now rather than a request.** It used to be
/// a buffer size asked of the device, which the ALSA host honoured and the
/// PipeWire host ignored - see [`render`] for that measurement. The audio is
/// held in this crate's own ring instead, so this is how long a stall the queue
/// covers on every host, and it is also exactly how late a cue is heard.
///
/// 60 ms is three and a half frames at 60 Hz, chosen against the stalls
/// actually measured in a Wipeout HD race on the machine that reported the
/// fault: frames of 40 to 80 ms, with a callback 25 ms late behind them. It
/// does not cover the worst of those and is not meant to - past here the delay
/// on a collision starts being something a player can feel, and the honest
/// place to spend the rest is the stall.
///
/// **It is a ceiling and not a constant.** The render thread stops a chunk
/// short of it rather than filling to the brim, so a cue is heard between
/// `60 ms - CHUNK` and 60 ms after the tick that raised it - about 49 to 60 ms
/// at 48 kHz. Against what it replaced that is roughly 40 ms added: the device
/// buffer this used to rely on was 5 to 21 ms on PipeWire, and cue emission
/// already cost up to a tick on top of either.
///
/// **A caller whose loop is slower than 60 Hz should ask for more**, because
/// what matters is frames rather than milliseconds: at a 30 Hz cap this is one
/// and four fifths of a frame.
pub const MIN_BUFFER: Duration = Duration::from_millis(60);

/// Frames a starved buffer ramps over, either side of the gap.
///
/// About 1.5 ms at 44.1 kHz. A buffer the ring could not fill used to be
/// zeroed outright, which puts a step discontinuity into the signal wherever
/// the waveform happened to be - and a step is a click, which is far more
/// audible than the millisecond of missing music around it.
const DECLICK_FRAMES: usize = 64;

/// A mixer, and optionally the device draining it.
pub struct Output {
    mixer: Arc<Mutex<Mixer>>,
    /// Dropping this stops the stream, so it is held even though nothing reads
    /// it. `None` is the null backend.
    stream: Option<cpal::Stream>,
    /// The thread filling the ring the callback drains.
    ///
    /// Declared after [`Self::stream`] so it is dropped after it: the device
    /// stops asking first, and only then is the thread told to stop and joined.
    /// Either order is safe - a ring whose far half has gone simply stops
    /// accepting or yielding - but this one never leaves the callback reading a
    /// ring nobody is filling.
    ahead: Option<render::Ahead>,
    sample_rate: u32,
    device: Option<String>,
    /// What the callback saw, shared with the callback itself.
    ///
    /// The mixer's own `starved` counter is the precedent: a failure the
    /// player can hear should be one a maintainer can count. Unlike `starved`
    /// this one is read by [`Output::report_health`] rather than by tests
    /// alone, because it is the only way a machine that cannot open a window
    /// learns what is happening on the machine that can.
    health: Arc<Health>,
    /// The recording `--tap-audio` asked for, and where it goes.
    tap: Option<(Arc<Tap>, std::path::PathBuf)>,
    /// The Zone visualiser's own input: a live spectrum of what
    /// [`Self::ahead`] is actually rendering. `Arc`'d rather than owned
    /// outright because [`render::Ahead`]'s thread writes it directly - see
    /// [`Self::spectrum`].
    spectrum: Arc<Spectrum>,
}

// `cpal::Stream` is deliberately not `Debug`, and the workspace warns on a
// missing one, so this is written out rather than derived.
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Output")
            .field("sample_rate", &self.sample_rate)
            .field("device", &self.device)
            .field("streaming", &self.stream.is_some())
            .field("rendering ahead", &self.ahead.is_some())
            .field("dropped", &self.health.dropped_buffers())
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
            health: Arc::new(Health::default()),
            ahead: None,
            tap: None,
            // No render-ahead thread ever runs on the null backend, so this
            // never publishes - which is the honest reading for a machine
            // with nothing playing, not an invented level.
            spectrum: Arc::new(Spectrum::new()),
        }
    }

    /// Pauses or restarts the device's stream, for a window that is not on
    /// screen (Android `Suspended`, a minimised desktop window).
    ///
    /// The mixer and its render-ahead thread are untouched: the ring simply
    /// fills and the thread waits, so a voice resumes where it was rather than
    /// being cut. The null backend has no stream and does nothing. A host that
    /// cannot pause a stream is logged and left playing; that is the only
    /// failure there is.
    pub fn set_paused(&self, paused: bool) {
        let Some(stream) = &self.stream else {
            return;
        };
        let result = if paused {
            stream.pause()
        } else {
            stream.play()
        };
        if let Err(e) = result {
            warn!(
                "audio: could not {} the stream: {e}",
                if paused { "pause" } else { "restart" }
            );
        }
    }

    /// Opens the default output device.
    ///
    /// # Errors
    ///
    /// If there is no default device, its configuration cannot be read, or the
    /// stream cannot be built or started. Callers that would rather be silent
    /// than fail should use [`Output::open_or_null`].
    pub fn open(tap: Option<&TapSpec>, buffer: Duration) -> Result<Self> {
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

        // **The device's own buffer, whatever it is.** This used to ask for a
        // bigger one; the ALSA host honoured that and the PipeWire host - the
        // default on every current desktop - does not, and cannot. The audio
        // that covers a stall is held in this crate's own ring instead, so
        // whatever the device asks for per callback is now nobody's problem.
        // See `output::render`.

        let mixer = Arc::new(Mutex::new(Mixer::new(sample_rate)));
        let health = Arc::new(Health::default());
        let ring_capacity = render::ring_capacity(sample_rate, buffer.max(MIN_BUFFER));
        if let Some(spec) = tap {
            log::info!(
                "audio: recording {:.0} s of output to {}",
                spec.seconds,
                spec.path.display()
            );
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a sample count from a caller-supplied duration"
        )]
        let recording = tap.map(|spec| {
            (
                Arc::new(Tap::new(
                    (sample_rate as f32 * spec.seconds.clamp(1.0, 600.0)) as usize * CHANNELS,
                )),
                spec.path.clone(),
            )
        });

        // **The device's own sample format, not `f32`.** This built an `f32`
        // stream unconditionally until finding U1 of the 2026-08-18 review, and
        // a device whose default format is anything else - bare ALSA `hw:` is
        // routinely `i16` - failed at `build_output_stream` and, through
        // `open_or_null`, left the whole session silent on a working card. The
        // mixer still renders `f32` and the conversion happens on the way out;
        // `spread` is generic over the destination for that reason.
        let (stream, producer) = build_stream(
            &device,
            config,
            supported.sample_format(),
            ring_capacity,
            &health,
            recording.as_ref().map(|(t, _)| t),
        )
        .with_context(|| format!("building an output stream on {name}"))?;

        let spectrum = Arc::new(Spectrum::new());
        // After the stream, so a device that refuses to open does not leave a
        // thread rendering into a ring nothing will ever read.
        let ahead = render::Ahead::spawn(
            Arc::clone(&mixer),
            producer,
            render::target_samples(sample_rate, buffer.max(MIN_BUFFER)),
            sample_rate,
            Arc::clone(&spectrum),
        );

        Ok(Self {
            mixer,
            stream: Some(stream),
            ahead: Some(ahead),
            sample_rate,
            device: Some(name),
            health,
            tap: recording,
            spectrum,
        })
    }

    /// Opens the default device, falling back to silence with a note on stdout.
    ///
    /// The same degradation the video path already takes when its decoder is
    /// missing: say what is absent, by name, and carry on.
    #[must_use]
    pub fn open_or_null(tap: Option<&TapSpec>, buffer: Duration) -> Self {
        match Self::open(tap, buffer) {
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

    /// Writes the recording once it is full, and says where it went.
    ///
    /// Called once a frame beside [`Output::report_health`]. Writing from here
    /// rather than at exit is deliberate: a run killed at the terminal still
    /// leaves the evidence behind.
    pub fn flush_tap(&self) {
        let Some((tap, path)) = &self.tap else {
            return;
        };
        if !tap.is_full() {
            return;
        }
        let Some(samples) = tap.take() else {
            return;
        };
        #[expect(clippy::cast_precision_loss, reason = "a duration for a message")]
        let seconds = samples.len() as f32 / (self.sample_rate as f32 * CHANNELS as f32);
        let wav = crate::wav::from_samples(&samples, self.sample_rate);
        match std::fs::write(path, wav) {
            Ok(()) => warn!(
                "audio: wrote {} - {seconds:.1} s of exactly what the device was handed",
                path.display()
            ),
            Err(e) => error!("audio: could not write {}: {e}", path.display()),
        }
    }

    /// What the output callback has seen, for a test or an overlay to read.
    #[must_use]
    pub fn health(&self) -> &Health {
        &self.health
    }

    /// A live spectrum of what this output is actually playing, for the Zone
    /// visualiser.
    ///
    /// Silence on [`Output::null`] and until the render-ahead thread's first
    /// chunk - see [`Spectrum`]'s own docs for why that is the honest default
    /// rather than a gap to fill in.
    #[must_use]
    pub fn spectrum(&self) -> &Spectrum {
        &self.spectrum
    }

    /// Logs what the callback saw since the last report, if anything changed.
    ///
    /// **Called from the frame loop, never from the callback.** Formatting a
    /// message allocates, and the audio thread is the one place in this crate
    /// where an allocation is a dropout of its own; the callback therefore only
    /// ever bumps an atomic and this reads them back from a thread that can
    /// afford the sentence.
    ///
    /// The mixer's own two counters are folded in here rather than reported
    /// separately, because the useful reading is which of the five moved
    /// together - see [`Health`].
    pub fn report_health(&self) {
        if !self.health.due() {
            return;
        }
        let (starved, clipped) = self.with_mixer(|mixer| (mixer.starved(), mixer.clipped()));
        self.health.report(starved, clipped);
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

/// The formats to try, the device's own default first, then `i16` and `f32`.
fn formats_to_try(default: cpal::SampleFormat) -> Vec<cpal::SampleFormat> {
    let mut formats = vec![default];
    for fallback in [cpal::SampleFormat::I16, cpal::SampleFormat::F32] {
        if !formats.contains(&fallback) {
            formats.push(fallback);
        }
    }
    formats
}

/// Builds the stream in the device's own default sample format and, when that
/// is refused, in the other formats a mixer can convert to.
///
/// **Some drivers advertise a default they then refuse.** Android's AAudio on a
/// Galaxy S24 reports `f32` and fails `build_output_stream` with
/// `IllegalArgument`, where the same device takes `i16`. The ring's consumer is
/// moved into the callback and lost when a build fails, so each attempt gets
/// a ring of its own and the producer of the one that worked is returned.
fn build_stream(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    default: cpal::SampleFormat,
    ring_capacity: usize,
    health: &Arc<Health>,
    tap: Option<&Arc<Tap>>,
) -> Result<(cpal::Stream, rtrb::Producer<f32>)> {
    let mut first_error = None;
    for format in formats_to_try(default) {
        let (producer, consumer) = rtrb::RingBuffer::new(ring_capacity);
        let built = match format {
            cpal::SampleFormat::F32 => build::<f32>(device, config, consumer, health, tap),
            cpal::SampleFormat::F64 => build::<f64>(device, config, consumer, health, tap),
            cpal::SampleFormat::I8 => build::<i8>(device, config, consumer, health, tap),
            cpal::SampleFormat::I16 => build::<i16>(device, config, consumer, health, tap),
            cpal::SampleFormat::I32 => build::<i32>(device, config, consumer, health, tap),
            cpal::SampleFormat::I64 => build::<i64>(device, config, consumer, health, tap),
            cpal::SampleFormat::U8 => build::<u8>(device, config, consumer, health, tap),
            cpal::SampleFormat::U16 => build::<u16>(device, config, consumer, health, tap),
            cpal::SampleFormat::U32 => build::<u32>(device, config, consumer, health, tap),
            cpal::SampleFormat::U64 => build::<u64>(device, config, consumer, health, tap),
            // `SampleFormat` is `#[non_exhaustive]`, and the packed 24-bit and
            // DSD formats have no `FromSample<f32>` to convert through. Named
            // rather than silently silent.
            other => Err(anyhow::anyhow!("unsupported sample format {other}")),
        };
        match built {
            Ok(stream) => {
                if first_error.is_some() {
                    warn!("audio: the default {default} stream was refused; using {format}");
                }
                return Ok((stream, producer));
            }
            Err(error) => {
                let error = error.context(format!("building a {format} output stream"));
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    Err(first_error.unwrap_or_else(|| anyhow::anyhow!("no sample format to try")))
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut ring: rtrb::Consumer<f32>,
    health: &Arc<Health>,
    tap: Option<&Arc<Tap>>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32> + Send + 'static,
{
    let device_channels = usize::from(config.channels).max(1);
    let callback_health = Arc::clone(health);
    let callback_tap = tap.cloned();
    // Seconds per frame, so lateness can be judged against the size of the
    // buffer actually handed over rather than a constant: a PipeWire quantum of
    // 256 frames is 5.3 ms and a 2,048-frame ALSA period is 46 ms.
    let seconds_per_frame = 1.0 / f64::from(config.sample_rate.max(1)) as f32;
    // The stereo the ring hands over, before it is spread across however many
    // channels the device actually has. Allocated once here, never in the
    // callback.
    let mut scratch: Vec<f32> = Vec::new();
    let mut declick = Declick::default();
    // When the previous callback returned, so the next one can say whether more
    // wall-clock time passed than the buffer between them was worth. This is
    // the measurement that separates "something in this process was late" from
    // "the samples this process produced have a step in them", and the two
    // sound identical from a chair.
    let mut previous: Option<Instant> = None;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let frames = data.len() / device_channels;

            let now = Instant::now();
            if let Some(previous) = previous {
                // Against the *previous* buffer's playing time, which is this
                // one's in every configuration cpal offers. Half a period of
                // slack, not a whole one: this fired only past `owed * 2` until
                // 2026-09-15, which let a callback a full period late - one
                // PipeWire xrun, exactly the fault that sounds like a skip -
                // count as nothing, so a run reading `0 late callback(s)` had
                // not actually ruled the device path out. Half a period is
                // still clear of ordinary jitter: at this bound a 60 s HD race
                // on an idle PipeWire graph (1,024-frame quantum) counted one
                // late callback, the race's own load stall (`frame: 138.5 ms`
                // beside it), with `pw-top` reporting zero xruns throughout.
                let owed = Duration::from_secs_f32(frames as f32 * seconds_per_frame);
                let elapsed = now.duration_since(previous);
                if elapsed > owed * 3 / 2 {
                    callback_health.late((elapsed - owed).as_micros() as u64);
                }
            }
            previous = Some(now);

            let tail = declick.tail;
            let recovering = declick.recovering;
            callback_health.buffer(frames);

            // **The whole of the audio thread's work: copy.** No lock, no
            // mixing, no allocation - the render thread did all of it ahead of
            // time. See `output::render`.
            scratch.clear();
            scratch.resize(frames * CHANNELS, 0.0);
            let short = {
                let (_, unfilled) = ring.pop_partial_slice(&mut scratch);
                unfilled.len()
            };

            if short == 0 {
                if recovering {
                    ramp_up(&mut scratch);
                    declick.recovering = false;
                }
                declick.tail = last_frame(&scratch);
            } else {
                // The ring ran dry: the render thread was starved for longer
                // than the queue was deep. Ramp what did arrive down rather
                // than stopping on it, and leave the rest silent.
                callback_health.dropped();
                let filled = scratch.len() - short;
                scratch[filled..].fill(0.0);
                if filled >= CHANNELS {
                    fade_out(&mut scratch[..filled]);
                } else if !recovering {
                    // Nothing arrived at all, so there is no signal to ramp -
                    // decay from where the previous buffer ended instead, which
                    // is the step that would otherwise be uncovered.
                    ramp_from(&mut scratch, tail);
                }
                declick.tail = [0.0; CHANNELS];
                declick.recovering = true;
            }

            if !recovering || short == 0 {
                callback_health.scan(&scratch, tail);
            }

            if let Some(tap) = &callback_tap {
                tap.push(&scratch);
            }

            spread(&scratch, data, device_channels);
        },
        move |err| error!("audio: output stream error: {err}"),
        None,
    )?;
    stream.play()?;
    Ok(stream)
}

/// What the callback remembers from one buffer to the next, so that a starved
/// one is a gap rather than a click. See [`DECLICK_FRAMES`].
#[derive(Debug, Default, Clone, Copy)]
struct Declick {
    /// The last stereo frame actually emitted, which a gap ramps down from.
    tail: [f32; CHANNELS],
    /// Whether the previous buffer ran short, so this one ramps back in.
    recovering: bool,
}

/// The last stereo frame in a buffer, or silence if there is none.
fn last_frame(stereo: &[f32]) -> [f32; CHANNELS] {
    let frames = stereo.len() / CHANNELS;
    if frames == 0 {
        return [0.0; CHANNELS];
    }
    let at = (frames - 1) * CHANNELS;
    [stereo[at], stereo[at + 1]]
}

/// Ramps the end of a short buffer down to zero, so the gap after it does not
/// start with a step.
fn fade_out(stereo: &mut [f32]) {
    let frames = stereo.len() / CHANNELS;
    if frames == 0 {
        return;
    }
    let ramp = frames.min(DECLICK_FRAMES);
    for step in 0..ramp {
        let gain = 1.0 - (step + 1) as f32 / ramp as f32;
        let at = (frames - ramp + step) * CHANNELS;
        stereo[at] *= gain;
        stereo[at + 1] *= gain;
    }
}

/// Decays a silent buffer from `tail`, for the gap that begins with a buffer
/// the ring could not fill at all.
fn ramp_from(stereo: &mut [f32], tail: [f32; CHANNELS]) {
    let frames = stereo.len() / CHANNELS;
    let ramp = frames.min(DECLICK_FRAMES);
    for frame in 0..ramp {
        let gain = 1.0 - (frame + 1) as f32 / ramp as f32;
        let at = frame * CHANNELS;
        stereo[at] = tail[0] * gain;
        stereo[at + 1] = tail[1] * gain;
    }
}

/// Ramps a buffer up from silence, the other edge of the same gap.
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
