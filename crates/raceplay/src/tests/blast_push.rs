//! The title's blast push, end to end through `Race::tick`: a Rocket that strikes the player
//! either posts its push for the craft's physics step to drain (a stun, Pulse's) or changes
//! its velocity at once (HD's).

use super::*;
use oag_gameplay::PlayerInputs;
use oag_title::weapons::BlastPush;

/// A race past its countdown with a Rocket fired from rival slot 1 straight into the player,
/// ticked until it has struck, under `push`.
fn player_struck_by_a_rocket(push: BlastPush) -> Race {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    race.sim.blast_push = push;
    for _ in 0..oag_race::COUNTDOWN_TICKS + 2 {
        race.tick(&PlayerInputs::none());
    }
    let body = race.sim.world.ships[0].physics.body;
    let before = race.sim.world.ships[0].physics.shield;
    race.sim.world.projectiles.spawn(
        oag_tables::weapons::Weapon::Rocket,
        body.position - body.forward() * 3.0,
        body.forward() * 600.0,
        1,
    );
    for _ in 0..3 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        race.sim.world.ships[0].physics.shield < before,
        "the rocket never struck the player - not what this test means to check"
    );
    race
}

/// Pulse's law: the push is posted and drained by the craft's own step, which arms the
/// collision stun. Dropping the wiring (the match in `Race::tick`, the `Setup` field) leaves
/// the craft unstunned.
#[test]
fn a_posted_blast_stuns_the_player_it_strikes() {
    let race = player_struck_by_a_rocket(BlastPush::Posted);
    assert!(
        race.sim.world.ships[0].physics.stun_timer > 0.0,
        "a posted blast armed no stun"
    );
}

/// The other law: the velocity changes at once and nothing is stunned.
#[test]
fn an_immediate_blast_shoves_the_player_without_a_stun() {
    let race = player_struck_by_a_rocket(BlastPush::Immediate);
    let physics = &race.sim.world.ships[0].physics;
    assert_eq!(physics.stun_timer, 0.0, "an immediate blast stunned");
    assert!(
        physics.body.linear_velocity.length() > 0.0,
        "an immediate blast did not move the player"
    );
}
