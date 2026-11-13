use super::*;

const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const NOSE: Vec3 = Vec3::new(0.0, 0.0, 1.0);
/// A Venom Rocket, as `WeaponStats_Elimination.xml` authors it.
const SPEED: f32 = 600.0;

fn still(x: f32, z: f32) -> Mover {
    Mover {
        position: Vec3::new(x, 0.0, z),
        velocity: NOSE * 150.0,
    }
}

fn path(others: &[Mover]) -> bool {
    target_in_path(Vec3::ZERO, UP, NOSE, SPEED, others.iter().copied())
}

#[test]
fn a_craft_on_the_nose_is_in_the_path_at_any_range_inside_the_horizon() {
    assert!(path(&[still(0.0, 50.0)]));
    // Far past this build's old 200-unit gate: the original has no range limit
    // short of the twenty-second horizon.
    assert!(path(&[still(0.0, 1500.0)]));
}

#[test]
fn the_corridor_widens_with_the_shot_flown() {
    // About eight and a half degrees of the shot's own travel plus four units.
    // A target at 150 a second ahead of a 600 shot is met a third further on
    // than it sits: 15 across at 50 is out, 15 across at 200 is in.
    assert!(!path(&[still(15.0, 50.0)]));
    assert!(path(&[still(15.0, 200.0)]));
    assert!(path(&[still(3.5, 0.5)]));
}

#[test]
fn nothing_behind_or_abeam_is_a_target() {
    assert!(!path(&[still(0.0, -50.0)]));
    assert!(!path(&[still(80.0, 0.0)]));
    assert!(!path(&[]));
}

#[test]
fn height_does_not_hide_a_target_the_plane_keeps_its_distance() {
    // Laid into the shooter's plane with its length kept: a craft 30 above on
    // the nose is 30 further along it, not out of the path.
    let above = Mover {
        position: Vec3::new(0.0, 30.0, 40.0),
        velocity: NOSE * 150.0,
    };
    assert!(path(&[above]));
}

fn situation() -> Situation {
    Situation {
        aimed: true,
        eliminator: true,
        gap_ahead: None,
        gap_behind: None,
        player_gap: None,
        target_in_path: true,
        odds: Odds {
            use_against_player: 1.1,
            use_against_ai: 1.2,
            absorb: 1.0,
        },
        shield: 100.0,
        law: Law::PULSE,
    }
}

#[test]
fn the_fire_index_follows_the_neighbours_and_an_eliminator_lifts_zero() {
    let mut s = situation();
    assert_eq!(s.fire_index(), 1);
    s.eliminator = false;
    assert_eq!(s.fire_index(), 0);
    s.gap_ahead = Some(60.0);
    assert_eq!(s.fire_index(), 3);
    s.gap_behind = Some(40.0);
    assert_eq!(s.fire_index(), 2);
    s.gap_ahead = Some(160.0);
    assert_eq!(s.fire_index(), 0);
}

#[test]
fn the_chance_is_the_rate_times_the_authored_use() {
    let mut s = situation();
    // 0.002 * 1.2 * 5, the long way round.
    let expected = 1.0 / ((1.0 / 0.002_f32) * (1.0 / (1.2_f32 * 5.0)));
    assert_eq!(s.fire_chance(), expected);
    s.player_gap = Some(100.0);
    let expected = 1.0 / ((1.0 / 0.002_f32) * (1.0 / (1.1_f32 * 5.0)));
    assert_eq!(s.fire_chance(), expected);
    // Outside an Eliminator, nobody close ahead: never.
    s.eliminator = false;
    assert_eq!(s.fire_chance(), 0.0);
}

#[test]
fn nothing_fires_inside_the_hold() {
    let mut ai = WeaponAi::default();
    let s = situation();
    for _ in 0..HOLD_TICKS {
        ai.tick(true);
        assert_ne!(ai.decide(&s, 1.0, 0.0, 1.0), Decision::Fire);
    }
    ai.tick(true);
    assert_eq!(ai.decide(&s, 1.0, 0.0, 1.0), Decision::Fire);
}

#[test]
fn an_aimed_weapon_with_nothing_in_the_path_opens_the_window_and_keeps_it() {
    let mut ai = WeaponAi {
        held_ticks: 100,
        wait_ticks: 0,
    };
    let mut s = situation();
    s.target_in_path = false;
    assert_ne!(ai.decide(&s, 0.0, 0.0, 1.0), Decision::Fire);
    assert_eq!(ai.wait_ticks, WAIT_TICKS);
    // The Quake is not aimed and fires on the roll alone.
    s.aimed = false;
    assert_eq!(ai.decide(&s, 1.0, 0.0, 1.0), Decision::Fire);
}

#[test]
fn a_target_inside_the_window_gets_the_top_rate_whatever_the_roll() {
    let mut ai = WeaponAi {
        held_ticks: 0,
        wait_ticks: 30,
    };
    let s = situation();
    // The fire roll fails and the hold is not met; the window alone fires.
    assert_eq!(ai.decide(&s, 0.09, 1.0, 1.0), Decision::Fire);
    assert_eq!(ai.wait_ticks, 0);
    let mut ai = WeaponAi {
        held_ticks: 0,
        wait_ticks: 30,
    };
    assert_ne!(ai.decide(&s, 0.1, 1.0, 1.0), Decision::Fire);
}

#[test]
fn the_clocks_count_ticks_and_an_empty_slot_resets_the_hold() {
    let mut ai = WeaponAi {
        held_ticks: 10,
        wait_ticks: 1,
    };
    ai.tick(true);
    assert_eq!(
        ai,
        WeaponAi {
            held_ticks: 11,
            wait_ticks: 0
        }
    );
    ai.tick(false);
    assert_eq!(
        ai,
        WeaponAi {
            held_ticks: 0,
            wait_ticks: 0
        }
    );
}

#[test]
fn the_absorb_index_follows_the_shield_the_hold_and_the_road_ahead() {
    let mut s = situation();
    s.eliminator = false;
    s.gap_ahead = Some(200.0);
    assert_eq!(s.absorb_index(0), 0);
    // Nobody inside 300 ahead.
    s.gap_ahead = Some(301.0);
    assert_eq!(s.absorb_index(0), 1);
    s.gap_ahead = None;
    assert_eq!(s.absorb_index(0), 1);
    // Held past fifteen seconds.
    assert_eq!(s.absorb_index(ITCHY_TICKS + 1), 2);
    s.gap_ahead = Some(200.0);
    // One per shield tier, clamped at four.
    for (shield, index) in [
        (81.0, 0),
        (80.0, 1),
        (60.0, 2),
        (40.0, 3),
        (20.0, 4),
        (5.0, 4),
    ] {
        s.shield = shield;
        assert_eq!(s.absorb_index(0), index, "shield {shield}");
    }
    // An Eliminator forces zero.
    s.eliminator = true;
    assert_eq!(s.absorb_index(ITCHY_TICKS + 1), 0);
}

#[test]
fn a_failed_fire_roll_absorbs_on_the_absorb_roll_after_the_short_hold() {
    let mut s = situation();
    s.eliminator = false;
    s.shield = 10.0;
    // rate[4] * absorb 1.0.
    assert_eq!(s.absorb_chance(0), 1.0 / ((1.0 / 0.05_f32) * (1.0 / 1.0)));
    let mut ai = WeaponAi {
        held_ticks: ABSORB_HOLD_TICKS,
        wait_ticks: 0,
    };
    assert_eq!(ai.decide(&s, 1.0, 1.0, 0.0), Decision::Hold);
    ai.held_ticks += 1;
    assert_eq!(ai.decide(&s, 1.0, 1.0, 0.0), Decision::Absorb);
    assert_eq!(ai.decide(&s, 1.0, 1.0, 0.05), Decision::Hold);
    // A healthy craft with somebody close ahead never absorbs: rate[0] is zero.
    s.shield = 100.0;
    s.gap_ahead = Some(50.0);
    assert_eq!(s.absorb_chance(0), 0.0);
}

/// HD's law with the authored `AllWeapons` row (`0.9 / 0.9 / 1.0`).
fn hd_law() -> Law {
    Law {
        step_scale: 0.5,
        all_weapons: Odds {
            use_against_player: 0.9,
            use_against_ai: 0.9,
            absorb: 1.0,
        },
        eliminator: EliminatorTerms::Authored {
            use_scale: 12.5,
            absorb_scale: 2.6,
        },
        use_scale: 1.0,
    }
}

fn long_way(rate: f32, odds: f32) -> f32 {
    1.0 / ((1.0 / rate) * (1.0 / odds))
}

#[test]
fn the_step_scale_halves_fire_absorb_and_the_early_out() {
    let mut s = situation();
    s.eliminator = false;
    s.gap_ahead = Some(50.0);
    s.shield = 10.0;
    let pulse_fire = s.fire_chance();
    let pulse_absorb = s.absorb_chance(0);
    s.law.step_scale = 0.5;
    assert_eq!(s.fire_chance(), long_way(0.5 * RATE[3], 1.2));
    assert!(s.fire_chance() < pulse_fire * 0.51);
    assert_eq!(s.absorb_chance(0), long_way(0.5 * RATE[4], 1.0));
    assert!(s.absorb_chance(0) < pulse_absorb * 0.51);
    // The window's flat chance is `k * rate[4]`: 0.03 passes Pulse's 0.05 and
    // fails HD's 0.025.
    let mut ai = WeaponAi {
        held_ticks: 0,
        wait_ticks: 5,
    };
    assert_eq!(ai.decide(&s, 0.03, 1.0, 1.0), Decision::Hold);
    s.law.step_scale = 1.0;
    assert_eq!(ai.decide(&s, 0.03, 1.0, 1.0), Decision::Fire);
}

#[test]
fn the_all_weapons_row_multiplies_use_and_absorb() {
    let mut s = situation();
    s.eliminator = false;
    s.gap_ahead = Some(50.0);
    s.shield = 10.0;
    s.law.all_weapons = hd_law().all_weapons;
    assert_eq!(s.fire_chance(), long_way(RATE[3], 1.2 * 0.9));
    s.player_gap = Some(100.0);
    assert_eq!(s.fire_chance(), long_way(RATE[3], 1.1 * 0.9));
    s.law.all_weapons.absorb = 0.5;
    assert_eq!(s.absorb_chance(0), long_way(RATE[4], 0.5));
}

#[test]
fn an_authored_eliminator_scales_use_and_absorb_and_keeps_the_absorb_index() {
    let mut s = situation();
    assert!(s.eliminator);
    assert_eq!(s.fire_chance(), long_way(RATE_ELIMINATOR[1], 1.2 * 5.0));
    assert_eq!(s.absorb_index(0), 0);
    s.law = hd_law();
    s.shield = 10.0;
    assert_eq!(
        s.fire_chance(),
        long_way(0.5 * RATE_ELIMINATOR[1], 1.2 * 0.9 * 12.5)
    );
    // The index is the setup's, not forced to zero: four shield tiers and the
    // nobody-ahead step clamp to four.
    assert_eq!(s.absorb_index(0), 4);
    assert_eq!(
        s.absorb_chance(0),
        long_way(0.5 * RATE_ELIMINATOR[4], 1.0 * 1.0 * 2.6)
    );
}

#[test]
fn the_demos_doubled_use_doubles_the_fire_odds_and_leaves_the_absorb_odds() {
    let mut s = situation();
    s.eliminator = false;
    s.gap_ahead = Some(50.0);
    let fire = s.fire_chance();
    let absorb = s.absorb_chance(0);
    s.law.use_scale = 2.0;
    assert_eq!(
        s.fire_chance(),
        long_way(s.rates()[s.fire_index()], 1.2 * 2.0)
    );
    assert!(s.fire_chance() > fire);
    assert_eq!(s.absorb_chance(0), absorb);
}
