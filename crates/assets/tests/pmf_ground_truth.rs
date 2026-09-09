//! Validates the [`pmf`](oag_video::pmf) parser and demuxer against every
//! movie on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! It lives in `oag-assets` rather than in `oag-formats` because reading a WAD
//! out of a disc image is what this crate is for, and `oag-formats` must not
//! depend on it.
//!
//! # What this is for
//!
//! Three independent arithmetic checks have to agree, and none of them can be
//! satisfied by a parser that is reading the layout wrong:
//!
//! 1. `stream_offset + stream_size` equals the entry's length, exactly.
//! 2. The program-stream walk accounts for **every byte**: zero strays.
//! 3. The H.264 access-unit count matches the frame count the header's own
//!    presentation timestamps imply at 30000/1001 Hz, to within one frame.
//!
//! The third is the strong one, because the two numbers come from opposite ends
//! of the file and are produced by completely different code.

use std::path::PathBuf;

use oag_assets::Archive;
use oag_pulse as pulse;
use oag_video::pmf;

/// How many `.PMF` blobs the USA disc holds. Fewer means the scan missed some.
const EXPECTED_MOVIES: usize = 17;

/// Bytes to peek when looking for the `PSMF` magic.
const PEEK: u64 = 8;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn open_data() -> Option<Archive> {
    let image = image()?;
    let spec = format!("{}:{}", image.display(), pulse::archives::DATA);
    Some(Archive::open(&spec).expect("opening Data.wad"))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_movie_on_the_disc_parses_and_demuxes() {
    let Some(mut data) = open_data() else { return };

    // Find the movies by their magic rather than by name: most of their names
    // are not recovered, and a scan cannot be fooled by a wrong guess.
    let candidates: Vec<usize> = (0..data.directory().entries.len())
        .filter(|&index| {
            data.peek(index, PEEK)
                .is_ok_and(|head| head.starts_with(b"PSMF"))
        })
        .collect();

    assert_eq!(
        candidates.len(),
        EXPECTED_MOVIES,
        "found {} PSMF blobs, expected {EXPECTED_MOVIES}",
        candidates.len()
    );

    let mut checked = 0usize;
    for index in candidates {
        let declared = data.entry_len(index).expect("entry length");
        let blob = data.read(index).expect("reading a movie");
        assert_eq!(blob.len() as u32, declared);

        let header = pmf::Header::parse(&blob)
            .unwrap_or_else(|e| panic!("entry {index}: header did not parse: {e}"));

        // 1. The header's own arithmetic accounts for the whole file.
        assert_eq!(
            header.total_len(),
            blob.len() as u64,
            "entry {index}: stream offset plus size must be the file length"
        );

        let video = header
            .video
            .unwrap_or_else(|| panic!("entry {index}: no video stream"));
        assert!(video.width > 0 && video.height > 0);
        assert_eq!(video.width % 16, 0, "width is stored in sixteenths");
        assert_eq!(video.height % 16, 0, "height is stored in sixteenths");

        // 2. Every byte of the program stream belongs to a pack or a packet.
        let demuxed = pmf::demux(&blob).expect("demuxing");
        assert_eq!(
            demuxed.stray_bytes, 0,
            "entry {index}: {} byte(s) unaccounted for",
            demuxed.stray_bytes
        );
        assert!(!demuxed.video.is_empty(), "entry {index}: no video payload");

        // The elementary stream must start with a NAL start code and its first
        // parameter set must be H.264.
        assert!(
            demuxed.video.starts_with(&[0, 0, 0, 1]) || demuxed.video.starts_with(&[0, 0, 1]),
            "entry {index}: video does not start with an Annex B start code"
        );

        // 3. Access units and presentation duration are two independent
        // measurements of the frame count.
        let counted = pmf::frame_count(&demuxed.video);
        let implied = header.expected_frame_count() as usize;
        assert!(
            counted.abs_diff(implied) <= 1,
            "entry {index}: counted {counted} access units, duration implies {implied}"
        );

        // An audio stream, when declared, must produce payloads.
        if header.audio.is_some() {
            assert!(
                !demuxed.audio.is_empty(),
                "entry {index}: declares audio but demuxed none"
            );
        } else {
            assert!(
                demuxed.audio.is_empty(),
                "entry {index}: demuxed audio it does not declare"
            );
        }

        checked += 1;
    }

    assert_eq!(checked, EXPECTED_MOVIES);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_intro_movie_is_where_the_front_end_xml_says_it_is() {
    let Some(mut data) = open_data() else { return };

    // The name is not in the executable. It comes from the `src` attribute of
    // the `Movie` widget in the front-end root XML plus `.PMF`, and this is the
    // assertion that the assembled name is right.
    let index = data
        .index_of_name(pulse::names::INTRO_MOVIE)
        .unwrap_or_else(|| panic!("{} is not in Data.wad", pulse::names::INTRO_MOVIE));

    let blob = data.read(index).expect("reading the intro");
    let header = pmf::Header::parse(&blob).expect("parsing the intro header");

    assert_eq!(&header.version, b"0014");
    assert_eq!(header.stream_offset, pmf::HEADER_LEN as u32);

    let video = header.video.expect("video stream");
    assert_eq!((video.width, video.height), (480, 272));

    let audio = header.audio.expect("audio stream");
    assert_eq!(audio.channels, 2);
    assert_eq!(audio.frequency_hz(), Some(44_100));

    let demuxed = pmf::demux(&blob).expect("demuxing the intro");
    assert_eq!(demuxed.stray_bytes, 0);
    assert_eq!(
        pmf::frame_count(&demuxed.video),
        1200,
        "the USA intro is 1200 frames, 40.04 s at 30000/1001 Hz"
    );

    // The backdrop is the other name the XML yields, and it has no audio.
    let backdrop = data
        .read_name(pulse::names::BACKDROP_MOVIE)
        .expect("reading the backdrop");
    let header = pmf::Header::parse(&backdrop).expect("parsing the backdrop header");
    assert_eq!(header.stream_count, 1, "the backdrop is silent");
    assert!(header.audio.is_none());
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_two_hundred_and_sixty_frame_reel_exists() {
    // The intro state's frame counters are 144, 231 and 260. Movies of exactly
    // 260 frames existing on the disc is what turns those numbers from
    // "constants in a decompiled function" into frame numbers of a real reel.
    // See docs/architecture/frontend-boot.md.
    let Some(mut data) = open_data() else { return };

    let mut lengths = Vec::new();
    for index in 0..data.directory().entries.len() {
        if !data
            .peek(index, PEEK)
            .is_ok_and(|head| head.starts_with(b"PSMF"))
        {
            continue;
        }
        let blob = data.read(index).expect("reading a movie");
        if let Ok(header) = pmf::Header::parse(&blob)
            && let Ok(demuxed) = pmf::demux(&blob)
        {
            lengths.push((index, pmf::frame_count(&demuxed.video), header));
        }
    }

    let matching: Vec<usize> = lengths
        .iter()
        .filter(|(_, frames, _)| *frames == 260)
        .map(|(index, _, _)| *index)
        .collect();

    assert!(
        matching.len() >= 3,
        "expected at least three 260-frame reels, found {matching:?} \
         among {:?}",
        lengths.iter().map(|(i, f, _)| (*i, *f)).collect::<Vec<_>>()
    );

    // And they are these three. The reel the intro state plays is addressed by
    // hash because no name for it has been recovered, so a hash that quietly
    // stopped matching would leave `--movie`'s default pointing at nothing.
    let mut hashes: Vec<u32> = matching
        .iter()
        .map(|&index| data.directory().entries[index].name_hash)
        .collect();
    hashes.sort_unstable();
    let mut expected = vec![
        pulse::hashes::DEVPUB_REEL_SCEE,
        pulse::hashes::DEVPUB_REEL_SCEI,
        pulse::hashes::DEVPUB_REEL_SCEA,
    ];
    expected.sort_unstable();
    assert_eq!(
        hashes,
        expected,
        "the 260-frame reels are the three dev/pub cuts; got {:?}",
        hashes
            .iter()
            .map(|h| format!("{h:08x}"))
            .collect::<Vec<_>>()
    );

    for index in matching {
        let header = &lengths
            .iter()
            .find(|(i, _, _)| *i == index)
            .expect("just collected")
            .2;
        let video = header.video.expect("video stream");
        assert_eq!(
            (video.width, video.height),
            (480, 272),
            "a dev/pub reel is full screen"
        );
    }
}
