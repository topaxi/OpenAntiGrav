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

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::mixer::{CHANNELS, Mixer};

/// The rate used when there is no device to ask.
///
/// 44,100 Hz because that is what every ATRAC3+ stream on the disc is; the PS2
/// PCM archives are 48,000 and get resampled, which is the direction that costs
/// nothing.
pub const DEFAULT_SAMPLE_RATE: u32 = 44_100;

/// A mixer, and optionally the device draining it.
pub struct Output {
    mixer: Arc<Mutex<Mixer>>,
    /// Dropping this stops the stream, so it is held even though nothing reads
    /// it. `None` is the null backend.
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    device: Option<String>,
}

// `cpal::Stream` is deliberately not `Debug`, and the workspace warns on a
// missing one, so this is written out rather than derived.
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Output")
            .field("sample_rate", &self.sample_rate)
            .field("device", &self.device)
            .field("streaming", &self.stream.is_some())
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

        // **The device's own sample format, not `f32`.** This built an `f32`
        // stream unconditionally until finding U1 of the 2026-08-18 review, and
        // a device whose default format is anything else - bare ALSA `hw:` is
        // routinely `i16` - failed at `build_output_stream` and, through
        // `open_or_null`, left the whole session silent on a working card. The
        // mixer still renders `f32` and the conversion happens on the way out;
        // `spread` is generic over the destination for that reason.
        let format = supported.sample_format();
        let stream = match format {
            cpal::SampleFormat::F32 => build::<f32>(&device, config, &mixer),
            cpal::SampleFormat::F64 => build::<f64>(&device, config, &mixer),
            cpal::SampleFormat::I8 => build::<i8>(&device, config, &mixer),
            cpal::SampleFormat::I16 => build::<i16>(&device, config, &mixer),
            cpal::SampleFormat::I32 => build::<i32>(&device, config, &mixer),
            cpal::SampleFormat::I64 => build::<i64>(&device, config, &mixer),
            cpal::SampleFormat::U8 => build::<u8>(&device, config, &mixer),
            cpal::SampleFormat::U16 => build::<u16>(&device, config, &mixer),
            cpal::SampleFormat::U32 => build::<u32>(&device, config, &mixer),
            cpal::SampleFormat::U64 => build::<u64>(&device, config, &mixer),
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
                println!("audio: no output device ({error:#}); running silent");
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
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32> + Send + 'static,
{
    let device_channels = usize::from(config.channels).max(1);
    let callback_mixer = Arc::clone(mixer);
    // Rendered stereo, before it is spread over however many channels the
    // device actually has. Allocated once here rather than in the callback.
    let mut scratch: Vec<f32> = Vec::new();

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let frames = data.len() / device_channels;
            scratch.clear();
            scratch.resize(frames * CHANNELS, 0.0);

            // `try_lock`, not `lock`. This runs on the audio thread,
            // where blocking on a tick that is mid-update is a dropout
            // for every voice rather than a late update for one. A
            // missed frame of silence is the cheaper failure.
            match callback_mixer.try_lock() {
                Ok(mut mixer) => mixer.render(&mut scratch),
                Err(_) => scratch.fill(0.0),
            }

            spread(&scratch, data, device_channels);
        },
        move |err| eprintln!("audio: output stream error: {err}"),
        None,
    )?;
    stream.play()?;
    Ok(stream)
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
mod tests {
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
}
