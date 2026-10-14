//! Pulse PSP's Multiplayer Time Trial on a real circuit: a Time Trial with two or more people
//! is those people and nobody else, they pass through each other, and the board they end on
//! is ranked by total time and names each of them.
//!
//! The rules are measured (`docs/ghidra/functions/psp-pulse-usa/mp-time-trial.md`); that the
//! head count switches them on in Time Trial rather than a menu mode of its own is **chosen**.
//! The synthetic half is `oag_raceplay`'s `tests::multiplayer_time_trial`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game mp_time_trial_ground_truth:: --run-ignored all
//! ```

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// Ticks before a race is called stuck.
const CAP: u64 = 30_000;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";

/// The circuit's race of `mode` with `people` people, the way `--players` builds it: the
/// opponents hatch on, which the multiplayer rules must override.
fn people_race(mode: oag_race::Mode, people: usize) -> Option<race::Race> {
    let image = oag_testdata::image(IMAGE)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        views: u8::try_from(people).unwrap(),
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    oag_game::split_screen::apply(&mut race, people, oag_render::viewport::Cut::Horizontal);
    Some(race)
}

/// The people fly every craft there is: two on the grid for two people, eight for eight, and
/// never an AI craft beside them (the hatch is on). One person is a lone craft as before.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_grid_is_the_people_and_nobody_else() {
    for people in [2, 5, 8] {
        let Some(race) = people_race(oag_race::Mode::TimeTrial, people) else {
            return;
        };
        let world = &race.sim.world;
        assert_eq!(usize::from(world.ship_count), people, "{people} people");
        assert_eq!(world.human_slots().count(), people, "all of them people");
        let mut rows: Vec<f32> = (0..people)
            .map(|slot| world.ships[slot].physics.body.position.length())
            .collect();
        rows.dedup();
        assert_eq!(rows.len(), people, "each on a grid spot of their own");
    }
}

/// Two craft put on top of each other and left to hover: a Multiplayer Time Trial's stay put
/// where they were, and Head to Head's (which collide) are pushed apart. The same load, the
/// same offset and the same ticks, so only the rule differs.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn craft_pass_through_each_other() {
    let apart_after_ten_ticks = |mode| -> Option<f32> {
        let mut race = people_race(mode, 2)?;
        let there = race.sim.world.ships[0].physics.body.position;
        race.sim.world.ships[1].physics.body.position = there + Vec3::new(0.0, 0.0, 1.0);
        for _ in 0..10 {
            race.tick(&PlayerInputs::none());
        }
        let world = &race.sim.world;
        Some(
            world.ships[0]
                .physics
                .body
                .position
                .distance(world.ships[1].physics.body.position),
        )
    };
    let Some(through) = apart_after_ten_ticks(oag_race::Mode::TimeTrial) else {
        return;
    };
    let bounced = apart_after_ten_ticks(oag_race::Mode::Head2Head).unwrap();
    assert!(
        through < 1.5,
        "a Multiplayer Time Trial's craft were pushed apart: {through}"
    );
    assert!(
        bounced > through + 0.5,
        "Head to Head's craft collide, so they separate ({bounced}) where Time Trial's do not ({through})"
    );
}

/// Two people fly the lap count out: the board has exactly their two rows, ranked by total
/// time, and the table names both by seat.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_board_is_ranked_by_total_time_and_names_both_people() {
    let Some(mut race) = people_race(oag_race::Mode::TimeTrial, 2) else {
        return;
    };
    race.set_autopilot(true);
    while !race.finished() && race.sim.world.tick < CAP {
        race.tick(&PlayerInputs::none());
    }
    assert!(race.finished(), "still running after {CAP} ticks");
    let board = race.results().expect("both people finished");
    assert_eq!(
        board.rows.len(),
        2,
        "two craft, two rows: no AI on the board"
    );
    assert_eq!(board.persons(), 2);
    let times: Vec<Option<u64>> = board.rows.iter().map(|row| row.finish_tick).collect();
    assert!(
        times.iter().all(Option::is_some),
        "both finished: {times:?}"
    );
    assert!(
        times[0] <= times[1],
        "rows run from the lowest total time up: {times:?}"
    );
    let places: Vec<u8> = board.rows.iter().map(|row| row.place).collect();
    assert_eq!(places, [1, 2]);

    let people = oag_game::results_tables::people_rows(Some(board), race.slot_teams())
        .expect("a board gives a table");
    let named: Vec<_> = people.rows.iter().map(|row| row.name.clone()).collect();
    for seat in [1, 2] {
        assert!(
            named.contains(&oag_ui_screens::endrace::PersonName::Player(seat)),
            "seat {seat} is on the table: {named:?}"
        );
    }
    assert_eq!(named.len(), 2, "nobody but the two people: {named:?}");
}
