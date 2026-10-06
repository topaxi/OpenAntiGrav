//! What the named pilots and the draws they produce in [`super`] are asserted to do.
//!
//! Split out of `pilot.rs` for the 200-line inline test cap
//! (`scripts/check-file-size.py`).

use super::*;
use crate::Personality;

/// **Makes the pilot refactor safe**, captured before `driver.rs` was touched.
///
/// `f32::to_bits` of what `Personality::from_seed` returned at `fd35d8f`,
/// before pilots existed, as literals: comparing against `from_seed` would be
/// circular now it delegates to [`Pilot::BALANCED`]. If this fails, a span in
/// `BALANCED` moved or an axis was inserted ahead of the frozen seven; never
/// fix it by regenerating. Order per row: `line_bias`, `wander`, `wander_rate`,
/// `look`, `commitment`, `patience`. Seeds 1, 7 and `0xC0FFEE` drew a negative
/// bias, so the coin at draw two runs both ways.
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

/// **Driven off [`Pilot::spans`], not a hand-written list**, so it cannot go
/// stale when draw fourteen is appended (hand-written lists left `trail`,
/// `width`, `inside`, then `courtesy`, `defence`, `caution` uncovered).
#[test]
fn every_built_in_pilot_stays_inside_the_ranges_it_declares() {
    for (name, pilot) in Pilot::BUILT_IN {
        for seed in 1..400u32 {
            let drawn = Personality::from_pilot(&pilot, &mut Rng::new(u64::from(seed)));
            // `spans` order: draw order minus `lean` (not a range). `line_bias`
            // compares by magnitude (the lean carries the sign); `wander_rate` is
            // the reciprocal of its drawn span.
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

/// **The append-order guard for the three roll axes**, over all four built-ins
/// rather than `BALANCED` alone.
///
/// `the_balanced_pilot_reproduces_the_personality_that_shipped_before_pilots_existed`
/// pins draws one to seven of one pilot; nothing pinned draws eight to sixteen,
/// the stretch an axis inserted in the middle would move. These are
/// `f32::to_bits` of the fifteen pre-existing axes of each built-in, captured at
/// `bc53a3f0` before `roll_chance`, `roll_floor` and `roll_airtime` were
/// appended. If this fails the three draws did not land at the end; never
/// regenerate. Order per row: `line_bias`, `wander`, `wander_rate`, `look`,
/// `commitment`, `patience`, `trail`, `width`, `inside`, `courtesy`, `defence`,
/// `caution`, `ram`, `provocation_ticks`, `trigger`.
#[test]
fn every_built_in_pilot_still_draws_what_it_drew_before_the_roll_axes_were_appended() {
    const BEFORE: [(&str, u32, [u32; 15]); 12] = [
        (
            "balanced",
            0x0000_0001,
            [
                0xbef9_3e4e,
                0x3e04_a8c3,
                0x3b96_e5e1,
                0x3f66_6aaa,
                0x3f86_0319,
                0x3f94_3455,
                0x3f73_d4bd,
                0x3f84_3cdd,
                0x3dc0_616b,
                0x3e80_d794,
                0x3ea7_033c,
                0x3f55_8af1,
                0x3e3f_7669,
                0x433d_7ad0,
                0x3f02_099d,
            ],
        ),
        (
            "balanced",
            0x0000_0007,
            [
                0xbf00_6988,
                0x3e75_1b9e,
                0x3b2e_798e,
                0x3f7e_bd9c,
                0x3f75_7eef,
                0x3f8f_83db,
                0x3f3a_c83c,
                0x3f6b_ef74,
                0x3d69_d5c2,
                0x3eae_d105,
                0x3eca_6bd0,
                0x3f0d_853c,
                0x3e41_3842,
                0x4356_c4fe,
                0x3f03_5722,
            ],
        ),
        (
            "balanced",
            0xdead_beef,
            [
                0x3f25_dbe6,
                0x3e4e_06c2,
                0x3b67_9c7a,
                0x3f91_7f6a,
                0x3f7a_cd64,
                0x3f7a_8519,
                0x3f69_a566,
                0x3f84_f150,
                0x3dbb_82ea,
                0x3e54_7fe9,
                0x3e84_8482,
                0x3f22_0635,
                0x3e15_653c,
                0x434e_57cc,
                0x3ee7_d01f,
            ],
        ),
        (
            "aggressive",
            0x0000_0001,
            [
                0xbefe_a2db,
                0x3d60_5bc2,
                0x3bae_f13e,
                0x3f5d_e023,
                0x3f86_3d06,
                0x3f6d_f565,
                0x3fa0_50c5,
                0x3f5d_cb5c,
                0x3ee0_30b6,
                0x3d26_88e5,
                0x3f54_f859,
                0x3ecd_1d36,
                0x3f6a_4ef0,
                0x43cf_e658,
                0x3f5a_9e68,
            ],
        ),
        (
            "aggressive",
            0x0000_0007,
            [
                0xbf02_7a14,
                0x3de0_a0bc,
                0x3b6e_03e8,
                0x3f6a_d92e,
                0x3f81_8b83,
                0x3f69_6738,
                0x3f83_ca85,
                0x3f3a_1e84,
                0x3eba_7570,
                0x3d9c_d38d,
                0x3f71_4c02,
                0x3e42_294f,
                0x3f6b_7ad6,
                0x43e4_f97e,
                0x3f5b_452b,
            ],
        ),
        (
            "aggressive",
            0xdead_beef,
            [
                0x3f21_aeb6,
                0x3db9_8be0,
                0x3b92_5fbc,
                0x3f7e_2f2a,
                0x3f82_a686,
                0x3f57_ab6f,
                0x3f9b_3919,
                0x3f5f_8e7a,
                0x3edd_c175,
                0x3cbc_7a97,
                0x3f39_5fc4,
                0x3e7f_ac39,
                0x3f4e_437d,
                0x43dd_f3d5,
                0x3f53_8da1,
            ],
        ),
        (
            "passive",
            0x0000_0001,
            [
                0xbeda_eeba,
                0x3e13_165e,
                0x3b66_3874,
                0x3f88_89ab,
                0x3f7c_dbb1,
                0x3f9e_3262,
                0x3f0d_6e56,
                0x3f8a_a343,
                0x3d80_40f2,
                0x3ef1_a688,
                0x3e43_8a4c,
                0x3f73_5b68,
                0x3d3b_0448,
                0x42d4_a3c0,
                0x3e8f_db38,
            ],
        ),
        (
            "passive",
            0x0000_0007,
            [
                0xbee0_9e4c,
                0x3e6d_0bda,
                0x3b15_e0c3,
                0x3f8f_0631,
                0x3f6b_f63d,
                0x3f9b_8465,
                0x3ea8_c3ac,
                0x3f78_bc41,
                0x3d1b_e3d7,
                0x3f14_68ee,
                0x3e7c_319f,
                0x3f3d_5720,
                0x3d3d_5c14,
                0x42f6_5bfc,
                0x3e91_cf80,
            ],
        ),
        (
            "passive",
            0xdead_beef,
            [
                0x3f0c_64ec,
                0x3e4d_c7f6,
                0x3b3c_4de0,
                0x3f98_b12f,
                0x3f6f_f115,
                0x3f91_15ef,
                0x3f03_3f00,
                0x3f8b_57b6,
                0x3d7a_03e2,
                0x3ed6_8a2f,
                0x3e0c_5921,
                0x3f4c_b7db,
                0x3d02_ed61,
                0x42eb_1fbb,
                0x3e75_51c8,
            ],
        ),
        (
            "shy",
            0x0000_0001,
            [
                0xbf1b_9d4d,
                0x3e33_5202,
                0x3b77_9e19,
                0x3f83_6af3,
                0x3f77_bcf9,
                0x3fa5_9189,
                0x3eb4_7646,
                0x3f91_09aa,
                0xbbc6_b620,
                0x3f52_6cde,
                0x3d66_f1c1,
                0x3f77_9246,
                0x0000_0000,
                0x4281_7ad0,
                0x3df0_9676,
            ],
        ),
        (
            "shy",
            0x0000_0007,
            [
                0xbf1e_2439,
                0x3e89_736a,
                0x3b1d_0fd5,
                0x3f89_e778,
                0x3f66_d785,
                0x3fa2_9ef3,
                0x3e04_ba8b,
                0x3f82_c487,
                0xbd2f_c3d8,
                0x3f6e_0288,
                0x3da0_cb89,
                0x3f53_8f6b,
                0x0000_0000,
                0x429a_c4fe,
                0x3df7_1a0e,
            ],
        ),
        (
            "shy",
            0xdead_beef,
            [
                0x3f37_1b22,
                0x3e71_aeb3,
                0x3b47_c8f7,
                0x3f93_9276,
                0x3f6a_d25d,
                0x3f97_2571,
                0x3ea0_1799,
                0x3f91_be1d,
                0xbc0a_4f18,
                0x3f44_deb1,
                0x3d0e_a316,
                0x3f5d_cfe8,
                0x0000_0000,
                0x4292_57cc,
                0x3da9_eeb2,
            ],
        ),
    ];

    for (name, seed, expected) in BEFORE {
        let pilot = Pilot::BUILT_IN
            .iter()
            .find(|(spelling, _)| *spelling == name)
            .map(|(_, pilot)| *pilot)
            .expect("a built-in pilot lost its name");
        let drawn = Personality::from_pilot_seed(&pilot, seed);
        let got = [
            drawn.line_bias.to_bits(),
            drawn.wander.to_bits(),
            drawn.wander_rate.to_bits(),
            drawn.look.to_bits(),
            drawn.commitment.to_bits(),
            drawn.patience.to_bits(),
            drawn.trail.to_bits(),
            drawn.width.to_bits(),
            drawn.inside.to_bits(),
            drawn.courtesy.to_bits(),
            drawn.defence.to_bits(),
            drawn.caution.to_bits(),
            drawn.ram.to_bits(),
            drawn.provocation_ticks.to_bits(),
            drawn.trigger.to_bits(),
        ];
        assert_eq!(
            got, expected,
            "{name} at seed {seed:#010x} no longer draws what it did: an axis \
             went into the middle of the draw order rather than onto the end"
        );
    }
}

/// The three roll axes actually draw, and inside their own spans. A frozen
/// order is worth nothing if the axes it froze read zero.
#[test]
fn every_built_in_pilot_draws_its_roll_axes_inside_their_spans() {
    for (name, pilot) in Pilot::BUILT_IN {
        for seed in 1..200u32 {
            let drawn = Personality::from_pilot_seed(&pilot, seed);
            for (axis, span, value) in [
                ("roll_chance", pilot.roll_chance, drawn.roll_chance),
                ("roll_floor", pilot.roll_floor, drawn.roll_floor),
                ("roll_airtime", pilot.roll_airtime, drawn.roll_airtime),
            ] {
                assert!(
                    (span.low..=span.high).contains(&value),
                    "{name} seed {seed}: {axis} drew {value}, outside [{}, {}]",
                    span.low,
                    span.high
                );
            }
        }
    }
}

/// The four must differ in how readily they roll, or the axis is one number
/// wearing four names. A walk over an ordered list, not pairwise `assert!`s:
/// each compares two `const`s, clippy folds them and `assertions_on_constants`
/// fires.
#[test]
fn the_four_built_ins_have_four_different_roll_characters() {
    // Most willing first. The floor and the minimum airborne time both run the
    // other way: the pilot that rolls readily is the one that will dig deepest
    // into the pool for it and wait least for a jump worth spending on.
    let order = [
        ("aggressive", Pilot::AGGRESSIVE),
        ("balanced", Pilot::BALANCED),
        ("passive", Pilot::PASSIVE),
        ("shy", Pilot::SHY),
    ];
    for pair in order.windows(2) {
        let ((keener, willing), (calmer, reluctant)) = (pair[0], pair[1]);
        assert!(
            willing.roll_chance.low > reluctant.roll_chance.low,
            "{keener} does not roll more readily than {calmer}"
        );
        assert!(
            willing.roll_floor.low < reluctant.roll_floor.low,
            "{keener} does not roll off a lower floor than {calmer}"
        );
        assert!(
            willing.roll_airtime.low <= reluctant.roll_airtime.low,
            "{keener} waits longer for a jump than {calmer}"
        );
    }
}
