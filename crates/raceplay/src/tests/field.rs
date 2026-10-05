//! A whole grid rather than one craft: separation, who owns what, and the AI
//! drivers that fly the other seven.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`.

use super::*;
use oag_gameplay::PlayerInputs;

/// Craft used to pass through each other, which is what this catches.
///
/// Two craft are overlapped and closing, and the pass has to separate them.
/// `setup`'s craft has no force law at all, which is what makes this a clean
/// test of the pair resolver alone: nothing else here can move a body, so
/// any separation is the resolver's.
#[test]
fn two_overlapping_craft_are_pushed_apart() {
    let mut race = race_with_a_grid();
    let where_it_was = race.sim.world.ships[0].physics.body.position;
    // Well inside the hull radius, and **not** coincident: two bodies at
    // exactly one point have no normal, which `pair::overlap` refuses.
    race.sim.world.ships[1].physics.body.position = where_it_was + Vec3::new(0.0, 0.0, 1.0);
    // Closing, or the separating gate refuses the pair.
    race.sim.world.ships[0].physics.body.linear_velocity = Vec3::new(0.0, 0.0, 8.0);
    race.sim.world.ships[1].physics.body.linear_velocity = Vec3::new(0.0, 0.0, -8.0);
    let before = race.sim.world.ships[0]
        .physics
        .body
        .position
        .distance(race.sim.world.ships[1].physics.body.position);

    race.resolve_craft_pairs();

    let after = race.sim.world.ships[0]
        .physics
        .body
        .position
        .distance(race.sim.world.ships[1].physics.body.position);
    assert!(
        after > before,
        "two overlapping craft went from {before:.3} apart to {after:.3}"
    );
    assert!(
        race.sim.world.ships[0].physics.body.linear_velocity.z < 8.0,
        "the closing craft was not slowed"
    );
}

/// Two bodies at exactly one point have no normal to push along. Refused
/// rather than divided by, and the pass must survive it: a spawn bug that
/// stacked two craft should not take the process down.
#[test]
fn coincident_craft_are_refused_rather_than_dividing_by_zero() {
    let mut race = race_with_a_grid();
    let where_it_was = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[1].physics.body.position = where_it_was;
    race.resolve_craft_pairs();
    assert!(race.sim.world.ships[0].physics.body.position.is_finite());
    assert!(race.sim.world.ships[1].physics.body.position.is_finite());
}

/// The pair pass must not touch craft that are nowhere near each other, or
/// a grid would shove itself apart on the start line.
#[test]
fn craft_that_are_not_touching_are_left_alone() {
    let mut race = race_with_a_grid();
    let before: Vec<_> = race
        .sim
        .world
        .ships
        .iter()
        .map(|ship| ship.physics.body.position)
        .collect();
    race.resolve_craft_pairs();
    for (slot, was) in before.iter().enumerate() {
        assert_eq!(
            race.sim.world.ships[slot].physics.body.position, *was,
            "slot {slot} was moved by a contact it is not in"
        );
    }
}

/// The wiring: every opponent is stepped, with controls a driver chose.
///
/// **What this cannot assert, and why.** [`setup`] builds a craft on
/// [`Handling::ZERO`] with an empty [`CollisionWorld`] - no engine
/// coefficient, no gravity, nothing to hover on - so no force law runs and
/// nobody here *moves*, whatever they hold. Inventing an engine to make them
/// move would be asserting this project's own arithmetic against itself.
///
/// So the claim is the one this level actually owns: the controls a driver
/// chose **reached the physics**. `ShipState::thrust` is written from the
/// input every step, on the `0..=100` scale
/// ([`oag_physics::controls::CONTROL_RANGE`]), and it is `0` on a craft that
/// was spawned and then never stepped - which is what this engine did until
/// **The line that would silently make opponents shoot themselves.** The
/// player's firing path passes owner `0` because the player is slot 0; an
/// opponent doing the same puts rockets in the air owned by the player,
/// which `projectile::step` flies through the player and detonates on
/// whoever actually fired them.
#[test]
fn an_opponents_rocket_is_owned_by_the_slot_that_fired_it() {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(one_rocket_table());
    // Whether to fire is `Race::opponent_fires`'s question, asked by the
    // caller; this asks only who owns what leaves the rails.
    let firing = 3usize;
    assert!(
        race.fire_opponent_rocket(firing),
        "nothing was fired to check the owner of"
    );

    let owners: Vec<u8> = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind.is_some())
        .map(|p| p.owner)
        .collect();
    assert!(!owners.is_empty(), "nothing was put in the air");
    for owner in owners {
        assert_eq!(
            owner as usize, firing,
            "a rocket fired by slot {firing} is owned by slot {owner}"
        );
    }
}

/// `Standing::distance` needs a closed ring to mean anything, and a guessed
/// gap would put a craft half a lap away in the mirror.
#[test]
fn a_craft_on_a_track_with_no_ring_sees_an_empty_field() {
    let mut race = race_with_a_grid();
    race.sim.course = None;
    for slot in 0..8 {
        assert_eq!(
            race.field_for(slot, &race.places()),
            oag_ai::Field::EMPTY,
            "slot {slot}"
        );
    }
}

/// A craft cannot be its own rival, and an inactive slot is not on the
/// track at all.
#[test]
fn the_field_a_driver_sees_never_includes_itself() {
    let race = race_with_a_grid();
    for slot in 0..8 {
        let field = race.field_for(slot, &race.places());
        for rival in [field.ahead, field.behind, field.alongside]
            .into_iter()
            .flatten()
        {
            assert_ne!(rival.slot as usize, slot, "slot {slot} saw itself");
            assert!(
                race.sim.world.ships[rival.slot as usize].active,
                "slot {slot} saw an inactive craft"
            );
        }
    }
}

/// The three channels are exclusive, and each is the nearest of its kind.
#[test]
fn the_field_a_driver_sees_orders_rivals_by_how_far_round_they_are() {
    let race = race_with_a_grid();
    for slot in 0..8 {
        let field = race.field_for(slot, &race.places());
        if let Some(ahead) = field.ahead {
            assert!(ahead.gap >= 0.0, "slot {slot}: an 'ahead' rival is behind");
        }
        if let Some(behind) = field.behind {
            assert!(behind.gap < 0.0, "slot {slot}: a 'behind' rival is ahead");
        }
        // A craft counted alongside is in neither of the other two.
        if let Some(alongside) = field.alongside {
            for other in [field.ahead, field.behind].into_iter().flatten() {
                assert_ne!(other.slot, alongside.slot, "slot {slot}: counted twice");
            }
        }
        // Nothing beyond the horizon.
        for rival in [field.ahead, field.behind, field.alongside]
            .into_iter()
            .flatten()
        {
            assert!(rival.gap.abs() < oag_ai::AWARENESS_RANGE, "slot {slot}");
        }
    }
}

/// On a grid the whole field is stacked within a couple of hull lengths, so
/// every craft has somebody to see - which is what makes the assertions
/// above worth making.
#[test]
fn a_craft_on_the_grid_can_see_somebody() {
    let race = race_with_a_grid();
    let seen = (0..8)
        .filter(|&slot| {
            let field = race.field_for(slot, &race.places());
            field.ahead.is_some() || field.behind.is_some() || field.alongside.is_some()
        })
        .count();
    assert_eq!(seen, 8, "only {seen} of eight craft could see a rival");
}

/// the AI landed. That the field then goes somewhere is
/// `race_ground_truth::the_ai_drives_the_field_along_the_track`, on real
/// geometry, where it can be true.
///
/// # This fixture has a horizon, and it is [`STALL_TICKS`] long
///
/// `race_with_a_grid` builds its craft on `Handling::ZERO` with an empty
/// collision world, so **no force law runs** and the craft hold full throttle
/// without ever moving. That is the exact condition the stall rescue exists to
/// catch, so running this fixture past [`STALL_TICKS`] recovers all seven
/// opponents and zeroes the throttle on the tick it does - a real failure of the
/// fixture, not of either mechanism. Any synthetic race test that wants to run
/// longer than this has to give its craft handling that moves them.
///
/// The run also has to clear [`oag_race::COUNTDOWN_TICKS`] first: opponents are
/// held at zero thrust through the start-line countdown exactly like the
/// player, per `oag_race::RaceState::thrust_gated`, and `stalled` only starts
/// counting a stopped craft once its commanded thrust reads above zero - so the
/// gated span costs nothing against the [`STALL_TICKS`] horizon above.
#[test]
fn the_opponents_are_driven_rather_than_parked() {
    let mut race = race_with_a_grid();
    assert_eq!(race.ship_count(), 8);
    for _ in 0..oag_race::COUNTDOWN_TICKS + u64::from(STALL_TICKS / 2) {
        race.tick(&PlayerInputs::none());
    }

    for slot in 1..8 {
        let ship = &race.sim.world.ships[slot];
        assert_eq!(
            ship.physics.thrust,
            oag_physics::controls::CONTROL_RANGE,
            "opponent {slot} is holding no throttle, so it is not being stepped"
        );
        assert!(
            ship.physics.body.position.is_finite(),
            "opponent {slot} left the world"
        );
    }
}

/// An opponent that leaves the circuit is put back on it.
///
/// **The failure this pins is not hypothetical.** Before it, an opponent
/// that came off receded from the track at racing speed for the rest of the
/// race, touching no `Reset` volume because there is none out in open
/// space, and seven of the disc's twelve circuits never saw a completed lap
/// as a result. See [`RESCUE_HALF_WIDTHS`].
#[test]
fn an_opponent_that_flies_off_the_circuit_is_put_back_on_it() {
    let mut race = race_with_a_grid();
    race.tick(&PlayerInputs::none());
    assert_eq!(race.respawns_of(1), 0);

    // Straight up and far away, which no reset volume in this fixture
    // covers - the point being that geometry cannot catch this.
    let away = race.sim.world.ships[1].physics.body.position + Vec3::Y * 100_000.0;
    race.sim.world.ships[1].physics.body.position = away;

    // Not on the first tick: a craft is only lost once it stays lost, or a
    // leap over a gap would teleport it mid-flight.
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.respawns_of(1),
        0,
        "one tick away is a jump, not a craft that is gone"
    );

    // Held out there until it is recovered, and then let go: putting it back
    // out on the tick after the rescue would measure the shove, not the
    // recovery.
    for _ in 0..RESCUE_TICKS {
        if race.respawns_of(1) > 0 {
            break;
        }
        race.sim.world.ships[1].physics.body.position = away;
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.respawns_of(1), 1, "the craft was never recovered");

    // Back on the line, and - the part that is easy to get wrong - its
    // driver knows where it was put. A driver left pointing at the old index
    // steers at the piece of track the craft fell off.
    let ship = &race.sim.world.ships[1];
    let residual = ship
        .physics
        .body
        .position
        .distance(race.line_of(1).point(ship.driver.index as usize));
    assert!(
        residual < race.sim.rescue_distance,
        "recovered {residual} from where its driver thinks it is"
    );

    // And nobody else was touched.
    for slot in [0usize, 2, 3] {
        assert_eq!(race.respawns_of(slot), 0, "craft {slot} was recovered too");
    }
}

/// Giving up on one craft must not give up on the rest of them.
///
/// The counters were a single set until opponents could respawn, so one
/// opponent wedged in a corner would have exhausted [`RESPAWN_GIVE_UP`] and
/// switched off the *player's* recovery for the remainder of the race.
#[test]
fn giving_up_on_one_craft_leaves_the_others_recoverable() {
    let mut race = race_with_a_grid();
    race.sim.respawn_disabled[1] = true;
    race.sim.respawns_in_a_row[1] = RESPAWN_GIVE_UP;

    let away = race.sim.world.ships[2].physics.body.position + Vec3::Y * 100_000.0;
    for _ in 0..=RESCUE_TICKS {
        race.sim.world.ships[1].physics.body.position += Vec3::Y * 100_000.0;
        race.sim.world.ships[2].physics.body.position = away;
        race.tick(&PlayerInputs::none());
    }

    assert_eq!(race.respawns_of(1), 0, "a craft given up on was recovered");
    assert_eq!(
        race.respawns_of(2),
        1,
        "giving up on craft 1 switched off craft 2's recovery"
    );
    assert!(
        !race.sim.respawn_disabled[0],
        "the player's was switched off"
    );
}

/// A wreck must not go on racing. A single race runs with `Damage` on, so an
/// opponent can empty its pool, and `oag_physics::step` integrates whatever
/// it is handed - which before this was a destroyed craft at full throttle.
#[test]
fn a_destroyed_opponent_stops_driving() {
    let mut race = race_with_a_grid();
    // Past the start-line countdown first - see `oag_race::RaceState::thrust_gated`
    // - so the opponent is actually driving, not just still held at the line,
    // when the first assertion below checks its throttle. `COUNTDOWN_TICKS`
    // calls still land on the last gated tick; one more releases it.
    for _ in 0..=oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(
        race.sim.world.ships[1].physics.thrust,
        oag_physics::controls::CONTROL_RANGE
    );

    race.sim.world.ships[1].physics.craft_state = oag_physics::CraftState::Destroyed;
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.sim.world.ships[1].physics.thrust, 0.0,
        "a destroyed opponent is still holding throttle"
    );
    // The rest of the field is unaffected: this is per craft, not a mode.
    assert_eq!(
        race.sim.world.ships[2].physics.thrust,
        oag_physics::controls::CONTROL_RANGE
    );
}

/// A driver has to *find* itself on the line, or the windowed search it seeds
/// drifts away from the craft it belongs to.
///
/// The grid puts every opponent ahead of the player along the straight, and
/// the line is sampled every 2.5 units, so a driver that located itself
/// honestly is not still standing on point zero - even with no force law to
/// move it there.
#[test]
fn every_driver_locates_itself_on_the_line() {
    let mut race = race_with_a_grid();
    race.tick(&PlayerInputs::none());
    for slot in 1..8 {
        assert!(
            race.sim.world.ships[slot].driver.index > 0,
            "opponent {slot}'s driver is still on the line's first point"
        );
    }
}

/// The player is not driven by the AI, whatever else is on the grid. A
/// released snapshot has to stay a released snapshot for slot 0, or the human
/// has lost control of their own craft.
#[test]
fn the_ai_never_touches_the_players_craft() {
    let mut race = race_with_a_grid();
    for _ in 0..120 {
        race.tick(&PlayerInputs::none());
    }
    let player = &race.sim.world.ships[0];
    assert_eq!(player.physics.thrust, 0.0);
    assert_eq!(player.driver, oag_ai::Driver::default());
}
