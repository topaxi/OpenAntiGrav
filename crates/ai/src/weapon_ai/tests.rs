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
        },
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
        assert!(!ai.decide(&s, 1.0, 0.0));
    }
    ai.tick(true);
    assert!(ai.decide(&s, 1.0, 0.0));
}

#[test]
fn an_aimed_weapon_with_nothing_in_the_path_opens_the_window_and_keeps_it() {
    let mut ai = WeaponAi {
        held_ticks: 100,
        wait_ticks: 0,
    };
    let mut s = situation();
    s.target_in_path = false;
    assert!(!ai.decide(&s, 0.0, 0.0));
    assert_eq!(ai.wait_ticks, WAIT_TICKS);
    // The Quake is not aimed and fires on the roll alone.
    s.aimed = false;
    assert!(ai.decide(&s, 1.0, 0.0));
}

#[test]
fn a_target_inside_the_window_gets_the_top_rate_whatever_the_roll() {
    let mut ai = WeaponAi {
        held_ticks: 0,
        wait_ticks: 30,
    };
    let s = situation();
    // The fire roll fails and the hold is not met; the window alone fires.
    assert!(ai.decide(&s, 0.09, 1.0));
    assert_eq!(ai.wait_ticks, 0);
    let mut ai = WeaponAi {
        held_ticks: 0,
        wait_ticks: 30,
    };
    assert!(!ai.decide(&s, 0.1, 1.0));
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
