//! Proves `oag_game::movie` plays Wipeout 2048's `intro.mp4` - not just parses
//! its header, but transcodes a real slice of it and decodes back a real
//! picture.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted Vita package
//! and `ffmpeg`. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(mp4_movie_ground_truth)'
//! ```
//!
//! # Why 90 frames, not the whole 2,984
//!
//! `intro.mp4` opens on a ~1.5 s fade from white - real decoded content, but a
//! single flat colour a glance would mistake for a stuck cache. The first
//! frame this project's own writeup names ("a leaf on wet tarmac", see
//! `docs/formats/2048-frontend.md`) is on screen by frame 45 and holds through
//! at least frame 105 - measured directly with `ffmpeg -vf
//! "select=not(mod(n\,15))"` on 2026-09-21. 90 frames (3.003 s) reaches well
//! into that shot without paying for the full 99.57 s / 2,984-frame lossless
//! encode `crates/video/tests/mp4_ground_truth.rs`'s own module doc measures
//! at about 80 s for the PSP intro's *smaller* picture - this file's transcode
//! runs in a few seconds instead.
//!
//! [`the_intro_cache_is_lossless`] is the `movie_ground_truth.rs`/
//! `hd_movie_ground_truth.rs` check pointed at the fifth container: what the
//! AV1 cache decodes back must be, byte for byte, what `ffmpeg` decodes
//! straight from the `.mp4`. It also writes one PNG of the decoded picture
//! under `cache_frame.png` so a human (or a
//! future automated check) can look at it rather than trust a byte count -
//! see that test's own doc for what it shows.

use std::path::{Path, PathBuf};
use std::process::Command;

use oag_game::movie;

/// How many frames to transcode - see this module's own "Why 90 frames" doc.
const FRAMES: usize = 90;

/// The decrypted Vita package's base directory, if it is there.
fn source() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/vita/PCSF00007/base")
}

/// `intro.mp4`'s own bytes, read straight out of `data.psarc` - the same
/// case-insensitive suffix lookup `crates/video/tests/mp4_ground_truth.rs`
/// uses, since this archive's manifest is not lowercased the way
/// `oag_assets::psarc::Archive`'s own doc comment describes Wipeout HD's.
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

/// Where this test's own scratch files go.
fn cache_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/cache/mp4-movie-ground-truth")
}

/// Where the PNG proof frame is written - a `data/scratch/` path, per this
/// project's convention that scratch artefacts survive under `data/`, not
/// `/tmp`.
fn scratch_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/scratch/drive-2026-09-21/mp4")
}

/// The cache must decode to exactly the frames `ffmpeg` gets from the `.mp4`
/// directly, and the picture it decodes must be a real one - not the black
/// (or, on this file, flat white) frame a stuck or silently-failed transcode
/// would produce.
#[test]
#[ignore = "needs data/extracted/vita/ and ffmpeg"]
fn the_intro_cache_is_lossless() {
    let Some(source) = source() else {
        println!("skipping: data/extracted/vita/PCSF00007/base not present");
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but the decrypted Vita package is missing"
        );
        return;
    };
    let blob = read_intro(&source);

    let mut movie = movie::open(
        &blob,
        "wo2048-intro",
        &cache_dir(),
        movie::Extent::Frames(FRAMES),
        movie::Decode::default(),
        None,
    )
    .expect("opening intro.mp4");
    let frames = movie
        .frames
        .as_mut()
        .unwrap_or_else(|| panic!("intro.mp4 did not transcode: {:?}", movie.no_picture_reason));

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

    // Frame 60 (2.0 s) sits inside the leaf-on-tarmac shot this project's own
    // writeup names - see this module's own doc. Writing it out as a PNG is
    // what turns "the bytes match" into something a person can look at.
    let proof_index = 60.min(FRAMES - 1);
    frames
        .read_frame(proof_index, &mut decoded)
        .expect("decoding the proof frame");
    let rgba = i420_to_rgba(&decoded);
    let png = oag_texture::png::encode_rgba(decoded.width, decoded.height, &rgba);
    let dir = scratch_dir();
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    let out = dir.join("cache_frame.png");
    std::fs::write(&out, png).unwrap_or_else(|e| panic!("writing {}: {e}", out.display()));
    println!("wrote frame {proof_index} to {}", out.display());
}

/// A quick-and-honest BT.601 I420 -> RGBA conversion, for writing a proof PNG
/// out of a decoded [`movie::VideoFrame`].
///
/// Nearest-neighbour chroma upsampling, not the bilinear a renderer would
/// use: this exists to make one frame visible for review, not to be a second
/// colour-conversion path anything plays through - `crates/render`'s
/// `video.wgsl` fragment shader is what actually draws a movie frame.
fn i420_to_rgba(frame: &movie::VideoFrame) -> Vec<u8> {
    let width = frame.width as usize;
    let height = frame.height as usize;
    let cw = frame.chroma_width as usize;
    let ch = frame.chroma_height as usize;
    let y_plane = &frame.bytes[..width * height];
    let u_plane = &frame.bytes[width * height..width * height + cw * ch];
    let v_plane = &frame.bytes[width * height + cw * ch..width * height + 2 * cw * ch];

    let mut out = vec![0u8; width * height * 4];
    for row in 0..height {
        for col in 0..width {
            let y = f32::from(y_plane[row * width + col]);
            let cu = col * cw / width;
            let cv = row * ch / height;
            let u = f32::from(u_plane[cv * cw + cu]) - 128.0;
            let v = f32::from(v_plane[cv * cw + cu]) - 128.0;

            let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
            let g = (y - 0.344_136 * u - 0.714_136 * v).clamp(0.0, 255.0) as u8;
            let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;

            let at = (row * width + col) * 4;
            out[at] = r;
            out[at + 1] = g;
            out[at + 2] = b;
            out[at + 3] = 255;
        }
    }
    out
}

/// Decodes `.mp4` straight to raw frames with `ffmpeg`, which is the
/// reference the cache must reproduce exactly.
///
/// The whole file goes in, container and all - the same shape
/// `hd_movie_ground_truth.rs::reference_frames` uses for `.bik` - and `-an`
/// is not passed even though `intro.mp4` carries an AAC track: a `rawvideo`
/// output takes only the video stream regardless.
fn reference_frames(blob: &[u8], frame_len: usize) -> Vec<u8> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).expect("creating the reference scratch directory");
    let source = dir.join("reference.mp4");
    std::fs::write(&source, blob).expect("writing intro.mp4");

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
