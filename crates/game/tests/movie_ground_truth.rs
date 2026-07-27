//! Validates the movie cache against a real disc, end to end.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship, and it needs `ffmpeg` on `PATH`. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! [ADR-0008](../../../docs/architecture/adr/0008-av1-movie-cache.md) turns the
//! movie cache into **lossless** AV1. Lossless is the entire basis for claiming
//! ADR-0004's fidelity rule still holds, and it is the one property that fails
//! *silently*: `ffmpeg`'s `-lossless 1` on its own is ignored and produces a
//! pretty, lossy picture that no other test here would notice.
//!
//! So this decodes the cache the game actually built and compares it, byte for
//! byte, with the same frames decoded straight from the H.264 by `ffmpeg`. If
//! the encoder flags in `movie.rs` ever stop meaning lossless, this is what
//! says so.
//!
//! The unit tests in `oag-formats::av1` cover the decoder and the rewind path
//! against a synthetic fixture, and those do run in CI.

use std::path::{Path, PathBuf};
use std::process::Command;

use oag_assets::pulse;
use oag_game::boot;
use oag_game::movie;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn have_ffmpeg() -> bool {
    let ok = Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        println!("skipping: ffmpeg is not on PATH");
    }
    ok
}

/// The frames this test converts. Small enough to keep the encode quick, big
/// enough to span more than one keyframe interval.
const FRAMES: usize = 48;

fn cache_dir() -> PathBuf {
    std::env::temp_dir().join("oag-movie-ground-truth")
}

fn load() -> Option<boot::Boot> {
    let image = image()?;
    if !have_ffmpeg() {
        return None;
    }
    let options = boot::Options {
        source: image.display().to_string(),
        movie: pulse::names::INTRO_MOVIE.to_string(),
        cache: cache_dir(),
        extent: movie::Extent::Frames(FRAMES),
        no_video: false,
    };
    Some(boot::load(&options).expect("loading the boot sequence"))
}

/// Decodes the cached elementary stream straight to raw frames, which is what
/// the cache used to hold and is the reference the AV1 must reproduce.
fn reference_frames(key_glob: &str, frame_len: usize) -> Vec<u8> {
    let dir = cache_dir();
    let es = std::fs::read_dir(&dir)
        .expect("the cache directory exists")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.extension().is_some_and(|e| e == "h264")
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with(key_glob))
        })
        .expect("the demuxed elementary stream was written beside the cache");

    let out = dir.join("reference.raw");
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(["-f", "h264"])
        .arg("-i")
        .arg(&es)
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

/// The cache must decode to exactly the frames `ffmpeg` gets from the H.264.
///
/// This is the test that catches a transcode that silently stopped being
/// lossless.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn the_cache_is_lossless() {
    let Some(loaded) = load() else { return };
    let mut movie = loaded.movie;
    let frames = movie
        .frames
        .as_mut()
        .expect("the movie transcoded; is libaom-av1 missing from this ffmpeg?");

    assert_eq!(frames.len, FRAMES, "the cache holds the frames asked for");

    let frame_len = frames.luma_len + 2 * frames.chroma_len;
    let key = frames
        .path()
        .file_name()
        .expect("the cache file has a name")
        .to_string_lossy()
        .split('-')
        .next()
        .expect("the cache name starts with the entry hash")
        .to_string();

    let reference = reference_frames(&key, frame_len);

    let mut decoded = Vec::new();
    for index in 0..FRAMES {
        frames.read_frame(index, &mut decoded).expect("decoding");
        let want = &reference[index * frame_len..(index + 1) * frame_len];
        assert_eq!(
            decoded.as_slice(),
            want,
            "frame {index} differs from what ffmpeg decodes: the transcode is not lossless"
        );
    }
}

/// Playing the same movie twice - which is what a looping movie does at every
/// wrap - must give the same picture.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn rewinding_a_real_movie_reproduces_its_frames() {
    let Some(loaded) = load() else { return };
    let mut movie = loaded.movie;
    let frames = movie.frames.as_mut().expect("the movie transcoded");

    let (mut first, mut again) = (Vec::new(), Vec::new());
    frames.read_frame(3, &mut first).expect("decoding forwards");
    frames
        .read_frame(FRAMES - 1, &mut Vec::new())
        .expect("running to the end");
    // Behind the cursor now: this flushes the decoder and replays.
    frames
        .read_frame(3, &mut again)
        .expect("decoding backwards");

    assert_eq!(first, again, "a rewind changed the picture");
}

/// The geometry the renderer is handed must be the movie's own.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn the_cache_geometry_matches_the_psmf_header() {
    let Some(loaded) = load() else { return };
    let movie = loaded.movie;
    let frames = movie.frames.as_ref().expect("the movie transcoded");

    assert_eq!((movie.width, movie.height), (480, 272));
    assert_eq!(frames.chroma_width, movie.width.div_ceil(2));
    assert_eq!(frames.chroma_height, movie.height.div_ceil(2));
    assert_eq!(frames.luma_len, (movie.width * movie.height) as usize);
}
