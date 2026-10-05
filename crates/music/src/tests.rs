//! The one part of this module that needs no disc: the gate both routes run
//! every candidate through, and the dispatch between the two containers.
//!
//! The MPEG half of that dispatch is asserted against **real** streams in
//! `crates/game/tests/hd_music_ground_truth.rs`, because a synthetic MPEG frame
//! is not something a decoder will open and a fixture that only proves "this is
//! not MPEG" would prove nothing about the case that matters.

use super::*;

/// A RIFF/WAVE header with a `fmt ` and a `fact` chunk and nothing else,
/// shaped the way a `Data.wad` entry's first kibibyte is.
fn header(channels: u16, sample_rate: u32, samples: u32) -> Vec<u8> {
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&0xfffe_u16.to_le_bytes()); // format tag, unread
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&sample_rate.to_le_bytes());
    fmt.extend_from_slice(&0u32.to_le_bytes()); // bytes per second, unread
    fmt.extend_from_slice(&560u16.to_le_bytes()); // block align
    fmt.extend_from_slice(&0u16.to_le_bytes()); // bits per sample, unread

    let mut out = b"RIFF\0\0\0\0WAVE".to_vec();
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&(u32::try_from(fmt.len()).expect("a small chunk")).to_le_bytes());
    out.extend_from_slice(&fmt);
    out.extend_from_slice(b"fact");
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&samples.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

/// The length comes from the `fact` count over the rate, and nothing else.
#[test]
fn a_stereo_stream_at_the_soundtracks_rate_is_measured_by_its_fact_chunk() {
    let seconds = riff_seconds(&header(2, 44_100, 44_100 * 180)).expect("a length");
    assert!((seconds - 180.0).abs() < 1e-9, "{seconds}");
}

/// **Checked on both routes, not just the population one.** A declared entry
/// that turned out to be mono would be a finding rather than something to play
/// at the wrong speed, which is why this gate sits below both.
#[test]
fn a_stream_that_is_not_stereo_at_44_100_is_not_a_soundtrack_track() {
    assert_eq!(
        riff_seconds(&header(1, 44_100, 44_100 * 180)),
        None,
        "the disc's 32 mono ATRAC3+ streams share the bitrate and the rate"
    );
    assert_eq!(riff_seconds(&header(2, 48_000, 48_000 * 180)), None);
}

/// A `fact` chunk is what a length needs, so an entry without one is skipped
/// rather than measured off its stored size - the padding error the module
/// docs record.
#[test]
fn a_stream_with_no_fact_chunk_has_no_length() {
    let mut without = header(2, 44_100, 0);
    let at = without
        .windows(4)
        .position(|window| window == b"fact")
        .expect("the fixture writes one");
    without.splice(at..at + 4, *b"junk");
    assert_eq!(riff_seconds(&without), None);
}

/// Anything that is not a RIFF/WAVE at all - a movie, a texture, a `.vex` -
/// falls out here rather than raising, because a bulk archive is full of them.
#[test]
fn a_blob_that_is_not_riff_is_skipped_rather_than_reported() {
    assert_eq!(riff_seconds(b"not a riff file at all"), None);
    assert_eq!(riff_seconds(&[]), None);
}

/// A blob that is neither container measures as nothing, rather than as one of
/// them with a wrong answer. The dispatch's failure case, and the one an
/// archive full of textures and models exercises constantly.
#[test]
fn a_blob_that_is_neither_container_measures_as_nothing() {
    assert_eq!(seconds_of(b"not a riff and not an mpeg frame"), None);
    assert_eq!(seconds_of(&[]), None);
}
