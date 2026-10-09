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
/// **That the sixteen are *frequency* bands is confidence 75, not 86.** The
/// count and the layout are solid; what the original measures per entry is
/// read from two consumers drawing them as an equaliser, a 32-entry sibling
/// array, and the shipped art's sixteen tagged regions - not from the
/// producer, which is unfound. The source indexes two blocks of eight, and
/// eight is also this title's speaker count, so a per-channel reading is not
/// excluded. Same page.
///
/// **The band *frequencies* are still this project's own choice** - see
/// [`band_frequencies`], and permanently rather than pending a read: the
/// original's per-band magnitudes come from filter records no PPU code in
/// that executable writes, on an engine (Sony's SCREAM, on MultiStream) whose
/// own analysis is not in the image. Four sweeps establishing that are in the
/// same page.
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

/// Converts a raw Goertzel magnitude into this band's own amplitude.
///
/// **Normalised by chunk size.** A Goertzel term's magnitude at resonance
/// scales with `samples.len() / 2` for a sinusoid of a given amplitude, so
/// dividing that out turns "the raw DFT term" into "roughly this band's own
/// amplitude, 0 to 1 for full scale" - independent of how many samples one
/// chunk holds, and the step that matters for *contrast* between bands:
/// skipping it leaves every band's own leakage floor (a rectangular window's
/// sidelobes decay slowly) sitting within a few dB of a real tone's peak,
/// which is exactly the failure this file's own tests caught - every band
/// from a single test tone clipped to `1.0`, tied, and the tie-break picked
/// the wrong one.
///
/// **This used to log-compress against a chosen `-40 dB` floor**, which was
/// an invention. [`Range`] replaces it with the original's own curve, which
/// needs an amplitude rather than a decibel: see that type.
fn band_amplitude(magnitude: f32, samples: usize) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a chunk is a few hundred to a few thousand samples"
    )]
    let scale = (samples as f32 / 2.0).max(1.0);
    magnitude / scale
}

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

/// Runs the per-band Goertzel transform over one rendered chunk and holds a
/// smoothed level per band, ready to publish into a [`Spectrum`].
///
/// Owns its own smoothing state, so it is a small struct on the render-ahead
/// thread rather than something [`Spectrum`] itself would need a lock to
/// update incrementally.
#[derive(Debug)]
pub(crate) struct Analyzer {
    frequencies: [f32; BANDS],
    ranges: [Range; BANDS],
}

impl Analyzer {
    pub(crate) fn new() -> Self {
        Self {
            frequencies: band_frequencies(),
            ranges: [Range::default(); BANDS],
        }
    }

    /// Folds one chunk of interleaved stereo samples in, and returns this
    /// chunk's levels.
    ///
    /// **No decay here.** An earlier version held each band against a `0.75`
    /// per-chunk fade, on the reasoning that a 512-frame update period would
    /// otherwise flicker. That was an invention in the wrong place: the
    /// original's sound system publishes an *instantaneous* normalised level
    /// (see [`Range`]) and the ballistics live one layer out, in the
    /// visualiser's own per-frame peak-hold -
    /// `oag_mesh::mesh_render::zone::Hold`, itself recovered. Holding here
    /// too would smear the attack the hold is supposed to catch.
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
        // How long this chunk covers, which is what the recovered peak and
        // floor rates are per: the original takes its own `dt` as an
        // argument. Deriving it here rather than taking it keeps the caller
        // from having to agree with the mixer about chunk sizes.
        #[expect(
            clippy::cast_precision_loss,
            reason = "a chunk is a few hundred to a few thousand frames"
        )]
        let dt = if rate > 0.0 {
            mono.len() as f32 / rate
        } else {
            0.0
        };
        let mut levels = [0.0f32; BANDS];
        for (band, &frequency) in self.frequencies.iter().enumerate() {
            let amplitude = band_amplitude(goertzel_magnitude(&mono, rate, frequency), mono.len());
            levels[band] = self.ranges[band].level(amplitude, dt);
        }
        levels
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
