//! A coarse, render-facing frequency spectrum of what is actually playing.
//!
//! # Why this exists
//!
//! Wipeout HD/Fury's Zone mode draws an audio-spectrum visualiser on the
//! track and on billboards - observed directly by the maintainer playing the
//! original, and corroborated from the disc's own microcode: `zoneTexVis` is
//! a 256-entry lookup keyed on a texel's alpha, sampled point-filtered, and
//! gated to up-facing surfaces exactly where a floor display would be. See
//! `docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md` and
//! `docs/formats/effectsettings.md#the-eq-keys-are-an-audio-spectrum-observed-in-play`.
//!
//! **The original's own feed is recovered, and it is audio.** This module
//! used to carry a paragraph saying the opposite - that whether HD's
//! per-frame `zoneTexVis` write was audio-reactive was open, and that this
//! was therefore not a port of the original's mechanism. It is now read:
//! `Environment_UpdateStageBlend` calls a sound-system getter once per band
//! per frame and lays the result out as sixteen ten-segment bar meters, and
//! this crate supplies exactly what that getter supplies -
//! [`BANDS`] levels in `0.0..=1.0`. See
//! `docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`.
//!
//! **The measurement is recovered too**, from the live band records:
//! a 1024-point spectrum of the music, summed over sixteen authored bin ranges
//! ([`BINS`]) and read before the music slider. What stays this project's own
//! is the window, which the capture could not separate from a sine window. See
//! `docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`, "What a band
//! measures". See
//! [ADR-0018](../../../docs/architecture/adr/0018-audio-mixer-architecture.md)
//! for why this lives here rather than in the mixer's own per-tick output:
//! a visualiser is analysis *of* the signal, not a cue the simulation
//! produces, and it must never reach anything `oag-gameplay` can see.
//!
//! # Where it runs
//!
//! [`Analyzer::process`] is called from [`crate::output::render::Ahead`]'s
//! render-ahead thread, once per rendered chunk, over the mixer's music tap
//! ([`crate::Mixer::music_tap`]). It is deliberately not called from the real-time device callback, which
//! allocates nothing and cannot afford a division-heavy loop. The result is
//! published into a [`Spectrum`], a small fixed-size snapshot behind a
//! `Mutex` on the same shape [`crate::output::Tap`] and
//! [`crate::output::Health`] already use to cross a thread boundary: cheap to
//! write every chunk, cheap to read once a frame, and never blocking the
//! audio path on the reader.

use std::sync::Mutex;

/// How many frequency bands the visualiser reads.
///
/// **Sixteen is recovered from the disc, not chosen.** HD/Fury's own
/// per-frame `zoneTexVis` writer loops a band index `0..=15` and reads each
/// band through a sound-system getter that clamps the index to `15` and
/// returns a default past it - `SoundSystem_GetBandLevel` (`0x00304530`),
/// `*(float *)(g_sound_system + 0x24 + band * 4)`. Corroborated from the
/// shipped art: every `zoneModeTrack*.gtf`'s alpha plateaus land on the
/// ten-texel band stride that writer lays down. See
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`.
///
/// The sixteen are frequency bands: see [`BINS`].
pub const BANDS: usize = 16;

/// The transform's length, and the sample rate its bins are authored at.
const FFT_SIZE: usize = 1024;
const AUTHORED_RATE: f32 = 48_000.0;

/// How many samples one spectrum moves on by: half the transform, about 94
/// spectra a second, what the band records' update cadence reads live.
const HOP: usize = FFT_SIZE / 2;

/// Each band's first and last bin of the 512 a 1024-point spectrum has, read
/// out of the live band records (`SoundSystem+0x614`). `46.875` Hz a bin, so
/// bands `10..=12`, the Fury backdrop's pulse, are `4.5` to `10` kHz. A band
/// is the sum of its bins' magnitudes.
const BINS: [(u16, u16); BANDS] = [
    (0, 0),
    (1, 1),
    (2, 2),
    (3, 5),
    (6, 11),
    (12, 20),
    (21, 32),
    (33, 49),
    (50, 70),
    (71, 96),
    (97, 128),
    (129, 167),
    (168, 213),
    (214, 266),
    (267, 328),
    (329, 511),
];

/// [`BINS`] at `sample_rate`, the same frequencies when the device is not
/// running at the rate they were authored for.
fn bins_at(sample_rate: f32) -> [(usize, usize); BANDS] {
    let scale = if sample_rate > 0.0 {
        AUTHORED_RATE / sample_rate
    } else {
        1.0
    };
    let bin = |at: u16| ((f32::from(at) * scale).round() as usize).min(FFT_SIZE / 2 - 1);
    BINS.map(|(first, last)| (bin(first), bin(last)))
}

/// An in-place radix-2 transform of [`FFT_SIZE`] complex points, tables built
/// once, nothing allocated per spectrum.
#[derive(Debug)]
struct Fft {
    /// The window over the block. **Hann is chosen, not measured**: it and a
    /// sine window correlate with the live band records equally well.
    window: Vec<f32>,
    /// `e^(-2 pi i k / FFT_SIZE)` for `k` under half the size.
    twiddles: Vec<(f32, f32)>,
    /// Each index with its bits reversed.
    reversed: Vec<u16>,
}

impl Fft {
    fn new() -> Self {
        let turn = 2.0 * std::f32::consts::PI / FFT_SIZE as f32;
        let bits = FFT_SIZE.trailing_zeros();
        Self {
            window: (0..FFT_SIZE)
                .map(|i| 0.5 - 0.5 * (turn * i as f32).cos())
                .collect(),
            twiddles: (0..FFT_SIZE / 2)
                .map(|k| {
                    let angle = -turn * k as f32;
                    (angle.cos(), angle.sin())
                })
                .collect(),
            reversed: (0..FFT_SIZE)
                .map(|i| (i.reverse_bits() >> (usize::BITS - bits)) as u16)
                .collect(),
        }
    }

    /// The magnitudes of `samples`' first [`HALF`] bins, into `magnitudes`.
    fn magnitudes(
        &self,
        samples: &[f32; FFT_SIZE],
        real: &mut [f32; FFT_SIZE],
        imaginary: &mut [f32; FFT_SIZE],
        magnitudes: &mut [f32; HALF],
    ) {
        for (i, &from) in self.reversed.iter().enumerate() {
            real[i] = samples[usize::from(from)] * self.window[usize::from(from)];
            imaginary[i] = 0.0;
        }
        let mut half = 1;
        while half < FFT_SIZE {
            let stride = FFT_SIZE / (half * 2);
            for start in (0..FFT_SIZE).step_by(half * 2) {
                for k in 0..half {
                    let (wr, wi) = self.twiddles[k * stride];
                    let (a, b) = (start + k, start + k + half);
                    let tr = real[b] * wr - imaginary[b] * wi;
                    let ti = real[b] * wi + imaginary[b] * wr;
                    real[b] = real[a] - tr;
                    imaginary[b] = imaginary[a] - ti;
                    real[a] += tr;
                    imaginary[a] += ti;
                }
            }
            half *= 2;
        }
        for (bin, magnitude) in magnitudes.iter_mut().enumerate() {
            *magnitude = (real[bin] * real[bin] + imaginary[bin] * imaginary[bin]).sqrt();
        }
    }
}

/// The bins a spectrum keeps.
const HALF: usize = FFT_SIZE / 2;

/// One band's auto-ranging normaliser: where this band's amplitude sits
/// between its own recent floor and its own recent peak, `0.0..=1.0`.
///
/// **Recovered**, from HD/Fury's own sixteen-band analysis loop at
/// `0x00307e78` - the function that fills the array
/// `SoundSystem_GetBandLevel` reads. Every constant below is a float that
/// loop loads from its own TOC; see
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`.
///
/// **Why an original would do this rather than compress logarithmically.** A
/// fixed floor - the `-40 dB` this replaced - leaves a quiet band pinned at
/// the bottom of its bar all race, because it measures the band against
/// *full scale*. This measures each band against **its own** recent range, so
/// a bassline and a hi-hat both fill their meters, and a band with nothing in
/// it collapses to a span of nothing and reads zero rather than reading
/// noise. That is what makes sixteen bars all move.
#[derive(Debug, Default, Clone, Copy)]
struct Range {
    peak: f32,
    floor: f32,
}

impl Range {
    /// How fast the peak falls back towards the floor, per second.
    /// `0xbe4ccccd`; the original stores it negative and adds.
    const PEAK_FALL: f32 = -0.2;
    /// How fast the floor rises towards the peak, per second. `0x3e4ccccd`.
    const FLOOR_RISE: f32 = 0.2;
    /// The floor may never exceed this fraction of the peak. `0x3f666666`.
    const FLOOR_CEILING: f32 = 0.9;
    /// A span at or under this reads as silence rather than as a level.
    /// `0x322bcc77` - small enough that it only catches a genuinely empty
    /// band, not a quiet one.
    const MIN_SPAN: f32 = 1.0e-8;

    /// Folds one chunk's `amplitude` in over `dt` seconds and returns the
    /// level, in the original's own order of operations: decay, then track,
    /// then let the current sample push either end outwards.
    fn level(&mut self, amplitude: f32, dt: f32) -> f32 {
        if self.peak > 0.0 {
            self.peak = (self.peak + (self.peak - self.floor) * dt * Self::PEAK_FALL).max(0.0);
        }
        let ceiling = self.peak * Self::FLOOR_CEILING;
        self.floor = if ceiling < self.floor {
            ceiling
        } else {
            (self.floor + (self.peak - self.floor) * dt * Self::FLOOR_RISE).min(ceiling)
        };
        // The current sample owns both ends: it raises the peak at once and
        // drops the floor at once, so an attack is never smeared and a gap
        // never reads as a level.
        self.peak = self.peak.max(amplitude);
        self.floor = self.floor.min(amplitude);
        let span = self.peak - self.floor;
        if span <= Self::MIN_SPAN {
            0.0
        } else {
            ((amplitude - self.floor) / span).clamp(0.0, 1.0)
        }
    }
}

/// Runs the transform over the music as it arrives and holds each band's
/// range and latest level, ready to publish into a [`Spectrum`].
///
/// Owns its own state, so it is a small struct on the render-ahead thread
/// rather than something [`Spectrum`] itself would need a lock to update
/// incrementally.
#[derive(Debug)]
pub(crate) struct Analyzer {
    fft: Fft,
    ranges: [Range; BANDS],
    levels: [f32; BANDS],
    /// The last [`FFT_SIZE`] samples, oldest first.
    block: Box<[f32; FFT_SIZE]>,
    /// Samples taken since the last spectrum.
    since: usize,
    real: Box<[f32; FFT_SIZE]>,
    imaginary: Box<[f32; FFT_SIZE]>,
    magnitudes: Box<[f32; HALF]>,
}

impl Analyzer {
    pub(crate) fn new() -> Self {
        Self {
            fft: Fft::new(),
            ranges: [Range::default(); BANDS],
            levels: [0.0; BANDS],
            block: Box::new([0.0; FFT_SIZE]),
            since: 0,
            real: Box::new([0.0; FFT_SIZE]),
            imaginary: Box::new([0.0; FFT_SIZE]),
            magnitudes: Box::new([0.0; HALF]),
        }
    }

    /// Folds `mono` in and returns the latest levels: one spectrum every
    /// [`HOP`] samples, however the chunks fall.
    ///
    /// **No decay here.** The original's sound system publishes an
    /// *instantaneous* normalised level (see [`Range`]) and the ballistics
    /// live one layer out, in each reader's own per-frame hold:
    /// `oag_mesh::mesh_render::zone::Hold` and `oag_ui::backdrop::Fury`.
    pub(crate) fn process(&mut self, mono: &[f32], sample_rate: u32) -> [f32; BANDS] {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an audio sample rate is well under f32's exact-integer range"
        )]
        let rate = sample_rate as f32;
        let dt = if rate > 0.0 { HOP as f32 / rate } else { 0.0 };
        let bins = bins_at(rate);
        for chunk in mono.chunks(HOP) {
            let take = chunk.len().min(HOP - self.since);
            let (now, rest) = chunk.split_at(take);
            self.push(now);
            if self.since == HOP {
                self.spectrum(&bins, dt);
            }
            self.push(rest);
        }
        self.levels
    }

    /// Moves `samples` into the block, no more than a hop's worth.
    fn push(&mut self, samples: &[f32]) {
        self.block.copy_within(samples.len().., 0);
        self.block[FFT_SIZE - samples.len()..].copy_from_slice(samples);
        self.since += samples.len();
    }

    /// One spectrum of the block, every band's level from it.
    fn spectrum(&mut self, bins: &[(usize, usize); BANDS], dt: f32) {
        self.since = 0;
        self.fft.magnitudes(
            &self.block,
            &mut self.real,
            &mut self.imaginary,
            &mut self.magnitudes,
        );
        for (band, &(first, last)) in bins.iter().enumerate() {
            let sum: f32 = self.magnitudes[first..=last].iter().sum();
            self.levels[band] = self.ranges[band].level(sum / FFT_SIZE as f32, dt);
        }
    }
}

/// The most recent spectrum snapshot, shared between the render-ahead thread
/// that computes it and whoever reads it once a frame.
///
/// Silence (`[0.0; BANDS]`) until the first chunk renders, and forever on
/// [`crate::Output::null`]. That is the original's own silent value: its
/// bands read `0` before the menu's music starts and across the loop's seam.
#[derive(Debug)]
pub struct Spectrum {
    levels: Mutex<[f32; BANDS]>,
}

impl Spectrum {
    pub(crate) fn new() -> Self {
        Self {
            levels: Mutex::new([0.0; BANDS]),
        }
    }

    /// Publishes one chunk's levels, from the render-ahead thread.
    ///
    /// `try_lock`, the same reasoning [`crate::output::Tap::push`] uses: the
    /// only other holder is a reader taking a snapshot, and a stale frame of
    /// levels is better than this thread blocking on a reader.
    pub(crate) fn publish(&self, levels: [f32; BANDS]) {
        if let Ok(mut guard) = self.levels.try_lock() {
            *guard = levels;
        }
    }

    /// The most recent published levels, each `0.0..=1.0`.
    #[must_use]
    ///
    /// Read on the frame loop's thread, which in the browser is the page's and
    /// may not wait on a lock: [`oag_thread::lock`] spins there instead.
    pub fn levels(&self) -> [f32; BANDS] {
        oag_thread::lock(&self.levels)
            .map(|guard| *guard)
            .unwrap_or([0.0; BANDS])
    }
}

#[cfg(test)]
mod tests;
