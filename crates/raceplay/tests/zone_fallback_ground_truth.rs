//! A Zone race on 2048's and Omega's HD-lineage circuits draws HD's Zone surface.
//!
//! **`#[ignore]`d and never run in CI.** It needs the extracted Vita packages under
//! `data/extracted/vita/PCSF00007` and `data/images/omega-ps4-eu.pkg`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^zone_fallback_ground_truth::/)'
//! ```
//!
//! 2048's `Environment_Load` (`0x8102f6d0`) reads a circuit's own `ZoneMode2048.effectSettings` and
//! gives a circuit with no environment tree HD/Fury's title-wide table instead; those circuits'
//! materials declare HD's Zone variant (9,392 programs under `DLC1/environments`,
//! `gxp_ground_truth`). What fails here: dropping `RaceDefaults::zone_fallback` from either title
//! (no grade, or a grade with no stage art, so `enabled` is zero), pointing the texture sets at
//! the wrong entries (the counts), the ladder (stage 4 at zone 5, `68-zone-5.png`'s `Flash`
//! palette), or letting the fallback grade a circuit that keeps its own Zone model.

use std::path::{Path, PathBuf};

use oag_raceplay as race;

const VINETA_K: &str = r"Data\art\published\DLC1\environments\Vineta_K\track.vex";
const ZONE_1: &str = r"Data\art\published\DLC1\environments\zone_1\track.vex";
const ALTIMA: &str = r"Data\art\published\environments\altima\track.vex";
const TECH_DE_RA: &str = r"Data\environments\tech_de_ra\track.vex";
const BRIDGE_2048: &str = r"Data\Environments2048\bridge\track.vex";

fn vita() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists()
        && path.join("dlc1/PSP2/dlc1.psarc").exists()
        && path.join("patch-v104/PSP2/data2.psarc").exists()
    {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is incomplete",
        path.display()
    );
    println!("skipping: {} not complete", path.display());
    None
}

fn load(source: &Path, track: &str, mode: oag_race::Mode) -> race::Loaded {
    race::load(&race::Options {
        source: source.display().to_string(),
        track: Some(track.to_string()),
        team: Some("Feisar".to_string()),
        mode,
        seed: Some(1),
        ..race::Options::default()
    })
    .expect("the race loads")
}

/// A 2048 Zone race on one of HD's ported race circuits: HD's title-wide table, both texture sets
/// as `.gxt`, the surface switched on, and zone 5 on stage 4 (`Flash`).
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_zone_race_on_an_hd_circuit_draws_hds_zone_surface() {
    let Some(source) = vita() else { return };
    let loaded = load(&source, VINETA_K, oag_race::Mode::Zone);
    let mut grade = loaded
        .zone_grade
        .expect("the title-wide table grades the circuit");
    assert!(
        loaded.report.iter().any(|l| l.contains(
            "data/ZoneEnvironmentHDFury/ZoneModeHDFury.effectSettings: the Zone colour grade"
        )),
        "{:#?}",
        loaded.report
    );
    assert_eq!(grade.stage_art_counts(), (15, 15));
    assert_eq!(grade.stage_scene_art_counts(), (15, 15));
    assert_eq!(grade.stage_for_zone(5), Some(4), "zone 5 reads Flash");
    grade.show_zone(5);
    assert_eq!(grade.blend().current, 4);
    let zone = grade.zone_uniform();
    assert_eq!(zone.enabled, 1.0, "the surface term is on");
    // `"4 Flash.Scene.Texture Colour"` and `"4 Flash.Track.Texture Colour"` of `ZoneModeHDFury`.
    assert_eq!(zone.scene.effect[..3], [0.709_804, 0.972_549, 0.0]);
    assert_eq!(zone.track.effect[..3], [4.694_118, 7.0, 0.247_059]);
}

/// The four `zone_N` arenas take the `DLC3` revision and the arena surface shape.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_zone_arena_takes_the_dlc3_table() {
    let Some(source) = vita() else { return };
    let loaded = load(&source, ZONE_1, oag_race::Mode::Zone);
    let grade = loaded.zone_grade.expect("the DLC3 table grades the arena");
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.contains("ZoneModeHDFuryDLC3.effectSettings: the Zone colour grade")),
        "{:#?}",
        loaded.report
    );
    let zone = grade.zone_uniform();
    assert_eq!(zone.enabled, 1.0);
    assert_eq!(zone.shape[0], 1.0, "an arena is the aniso shape");
}

/// A native 2048 circuit keeps its own table and its own Zone model: the stage grade without a
/// surface recolour, and no other circuit's table.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_native_2048_circuit_keeps_its_own_zone_table_and_draws_no_surface() {
    let Some(source) = vita() else { return };
    let loaded = load(&source, ALTIMA, oag_race::Mode::Zone);
    let grade = loaded.zone_grade.expect("its own ZoneMode2048 table");
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.contains("altima\\ZoneMode2048.effectSettings: the Zone colour grade")),
        "{:#?}",
        loaded.report
    );
    assert_eq!(grade.zone_uniform().enabled, 0.0);
    assert_eq!(grade.stage_art_counts().0, 0);
}

/// Detonator's per-stage `.gxt` sets ride the fallback alone
/// (`Detonator::fallback_stage_textures`, `Environment_LoadHDFuryContent`): a native circuit's
/// Detonator race loads its own table and no stage art, and a ported one loads both sets.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_detonator_race_binds_its_stage_art_on_a_ported_circuit_and_not_a_native_one() {
    let Some(source) = vita() else { return };
    let native = load(&source, ALTIMA, oag_race::Mode::Detonator);
    let grade = native.zone_grade.expect("its own ZoneMode2048 table");
    assert_eq!(grade.zone_uniform().enabled, 0.0);
    assert_eq!(grade.stage_art_counts().0, 0, "{:#?}", native.report);
    assert!(
        !native.report.iter().any(|l| l.contains("detonatorMode")),
        "{:#?}",
        native.report
    );
    let ported = load(&source, VINETA_K, oag_race::Mode::Detonator);
    let grade = ported.zone_grade.expect("HD/Fury's Detonator table");
    assert_eq!(grade.stage_art_counts(), (15, 15), "{:#?}", ported.report);
}

/// Outside a Zone race nothing is graded.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_time_trial_on_an_hd_circuit_carries_no_zone_grade() {
    let Some(source) = vita() else { return };
    let loaded = load(&source, VINETA_K, oag_race::Mode::TimeTrial);
    assert!(loaded.zone_grade.is_none());
}

fn omega() -> Option<PathBuf> {
    oag_testdata::image("data/images/omega-ps4-eu.pkg")
}

/// An Omega Zone race on an HD-lineage circuit: HD's title-wide table, the `.gnf` sets, the
/// surface on.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn an_omega_zone_race_on_an_hd_circuit_draws_hds_zone_surface() {
    let Some(source) = omega() else { return };
    let loaded = load(&source, TECH_DE_RA, oag_race::Mode::Zone);
    let mut grade = loaded
        .zone_grade
        .expect("the title-wide table grades the circuit");
    assert_eq!(grade.stage_art_counts(), (15, 15));
    assert_eq!(grade.stage_scene_art_counts(), (15, 15));
    grade.show_zone(5);
    assert_eq!(grade.blend().current, 4);
    assert_eq!(grade.zone_uniform().enabled, 1.0);
}

/// Omega's 2048-lineage circuits keep their own `trackZone` model, which this build passes over,
/// so no grade is laid over the circuit the original draws another way.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn an_omega_2048_lineage_circuit_is_not_graded_over_its_missing_zone_model() {
    let Some(source) = omega() else { return };
    let loaded = load(&source, BRIDGE_2048, oag_race::Mode::Zone);
    assert!(loaded.zone_grade.is_none(), "{:#?}", loaded.report);
}
