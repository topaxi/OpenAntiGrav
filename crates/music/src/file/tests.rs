use super::*;

/// A half second of a 440 Hz stereo sine at 48 kHz, made for this repository.
fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Counts upward zero crossings, so a decoder that returns silence or noise fails.
fn rising_crossings(pcm: &Pcm) -> usize {
    let left: Vec<i16> = pcm
        .samples
        .iter()
        .step_by(usize::from(pcm.channels))
        .copied()
        .collect();
    left.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count()
}

fn check(name: &str, extension: &str, frames: std::ops::RangeInclusive<usize>) {
    let pcm = decode(&fixture(name), extension).expect("decodes");
    assert_eq!((pcm.channels, pcm.sample_rate), (2, 48_000), "{name}");
    let got = pcm.samples.len() / 2;
    assert!(frames.contains(&got), "{name}: {got} frames");
    let crossings = rising_crossings(&pcm);
    assert!(
        (218..=222).contains(&crossings),
        "{name}: {crossings} cycles"
    );
}

#[test]
fn flac_decodes_to_its_exact_length() {
    check("sine.flac", "flac", 24_000..=24_000);
}

#[test]
fn ogg_vorbis_decodes() {
    check("sine.ogg", "ogg", 24_000..=25_100);
}

#[test]
fn opus_decodes_without_its_pre_skip() {
    check("sine.opus", "opus", 24_000..=24_000);
}

#[test]
fn a_wrong_extension_is_an_error_not_a_misreading() {
    assert!(decode(b"not audio at all, not audio at all", "flac").is_err());
}

/// An ID3v2.4 text frame holding `text` as UTF-8.
fn id3_frame(id: &str, text: &str) -> Vec<u8> {
    let body_len = text.len() + 1;
    let mut out = id.as_bytes().to_vec();
    out.extend(syncsafe(body_len));
    out.extend([0, 0, 3]);
    out.extend(text.as_bytes());
    out
}

fn syncsafe(n: usize) -> [u8; 4] {
    [
        ((n >> 21) & 0x7f) as u8,
        ((n >> 14) & 0x7f) as u8,
        ((n >> 7) & 0x7f) as u8,
        (n & 0x7f) as u8,
    ]
}

/// Twenty silent MPEG-1 Layer III frames, 128 kbit/s at 44.1 kHz (417 bytes each), behind an
/// ID3v2.4 tag carrying `frames`.
fn silent_mp3(frames: &[Vec<u8>]) -> Vec<u8> {
    let body: Vec<u8> = frames.concat();
    let mut out = b"ID3\x04\x00\x00".to_vec();
    out.extend(syncsafe(body.len()));
    out.extend(body);
    for _ in 0..20 {
        let mut frame = vec![0xFF, 0xFB, 0x90, 0x64];
        frame.resize(417, 0);
        out.extend(frame);
    }
    out
}

#[test]
fn an_id3_tag_names_the_mp3() {
    let blob = silent_mp3(&[
        id3_frame("TIT2", "Song Name"),
        id3_frame("TPE1", "The Artist"),
    ]);
    let (pcm, tags) = decode_tagged(&blob, "mp3").expect("decodes");
    assert!(!pcm.samples.is_empty());
    assert_eq!(tags.title.as_deref(), Some("Song Name"));
    assert_eq!(tags.artist.as_deref(), Some("The Artist"));
}

#[test]
fn an_untagged_mp3_has_no_tags() {
    let (_, tags) = decode_tagged(&silent_mp3(&[]), "mp3").expect("decodes");
    assert_eq!(tags, Tags::default());
}

/// `sine.flac` with a `VORBIS_COMMENT` block inserted after `STREAMINFO`.
fn tagged_flac(comments: &[&str]) -> Vec<u8> {
    let flac = fixture("sine.flac");
    assert_eq!(&flac[..4], b"fLaC");
    // STREAMINFO is 34 bytes behind a 4-byte header; clear its "last block" flag when set.
    let streaminfo_end = 4 + 4 + 34;
    let was_last = flac[4] & 0x80 != 0;
    let mut block = Vec::new();
    let vendor = b"oag";
    block.extend((vendor.len() as u32).to_le_bytes());
    block.extend(vendor);
    block.extend((comments.len() as u32).to_le_bytes());
    for comment in comments {
        block.extend((comment.len() as u32).to_le_bytes());
        block.extend(comment.as_bytes());
    }
    let mut out = flac[..streaminfo_end].to_vec();
    out[4] &= 0x7f;
    let len = block.len() as u32;
    out.push(if was_last { 0x84 } else { 0x04 });
    out.extend(&len.to_be_bytes()[1..]);
    out.extend(block);
    out.extend(&flac[streaminfo_end..]);
    out
}

#[test]
fn a_vorbis_comment_names_the_flac() {
    let blob = tagged_flac(&["TITLE=Flac Song", "ARTIST=Flac Artist"]);
    let (pcm, tags) = decode_tagged(&blob, "flac").expect("decodes");
    assert_eq!(pcm.samples.len() / 2, 24_000);
    assert_eq!(tags.title.as_deref(), Some("Flac Song"));
    assert_eq!(tags.artist.as_deref(), Some("Flac Artist"));
}
