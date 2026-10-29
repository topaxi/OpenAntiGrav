//! `Track Creation`'s `Distance(m)` row against the disc. The original prints the circuit's
//! authored `<Physical Length>` (`TrackStats_ParseElement` `0x088c46f8`), which
//! `oag_raceplay::authored_length` reads and must equal exactly. The measured lap
//! `oag_raceplay::circuit_length` sums stays the fallback for a circuit whose `stats.xml` is
//! absent, and is held to within three per cent of the screen.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.

use std::path::PathBuf;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// The three circuits `Track Creation` offers on a fresh profile, with the
/// `Distance(m)` each shows - read off the 2026-09-09 PPSSPP capture in
/// `docs/ui/selection-screens.md`.
const SHOWN: [(&str, f32); 3] = [
    (r"Data\Environments\16_Track\track.vex", 5178.0),
    (r"Data\Environments\03_Track\track.vex", 5350.0),
    (r"Data\Environments\02_Track\track_reversed.vex", 4419.0),
];

/// The wiring's test: the figure the screen's worker takes is the authored one, `5178` for
/// Talon's Junction White, and `stats_reversed.xml` serves the reversed entry.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_authored_length_is_the_distance_the_screen_prints() {
    let Some(path) = image() else {
        return;
    };
    let source = path.to_string_lossy().into_owned();
    let mut archives =
        oag_assets::Archives::open(&source, oag_pulse::TITLE).expect("the archives open");
    for (name, shown) in SHOWN {
        let length = oag_raceplay::authored_length(&source, &mut archives, oag_pulse::TITLE, name);
        assert_eq!(length, Some(shown), "{name}");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_racing_line_length_is_the_distance_the_screen_prints() {
    let Some(path) = image() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    for (name, shown) in SHOWN {
        let blob = archives.read_name(name).expect("the circuit resolves");
        let length = oag_raceplay::circuit_length(&blob).expect("it measures");
        let off = (length - shown).abs() / shown;
        // Consistently 1.7 to 2.3 per cent short of the screen, and not by
        // sampling - see `circuit_length`'s own doc for the measurement.
        // Three per cent catches a wrong ring (one path alone is a third
        // short) without pretending the two curves are the same one.
        assert!(
            off < 0.03,
            "{name}: measured {length:.0}, the screen prints {shown:.0} ({:.1}% off)",
            off * 100.0
        );
    }
}
