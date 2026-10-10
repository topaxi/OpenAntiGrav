//! The Zone and Eliminator tables' builders: what a finished race hands the screens.

use super::{elimination_results, zone_results};

fn ship(kills: u32, deaths: u32) -> oag_gameplay::Ship {
    let mut ship = oag_gameplay::Ship::default();
    ship.standing.kills = kills;
    ship.standing.deaths = deaths;
    ship
}

/// Every ship in the field becomes a row, slot 0 is the player's, and each slot's team
/// is the roster's id at that index - kills and deaths straight off the standing.
#[test]
fn the_eliminator_table_reads_kills_and_deaths_off_every_ship() {
    let ships = [ship(1, 2), ship(4, 0), ship(4, 1)];
    let teams = [
        "Assegai".to_string(),
        "Feisar".to_string(),
        "Qirex".to_string(),
    ];
    let table = elimination_results(&ships, Some(&teams));
    let rows: Vec<_> = table
        .rows()
        .iter()
        .map(|row| (row.team_name.as_deref(), row.kills, row.deaths, row.player))
        .collect();
    assert_eq!(
        rows,
        [
            (Some("Feisar"), 4, 0, false),
            (Some("Qirex"), 4, 1, false),
            (Some("Assegai"), 1, 2, true),
        ]
    );
    assert_eq!(table.place(), Some(3));
}

/// A launch that named no team draws no team cells - the roster is not guessed.
#[test]
fn a_launch_with_no_team_leaves_the_team_cells_absent() {
    let table = elimination_results(&[ship(0, 0), ship(1, 0)], None);
    assert!(table.rows().iter().all(|row| row.team_name.is_none()));
    assert_eq!(table.place(), Some(2));
}

/// The two numbers the world keeps come through as they are, the two the view tallies
/// come from it, and the two the original steps by an unrecovered rule stay off.
#[test]
fn the_zone_table_blanks_the_two_rows_it_cannot_count() {
    let state = oag_race::RaceState {
        zone: 9,
        score: 12_345,
        ..oag_race::RaceState::default()
    };
    let table = zone_results(&state, oag_raceplay::RunStats::default());
    assert_eq!(table.zones_cleared, 9);
    assert_eq!(table.score, 12_345);
    assert_eq!(table.perfect_zones, Some(0));
    assert_eq!(table.top_speed_kmh, Some(0));
    assert_eq!(table.laps_cleared, None);
    assert_eq!(table.perfect_laps, None);
}

/// A mid-Tournament menu opens on `ER_NEXT_RACE`, so a plain confirm
/// continues the tournament instead of leaving it.
#[test]
fn mid_tournament_menu_opens_on_next_race() {
    use super::menu_options;
    use oag_ui_screens::endrace::MenuOption;

    assert_eq!(
        menu_options(true, true),
        [
            MenuOption::NextRace,
            MenuOption::ReturnToGrid,
            MenuOption::ViewResultsAgain
        ]
    );
    assert_eq!(menu_options(true, false)[0], MenuOption::ReturnToGrid);
}
