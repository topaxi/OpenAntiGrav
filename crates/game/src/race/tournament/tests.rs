use super::*;
use crate::scoreboard::Row;

fn board(places: &[(u8, u8, bool)]) -> Board {
    Board {
        rows: places
            .iter()
            .map(|&(slot, place, finished)| Row {
                place,
                slot,
                laps_completed: 3,
                finish_tick: finished.then_some(1000),
                best_lap_ticks: None,
                player: slot == 0,
            })
            .collect(),
        laps_target: Some(3),
        tick: 1000,
    }
}

/// The headless case the implementation brief asks for: a two-leg
/// tournament accumulates points across both legs and ranks the field
/// correctly, with no real race or disc image involved.
#[test]
fn a_two_leg_tournament_accumulates_and_ranks_correctly() {
    let mut progress = Progress::new(vec!["leg_a.vex".to_string(), "leg_b.vex".to_string()]);
    assert_eq!(progress.current_track(), Some("leg_a.vex"));
    assert!(!progress.is_last_leg());

    // Leg 1: the player (slot 0) wins, slot 1 comes second.
    progress.record_leg(&board(&[(0, 1, true), (1, 2, true)]));
    assert_eq!(progress.points(0), 8);
    assert_eq!(progress.points(1), 6);

    assert!(progress.advance());
    assert_eq!(progress.current_track(), Some("leg_b.vex"));
    assert!(progress.is_last_leg());

    // Leg 2: slot 1 wins, the player comes second.
    progress.record_leg(&board(&[(0, 2, true), (1, 1, true)]));
    assert_eq!(progress.points(0), 8 + 6);
    assert_eq!(progress.points(1), 6 + 8);
    assert_eq!(progress.points(0), progress.points(1), "an exact tie");

    // A tie in total points is broken by slot order - slot 0 leads.
    assert_eq!(progress.rank(0), 1);
    assert_eq!(progress.rank(1), 2);

    // The last leg has no next leg to advance to.
    assert!(!progress.advance());
    assert!(progress.is_last_leg());
}

#[test]
fn a_craft_that_never_finishes_a_leg_scores_nothing_for_it() {
    let mut progress = Progress::new(vec!["leg_a.vex".to_string()]);
    progress.record_leg(&board(&[(0, 1, true), (1, 3, false)]));
    assert_eq!(progress.points(0), 8);
    assert_eq!(
        progress.points(1),
        0,
        "did not finish - a race state 7/destroyed craft scores 0"
    );
}

#[test]
fn recording_the_same_leg_twice_does_not_double_count() {
    let mut progress = Progress::new(vec!["leg_a.vex".to_string(), "leg_b.vex".to_string()]);
    progress.record_leg(&board(&[(0, 1, true)]));
    progress.record_leg(&board(&[(0, 1, true)]));
    assert_eq!(
        progress.points(0),
        8,
        "the finish transition can re-enter a tick"
    );
}
