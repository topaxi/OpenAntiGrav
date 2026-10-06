//! What makes one driver drive unlike the next.
//!
//! Split out of [`super`] like [`super::tuning`]; [`Personality`] is
//! re-exported from there.

use oag_core::Rng;

use crate::pilot::Pilot;

// Every axis below departs from a `Tuning` constant by name; the import makes
// those doc links resolve, no code here reads one.
#[allow(unused_imports)]
use super::Tuning;

/// What makes one driver drive unlike the next.
///
/// **Every field is a departure from the shared [`Tuning`]**: a multiplier of
/// one or a bias of zero gives back exactly the driver that was here before.
/// The shared tuning is what `tests/closed_loop.rs` measured and stays the
/// centre of the distribution, so the field spreads around a controller known
/// to be stable.
///
/// Derived from a seed rather than stored (a driver stays three `u32`s and a
/// replay reproduces it): [`Personality::from_seed`]. The first piece of the
/// skill vector `docs/gameplay/ai.md` describes; the axes not here are in
/// [what is missing](crate#what-a-personality-does-not-cover).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Personality {
    /// Which side of the line this driver sits on and how far, as a fraction of
    /// the room on that side. Negative is left. This breaks the queue: eight
    /// craft tracking one line drive nose to tail, eight each holding a
    /// different part of the corridor look like a field.
    pub line_bias: f32,
    /// How much of the corridor this driver spends drifting about its bias.
    /// Zero is a driver on rails.
    pub wander: f32,
    /// How fast that drift moves, in noise steps per tick. Small: under a couple
    /// of seconds a step reads as a twitch.
    pub wander_rate: f32,
    /// Multiplier on the lookahead. Above one aims further down the road, cuts
    /// more corner and is smoother; below one chases the line.
    pub look: f32,
    /// Multiplier on [`Tuning::lateral_accel`], so on corner speed.
    ///
    /// **This strings the field out.** Speed goes as its square root, so a few
    /// per cent here is a couple in corner speed: small on one corner, a gap by
    /// the end of a lap. It must not be generous: over what the hull can hold is
    /// a driver in the wall.
    pub commitment: f32,
    /// Multiplier on [`Tuning::brake_lookahead`]: how far ahead it worries about
    /// a corner. Below one is a late braker.
    pub patience: f32,
    /// Multiplier on [`Tuning::trail_gain`]: how readily this driver spends
    /// grip on rotating the craft when the steering has run out of lock.
    pub trail: f32,
    /// Multiplier on [`Tuning::corridor_use`]: how much of the corridor this
    /// driver is willing to use at all.
    pub width: f32,
    /// Bias toward the inside of the corner ahead, signed by its curvature, in
    /// [`Personality::line_bias`]'s units. Unlike the bias it **swaps sides with
    /// the corner**, as a racing line does.
    pub inside: f32,
    /// How readily this driver moves out of the way of a craft behind it.
    pub courtesy: f32,
    /// How readily this driver moves to cover one.
    pub defence: f32,
    /// How early it lifts off for a craft close ahead.
    pub caution: f32,
    /// How readily it throws the craft sideways at a rival level with it.
    pub ram: f32,
    /// How many ticks of provocation being overtaken is worth.
    pub provocation_ticks: f32,
    /// How readily it puts a weapon in the air once it has a target.
    pub trigger: f32,
    /// How readily it commits to a barrel roll, per airborne window.
    pub roll_chance: f32,
    /// The fraction of its shield pool it keeps back from one.
    pub roll_floor: f32,
    /// How long a flight has to last, in seconds, before it is worth rolling.
    pub roll_airtime: f32,
}

impl Personality {
    /// The driver that was here before personalities: the field's tuning
    /// exactly, on the authored line.
    ///
    /// **The identity, and every later axis must keep it one**: an appended
    /// multiplier is neutral at one and a bias at zero, since every exact
    /// assertion written against seed zero measures this.
    pub const NEUTRAL: Self = Self {
        line_bias: 0.0,
        wander: 0.0,
        wander_rate: 0.0,
        look: 1.0,
        commitment: 1.0,
        patience: 1.0,
        trail: 1.0,
        width: 1.0,
        inside: 0.0,
        courtesy: 0.0,
        defence: 0.0,
        caution: 0.0,
        ram: 0.0,
        provocation_ticks: 0.0,
        trigger: 0.0,
        // **Never asks for a roll**: a chance of zero is neutral, and keeps
        // `Driver::default()` the plain line-follower; one that spends a
        // twelfth of its shield mid-jump is not.
        roll_chance: 0.0,
        // **A floor of zero, deliberately not protection.** `roll_chance` gates
        // `Driver::wants_to_roll` only; `oag_physics::barrel_roll`'s *gesture*
        // path reads this floor alone, so seed zero's accidental alternation is
        // ungated. Right, not an oversight: seed zero flies slot 0 under
        // `--autopilot` and the Autopilot pickup, and a human pad carries no
        // floor either. Unreachable in practice anyway: an opponent armed
        // **zero** rolls by accidental alternation on all twelve circuits once
        // the grounded gate landed.
        roll_floor: 0.0,
        roll_airtime: 0.0,
    };

    /// This personality with its **driving** axes spent only `share` of the way
    /// out from [`Self::NEUTRAL`]: line bias, wander, inside line, lookahead,
    /// patience, differential and width.
    ///
    /// What a driver on a speed plan flies with (`planned::plan_slack`). The
    /// plan was learned by the neutral driver, so each axis steers a line or
    /// brakes at a point it never tried: an Ace spends none, a Novice all.
    /// **Commitment is not here** (it already is the plan's margin), nor
    /// anything social, aggressive or airborne: those are about other craft and
    /// the roll, not the line.
    #[must_use]
    pub fn spent(&self, share: f32) -> Self {
        let toward = |value: f32, neutral: f32| neutral + (value - neutral) * share;
        Self {
            line_bias: self.line_bias * share,
            wander: self.wander * share,
            inside: self.inside * share,
            look: toward(self.look, 1.0),
            patience: toward(self.patience, 1.0),
            trail: toward(self.trail, 1.0),
            width: toward(self.width, 1.0),
            ..*self
        }
    }

    /// Derives a personality from a seed.
    ///
    /// **Seed zero is [`Self::NEUTRAL`], a special case**: `Rng::new(0)` remaps
    /// a zero seed, so it would otherwise draw an ordinary personality and every
    /// test against the shared tuning would measure a random driver. It also
    /// keeps [`Driver::default`] the plain line-follower.
    ///
    /// The draws are in a fixed order off [`oag_core::Rng`], a pure function of
    /// the seed on every platform. None come from the world's generator: that
    /// would move every later pickup roll and make a craft's character depend
    /// on how many pickups had been drawn.
    #[must_use]
    pub fn from_seed(seed: u32) -> Self {
        Self::from_pilot_seed(&Pilot::BALANCED, seed)
    }

    /// Derives a personality from a pilot and a seed. Seed zero is
    /// [`Self::NEUTRAL`] whatever the pilot, with **zero** draws.
    #[must_use]
    pub fn from_pilot_seed(pilot: &Pilot, seed: u32) -> Self {
        if seed == 0 {
            return Self::NEUTRAL;
        }
        Self::from_pilot(pilot, &mut Rng::new(u64::from(seed)))
    }

    /// Draws a personality out of a pilot's ranges.
    ///
    /// # The draw order is frozen
    ///
    /// **Draws one to seven are what shipped before pilots existed, in this
    /// order, for ever; a new axis appends after them.** [`Pilot::BALANCED`]
    /// holds those spans, so this reproduces the pre-pilot personality bit for
    /// bit for every seed:
    /// `pilot::tests::the_balanced_pilot_reproduces_the_personality_that_shipped_before_pilots_existed`
    /// against literals captured before the change.
    ///
    /// Sequential lets, not a closure in a struct literal: draw *order* makes
    /// this reproducible and a literal's field order moves without thought.
    ///
    /// Takes the generator so a caller can prove two pilots consume the same
    /// draws by comparing `Rng::snapshot`.
    #[must_use]
    pub fn from_pilot(pilot: &Pilot, rng: &mut Rng) -> Self {
        // Rarely near zero: the middle of the corridor is where everyone is.
        let magnitude = pilot.line_bias.draw(rng);
        let line_bias = magnitude * pilot.lean.sign(rng);
        let wander = pilot.wander.draw(rng);
        // Drawn as a period, stored as a rate: 2.5 to 7 seconds a step at 60 Hz
        // for the balanced pilot, slower than a corner, so the drift is what a
        // driver *is* rather than what happens to it mid-bend.
        let wander_rate = 1.0 / pilot.wander_period.draw(rng);
        let look = pilot.look.draw(rng);
        let commitment = pilot.commitment.draw(rng);
        let patience = pilot.patience.draw(rng);
        let trail = pilot.trail.draw(rng);
        let width = pilot.width.draw(rng);
        let inside = pilot.inside.draw(rng);
        let courtesy = pilot.courtesy.draw(rng);
        let defence = pilot.defence.draw(rng);
        let caution = pilot.caution.draw(rng);
        let ram = pilot.ram.draw(rng);
        let provocation_ticks = pilot.provocation_ticks.draw(rng);
        let trigger = pilot.trigger.draw(rng);
        // Draws 17, 18 and 19, appended 2026-09-06 in this order, leaving
        // everything above untouched; see the module docs on [`Pilot`].
        let roll_chance = pilot.roll_chance.draw(rng);
        let roll_floor = pilot.roll_floor.draw(rng);
        let roll_airtime = pilot.roll_airtime.draw(rng);

        Self {
            line_bias,
            wander,
            wander_rate,
            look,
            commitment,
            patience,
            trail,
            width,
            inside,
            courtesy,
            defence,
            caution,
            ram,
            provocation_ticks,
            trigger,
            roll_chance,
            roll_floor,
            roll_airtime,
        }
    }
}

impl Default for Personality {
    fn default() -> Self {
        Self::NEUTRAL
    }
}
