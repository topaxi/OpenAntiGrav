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
//! What stays this project's own: the band centre frequencies, the
//! magnitude-to-level curve, and the analyser's ballistics. The disc says how
//! many bands there are and what is done with them, not how a band is
//! measured. See
//! [ADR-0018](../../../docs/architecture/adr/0018-audio-mixer-architecture.md)
//! for why this lives here rather than in the mixer's own per-tick output:
//! a visualiser is analysis *of* the signal, not a cue the simulation
//! produces, and it must never reach anything `oag-gameplay` can see.
//!
//! # Where it runs
//!
//! [`Analyzer::process`] is called from [`crate::output::render::Ahead`]'s
//! render-ahead thread, once per rendered chunk (about 10 ms). It is
//! deliberately not called from the real-time device callback, which
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
/// **The band *frequencies* are still this project's own choice** - see
/// [`band_frequencies`]. Nothing on the disc says what the original's
/// sixteen bands are centred on.
pub const BANDS: usize = 16;

/// The band centre frequencies, in Hz, geometrically spaced from 80 Hz to
/// 8,000 Hz - the range a small on-screen equaliser reads as "bass to
/// treble" without needing sub-bass or ultrasonic bins nothing on a race
/// soundtrack fills anyway.
fn band_frequencies() -> [f32; BANDS] {
    const LOW: f32 = 80.0;
    const HIGH: f32 = 8_000.0;
    let ratio = (HIGH / LOW).ln();
    std::array::from_fn(|i| {
        #[expect(
            clippy::cast_precision_loss,
            reason = "BANDS is 16; no precision lost casting a band index"
        )]
        let t = i as f32 / (BANDS - 1) as f32;
        LOW * (ratio * t).exp()
    })
}

/// One band's magnitude at a target frequency, by the Goertzel algorithm -
/// a single-frequency DFT term, computed in one pass with no allocation and
/// no external FFT dependency for what is otherwise a 32-bin transform run a
/// hundred times a second.
///
/// `target_hz` need not land on an exact FFT bin: Goertzel is a resonant
/// filter tuned to it either way, which is what lets [`band_frequencies`]
/// space bands geometrically rather than to the block size's own bin grid.
fn goertzel_magnitude(samples: &[f32], sample_rate: f32, target_hz: f32) -> f32 {
    if samples.is_empty() || sample_rate <= 0.0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a chunk is hundreds of samples, far under f32's exact-integer range"
    )]
    let n = samples.len() as f32;
    let k = (0.5 + n * target_hz / sample_rate).floor();
    let omega = 2.0 * std::f32::consts::PI * k / n;
    let coeff = 2.0 * omega.cos();
    let mut q1 = 0.0f32;
    let mut q2 = 0.0f32;
    for &sample in samples {
        let q0 = coeff * q1 - q2 + sample;
        q2 = q1;
        q1 = q0;
    }
    (q1 * q1 + q2 * q2 - q1 * q2 * coeff).max(0.0).sqrt()
}

/// Converts a raw Goertzel magnitude into a `0.0..=1.0` display level.
///
/// **Normalised by chunk size first.** A Goertzel term's magnitude at
/// resonance scales with `samples.len() / 2` for a sinusoid of a given
/// amplitude, so dividing that out turns "the raw DFT term" into "roughly
/// this band's own amplitude, 0 to 1 for full scale" - independent of how
/// many samples one chunk holds, and the step that matters for *contrast*
/// between bands: skipping it leaves every band's own leakage
/// floor (a rectangular window's sidelobes decay slowly) sitting within a
/// few dB of a real tone's peak, which is exactly the failure this file's own
/// tests caught - every band from a single test tone clipped to `1.0`, tied,
/// and the tie-break picked the wrong one.
///
/// Log-compressed after that, because ear and eye alike read loudness
/// logarithmically and a linear amplitude would leave every band but the
/// loudest looking dark. `FLOOR_DB` is a chosen dynamic range for a legible
/// display, **not a measurement of anything on the disc or in this crate's
/// own mixer** - there is no authored calibration to read.
fn display_level(magnitude: f32, samples: usize) -> f32 {
    const FLOOR_DB: f32 = -40.0;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a chunk is a few hundred to a few thousand samples"
    )]
    let scale = (samples as f32 / 2.0).max(1.0);
    let amplitude = (magnitude / scale).max(1e-6);
    let db = 20.0 * amplitude.log10();
    ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0)
}

/// Runs the per-band Goertzel transform over one rendered chunk and holds a
/// smoothed level per band, ready to publish into a [`Spectrum`].
///
/// Owns its own smoothing state, so it is a small struct on the render-ahead
/// thread rather than something [`Spectrum`] itself would need a lock to
/// update incrementally.
#[derive(Debug)]
pub(crate) struct Analyzer {
    frequencies: [f32; BANDS],
    levels: [f32; BANDS],
}

impl Analyzer {
    pub(crate) fn new() -> Self {
        Self {
            frequencies: band_frequencies(),
            levels: [0.0; BANDS],
        }
    }

    /// Folds one chunk of interleaved stereo samples in, and returns the
    /// updated levels.
    ///
    /// **Attack is instant, decay is not** - the same shape a real spectrum
    /// analyser's ballistics take, and without it a 512-frame (about 10 ms)
    /// update period reads as flicker rather than as motion. `0.75` decays a
    /// band to under 5% of a peak in about 200 ms at this chunk rate, fast
    /// enough to track a beat and slow enough not to strobe.
    pub(crate) fn process(&mut self, stereo: &[f32], sample_rate: u32) -> [f32; BANDS] {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an audio sample rate is well under f32's exact-integer range"
        )]
        let rate = sample_rate as f32;
        // Mono sum: the visualiser is one signal, not a stereo pair of them,
        // and summing here is one pass rather than one Goertzel run per
        // channel.
        let mono: Vec<f32> = stereo
            .as_chunks::<2>()
            .0
            .iter()
            .map(|frame| (frame[0] + frame[1]) * 0.5)
            .collect();
        const DECAY: f32 = 0.75;
        for (band, &frequency) in self.frequencies.iter().enumerate() {
            let level = display_level(goertzel_magnitude(&mono, rate, frequency), mono.len());
            self.levels[band] = level.max(self.levels[band] * DECAY);
        }
        self.levels
    }
}

/// The most recent spectrum snapshot, shared between the render-ahead thread
/// that computes it and whoever reads it once a frame.
///
/// Silence (`[0.0; BANDS]`) until the first chunk renders, and forever on
/// [`crate::Output::null`] - the honest state for a machine with nothing
/// playing: an equaliser with no signal shows nothing, not an invented level.
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
    pub fn levels(&self) -> [f32; BANDS] {
        self.levels
            .lock()
            .map(|guard| *guard)
            .unwrap_or([0.0; BANDS])
    }
}

#[cfg(test)]
mod tests;
