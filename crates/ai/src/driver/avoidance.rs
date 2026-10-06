//! Steering around something laid on the track.
//!
//! One term in the fraction-of-the-room units of [`Driver::drift`]'s sum, so it
//! cannot fight the corridor: that clamp is the backstop and this is one more
//! opinion about where in the road to be.
//!
//! # It aims to miss the trigger, not to survive the blast
//!
//! A mine's `blastradius` is wider than a Pulse lane and its `trigger_radius` a
//! few units, so "outside the blast" is not a manoeuvre and "do not trip it" is.
//! The caller measures against the trigger radius for that reason
//! ([`crate::Hazard`]).
//!
//! # Ours, and not much of a claim either way
//!
//! Nothing is read about what the original's drivers do around a laid charge:
//! `WeaponAi_Update` (`0x08851550`) would hold it and only its *table* has been
//! read (`docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`). This is the
//! smallest term that stops an opponent driving through a mine, written for
//! fairness rather than fidelity: a charge the player can dodge and an opponent
//! cannot is the worse wrong answer.

use super::Driver;
use crate::Hazard;

/// How far ahead a laid charge is worth reacting to, in units.
///
/// **Ours.** Shorter than [`super::AWARENESS_RANGE`]: a charge does not move,
/// and reacting to one a corner away holds a line for a threat not on this
/// stretch. Long enough to be a lean, not a swerve: about a second and a half
/// in Venom.
pub const LOOKAHEAD: f32 = 90.0;

/// How much of the corridor a dodge may spend.
///
/// Larger than [`super::SOCIAL_MAX`], deliberately: yielding is a courtesy,
/// missing a mine is not. Short of the whole corridor so a dodge on the apex
/// does not put the craft in the wall; a term needing the clamp to save it has
/// stopped being a lean.
const AVOID_MAX: f32 = 0.8;

/// How near the centre a charge has to be before which side it is on stops
/// meaning anything, in units.
///
/// **A quarter of a unit, much tighter than [`Driver::social`]'s deadband**
/// ([`super::SOCIAL_MIN_GAP`], fourteen, sized for a craft four units long). The
/// caller only reports a charge it could trip, so every offset here is already
/// inside a trigger radius plus half a hull. Fourteen would swallow **every**
/// charge and send the whole term to the seed, which the first version did: a
/// driver dodging in a direction unrelated to the mine.
const CENTRED: f32 = 0.25;

impl Driver {
    /// Which way to lean to miss the nearest laid charge, and how hard. Zero
    /// when there is nothing to miss, which is almost always.
    ///
    /// # Three rules, and the third is the one worth reading
    ///
    /// 1. **Urgency rises as the charge nears**: linear in the distance over
    ///    [`LOOKAHEAD`], so a late-noticing driver leans earlier and harder.
    /// 2. **The lean is away from the side the charge is on**, by the sign of
    ///    [`Hazard::offset`], the axis [`crate::Rival::offset`] uses.
    /// 3. **A charge dead ahead has no side**, and its offset's sign is noise,
    ///    as with a rival astern in [`Driver::social`]. That resolves it from
    ///    the corner ahead; a charge on a straight is exactly the case that
    ///    matters and a straight has no outside. So the side comes from the
    ///    **driver's own seed**: deterministic and different between craft, so a
    ///    field meeting one charge splits around it. "Dead ahead" is
    ///    [`CENTRED`], the constant already got wrong once.
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
