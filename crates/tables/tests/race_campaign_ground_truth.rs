//! Validates [`race_campaign`](oag_tables::race_campaign) against the real
//! campaign: `Data\Plugins\grids\grid_00.xml` .. `grid_15.xml` inside
//! `Data.wad` on the USA PSP pressing.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The test skips with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure.
//!
//! # What this is for
//!
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md` measured **16 grids,
//! 236 cells**, and an exact per-class lap census: 3 Venom, 4 Flash, 4 Rapier,
//! 5 Phantom, Speed Lap always 7, Zone always 0. This is the check that the
//! parser reproduces those counts against the disc the RE pass read them off
//! of. **If this ever disagrees with the numbers above, that is a finding
//! about the parser or the disc, not a reason to edit the numbers here** - see
//! `CLAUDE.md`'s rule against adjusting a measurement to match code.

use std::path::PathBuf;

use oag_assets::Archive;
use oag_pulse::campaign::{self, DEFINITION_ENTRY};
use oag_tables::race_campaign::{self, Mode};

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

const ARCHIVE: &str = "PSP_GAME/USRDIR/Data.wad";

/// Opens `Data.wad` on the USA PSP pressing, or `None` if the disc is absent.
fn archive() -> Option<Archive> {
    let path = image("pulse-psp-usa.chd")?;
    let spec = format!("{}:{}", path.display(), ARCHIVE);
    Some(Archive::open(&spec).unwrap_or_else(|e| panic!("{spec}: {e}")))
}

/// Reads and parses every grid `Definition.xml` itself lists, walking the
/// disc's own list rather than assuming [`campaign::GRID_COUNT`] up front -
/// so a disc that ships a different count is a finding this test surfaces,
/// not a shape it has to already know.
fn read_all_grids(archive: &mut Archive) -> Vec<race_campaign::Grid> {
    let definition = archive
        .read_name(DEFINITION_ENTRY)
        .unwrap_or_else(|e| panic!("{DEFINITION_ENTRY}: {e}"));
    let expanded =
        oag_tables::fexml::text(&definition).unwrap_or_else(|e| panic!("{DEFINITION_ENTRY}: {e}"));
    let entries = race_campaign::definition_entries(&expanded);

    entries
        .iter()
        .map(|src| {
            let blob = archive
                .read_name(src)
                .unwrap_or_else(|e| panic!("{src}: {e}"));
            race_campaign::from_blob(&blob).unwrap_or_else(|e| panic!("{src}: {e}"))
        })
        .collect()
}

/// **The headline claim**: 16 grids, 236 cells total.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn sixteen_grids_hold_two_hundred_and_thirty_six_cells() {
    let Some(mut archive) = self::archive() else {
        return;
    };

    let grids = read_all_grids(&mut archive);
    assert_eq!(grids.len(), 16, "grid count");
    assert_eq!(grids.len(), usize::from(campaign::GRID_COUNT));

    let total_cells: usize = grids.iter().map(|g| g.cells.len()).sum();
    assert_eq!(total_cells, 236, "cell count across all sixteen grids");

    println!("{} grids, {total_cells} cells", grids.len());
}

/// [`campaign::entry_name`] must agree with what `Definition.xml` itself
/// lists, index for index - the constant is a convenience over the same
/// disc-authored names, not a second source of truth.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn entry_name_agrees_with_definition_xml() {
    let Some(mut archive) = self::archive() else {
        return;
    };

    let definition = archive.read_name(DEFINITION_ENTRY).expect("Definition.xml");
    let expanded = oag_tables::fexml::text(&definition).expect("expand");
    let entries = race_campaign::definition_entries(&expanded);

    assert_eq!(entries.len(), usize::from(campaign::GRID_COUNT));
    for (index, entry) in entries.iter().enumerate() {
        let index = u8::try_from(index).expect("fewer than 256 grids");
        assert_eq!(*entry, campaign::entry_name(index), "grid index {index}");
    }
}

/// The exact per-class lap census `race-campaign.md` recorded: 3 Venom, 4
/// Flash, 4 Rapier, 5 Phantom with **no exception** across every `Race`,
/// `Tournament` and `Head2Head` cell; `Speed Lap` always 7; `Zone` always 0;
/// `Elimination` carries no `laps` attribute at all.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn lap_counts_match_the_class_exactly_with_no_exception() {
    let Some(mut archive) = self::archive() else {
        return;
    };

    let grids = read_all_grids(&mut archive);
    let mut checked = 0usize;

    for grid in &grids {
        for cell in &grid.cells {
            match cell.mode {
                Mode::Race | Mode::Tournament | Mode::Head2Head => {
                    let expected = match cell.speed_class() {
                        Some(oag_tables::handling::SpeedClass::Venom) => 3,
                        Some(oag_tables::handling::SpeedClass::Flash) => 4,
                        Some(oag_tables::handling::SpeedClass::Rapier) => 4,
                        Some(oag_tables::handling::SpeedClass::Phantom) => 5,
                        None => panic!(
                            "{}: {} cell has no recognised speed class ({:?})",
                            grid.name, cell.name, cell.class
                        ),
                    };
                    assert_eq!(
                        cell.laps,
                        Some(expected),
                        "{}: {} ({} {})",
                        grid.name,
                        cell.name,
                        cell.mode,
                        cell.class
                    );
                    checked += 1;
                }
                Mode::SpeedLap => {
                    assert_eq!(cell.laps, Some(7), "{}: {}", grid.name, cell.name);
                    checked += 1;
                }
                Mode::Zone => {
                    assert_eq!(cell.laps, Some(0), "{}: {}", grid.name, cell.name);
                    checked += 1;
                }
                Mode::Elimination => {
                    assert_eq!(
                        cell.laps, None,
                        "{}: {} carries a laps attribute Elimination should not have",
                        grid.name, cell.name
                    );
                    checked += 1;
                }
                Mode::TimeTrial => {
                    // Same per-class table as Race/Tournament/Head2Head, per
                    // race-campaign.md, but not independently asserted here
                    // to keep this test's claim to the modes it names.
                    checked += 1;
                }
                Mode::CustomGrid | Mode::AiRace | Mode::Other(_) => {
                    panic!(
                        "{}: {} is mode {:?}, not authored by any shipped grid",
                        grid.name, cell.name, cell.mode
                    );
                }
            }
        }
    }

    assert_eq!(
        checked, 236,
        "every one of the 236 cells should be classified"
    );
}

/// The mode census `race-campaign.md` recorded: `Race` 59, `Time Trial` 47,
/// `Speed Lap` 42, `Tournament` 27, `Head2Head` 23, `Elimination` 22, `Zone`
/// 16 - summing to 236.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_mode_census_matches_the_re_pass() {
    let Some(mut archive) = self::archive() else {
        return;
    };

    let grids = read_all_grids(&mut archive);
    let mut race = 0;
    let mut tournament = 0;
    let mut time_trial = 0;
    let mut zone = 0;
    let mut elimination = 0;
    let mut head2head = 0;
    let mut speed_lap = 0;

    for grid in &grids {
        for cell in &grid.cells {
            match cell.mode {
                Mode::Race => race += 1,
                Mode::Tournament => tournament += 1,
                Mode::TimeTrial => time_trial += 1,
                Mode::Zone => zone += 1,
                Mode::Elimination => elimination += 1,
                Mode::Head2Head => head2head += 1,
                Mode::SpeedLap => speed_lap += 1,
                Mode::CustomGrid | Mode::AiRace | Mode::Other(_) => {
                    panic!(
                        "{}: {} unexpected mode {:?}",
                        grid.name, cell.name, cell.mode
                    )
                }
            }
        }
    }

    println!(
        "Race {race}, Tournament {tournament}, Time Trial {time_trial}, Zone {zone}, \
         Elimination {elimination}, Head2Head {head2head}, Speed Lap {speed_lap}"
    );

    assert_eq!(race, 59, "Race");
    assert_eq!(time_trial, 47, "Time Trial");
    assert_eq!(speed_lap, 42, "Speed Lap");
    assert_eq!(tournament, 27, "Tournament");
    assert_eq!(head2head, 23, "Head2Head");
    assert_eq!(elimination, 22, "Elimination");
    assert_eq!(zone, 16, "Zone");
    assert_eq!(
        race + tournament + time_trial + zone + elimination + head2head + speed_lap,
        236
    );
}

/// The `RequiredPoints` ladder and each grid's own maximum:
/// `12/16/20/24/28` against `24/30/36/42/48`, per `race-campaign.md`'s table.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn required_points_and_maxima_match_the_documented_ladder() {
    let Some(mut archive) = self::archive() else {
        return;
    };

    let grids = read_all_grids(&mut archive);
    assert_eq!(grids.len(), 16);

    let expected_required = [
        12, 16, 20, 24, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 0,
    ];
    let expected_max = [
        24, 30, 36, 42, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
    ];

    for (index, grid) in grids.iter().enumerate() {
        assert_eq!(
            grid.required_points, expected_required[index],
            "{}: RequiredPoints",
            grid.name
        );
        assert_eq!(
            grid.max_points(),
            expected_max[index],
            "{}: max points",
            grid.name
        );
    }

    // grid0 unlocks nothing; grid1..grid15 each name the grid before it.
    assert_eq!(grids[0].unlock_grid, None, "grid0 should have no <Unlock>");
    for (index, grid) in grids.iter().enumerate().skip(1) {
        assert_eq!(
            grid.unlock_grid.as_deref(),
            Some(format!("Grid{}", index - 1)).as_deref(),
            "{}: <Unlock Grid=>",
            grid.name
        );
    }
}

/// `Group="1"` is authored on `grid12`..`grid15` and nowhere else, per
/// `race-campaign.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn group_marks_exactly_the_last_four_grids() {
    let Some(mut archive) = self::archive() else {
        return;
    };

    let grids = read_all_grids(&mut archive);
    for (index, grid) in grids.iter().enumerate() {
        let expected = if index >= 12 { Some(1) } else { None };
        assert_eq!(grid.group, expected, "{}: Group", grid.name);
    }
}

/// `Unlock_GridPointsMet` against the real disc data: `grid0`'s own eight
/// cells, fully golded, sum to 24 points against its authored
/// `RequiredPoints="12"` - so `grid1`'s own `<Unlock Grid="Grid0">` row is
/// met - while nothing golded, or a medal on the *wrong* grid, is not. This
/// exercises the comparison itself on the genuine authored numbers, not an
/// invented fixture - see `crates/tables/src/race_campaign/tests.rs` for the
/// unit-level coverage of the function's own edge cases.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn grid_points_met_reproduces_the_unlock_ladder_on_real_data() {
    let Some(mut archive) = self::archive() else {
        return;
    };

    let grids = read_all_grids(&mut archive);
    assert_eq!(grids[0].required_points, 12);
    assert_eq!(grids[0].cells.len(), 8);

    let none = |_: &str| None;
    assert!(!race_campaign::grid_points_met(&grids, "Grid0", &none));

    let gold_in_grid0 = |name: &str| {
        grids[0]
            .cells
            .iter()
            .any(|cell| cell.name == name)
            .then_some(race_campaign::Medal::Gold)
    };
    assert!(race_campaign::grid_points_met(
        &grids,
        "Grid0",
        &gold_in_grid0
    ));
    // A medal on grid0's own cells does not satisfy grid1's unlock row -
    // the comparison is per named grid, not "any medal anywhere".
    assert!(!race_campaign::grid_points_met(
        &grids,
        "Grid1",
        &gold_in_grid0
    ));
}
