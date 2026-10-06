//! What the energy pool and the cost of a wall in [`super`] are asserted to do. Split out of
//! `damage.rs` under the 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;

fn dimensions(shield: f32) -> Dimensions {
    Dimensions {
        shield,
        ..Dimensions::default()
    }
}

fn state(shield: f32) -> ShipState {
    ShipState {
        shield,
        ..ShipState::default()
    }
}

fn wall(impulse_sum: f32) -> WallResponse {
    WallResponse {
        impulse_sum,
        ..WallResponse::default()
    }
}

/// The headline law: `|p| * 0.05 * 0.7`.
#[test]
fn contact_damage_is_the_impulse_times_the_recovered_pair() {
    let amount = contact_damage(100.0, DamageRules::default());
    assert!(
        (amount - 3.5).abs() < 1e-5,
        "100 units of impulse should cost 3.5 energy, cost {amount}"
    );
}

/// The rule that reads like a bug and is not: no weapons halves *collision*
/// damage. A time trial is exactly this case.
#[test]
fn turning_weapons_off_halves_what_a_wall_costs() {
    let rules = DamageRules {
        weapons: false,
        ..DamageRules::default()
    };
    assert!((contact_damage(100.0, rules) - 1.75).abs() < 1e-5);
}

#[test]
fn reset_fills_the_pool_from_the_hull() {
    let mut s = state(0.0);
    reset(&mut s, &dimensions(300.0));
    assert_eq!(s.shield, 300.0);
}

#[test]
fn a_contact_takes_energy_and_the_pool_stops_at_zero() {
    let d = dimensions(300.0);
    let mut s = state(2.0);
    let report = apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());
    assert_eq!(s.shield, 0.0, "the pool must not go negative");
    assert!(report.depleted, "reaching zero is the signal Zone needs");
    assert!((report.lost - 35.0).abs() < 1e-4);
}

/// `depleted` is an edge. A ship that is already at zero must not re-signal
/// every tick it keeps scraping, or whatever ends the race ends it repeatedly.
#[test]
fn an_already_empty_pool_does_not_re_signal() {
    let d = dimensions(300.0);
    let mut s = state(0.0);
    let report = apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());
    assert!(!report.depleted);
}

/// The critical warning is a crossing, not a level - the original tests the
/// percentage on both sides of the subtraction.
#[test]
fn the_critical_warning_fires_on_the_crossing_and_not_below_it() {
    let d = dimensions(100.0);

    // 25 % -> 15 %, one crossing of the 20 % line.
    let mut s = state(25.0);
    let crossing = apply_contact(&mut s, &d, &wall(10.0 / CONTACT_DAMAGE_SCALE), d_rules());
    assert!(crossing.crossed_critical);

    // 15 % -> 5 %, already below, so no second warning.
    let again = apply_contact(&mut s, &d, &wall(10.0 / CONTACT_DAMAGE_SCALE), d_rules());
    assert!(!again.crossed_critical, "a level test would fire here");
}

fn d_rules() -> DamageRules {
    DamageRules::default()
}

/// Damage off does not skip the subtraction; it floors the result at 20 and
/// regenerates. A test that asserted "no damage" would pin the wrong reading.
#[test]
fn damage_off_still_subtracts_but_cannot_empty_the_pool() {
    let d = dimensions(300.0);
    let rules = DamageRules {
        damage: false,
        ..DamageRules::default()
    };
    let mut s = state(300.0);
    let report = apply_contact(&mut s, &d, &wall(1000.0), rules);
    assert!(report.lost > 0.0, "the subtraction still happens");
    assert_eq!(s.shield, 300.0 - 35.0);

    let mut empty = state(1.0);
    apply_contact(&mut empty, &d, &wall(1000.0), rules);
    assert_eq!(
        empty.shield, REGENERATION_FLOOR,
        "the floor is what makes a craft undestroyable, not a skipped subtraction"
    );
}

#[test]
fn regeneration_ramps_at_four_a_second_and_stops_at_the_maximum() {
    let d = dimensions(50.0);
    let rules = DamageRules {
        damage: false,
        ..DamageRules::default()
    };
    let mut s = state(30.0);
    regenerate(&mut s, &d, rules, 1.0);
    assert_eq!(s.shield, 34.0);

    let mut full = state(49.0);
    regenerate(&mut full, &d, rules, 10.0);
    assert_eq!(full.shield, 50.0);
}

#[test]
fn regeneration_does_nothing_while_damage_is_on() {
    let d = dimensions(300.0);
    let mut s = state(100.0);
    regenerate(&mut s, &d, DamageRules::default(), 1.0);
    assert_eq!(s.shield, 100.0);
}

/// The whole destroyed sequence, in the order the original runs it.
#[test]
fn emptying_the_pool_blows_the_craft_up_and_then_eliminates_it() {
    let d = dimensions(300.0);
    let mut s = state(1.0);
    let report = apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());

    assert!(report.depleted);
    assert_eq!(s.craft_state, CraftState::Destroyed);
    assert_eq!(s.state_timer, DESTROYED_DURATION);

    // The explosion runs for half a second and not a tick less.
    let dt = 1.0 / 60.0;
    for _ in 0..29 {
        advance_state(&mut s, dt);
        assert_eq!(s.craft_state, CraftState::Destroyed);
    }
    advance_state(&mut s, dt);
    assert_eq!(s.craft_state, CraftState::Eliminated);
    assert_eq!(s.state_timer, 0.0);
}

/// `Ship_Damage` refuses outright outside the racing states, so a craft
/// already blowing up cannot be blown up again - and `depleted` cannot fire
/// twice, which is what a mode ending on it depends on.
#[test]
fn a_destroyed_craft_takes_no_further_damage() {
    let d = dimensions(300.0);
    let mut s = state(1.0);
    apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());
    let pool = s.shield;

    let again = apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());
    assert_eq!(
        again,
        Shield::default(),
        "a destroyed craft kept taking hits"
    );
    assert_eq!(s.shield, pool);
    assert_eq!(s.state_timer, DESTROYED_DURATION, "the explosion restarted");
}

/// Nothing advances outside the explosion, so a racing craft's timer cannot
/// drift and an eliminated one stays eliminated.
#[test]
fn only_the_explosion_runs_the_state_timer_down() {
    let mut racing = state(100.0);
    racing.state_timer = 5.0;
    advance_state(&mut racing, 1.0);
    assert_eq!(racing.state_timer, 5.0);

    let mut done = state(0.0);
    done.craft_state = CraftState::Eliminated;
    advance_state(&mut done, 1.0);
    assert_eq!(done.craft_state, CraftState::Eliminated);
}

/// Taking the grid clears the sequence as well as filling the pool - a
/// restart must not leave the previous run's wreck in place.
#[test]
fn reset_puts_a_wrecked_craft_back_in_the_race() {
    let mut s = state(0.0);
    s.craft_state = CraftState::Eliminated;
    s.state_timer = 3.0;
    reset(&mut s, &dimensions(300.0));
    assert_eq!(s.craft_state, CraftState::Racing);
    assert_eq!(s.state_timer, 0.0);
    assert_eq!(s.shield, 300.0);
}

/// The whole of what a fired Shield does: refuse the hit outright, refuse it
/// on the edge that also suppresses the two signals - a shielded craft neither
/// dies nor cries critical - and report the absorb so the shell can flash.
#[test]
fn a_running_shield_refuses_the_hit_and_both_its_signals() {
    let d = dimensions(100.0);
    // Low enough that an unshielded craft would be destroyed outright, so
    // this cannot pass by the hit merely being small.
    let mut s = state(1.0);
    s.shield_pickup_timer = 0.5;

    let report = apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());
    assert_eq!(
        report,
        Shield {
            absorbed: true,
            ..Shield::default()
        },
        "a shielded craft took a hit"
    );
    assert_eq!(s.shield, 1.0, "the pool moved");
    assert_eq!(s.craft_state, CraftState::Racing, "a shielded craft died");
}

/// A shield with nothing to swallow does not flash. The original's flash arm
/// is on the branch a *hit* takes, so a tick with no contact at all must not
/// reach it - otherwise a raised shield strobes for its whole duration.
#[test]
fn a_shield_with_no_contact_reports_no_absorb() {
    let d = dimensions(100.0);
    let mut s = state(100.0);
    s.shield_pickup_timer = 0.5;

    let report = apply_contact(&mut s, &d, &wall(0.0), DamageRules::default());
    assert_eq!(report, Shield::default(), "an untouched shield flashed");
}

/// The timer runs one tick longer than the arithmetic ([`advance_shield_pickup`]) and the tick it
/// expires on takes damage again. The pair pins the ordering in `crate::step`: advancing before
/// `apply_contact` would cost the firing tick, and reading `>= 0.0` would leak a tick at the other
/// end.
#[test]
fn a_fired_shield_refuses_damage_for_its_authored_duration() {
    let d = dimensions(300.0);
    let dt = 1.0 / 60.0;
    let mut s = state(300.0);
    s.shield_pickup_timer = 0.75;

    let mut protected = 0;
    for _ in 0..120 {
        let report = apply_contact(&mut s, &d, &wall(10.0), DamageRules::default());
        if report.lost == 0.0 {
            protected += 1;
        }
        advance_shield_pickup(&mut s, dt);
    }
    assert_eq!(
        protected, 46,
        "0.75 s at 60 Hz protects 46 ticks, not 45 - see advance_shield_pickup"
    );
    assert!(
        s.shield < 300.0,
        "the pool must be spendable once it expires"
    );
}

/// Floored, so a long-expired shield does not drift the determinism hash by
/// an ever-growing negative - the same guarantee `advance_turbo` gives.
#[test]
fn an_expired_shield_stops_at_zero() {
    let mut s = state(100.0);
    s.shield_pickup_timer = 0.01;
    for _ in 0..100 {
        advance_shield_pickup(&mut s, 1.0);
    }
    assert_eq!(s.shield_pickup_timer, 0.0);
}

/// Taking the grid clears a running shield, for the reason [`reset`] gives.
#[test]
fn reset_clears_a_running_shield() {
    let mut s = state(0.0);
    s.shield_pickup_timer = 5.0;
    reset(&mut s, &dimensions(300.0));
    assert_eq!(s.shield_pickup_timer, 0.0);
}

/// A weapon hit spends the pool through the same body a wall does - the
/// clamp, the two edges and the destroyed transition - and differs only in
/// that the amount is the disc's number rather than a scaled impulse.
#[test]
fn a_weapon_hit_spends_the_authored_amount_and_shares_the_contact_path() {
    let d = dimensions(100.0);
    let mut s = state(100.0);

    let report = apply_weapon(&mut s, &d, 30.0, DamageRules::default());
    assert_eq!(report.lost, 30.0, "a weapon hit is not impulse-scaled");
    assert_eq!(s.shield, 70.0);
    assert!(!report.crossed_critical);

    // Straight through the 20 % line, and then to zero on the next one.
    let crossing = apply_weapon(&mut s, &d, 60.0, DamageRules::default());
    assert!(crossing.crossed_critical);
    let killing = apply_weapon(&mut s, &d, 60.0, DamageRules::default());
    assert!(killing.depleted);
    assert_eq!(s.shield, 0.0, "the pool must not go negative");
    assert_eq!(s.craft_state, CraftState::Destroyed);
}

/// The gate is shared, which is the reason [`subtract`] exists: a shielded
/// craft has to be immune to a rocket and not only to a wall.
#[test]
fn a_running_shield_refuses_a_weapon_hit_too() {
    let d = dimensions(100.0);
    let mut s = state(100.0);
    s.shield_pickup_timer = 0.5;
    assert_eq!(
        apply_weapon(&mut s, &d, 100.0, DamageRules::default()),
        Shield {
            absorbed: true,
            ..Shield::default()
        }
    );
    assert_eq!(s.shield, 100.0);

    let mut wrecked = state(100.0);
    wrecked.craft_state = CraftState::Eliminated;
    assert_eq!(
        apply_weapon(&mut wrecked, &d, 100.0, DamageRules::default()),
        Shield::default()
    );
}

/// `landed` is `Ship_Damage`'s weapon branch running, which is what throws
/// the struck hull's sparks: the killing hit lands, a shielded or wrecked
/// craft does not. See `docs/ghidra/functions/psp-pulse-usa/shield.md`.
#[test]
fn a_hit_lands_through_the_gate_and_the_killing_one_lands_too() {
    let d = dimensions(100.0);
    let mut s = state(5.0);
    let killing = apply_weapon(&mut s, &d, 30.0, DamageRules::default());
    assert!(killing.depleted && killing.landed());

    let mut shielded = state(100.0);
    shielded.shield_pickup_timer = 0.5;
    assert!(!apply_weapon(&mut shielded, &d, 30.0, DamageRules::default()).landed());

    // Destroyed by the hit above, so the next one is refused outright.
    assert!(!apply_weapon(&mut s, &d, 30.0, DamageRules::default()).landed());
}

/// The weapons-off halving is `Ship_Damage`'s and applies to every amount,
/// not only to a wall's. Unreachable today and reproduced anyway.
#[test]
fn turning_weapons_off_halves_a_weapon_hit_as_well() {
    let rules = DamageRules {
        weapons: false,
        ..DamageRules::default()
    };
    assert_eq!(weapon_damage(30.0, rules), 15.0);
    assert_eq!(weapon_damage(30.0, DamageRules::default()), 30.0);
}

/// A ship whose `<Misc>` never loaded has a zero maximum, and the HUD divides
/// by it. The original produces a NaN; this does not.
#[test]
fn a_zero_pool_reads_as_zero_percent_rather_than_nan() {
    assert_eq!(percent(0.0, 0.0), 0.0);
    assert!(percent(0.0, 0.0).is_finite());
}
