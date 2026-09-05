//! The load report names the start gantry it does not draw.
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
//! `pulses_default_track_reports_its_own_trackstartup_xml`, which asserts the
//! slot *counts* on the same report - and that file sits exactly on its
//! `check-size` ceiling, which is lowered and never raised. So this takes the
//! same route every other ground-truth test here already takes and carries its
//! own `image()`/`load()` pair rather than growing a frozen file.
//!
//! # What it is for
//!
//! `docs/rendering/start-gantry.md` recovers what slot 8's model *does* -
//! `3`, `2`, `1` and `GO` as four UV cells walked across a palette staircase -
//! while where it stands is still unrecovered. The honest shape of that is a
//! named absence rather than a plausible placement, and the model's own name
//! (`321Go_StartFinish`) plus a spline that knows where the start line is
//! makes the plausible placement genuinely tempting. **An absence that stops
//! being stated is an absence that stops being fixed**, so the line is
//! asserted rather than left to survive by habit.

use std::path::{Path, PathBuf};

use oag_game::race;

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

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_report_names_the_start_gantry_and_says_it_is_not_loaded() {
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
        report.contains("slot 8 is the start gantry"),
        "the gantry is not named: {report}"
    );
    assert!(
        report.contains("/Data/Environments/321_Go/321Go_StartFinish.vex"),
        "the gantry line names no file: {report}"
    );
    assert!(
        report.contains("it is not loaded"),
        "the gantry line does not say it draws nothing: {report}"
    );
}
