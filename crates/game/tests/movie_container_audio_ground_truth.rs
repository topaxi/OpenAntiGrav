//! Proves the container-hosted audio route actually decodes: Wipeout 2048's
//! `intro.mp4` AAC track and one Wipeout HD/Fury `.bik` reel's Bink Audio
//! track, both through `MovieAudio::decode`, not just parsed off the header.
//!
//! **`#[ignore]`d and never run in CI.** It needs `ffmpeg` and this project's
//! own game content - the decrypted Vita package for the first, a disc image
//! for the second. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(movie_container_audio_ground_truth)'
//! ```
//!
//! # Why `--no-video`'s own path, not the full transcode
//!
//! Both movies open with [`movie::Decode::no_video`] set. `bink::open` and
//! `mp4::open` compute [`movie::Movie::audio`] *before* branching on that flag
//! - see their own comments on why - so this exercises exactly the route a
//! real boot takes without also paying for `intro.mp4`'s multi-minute lossless
//! AV1 encode or the logo reel's ~98 s one
//! (`docs/formats/mp4.md`/`docs/formats/bik.md` both measure the full-length
//! cost). The audio-only decode this test actually runs is seconds.

use std::path::{Path, PathBuf};

use oag_game::movie;

/// Where this test's own scratch files go - a directory of its own so a run
/// here never collides with `mp4_movie_ground_truth.rs`'s or
/// `hd_movie_ground_truth.rs`'s video caches.
fn cache_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/cache/movie-container-audio-ground-truth")
}

/// The decrypted Vita package's base directory, if it is there.
fn vita_source() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/vita/PCSF00007/base")
}

/// `intro.mp4`'s own bytes, read straight out of `data.psarc`.
fn read_intro(source: &Path) -> Vec<u8> {
    let psarc_path = source.join("PSP2/data.psarc");
    let mut archive = oag_assets::psarc::Archive::open_file(&psarc_path)
        .unwrap_or_else(|e| panic!("opening {}: {e}", psarc_path.display()));
    let path = archive
        .paths()
        .iter()
        .find(|p| p.to_ascii_lowercase().ends_with("intro.mp4"))
        .unwrap_or_else(|| panic!("no path in {} ends with intro.mp4", psarc_path.display()))
        .clone();
    archive
        .read_path(&path)
        .unwrap_or_else(|e| panic!("reading {path}: {e}"))
}

/// `intro.mp4`'s own audio track has an exact duration this project can
/// compute with no decode at all - `frame_count * frame_delta / sample_rate`,
/// per `oag_video::mp4::AudioTrack::frame_delta`'s own doc. What `ffmpeg`
/// actually decodes must land within a tenth of a second of it.
#[test]
#[ignore = "needs data/extracted/vita/ and ffmpeg"]
fn intro_mp4_audio_decodes_within_a_tenth_of_a_second_of_its_header() {
    let Some(source) = vita_source() else {
        println!("skipping: data/extracted/vita/PCSF00007/base not present");
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but the decrypted Vita package is missing"
        );
        return;
    };
    let blob = read_intro(&source);
    let header = oag_video::mp4::parse(&blob).expect("parsing intro.mp4");
    let audio_header = header.audio.expect("intro.mp4 carries an audio track");
    let expected_seconds = audio_header.frame_count as f64 * f64::from(audio_header.frame_delta)
        / f64::from(audio_header.sample_rate);

    let movie = movie::open(
        &blob,
        "container-audio-intro",
        &cache_dir(),
        movie::Extent::Whole,
        movie::Decode {
            no_video: true,
            ..movie::Decode::default()
        },
        None,
    )
    .expect("opening intro.mp4 without decoding video");
    assert!(movie.frames.is_none(), "--no-video should skip the picture");

    let audio = movie.audio.expect("intro.mp4 should carry a MovieAudio");
    assert!(
        (audio.seconds() - expected_seconds).abs() < 0.001,
        "MovieAudio::seconds should be exact, not approximate, for MP4"
    );

    let pcm = audio
        .decode(&cache_dir())
        .unwrap_or_else(|e| panic!("decoding intro.mp4's audio track: {e:#}"));
    assert_eq!(pcm.channels, 2);
    assert_eq!(pcm.sample_rate, 48_000);
    assert!(
        !pcm.samples.is_empty(),
        "the decode produced no samples at all"
    );
    assert!(
        pcm.samples.iter().any(|&s| s != 0),
        "the decode is not silence - a movie with a real soundtrack should not decode to all zeros"
    );

    let decoded_seconds =
        pcm.samples.len() as f64 / f64::from(pcm.channels) / f64::from(pcm.sample_rate);
    assert!(
        (decoded_seconds - expected_seconds).abs() < 0.1,
        "decoded {decoded_seconds:.3}s, header says {expected_seconds:.3}s"
    );
}

/// The decrypted HD/Fury image, if it is there.
fn hd_image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// The archive and path the logo reel lives at - the same pair
/// `hd_movie_ground_truth.rs` pins, for the same reason: `DATA02`'s copy is
/// the one `oag_hd::frontend::names::STUDIO_LOGO_MOVIE` names, not a claim
/// about which of the two families a PS3 actually plays.
const LOGO_ARCHIVE: &str = "DATA02.PSARC";
const LOGO_PATH: &str = "/data/fe/images/studioliverpool.bik";

fn read_logo_reel(image: &Path) -> Vec<u8> {
    let spec = format!("{}:PS3_GAME/USRDIR/{LOGO_ARCHIVE}", image.display());
    let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    archive.read_path(LOGO_PATH).expect("reading the logo reel")
}

/// The logo reel's four Bink Audio tracks have no per-track sample count in
/// the container header at all - `ContainerTrack::seconds`'s own doc explains
/// why `MovieAudio::seconds` falls back to the *video's* own duration for
/// Bink. What `ffmpeg` actually decodes off track 0 must still land close to
/// it: the four tracks are a synced dub of the same reel, not four different
/// lengths.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn the_logo_reels_track_zero_decodes_close_to_the_reels_own_duration() {
    let Some(image) = hd_image() else {
        println!("skipping: data/images/hdfury-ps3-eu-dec.iso not present");
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but the HD/Fury image is missing"
        );
        return;
    };
    let blob = read_logo_reel(&image);
    let header = oag_video::bik::parse(&blob).expect("parsing the reel");
    assert_eq!(header.audio.len(), 4, "the reel should carry four tracks");
    let video_seconds = header.seconds();

    let movie = movie::open(
        &blob,
        "container-audio-hd-logo",
        &cache_dir(),
        movie::Extent::Whole,
        movie::Decode {
            no_video: true,
            ..movie::Decode::default()
        },
        None,
    )
    .expect("opening the reel without decoding video");
    assert!(movie.frames.is_none(), "--no-video should skip the picture");

    let audio = movie.audio.expect("the reel should carry a MovieAudio");
    // `codec_clause` is crate-private (it composes `boot::load_movie_sound`'s
    // report line, nothing this test crate reaches directly), so the same
    // "a choice, not a measurement" fact is checked through `Debug` instead.
    let debug = format!("{audio:?}");
    assert!(
        debug.contains("chosen, not measured"),
        "a four-track file should say its pick is a choice: {debug}"
    );

    let pcm = audio
        .decode(&cache_dir())
        .unwrap_or_else(|e| panic!("decoding the reel's audio track: {e:#}"));
    assert_eq!(pcm.channels, 2);
    assert_eq!(pcm.sample_rate, 48_000);
    assert!(
        pcm.samples.iter().any(|&s| s != 0),
        "the decode is not silence"
    );

    let decoded_seconds =
        pcm.samples.len() as f64 / f64::from(pcm.channels) / f64::from(pcm.sample_rate);
    assert!(
        (decoded_seconds - video_seconds).abs() < 0.1,
        "decoded {decoded_seconds:.3}s, the reel's own video duration is {video_seconds:.3}s"
    );
}
