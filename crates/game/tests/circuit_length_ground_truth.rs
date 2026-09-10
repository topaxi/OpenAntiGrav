//! `Track Creation`'s `Distance(m)` row against the disc: the racing line's
//! length, summed the way `oag_game::race::circuit_length` sums it,
//! against the number the original prints on the screen - close, and not
//! equal; see `circuit_length`'s doc for the measured gap.
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
        let length = oag_game::race::circuit_length(&blob).expect("it measures");
        eprintln!(
            "DBG {name}: measured {length:.1}, shown {shown:.0}, ratio {:.4}",
            shown / length
        );
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
