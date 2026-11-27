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
    race.core.sim.blast_push = push;
    for _ in 0..oag_race::COUNTDOWN_TICKS + 2 {
        race.tick(&PlayerInputs::none());
    }
    let body = race.core.sim.world.ships[0].physics.body;
    let before = race.core.sim.world.ships[0].physics.shield;
    race.core.sim.world.projectiles.spawn(
        oag_tables::weapons::Weapon::Rocket,
        body.position - body.forward() * 3.0,
        body.forward() * 600.0,
        1,
    );
    for _ in 0..3 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        race.core.sim.world.ships[0].physics.shield < before,
        "the rocket never struck the player - not what this test means to check"
    );
    race
}

/// Pulse's law: the push is posted and drained by the craft's own step, which arms the
/// collision stun. Dropping the wiring (the match in `Race::tick`, the `SimSetup` field) leaves
/// the craft unstunned.
#[test]
fn a_posted_blast_stuns_the_player_it_strikes() {
    let race = player_struck_by_a_rocket(BlastPush::Posted);
    assert!(
        race.core.sim.world.ships[0].physics.stun_timer > 0.0,
        "a posted blast armed no stun"
    );
}

/// The other law: the velocity changes at once and nothing is stunned.
#[test]
fn an_immediate_blast_shoves_the_player_without_a_stun() {
    let race = player_struck_by_a_rocket(BlastPush::Immediate);
    let physics = &race.core.sim.world.ships[0].physics;
    assert_eq!(physics.stun_timer, 0.0, "an immediate blast stunned");
    assert!(
        physics.body.linear_velocity.length() > 0.0,
        "an immediate blast did not move the player"
    );
}

/// Pulse's and HD's law, off each title's own data: the blast is posted (a stun) and its push
/// drains along the player's side axis (`oag_physics::laws::Laws::pending_impulse_on_side`),
/// so a Rocket from straight behind shoves the craft sideways, not ahead. Omega keeps the
/// immediate push, which leaves a blast from behind along forward and so no side speed: the
/// two titles part by the blast's side speed. Reverting Pulse's or HD's
/// `pending_impulse_on_side`, dropping the field from `launch_hover::physics_laws`, or the
/// axis choice in `oag_physics::wall::apply_pending_impulse` turns this red.
#[test]
fn a_posted_blast_shoves_the_player_sideways_on_pulse_and_hd() {
    let side_speed = |title: &oag_title::Title, laws: Option<&oag_title::craft_laws::CraftLaws>| {
        let mut race =
            race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
        race.core.sim.blast_push = title.weapons.blast_push.value;
        race.core.sim.physics_laws.pending_impulse_on_side =
            crate::launch_hover::physics_laws(laws).pending_impulse_on_side;
        for _ in 0..oag_race::COUNTDOWN_TICKS + 2 {
            race.tick(&PlayerInputs::none());
        }
        let body = race.core.sim.world.ships[0].physics.body;
        let before = race.core.sim.world.ships[0].physics.shield;
        race.core.sim.world.projectiles.spawn(
            oag_tables::weapons::Weapon::Rocket,
            body.position - body.forward() * 3.0,
            body.forward() * 600.0,
            1,
        );
        for _ in 0..3 {
            race.tick(&PlayerInputs::none());
        }
        let physics = &race.core.sim.world.ships[0].physics;
        assert!(
            physics.shield < before,
            "the rocket never struck the player"
        );
        if title.weapons.blast_push.value == BlastPush::Posted {
            assert!(physics.stun_timer > 0.0, "a posted blast armed no stun");
        }
        physics.body.linear_velocity.dot(physics.body.right()).abs()
    };
    let hd = side_speed(oag_hd::TITLE, Some(&oag_hd::race::CRAFT_LAWS));
    let pulse = side_speed(oag_pulse::TITLE, None);
    let omega = side_speed(oag_omega::TITLE, Some(&oag_omega::race::CRAFT_LAWS));
    assert!(
        hd > omega + 1.0,
        "HD's side speed {hd} is not the blast's over Omega's {omega}"
    );
    assert!(
        pulse > omega + 1.0,
        "Pulse's side speed {pulse} is not the blast's over Omega's {omega}"
    );
}
