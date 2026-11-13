//! Omega's 2048 campaign events bank XP through the path 2048's do: `load_event` carries the
//! event's `xp_awards`/`xp_column` out of Omega's own `XPAwards.xml`, the race arms a tally from
//! them, and `Store::bank_event_xp` adds the authored amount to the profile's shared total.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/omega_event_xp_ground_truth::/)'
//! ```

use oag_game::endrace::touch::Standing;
use oag_game::records;
use oag_raceplay as race;
use oag_tables::xp::{Column, award};

const EVENT: &str = "2048 - Event 1";

fn source() -> Option<String> {
    oag_testdata::exact("data/extracted/ps4").map(|path| path.display().to_string())
}

fn options(source: &str) -> race::Options {
    race::Options {
        source: source.to_string(),
        ..race::Options::default()
    }
}

/// The event launch carries Omega's own award table and the event's mode column, and resolves
/// the circuit under Omega's `Environments2048`, not 2048's published-environments directory.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn an_omega_event_launch_carries_omegas_award_table() {
    let Some(source) = source() else { return };
    let loaded = race::load_event(&options(&source), EVENT).expect("the event loads");
    let progress = loaded.campaign_2048_event.expect("an event fills this");
    let mut archives = oag_omega::open(&source).expect("open the omega source");
    let (awards, _) = oag_2048::campaign::xp_tables(&mut archives).expect("both XP tables");
    assert_eq!(progress.xp_awards.as_ref(), Some(&awards));
    assert_eq!(progress.xp_column, Column::Race);
}

/// One finished Omega event raises `Store::xp` by the authored amount: its pass award and
/// `Finished Nth`, plus whatever the race's own tally paid. The race is run to its end under
/// the autopilot; the expected values are read from Omega's parsed table.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn a_finished_omega_event_raises_the_stored_xp_by_the_authored_award() {
    let Some(source) = source() else { return };
    let loaded = race::load_event(&options(&source), EVENT).expect("the event loads");
    let progress = loaded
        .campaign_2048_event
        .clone()
        .expect("an event fills this");
    let awards = progress.xp_awards.clone().expect("Omega's table");
    let mut run = race::Race::start(loaded.setup);
    run.arm_event_xp(&progress);
    run.set_autopilot(true);
    while !run.finished() && run.sim.world.tick < 30_000 {
        run.tick(&oag_gameplay::PlayerInputs::none());
    }
    assert!(run.finished(), "the event did not finish in 30000 ticks");
    let facts = oag_game::endrace::touch::Facts::from_race(&run, progress.objectives);
    let Standing::Place(place) = facts.standing else {
        panic!("a single race comes to a place, not {:?}", facts.standing);
    };

    let (mut store, _) = records::parse("").expect("an empty file parses");
    assert_eq!(store.xp(), 0);
    let in_race = run.xp_tally_mut().expect("armed").total();
    let paid = store.bank_event_xp(
        run.xp_tally_mut().expect("armed"),
        None,
        Some(records::Medal::Bronze),
        Some(place),
    );
    let pass = awards.xp(award::EVENT_PASS, progress.xp_column);
    let nth = awards.xp(
        award::FINISHED_1ST + u32::from(place) - 1,
        progress.xp_column,
    );
    assert!(pass > 0 && nth > 0, "the table authors both rows");
    assert_eq!(store.xp(), paid);
    assert!(paid >= pass + nth, "{paid} < {pass} + {nth}");
    if in_race == 0 {
        assert_eq!(paid, pass + nth);
    }
}
