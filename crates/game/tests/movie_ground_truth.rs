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

use oag_game::boot;
use oag_game::movie;
use oag_pulse as pulse;

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
        // No saved language: these boot a fresh install every time.
        language: None,
        source: image.display().to_string(),
        // No downloadable content either: a pack carries no movies.
        dlc: Vec::new(),
        leg: oag_game::frontend::Leg::LogoFmv,
        movie: Some(pulse::names::INTRO_MOVIE.to_string()),
        cache: cache_dir(),
        audio_cache: oag_game::boot::default_audio_cache_dir(),
        extent: movie::Extent::Frames(FRAMES),
        no_video: false,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    Some(boot::load(&options).expect("loading the boot sequence"))
}

/// Like [`load`], but asks for the intro's full length (`Extent::Whole`)
/// rather than [`FRAMES`]. Real gameplay mostly plays movies this way -
/// `Extent::Frames` exists for the intro's own early cutoff and not much
/// else - so a decoder that is only ever exercised against a truncated
/// extent in tests could have a wanted-count bug nothing here would catch.
#[cfg(all(target_os = "linux", feature = "native-video"))]
fn load_whole() -> Option<boot::Boot> {
    let image = image()?;
    if !have_ffmpeg() {
        return None;
    }
    let options = boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_game::frontend::Leg::LogoFmv,
        movie: Some(pulse::names::INTRO_MOVIE.to_string()),
        cache: cache_dir(),
        audio_cache: oag_game::boot::default_audio_cache_dir(),
        extent: movie::Extent::Whole,
        no_video: false,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    Some(boot::load(&options).expect("loading the boot sequence"))
}

/// Demuxes the intro's `.PMF` straight off the disc image, independent of
/// whatever the boot path under test did with it - so this reference does
/// not depend on the AV1 cache's `transcode` step having run and left the
/// elementary stream sitting beside it, which the `native-video` path has no
/// reason to do (see [`GstDecoder`](../src/movie/gst.rs), which decodes the
/// demuxed bytes in memory and never touches the cache directory at all).
fn intro_elementary_stream(image: &Path) -> Vec<u8> {
    let mut archives =
        pulse::open(&image.display().to_string()).expect("opening the disc's archives");
    let blob = archives
        .read_name(pulse::names::INTRO_MOVIE)
        .expect("reading the intro movie out of its archive");
    let demuxed = oag_formats::pmf::demux(&blob).expect("demuxing the intro's program stream");
    demuxed.video
}

/// Decodes an elementary stream straight to raw frames with `ffmpeg`, which
/// is the reference every decoder under test must reproduce exactly.
fn reference_frames(elementary_stream: &[u8], frame_len: usize) -> Vec<u8> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).expect("creating the reference scratch directory");
    let es = dir.join("reference.h264");
    std::fs::write(&es, elementary_stream).expect("writing the elementary stream");

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
    let Some(image) = image() else { return };
    let Some(loaded) = load() else { return };
    let mut movie = loaded.movie.expect("the PSP disc carries the reel");
    let frames = movie
        .frames
        .as_mut()
        .expect("the movie transcoded; is libaom-av1 missing from this ffmpeg?");

    assert_eq!(frames.len, FRAMES, "the cache holds the frames asked for");

    let frame_len = frames.luma_len + 2 * frames.chroma_len;
    let reference = reference_frames(&intro_elementary_stream(&image), frame_len);

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

/// The `native-video` feature must actually be the one serving the frames it
/// produced, not silently falling back to the AV1 cache. `the_cache_is_lossless`
/// above is decoder-agnostic - it passes just as well whichever tier serves
/// the movie - so on its own it cannot tell a healthy GStreamer install from
/// a broken one that fell through to the ffmpeg cache unnoticed. This is the
/// check that closes that gap: it fails loudly instead of quietly passing for
/// the wrong reason. See [ADR-0017](../../../docs/architecture/adr/0017-gstreamer-native-video.md).
#[test]
#[cfg(all(target_os = "linux", feature = "native-video"))]
#[ignore = "needs data/images/, ffmpeg, and a working GStreamer H.264 decode element"]
fn native_video_decodes_through_gstreamer_not_the_cache() {
    let Some(image) = image() else { return };
    let Some(loaded) = load() else { return };
    let mut movie = loaded.movie.expect("the PSP disc carries the reel");
    let frames = movie
        .frames
        .as_mut()
        .expect("the movie transcoded; is libaom-av1 missing from this ffmpeg?");

    let name = frames
        .path()
        .file_name()
        .expect("the cache file has a name")
        .to_string_lossy()
        .into_owned();
    if !name.ends_with("-gst") {
        println!(
            "skipping: {name} was not served by GStreamer - no working \
             native-video decode element on this machine, fell back to the \
             AV1 cache instead"
        );
        return;
    }

    assert_eq!(frames.len, FRAMES, "the cache holds the frames asked for");

    let frame_len = frames.luma_len + 2 * frames.chroma_len;
    let reference = reference_frames(&intro_elementary_stream(&image), frame_len);

    let mut decoded = movie::VideoFrame::default();
    for index in 0..FRAMES {
        frames.read_frame(index, &mut decoded).expect("decoding");
        let want = &reference[index * frame_len..(index + 1) * frame_len];
        assert_eq!(
            decoded.bytes.as_slice(),
            want,
            "frame {index} differs from what ffmpeg decodes: GStreamer's decode is not correct"
        );
    }
}

/// Playing the same movie twice - which is what a looping movie does at every
/// wrap - must give the same picture.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn rewinding_a_real_movie_reproduces_its_frames() {
    let Some(loaded) = load() else { return };
    let mut movie = loaded.movie.expect("the PSP disc carries the reel");
    let frames = movie.frames.as_mut().expect("the movie transcoded");

    let (mut first, mut again) = (movie::VideoFrame::default(), movie::VideoFrame::default());
    frames.read_frame(3, &mut first).expect("decoding forwards");
    frames
        .read_frame(FRAMES - 1, &mut movie::VideoFrame::default())
        .expect("running to the end");
    // Behind the cursor now: this flushes the decoder and replays.
    frames
        .read_frame(3, &mut again)
        .expect("decoding backwards");

    assert_eq!(first.bytes, again.bytes, "a rewind changed the picture");
}

/// A [`movie::Feed`] must hand over the same frames, in the same order, that
/// reading the store straight through does.
///
/// The unit tests in `movie.rs` cover the ring and the position arithmetic
/// without a decoder. What only a real movie can cover is the **worker**: that
/// the frames it pushes are the right ones, that it runs ahead rather than
/// waiting to be asked, and that a looping feed wraps back to frame 0 without
/// replaying the movie.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn a_feed_hands_over_the_frames_the_store_would() {
    let Some(loaded) = load() else { return };
    let mut movie = loaded.movie.expect("the PSP disc carries the reel");
    let store = movie.frames.take().expect("the movie transcoded");

    // The reference, decoded synchronously the way the render thread used to.
    let mut reference = Vec::new();
    let mut picture = movie::VideoFrame::default();
    let mut sync = store;
    for index in 0..FRAMES {
        sync.read_frame(index, &mut picture).expect("decoding");
        reference.push(picture.bytes.clone());
    }

    // `repeat`, so the position past the last frame is frame 0 again.
    let mut feed = movie::Feed::spawn(sync, true, movie.width, movie.height);
    assert_eq!(feed.len(), FRAMES);

    // One full pass, then far enough round again to have wrapped twice.
    for position in 0..(FRAMES as u64 * 2 + 5) {
        let frame = take_eventually(&mut feed, position);
        let index = (position % FRAMES as u64) as usize;
        assert_eq!(frame.position, position);
        assert_eq!(
            frame.index, index,
            "position {position} named the wrong frame"
        );
        assert_eq!(
            frame.picture.bytes, reference[index],
            "the feed's picture at position {position} is not the store's frame {index}"
        );
    }

    // Back to the start, which is what opening the menus a second time does.
    feed.restart();
    let frame = take_eventually(&mut feed, 0);
    assert_eq!(frame.index, 0, "a restarted feed begins again at frame 0");
    assert_eq!(frame.picture.bytes, reference[0]);
    assert!(feed.take_error().is_none(), "nothing failed to decode");
}

/// Waits for the worker to reach `position`, which it does in milliseconds.
///
/// `take_upto` returning `None` is the ordinary "not decoded yet" answer the
/// render thread deals with by keeping the picture it has. A test wants the
/// frame, so it waits - but with a bound, because a feed that never produces
/// anything must fail rather than hang the suite.
fn take_eventually(feed: &mut movie::Feed, position: u64) -> movie::Frame {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Some(frame) = feed.take_upto(position) {
            return frame;
        }
        if let Some(reason) = feed.take_error() {
            panic!("the feed failed at position {position}: {reason}");
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the feed never produced position {position}"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// The geometry the renderer is handed must be the movie's own.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn the_cache_geometry_matches_the_psmf_header() {
    let Some(loaded) = load() else { return };
    let movie = loaded.movie.expect("the PSP disc carries the reel");
    let frames = movie.frames.as_ref().expect("the movie transcoded");

    assert_eq!((movie.width, movie.height), (480, 272));
    assert_eq!(frames.chroma_width, movie.width.div_ceil(2));
    assert_eq!(frames.chroma_height, movie.height.div_ceil(2));
    assert_eq!(frames.luma_len, (movie.width * movie.height) as usize);
}

/// `native-video`'s decoder must decode the *whole* movie when asked to, not
/// just a truncated `Extent::Frames` extent - which is all the other tests
/// here use, since [`FRAMES`] keeps them quick. Real gameplay mostly plays
/// movies with `Extent::Whole` (the intro's own early cutoff is the
/// exception, not the rule), so a `wanted`-count bug that only shows up past
/// a truncated extent needs its own check rather than riding along on tests
/// that never ask for the full length.
#[test]
#[cfg(all(target_os = "linux", feature = "native-video"))]
#[ignore = "needs data/images/, ffmpeg, and a working GStreamer H.264 decode element"]
fn native_video_decodes_the_whole_movie_length() {
    let Some(loaded) = load_whole() else { return };
    let movie = loaded.movie.expect("the PSP disc carries the reel");
    let frames = movie.frames.as_ref().expect("the movie transcoded");

    let name = frames
        .path()
        .file_name()
        .expect("the cache file has a name")
        .to_string_lossy()
        .into_owned();
    if !name.ends_with("-gst") {
        println!(
            "skipping: {name} was not served by GStreamer - no working \
             native-video decode element on this machine, fell back to the \
             AV1 cache instead"
        );
        return;
    }

    assert_eq!(
        frames.len, movie.frame_count,
        "GStreamer decoded {} frames but the PSMF header declares {} access units - \
         a partial decode this quiet would otherwise only show up as a movie that \
         silently freezes or loops early",
        frames.len, movie.frame_count
    );
}
