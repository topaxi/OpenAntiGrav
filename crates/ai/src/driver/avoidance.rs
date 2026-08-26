//! Steering around something laid on the track.
//!
//! One term, in the same fraction-of-the-room units as every other term
//! [`Driver::drift`] sums, so it cannot fight the corridor: the clamp there is
//! the backstop and this is just another opinion about where in the road to be.
//!
//! # It aims to miss the trigger, not to survive the blast
//!
//! A mine's `blastradius` is wider than a Pulse lane and its `trigger_radius` is
//! a few units. So "get outside the blast" is not a manoeuvre a craft can make,
//! and "do not trip it" is - a charge that is never tripped never goes off. The
//! caller measures the threat against the trigger radius for exactly that
//! reason; see [`crate::Hazard`].
//!
//! # Ours, and not much of a claim either way
//!
//! Nothing has been read about what the original's drivers do around a laid
//! charge. `WeaponAi_Update` (`0x08851550`) is where that would live and only
//! its authored *table* has been read - see
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`. What this is, is the
//! smallest term that stops an opponent driving through a mine it can see
//! nothing of, and the reason it was written is fairness rather than fidelity:
//! a charge the player can steer around and an opponent cannot is a worse wrong
//! answer than a dodge nobody has verified.

use super::Driver;
use crate::Hazard;

/// How far ahead a laid charge is worth reacting to, in units.
///
/// **Ours.** Shorter than [`super::AWARENESS_RANGE`], because a charge does not
/// move: it will still be there when the craft is closer, and reacting to one a
/// corner away would mean holding a line for a threat that is not on this
/// stretch of road. Long enough at racing speed to be a lean rather than a
/// swerve - about a second and a half of travel in Venom.
pub const LOOKAHEAD: f32 = 90.0;

/// How much of the corridor a dodge may spend.
///
/// Larger than [`super::SOCIAL_MAX`], and deliberately: yielding to a rival is
/// a courtesy and missing a mine is not. Still short of the whole corridor, so
/// a driver dodging a charge on the apex does not put itself in the wall -
/// [`Driver::drift`] clamps against the corridor either way, and a term that
/// needed the clamp to save it would be one that had stopped being a lean.
const AVOID_MAX: f32 = 0.8;

/// How near the centre a charge has to be before which side it is on stops
/// meaning anything, in units.
///
/// **A quarter of a unit, which is much tighter than the deadband
/// [`Driver::social`] uses on a rival, and it has to be.** That one is
/// [`super::SOCIAL_MIN_GAP`] at fourteen units, sized for a craft four units
/// long deciding which way to move over. A charge is not a craft: the caller
/// only reports one it could actually trip, so every offset that reaches here
/// is already inside a trigger radius plus half a hull - a handful of units at
/// most. A fourteen-unit deadband would swallow **every** charge and send the
/// whole term to the seed, which is what the first version of this file did.
/// It read as a driver dodging in a direction unrelated to where the mine was.
///
/// So this is only asking "is it dead centre", and at a quarter of a unit the
/// answer is almost always no.
const CENTRED: f32 = 0.25;

impl Driver {
    /// Which way to lean to miss the nearest laid charge, and how hard.
    ///
    /// Zero when there is nothing to miss, which is almost always.
    ///
    /// # Three rules, and the third is the one worth reading
    ///
    /// 1. **Urgency rises as the charge nears.** Linear in the closing distance
    ///    over [`LOOKAHEAD`], so a driver leans earlier and harder the later it
    ///    noticed - which on a straight is a smooth drift and in a corner is
    ///    whatever the corridor will allow.
    /// 2. **The lean is away from the side the charge is on**, by the sign of
    ///    [`Hazard::offset`] - the same axis [`crate::Rival::offset`] uses, so
    ///    this and the social lean cannot disagree about which way is right.
    /// 3. **A charge dead ahead has no side to be on**, and its offset's sign is
    ///    then noise - the same problem [`Driver::social`] has with a rival
    ///    directly astern. That one resolves it from the corner ahead; this one
    ///    cannot, because a charge on a straight is exactly the case that
    ///    matters and a straight has no outside. So the side comes from the
    ///    **driver's own seed**: deterministic, fixed for the life of the craft,
    ///    and different between craft - so a field meeting one charge together
    ///    splits around it instead of all diving the same way. "Dead ahead" is
    ///    [`CENTRED`], and sizing that constant is the one thing here that has
    ///    already been got wrong once.
    pub(super) fn avoidance(&self, hazard: Option<Hazard>) -> f32 {
        let Some(hazard) = hazard else {
            return 0.0;
        };
        if hazard.distance <= 0.0 || hazard.distance >= LOOKAHEAD {
            return 0.0;
        }
        let side = if hazard.offset.abs() > CENTRED {
            -hazard.offset.signum()
        } else if self.seed.is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        let urgency = 1.0 - hazard.distance / LOOKAHEAD;
        side * urgency * AVOID_MAX
    }
}
