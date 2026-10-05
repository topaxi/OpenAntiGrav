//! ATRAC9, decoded out of process and cached - the Vita's own codec, the way
//! [`crate::at3`] is the PSP/PS2's ATRAC3+.
//!
//! # Why this shares `at3`'s plumbing rather than duplicating it
//!
//! [`crate::at3::decode_riff`] is already codec-agnostic in practice: it
//! writes the RIFF bytes to a scratch file and hands them to `ffmpeg`
//! unaltered, and `ffmpeg`'s own WAV demuxer reads the real
//! `WAVE_FORMAT_EXTENSIBLE` subformat GUID out of the `fmt ` chunk and picks
//! its own decoder from it - nothing in that path names ATRAC3+. What *is*
//! codec-specific is [`describe`] below: it refuses anything whose GUID is
//! not measured to be ATRAC9's, so a caller cannot end up here by accident
//! the way [`crate::at3::describe`] (which accepts any readable `fmt ` chunk,
//! full stop) would.
//!
//! # Where this came from
//!
//! 2048 ships `data/audio/music/FEMusic/frontend_stereo.at9` inside its base
//! `PSP2/data.psarc`, RIFF-wrapped exactly like the PSP titles' soundtrack -
//! 48000 Hz stereo, a 302-second `fact` chunk. See
//! `docs/formats/2048-frontend.md` for how that path was found and why it is
//! named [`oag_title::Music::front_end`] rather than a cue out of
//! `frontend.bnk`, a version-5 bank whose names are hashes
//! (`oag_formats::sblk::Bank::sound_names` returns empty for it;
//! `Bank::cue_named` resolves by FNV-1 hash; whether it carries a music cue
//! was not checked).

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::at3::{self, Format, Pcm};

/// The `WAVE_FORMAT_EXTENSIBLE` subformat GUID that means ATRAC9.
///
/// `{47E142D2-36BA-4D8D-88FC-61654F8C836C}` in the mixed-endian order a
/// `WAVEFORMATEXTENSIBLE` stores a GUID in, i.e. these bytes as they sit in
/// the file. Sony does not publish this one either, the same as `at3`'s own
/// ATRAC3+ GUID - measured directly off `frontend_stereo.at9`'s own `fmt `
/// chunk, 2026-09-25, the same way that module's own GUID was read off
/// `SND0.AT3`.
const ATRAC9_GUID: [u8; 16] = [
    0xd2, 0x42, 0xe1, 0x47, 0xba, 0x36, 0x8d, 0x4d, 0x88, 0xfc, 0x61, 0x65, 0x4f, 0x8c, 0x83, 0x6c,
];

/// Bytes into the `fmt ` chunk body where the subformat GUID starts: 16 for
/// the `WAVEFORMATEX` core plus 2 (`cbSize`) plus 2 (`wValidBitsPerSample`)
/// plus 4 (`dwChannelMask`).
const SUBFORMAT_GUID_AT: usize = 24;

/// Whether `blob` is a RIFF-wrapped ATRAC9 stream, checked by its own
/// `WAVE_FORMAT_EXTENSIBLE` subformat GUID.
///
/// **Not** "any RIFF/WAVE with a `fmt ` chunk" - that is what
/// [`crate::at3::describe`] answers, deliberately loosely, because nothing
/// downstream of it ever saw a non-ATRAC3+ RIFF file before this module
/// existed. Checking the GUID here is what lets `crate::music::decode` try
/// both without the looser one shadowing this one.
///
/// # Errors
///
/// Not a RIFF/WAVE, no readable `fmt ` chunk, a `fmt ` chunk too short to
/// carry an extensible subformat GUID, a format tag that is not
/// `WAVE_FORMAT_EXTENSIBLE`, or a GUID that is not ATRAC9's.
pub fn describe(blob: &[u8]) -> Result<Format> {
    if blob.len() < 12 || !blob.starts_with(b"RIFF") || &blob[8..12] != b"WAVE" {
        bail!("not a RIFF/WAVE file");
    }

    let mut at = 12usize;
    while at + 8 <= blob.len() {
        let id = &blob[at..at + 4];
        let len = u32::from_le_bytes(blob[at + 4..at + 8].try_into().expect("four bytes")) as usize;
        if id == b"data" {
            break;
        }
        let body = blob
            .get(at + 8..at + 8 + len)
            .context("a RIFF chunk runs past the end of the file")?;

        if id == b"fmt " {
            if body.len() < SUBFORMAT_GUID_AT + 16 {
                bail!(
                    "the fmt chunk is {} bytes, too short for an extensible subformat GUID",
                    body.len()
                );
            }
            let tag = u16::from_le_bytes(body[0..2].try_into().expect("two bytes"));
            if tag != 0xfffe {
                bail!("fmt chunk is not WAVE_FORMAT_EXTENSIBLE, so it names no subformat GUID");
            }
            let guid: [u8; 16] = body[SUBFORMAT_GUID_AT..SUBFORMAT_GUID_AT + 16]
                .try_into()
                .expect("sixteen bytes");
            if guid != ATRAC9_GUID {
                bail!("fmt chunk's subformat GUID is not ATRAC9's");
            }
            return Ok(Format {
                channels: u16::from_le_bytes(body[2..4].try_into().expect("two bytes")),
                sample_rate: u32::from_le_bytes(body[4..8].try_into().expect("four bytes")),
                block_align: u16::from_le_bytes(body[12..14].try_into().expect("two bytes")),
            });
        }

        // RIFF pads every odd-length chunk to an even boundary, and the pad
        // byte is not counted in the length - see `at3::describe`'s own copy
        // of this rule.
        at += 8 + len + (len & 1);
    }

    bail!("no fmt chunk")
}

/// Decodes a RIFF-wrapped ATRAC9 file, through the same `ffmpeg`-backed cache
/// [`crate::at3::decode`] uses - see this module's own doc for why sharing it
/// is safe rather than accidental.
///
/// # Errors
///
/// As [`describe`], plus an `ffmpeg` that is absent or fails, or a cache file
/// that will not write or read back.
pub fn decode(at9: &[u8], cache_dir: &Path) -> Result<Pcm> {
    let format = describe(at9)?;
    at3::decode_riff(at9, format, cache_dir)
}

#[cfg(test)]
mod tests;
