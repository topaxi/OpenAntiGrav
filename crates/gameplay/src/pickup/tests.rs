//! What the pickup draw and the inventory in [`super`] are asserted to do.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of `pickup.rs`:
//! the tests went past the 200 lines an inline test module may hold when the
//! recovered draw arrived. See `scripts/check-file-size.py`, which is the rule
//! as a gate.

use super::*;
use oag_formats::weapons::PickupOdds;

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

/// The stand-in for "weighted, and not implemented". **It has to be a weapon
/// that stays out of [`IMPLEMENTED`]**, or the tests below invert silently
/// the day it lands - which is exactly what happened to the `Rocket` these
/// two used before. Quake needs track deformation and is a long way off.
const UNIMPLEMENTED: Weapon = Weapon::Quake;

#[test]
fn a_class_that_weights_nothing_implemented_hands_out_nothing() {
    // A quake is weighted and a quake cannot be handed out, so this is the
    // real shape of the restriction rather than an empty table.
    let quakes_only = table(&[(UNIMPLEMENTED, 1.0, 1.0)]);
    let mut rng = Rng::new(1);
    assert_eq!(
        draw(&mut rng, &quakes_only, Driver::HUMAN_UNPLACED, None),
        None
    );
}

#[test]
fn a_zero_weight_is_never_drawn() {
    let no_turbo = table(&[(Weapon::Turbo, 0.0, 1.0)]);
    let mut rng = Rng::new(1);
    for _ in 0..100 {
        assert_eq!(
            draw(&mut rng, &no_turbo, Driver::HUMAN_UNPLACED, None),
            None
        );
    }
}

/// The draw reads the *driver's own* column. With Turbo weighted for an AI
/// and not for a human, the two answers have to differ - a reader that took
/// whichever column came first would pass every other test here.
#[test]
fn the_two_drivers_read_their_own_columns() {
    let ai_only = table(&[(Weapon::Turbo, 0.0, 1.0)]);
    let mut rng = Rng::new(7);
    assert_eq!(
        draw(&mut rng, &ai_only, Driver::Ai, None),
        Some(Weapon::Turbo)
    );
    assert_eq!(draw(&mut rng, &ai_only, Driver::HUMAN_UNPLACED, None), None);
}

/// **The only thing about the draw that can be checked against the
/// original's data**, and it is a distribution rather than a sequence - see
/// the module docs.
///
/// Two implemented weapons weighted 3:1 against each other, plus an
/// unimplemented one weighted far above both. The second half is the part
/// that catches a real mistake: a walk that summed the *table's* total
/// rather than the implemented subset's would spend most of its rolls past
/// the end of the live weights and fall through to the trailing `find`,
/// which returns the last implemented weapon every time. That reads as
/// "mostly Shield" rather than as an error.
#[test]
fn the_walk_visits_a_weight_in_proportion_to_it() {
    let weighted = table(&[
        (Weapon::Turbo, 3.0, 3.0),
        (Weapon::Shield, 1.0, 1.0),
        (UNIMPLEMENTED, 96.0, 96.0),
    ]);
    let mut rng = Rng::new(99);
    let (mut turbos, mut shields) = (0, 0);
    for _ in 0..10_000 {
        match draw(&mut rng, &weighted, Driver::HUMAN_UNPLACED, None) {
            Some(Weapon::Turbo) => turbos += 1,
            Some(Weapon::Shield) => shields += 1,
            other => panic!("drew {other:?}, which is not implemented"),
        }
    }
    assert_eq!(turbos + shields, 10_000, "every draw must land somewhere");
    // 3:1 over 10,000 draws puts Turbo at 7,500 with a standard deviation of
    // about 43. A 500-wide window is roughly 11 sigma - wide enough that no
    // seed makes this flaky, narrow enough to reject an even split.
    assert!(
        (7_000..=8_000).contains(&turbos),
        "3:1 odds drew {turbos} turbos and {shields} shields"
    );
}

/// The property the determinism gate needs from this: same seed, same
/// sequence. It is the one guarantee that survives the original's PRNG being
/// unrecovered.
#[test]
fn the_same_seed_draws_the_same_sequence() {
    let weighted = table(&[(Weapon::Turbo, 1.0, 1.0)]);
    let sequence = |seed| {
        let mut rng = Rng::new(seed);
        (0..50)
            .map(|_| draw(&mut rng, &weighted, Driver::HUMAN_UNPLACED, None))
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

/// **The leader gets `front` and the tail gets `back`**, which is the
/// recovered blend and the reason the shipped table authors those columns.
///
/// Built as the shipped Venom table builds it - one weapon carrying `front`
/// and the other `back`, with equal `human` - so the only thing that can
/// separate the two answers is the place.
#[test]
fn the_players_odds_bend_with_their_place() {
    // Shield is the leader's weapon, Turbo the tail's, exactly as the disc
    // weights them.
    let odds = blended(&[
        (Weapon::Shield, 1.0, 1.0, 8.0, 0.0),
        (Weapon::Turbo, 1.0, 1.0, 0.0, 8.0),
    ]);

    let count = |driver| {
        let mut rng = Rng::new(11);
        (0..2_000)
            .filter(|_| draw(&mut rng, &odds, driver, None) == Some(Weapon::Shield))
            .count()
    };

    let leader = count(Driver::Human { place: 1, field: 8 });
    let tail = count(Driver::Human { place: 8, field: 8 });
    assert!(
        leader > tail,
        "the leader drew {leader} shields and the tail {tail} - the blend is \
         not reading the place, or is reading it backwards"
    );
    // And it is a big difference rather than noise: `front` is eight times
    // the flat weight.
    assert!(
        leader > tail + 500,
        "leader {leader} against tail {tail} is within noise of no blend"
    );
}

/// The AI's column is spent **flat**. The rubber-banding is aimed at the
/// player, and an opponent's odds must not move with its place.
#[test]
fn an_opponents_odds_do_not_bend() {
    let odds = blended(&[
        (Weapon::Shield, 1.0, 1.0, 8.0, 0.0),
        (Weapon::Turbo, 1.0, 1.0, 0.0, 8.0),
    ]);
    let mut first = Rng::new(11);
    let mut second = Rng::new(11);
    let front: Vec<_> = (0..200)
        .map(|_| draw(&mut first, &odds, Driver::Ai, None))
        .collect();
    let back: Vec<_> = (0..200)
        .map(|_| draw(&mut second, &odds, Driver::Ai, None))
        .collect();
    assert_eq!(front, back, "the AI column is not a function of place");
}

/// An unplaced craft spends the `human` column alone rather than blending
/// against place zero.
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

/// The divisor is the whole field, so the last-placed craft never reaches
/// the far end of the blend. The original's own arithmetic.
#[test]
fn the_blend_never_quite_reaches_the_back_column() {
    assert_eq!(Driver::descent(1, 8), Some(0.0));
    assert_eq!(Driver::descent(8, 8), Some(7.0 / 8.0));
}

/// **The same weapon is not handed out twice running.** Recovered from the
/// grant, which compares against the previous grant and re-rolls.
///
/// Weighted evenly across all four implemented weapons, which is close to
/// what the shipped Venom table gives them (14/14/11/9) and is the range
/// [`REDRAW_ATTEMPTS`] is sized for. An unguarded draw would repeat on about
/// a quarter of these 500 draws, so zero repeats is a strong signal rather
/// than a fixture that could not fail.
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
        let drawn = draw(&mut rng, &odds, Driver::HUMAN_UNPLACED, last);
        assert_ne!(drawn, last, "the same weapon came out twice running");
        last = drawn;
    }
}

/// **The rule is best-effort, and this is where it gives out.** The retry is
/// bounded, so a weapon that dominates its table hard enough will eventually
/// be handed out twice running rather than the draw spinning for ever.
///
/// Pinned as a *behaviour* rather than left as a footnote: it is the one
/// place this deviates from the original, which loops unbounded, and a table
/// with a single live weapon is exactly the case that would hang.
#[test]
fn an_overwhelming_weight_terminates_and_may_repeat() {
    let odds = table(&[(Weapon::Turbo, 1.0, 1.0)]);
    let mut rng = Rng::new(5);
    assert_eq!(
        draw(&mut rng, &odds, Driver::HUMAN_UNPLACED, Some(Weapon::Turbo)),
        Some(Weapon::Turbo),
        "the only weighted weapon must still come out"
    );
}

/// The memory outlasts firing, which is what makes the rule bite at all: a
/// craft that fires and crosses another pad still must not be handed the
/// same thing.
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

/// **Every rear weapon can be both laid and tripped.**
///
/// `mine::Drop::for_weapon` says how many charges a press lays and
/// `mine::TriggerRadii::get` says how close a craft has to come to set one off,
/// and the two are the same list of weapons written twice. Neither fails loudly
/// when they disagree: a weapon missing from the first lays nothing at all, and
/// one missing from the second is laid and can then never be tripped - it just
/// sits there until its fuse runs out, which reads as a tuning problem rather
/// than as a wiring one.
///
/// `every_implemented_weapon_has_a_fire_arm_on_both_paths` cannot catch it,
/// because the fire arm *does* exist in both cases.
///
/// The table authors every weapon in [`IMPLEMENTED`], so a weapon that answers
/// one call and not the other is a missing `match` arm rather than a missing
/// `<Weapon>` element.
#[test]
fn every_rear_weapon_can_be_laid_and_tripped() {
    use crate::projectile::mine::{Drop, TriggerRadii};

    let table = oag_formats::weapons::parse(
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
