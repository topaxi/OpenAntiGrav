//! The Omega Collection's Wwise media (`.wem`), decoded in process.
//!
//! Every `.wem` on that title is ATRAC9 - `oag_formats::wwise::wem` has the
//! identification and its evidence - and this turns one into the [`Pcm`] every
//! other codec here produces, shaped for [`oag_audio::Sound::new`].
//!
//! # Why in process, and why not through `ffmpeg`
//!
//! [ADR-0024]'s order: a library first, our own decoder only for a format that
//! is *Wipeout's*, `ffmpeg` last. ATRAC9 is Sony's, so it is not ours to write,
//! and `atrac9dec` (pure Rust, MIT, a port of LibAtrac9) is a library, so
//! `ffmpeg` is not needed and there is **no cache**: a decode is faster than
//! real time. That is the opposite of ATRAC3+ ([`crate::at3`], [ADR-0019]),
//! for which no Rust decoder exists.
//!
//! # What was checked, and what was not
//!
//! The published crate refused every multi-channel stream and departed from
//! LibAtrac9 on stereo too, so it is vendored with fixes (`vendor/atrac9dec`).
//! Every ATRAC9 stream Omega and 2048 ship decodes sample-identical to
//! LibAtrac9 and within one least-significant bit of FFmpeg's independent
//! decoder, negated: a polarity difference between the two implementations,
//! inaudible on its own and not corrected here, since which polarity Sony's
//! decoder has is not known. `docs/formats/atrac9.md` has the numbers;
//! `crates/music/tests/omega_wem_ground_truth.rs` decodes every `.wem` on the
//! archives.
//!
//! [ADR-0019]: ../../../docs/architecture/adr/0019-atrac3plus-out-of-process.md
//! [ADR-0024]: ../../../docs/architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md

use anyhow::{Context, Result, anyhow};
use oag_formats::wwise::wem::Wem;

use crate::at3::{Format, Pcm};

/// What a `.wem` declares about itself, without decoding it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stream {
    pub channels: u16,
    pub sample_rate: u32,
    /// Frames after the encoder delay, before the padding - the length a
    /// player hears.
    pub frames: u32,
}

/// What `blob` declares, or an error when it is not an ATRAC9 `.wem`.
///
/// # Errors
///
/// When it is not RIFF/WAVE, is not ATRAC9, or carries extra data other than
/// the 18 bytes every shipped file does.
pub fn describe(blob: &[u8]) -> Result<Stream> {
    let wem = Wem::parse(blob).map_err(|e| anyhow!("{e}"))?;
    let format = wem.format();
    let atrac9 = format
        .atrac9()
        .context("not ATRAC9 with the 18-byte extra data Wwise writes")?;
    Ok(Stream {
        channels: format.channels,
        sample_rate: format.sample_rate,
        frames: atrac9.samples,
    })
}

/// Decodes an ATRAC9 `.wem` to interleaved 16-bit PCM.
///
/// The encoder delay is skipped and the padding trimmed, so the result is
/// exactly [`Stream::frames`] long.
///
/// # Errors
///
/// When [`describe`] would refuse it, or as [`crate::at9::decode_superframes`].
pub fn decode(blob: &[u8]) -> Result<Pcm> {
    let wem = Wem::parse(blob).map_err(|e| anyhow!("{e}"))?;
    let format = wem.format();
    let atrac9 = format
        .atrac9()
        .context("not ATRAC9 with the 18-byte extra data Wwise writes")?;
    let mut samples = crate::at9::decode_superframes(
        &atrac9.config,
        Format {
            channels: format.channels,
            sample_rate: format.sample_rate,
            block_align: format.block_align,
        },
        wem.payload(),
    )?;
    let channels = usize::from(format.channels);
    let skip = (atrac9.delay as usize * channels).min(samples.len());
    samples.drain(..skip);
    samples.truncate(atrac9.samples as usize * channels);
    Ok(Pcm {
        samples,
        channels: format.channels,
        sample_rate: format.sample_rate,
    })
}
