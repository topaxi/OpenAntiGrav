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
//! The three roll axes ([`Pilot::roll_chance`], [`Pilot::roll_floor`],
//! [`Pilot::roll_airtime`]) are draws 17, 18 and 19, appended in that order on
//! 2026-09-06 and fixed there. `every_built_in_pilot_still_draws_what_it_drew_
//! before_the_roll_axes_were_appended` is the claim that appending them moved
//! no earlier axis of any pilot, against a table of literals captured from the
//! tree before the change - the same guard
//! [`Pilot::BALANCED`]'s own golden table is, one level up.
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
    /// How readily this pilot puts a weapon in the air once it has a target.
    /// Draw 16.
    ///
    /// Rolled once a tick against [`Personality::trigger`], so it sets the
    /// *expected delay* to fire after a target enters the cone rather than a
    /// probability of firing at all - see `Driver::wants_to_fire`.
    pub trigger: Span,
    /// How readily this pilot commits to a barrel roll, **per airborne
    /// window**. Draw 17.
    ///
    /// A true probability and not a rate, unlike [`Self::trigger`]: the driver
    /// makes exactly one decision per flight, on the first tick past
    /// [`Self::roll_airtime`], and lives with it until it lands. See
    /// `Driver::wants_to_roll`.
    ///
    /// **Invented, and the whole axis is a deliberate deviation** - the
    /// original's opponents never barrel-roll at all. See that method.
    pub roll_chance: Span,
    /// The fraction of its shield pool this pilot keeps back rather than
    /// spending on a barrel roll. Draw 18.
    ///
    /// The energy budget the maintainer's directive is phrased against: an
    /// `Ace` rolls "as long as there's energy budget", and this is the budget.
    /// Carried to `oag_physics` on `ShipControls::roll_shield_floor`, which is
    /// where it is enforced - a human's pad leaves that field at `0.0` and gets
    /// the recovered behaviour exactly.
    pub roll_floor: Span,
    /// How long a flight has to have lasted, **in seconds**, before this pilot
    /// thinks it is worth rolling. Draw 19.
    ///
    /// Seconds rather than ticks, unlike [`Self::wander_period`] and
    /// [`Self::provocation_ticks`], because what it is compared against is
    /// `ShipState::time_airborne`, which is seconds. A roll that does not
    /// complete before touchdown is shield spent for nothing, so this is the
    /// axis that keeps a pilot from paying for a hop.
    pub roll_airtime: Span,
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
        trigger: Span::new(0.4, 0.8),
        // Rolls about half the flights that last long enough, and keeps back
        // the fifth of the pool `oag_physics` used to hold as a bare constant.
        roll_chance: Span::new(0.30, 0.55),
        roll_floor: Span::new(0.20, 0.35),
        roll_airtime: Span::new(0.45, 0.70),
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
        trigger: Span::new(0.8, 1.0),
        // Rolls off almost anything and off the lowest floor of the four: the
        // aggressive pilot buys the payout and worries about the pool later.
        roll_chance: Span::new(0.65, 0.95),
        roll_floor: Span::new(0.15, 0.25),
        roll_airtime: Span::new(0.35, 0.55),
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
        trigger: Span::new(0.2, 0.5),
        // Rarely, and only with plenty in hand: this one wants the tidy line
        // more than it wants the payout.
        roll_chance: Span::new(0.10, 0.25),
        roll_floor: Span::new(0.40, 0.60),
        roll_airtime: Span::new(0.65, 0.95),
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
        trigger: Span::new(0.05, 0.3),
        // The most reluctant of the four, off the highest floor. A shy pilot
        // that has spent its pool on showing off is a contradiction.
        roll_chance: Span::new(0.05, 0.15),
        roll_floor: Span::new(0.50, 0.70),
        roll_airtime: Span::new(0.75, 1.10),
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

    /// This pilot, if every span in it is the right way round and no axis asks
    /// for more grip than the hull has.
    ///
    /// The check lives here rather than at the edge that reads files, so a
    /// pilot built in Rust cannot skip it either.
    ///
    /// # Errors
    ///
    /// Names the axis that is wrong, because "a pilot is invalid" is not a
    /// message anyone can act on.
    pub fn validated(self) -> Result<Self, &'static str> {
        if !self.is_well_formed() {
            return Err("a range runs backwards");
        }
        if self.commitment.high > Self::MAX_COMMITMENT {
            return Err("commitment asks for more grip than the hull has");
        }
        Ok(self)
    }

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
    pub fn spans(&self) -> [Span; 18] {
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
            self.trigger,
            self.roll_chance,
            self.roll_floor,
            self.roll_airtime,
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
mod tests;
