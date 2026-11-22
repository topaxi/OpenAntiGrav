//! A Detonator race flies Detonator's own hull, livery and handling file, and draws its own mine,
//! bomb and pad models.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^detonator_hull_ground_truth::/)'
//! ```
//!
//! The evidence is `docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`. Zone's own numbers
//! are the control: the same load in `Mode::Zone` must keep flying Zone's hull and file.

use oag_raceplay as race;

const HD: &str = "data/images/hdfury-ps3-eu-dec.iso";

fn load(mode: oag_race::Mode, team: &str) -> Option<race::Loaded> {
    let path = oag_testdata::image(HD)?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            team: Some(team.to_string()),
            mode,
            ..race::Options::default()
        })
        .expect("loading an HD race"),
    )
}

/// The hull's width (`Misc.width`), which differs between Zone's file (5.0) and Detonator's (4.0).
fn width(loaded: race::Loaded) -> f32 {
    let race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.core.sim.world.ships[0].handling.dimensions.width
}

/// Every craft flies `/data/ships/detonator/ship.vex`, in the team's own `zoneship_<team>` livery,
/// and the hull's locators and flare are that folder's.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_craft_flies_the_detonator_hull_in_its_teams_livery() {
    let Some(loaded) = load(oag_race::Mode::Detonator, "Qirex") else {
        return;
    };
    let livery = &loaded.liveries[0];
    assert!(
        livery
            .hull
            .label
            .to_ascii_lowercase()
            .contains("ships\\detonator\\ship.vex"),
        "the hull is {}",
        livery.hull.label
    );
    assert!(!livery.hull.indices.is_empty(), "the hull draws");
    assert!(
        livery.cannon_flash.iter().all(Option::is_some),
        "the Detonator folder's locators.vex authors both cannon_flash nodes"
    );
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("Detonator livery zoneship_quirex replaces zoneship_zone")),
        "the team's livery is swapped in: {:#?}",
        loaded.report
    );
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("a Detonator craft raises no shield")),
        "the shield is skipped and the report says so"
    );
}

/// The race flies `/data/ships/detonator/handlingstats.xml` (`Misc.width` 4.0), where a Zone race
/// on the same team flies Zone's (5.0).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_race_flies_its_own_handling_file_and_zone_keeps_its_own() {
    let Some(loaded) = load(oag_race::Mode::Detonator, "Qirex") else {
        return;
    };
    assert!(
        loaded.report.iter().any(|line| line.starts_with(
            "detonator: flying /data/ships/detonator/handlingstats.xml's classless block"
        )),
        "no Detonator handling line in the report"
    );
    assert_eq!(width(loaded), 4.0);
    let Some(zone) = load(oag_race::Mode::Zone, "Qirex") else {
        return;
    };
    assert!(
        zone.report
            .iter()
            .any(|line| line.starts_with("zone: flying /data/ships/zone/handlingstats.xml")),
        "Zone keeps its own file"
    );
    assert_eq!(width(zone), 5.0);
}
