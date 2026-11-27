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
        menu_options(true, true, false),
        [
            MenuOption::NextRace,
            MenuOption::ReturnToGrid,
            MenuOption::ViewResultsAgain
        ]
    );
    assert_eq!(
        menu_options(true, false, false)[0],
        MenuOption::ReturnToGrid
    );
    // WATCH REPLAY follows VIEW RESULTS AGAIN, only when asked for.
    assert_eq!(
        menu_options(false, false, true),
        [
            MenuOption::ReturnToMenu,
            MenuOption::RaceAgain,
            MenuOption::ViewResultsAgain,
            MenuOption::WatchReplay
        ]
    );
    assert!(!menu_options(false, false, false).contains(&MenuOption::WatchReplay));
}

/// WATCH REPLAY is offered on a recorded race on Pulse's list and HD's blocks, and nowhere it
/// could not be drawn, would break the original's screen, or would end a Tournament.
#[test]
fn watch_replay_is_offered_on_pulse_and_hd_recorded_races() {
    use super::offers_watch_replay;
    use oag_title::EndRaceDialect::{Field, Pulse, Touch};

    for dialect in [Pulse, Field] {
        assert!(offers_watch_replay(true, Some(dialect), false, false));
        assert!(
            !offers_watch_replay(false, Some(dialect), false, false),
            "not recorded"
        );
        assert!(
            !offers_watch_replay(true, Some(dialect), true, false),
            "--original-menu"
        );
        assert!(
            !offers_watch_replay(true, Some(dialect), false, true),
            "a leg to race"
        );
    }
    assert!(!offers_watch_replay(true, Some(Touch), false, false));
    assert!(!offers_watch_replay(true, None, false, false));
}

/// HD's own title package resolves to a dialect that offers the row, and Omega, which draws no
/// end-of-race menu, does not.
#[test]
fn the_title_packages_that_offer_watch_replay() {
    use super::offers_watch_replay;

    let offers = |title| offers_watch_replay(true, crate::endrace::dialect(title), false, false);
    assert!(offers(oag_pulse::TITLE) && offers(oag_hd::TITLE));
    assert!(!offers(oag_2048::TITLE) && !offers(oag_omega::TITLE) && !offers(oag_pure::TITLE));
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
        points: None,
    };
    Board {
        rows: vec![
            row(0, 1),
            row(order[0], 2),
            row(order[1], 3),
            row(order[2], 4),
        ],
        laps_target: Some(1),
        tick: 1000,
    }
}

/// Regression: a person winning every leg of a Tournament saw 8 for themselves
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
    assert_eq!(
        points(&page.leg),
        [8, 6, 5, 4],
        "leg 1 by running place, opponents included"
    );
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

/// The HD results model carries a Detonator run's rows: the session builds them off the race
/// and the scoreboard (`crate::results_tables::detonator_results`), and the screen draws them in
/// place of the field grid. Dropping the call leaves the headline over the time-trial grid.
#[test]
fn the_session_fills_the_detonator_rows_for_an_hd_detonator_race() {
    let source = include_str!("../../session/endrace.rs");
    assert!(
        source.contains("results_tables::detonator_results("),
        "build_endrace must build FieldResults::detonator"
    );
}

/// The record notice is judged where the race is saved, before it is folded into the store, and
/// the results model takes it; without either the screen shows no record of the run.
#[test]
fn the_session_judges_the_detonator_score_before_saving_and_the_results_take_it() {
    let save = include_str!("../../session/frame.rs");
    let judged = save
        .find("judge_score(")
        .expect("the finish arm judges the score");
    let saved = save
        .find("self.records.record(key, observation)")
        .expect("the finish arm saves the race");
    assert!(judged < saved);
    let build = include_str!("../../session/endrace.rs");
    assert!(build.contains("results.record = stage.score_record"));
}

/// The live EndRace renderer is its own: its `default` text must take HD's ratio and it must hold
/// the `Title` and button faces, or the screen a player sees draws small and without the cross
/// while the `--menu-page` still looks right.
#[test]
fn the_session_builds_the_end_screens_with_hds_faces() {
    let source = include_str!("../../session/endrace/prefetch.rs");
    let body = &source[..source.find("#[cfg(test)]").expect("its tests")];
    assert!(body.contains("FaceScales::default_ratio("));
    assert!(body.contains("shell.menu_skin.default_face_is_row_face"));
    assert!(body.contains("prep.install_faces("));
}
