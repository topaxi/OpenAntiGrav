//! RIFF-wrapped ATRAC9, the Vita's own codec, decoded in process.
//!
//! [`crate::at3`] is the PSP/PS2's ATRAC3+ in the same wrapper; [`crate::wem`]
//! is the Omega Collection's ATRAC9 in Wwise's. Both ATRAC9 shapes decode
//! through [`decode_superframes`] here, on the vendored `atrac9dec`
//! (`vendor/atrac9dec`, a port of LibAtrac9; `docs/formats/atrac9.md` has the
//! patches, the sources and their licences, and the conformance numbers).
//!
//! # What the wrapper carries
//!
//! A `WAVE_FORMAT_EXTENSIBLE` `fmt ` chunk whose subformat GUID is ATRAC9's,
//! followed by twelve bytes of codec data: a version word (1), the four-byte
//! ATRAC9 configuration word, and a reserved word. A `fact` chunk declares the
//! stream's length and its encoder delay; the `data` chunk is whole
//! superframes.
//!
//! # Where this came from
//!
//! 2048 ships `data/audio/music/FEMusic/frontend_stereo.at9` inside its base
//! `PSP2/data.psarc`, RIFF-wrapped exactly like the PSP titles' soundtrack -
//! 48000 Hz stereo, a 302-second `fact` chunk. See
//! `docs/formats/2048-frontend.md` for how that path was found and why it is
//! named [`oag_title::Music::front_end`] rather than a cue out of
//! `frontend.bnk`.

use anyhow::{Context, Result, anyhow, bail, ensure};
use atrac9dec::Atrac9Decoder;

use crate::at3::{Format, Pcm};

/// The `WAVE_FORMAT_EXTENSIBLE` subformat GUID that means ATRAC9.
///
/// `{47E142D2-36BA-4D8D-88FC-61654F8C836C}` in the mixed-endian order a
/// `WAVEFORMATEXTENSIBLE` stores a GUID in, i.e. these bytes as they sit in
/// the file. Measured off `frontend_stereo.at9`'s own `fmt ` chunk.
const ATRAC9_GUID: [u8; 16] = [
    0xd2, 0x42, 0xe1, 0x47, 0xba, 0x36, 0x8d, 0x4d, 0x88, 0xfc, 0x61, 0x65, 0x4f, 0x8c, 0x83, 0x6c,
];

/// Bytes into the `fmt ` chunk body where the subformat GUID starts: 16 for
/// the `WAVEFORMATEX` core plus 2 (`cbSize`) plus 2 (`wValidBitsPerSample`)
/// plus 4 (`dwChannelMask`).
const SUBFORMAT_GUID_AT: usize = 24;

/// Bytes into the `fmt ` chunk body where the ATRAC9 configuration word sits:
/// after the GUID and a four-byte version word.
const CONFIG_AT: usize = SUBFORMAT_GUID_AT + 16 + 4;

/// What a RIFF ATRAC9 file declares, and where its frames are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header<'a> {
    pub format: Format,
    /// The four-byte ATRAC9 configuration word.
    pub config: [u8; 4],
    /// The `fact` chunk's first word: samples per channel a player hears.
    pub samples: Option<u32>,
    /// The `fact` chunk's third word: samples per channel of encoder delay at
    /// the head of the decode.
    pub delay: u32,
    /// The `data` chunk's body: whole superframes. Short when the caller read
    /// only the head of the file.
    pub frames: &'a [u8],
}

/// Walks a RIFF ATRAC9 file's chunks.
///
/// # Errors
///
/// Not a RIFF/WAVE, no readable `fmt ` chunk, a `fmt ` chunk too short to
/// carry an extensible subformat GUID and its codec data, a format tag that is
/// not `WAVE_FORMAT_EXTENSIBLE`, or a GUID that is not ATRAC9's.
pub fn header(blob: &[u8]) -> Result<Header<'_>> {
    if blob.len() < 12 || !blob.starts_with(b"RIFF") || &blob[8..12] != b"WAVE" {
        bail!("not a RIFF/WAVE file");
    }

    let mut at = 12usize;
    let mut found: Option<(Format, [u8; 4])> = None;
    let mut samples = None;
    let mut delay = 0;
    let mut frames: &[u8] = &[];
    while at + 8 <= blob.len() {
        let id = &blob[at..at + 4];
        let len = u32::from_le_bytes(blob[at + 4..at + 8].try_into().expect("four bytes")) as usize;
        if id == b"data" {
            frames = &blob[at + 8..(at + 8 + len).min(blob.len())];
            break;
        }
        let body = blob
            .get(at + 8..at + 8 + len)
            .context("a RIFF chunk runs past the end of the file")?;

        if id == b"fmt " {
            if body.len() < CONFIG_AT + 4 {
                bail!(
                    "the fmt chunk is {} bytes, too short for an extensible subformat GUID \
                     and ATRAC9's codec data",
                    body.len()
                );
            }
            let tag = u16::from_le_bytes(body[0..2].try_into().expect("two bytes"));
            if tag != 0xfffe {
                bail!("fmt chunk is not WAVE_FORMAT_EXTENSIBLE, so it names no subformat GUID");
            }
            if body[SUBFORMAT_GUID_AT..SUBFORMAT_GUID_AT + 16] != ATRAC9_GUID {
                bail!("fmt chunk's subformat GUID is not ATRAC9's");
            }
            found = Some((
                Format {
                    channels: u16::from_le_bytes(body[2..4].try_into().expect("two bytes")),
                    sample_rate: u32::from_le_bytes(body[4..8].try_into().expect("four bytes")),
                    block_align: u16::from_le_bytes(body[12..14].try_into().expect("two bytes")),
                },
                body[CONFIG_AT..CONFIG_AT + 4]
                    .try_into()
                    .expect("four bytes"),
            ));
        } else if id == b"fact" && body.len() >= 12 {
            samples = Some(u32::from_le_bytes(body[0..4].try_into().expect("four")));
            delay = u32::from_le_bytes(body[8..12].try_into().expect("four"));
        }

        // RIFF pads every odd-length chunk to an even boundary, and the pad
        // byte is not counted in the length.
        at += 8 + len + (len & 1);
    }

    let (format, config) = found.context("no fmt chunk")?;
    Ok(Header {
        format,
        config,
        samples,
        delay,
        frames,
    })
}

/// Whether `blob` is a RIFF-wrapped ATRAC9 stream, checked by its own
/// `WAVE_FORMAT_EXTENSIBLE` subformat GUID, and its geometry.
///
/// **Not** "any RIFF/WAVE with a `fmt ` chunk" - that is what
/// [`crate::at3::describe`] answers, deliberately loosely. Checking the GUID
/// here is what lets `crate::decode` try both without the looser one shadowing
/// this one.
///
/// # Errors
///
/// As [`header`].
pub fn describe(blob: &[u8]) -> Result<Format> {
    Ok(header(blob)?.format)
}

/// Decodes a RIFF-wrapped ATRAC9 file to interleaved 16-bit PCM, in process.
///
/// The `fact` chunk's encoder delay is skipped and the padding after its
/// declared length trimmed, so the result is exactly what the file says a
/// player hears. Without a `fact` chunk the whole decode is returned.
///
/// # Errors
///
/// As [`header`], plus a configuration word that disagrees with `fmt `, a
/// superframe that will not decode, or a `fact` chunk that declares more than
/// the frames hold.
pub fn decode(at9: &[u8]) -> Result<Pcm> {
    let h = header(at9)?;
    let mut samples = decode_superframes(&h.config, h.format, h.frames)?;
    let channels = usize::from(h.format.channels);
    if let Some(declared) = h.samples {
        let skip = h.delay as usize * channels;
        let keep = declared as usize * channels;
        ensure!(
            skip + keep <= samples.len(),
            "the fact chunk declares {} samples after a {}-sample delay; the frames decode to {}",
            declared,
            h.delay,
            samples.len() / channels.max(1)
        );
        samples.drain(..skip);
        samples.truncate(keep);
    }
    Ok(Pcm {
        samples,
        channels: h.format.channels,
        sample_rate: h.format.sample_rate,
    })
}

/// Decodes whole ATRAC9 superframes to interleaved 16-bit PCM, untrimmed.
///
/// `format` is what the container declares; the configuration word must agree
/// with it on channels, rate and superframe size, or this refuses rather than
/// decode at the wrong geometry.
///
/// # Errors
///
/// A configuration word `atrac9dec` refuses, a disagreement with `format`, a
/// payload that is not whole superframes, or a superframe that will not decode.
pub fn decode_superframes(config: &[u8; 4], format: Format, frames: &[u8]) -> Result<Vec<i16>> {
    let mut decoder =
        Atrac9Decoder::new(config).map_err(|e| anyhow!("the ATRAC9 config word: {e}"))?;
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
    ensure!(
        frames.len().is_multiple_of(info.superframe_size),
        "{} bytes of frames is not a whole number of {}-byte superframes",
        frames.len(),
        info.superframe_size
    );
    let per_superframe = info.channels * info.frame_samples * info.frames_in_superframe;
    let mut samples = Vec::with_capacity(frames.len() / info.superframe_size * per_superframe);
    let mut block = vec![0i16; per_superframe];
    for (index, superframe) in frames.chunks_exact(info.superframe_size).enumerate() {
        decoder
            .decode(superframe, &mut block)
            .map_err(|e| anyhow!("superframe {index}: {e}"))?;
        samples.extend_from_slice(&block);
    }
    Ok(samples)
}

#[cfg(test)]
mod tests;
