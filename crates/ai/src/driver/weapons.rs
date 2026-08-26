//! When a driver pulls the trigger, and which way the weapon points.
//!
//! Split out of `driver.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, and the seam is real rather than convenient:
//! everything here is a *decision to spend a pickup*, and nothing else on
//! [`Driver`] reads a weapon at all.
//!
//! **Both functions are ours**, and it is worth saying once here rather than
//! twice below. `WeaponAi_Update` (`0x08851550`) is where the original decides
//! this, and only its authored *table* has been read - see
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`. What the two share is
//! deliberate: one noise stream, one trigger rate, one personality term, so a
//! pilot that shoots readily also lays mines readily.

use super::{Context, Driver, TRIGGER_RATE, WEAPON_STREAM};
use crate::noise::roll;

/// How far a forward weapon is worth firing, in units.
const WEAPON_RANGE: f32 = 200.0;

/// And how close is too close.
///
/// `oag_gameplay::projectile::blast` damages **every** craft in radius,
/// including the one that fired, so a rocket let go at point-blank is a rocket
/// fired at yourself.
const WEAPON_MIN_RANGE: f32 = 20.0;

/// How far off the nose a target may sit, as a cosine.
///
/// A cosine and never an angle: `docs/architecture/determinism.md` forbids the
/// transcendental, and `Rival::cos_bearing` is a dot product the caller already
/// had. About twenty degrees.
const WEAPON_CONE: f32 = 0.94;

/// How bent the road between here and the target may be before a shot is not
/// worth taking.
const WEAPON_CURVATURE: f32 = 1.0 / 400.0;

/// How close a rival behind has to be before laying mines is worth it.
///
/// **Ours, and it is the mirror of [`WEAPON_RANGE`] rather than a second
/// judgement.** A mine is not aimed, so none of the forward weapon's other
/// gates - the cone, the curvature, the minimum range - transfer: a mine laid
/// with somebody right on your tail is a mine that works, which is the opposite
/// of a rocket fired at point-blank.
///
/// Shorter than the forward range because a cluster covers about sixty units of
/// track and stays where it is put: a rival two hundred units back has time to
/// see it and go round, and the pickup is better spent later.
const MINE_RANGE: f32 = 120.0;

impl Driver {
    /// Whether this driver would put a forward weapon in the air this tick, and
    /// at whom.
    ///
    /// The target slot comes back even though a Rocket is unguided and will not
    /// use it, because the *decision* is the part worth testing and a guided
    /// weapon will want it. A caller that only needs "yes" reads `is_some`.
    ///
    /// Five gates:
    ///
    /// 1. There is a craft ahead at all.
    /// 2. It is inside [`WEAPON_RANGE`] and outside [`WEAPON_MIN_RANGE`] - the
    ///    near bound matters, because the blast catches the firer too.
    /// 3. It is inside the cone, by [`Rival::cos_bearing`].
    /// 4. **The road between here and there is straight enough**, by the same
    ///    `max_curvature` the Turbo gate uses. A rocket round a corner is a
    ///    rocket in a wall, and using the same notion of "is this a straight"
    ///    keeps the two decisions consistent rather than inventing a second.
    /// 5. The trigger roll - see [`TRIGGER_RATE`], which is a rate and not a
    ///    probability.
    ///
    /// **The roll does not touch the world's generator.** It is
    /// `noise::roll(seed, phase, WEAPON_STREAM)`, a pure function of this
    /// driver's own seed and tick count, for the reason
    /// `Personality::from_pilot` gives: a draw from that stream would move every
    /// later pickup roll and make *which craft shoots* depend on how many
    /// pickups had been handed out.
    #[must_use]
    pub fn wants_to_fire(&self, ctx: &Context<'_>) -> Option<u8> {
        let personality = self.personality(ctx.pilot);
        if personality.trigger <= 0.0 {
            return None;
        }
        // **Noticed, not measured.** A driver cannot shoot at a craft it has
        // not seen yet; the clock itself is advanced by [`Self::drive`], which
        // is the tick this one shares. See [`Reflex`].
        let target = self.reflex.filter(ctx.field).ahead?;
        if target.range <= WEAPON_MIN_RANGE || target.range > WEAPON_RANGE {
            return None;
        }
        if target.cos_bearing < WEAPON_CONE {
            return None;
        }
        let span = (ctx.tuning.look_min + ctx.tuning.look_speed * target.range) * 0.5;
        if ctx
            .line
            .max_curvature(self.index as usize, target.range, span)
            > WEAPON_CURVATURE
        {
            return None;
        }
        let appetite = personality.trigger * (1.0 + self.provoked());
        if roll(self.seed, self.phase, WEAPON_STREAM) >= appetite * TRIGGER_RATE {
            return None;
        }
        Some(target.slot)
    }

    /// Whether this driver wants to lay a cluster of mines right now.
    ///
    /// **Nothing about *when* an opponent drops mines is recovered**, the same
    /// way nothing about when one fires a rocket is: `WeaponAi_Update` decides
    /// it in the original and only its *table* has been read (`ai-stats.md`).
    /// So this is the forward weapon's rule with the parts that do not apply
    /// taken out, rather than a second invented policy:
    ///
    /// 1. There is somebody **behind**, noticed rather than merely present -
    ///    the same [`Reflex`] channel [`Self::wants_to_fire`] uses for its own
    ///    direction, so a driver cannot react to a rival it has not seen yet.
    /// 2. That rival is inside [`MINE_RANGE`].
    /// 3. The trigger roll, off the same [`WEAPON_STREAM`] and the same
    ///    [`TRIGGER_RATE`], scaled by the same personality and provocation.
    ///
    /// **No cone, no curvature and no minimum range**, and each absence is the
    /// point rather than an omission: a mine is dropped rather than aimed, a
    /// corner between here and the rival is a *reason* to lay them, and a rival
    /// close enough to touch is the best moment there is. A driver that has just
    /// been passed drops more readily, which `provoked` already does for
    /// shooting.
    ///
    /// Returns the rival's slot, the way [`Self::wants_to_fire`] does, even
    /// though a mine has no target - a caller wanting to log or draw the
    /// decision has the same thing in hand for both weapons.
    #[must_use]
    pub fn wants_to_drop(&self, ctx: &Context<'_>) -> Option<u8> {
        let personality = self.personality(ctx.pilot);
        if personality.trigger <= 0.0 {
            return None;
        }
        let rival = self.reflex.filter(ctx.field).behind?;
        if rival.range > MINE_RANGE {
            return None;
        }
        let appetite = personality.trigger * (1.0 + self.provoked());
        if roll(self.seed, self.phase, WEAPON_STREAM) >= appetite * TRIGGER_RATE {
            return None;
        }
        Some(rival.slot)
    }
}
