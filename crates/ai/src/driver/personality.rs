//! What makes one driver drive unlike the next.
//!
//! Its own file rather than another block in [`super`], for the reason
//! [`super::tuning`] gives; a move only, and [`Personality`] is re-exported
//! from there so every path that named it still does.

use oag_core::Rng;

use crate::pilot::Pilot;

// Every axis below is a departure from one of `Tuning`'s constants and says so
// by name, so the import is what makes those doc links resolve rather than dead
// text - there is no code here that reads one.
#[allow(unused_imports)]
use super::Tuning;

/// What makes one driver drive unlike the next.
///
/// **Every field is a departure from the [`Tuning`] the whole field shares**,
/// not a replacement for it: a multiplier of one or a bias of zero gives back
/// exactly the driver that was here before this existed. That is deliberate -
/// the shared tuning is what `tests/closed_loop.rs` measured and it stays the
/// centre of the distribution, so the field is spread around a controller that
/// is known to be stable rather than around eight untested ones.
///
/// It is derived from a seed rather than stored, so a driver stays three
/// `u32`s and a replay reproduces it without carrying it. See
/// [`Personality::from_seed`].
///
/// This is the first piece of the skill vector `docs/gameplay/ai.md` describes.
/// Three of its axes are here - line noise, grip used through a corner, how
/// early the braking is - and the ones that are not are named in
/// [what is missing](crate#what-a-personality-does-not-cover).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Personality {
    /// Which side of the line this driver sits on and how far, as a fraction of
    /// the room the corridor gives on that side. Negative is left.
    ///
    /// This is the one that breaks the queue. Eight craft tracking one line to
    /// the centimetre drive nose to tail because there is nowhere else for them
    /// to be; eight craft each holding a different part of the corridor look
    /// like a field.
    pub line_bias: f32,
    /// How much of the corridor this driver spends drifting about its bias, in
    /// the same units. Zero is a driver on rails.
    pub wander: f32,
    /// How fast that drift moves, in noise steps per tick. Small: a wander that
    /// completes in under a couple of seconds reads as a twitch rather than as
    /// a driver.
    pub wander_rate: f32,
    /// Multiplier on the lookahead. Above one is a driver who aims further down
    /// the road, cuts more corner and is smoother; below one is one who chases
    /// the line and looks busier doing it.
    pub look: f32,
    /// Multiplier on [`Tuning::lateral_accel`], so on the speed a corner is
    /// taken at.
    ///
    /// **This is what strings the field out.** Cornering speed goes as the
    /// square root of it, so a spread of a few per cent here is a spread of a
    /// couple of per cent in corner speed - small on one corner, a gap by the
    /// end of a lap. It is also the axis that must not be generous: over what
    /// the hull can hold is not a faster driver, it is a driver in the wall.
    pub commitment: f32,
    /// Multiplier on [`Tuning::brake_lookahead`] - how far ahead this driver
    /// starts worrying about a corner. Below one is a late braker.
    pub patience: f32,
    /// Multiplier on [`Tuning::trail_gain`]: how readily this driver spends
    /// grip on rotating the craft when the steering has run out of lock.
    pub trail: f32,
    /// Multiplier on [`Tuning::corridor_use`]: how much of the corridor this
    /// driver is willing to use at all.
    pub width: f32,
    /// Bias toward the inside of the corner ahead, signed by its curvature, in
    /// the same fraction-of-the-room units as [`Personality::line_bias`].
    ///
    /// Unlike the bias, this **swaps sides with the corner**, which is what a
    /// driver taking a racing line does and what a fixed lean cannot express.
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
    /// The driver that was here before personalities were: shares the field's
    /// tuning exactly and drives the authored line.
    /// **The identity, and every axis added later has to keep it one.** An
    /// appended multiplier is neutral at one and an appended bias at zero, so
    /// this stays the driver that was here before any of it existed - which is
    /// what every exact assertion written against seed zero is measuring.
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
        // **Never asks for a roll.** A chance of zero is the neutral value the
        // way a bias of zero is, and it is what keeps `Driver::default()` the
        // plain line-follower every exact assertion in this crate is written
        // against - a driver that occasionally spends a twelfth of its shield
        // mid-jump is not one of those.
        roll_chance: 0.0,
        // **And a floor of zero, which is deliberately not protection.**
        // `roll_chance` gates `Driver::wants_to_roll` and nothing else; the
        // *gesture* path in `oag_physics::barrel_roll` reads this floor alone,
        // so seed zero's accidental alternation is ungated where a seeded
        // driver's is. That is the right answer rather than an oversight: seed
        // zero is what flies slot 0 under `--autopilot` and under the Autopilot
        // pickup, and the player's own craft should behave like the player's
        // own craft - a human pad carries no floor either. It is also
        // unreachable in practice: an opponent armed **zero** rolls by
        // accidental alternation on all twelve circuits once the grounded gate
        // landed.
        roll_floor: 0.0,
        roll_airtime: 0.0,
    };

    /// This personality with its **driving** axes spent only `share` of the
    /// way out from [`Self::NEUTRAL`]: the line bias, the wander, the inside
    /// line, the lookahead, the patience, the differential and the width.
    ///
    /// What a driver on a speed plan flies with - see `planned::plan_slack`.
    /// The plan was learned by the neutral driver, so every one of these axes
    /// steers a line or brakes at a point the plan never tried; an Ace spends
    /// none of them and a Novice all. **The commitment is not here**, because
    /// it already is the plan's margin, and nothing social, aggressive or
    /// airborne is either: those are about other craft and the roll, not the
    /// line.
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
    /// **Seed zero is [`Self::NEUTRAL`], as a special case rather than by
    /// accident**: `Rng::new(0)` remaps a zero seed, so it would otherwise draw
    /// an ordinary personality and every test written against the shared tuning
    /// would start measuring a random driver instead. It is also what makes
    /// [`Driver::default`] the plain line-follower it has always been.
    ///
    /// The draws are in a fixed order off [`oag_core::Rng`], so this is a pure
    /// function of the seed on every platform: no clock, no OS entropy, and no
    /// draw from the world's own generator - a personality taken from that
    /// stream would move every later pickup roll, and which craft has which
    /// character would depend on how many pickups had been drawn.
    #[must_use]
    pub fn from_seed(seed: u32) -> Self {
        Self::from_pilot_seed(&Pilot::BALANCED, seed)
    }

    /// Derives a personality from a pilot and a seed.
    ///
    /// Seed zero is [`Self::NEUTRAL`] whatever the pilot, with **zero** draws,
    /// for the reason [`Self::from_seed`] gives.
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
    /// order, for ever; a new axis appends after them and never goes between.**
    /// [`Pilot::BALANCED`] holds the spans those seven used, so this reproduces
    /// the pre-pilot personality bit for bit for every seed - which is what
    /// made introducing pilots a change to no behaviour and no world hash.
    /// `pilot::tests::the_balanced_pilot_reproduces_the_personality_that_shipped_before_pilots_existed`
    /// is that claim, against literals captured before the change.
    ///
    /// Sequential lets rather than a closure in a struct literal, because the
    /// *order* of the draws is what makes this reproducible and a struct
    /// literal's field order is a thing a later edit moves without thinking.
    /// Two craft that drew the same values in a different order are two
    /// different craft.
    ///
    /// Takes the generator rather than a seed so a caller can prove two pilots
    /// consume the same draws by comparing `Rng::snapshot` afterwards.
    #[must_use]
    pub fn from_pilot(pilot: &Pilot, rng: &mut Rng) -> Self {
        // Rarely near zero: the middle of the corridor is where everyone would
        // be anyway.
        let magnitude = pilot.line_bias.draw(rng);
        let line_bias = magnitude * pilot.lean.sign(rng);
        let wander = pilot.wander.draw(rng);
        // Drawn as a period and stored as a rate. Two and a half to seven
        // seconds a step at 60 Hz for the balanced pilot - slower than a
        // corner, so the drift is something a driver *is* rather than something
        // that happens to it mid-bend.
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
        // Draws 17, 18 and 19, appended 2026-09-06 and fixed in this order.
        // Everything above them is untouched by their arrival, which is the
        // whole reason they are here rather than beside the axes they are
        // about - see the module docs on [`Pilot`].
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
