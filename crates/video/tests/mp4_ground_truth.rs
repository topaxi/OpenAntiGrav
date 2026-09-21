//! Validates `oag_video::mp4` against every real `.mp4` Wipeout 2048 ships.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted Vita package,
//! which this project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-video --run-ignored all \
//!     -E 'binary(mp4_ground_truth)'
//! ```
//!
//! # Container only - no transcode here
//!
//! Every check below is `parse`/`top_level_boxes` against real bytes: no
//! `ffmpeg`, no AV1 cache, no picture. The transcode - proving the cache
//! decodes to an actual frame rather than a black one - is
//! `crates/game/tests/mp4_movie_ground_truth.rs`, kept apart so this file's
//! 26-file sweep stays seconds rather than the minutes a 47 MB lossless AV1
//! encode costs.
//!
//! [`every_mp4_on_the_disc_reads_its_own_header`] is the survey: 26 files,
//! three arithmetic invariants each, mirroring what
//! `crates/game/tests/hd_movie_ground_truth.rs` does for Wipeout HD's 37
//! `.bik` files. [`intro_is_960x544_at_30000_1001_with_stereo_aac`] pins the
//! one file with an audio track and the numbers `ffprobe` independently
//! measured before this module existed - see `docs/formats/mp4.md`.

use std::path::PathBuf;
use std::time::Instant;

use oag_video::mp4;

/// The decrypted Vita package's base directory, if it is there.
fn source() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/vita/PCSF00007/base")
}

fn open_data_psarc(source: &std::path::Path) -> oag_assets::psarc::Archive {
    let psarc_path = source.join("PSP2/data.psarc");
    oag_assets::psarc::Archive::open_file(&psarc_path)
        .unwrap_or_else(|e| panic!("opening {}: {e}", psarc_path.display()))
}

/// Every `data/Videos/*.mp4` path the archive's own manifest names, sorted.
fn every_mp4_path(archive: &oag_assets::psarc::Archive) -> Vec<String> {
    let mut paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".mp4"))
        .cloned()
        .collect();
    paths.sort();
    paths
}

/// The one path among `archive`'s own manifest whose lowercase form ends
/// with `suffix`.
///
/// Case-insensitive and by suffix rather than a hardcoded full path: this
/// archive's manifest is not lowercased the way `oag_assets::psarc::Archive`'s
/// own doc comment describes Wipeout HD's PSARCs (`python3 scripts/psarc.py
/// list` reports `data/Videos/intro.mp4`, mixed case, no leading slash) - so a
/// test that assumed HD's convention here would be checking a path that does
/// not exist rather than the real one.
fn find_path(archive: &oag_assets::psarc::Archive, suffix: &str) -> String {
    let suffix = suffix.to_ascii_lowercase();
    let matches: Vec<&String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(&suffix))
        .collect();
    match matches.as_slice() {
        [only] => (*only).clone(),
        [] => panic!("no path in the archive ends with {suffix}"),
        many => panic!("{} paths end with {suffix}: {many:?}", many.len()),
    }
}

/// The disc carries 26 `.mp4` entries and every one of them parses.
///
/// **Three independent invariants per file**, the same shape
/// `docs/formats/bik.md` scored 92 on HD's 37 `.bik` files:
///
/// 1. every top-level box's declared size sums to the file's own length -
///    [`mp4::top_level_boxes`] cannot return successfully otherwise;
/// 2. the video track's `stsz` sample count equals its `stts` sum;
/// 3. the video track's `mdhd` duration equals `frame_count * frame_delta`.
///
/// The third is the load-bearing one: it agrees only when `mdhd`, `stts` and
/// `stsz` - three different boxes - were all read at the right offsets.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_mp4_on_the_disc_reads_its_own_header() {
    let Some(source) = source() else {
        println!("skipping: data/extracted/vita/PCSF00007/base not present");
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but the decrypted Vita package is missing"
        );
        return;
    };
    let mut archive = open_data_psarc(&source);
    let paths = every_mp4_path(&archive);
    assert_eq!(paths.len(), 26, "shipped .mp4 entries: {paths:?}");

    let started = Instant::now();
    let mut with_audio = 0;
    for path in &paths {
        let blob = archive
            .read_path(path)
            .unwrap_or_else(|e| panic!("reading {path}: {e}"));

        // Invariant 1: every top-level box's size sums to the file length -
        // `top_level_boxes` cannot succeed otherwise, and `moov` is always
        // the last of them on all 26 files.
        let boxes = mp4::top_level_boxes(&blob)
            .unwrap_or_else(|e| panic!("{path}: walking the top level: {e}"));
        let sum: u64 = boxes.iter().map(|b| b.size).sum();
        assert_eq!(sum, blob.len() as u64, "{path}: box sizes vs file length");
        assert_eq!(
            boxes.last().map(|b| &b.kind),
            Some(b"moov"),
            "{path}: moov is not the last top-level box"
        );

        let header =
            mp4::parse(&blob).unwrap_or_else(|e| panic!("{path}: parsing the header: {e}"));

        assert!(
            header.width > 0 && header.height > 0 && header.frame_count > 0,
            "{path} declares nothing to play"
        );

        // Invariant 2: two different boxes counting the same samples.
        assert_eq!(
            header.frame_count, header.stts_sample_count,
            "{path}: stsz and stts disagree on the video track's sample count"
        );

        // Invariant 3: mdhd's own duration against stts's own delta,
        // multiplied out independently of Header::frame_rate's reduction.
        assert_eq!(
            header.duration,
            header.frame_count as u64 * u64::from(header.frame_delta),
            "{path}: mdhd duration does not equal frame_count * frame_delta"
        );

        if let Some(audio) = &header.audio {
            with_audio += 1;
            assert_eq!(&audio.codec, b"mp4a", "{path}'s audio track");
            assert!(audio.sample_rate > 0 && audio.channel_count > 0, "{path}");
            assert!(
                audio.frame_delta > 0,
                "{path}'s audio track declares no stts delta"
            );
        }
    }
    let elapsed = started.elapsed();
    println!("parsed 26 headers in {elapsed:?}");

    // Only `intro.mp4` carries an audio track - every shipunlocks clip and
    // every recap/best-bits reel is silent by construction.
    assert_eq!(with_audio, 1, "only intro.mp4 should carry an audio track");
}

/// `intro.mp4` is 960x544 at 30000/1001, 2,984 video frames, with a stereo
/// 48 kHz AAC track of 4,666 frames - independently measured by `ffprobe`
/// before this module existed. See `docs/formats/mp4.md`.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn intro_is_960x544_at_30000_1001_with_stereo_aac() {
    let Some(source) = source() else {
        println!("skipping: data/extracted/vita/PCSF00007/base not present");
        return;
    };
    let mut archive = open_data_psarc(&source);
    let path = find_path(&archive, "intro.mp4");
    let blob = archive
        .read_path(&path)
        .unwrap_or_else(|e| panic!("reading {path}: {e}"));

    let boxes = mp4::top_level_boxes(&blob).expect("walking intro.mp4's top level");
    let kinds: Vec<[u8; 4]> = boxes.iter().map(|b| b.kind).collect();
    assert_eq!(
        kinds,
        [*b"ftyp", *b"free", *b"mdat", *b"moov"],
        "intro.mp4's own box order"
    );

    let header = mp4::parse(&blob).expect("parsing intro.mp4");
    assert_eq!((header.width, header.height), (960, 544));
    assert_eq!(header.frame_count, 2984);
    assert_eq!(header.stts_sample_count, 2984);
    assert_eq!(header.timescale, 30_000);
    assert_eq!(header.frame_delta, 1_001);
    assert_eq!(header.duration, 2_986_984);
    assert_eq!(header.frame_rate, (30_000, 1_001));
    assert!(
        (header.seconds() - 99.566).abs() < 0.001,
        "{}",
        header.seconds()
    );

    let audio = header.audio.expect("intro.mp4 carries an audio track");
    assert_eq!(&audio.codec, b"mp4a");
    assert_eq!(audio.sample_rate, 48_000);
    assert_eq!(audio.channel_count, 2);
    assert_eq!(audio.frame_count, 4_666);
    // 1,024 samples per AAC frame at this track's own 48,000 Hz `mdhd`
    // timescale - see `AudioTrack::frame_delta`'s own doc for why a tick of
    // this particular timescale is a PCM sample.
    assert_eq!(audio.frame_delta, 1_024);
    assert!(
        (audio.frame_count as f64 * f64::from(audio.frame_delta) / f64::from(audio.sample_rate)
            - 99.541)
            .abs()
            < 0.001,
        "the audio track's own exact duration"
    );
}

/// `bb2048Zone8.mp4` is the one file whose `free` box lands after `mdat`
/// rather than before it - the box-order finding `oag_video::mp4`'s own
/// module doc names, pinned here so a future change to
/// [`mp4::top_level_boxes`] cannot quietly start assuming a fixed sequence.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn bb2048zone8_swaps_free_and_mdat() {
    let Some(source) = source() else {
        println!("skipping: data/extracted/vita/PCSF00007/base not present");
        return;
    };
    let mut archive = open_data_psarc(&source);
    let path = find_path(&archive, "bb2048zone8.mp4");
    let blob = archive
        .read_path(&path)
        .unwrap_or_else(|e| panic!("reading {path}: {e}"));

    let boxes = mp4::top_level_boxes(&blob).expect("walking its top level");
    let kinds: Vec<[u8; 4]> = boxes.iter().map(|b| b.kind).collect();
    assert_eq!(kinds, [*b"ftyp", *b"mdat", *b"free", *b"moov"]);

    // Its own timescale/delta is a different rational close to 29.97 Hz, not
    // a rounding of intro.mp4's 30000/1001 - see `oag_video::mp4::Header`.
    let header = mp4::parse(&blob).expect("parsing it");
    assert_eq!(header.frame_rate, (2_997, 100));
}
