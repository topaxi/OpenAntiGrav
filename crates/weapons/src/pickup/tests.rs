//! What the pickup draw and the inventory in [`super`] are asserted to do. Its own
//! file: the tests passed the 200 lines an inline test module may hold
//! (`scripts/check-file-size.py`).

use super::*;
use oag_tables::weapons::PickupOdds;

fn table(odds: &[(Weapon, f32, f32)]) -> PickupTable {
    PickupTable {
        class: "VENOM".to_string(),
        odds: odds
            .iter()
            .map(|&(weapon, human, ai)| {
                (
                    weapon,
                    PickupOdds {
                        human,
                        ai,
                        front: 0.0,
                        back: 0.0,
                    },
                )
            })
            .collect(),
    }
}

/// The stand-in for "weighted, and not drawable". Every weapon is implemented
/// since the Repulser (2026-10-04), so the restriction is reached through
/// `allowed`: the Repulser stays weighted and is left out of the subset passed.
const EXCLUDED: Weapon = Weapon::Repulser;

/// What every test here passes as `allowed`: everything but [`EXCLUDED`].
fn drawable() -> Vec<Weapon> {
    IMPLEMENTED
        .iter()
        .copied()
        .filter(|&w| w != EXCLUDED)
        .collect()
}

#[test]
fn a_class_that_weights_nothing_implemented_hands_out_nothing() {
    // A weighted Repulser that cannot be handed out: the real shape of the
    // restriction.
    let repulsers_only = table(&[(EXCLUDED, 1.0, 1.0)]);
    let mut rng = Rng::new(1);
    let allowed = drawable();
    assert_eq!(
        draw(
            &mut rng,
            &repulsers_only,
            Driver::HUMAN_UNPLACED,
            None,
            Some(&allowed)
        ),
        None
    );
}

#[test]
fn a_zero_weight_is_never_drawn() {
    let no_turbo = table(&[(Weapon::Turbo, 0.0, 1.0)]);
    let mut rng = Rng::new(1);
    for _ in 0..100 {
        assert_eq!(
            draw(&mut rng, &no_turbo, Driver::HUMAN_UNPLACED, None, None),
            None
        );
    }
}

/// The draw reads the driver's own column: with Turbo weighted for an AI and not a
/// human, the answers differ (a reader taking the first column would pass the rest).
#[test]
fn the_two_drivers_read_their_own_columns() {
    let ai_only = table(&[(Weapon::Turbo, 0.0, 1.0)]);
    let mut rng = Rng::new(7);
    assert_eq!(
        draw(&mut rng, &ai_only, Driver::Ai, None, None),
        Some(Weapon::Turbo)
    );
    assert_eq!(
        draw(&mut rng, &ai_only, Driver::HUMAN_UNPLACED, None, None),
        None
    );
}

/// The only thing about the draw checkable against the original's data: a
/// distribution, not a sequence (see the module docs). Two implemented weapons
/// weighted 3:1 plus an unimplemented one weighted far above both. The second half
/// catches a walk summing the table's total instead of the implemented subset's:
/// most rolls would fall through to the trailing `find` and return the last
/// implemented weapon, reading as "mostly Shield" rather than an error.
#[test]
fn the_walk_visits_a_weight_in_proportion_to_it() {
    let weighted = table(&[
        (Weapon::Turbo, 3.0, 3.0),
        (Weapon::Shield, 1.0, 1.0),
        (EXCLUDED, 96.0, 96.0),
    ]);
    let mut rng = Rng::new(99);
    let allowed = drawable();
    let (mut turbos, mut shields) = (0, 0);
    for _ in 0..10_000 {
        match draw(
            &mut rng,
            &weighted,
            Driver::HUMAN_UNPLACED,
            None,
            Some(&allowed),
        ) {
            Some(Weapon::Turbo) => turbos += 1,
            Some(Weapon::Shield) => shields += 1,
            other => panic!("drew {other:?}, which is not implemented"),
        }
    }
    assert_eq!(turbos + shields, 10_000, "every draw must land somewhere");
    // 3:1 over 10,000 draws puts Turbo at 7,500, sd about 43; a 500-wide window is
    // about 11 sigma: no seed flakes, an even split is rejected.
    assert!(
        (7_000..=8_000).contains(&turbos),
        "3:1 odds drew {turbos} turbos and {shields} shields"
    );
}

/// `allowed` restricts the draw without changing the odds of what stays in (the
/// 2048 campaign's weapon-set gate, wired through `race::pads`): a non-empty list
/// filters, an empty `Some` hands out nothing rather than falling back.
#[test]
fn allowed_restricts_the_draw_without_changing_the_ratio_of_what_stays_in() {
    let weighted = table(&[
        (Weapon::Turbo, 3.0, 3.0),
        (Weapon::Shield, 1.0, 1.0),
        (Weapon::Rocket, 50.0, 50.0),
    ]);
    let mut rng = Rng::new(99);
    for _ in 0..1_000 {
        let drawn = draw(
            &mut rng,
            &weighted,
            Driver::HUMAN_UNPLACED,
            None,
            Some(&[Weapon::Turbo, Weapon::Shield]),
        );
        assert!(
            matches!(drawn, Some(Weapon::Turbo) | Some(Weapon::Shield)),
            "drew {drawn:?}, which `allowed` does not list"
        );
    }

    // `Some(&[])` hands out nothing: a weapon set with no recognised bit, not a
    // silent fall-through to the unrestricted table.
    assert_eq!(
        draw(&mut rng, &weighted, Driver::HUMAN_UNPLACED, None, Some(&[])),
        None
    );
}

/// Same seed, same sequence: the guarantee the determinism gate needs, and the one
/// that survives the original's PRNG being unrecovered.
#[test]
fn the_same_seed_draws_the_same_sequence() {
    let weighted = table(&[(Weapon::Turbo, 1.0, 1.0)]);
    let sequence = |seed| {
        let mut rng = Rng::new(seed);
        (0..50)
            .map(|_| draw(&mut rng, &weighted, Driver::HUMAN_UNPLACED, None, None))
            .collect::<Vec<_>>()
    };
    assert_eq!(sequence(4), sequence(4));
}

/// A four-column table, for the tests that need `front`/`back`.
fn blended(odds: &[(Weapon, f32, f32, f32, f32)]) -> PickupTable {
    PickupTable {
        class: "VENOM".to_string(),
        odds: odds
            .iter()
            .map(|&(weapon, human, ai, front, back)| {
                (
                    weapon,
                    PickupOdds {
                        human,
                        ai,
                        front,
                        back,
                    },
                )
            })
            .collect(),
    }
}

/// The leader gets `front` and the tail gets `back`, the recovered blend. Built as
/// the shipped Venom table builds it (one weapon with `front`, the other `back`,
/// equal `human`), so only the place separates the answers.
#[test]
fn the_players_odds_bend_with_their_place() {
    // Shield is the leader's weapon, Turbo the tail's, as the disc weights them.
    let odds = blended(&[
        (Weapon::Shield, 1.0, 1.0, 8.0, 0.0),
        (Weapon::Turbo, 1.0, 1.0, 0.0, 8.0),
    ]);

    let count = |driver| {
        let mut rng = Rng::new(11);
        (0..2_000)
            .filter(|_| draw(&mut rng, &odds, driver, None, None) == Some(Weapon::Shield))
            .count()
    };

    let leader = count(Driver::Human { place: 1, field: 8 });
    let tail = count(Driver::Human { place: 8, field: 8 });
    assert!(
        leader > tail,
        "the leader drew {leader} shields and the tail {tail} - the blend is \
         not reading the place, or is reading it backwards"
    );
    // A big difference, not noise: `front` is eight times the flat weight.
    assert!(
        leader > tail + 500,
        "leader {leader} against tail {tail} is within noise of no blend"
    );
}

/// The AI's column is spent flat: rubber-banding is aimed at the player.
#[test]
fn an_opponents_odds_do_not_bend() {
    let odds = blended(&[
        (Weapon::Shield, 1.0, 1.0, 8.0, 0.0),
        (Weapon::Turbo, 1.0, 1.0, 0.0, 8.0),
    ]);
    let mut first = Rng::new(11);
    let mut second = Rng::new(11);
    let front: Vec<_> = (0..200)
        .map(|_| draw(&mut first, &odds, Driver::Ai, None, None))
        .collect();
    let back: Vec<_> = (0..200)
        .map(|_| draw(&mut second, &odds, Driver::Ai, None, None))
        .collect();
    assert_eq!(front, back, "the AI column is not a function of place");
}

/// An unplaced craft spends `human` alone rather than blending against place zero.
#[test]
fn an_unplaced_player_gets_no_blend() {
    let odds = blended(&[(Weapon::Shield, 1.0, 1.0, 8.0, 0.0)]);
    assert_eq!(
        Driver::HUMAN_UNPLACED.weight(&odds, Weapon::Shield),
        1.0,
        "an unplaced craft picked up a front or back weight"
    );
    assert_eq!(Driver::descent(0, 8), None);
    assert_eq!(Driver::descent(4, 0), None);
}

/// The divisor is the whole field, so last place never reaches the far end of the
/// blend (the original's arithmetic).
#[test]
fn the_blend_never_quite_reaches_the_back_column() {
    assert_eq!(Driver::descent(1, 8), Some(0.0));
    assert_eq!(Driver::descent(8, 8), Some(7.0 / 8.0));
}

/// The same weapon is not handed out twice running (recovered: the grant compares
/// against the previous grant and re-rolls). Weighted evenly over four implemented
/// weapons, close to the shipped Venom table (14/14/11/9) and the range
/// [`REDRAW_ATTEMPTS`] is sized for; an unguarded draw repeats on about a quarter
/// of 500 draws, so zero repeats is a strong signal.
#[test]
fn a_pad_does_not_hand_out_the_same_weapon_twice_running() {
    let odds = table(&[
        (Weapon::Turbo, 1.0, 1.0),
        (Weapon::Shield, 1.0, 1.0),
        (Weapon::Rocket, 1.0, 1.0),
        (Weapon::Missile, 1.0, 1.0),
    ]);
    let mut rng = Rng::new(3);
    let mut last = None;
    for _ in 0..500 {
        let drawn = draw(&mut rng, &odds, Driver::HUMAN_UNPLACED, last, None);
        assert_ne!(drawn, last, "the same weapon came out twice running");
        last = drawn;
    }
}

/// The rule is best-effort and this is where it gives out: the retry is bounded,
/// so a weapon dominating its table is eventually handed out twice running rather
/// than the draw spinning. Pinned as behaviour: it is the one deviation from the
/// original's unbounded loop, and a single-weapon table would hang.
#[test]
fn an_overwhelming_weight_terminates_and_may_repeat() {
    let odds = table(&[(Weapon::Turbo, 1.0, 1.0)]);
    let mut rng = Rng::new(5);
    assert_eq!(
        draw(
            &mut rng,
            &odds,
            Driver::HUMAN_UNPLACED,
            Some(Weapon::Turbo),
            None
        ),
        Some(Weapon::Turbo),
        "the only weighted weapon must still come out"
    );
}

/// The memory outlasts firing: a craft that fires and crosses another pad must not
/// be handed the same thing.
#[test]
fn what_was_granted_is_remembered_after_it_is_fired() {
    let mut held = Held::empty();
    held.grant(Weapon::Rocket);
    assert_eq!(held.take(), Some(Weapon::Rocket));
    assert!(held.is_empty());
    assert_eq!(held.last, Some(Weapon::Rocket), "the memory was cleared");
}

#[test]
fn a_held_slot_gives_up_what_it_holds_exactly_once() {
    let mut held = Held::empty();
    assert!(held.is_empty());
    held.weapon = Some(Weapon::Turbo);
    assert!(!held.is_empty());
    assert_eq!(held.take(), Some(Weapon::Turbo));
    assert_eq!(held.take(), None);
}

/// A re-press mid-cluster does nothing, so a mashed fire button cannot lay mines
/// faster than [`crate::projectile::mine::CLUSTER`] per
/// [`crate::projectile::mine::DROP_INTERVAL`]; see [`Held::begin_drop`].
#[test]
fn a_re_press_mid_cluster_does_not_restart_the_drop() {
    let mut held = Held::empty();
    held.grant(Weapon::Mine);
    held.begin_drop(5);
    assert_eq!(held.dropping, 5);

    // One charge out, as `Race::lay_mines` would over some ticks, so a re-arm shows
    // as the counter jumping back up.
    held.dropping = 3;
    held.drop_reload = 0.05;

    held.begin_drop(5);
    assert_eq!(
        held.dropping, 3,
        "a re-press mid-cluster must not top the counter back up"
    );
    assert_eq!(
        held.drop_reload, 0.05,
        "a re-press mid-cluster must not reset the reload timer either"
    );
}

/// A fresh grant, with nothing dropping, is unaffected by the guard above.
#[test]
fn begin_drop_still_arms_a_cluster_from_empty() {
    let mut held = Held::empty();
    held.grant(Weapon::Mine);
    held.begin_drop(5);
    assert_eq!(held.dropping, 5);
    assert_eq!(held.drop_reload, 0.0);
}

/// Taking a pickup mid-drop ends the drop too, or a stale counter would swallow a
/// later grant's first press; see [`Held::take`].
#[test]
fn taking_a_pickup_mid_drop_clears_the_drop_state_too() {
    let mut held = Held::empty();
    held.grant(Weapon::Mine);
    held.begin_drop(5);
    held.dropping = 3;
    held.drop_reload = 0.05;

    assert_eq!(held.take(), Some(Weapon::Mine));
    assert_eq!(held.dropping, 0);
    assert_eq!(held.drop_reload, 0.0);
    assert!(!held.is_dropping());

    // The slot is immediately usable for a fresh cluster.
    held.grant(Weapon::Mine);
    held.begin_drop(5);
    assert_eq!(held.dropping, 5);
}

/// Every rear weapon can be both laid and tripped. `mine::Drop::for_weapon` and
/// `mine::TriggerRadii::get` are the same list twice and neither fails loudly: a
/// weapon missing from the first lays nothing, one missing from the second can
/// never be tripped (it reads as tuning, not wiring).
/// `every_implemented_weapon_has_a_fire_arm_on_both_paths` cannot catch it, the
/// fire arm existing either way. The table authors every weapon in
/// [`IMPLEMENTED`], so a split answer is a missing `match` arm.
#[test]
fn every_rear_weapon_can_be_laid_and_tripped() {
    use crate::projectile::mine::{Drop, TriggerRadii};

    let table = oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Mine"><Stats absorb="1" blastforce="2" blastradius="3" damage="4" timetodie="5" trigger_radius="6"/></Weapon>
             <Weapon type="Bomb"><Stats absorb="7" blastforce="8" blastradius="9" damage="10" damageradius="11" timetodie="12" trigger_radius="13"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse");
    let radii = TriggerRadii::from_table(Some(&table));

    for &weapon in IMPLEMENTED {
        let laid = Drop::for_weapon(weapon, &table);
        let trippable = radii.get(weapon);
        assert_eq!(
            laid.is_some(),
            trippable.is_some(),
            "{weapon:?} answers one of `Drop::for_weapon` and `TriggerRadii::get` \
             and not the other, so it is either laid and untrippable or not laid \
             at all. Both `match`es are in `projectile/mine.rs` and grow together."
        );
        if let Some(laid) = laid {
            assert!(laid.count > 0, "{weapon:?} lays nothing");
            assert_eq!(
                Some(laid.trigger_radius),
                trippable,
                "{weapon:?}: the two paths disagree about its trigger radius"
            );
        }
    }
}
