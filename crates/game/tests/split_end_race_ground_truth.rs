//! A two-person split race driven to its end on a real circuit: the view of the
//! person who finishes first, then the board both people end on.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game split_end_race_ground_truth:: --run-ignored all
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
    oag_game::split_screen::apply(&mut race, 2, oag_render::viewport::Cut::Horizontal);
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

    // The two-person screens' own feed: each person's laps off their own craft, the place the
    // board gave them, and the speedup-pad tally for the first craft alone. This is the
    // library half of `Session::build_endrace`'s split branch (the binary-crate half, the
    // `ResultsModel::PulseSplit` choice and the loyalty banking, is verified by the live walk).
    let boosts = race.run_stats().boosts_by_lap;
    let people = oag_game::results_tables::split_people(board, &race.sim.world, &boosts);
    assert_eq!(people.len(), 2, "two people, seat order");
    assert_eq!((people[0].slot, people[1].slot), (0, 1));
    let laps = board.laps_target.expect("a race with a lap target") as usize;
    for person in &people {
        assert_eq!(person.laps.len(), laps, "slot {}'s own splits", person.slot);
        let row = board
            .rows
            .iter()
            .find(|row| usize::from(row.slot) == person.slot);
        assert_eq!(person.place, row.map(|row| row.place), "the board's place");
        assert!(person.total_ticks > 0);
    }
    assert!(people[0].laps.iter().all(|lap| lap.boosts.is_some()));
    assert!(
        people[1].laps.iter().all(|lap| lap.boosts.is_none()),
        "the tally is slot 0's"
    );

    // Drawn through the PSP's borrowed `Results Split`: each person's table is there with
    // their headline, header row and totals row.
    let source = image.display().to_string();
    let mut archives = oag_source::title::open_source(&source, Vec::new(), Vec::new())
        .expect("the disc opens")
        .archives;
    let space = oag_display::space::Space::PSP;
    let screens = oag_game::endrace::load(
        &mut archives,
        &oag_ui::language::StringTable::default(),
        oag_ui_screens::picker::FaceScales::default(),
        [space.size.0, space.size.1],
        &oag_hud::sprite::Sheet::default(),
        &[],
        oag_pulse::TITLE,
    )
    .expect("Pulse's EndRace screens read");
    let layout = screens
        .results_split
        .as_ref()
        .expect("the PSP borrows Results Split");
    let results: Vec<_> = people
        .iter()
        .map(|person| {
            person.results(oag_ui_screens::endrace::Headline::Position(
                person.place.expect("a field of more than one"),
            ))
        })
        .collect();
    let skin = oag_ui::menu::Skin::new(oag_pulse::FRONT_END.menu.unwrap(), space, 22.0);
    let layers = oag_ui_screens::endrace::split_results_draw_list(
        &results,
        layout,
        &skin,
        &oag_ui::menu::Frame::default(),
        &oag_ui::language::StringTable::default(),
        None,
        false,
        &|_| None,
    );
    let texts: Vec<&str> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let count = |wanted: &str| texts.iter().filter(|text| **text == wanted).count();
    assert_eq!(count("PRO_STATS_TOT"), 2, "a totals row each: {texts:?}");
    assert_eq!(count("RC_LAP"), 2, "a header row each: {texts:?}");
    for person in &people {
        let id = format!(
            "ER_{}",
            ["1ST", "2ND", "3RD", "4TH", "5TH", "6TH", "7TH", "8TH"]
                [usize::from(person.place.unwrap()) - 1]
        );
        assert!(
            texts.iter().any(|text| *text == format!("{id}P")),
            "slot {}'s headline {id}P in {texts:?}",
            person.slot
        );
    }
}
