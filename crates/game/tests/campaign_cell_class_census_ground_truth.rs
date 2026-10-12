//! Which `class` a campaign cell authors, on every title that ships grids.
//!
//! `oag_game::campaign::launch::plan_cell` falls back to the RACE page's class
//! for a cell whose class is not one of `SpeedClass`'s four (a Zone cell's
//! literal `Zone`); `plan_cell_for` adds the title's own longer ladder, which is
//! how Wipeout Pure's `Vector` races as Vector. The census, taken 2026-10-12
//! over Pulse (PSP USA and EU, PS2 EU), Wipeout HD/Fury and Omega, is what says
//! the four-rung titles' behaviour is unchanged: every grid cell's class is one
//! of the four or `Zone`, bar the `NitroBattle` cells of HD and Omega (every one of them a mode the engine refuses), whose class slot
//! holds a mode name the mode refuses before the class is read.
//!
//! **`#[ignore]`d and never run in CI**: it needs disc images.

use std::collections::BTreeSet;

use oag_assets::Archives;
use oag_tables::handling::SpeedClass;

fn classes(archives: &mut Archives, entry: &str, grid_archive: Option<&str>) -> BTreeSet<String> {
    oag_game::campaign::read_grids(archives, entry, grid_archive)
        .expect("the grids read")
        .into_iter()
        .flat_map(|grid| grid.cells)
        .inspect(|cell| {
            if cell.class == "NitroBattle" {
                assert!(
                    oag_game::campaign::race_mode_for_cell(cell.mode.clone()).is_none(),
                    "{}: a NitroBattle-class cell whose mode launches, so its class would be read",
                    cell.name
                );
            }
        })
        .map(|cell| cell.class)
        .collect()
}

fn assert_known(label: &str, found: &BTreeSet<String>) {
    println!("{label}: {found:?}");
    assert!(!found.is_empty(), "{label}: no cells read");
    for class in found {
        assert!(
            SpeedClass::from_name(class).is_some()
                || class.eq_ignore_ascii_case("Zone")
                || (matches!(label, "hdfury" | "omega") && class == "NitroBattle"),
            "{label}: a cell authors class {class:?}, which is neither a speed class nor Zone - \
             plan_cell_for's ladder must be given for this title"
        );
    }
}

#[test]
#[ignore = "needs disc images"]
fn every_shipped_cell_class_is_a_speed_class_or_zone() {
    let mut read = 0;
    for image in [
        "data/images/pulse-psp-usa.chd",
        "data/images/pulse-psp-eu.chd",
        "data/images/pulse-ps2-eu.chd",
    ] {
        let Some(path) = oag_testdata::image(image) else {
            continue;
        };
        let mut archives = oag_pulse::open(&path.display().to_string()).expect("open as Pulse");
        let title = oag_pulse::TITLE.campaign;
        let found = classes(
            &mut archives,
            title.definition_entry.expect("Pulse names its grids"),
            title.grid_archive,
        );
        assert_known(image, &found);
        read += 1;
    }
    if let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu.iso") {
        let mut archives = oag_hd::open(&path.display().to_string()).expect("open as HD");
        let title = oag_hd::TITLE.campaign;
        let found = classes(
            &mut archives,
            title.definition_entry.expect("HD names its grids"),
            title.grid_archive,
        );
        assert_known("hdfury", &found);
        read += 1;
    }
    if let Some(path) = oag_testdata::exact("data/extracted/ps4") {
        let mut archives = oag_omega::open(&path.display().to_string()).expect("open as Omega");
        let found = classes(&mut archives, oag_omega::campaign::DEFINITION_ENTRY, None);
        assert_known("omega", &found);
        read += 1;
    }
    println!("{read} title image(s) read");
}
