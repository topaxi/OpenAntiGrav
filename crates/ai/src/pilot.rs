//! Named characters a driver can be drawn from.
//!
//! # A pilot is ranges, a personality is one draw from them
//!
//! [`Personality`](crate::Personality) is what a single craft got: six-odd
//! numbers, fixed for the race. A [`Pilot`] is the *distribution* those numbers
//! came out of - so four aggressive craft are four different aggressive
//! drivers, not one aggressive driver four times. That is the same argument
//! that put a personality on each craft in the first place, one level up.
//!
//! # The draw order is frozen
//!
//! **Draws one to seven are what shipped before pilots existed, in that order,
//! for ever. A new axis appends after them; it never goes between.** Two things
//! rest on it:
//!
//! - [`Pilot::BALANCED`] reproduces the pre-pilot personality bit for bit, for
//!   every seed, so introducing pilots moved no behaviour and no world hash.
//!   `the_balanced_pilot_reproduces_the_personality_that_shipped_before_pilots_existed`
//!   is that claim, against a table of literals captured before the change.
//! - Every pilot consumes the *same* draws in the same order, so two pilots
//!   that differ only in their fourth axis still resolve their fifth from the
//!   same place in the stream. An axis inserted in the middle would silently
//!   re-roll every axis after it, for every pilot.
//!
//! An axis a pilot wants to hold fixed uses [`Span::fixed`], which still draws
//! and throws the value away, and a pilot that wants a side uses [`Lean`],
//! which does the same. Skipping the draw would make the sequence depend on
//! *which* pilot was being resolved, which is the same bug from the other end.

use oag_core::Rng;

/// One axis's range, and the single draw that resolves it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    /// The low end, inclusive.
    pub low: f32,
    /// The high end. Approached and not reached - `Rng::next_f32` is half-open.
    pub high: f32,
}

impl Span {
    /// A range.
    #[must_use]
    pub const fn new(low: f32, high: f32) -> Self {
        Self { low, high }
    }

    /// An axis with no variance, which **still consumes its draw**.
    ///
    /// See the note on draw order in [the module docs](self): an axis that
    /// skipped its draw when a pilot happened to fix it would shift every later
    /// axis, and two pilots would disagree about what the same seed means.
    #[must_use]
    pub const fn fixed(value: f32) -> Self {
        Self {
            low: value,
            high: value,
        }
    }

    /// One draw, scaled into the range.
    pub fn draw(&self, rng: &mut Rng) -> f32 {
        self.low + (self.high - self.low) * rng.next_f32()
    }

    /// Whether the range is the right way round.
    #[must_use]
    pub fn is_ordered(&self) -> bool {
        self.low <= self.high
    }
}

/// Which side of the line a pilot leans, before the magnitude is drawn.
///
/// Every variant consumes exactly one draw - see [`Span::fixed`] for why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lean {
    /// A coin flip, which is what the field did before pilots existed.
    Either,
    /// Always the left of the line.
    Left,
    /// Always the right.
    Right,
}

impl Lean {
    /// Applies the lean to a magnitude, consuming the draw either way.
    pub fn sign(self, rng: &mut Rng) -> f32 {
        let flip = rng.next_f32();
        match self {
            Self::Either => {
                if flip < 0.5 {
                    -1.0
                } else {
                    1.0
                }
            }
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }
}

/// A named character: ranges rather than values.
///
/// **Every number in every built-in below is this project's own.** Both games
/// author their AI as tuning data - `Data\XML\AIControlStats.xml` and
/// `AIRaceStats_<class>.xml` - and no value from either appears here, per
/// [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md).
/// These were picked to spread a field around a controller already known to be
/// stable, not fitted to anything.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pilot {
    /// How far off the line this pilot sits, as a fraction of the room on that
    /// side. Draw 1.
    pub line_bias: Span,
    /// Which side that is. Draw 2.
    pub lean: Lean,
    /// How much of the corridor is spent drifting about the bias. Draw 3.
    pub wander: Span,
    /// **Ticks per drift step**, not steps per tick. Draw 4.
    ///
    /// The reciprocal is taken after the draw, because that is the order the
    /// frozen sequence used - drawing the rate directly would land on different
    /// numbers for the same seed.
    pub wander_period: Span,
    /// Multiplier on the lookahead. Draw 5.
    pub look: Span,
    /// Multiplier on `Tuning::lateral_accel`, so on corner speed. Draw 6.
    ///
    /// **The axis that must not be generous.** Over what the hull can hold is
    /// not a faster driver, it is a driver in the wall, and
    /// `no_pilot_asks_for_more_grip_than_the_hull_has` is the ceiling on it.
    pub commitment: Span,
    /// Multiplier on `Tuning::brake_lookahead`. Below one is a late braker.
    /// Draw 7.
    pub patience: Span,
    /// Multiplier on `Tuning::trail_gain` - how readily this pilot spends grip
    /// on rotating the craft. Draw 8.
    pub trail: Span,
    /// Multiplier on `Tuning::corridor_use` - how much of the corridor is used
    /// at all. Draw 9.
    pub width: Span,
    /// Bias toward the inside of whatever corner is ahead, signed by its
    /// curvature. Draw 10.
    ///
    /// Distinct from [`Pilot::lean`], which is a fixed side of the line: an
    /// inside line swaps sides with the corner, which is what a racing driver
    /// actually does and what a fixed lean cannot express.
    pub inside: Span,
    /// How readily this pilot moves **out of the way** of a craft behind it.
    /// Draw 11.
    ///
    /// The opposite sign of [`Pilot::defence`], and the two are resolved as one
    /// number - see `Driver::social`.
    pub courtesy: Span,
    /// How readily this pilot moves **to cover** a craft behind it. Draw 12.
    pub defence: Span,
    /// How early this pilot lifts off for a craft close ahead. Draw 13.
    ///
    /// **The axis that stops the other two causing pile-ups**: courtesy and
    /// defence both sometimes move craft toward each other, and nothing else
    /// here reacts to a closing gap at all.
    pub caution: Span,
    /// How readily this pilot throws the craft sideways at a rival level with
    /// it. Draw 14.
    pub ram: Span,
    /// How long being overtaken stings, in ticks. Draw 15.
    ///
    /// **How provokable a pilot is, expressed as a duration rather than as a
    /// decay rate.** `Driver` must stay `Eq` to live in the world snapshot, so
    /// what it carries is an integer countdown; a per-pilot decay rate would be
    /// an `f32` on that type. This is the same knob from the other end.
    pub provocation_ticks: Span,
}

impl Pilot {
    /// The field as it drove before pilots existed.
    ///
    /// **Its first seven spans are the literals `Personality::from_seed` used**,
    /// which is what makes the refactor that introduced pilots a no-op on
    /// behaviour and on the world hash. Do not retune them without moving the
    /// golden table with them, and read the module docs first.
    pub const BALANCED: Self = Self {
        line_bias: Span::new(0.25, 0.85),
        lean: Lean::Either,
        wander: Span::new(0.10, 0.30),
        wander_period: Span::new(150.0, 420.0),
        look: Span::new(0.85, 1.15),
        commitment: Span::new(0.93, 1.05),
        patience: Span::new(0.85, 1.20),
        trail: Span::new(0.7, 1.1),
        width: Span::new(0.9, 1.1),
        inside: Span::new(0.0, 0.15),
        courtesy: Span::new(0.15, 0.40),
        defence: Span::new(0.15, 0.40),
        caution: Span::new(0.5, 0.9),
        ram: Span::new(0.05, 0.20),
        provocation_ticks: Span::new(120.0, 300.0),
    };

    /// Brakes late, commits hard, holds a tight inside line and rotates the
    /// craft on the airbrakes.
    pub const AGGRESSIVE: Self = Self {
        line_bias: Span::new(0.30, 0.80),
        lean: Lean::Either,
        // Least wander of the four: this one is deliberate about where it is.
        wander: Span::new(0.04, 0.14),
        wander_period: Span::new(150.0, 300.0),
        look: Span::new(0.84, 1.00),
        // Capped at the same ceiling as everyone else - see `commitment`.
        commitment: Span::new(1.00, 1.05),
        patience: Span::new(0.78, 0.95),
        trail: Span::new(1.0, 1.4),
        width: Span::new(0.7, 0.95),
        inside: Span::new(0.25, 0.55),
        courtesy: Span::new(0.0, 0.10),
        defence: Span::new(0.55, 0.95),
        caution: Span::new(0.15, 0.45),
        ram: Span::new(0.55, 0.95),
        provocation_ticks: Span::new(300.0, 600.0),
    };

    /// Looks further ahead, brakes earlier, gives up corner speed for a tidy
    /// line.
    pub const PASSIVE: Self = Self {
        line_bias: Span::new(0.25, 0.70),
        lean: Lean::Either,
        wander: Span::new(0.12, 0.28),
        wander_period: Span::new(220.0, 480.0),
        look: Span::new(1.04, 1.20),
        commitment: Span::new(0.90, 0.99),
        patience: Span::new(1.06, 1.26),
        trail: Span::new(0.3, 0.7),
        width: Span::new(0.95, 1.15),
        inside: Span::new(0.0, 0.10),
        courtesy: Span::new(0.35, 0.65),
        defence: Span::new(0.05, 0.25),
        caution: Span::new(0.7, 1.0),
        ram: Span::new(0.0, 0.05),
        provocation_ticks: Span::new(60.0, 180.0),
    };

    /// Runs wide, brakes earliest, and stays out of everyone's way.
    ///
    /// The yielding half of that is not here: it needs to know a rival is
    /// behind, which is `Field` and a later stage. What this pilot is today is
    /// the driving style that goes with it.
    pub const SHY: Self = Self {
        line_bias: Span::new(0.45, 0.85),
        lean: Lean::Either,
        wander: Span::new(0.15, 0.32),
        wander_period: Span::new(200.0, 460.0),
        look: Span::new(1.00, 1.16),
        commitment: Span::new(0.88, 0.97),
        patience: Span::new(1.10, 1.32),
        trail: Span::new(0.1, 0.5),
        width: Span::new(1.0, 1.2),
        inside: Span::new(-0.10, 0.05),
        courtesy: Span::new(0.70, 1.00),
        defence: Span::new(0.0, 0.08),
        caution: Span::new(0.8, 1.0),
        ram: Span::new(0.0, 0.0),
        provocation_ticks: Span::new(30.0, 120.0),
    };

    /// The four, with the names a config file and a menu spell them by.
    pub const BUILT_IN: [(&'static str, Self); 4] = [
        ("balanced", Self::BALANCED),
        ("aggressive", Self::AGGRESSIVE),
        ("passive", Self::PASSIVE),
        ("shy", Self::SHY),
    ];

    /// The most grip any pilot may assume it has, as a multiple of
    /// `Tuning::lateral_accel`.
    ///
    /// `Tuning::lateral_accel` is deliberately conservative, so a little over
    /// one is a driver using the margin rather than one exceeding the hull.
    /// This is the number a user-authored pilot is checked against, and it is
    /// the only thing standing between a hand-written file and a craft that
    /// corners faster than the physics allows.
    pub const MAX_COMMITMENT: f32 = 1.05;

    /// Whether every span in this pilot is the right way round.
    ///
    /// A reversed span still draws, and lands *outside* the range it appears to
    /// state, with nothing complaining - so this is checked at the edge where
    /// pilots come in from outside the tree.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.spans().iter().all(Span::is_ordered)
    }

    /// Every span, in draw order. `lean` is not one - it is not a range.
    #[must_use]
    pub fn spans(&self) -> [Span; 14] {
        [
            self.line_bias,
            self.wander,
            self.wander_period,
            self.look,
            self.commitment,
            self.patience,
            self.trail,
            self.width,
            self.inside,
            self.courtesy,
            self.defence,
            self.caution,
            self.ram,
            self.provocation_ticks,
        ]
    }
}

impl Default for Pilot {
    fn default() -> Self {
        Self::BALANCED
    }
}

/// Which pilot a grid slot draws, as an index into a roster of `count`.
///
/// **A separately mixed stream from `Driver::for_slot`'s**, and the different
/// multiplier is the whole point: sharing that stream would tie a craft's pilot
/// to its personality seed, so the aggressive slot would always be the one that
/// also drew a high commitment, and a field of eight would have four
/// characters. Nothing here reads a clock or the world's generator.
#[must_use]
pub fn pilot_for_slot(race_seed: u64, slot: u32, count: u32) -> u32 {
    if count == 0 {
        return 0;
    }
    let mixed = race_seed ^ u64::from(slot).wrapping_mul(0x8b4a_2f1d_6c39_e57b);
    Rng::new(mixed).below(count)
}

#[cfg(test)]
mod tests {
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
}
