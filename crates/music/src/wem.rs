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
//! The crate is young and single-author, so it was measured before it was
//! taken (`docs/formats/wwise.md`): against FFmpeg's independent decoder on a
//! stereo voice clip it agrees to within one least-significant bit on 99.75%
//! of samples and **is the negation of it** - a polarity difference between two
//! implementations, inaudible on its own and not corrected here, since which
//! polarity Sony's decoder has is not known. `crates/game/tests/
//! omega_wem_ground_truth.rs` decodes every `.wem` on the archives.
//!
//! The crate's README rates multi-channel decoding "partial" and skips the
//! bandwidth-extension payload; the stereo files are the ones a cue plays.
//!
//! [ADR-0019]: ../../../docs/architecture/adr/0019-atrac3plus-out-of-process.md
//! [ADR-0024]: ../../../docs/architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md

use anyhow::{Context, Result, anyhow, ensure};
use atrac9dec::Atrac9Decoder;
use oag_formats::wwise::wem::Wem;

use crate::at3::Pcm;

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
/// When [`describe`] would refuse it, when the configuration word does not
/// agree with `fmt ` (channels, rate, one frame per block), or when a frame
/// will not decode.
pub fn decode(blob: &[u8]) -> Result<Pcm> {
    let wem = Wem::parse(blob).map_err(|e| anyhow!("{e}"))?;
    let format = wem.format();
    let atrac9 = format
        .atrac9()
        .context("not ATRAC9 with the 18-byte extra data Wwise writes")?;
    let mut decoder =
        Atrac9Decoder::new(&atrac9.config).map_err(|e| anyhow!("the ATRAC9 config word: {e}"))?;
    let info = decoder.codec_info();
    ensure!(
        info.channels == usize::from(format.channels)
            && info.sampling_rate == format.sample_rate
            && info.superframe_size == usize::from(format.block_align),
        "the ATRAC9 config word ({} channel(s), {} Hz, {}-byte superframe) disagrees with fmt \
         ({} channel(s), {} Hz, block align {})",
        info.channels,
        info.sampling_rate,
        info.superframe_size,
        format.channels,
        format.sample_rate,
        format.block_align
    );
    let per_superframe = info.channels * info.frame_samples * info.frames_in_superframe;
    let mut samples = Vec::with_capacity(wem.frames() * per_superframe);
    let mut block = vec![0i16; per_superframe];
    for (index, frame) in wem
        .payload()
        .chunks_exact(usize::from(format.block_align))
        .enumerate()
    {
        decoder
            .decode(frame, &mut block)
            .map_err(|e| anyhow!("frame {index}: {e}"))?;
        samples.extend_from_slice(&block);
    }
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
