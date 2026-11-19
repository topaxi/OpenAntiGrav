//! AAC in an MP4, decoded in process: Wipeout 2048's `intro.mp4` sound track.
//!
//! The only MP4 of 2048's 26 that carries audio (`docs/formats/mp4.md`, "The
//! audio is inside the container too"): stereo 48 kHz AAC, 4,666 frames.
//! `symphonia` reads the container and decodes the codec, as it does HD's MP3
//! in [`crate::mp3`]; the conformance against FFmpeg is on
//! `docs/formats/mp4.md`.

use anyhow::Result;

use crate::at3::Pcm;

/// Decodes the first audio track of an MP4 file to interleaved 16-bit PCM.
///
/// # Errors
///
/// A blob that is not an MP4 `symphonia` reads, a track whose codec it does
/// not decode, or one that decodes to nothing at all.
pub fn decode_mp4(blob: &[u8]) -> Result<Pcm> {
    crate::file::decode(blob, "mp4")
}
