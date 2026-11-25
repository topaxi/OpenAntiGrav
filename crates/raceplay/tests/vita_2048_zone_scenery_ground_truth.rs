//! A Wipeout 2048 Zone race draws `trackZone.rcsmodel` and moves it through
//! its own skeleton and clip, and a time trial on the same circuit does not.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted Vita package
//! extracted with `oag-unpack`; see `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^vita_2048_zone_scenery_ground_truth::/)'
//! ```
//!
//! What it holds: `altima`'s `trackZone` is 6,349 submeshes over 966 skeleton
//! nodes and 7 clip tracks at 30 Hz, against the race model's 2,817; 1,716 of
//! the Zone model's submeshes are on the `fc01_dummy` placeholder and are drawn as
//! the circuit's fog colour (its program's whole output); 1,587 take a `Zone_ColourN` uniform as their colour. Drop the Zone
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
        track.contains("1716 submesh(es) on fc01_dummy drawn as the circuit's fog colour"),
        "{track}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("fc01_dummy:") && l.contains("vertices drawn as fogColour")),
        "the placeholder's vertices were not marked: {lines:#?}"
    );
    assert!(
        track.contains("1587 submesh(es) take a Zone_ColourN"),
        "{track}"
    );
    // The model's own hidden nodes and, Zone running weapons off, its 61 weapon pad submeshes
    // (`docs/gameplay/pickups.md`, "A weapons-off race draws no weapon pad").
    assert!(track.contains("2363 hidden"), "{track}");
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

/// **A Zone race on an HD-lineage 2048 circuit (DLC1's Vineta K) draws its road through the
/// `Track` colour group.** The container carries no per-chunk track bit that has been read, so
/// the shapes named `*track*` stand in for HD's (chosen, not measured); without them the road
/// took the `Scene` group, the pale sheet the pair `2048-vita/zone-dlc-vineta-k-z5` showed.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_zone_race_on_dlc_vineta_k_draws_its_track_shapes_through_the_track_group() {
    let Some(source) = source() else { return };
    let lines = race::load(&race::Options {
        source: source.display().to_string(),
        mode: oag_race::Mode::Zone,
        track: Some(r"Data\art\published\DLC1\environments\Vineta_K\track.vex".into()),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {}: {e:#}", source.display()))
    .report;
    let track = lines
        .iter()
        .find(|l| l.contains("Vineta_K") && l.contains("triangle(s) over"))
        .expect("a track model line");
    assert!(
        track.contains("submesh(es) on track-named shapes take the Track Zone group"),
        "{track}"
    );
}
