//! An Eliminator with the player parked, on a real circuit out of a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! The question it answers is the one a player asks: *does the mode end?* With
//! the player parked, the eight-craft field has to fight to a finish by
//! itself, which needs three things wired at once - weapons that reach the
//! field, kills credited when a weapon finishes a craft, and a kill target the
//! event ends on.

use oag_game::race;
use oag_gameplay::PlayerInputs;

/// Six game-minutes at 60 Hz: past that a run is a wait, not a measurement.
const TICKS: u64 = 60 * 60 * 6;

fn loaded_eliminator(kill_target: Option<u32>) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        eliminator_kill_target: kill_target,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// The field, left alone, credits kills to its craft and reaches a finish.
///
/// A target of 2 rather than the original's 5, because this build's opponents
/// are measured as far less lethal than the original's (see
/// `docs/gameplay/race-modes.md`, "The Eliminator finish"): five is a long wait
/// here. What it proves is the wiring - a weapon's kill is counted, the count
/// is compared with the target, and the race ends - which fails if any of the
/// three is dropped.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_parked_player_eliminator_reaches_a_finish() {
    let Some(mut race) = loaded_eliminator(Some(2)) else {
        return;
    };
    let parked = PlayerInputs::single(oag_gameplay::InputSnapshot::new());
    let mut ended = None;
    for tick in 0..TICKS {
        race.tick(&parked);
        if race.sim.world.primary_race().finished {
            ended = Some(tick);
            break;
        }
    }
    let kills: Vec<u32> = (0..race.sim.world.ship_count as usize)
        .map(|slot| race.sim.world.ships[slot].standing.kills)
        .collect();
    println!("ended at tick {ended:?}, kills {kills:?}");
    assert!(
        ended.is_some(),
        "no craft reached 2 kills in six game-minutes: {kills:?}"
    );
    assert!(
        kills.iter().any(|&k| k >= 2),
        "the race ended without anyone on the target: {kills:?}"
    );
    assert_eq!(kills[0], 0, "the parked player cannot have scored");
}
