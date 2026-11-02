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
