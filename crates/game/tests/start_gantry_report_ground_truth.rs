//! The load report names the start gantry, where it stands, and what of it is
//! not drawn.
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
//! # What it is for, and what changed under it
//!
//! **This file used to assert the opposite of what it asserts now, and that is
//! the intended outcome rather than a weakening.** Until 2026-09-06 slot 8's
//! world transform was unrecovered, so the honest shape was a *named absence*:
//! the report had to say the gantry existed, name its file, and state that it
//! was not loaded. The model's own name (`321Go_StartFinish`) plus a spline
//! that knows where the start line is made a plausible-looking placement
//! genuinely tempting, and "an absence that stops being stated is an absence
//! that stops being fixed" is why the line was pinned by a test rather than
//! left to survive by habit.
//!
//! The absence is gone. `oag_render::gantry` measures the mounting surface each
//! circuit's own track model authors and `race::gantry` stands the model on it
//! (`docs/rendering/start-gantry.md`), so the true answer is now the other one.
//!
//! **The rule the file protects is unchanged, and it still has teeth**, because
//! the placement did not make everything about the gantry knowable. Two things
//! are still deliberately not drawn, and the report has to keep saying so:
//!
//! - the `FINAL LAP` and chequered boards, whose triggers are unrecovered -
//!   `oag_render::gantry::clip_to_panel` drops them and the report gives the
//!   count;
//! - billboard slots 1-7, whose mounting surfaces are still unrecovered.
//!
//! So this asserts the positive claim *and* both remaining absences. A report
//! that started drawing the lap board on a guess, or quietly stopped mentioning
//! the seven unplaced hoardings, fails here.

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

fn report_for_the_default_track() -> Option<String> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded.report.join("\n");
    println!("{report}");
    Some(report)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_report_names_the_start_gantry_and_the_mount_it_stands_on() {
    let Some(report) = report_for_the_default_track() else {
        return;
    };

    assert!(
        report.contains("start gantry"),
        "the gantry is not named: {report}"
    );
    assert!(
        report.contains("/Data/Environments/321_Go/321Go_StartFinish.vex"),
        "the gantry line names no file: {report}"
    );
    // **The placement, not just the file.** Naming the model without saying
    // where it went would be as uninformative as the old "not loaded" line was
    // once the placement existed - and it is exactly what a regression to a
    // hardcoded coordinate would still print. `16_Track`'s mount is mesh node
    // 74, measured off its own geometry; see `docs/rendering/start-gantry.md`.
    assert!(
        report.contains("on node Some(74)"),
        "the gantry line does not say which node it measured its mount from: {report}"
    );
    assert!(
        report.contains("measured off this circuit's own geometry"),
        "the gantry line does not say the mount is measured rather than assumed: {report}"
    );
    // And that it did *not* silently fall back. `place` pushes a line starting
    // "no start gantry" for every link it could not follow, so any of them
    // appearing here means this circuit did not get the gantry it claims to.
    assert!(
        !report.contains("no start gantry"),
        "the report both places and fails to place the gantry: {report}"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_report_still_names_everything_about_the_gantry_that_is_not_drawn() {
    let Some(report) = report_for_the_default_track() else {
        return;
    };

    // The `FINAL LAP` and chequered states. They are in the file, they are
    // legible, and they are parked beside the countdown panel rather than
    // hidden - so drawing them is drawing an effect whose trigger nobody has
    // recovered, on lap one, every race. The report says how many draws went.
    assert!(
        report.contains("parked outside the panel are not drawn"),
        "the report does not say the unwired states are dropped: {report}"
    );
    assert!(
        report.contains("whose trigger is unrecovered"),
        "the report does not say *why* they are dropped: {report}"
    );
    // The other seven slots, which the placement did not touch.
    assert!(
        report.contains("slots 1-7 unplaced"),
        "the report no longer states that the hoarding slots are unplaced: {report}"
    );
}
