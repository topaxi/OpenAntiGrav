//! Validates Wipeout HD / Fury's Bink video against its real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! Two things, and they fail in different ways.
//!
//! [`every_bik_on_the_disc_reads_its_own_header`] is the **survey**, and it is
//! the whole of `docs/formats/bik.md`'s confidence score expressed as a test:
//! three arithmetic invariants over all 37 files, none of which can hold by
//! accident, and the third of which can only hold if the variable-length audio
//! arrays in the middle of the header were sized and ordered right.
//!
//! [`the_bink_cache_is_lossless`] is the **transcode**, and it is the
//! `movie_ground_truth.rs` check pointed at a fourth container: what comes back
//! out of the AV1 cache must be, byte for byte, what `ffmpeg` decodes straight
//! from the `.bik`. It is the test that catches a conversion which quietly
//! stopped being lossless, and here it also catches a subtler thing - the logo
//! reel carries four audio tracks inside the same file, so it is the first
//! transcode input this project has handed `ffmpeg` that holds more than a
//! picture.

use std::path::{Path, PathBuf};
use std::process::Command;

use oag_game::movie;
use oag_video::bik;

/// The decrypted HD/Fury image, if it is there.
///
/// Only the decrypted one: a PS3 disc reads as noise until layer 1 is
/// decrypted, so the encrypted image beside it is not a fallback. Skipped
/// rather than failed when absent, the way the other ground-truth tests do it.
fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// The seven archives, so the sweep can find every `.bik` wherever it lives.
const ARCHIVES: [&str; 7] = [
    "DATA00.PSARC",
    "DATA01.PSARC",
    "DATA02.PSARC",
    "DATA03.PSARC",
    "DATA04.PSARC",
    "DATA05.PSARC",
    "DATA06.PSARC",
];

/// The logo reel the front end's own XML names, and the archive holding it.
///
/// `DATA02`'s copy rather than `DATA00`'s `_fury` one, and that is a **choice,
/// not a reading**: `oag_hd::frontend::names::STUDIO_LOGO_MOVIE` is the
/// spelling `DATA06`'s `skin.xml` uses, which of the six skins the runtime
/// loads is documented as unknown, and the two families name different reels.
/// This test pins the one whose path matches that constant; nothing here claims
/// it is the one a PS3 plays.
const LOGO_ARCHIVE: &str = "DATA02.PSARC";

/// Its path inside that archive, lower-cased the way the PSARC stores it.
const LOGO_PATH: &str = "/data/fe/images/studioliverpool.bik";

/// How many frames of the reel the lossless check converts.
///
/// Four rather than all 531, because this is 1920x1080: the whole reel is about
/// 98 seconds of `libaom` and 21 MiB of cache, measured on 2026-08-17, and what
/// is under test is whether the conversion is lossless rather than how long it
/// runs. `movie_ground_truth.rs` caps its own check at 24 for the same reason
/// at a twenty-sixth of the pixels.
const FRAMES: usize = 4;

/// Every `.bik` on the disc, as `(archive, path, bytes)`.
fn every_bik(image: &Path) -> Vec<(&'static str, String, Vec<u8>)> {
    let mut found = Vec::new();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|path| path.to_ascii_lowercase().ends_with(".bik"))
            .cloned()
            .collect();
        for path in paths {
            let blob = open.read_path(&path).expect("reading a .bik entry");
            found.push((archive, path, blob));
        }
    }
    found
}

/// One entry, straight out of its archive.
fn read(image: &Path, archive: &str, path: &str) -> Vec<u8> {
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    open.read_path(path).expect("reading the entry")
}

/// Where this test's own scratch files go.
fn cache_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/cache/hd-movie-ground-truth")
}

/// The disc carries 37 `.bik` files and every one of them parses.
///
/// **Three independent invariants per file**, which is what puts
/// `docs/formats/bik.md` in the rubric's 85-94 band:
///
/// 1. the length at `+0x04` plus 8 is the entry's real size;
/// 2. the frame count at `+0x10` repeats the one at `+0x08`;
/// 3. the first entry of the offset table, keyframe bit masked off, lands
///    exactly where this layout says the header ends.
///
/// The third is the load-bearing one. It walks past the per-track arrays, so it
/// can only agree when the track count, the array order and the array widths
/// were all read right - and six of the files have tracks where 31 have none,
/// which is what makes the sweep test both shapes rather than one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_bik_on_the_disc_reads_its_own_header() {
    let Some(image) = image() else { return };
    let files = every_bik(&image);
    assert_eq!(files.len(), 37, "the disc carries 37 .bik entries");

    let mut with_audio = 0;
    for (archive, path, blob) in &files {
        let header = bik::parse(blob).unwrap_or_else(|e| panic!("{archive} {path}: {e}"));

        assert!(
            header.declares_length_of(blob.len()),
            "{archive} {path} declares {} byte(s) and the entry is {}",
            header.declared_len,
            blob.len()
        );
        assert_eq!(
            bik::first_frame_offset(blob, &header),
            Some(header.header_len),
            "{archive} {path}'s first frame does not land on the end of its header"
        );
        assert_eq!(
            header.revision,
            bik::SHIPPED_REVISION,
            "{archive} {path} is not the revision the survey measured"
        );
        assert!(
            header.frame_count > 0 && header.width > 0 && header.height > 0,
            "{archive} {path} declares nothing to play"
        );
        // The second invariant, read off the bytes rather than through
        // `bik::parse` - the parser has no reason to carry a field that only
        // ever repeats another, and asserting it here is what keeps "it repeats"
        // a measurement rather than an assumption the parser encodes.
        let repeated = u32::from_le_bytes(blob[0x10..0x14].try_into().expect("four bytes"));
        assert_eq!(
            repeated as usize, header.frame_count,
            "{archive} {path}'s two frame counts disagree"
        );

        if !header.audio.is_empty() {
            with_audio += 1;
            for track in &header.audio {
                assert_eq!(track.sample_rate, 48_000, "{archive} {path}");
                assert_eq!(track.channels(), 2, "{archive} {path}");
                assert!(track.is_dct(), "{archive} {path}");
            }
        }
    }

    // Six: the four Zone circuit previews carry one track each and the two
    // studio logo reels carry four. Asserted so that a parse which started
    // reading the track count out of the wrong field - and so found audio
    // everywhere or nowhere - fails here rather than passing the three
    // invariants above by luck.
    assert_eq!(with_audio, 6, "six of the 37 files carry audio");
}

/// The logo reel is 1080p at 59.94 Hz, with four stereo tracks inside it.
///
/// Pinned separately from the sweep because these are the numbers anything
/// playing the reel has to agree with, and because they are the first evidence
/// in this project of what is actually *inside* a `.bik` -
/// `oag_hd::frontend::names::STUDIO_LOGO_MOVIE`'s own doc comment records that
/// the name was the whole of the evidence until now.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_logo_reel_is_1080p_at_59_94_with_four_stereo_tracks() {
    let Some(image) = image() else { return };
    let blob = read(&image, LOGO_ARCHIVE, LOGO_PATH);
    let header = bik::parse(&blob).expect("parsing the reel");

    assert_eq!((header.width, header.height), (1920, 1080));
    assert_eq!(header.frame_count, 531);
    assert_eq!(header.frame_rate, (10_000_000, 166_833));
    assert_eq!(header.audio.len(), 4);
    // 8.8588 s. The same count against a rounded 60 Hz would be 8.85, which is
    // why `Header::frame_rate` stays the file's own fraction.
    assert!(
        (header.seconds() - 8.8588).abs() < 0.0001,
        "{}",
        header.seconds()
    );

    // The `.bik` path is the only one that can answer this with no `ffmpeg` at
    // all, because it is the only one reading a container header for it: a
    // program stream has none, and this is the difference that buys.
    let movie = movie::open(
        &blob,
        "hd-logo-no-video",
        &cache_dir(),
        movie::Extent::Whole,
        movie::Decode {
            no_video: true,
            ..movie::Decode::default()
        },
        None,
    )
    .expect("opening it without decoding");
    assert_eq!(movie.frame_count, 531);
    assert_eq!((movie.width, movie.height), (1920, 1080));
    assert_eq!(movie.frame_rate, (10_000_000, 166_833));
    assert!(movie.frames.is_none());
}

/// The cache must decode to exactly the frames `ffmpeg` gets from the `.bik`.
///
/// The `movie_ground_truth.rs` check at a fourth container, and it carries one
/// extra thing there: the reel has four audio tracks in the same file, so this
/// is also what proves an IVF holding video alone comes back out of an input
/// holding more than video.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn the_bink_cache_is_lossless() {
    let Some(image) = image() else { return };
    let blob = read(&image, LOGO_ARCHIVE, LOGO_PATH);

    let mut movie = movie::open(
        &blob,
        "hd-logo",
        &cache_dir(),
        movie::Extent::Frames(FRAMES),
        movie::Decode::default(),
        None,
    )
    .expect("opening the reel");
    let frames = movie
        .frames
        .as_mut()
        .expect("the reel transcoded; is libaom-av1 missing from this ffmpeg?");

    assert_eq!(frames.len, FRAMES, "the cache holds the frames asked for");

    let frame_len = frames.luma_len + 2 * frames.chroma_len;
    let reference = reference_frames(&blob, frame_len);

    let mut decoded = movie::VideoFrame::default();
    for index in 0..FRAMES {
        frames.read_frame(index, &mut decoded).expect("decoding");
        let want = &reference[index * frame_len..(index + 1) * frame_len];
        assert_eq!(
            decoded.bytes.as_slice(),
            want,
            "frame {index} differs from what ffmpeg decodes: the transcode is not lossless"
        );
    }
}

/// Decodes a `.bik` straight to raw frames with `ffmpeg`, which is the
/// reference the cache must reproduce exactly.
///
/// The whole file goes in, container and all - no demux step, which is the
/// thing that makes this path short - and `-an` is not passed for the same
/// reason the transcode does not pass it: a `rawvideo` output takes the video
/// stream and leaves the four audio ones where they are.
fn reference_frames(blob: &[u8], frame_len: usize) -> Vec<u8> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).expect("creating the reference scratch directory");
    let source = dir.join("reference.bik");
    std::fs::write(&source, blob).expect("writing the reel");

    let out = dir.join("reference.raw");
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .arg("-i")
        .arg(&source)
        .args(["-frames:v", &FRAMES.to_string()])
        .args(["-f", "rawvideo"])
        .args(["-pix_fmt", movie::PIXEL_FORMAT])
        .arg(&out)
        .status()
        .expect("running ffmpeg");
    assert!(status.success(), "ffmpeg failed to decode the reference");

    let bytes = std::fs::read(&out).expect("reading the reference frames");
    assert_eq!(
        bytes.len(),
        frame_len * FRAMES,
        "the reference is not {FRAMES} whole frames"
    );
    bytes
}
