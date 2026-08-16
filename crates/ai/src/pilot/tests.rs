//! What the named pilots and the draws they produce in [`super`] are asserted to do.
//!
//! Split out of `pilot.rs` under the 200-line cap on inline `#[cfg(test)]`
//! modules; see `scripts/check-file-size.py`.

use super::*;
use crate::Personality;

/// **The test that makes the whole refactor safe**, and the one that had to
/// be captured before `driver.rs` was touched.
///
/// These are `f32::to_bits` of what `Personality::from_seed` returned at
/// `fd35d8f`, before pilots existed - committed as literals precisely
/// because comparing against `from_seed` would be circular now that it
/// delegates to [`Pilot::BALANCED`]. If this fails, either a span in
/// `BALANCED` moved or an axis was inserted into the draw order ahead of
/// the frozen seven. Neither is a thing to fix by regenerating the table.
///
/// Order per row: `line_bias`, `wander`, `wander_rate`, `look`,
/// `commitment`, `patience`. Seeds 1, 7 and `0xC0FFEE` drew a negative
/// bias, so the coin flip at draw two is exercised both ways.
#[test]
fn the_balanced_pilot_reproduces_the_personality_that_shipped_before_pilots_existed() {
    const SHIPPED: [(u32, [u32; 6]); 8] = [
        (
            0x0000_0001,
            [
                0xbef9_3e4e,
                0x3e04_a8c3,
                0x3b96_e5e1,
                0x3f66_6aaa,
                0x3f86_0319,
                0x3f94_3455,
            ],
        ),
        (
            0x0000_0002,
            [
                0x3ecd_ae6f,
                0x3e00_f46a,
                0x3b90_9d5b,
                0x3f90_9d05,
                0x3f78_f38f,
                0x3f87_b4f9,
            ],
        ),
        (
            0x0000_0003,
            [
                0x3f08_cd3a,
                0x3e72_0a74,
                0x3b8b_8bc6,
                0x3f87_3bb3,
                0x3f81_b2ee,
                0x3f96_cfb0,
            ],
        ),
        (
            0x0000_0007,
            [
                0xbf00_6988,
                0x3e75_1b9e,
                0x3b2e_798e,
                0x3f7e_bd9c,
                0x3f75_7eef,
                0x3f8f_83db,
            ],
        ),
        (
            0x0000_002a,
            [
                0x3eff_16d2,
                0x3dce_6e8d,
                0x3b5d_a612,
                0x3f85_c362,
                0x3f80_2d13,
                0x3f8b_785d,
            ],
        ),
        (
            0x00c0_ffee,
            [
                0xbf02_9c98,
                0x3e83_e041,
                0x3b2b_6b78,
                0x3f6a_3dd6,
                0x3f72_db50,
                0x3f62_6cbf,
            ],
        ),
        (
            0xdead_beef,
            [
                0x3f25_dbe6,
                0x3e4e_06c2,
                0x3b67_9c7a,
                0x3f91_7f6a,
                0x3f7a_cd64,
                0x3f7a_8519,
            ],
        ),
        (
            0xffff_ffff,
            [
                0x3e97_b07e,
                0x3e68_ad10,
                0x3b7b_afc8,
                0x3f59_f748,
                0x3f71_32cf,
                0x3f5c_015f,
            ],
        ),
    ];

    for (seed, expected) in SHIPPED {
        let drawn = Personality::from_pilot_seed(&Pilot::BALANCED, seed);
        let got = [
            drawn.line_bias.to_bits(),
            drawn.wander.to_bits(),
            drawn.wander_rate.to_bits(),
            drawn.look.to_bits(),
            drawn.commitment.to_bits(),
            drawn.patience.to_bits(),
        ];
        assert_eq!(
            got, expected,
            "seed {seed:#010x} no longer draws what it did"
        );
    }
}

/// Seed zero was a special case before pilots and stays one after: it draws
/// nothing at all, so no pilot can turn the plain line-follower into a
/// character.
#[test]
fn seed_zero_is_the_plain_line_follower_whatever_the_pilot() {
    for (name, pilot) in Pilot::BUILT_IN {
        assert_eq!(
            Personality::from_pilot_seed(&pilot, 0),
            Personality::NEUTRAL,
            "{name} gave seed zero a character"
        );
    }
}

#[test]
fn a_fixed_span_still_consumes_its_draw() {
    let mut fixed = Rng::new(7);
    let mut ranged = Rng::new(7);
    assert_eq!(Span::fixed(0.5).draw(&mut fixed), 0.5);
    let _ = Span::new(0.0, 1.0).draw(&mut ranged);
    assert_eq!(fixed.snapshot(), ranged.snapshot());
}

#[test]
fn a_fixed_lean_still_consumes_its_draw() {
    let mut fixed = Rng::new(7);
    let mut either = Rng::new(7);
    assert_eq!(Lean::Left.sign(&mut fixed), -1.0);
    let _ = Lean::Either.sign(&mut either);
    assert_eq!(fixed.snapshot(), either.snapshot());
}

/// The invariant the whole module rests on: two pilots take the same draws
/// in the same order, so an axis inserted in the middle of one would show
/// up here immediately.
#[test]
fn every_pilot_consumes_the_same_draws_in_the_same_order() {
    let mut states = Vec::new();
    for (_, pilot) in Pilot::BUILT_IN {
        let mut rng = Rng::new(99);
        let _ = Personality::from_pilot(&pilot, &mut rng);
        states.push(rng.snapshot());
    }
    for state in &states {
        assert_eq!(
            *state, states[0],
            "one pilot drew a different number of values"
        );
    }
}

#[test]
fn every_built_in_pilot_is_well_formed() {
    for (name, pilot) in Pilot::BUILT_IN {
        assert!(pilot.is_well_formed(), "{name} has a reversed span");
    }
}

/// The ceiling that keeps a character from becoming a craft in the wall.
#[test]
fn no_pilot_asks_for_more_grip_than_the_hull_has() {
    for (name, pilot) in Pilot::BUILT_IN {
        assert!(
            pilot.commitment.high <= Pilot::MAX_COMMITMENT,
            "{name} asks for {} of the hull's grip",
            pilot.commitment.high
        );
    }
}

/// **Driven off [`Pilot::spans`] rather than a hand-written list**, so it
/// cannot go stale when draw fourteen is appended. Writing the axes out by
/// hand is how `trail`, `width` and `inside` went uncovered when they
/// landed, and then `courtesy`, `defence` and `caution` after them.
#[test]
fn every_built_in_pilot_stays_inside_the_ranges_it_declares() {
    for (name, pilot) in Pilot::BUILT_IN {
        for seed in 1..400u32 {
            let drawn = Personality::from_pilot(&pilot, &mut Rng::new(u64::from(seed)));
            // In the same order `spans` returns, which is draw order with
            // `lean` (not a range) left out. `line_bias` is compared by
            // magnitude because the lean carries its sign, and
            // `wander_rate` is the reciprocal of the span it was drawn from.
            let values = [
                drawn.line_bias.abs(),
                drawn.wander,
                1.0 / drawn.wander_rate,
                drawn.look,
                drawn.commitment,
                drawn.patience,
                drawn.trail,
                drawn.width,
                drawn.inside,
                drawn.courtesy,
                drawn.defence,
                drawn.caution,
            ];
            for (axis, (value, span)) in values.iter().zip(pilot.spans()).enumerate() {
                assert!(
                    *value >= span.low && *value <= span.high,
                    "{name} seed {seed}: axis {axis} drew {value}, outside {span:?}"
                );
            }
        }
    }
}

/// A built-in table that declared the social axes and gave them all the same
/// values would pass every mechanism test above and still field four pilots
/// that behave identically when pressed.
#[test]
fn the_built_in_pilots_disagree_about_yielding() {
    let lean = |pilot: &Pilot| {
        let mid = |span: Span| (span.low + span.high) * 0.5;
        mid(pilot.defence) - mid(pilot.courtesy)
    };
    assert!(
        lean(&Pilot::AGGRESSIVE) > 0.3,
        "the aggressive pilot should cover: {}",
        lean(&Pilot::AGGRESSIVE)
    );
    assert!(
        lean(&Pilot::SHY) < -0.3,
        "the shy pilot should yield: {}",
        lean(&Pilot::SHY)
    );
    assert!(lean(&Pilot::PASSIVE) < 0.0);
    // And they must actually lift for a craft ahead, or `caution` is a
    // field nothing reads.
    for (name, pilot) in Pilot::BUILT_IN {
        assert!(pilot.caution.high > 0.0, "{name} never lifts for anybody");
    }
}

#[test]
fn a_leaning_pilot_only_ever_picks_its_own_side() {
    let left = Pilot {
        lean: Lean::Left,
        ..Pilot::BALANCED
    };
    for seed in 1..200u32 {
        let drawn = Personality::from_pilot(&left, &mut Rng::new(u64::from(seed)));
        assert!(drawn.line_bias < 0.0, "seed {seed} leant right");
    }
}

#[test]
fn a_slot_draws_a_pilot_from_the_race_seed_alone() {
    assert_eq!(pilot_for_slot(42, 3, 4), pilot_for_slot(42, 3, 4));
    assert_eq!(pilot_for_slot(0, 0, 0), 0, "an empty roster picks nothing");
    for seed in 0..200u64 {
        assert!(pilot_for_slot(seed, 1, 4) < 4);
    }
}

/// **Each craft picks its own**, so a grid is a mixture rather than seven
/// copies of whatever the race drew once. With four pilots and seven slots
/// a monoculture is possible by luck; what would be a bug is it being
/// *common*.
#[test]
fn a_grid_draws_a_mixture_of_pilots_rather_than_one_for_everybody() {
    let mut monocultures = 0;
    let races = 500u64;
    for race in 0..races {
        let drawn: Vec<u32> = (1..8).map(|slot| pilot_for_slot(race, slot, 4)).collect();
        if drawn.windows(2).all(|pair| pair[0] == pair[1]) {
            monocultures += 1;
        }
    }
    // Seven independent draws from four all agreeing is 4 * (1/4)^7, about
    // one race in 4,096. A handful in five hundred would be luck; a tenth
    // of them would mean the draws are not independent.
    assert!(
        monocultures < 5,
        "{monocultures} of {races} grids fielded a single pilot"
    );
}

/// And the mixture changes from race to race, or every grid is the same
/// grid.
#[test]
fn a_different_race_fields_a_different_mixture() {
    let of = |race: u64| -> Vec<u32> { (1..8).map(|slot| pilot_for_slot(race, slot, 4)).collect() };
    let first = of(1);
    let differing = (2..40u64).filter(|race| of(*race) != first).count();
    assert!(
        differing > 30,
        "only {differing} of 38 later races fielded a different mixture"
    );
}

/// If the pilot a slot draws tracked its personality seed, a field of eight
/// would be four characters wearing eight names.
#[test]
fn the_pilot_a_slot_draws_does_not_track_its_personality_seed() {
    let mut seen = [0usize; 4];
    let mut agreements = 0;
    let races = 400u64;
    for race in 0..races {
        for slot in 1..8u32 {
            let pilot = pilot_for_slot(race, slot, 4) as usize;
            seen[pilot] += 1;
            // Wholly unrelated streams, so the low bit of one should agree
            // with the low bit of the other about half the time.
            if pilot % 2 == (crate::Driver::for_slot(race, slot).seed % 2) as usize {
                agreements += 1;
            }
        }
    }
    for (index, count) in seen.iter().enumerate() {
        assert!(*count > 0, "pilot {index} was never drawn");
    }
    let total = races as usize * 7;
    let ratio = agreements as f32 / total as f32;
    assert!(
        (0.4..0.6).contains(&ratio),
        "pilot choice tracks the personality seed: {ratio} agreement"
    );
}
