//! Validates the [`ipf`](oag_formats::ipf) container and its decode route
//! against both `.IPF` files on the real PS2 disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship; the decode half also needs `ffmpeg` on `PATH`. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! The container reading rests on one arithmetic identity and one measurement,
//! and a wrong reading fails both:
//!
//! 1. `32 + frames * stride` is the file length, **exactly** - which is what
//!    separates a 32-byte file header with 64-byte slot headers from the
//!    96-byte flat header the same bytes also admit.
//! 2. Every slot's declared payload length equals the extent of its non-zero
//!    bytes, on **every** frame of both files. A field that is not the length
//!    does not do that 495 times.
//!
//! The decode half then checks that `ffmpeg`'s own IPU demuxer, splitting the
//! concatenated bitstream by the `00 00 01 B0` markers inside it and knowing
//! nothing about the slots, lands on exactly the boundaries the slot headers
//! declared - and that the frames come out at the size the header promised.
//!
//! See `docs/formats/ipf.md`.

use std::path::{Path, PathBuf};
use std::process::Command;

use oag_formats::ipf;
use oag_game::movie;

/// Every `.IPF` the PS2 disc ships, with what its header must declare.
///
/// Both are the same nine-second loop: 225 frames at the PAL 25 Hz, 270 at the
/// NTSC 30000/1001. See `docs/formats/ipf.md`.
const FILES: [(&str, u32, u32, usize); 2] = [
    ("DATA/MOVIES/BG512.IPF", 512, 512, 225),
    ("DATA/MOVIES/BG640.IPF", 640, 448, 270),
];

/// Frames the decode half converts. Small enough that the lossless AV1 encode
/// stays quick; the container half covers every frame of both files.
const DECODE_FRAMES: usize = 8;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-ps2-eu.chd");

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

fn read(source: &Path, path: &str) -> Vec<u8> {
    let (found, blob) = oag_assets::read_loose_file(&source.display().to_string(), &[path])
        .expect("reading the disc")
        .unwrap_or_else(|| panic!("{path} is not on the disc"));
    assert!(
        found
            .to_ascii_uppercase()
            .ends_with(&path.to_ascii_uppercase()),
        "asked for {path}, got {found}"
    );
    blob
}

#[test]
#[ignore = "needs data/images/"]
fn both_ipf_files_close_to_the_byte() {
    let Some(image) = image() else { return };

    for (path, width, height, frames) in FILES {
        let blob = read(&image, path);
        let parsed = ipf::parse(&blob).unwrap_or_else(|e| panic!("{path}: {e}"));
        let header = parsed.header;

        assert_eq!(header.width, width, "{path}: width");
        assert_eq!(header.height, height, "{path}: height");
        assert_eq!(header.frame_count, frames, "{path}: frame count");
        assert_eq!(header.alignment, 16, "{path}: the field at +0x10");
        assert_eq!(parsed.len(), frames, "{path}: frames cut out");

        // 1. The file is exactly its header plus its slots. Nothing over, and
        //    - the part a 96-byte-header reading gets wrong - nothing under.
        assert_eq!(
            blob.len(),
            ipf::HEADER_LEN + frames * header.frame_stride,
            "{path}: the slots must fill the file exactly"
        );

        // The stride is the worst frame rounded up to the alignment, which is
        // what makes a seek a multiply.
        let longest = parsed.frames.iter().map(|f| f.len()).max().unwrap_or(0);
        assert_eq!(
            header.frame_stride,
            (ipf::FRAME_HEADER_LEN + longest).next_multiple_of(header.alignment as usize),
            "{path}: stride is not the longest payload padded to the alignment"
        );

        for (index, frame) in parsed.frames.iter().enumerate() {
            // 2. The declared length is the non-zero extent, every time. The
            //    slot is zero-filled past its payload, so a length field read
            //    one byte or one field out lands in the padding or cuts the
            //    stream short, and this says so.
            assert!(
                frame.last().is_some_and(|&b| b != 0),
                "{path}: frame {index} ends on a zero, so the length is short"
            );
            let slot = ipf::HEADER_LEN + index * header.frame_stride;
            let padding =
                &blob[slot + ipf::FRAME_HEADER_LEN + frame.len()..slot + header.frame_stride];
            assert!(
                padding.iter().all(|&b| b == 0),
                "{path}: frame {index} has non-zero bytes past its declared length"
            );

            // Every frame is one IPU picture: a four-byte picture header and
            // the end marker the bitstream closes on.
            assert!(frame.len() > 8, "{path}: frame {index} is too short");
            assert_eq!(
                &frame[frame.len() - 4..],
                &[0x00, 0x00, 0x01, 0xb0],
                "{path}: frame {index} does not end on the IPU end marker"
            );
        }
    }
}

#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn both_ipf_files_decode_through_the_movie_cache() {
    let Some(image) = image() else { return };
    if !have_ffmpeg() {
        return;
    }
    let cache = std::env::temp_dir().join("oag-ipf-ground-truth");

    for (path, width, height, frames) in FILES {
        let blob = read(&image, path);
        let key = format!("{}-{}", path.replace('/', "_"), blob.len());

        // Without a transcode first: the geometry and the count come straight
        // off the container, so they must be exact even with no decoder.
        let described = movie::open(
            &blob,
            &key,
            &cache,
            movie::Extent::Whole,
            movie::Decode {
                no_video: true,
                ..movie::Decode::default()
            },
            None,
        )
        .unwrap_or_else(|e| panic!("{path}: {e:#}"));
        assert_eq!(described.width, width, "{path}: width");
        assert_eq!(described.height, height, "{path}: height");
        assert_eq!(described.frame_count, frames, "{path}: frame count");
        assert_eq!(described.display_aspect, (4, 3), "{path}: display aspect");
        let expected_rate = if width == 640 {
            (30_000, 1001)
        } else {
            (25, 1)
        };
        assert_eq!(described.frame_rate, expected_rate, "{path}: frame rate");

        // Both cuts must run the same nine seconds; a swapped PAL/NTSC pairing
        // would put one at 10.8 s and the other at 7.5 s.
        let seconds = frames as f64 * described.frame_rate.1 as f64 / described.frame_rate.0 as f64;
        assert!(
            (seconds - 9.0).abs() < 0.05,
            "{path}: {seconds:.3} s, expected a nine-second loop"
        );

        let movie = movie::open(
            &blob,
            &key,
            &cache,
            movie::Extent::Frames(DECODE_FRAMES),
            movie::Decode::default(),
            None,
        )
        .unwrap_or_else(|e| panic!("{path}: {e:#}"));
        let store = movie.frames.unwrap_or_else(|| {
            panic!(
                "{path} did not transcode: {}",
                movie.no_picture_reason.unwrap_or_default()
            )
        });
        assert_eq!(store.len, DECODE_FRAMES, "{path}: frames cached");
        assert_eq!(
            store.luma_len,
            (width * height) as usize,
            "{path}: the decoded picture is not the size the container declares"
        );
    }
}

/// The name the PS2's own front-end XML uses must reach the file on the disc.
///
/// Two things this build had wrong meet here: the widget's `src` already
/// carries its extension on PS2, so appending `.PMF` asked for
/// `Data\Movies\Backdrop.ipf.PMF`; and the name is not a filename at all -
/// `Movie_ResolveSourcePath` maps it onto the region's cut. See
/// `docs/ghidra/functions/ps2-pulse-eu/movie-paths.md`.
#[test]
#[ignore = "needs data/images/"]
fn the_front_ends_own_backdrop_name_resolves_to_the_pal_cut() {
    let Some(image) = image() else { return };

    let options = oag_game::boot::Options {
        // No saved language: these boot a fresh install every time.
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_game::frontend::Leg::LogoFmv,
        movie: Some(r"Data\Movies\Backdrop.ipf".to_string()),
        cache: std::env::temp_dir().join("oag-ipf-ground-truth"),
        audio_cache: oag_game::boot::default_audio_cache_dir(),
        extent: movie::Extent::Whole,
        // The resolution is what is under test, not the transcode.
        no_video: true,
        refresh_video: false,
    };

    let loaded = oag_game::boot::load(&options).expect("booting the PS2 front end");
    for line in &loaded.report {
        println!("{line}");
    }

    let backdrop = loaded.movie.expect("the backdrop must resolve");
    assert_eq!((backdrop.width, backdrop.height), (512, 512));
    assert_eq!(backdrop.frame_count, 225);
    assert_eq!(backdrop.frame_rate, (25, 1));
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("BG512.IPF") && line.contains("IPU video")),
        "the report does not name the file the backdrop came from"
    );
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains(r"plays Data\Movies\Backdrop.ipf")),
        "the FE Screen widget's own name is not reported as the XML spells it"
    );
    assert!(
        !loaded.report.iter().any(|line| line.contains(".ipf.PMF")),
        "a PS2 src must not have .PMF appended to it"
    );
}

/// `ffmpeg`'s IPU demuxer, which knows nothing about the slot framing, must cut
/// the concatenated bitstream at exactly the boundaries the slot headers
/// declare.
///
/// That is the independent confirmation that the `u32` at the head of each slot
/// is the payload length: two unrelated readings of the same file agreeing on
/// 225 boundaries.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn the_slot_lengths_are_the_bitstreams_own_frame_boundaries() {
    let Some(image) = image() else { return };
    if !have_ffmpeg() {
        return;
    }
    let probe = Command::new("ffprobe")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if !probe.is_ok_and(|s| s.success()) {
        println!("skipping: ffprobe is not on PATH");
        return;
    }

    let dir = std::env::temp_dir().join("oag-ipf-ground-truth");
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");

    for (path, ..) in FILES {
        let blob = read(&image, path);
        let parsed = ipf::parse(&blob).unwrap_or_else(|e| panic!("{path}: {e}"));

        // The same wrapper `movie::transcode_ipu` writes: ffmpeg's `ipum`
        // header, then the frames end to end with the slots removed.
        let bitstream = parsed.bitstream();
        let mut wrapped = Vec::with_capacity(16 + bitstream.len());
        wrapped.extend_from_slice(b"ipum");
        wrapped.extend_from_slice(&(bitstream.len() as u32).to_le_bytes());
        wrapped.extend_from_slice(&(parsed.header.width as u16).to_le_bytes());
        wrapped.extend_from_slice(&(parsed.header.height as u16).to_le_bytes());
        wrapped.extend_from_slice(&(parsed.len() as u32).to_le_bytes());
        wrapped.extend_from_slice(&bitstream);

        let source = dir.join(format!("{}.ipu", path.replace('/', "_")));
        std::fs::write(&source, &wrapped).expect("writing the ipu");

        let output = Command::new("ffprobe")
            .args(["-v", "error"])
            .args(["-select_streams", "v:0"])
            .args(["-show_entries", "packet=size"])
            .args(["-of", "csv=p=0"])
            .arg(&source)
            .output()
            .expect("running ffprobe");
        assert!(output.status.success(), "{path}: ffprobe failed");

        let sizes: Vec<usize> = String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .map(|n| n.parse().expect("a packet size"))
            .collect();
        let declared: Vec<usize> = parsed.frames.iter().map(|f| f.len()).collect();
        assert_eq!(
            sizes, declared,
            "{path}: ffmpeg's frame boundaries differ from the slot lengths"
        );
    }
}
