//! A Zone craft's flaps: fully out on the grid, folded back after the start, and stowed on the
//! hull at rest, on every title that draws them.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! The level is the view's (`docs/physics/grid-state.md`, "The flaps on the grid"). The pose is
//! the hull's: Omega's shared Zone hull poses its flap nodes 45 degrees out of its `.vex` hinge,
//! so a level of zero still drew them raised until the hinge came from the `.vex`
//! ("The Zone hull's flaps"). Drop `Model::seat_flaps_on_vex` from the hull load and the Omega
//! case goes red.

use oag_raceplay as race;

/// `hinge_aft` checks that the hinge's third axis points aft, as every Zone hull authored in the
/// hinge's frame does; HD's hinge is the identity (its node table places the flap) and the PSP
/// titles' is their own.
fn check(source: &std::path::Path, hinge_aft: bool) {
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        mode: oag_race::Mode::Zone,
        no_effects: true,
        ..race::Options::default()
    })
    .expect("loading a Zone race");
    let flaps = loaded.liveries[0].hull.airbrakes.clone();
    assert!(
        flaps.iter().all(Option::is_some),
        "the Zone hull has both flaps"
    );
    if hinge_aft {
        for flap in flaps.iter().flatten() {
            let axis = flap.hinge.z_axis.truncate().normalize();
            assert!(
                axis.z < -0.99,
                "the stowed hinge points aft, not {axis:?}: the flap rests raised"
            );
        }
    }
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    let grid = race.airbrake_flaps_of(0);
    assert!(grid[0] > 0.0 && grid[1] > 0.0, "out on the grid: {grid:?}");
    for _ in 0..oag_race::COUNTDOWN_TICKS + 300 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    assert_eq!(
        race.airbrake_flaps_of(0),
        [0.0; 2],
        "folded after the start"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_zone_flaps_fold_after_the_start() {
    if let Some(source) = oag_testdata::image("data/images/pulse-psp-usa.chd") {
        check(&source, false);
    }
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_zone_flaps_fold_after_the_start() {
    if let Some(source) = oag_testdata::image("data/images/pulse-ps2-eu.chd") {
        check(&source, false);
    }
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pure_zone_flaps_fold_after_the_start() {
    if let Some(source) = oag_testdata::image("data/images/pure-psp-usa.chd") {
        check(&source, false);
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_zone_flaps_fold_after_the_start() {
    if let Some(source) = oag_testdata::image("hdfury-ps3-eu-dec.iso") {
        check(&source, false);
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn wipeout_2048_zone_flaps_fold_after_the_start() {
    if let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") {
        check(&source, true);
    }
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_zone_flaps_fold_after_the_start_and_rest_on_the_hull() {
    if let Some(source) = oag_testdata::exact("data/extracted/ps4") {
        check(&source, true);
    }
}

/// The same hull built as a front-end model (`oag_livery::entry::ps3_scene_model`, what the
/// menus draw) rests the same way.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_zone_hull_rests_on_the_hull_as_a_menu_model() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&source.display().to_string()).expect("opening Omega");
    let entry = r"Data\art\published\hdships\Zone\Ship.vex";
    let blob = archives.read_name(entry).expect("the Zone hull");
    let (model, _) =
        oag_livery::entry::ps3_scene_model(&mut archives, entry, &blob, None, &mut Vec::new())
            .expect("the Zone hull builds");
    for flap in model.airbrakes.iter() {
        let axis = flap
            .as_ref()
            .expect("a flap")
            .hinge
            .z_axis
            .truncate()
            .normalize();
        assert!(axis.z < -0.99, "the stowed hinge points aft, not {axis:?}");
    }
}
