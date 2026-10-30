//! The Zone and Eliminator tables' builders: what a finished race hands the screens.

use super::{elimination_results, tournament_results, zone_results};
use oag_raceplay::scoreboard::{Board, Row};
use oag_raceplay::tournament::Progress;

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

/// A board as a leg that ends on the person's finish leaves it: the person (slot 0) has
/// crossed, the opponents are running in `order`.
fn leg_board(order: [u8; 3]) -> Board {
    let row = |slot: u8, place: u8| Row {
        place,
        slot,
        laps_completed: 1,
        finish_tick: (slot == 0).then_some(1000),
        best_lap_ticks: None,
        player: slot == 0,
        name: None,
    };
    Board {
        rows: vec![row(0, 1), row(order[0], 2), row(order[1], 3), row(order[2], 4)],
        laps_target: Some(1),
        tick: 1000,
    }
}

/// The maintainer's report: a person winning every leg of a Tournament saw 8 for themselves
/// and 0 for everyone else on every end-race table. Two legs through the same calls
/// `Session::frame` and `Session::build_endrace` make (`Progress::record_leg`, then
/// `tournament_results` for the page's rows): each leg's own points for every slot, and the
/// totals carried across legs.
#[test]
fn a_two_leg_tournament_scores_every_slot_and_carries_the_totals() {
    let mut progress = Progress::new(vec!["01_Track".to_string(), "02_Track".to_string()]);
    let teams: Vec<String> = ["Assegai", "Feisar", "Qirex", "Goteki"]
        .map(str::to_string)
        .to_vec();
    let points = |rows: &[oag_ui_screens::endrace::TournamentRow]| -> Vec<u32> {
        rows.iter().map(|row| row.points).collect()
    };

    let first = leg_board([1, 2, 3]);
    progress.record_leg(&first);
    let page = tournament_results(Some(&first), &progress, Some(&teams), false).expect("a board");
    assert_eq!(points(&page.leg), [8, 6, 5, 4], "leg 1 by running place, opponents included");
    assert_eq!(points(&page.standings), [8, 6, 5, 4]);

    assert!(progress.advance());
    // Leg 2: slot 3 has overtaken, the person wins again.
    let second = leg_board([3, 1, 2]);
    progress.record_leg(&second);
    let page = tournament_results(Some(&second), &progress, Some(&teams), true).expect("a board");
    assert_eq!(points(&page.leg), [8, 6, 5, 4]);
    assert_eq!(
        points(&page.standings),
        [16, 11, 10, 9],
        "slot 0 8+8, slot 1 6+5, slot 3 4+6, slot 2 5+4, ranked by total"
    );
    assert!(page.standings[0].player);
}
