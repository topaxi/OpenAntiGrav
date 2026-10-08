use oag_tables::race_campaign::{Cell, Difficulty, Mode};

use super::{RaceExit, race_exit};

fn cell(mode: Mode) -> Cell {
    Cell {
        name: "gridX_1_1".to_string(),
        track: Some("18_Track".to_string()),
        mode,
        class: "Flash".to_string(),
        weapons: false,
        damage: true,
        locked: None,
        status: None,
        ai_count: None,
        skill: None,
        skill_easy: None,
        skill_hard: None,
        laps: Some(4),
        ship: None,
        ship_choice: None,
        gold: 1,
        silver: 0,
        bronze: 0,
        tournament_tracks: Vec::new(),
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

/// Every campaign mode leaves to `Cell Selection`: the original's pause rows
/// all end at `Show Unlocks`, whose redirect sends a campaign launch there.
#[test]
fn a_campaign_race_of_any_mode_leaves_to_its_cell() {
    for mode in [
        Mode::Race,
        Mode::TimeTrial,
        Mode::SpeedLap,
        Mode::Zone,
        Mode::Elimination,
        Mode::Tournament,
        Mode::Head2Head,
    ] {
        assert_eq!(
            race_exit(Some(&cell(mode.clone())), Some(Difficulty::Hard)),
            RaceExit::CellSelection {
                cell: "gridX_1_1".to_string(),
                difficulty: Some(Difficulty::Hard),
            },
            "{mode:?}"
        );
    }
}

#[test]
fn a_race_with_no_cell_leaves_to_the_menus() {
    assert_eq!(race_exit(None, None), RaceExit::Menus);
}
