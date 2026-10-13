//! A two-person split race driven to its end on a real circuit: the view of the
//! person who finishes first, then the board both people end on.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test split_end_race_ground_truth --run-ignored all
//! ```
//!
//! The synthetic tests (`oag_raceplay`'s `results` tests, `oag_game::results_tables`)
//! write the standings by hand. This one lets `Race::set_autopilot` fly both seats
//! over the line, which needs the circuit's ring and racing line, and reads what the
//! end screens are built from: the board, the rows `Session::build_endrace` makes of
//! it, and the view a person has between their own line and the other's.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// Ticks before the race is called stuck: a little over eight minutes, where both
/// seats crossed after about 7,700 ticks on this circuit.
const CAP: u64 = 30_000;

const FINISHED_LINE: &str = "OAG_SPLIT_FINISHED";

fn finished_line(race: &race::Race, slot: usize) -> bool {
    race.readout_for(slot).messages[oag_hud::messages::SLOTS - 1]
        .as_ref()
        .is_some_and(|line| line.text == FINISHED_LINE)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn two_people_reach_the_end_with_both_results_and_one_view() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        views: 2,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    oag_game::split_screen::apply(&mut race, 2);
    race.set_autopilot(true);

    // Between the first person's line and the second's: the finished person's view
    // holds and says so, the other's does not, and the split is still drawn.
    let mut held = 0;
    while !race.finished() && race.sim.world.tick < CAP {
        race.tick(&PlayerInputs::none());
        race.drain_cues();
        let done: Vec<usize> = (0..2)
            .filter(|&slot| race.sim.world.ships[slot].standing.finished())
            .collect();
        if done.len() == 1 && !race.finished() {
            held += 1;
            let (done, racing) = (done[0], 1 - done[0]);
            assert!(
                finished_line(&race, done),
                "slot {done} crossed and says nothing"
            );
            assert!(
                !finished_line(&race, racing),
                "slot {racing} is still racing"
            );
            assert_eq!(race.views_drawn(), 2, "the split stays until both are done");
        }
    }
    assert!(
        race.finished(),
        "still running after {CAP} ticks: slot 0 on lap {}, slot 1 on lap {}",
        race.sim.world.race[0].lap,
        race.sim.world.race[1].lap
    );
    assert!(
        held > 0,
        "no tick had one person over the line and the other racing"
    );

    // The end: one view over the end screens, the split kept for the records and the
    // restart, and a board that carries both people.
    assert_eq!(race.views_drawn(), 1);
    assert_eq!(race.viewport_count(), 2);
    let board = race.results().expect("both people finished");
    assert_eq!(board.persons(), 2);
    let people = oag_game::results_tables::people_rows(Some(board), race.slot_teams())
        .expect("a board gives a table");
    let named: Vec<_> = people
        .rows
        .iter()
        .filter(|row| row.person)
        .map(|row| row.name.clone())
        .collect();
    for seat in [1, 2] {
        assert!(
            named.contains(&oag_ui_screens::endrace::PersonName::Player(seat)),
            "seat {seat} is on the table, named: {named:?}"
        );
    }
    assert_eq!(named.len(), 2);
    let seats: Vec<u8> = oag_game::results_tables::field_rows(Some(board))
        .iter()
        .filter_map(|row| row.seat)
        .collect();
    assert_eq!(seats.len(), 2, "both people are named on the HD grid too");
}
