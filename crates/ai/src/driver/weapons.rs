//! When a driver pulls the trigger, and which way the weapon points.
//!
//! Split out of `driver.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, and the seam is real rather than convenient:
//! everything here is a *decision to spend a pickup*, and nothing else on
//! [`Driver`] reads a weapon at all.
//!
//! # Two weapons, two directions, and the pair must not be confused
//!
//! [`Driver::wants_to_fire`] is for weapons that leave the **nose** and
//! [`Driver::wants_to_drop`] for the two that come out of the **tail** - the
//! Mine and, when it lands, the Bomb. They read different halves of
//! [`crate::Field`] and neither falls back to the other, which is the whole
//! point: a driver reeling somebody in has a rival *ahead* and nobody behind,
//! and a cluster laid then goes on the track behind the overtaker where there
//! is nobody to hit. That is the pickup thrown away for nothing, and from the
//! cockpit it reads as the AI doing something inexplicable. It is pinned by
//! `a_driver_does_not_lay_mines_at_a_craft_ahead` and by its mirror, so a rear
//! weapon wired to the forward gate fails rather than merely looking odd.
//!
//! The forward gate rejects a craft astern on its own, through the cone -
//! `Rival::cos_bearing` is `-1.0` back there - but that is a consequence rather
//! than a statement, so the mirror test states it.
//!
//! **Both functions are ours**, and it is worth saying once here rather than
//! twice below. `WeaponAi_Update` (`0x08851550`) is where the original decides
//! this; its forward-weapon fire law is ported as [`crate::weapon_ai`] and is
//! what a race runs wherever `WeaponAIstats.xml` was read, so
//! [`Driver::wants_to_fire`] is the fallback for a race without it. See
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`. What the two share is
//! deliberate: one noise stream, one trigger rate, one personality term, so a
//! pilot that shoots readily also lays mines readily.

use super::{Context, Driver, TRIGGER_RATE, WEAPON_STREAM};
use crate::noise::roll;

/// How far a forward weapon is worth firing, in units.
const WEAPON_RANGE: f32 = 200.0;

/// And how close is too close.
///
/// `oag_weapons::projectile::blast` damages **every** craft in radius,
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
        let target = self.aimable(ctx, WEAPON_MIN_RANGE)?;
        let appetite = personality.trigger * (1.0 + self.provoked());
        if roll(self.seed, self.phase, WEAPON_STREAM) >= appetite * TRIGGER_RATE {
            return None;
        }
        Some(target.slot)
    }

    /// Whether this driver is holding the trigger down on a **continuously
    /// fired** weapon right now - the Cannon, and nothing else so far.
    ///
    /// **Chosen, not measured. No confidence score.** The original decides this
    /// with an *uninitialised* byte - `Cannon_UpdateReload` gates on the
    /// fire-held flag of the firing craft's control record, which for an
    /// opponent is `Ai + 0x1e`, which nothing in the image ever writes, in an
    /// object that is never zero-filled. See
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`. There is
    /// no design there to be faithful to, and the project's standing ruling is
    /// that opponent behaviour is a design axis rather than a fidelity one:
    /// **tuned for a good race, not for the heap.**
    ///
    /// # What it is tuned for
    ///
    /// A Cannon in the original empties thirty rounds in a second and a half.
    /// Held down unconditionally that is a magazine sprayed at empty track the
    /// instant the pickup lands, which is neither threatening nor interesting -
    /// the player mostly never sees it. So this holds the trigger **only while
    /// there is somebody in front worth hitting**, by exactly the aiming gates
    /// [`Self::wants_to_fire`] uses. The weapon then reads as an opponent
    /// leaning on the player down a straight and letting go through the corner,
    /// and its thirty rounds are spent where they make the race harder.
    ///
    /// **Two deliberate differences from `wants_to_fire`:**
    ///
    /// - **No trigger roll.** That roll is a *delay before a single shot* (see
    ///   [`TRIGGER_RATE`]); against a per-tick reload countdown it would instead
    ///   throttle the fire *rate* to a twentieth of the authored one, which is
    ///   the authored `rate` quietly overridden by an AI constant. The Cannon's
    ///   cadence is the disc's; only the decision to hold is ours.
    /// - **No minimum range.** [`WEAPON_MIN_RANGE`] exists because a blast
    ///   catches the craft that fired it, and a Cannon round has no blast. Point
    ///   blank is exactly where this weapon should be used.
    ///
    /// Everything else is shared with `wants_to_fire` on purpose: a pilot that
    /// will not take a rocket shot round a corner does not hose a Cannon round
    /// one either.
    #[must_use]
    pub fn holds_fire(&self, ctx: &Context<'_>) -> bool {
        if self.personality(ctx.pilot).trigger <= 0.0 {
            return false;
        }
        self.aimable(ctx, 0.0).is_some()
    }

    /// The rival ahead this driver could aim a forward weapon at, if there is
    /// one: noticed, in range, inside the cone, with straight enough road in
    /// between.
    ///
    /// **Noticed, not measured.** A driver cannot shoot at a craft it has not
    /// seen yet; the clock itself is advanced by [`Self::drive`], which is the
    /// tick this one shares. See [`Reflex`].
    fn aimable(&self, ctx: &Context<'_>, min_range: f32) -> Option<crate::Rival> {
        let target = self.reflex.filter(ctx.field).ahead?;
        if target.range <= min_range || target.range > WEAPON_RANGE {
            return None;
        }
        if target.cos_bearing < WEAPON_CONE {
            return None;
        }
        let look = ctx.tuning.look_min + ctx.tuning.look_speed * target.range;
        let span = super::curvature_span(ctx.tuning, look);
        if ctx.line.max_curvature_stepped(
            self.index as usize,
            target.range,
            super::pace::curvature_chord(ctx.tuning, span),
            span,
        ) > WEAPON_CURVATURE
        {
            return None;
        }
        Some(target)
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
