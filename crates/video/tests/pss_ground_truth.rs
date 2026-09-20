//! Validates `oag_video::pss` against the real PS2 disc.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.

use std::path::Path;

use oag_disc::DiscImage;
use oag_video::pss;

/// The disc both intro cuts live on.
const IMAGE: &str = "pulse-ps2-eu.chd";

/// The 640-wide intro cut - what `just play` actually boots with.
const INTRO640: &str = "54748/DATA/MOVIES/INTRO640.PSS";

/// The 512-wide intro cut, on the same disc. See the module docs on
/// [`pss`] for why this is measured separately rather than assumed to match
/// [`INTRO640`]'s body length - only the framing is shared.
const INTRO512: &str = "54748/DATA/MOVIES/INTRO512.PSS";

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image(IMAGE)
}

fn read_file(image: &Path, path: &str) -> Vec<u8> {
    let mut disc = DiscImage::open(image).expect("the image opens");
    disc.read_file(path)
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Measured 2026-09-17 on the intro cut this disc boots with, and again once
/// `oag_video::pss` existed to check itself against the disc rather than a
/// hand probe: 1,795 `private_stream_1` packets and a `SSbd`-declared body of
/// exactly 7,303,168 bytes, 48 kHz stereo, `format` code 1 - which this
/// project has since told apart from PS-ADPCM as plain 16-bit PCM. See the
/// module docs on `oag_video::pss` for the evidence.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd - run under `just test-data`"]
fn intro640_demuxes_to_the_measured_body() {
    let Some(image) = image() else { return };
    let blob = read_file(&image, INTRO640);

    let demuxed = pss::demux(&blob).expect("demuxing INTRO640.PSS");

    assert_eq!(demuxed.packet_count, 1_795);
    assert_eq!(demuxed.body.len(), 7_303_168);
    assert!(demuxed.format.is_pcm16());
    assert_eq!(demuxed.format.sample_rate, 48_000);
    assert_eq!(demuxed.format.channels, 2);
    assert_eq!(demuxed.format.interleave, 512);

    // The body is a whole number of stereo interleave pairs (512 bytes per
    // channel, two channels), with no trailing partial pair - what makes a
    // clean per-channel de-interleave possible with nothing left over.
    assert_eq!(demuxed.body.len() % (demuxed.format.interleave * 2), 0);
}

/// The 512-wide cut: same framing, same format, a body measured on its own
/// rather than assumed to match [`INTRO640`]'s duration.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd - run under `just test-data`"]
fn intro512_demuxes_with_the_same_format_and_its_own_body_length() {
    let Some(image) = image() else { return };
    let blob = read_file(&image, INTRO512);

    let demuxed = pss::demux(&blob).expect("demuxing INTRO512.PSS");

    assert_eq!(demuxed.packet_count, 1_794);
    assert_eq!(demuxed.body.len(), 7_296_000);
    assert!(demuxed.format.is_pcm16());
    assert_eq!(demuxed.format.sample_rate, 48_000);
    assert_eq!(demuxed.format.channels, 2);
    assert_eq!(demuxed.format.interleave, 512);
}
