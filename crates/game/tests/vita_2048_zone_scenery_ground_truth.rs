//! A Wipeout 2048 Zone race draws `trackZone.rcsmodel` and moves it through
//! its own skeleton and clip, and a time trial on the same circuit does not.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted Vita package
//! extracted with `oag-unpack`; see `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(vita_2048_zone_scenery_ground_truth)'
//! ```
//!
//! What it holds: `altima`'s `trackZone` is 6,349 submeshes over 966 skeleton
//! nodes and 7 clip tracks at 30 Hz, against the race model's 2,817; 1,716 of
//! the Zone model's submeshes are on the `fc01_dummy` placeholder and are not
//! drawn; 1,587 take a `Zone_ColourN` uniform as their colour. Drop the Zone
//! branch in `race::load::geometry::sibling_model` and every line here goes.

use oag_raceplay as race;
use std::path::{Path, PathBuf};

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but the 2048 package is not extracted"
    );
    println!("skipping: 2048 package not extracted under data/extracted/vita/");
    None
}

fn report(source: &Path, mode: oag_race::Mode) -> Vec<String> {
    race::load(&race::Options {
        source: source.display().to_string(),
        mode,
        zone_model: true,
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {}: {e:#}", source.display()))
    .report
}

fn track_line(report: &[String]) -> &str {
    report
        .iter()
        .find(|l| l.contains("altima") && l.contains("triangle(s) over"))
        .expect("a track model line")
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_zone_race_on_altima_draws_its_zone_model_through_its_zone_skeleton() {
    let Some(source) = source() else { return };
    let lines = report(&source, oag_race::Mode::Zone);
    assert!(
        lines
            .iter()
            .any(|l| l.contains("trackZone.rcsmodel is this circuit's Zone model")),
        "no Zone model line in {lines:#?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("966 skeleton node(s), 7 animated over 7 track(s)")),
        "the Zone skeleton and clip were not loaded"
    );
    let track = track_line(&lines);
    assert!(track.contains("over 6349 submesh(es)"), "{track}");
    assert!(
        track.contains("1716 submesh(es) on the placeholder shader fc01_dummy not drawn"),
        "{track}"
    );
    assert!(
        track.contains("1587 submesh(es) take a Zone_ColourN"),
        "{track}"
    );
    assert!(track.contains("2302 hidden"), "{track}");
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_time_trial_on_altima_still_draws_the_race_model() {
    let Some(source) = source() else { return };
    let lines = report(&source, oag_race::Mode::TimeTrial);
    assert!(
        !lines.iter().any(|l| l.contains("Zone model")),
        "a time trial drew the Zone model"
    );
    let track = track_line(&lines);
    assert!(track.contains("over 2817 submesh(es)"), "{track}");
    assert!(!track.contains("placeholder"), "{track}");
}
