//! The PSP movies, cut into the access units the web build feeds the
//! browser's WebCodecs decoder (`movie/webcodecs.rs`).
//!
//! **`#[ignore]`d and never run in CI**: it reads a real disc image, which this
//! project does not ship. Run it with `just test-data`.
//!
//! WebCodecs is handed one picture per chunk, Annex B with no `description`,
//! so the first chunk must carry the parameter sets in band and be a picture a
//! decoder can start on. That is checked here, on the disc's own bytes,
//! rather than discovered as a decode error in a browser console.

use oag_pulse as pulse;
use oag_video::pmf;

fn nal_types(unit: &[u8]) -> Vec<u8> {
    pmf::nal_units(unit)
        .filter_map(|nal| nal.first().map(|b| b & 0x1f))
        .collect()
}

fn check(image: &str, movie: &str) {
    let Some(image) = oag_testdata::image(image) else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the archives");
    let blob = archives.read_name(movie).expect("reading the movie");
    let video = pmf::demux(&blob).expect("demuxing").video;

    let units = pmf::access_units(&video);
    assert_eq!(
        units.len(),
        pmf::frame_count(&video),
        "{movie}: one unit per frame"
    );
    assert_eq!(
        units.first().map(|u| u.start),
        Some(0),
        "{movie}: nothing dropped"
    );
    assert_eq!(
        units.last().map(|u| u.end),
        Some(video.len()),
        "{movie}: nothing dropped"
    );
    assert!(
        units.windows(2).all(|w| w[0].end == w[1].start),
        "{movie}: contiguous"
    );

    let first = nal_types(&video[units[0].clone()]);
    assert!(
        first.contains(&7),
        "{movie}: the first unit carries the SPS, has {first:?}"
    );
    assert!(
        first.contains(&8),
        "{movie}: the first unit carries the PPS, has {first:?}"
    );
    assert!(
        first.contains(&5),
        "{movie}: the first unit is an IDR picture, has {first:?}"
    );

    let codec = pmf::avc_codec(&video).expect("an SPS");
    let idr = units
        .iter()
        .filter(|u| nal_types(&video[(*u).clone()]).contains(&5))
        .count();
    println!("{movie}: {} units, {idr} IDR, codec {codec}", units.len());
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_intro_cuts_into_one_unit_per_frame_starting_on_an_idr() {
    check("data/images/pulse-psp-eu.chd", pulse::names::INTRO_MOVIE);
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_backdrop_cuts_into_one_unit_per_frame_starting_on_an_idr() {
    check("data/images/pulse-psp-eu.chd", pulse::names::BACKDROP_MOVIE);
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_devpub_reel_cuts_into_one_unit_per_frame_starting_on_an_idr() {
    check("data/images/pulse-psp-eu.chd", pulse::names::DEVPUB_REEL);
}
