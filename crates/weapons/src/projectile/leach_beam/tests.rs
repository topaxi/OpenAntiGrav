//! What the LeachBeam's link, lifetime and transfer in [`super`] are asserted to
//! do. Split under the 200-line cap on inline test modules.

use super::*;
use crate::test_craft::Ship;
use oag_core::math::Vec3;
use oag_physics::DamageRules;
use oag_physics::damage::CraftState;

/// The Race table's own LeachBeam, as parsed off `pulse-psp-usa.chd` - see
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
fn stats() -> LeachBeamStats {
    LeachBeamStats {
        absorb: 10.0,
        lock_min_dist: 10.0,
        lock_max_dist: 200.0,
        damage: 0.1,
        repair: 0.1,
        slow_ship_factor: 0.8,
        range: 250.0,
        active_time: 3.0,
        energy_multiplier: 50.0,
    }
}

fn rules() -> DamageRules {
    DamageRules {
        weapons: true,
        damage: true,
    }
}

/// Two craft, both racing, `apart` units from each other on the x axis, each
/// with a shield pool part-drained so a repair has room to land.
fn field(apart: f32) -> [Ship; MAX_SHIPS] {
    let mut ships: [Ship; MAX_SHIPS] = std::array::from_fn(|_| Ship::default());
    for (index, ship) in ships.iter_mut().enumerate().take(2) {
        ship.active = true;
        ship.handling.dimensions.shield = 100.0;
        ship.physics.shield = 50.0;
        ship.physics.body.position = Vec3::new(apart * index as f32, 0.0, 0.0);
    }
    ships
}

const DT: f32 = 1.0 / 60.0;

#[test]
fn a_locked_beam_carries_its_authored_numbers() {
    let beam = Beam::locked(0, 1, &stats());
    assert_eq!(beam.owner, 0);
    assert_eq!(beam.target, 1);
    assert_eq!(beam.kind, Kind::Locked);
    assert_eq!(beam.range, 250.0);
    assert_eq!(beam.active_time, 3.0);
    assert!(beam.connected());
}

/// The whole of `LeachBeam_InitUnlocked`'s contribution: it does nothing, and
/// then it stops.
#[test]
fn a_beam_fired_with_no_lock_drains_nobody_and_expires_on_its_own_clock() {
    let mut ships = field(50.0);
    let mut beam = Beam::unlocked(0, &stats());
    let before = ships[1].physics.shield;

    let mut ticks = 0;
    loop {
        let report = beam.advance(&mut ships, 2, rules(), DT);
        ticks += 1;
        assert!(!report.drained, "an unlocked beam drained somebody");
        if report.retired {
            break;
        }
        assert!(ticks < 600, "an unlocked beam never expired");
    }

    assert_eq!(ships[1].physics.shield, before);
    // The age, not the tick count: `age` accumulates `dt`, so 45 additions of
    // `1.0 / 60.0` land a hair under `0.75` and the beam retires on the 46th.
    assert!(
        beam.age >= UNLOCKED_FIZZLE_SECONDS && beam.age < UNLOCKED_FIZZLE_SECONDS + DT,
        "fizzled at {} rather than {UNLOCKED_FIZZLE_SECONDS}",
        beam.age
    );
    assert!((44..=46).contains(&ticks), "fizzled after {ticks} ticks");
}

/// `energy_multiplier` is a one-shot on each half, so tick one is fifty times
/// tick two and tick three is the same as tick two.
#[test]
fn the_first_draining_tick_costs_the_multiplier_and_the_rest_do_not() {
    let mut ships = field(50.0);
    let mut beam = Beam::locked(0, 1, &stats());

    let victim_before = ships[1].physics.shield;
    let shooter_before = ships[0].physics.shield;
    beam.advance(&mut ships, 2, rules(), DT);
    let first_taken = victim_before - ships[1].physics.shield;
    let first_given = ships[0].physics.shield - shooter_before;

    let victim_mid = ships[1].physics.shield;
    let shooter_mid = ships[0].physics.shield;
    beam.advance(&mut ships, 2, rules(), DT);
    let second_taken = victim_mid - ships[1].physics.shield;
    let second_given = ships[0].physics.shield - shooter_mid;

    assert!(
        (first_taken - 5.0).abs() < 1e-4,
        "first tick took {first_taken}, not damage * energy_multiplier"
    );
    assert!(
        (first_given - 5.0).abs() < 1e-4,
        "first tick gave {first_given}, not repair * energy_multiplier"
    );
    assert!(
        (second_taken - 0.1).abs() < 1e-4,
        "second tick took {second_taken}, not the bare damage"
    );
    assert!(
        (second_given - 0.1).abs() < 1e-4,
        "second tick gave {second_given}, not the bare repair"
    );
}

/// The transfer is conservative on the Race table, which authors `damage` and
/// `repair` equal - so the field's total energy does not move while a beam
/// holds. Not a law of the weapon (Eliminator authors them apart); a check that
/// this build spends the two attributes on the two craft the right way round.
#[test]
fn what_the_victim_loses_the_shooter_gains_on_the_race_table() {
    let mut ships = field(50.0);
    let mut beam = Beam::locked(0, 1, &stats());
    let total_before = ships[0].physics.shield + ships[1].physics.shield;
    for _ in 0..10 {
        beam.advance(&mut ships, 2, rules(), DT);
    }
    let total_after = ships[0].physics.shield + ships[1].physics.shield;
    assert!(
        (total_before - total_after).abs() < 1e-3,
        "{total_before} became {total_after}"
    );
    assert!(ships[1].physics.shield < ships[0].physics.shield);
}

#[test]
fn a_target_that_pulls_beyond_range_breaks_the_link_and_the_beam_lingers() {
    let mut ships = field(50.0);
    let mut beam = Beam::locked(0, 1, &stats());
    assert!(beam.advance(&mut ships, 2, rules(), DT).drained);

    ships[1].physics.body.position = Vec3::new(300.0, 0.0, 0.0);
    let report = beam.advance(&mut ships, 2, rules(), DT);
    assert!(report.disconnected, "300 units apart and still connected");
    assert!(!report.retired, "retired without lingering");
    assert!(!beam.connected());

    let broken_at = beam.disconnected_at.expect("a broken link records when");
    let mut ticks = 0;
    while !beam.advance(&mut ships, 2, rules(), DT).retired {
        ticks += 1;
        assert!(ticks < 600, "a disconnected beam never retired");
    }
    assert!(beam.age >= broken_at + DISCONNECT_LINGER_SECONDS);
}

/// A beam nobody breaks still lets go on `active_time`.
#[test]
fn a_held_link_lets_go_on_its_authored_active_time() {
    let mut ships = field(50.0);
    let mut beam = Beam::locked(0, 1, &stats());
    let mut drained_ticks = 0;
    loop {
        let report = beam.advance(&mut ships, 2, rules(), DT);
        if report.drained {
            drained_ticks += 1;
        }
        if report.disconnected {
            break;
        }
        assert!(beam.age < 10.0, "a beam outlived its active_time");
    }
    assert!(
        (beam.age - 3.0).abs() <= DT,
        "let go at {} rather than active_time",
        beam.age
    );
    // Every tick before the one that let go drained, none after (the rule, not the
    // accumulation drift, as above).
    assert!(
        (178..=180).contains(&drained_ticks),
        "drained on {drained_ticks} ticks of a three-second beam"
    );
}

/// `LeachBeam_Drain`'s first line: the *shooter*'s own shield stops the whole
/// transfer, not just its own half.
#[test]
fn a_shielded_shooter_leaches_nothing() {
    let mut ships = field(50.0);
    ships[0].physics.shield_pickup_timer = 1.0;
    let mut beam = Beam::locked(0, 1, &stats());
    let victim_before = ships[1].physics.shield;
    let shooter_before = ships[0].physics.shield;
    beam.advance(&mut ships, 2, rules(), DT);
    assert_eq!(ships[1].physics.shield, victim_before);
    assert_eq!(ships[0].physics.shield, shooter_before);
}

/// A shielded *target* is a disconnect, not a swallowed hit - the original
/// tests the target's shield in `LeachBeam_UpdatePool`'s link gate, before it
/// ever reaches the drain.
#[test]
fn a_shielded_target_breaks_the_link() {
    let mut ships = field(50.0);
    ships[1].physics.shield_pickup_timer = 1.0;
    let mut beam = Beam::locked(0, 1, &stats());
    let report = beam.advance(&mut ships, 2, rules(), DT);
    assert!(report.disconnected);
    assert!(!report.drained);
}

/// `Ship_ApplyPendingWeaponDamage`'s `kind == 7` arm: every draining tick
/// arms the victim's one-shot thrust scale with the authored `slowShipFactor`,
/// and the shooter's own throttle is untouched.
#[test]
fn every_draining_tick_arms_the_victims_thrust_scale_and_not_the_shooters() {
    let mut ships = field(50.0);
    let mut beam = Beam::locked(0, 1, &stats());

    for _ in 0..3 {
        // The composition root consumes it between ticks; model that.
        ships[1].pending_thrust_scale = 1.0;
        let report = beam.advance(&mut ships, 2, rules(), DT);
        assert!(report.drained);
        assert_eq!(ships[1].pending_thrust_scale, 0.8);
        assert_eq!(ships[0].pending_thrust_scale, 1.0);
    }
}

/// The arm sits inside the racing gate: a victim already blowing up takes no
/// damage from `Ship_Damage` and no throttle either.
#[test]
fn a_victim_that_is_not_racing_is_not_throttled() {
    let mut ships = field(50.0);
    ships[1].physics.craft_state = oag_physics::damage::CraftState::Destroyed;
    let mut beam = Beam::locked(0, 1, &stats());

    beam.advance(&mut ships, 2, rules(), DT);
    assert_eq!(ships[1].pending_thrust_scale, 1.0);
}

/// `LeachBeam_UpdatePool` breaks the link unless `Ship_State` is 1 for both
/// craft: a kill (Destroyed, then Eliminated) lets go of the beam.
#[test]
fn a_kill_breaks_the_link() {
    for state in [CraftState::Destroyed, CraftState::Eliminated] {
        let mut ships = field(50.0);
        let mut beam = Beam::locked(0, 1, &stats());
        assert!(beam.advance(&mut ships, 2, rules(), DT).drained);
        ships[1].physics.craft_state = state;
        let report = beam.advance(&mut ships, 2, rules(), DT);
        assert!(report.disconnected, "{state:?} target kept the link");
        assert!(!report.drained);
    }
}

/// The same check on the shooter: a beam does not outlive its owner's death.
#[test]
fn a_dead_shooter_breaks_the_link() {
    let mut ships = field(50.0);
    let mut beam = Beam::locked(0, 1, &stats());
    ships[0].physics.craft_state = CraftState::Destroyed;
    assert!(beam.advance(&mut ships, 2, rules(), DT).disconnected);
}

/// A broken link never re-forms: the respawned target is not drained again.
#[test]
fn a_respawned_target_is_not_drained() {
    let mut ships = field(50.0);
    let mut beam = Beam::locked(0, 1, &stats());
    ships[1].physics.craft_state = CraftState::Eliminated;
    assert!(beam.advance(&mut ships, 2, rules(), DT).disconnected);
    ships[1].physics.craft_state = CraftState::Racing;
    let shield = ships[1].physics.shield;
    for _ in 0..10 {
        assert!(!beam.advance(&mut ships, 2, rules(), DT).drained);
    }
    assert_eq!(ships[1].physics.shield, shield);
    assert!(!beam.connected());
}
