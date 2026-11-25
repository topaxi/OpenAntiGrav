//! A weapons-off race on 2048's container (2048 and Omega) draws no weapon pad.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! The container names no pad class, so the circuit's weapon pads come with its world
//! geometry and only the loader's choice of `HidePads` keeps them out of a Zone race
//! (`docs/gameplay/pickups.md`, "A weapons-off race draws no weapon pad"). Pass
//! `HidePads::None` in `load::geometry::hidden_pads` and every difference here reads zero.

use oag_race::Mode;
use oag_raceplay as race;

fn track_triangles(source: &str, track: &str, mode: Mode, weapons: Option<bool>) -> usize {
    let loaded = race::load(&race::Options {
        source: source.to_string(),
        track: Some(track.to_string()),
        mode,
        weapons_override: weapons,
        no_effects: true,
        ..race::Options::default()
    })
    .unwrap_or_else(|error| panic!("loading {track} in {mode:?}: {error:#}"));
    assert_eq!(
        loaded
            .report
            .iter()
            .any(|line| line
                .contains("weapons are off, the circuit's own weapon pads are not drawn")),
        !loaded.setup.weapons_on(),
        "{mode:?}: the report says what the build did"
    );
    loaded.track_model.indices.len() / 3
}

/// `weapon` is the circuit's weapon pad triangles, counted off the `.rcsmodel` by material and
/// shape name (`crates/mesh/tests/psp2_circuit_pads_ground_truth.rs`).
fn check(source: &str, track: &str, weapon: usize) {
    let armed = track_triangles(source, track, Mode::SingleRace, None);
    let unarmed = track_triangles(source, track, Mode::SingleRace, Some(false));
    assert_eq!(armed - unarmed, weapon, "{track}: WEAPONS off");
    let zone = track_triangles(source, track, Mode::Zone, None);
    assert_eq!(armed - zone, weapon, "{track}: Zone");
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_draws_no_weapon_pad_with_weapons_off() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    check(
        &path.display().to_string(),
        r"Data\environments\talons_junction\track.vex",
        4527,
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn wipeout_2048_draws_no_weapon_pad_with_weapons_off() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    check(
        &path.display().to_string(),
        r"data\art\published\environments\altima\track.vex",
        14_762,
    );
}
