//! A track's precomputed spectrum sidecar: the `.fft` file beside 2048's
//! `*_stereo.at9`.
//!
//! The original never analyses the audio it plays. Each track ships a table of
//! band levels, one row per 30th of a second, and the music player samples it at
//! the stream's position (`MusicAnalysis_LoadFft` `0x8125f1de`,
//! `MusicAnalysis_SampleAt` `0x8125f372`;
//! `docs/ghidra/functions/vita-2048-eu-v104/canvas-music-wave.md`, "The data").
//!
//! Layout, little-endian: `AFEX`, `u32` version (1), `u32` bands (12), `u32`
//! frames, then `f32[frames][bands]`, each 0 to 1. `size = 16 + 4 * bands *
//! frames` holds for all 13 files on the disc.

use anyhow::{Result, bail, ensure};

/// Bands in every shipped file.
pub const BANDS: usize = 12;
/// Rows a second.
pub const ROWS_PER_SECOND: f32 = 30.0;
/// The file magic.
const MAGIC: &[u8; 4] = b"AFEX";
/// Bytes before the first row.
const HEADER: usize = 16;

/// One row of band levels.
pub type Levels = [f32; BANDS];

/// A parsed spectrum sidecar.
#[derive(Debug, Clone, PartialEq)]
pub struct Spectrum {
    rows: Vec<Levels>,
}

impl Spectrum {
    /// Reads a sidecar.
    ///
    /// # Errors
    ///
    /// A wrong magic or version, a band count other than [`BANDS`], or a size
    /// that is not `16 + 4 * bands * frames`.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() >= HEADER,
            "spectrum file shorter than its header"
        );
        if &bytes[..4] != MAGIC {
            bail!("spectrum file does not start with AFEX");
        }
        let word = |at: usize| {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        ensure!(word(4) == 1, "spectrum version {} is not 1", word(4));
        ensure!(
            word(8) as usize == BANDS,
            "spectrum has {} bands, not {BANDS}",
            word(8)
        );
        let frames = word(12) as usize;
        ensure!(
            frames
                .checked_mul(BANDS * 4)
                .and_then(|body| body.checked_add(HEADER))
                == Some(bytes.len()),
            "spectrum size {} does not match {frames} frames of {BANDS} bands",
            bytes.len()
        );
        let (body, _) = bytes[HEADER..].as_chunks::<{ BANDS * 4 }>();
        let rows = body
            .iter()
            .map(|row| {
                let mut levels = [0.0; BANDS];
                let (floats, _) = row.as_chunks::<4>();
                for (level, raw) in levels.iter_mut().zip(floats) {
                    *level = f32::from_le_bytes(*raw);
                }
                levels
            })
            .collect();
        Ok(Self { rows })
    }

    /// Rows in the file.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.rows.len()
    }

    /// The track length the rows cover, in seconds.
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.rows.len() as f32 / ROWS_PER_SECOND
    }

    /// The levels `seconds` into the track: row `uint(t * 30)` blended linearly
    /// with the next by the fraction. Negative time, or time past the last row,
    /// is all zero (`MusicAnalysis_SampleAt`).
    #[must_use]
    pub fn sample_at(&self, seconds: f32) -> Levels {
        if seconds.is_nan() || seconds < 0.0 {
            return [0.0; BANDS];
        }
        let position = seconds * ROWS_PER_SECOND;
        let index = position as usize;
        let Some(low) = self.rows.get(index) else {
            return [0.0; BANDS];
        };
        let Some(high) = self.rows.get(index + 1) else {
            return [0.0; BANDS];
        };
        let fraction = position - index as f32;
        let mut out = [0.0; BANDS];
        for (band, level) in out.iter_mut().enumerate() {
            *level = low[band] + (high[band] - low[band]) * fraction;
        }
        out
    }
}

#[cfg(test)]
mod tests;
