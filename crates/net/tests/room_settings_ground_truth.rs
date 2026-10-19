//! Omega's room rows on a real circuit: what `oag_net::race::prepare` does with a room's barrel
//! rolls, Pilot Assist list and zone target, on both ends of a race.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image (ADR-0006).

use oag_net::wire::RaceParams;
use oag_physics::pilot_assist::Level;

const IMAGE: &str = "pulse-psp-eu.chd";

fn params() -> RaceParams {
    RaceParams {
        title: String::new(),
        track: "Data\\Environments\\01_Track\\track.vex".into(),
        team: String::new(),
        teams: Vec::new(),
        class: "VENOM".into(),
        mode: "zone".into(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        seed: 0x5EED,
        people: 2,
        opponents: false,
        weapons: false,
        laps: 0,
        no_barrel_rolls: false,
        assists: Vec::new(),
        zone_target: 0,
    }
}

/// Each setting reaches the race `prepare` builds, and the zone target ends the Zone race on the
/// zone it names while the same race with none runs on. A target is also in the state hash, so
/// two ends that disagree about it differ at tick 0.
#[test]
#[ignore = "needs a disc image"]
fn a_rooms_rolls_assists_and_zone_target_reach_the_race() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let source = image.display().to_string();
    let none = oag_gameplay::PlayerInputs::none();

    let (plain, _) = oag_net::race::load(&params(), &source).expect("loads");
    assert!(plain.barrel_rolls());
    assert_eq!(plain.pilot_assist_level_of(0), Level::Off);

    let room = RaceParams {
        no_barrel_rolls: true,
        assists: vec![2, 1],
        zone_target: 1,
        ..params()
    };
    let (mut race, _) = oag_net::race::load(&room, &source).expect("loads");
    assert!(!race.barrel_rolls(), "rolls are off");
    assert_eq!(
        (race.pilot_assist_level_of(0), race.pilot_assist_level_of(1)),
        (Level::Extreme, Level::Normal),
        "each seat flies its own level"
    );
    assert_ne!(
        oag_net::race::fingerprint(&race),
        oag_net::race::fingerprint(&plain),
        "a disagreement about the target is seen at tick 0"
    );

    // The countdown and one 600 tick zone, with a margin: the target ends the race.
    let mut ended = None;
    let (mut free, _) = oag_net::race::load(&params(), &source).expect("loads");
    for tick in 0..2_400 {
        race.tick_sim_only(&none);
        free.tick_sim_only(&none);
        if ended.is_none() && race.sim.world.race[0].finished {
            ended = Some(tick);
        }
    }
    let ended = ended.expect("the Zone race ended at its target");
    assert!(
        (600..2_400).contains(&ended),
        "ended at tick {ended}, after the first zone and not before"
    );
    assert!(
        !free.sim.world.race[0].finished,
        "with no target the Zone race runs on"
    );
}
