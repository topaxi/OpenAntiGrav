//! What [`super::describe`] is asserted to do: accept the real shape
//! `frontend_stereo.at9`'s own `fmt ` chunk has, and refuse everything that
//! is not that - a bare RIFF/WAVE with no subformat GUID at all (which
//! `at3::describe` accepts, deliberately loosely) and a GUID that names some
//! other codec.

use super::*;

/// Builds a minimal RIFF/WAVE with a 52-byte `WAVEFORMATEXTENSIBLE` `fmt `
/// chunk carrying `guid`, and an empty `data` chunk - enough for [`describe`]
/// to walk, nothing [`super::decode`] would try to hand `ffmpeg`.
fn riff_with_guid(channels: u16, sample_rate: u32, block_align: u16, guid: [u8; 16]) -> Vec<u8> {
    let mut fmt_body = Vec::with_capacity(52);
    fmt_body.extend_from_slice(&0xfffe_u16.to_le_bytes()); // wFormatTag: extensible
    fmt_body.extend_from_slice(&channels.to_le_bytes());
    fmt_body.extend_from_slice(&sample_rate.to_le_bytes());
    fmt_body.extend_from_slice(&0u32.to_le_bytes()); // nAvgBytesPerSec, unread
    fmt_body.extend_from_slice(&block_align.to_le_bytes());
    fmt_body.extend_from_slice(&0u16.to_le_bytes()); // wBitsPerSample
    fmt_body.extend_from_slice(&34u16.to_le_bytes()); // cbSize
    fmt_body.extend_from_slice(&0u16.to_le_bytes()); // wValidBitsPerSample/samples-per-frame
    fmt_body.extend_from_slice(&3u32.to_le_bytes()); // dwChannelMask
    fmt_body.extend_from_slice(&guid);
    fmt_body.extend_from_slice(&[0u8; 12]); // codec extra data, unread here
    assert_eq!(fmt_body.len(), 52);

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&0u32.to_le_bytes()); // size, unread by describe
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&(fmt_body.len() as u32).to_le_bytes());
    out.extend_from_slice(&fmt_body);
    out.extend_from_slice(b"data");
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

#[test]
fn the_real_files_own_guid_and_geometry_read() {
    let blob = riff_with_guid(2, 48_000, 384, ATRAC9_GUID);
    let format = describe(&blob).expect("ATRAC9's own GUID must resolve");
    assert_eq!(format.channels, 2);
    assert_eq!(format.sample_rate, 48_000);
    assert_eq!(format.block_align, 384);
}

/// The GUID a `WAVE_FORMAT_EXTENSIBLE` `fmt ` chunk with no subformat at all
/// would carry is not this one - a zeroed GUID stands in for "anything but
/// ATRAC9" here, since the point is that *some other* codec's stream must not
/// resolve.
#[test]
fn a_different_subformat_guid_is_refused() {
    let blob = riff_with_guid(2, 48_000, 384, [0u8; 16]);
    assert!(describe(&blob).is_err());
}

/// Wipeout Pulse's own ATRAC3+ GUID, refused the same way - this is the
/// specific case [`super`]'s own module doc explains sharing `at3::decode_riff`
/// does not risk: `describe` never lets an ATRAC3+ stream through as if it
/// were ATRAC9's.
#[test]
fn atrac3plus_s_own_guid_is_refused_too() {
    const ATRAC3PLUS_GUID: [u8; 16] = [
        0xbf, 0xaa, 0x23, 0xe9, 0x58, 0xcb, 0x71, 0x44, 0xa1, 0x19, 0xff, 0xfa, 0x01, 0xe4, 0xce,
        0x62,
    ];
    let blob = riff_with_guid(2, 44_100, 560, ATRAC3PLUS_GUID);
    assert!(describe(&blob).is_err());
}

#[test]
fn a_non_extensible_format_tag_names_no_subformat() {
    // wFormatTag PCM (1) rather than WAVE_FORMAT_EXTENSIBLE (0xfffe): the
    // bytes after it are not a subformat GUID at all, and reading them as
    // one would be a coincidence, not a check.
    let mut blob = riff_with_guid(2, 48_000, 384, ATRAC9_GUID);
    // The `fmt ` chunk body starts right after the 8-byte "fmt "+len header,
    // itself right after the 12-byte RIFF/WAVE preamble.
    blob[20] = 0x01;
    blob[21] = 0x00;
    assert!(describe(&blob).is_err());
}

#[test]
fn not_a_riff_file_at_all_is_refused() {
    assert!(describe(b"not a riff file, just some bytes").is_err());
}

#[test]
fn a_truncated_fmt_chunk_is_refused_rather_than_panicking() {
    let blob = riff_with_guid(2, 48_000, 384, ATRAC9_GUID);
    // Cut the file inside the fmt chunk body, well before the subformat GUID.
    let truncated = &blob[..28];
    assert!(describe(truncated).is_err());
}
