//! The load report says nothing about a start gantry on Wipeout Pure, because
//! there is none to name.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The test skips with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # Why this is its own file
//!
//! It belongs beside `race_ground_truth.rs`'s
//! `pulses_default_track_reports_its_own_trackstartup_xml` and its own Pulse
//! sibling `start_gantry_report_ground_truth.rs`, both of which sit at or near
//! `race_ground_truth.rs`'s frozen `check-size` ceiling - so this takes the
//! same route and carries its own `image()`/`load()` pair.
//!
//! # What it is for
//!
//! `docs/rendering/start-gantry.md`'s "Wipeout Pure" section records that
//! `crates/formats/tests/start_gantry_pure_ground_truth.rs` reads all sixteen
//! circuits' own `TrackStartup.xml` and finds no `num="8"` on any of them,
//! model or colour - so `oag_raceplay::load`'s billboard-slot-8 report line
//! (added for Pulse in `crates/raceplay/src/load.rs`) never fires here, and
//! that is the correct, title-agnostic behaviour rather than a gap: the same
//! code that names Pulse's gantry by not finding a slot 8 on Pure's default
//! circuit, says nothing, honestly, because the circuit's own manifest says
//! nothing.

use std::path::PathBuf;

use oag_raceplay as race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pure-psp-usa.chd")
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pures_default_track_names_no_gantry_and_no_billboard_location() {
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded.report.join("\n");
    println!("{report}");

    assert!(
        !report.contains("slot 8 is the start gantry"),
        "the report names a gantry on Pure's default track, which contradicts \
         start_gantry_pure_ground_truth.rs: {report}"
    );
    assert!(
        report.contains("naming a model and"),
        "the trackstartup.xml line is missing entirely: {report}"
    );
    assert!(
        report.contains("0 naming a model"),
        "Pure's default track (01_Vineta_K) now names a billboard model - \
         re-check whether it is the gantry: {report}"
    );
}
