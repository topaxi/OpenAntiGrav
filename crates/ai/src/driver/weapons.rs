//! When a driver pulls the trigger, and which way the weapon points.
//!
//! Split out of `driver.rs` for `scripts/check-file-size.py`; everything here
//! is a *decision to spend a pickup*.
//!
//! # Two weapons, two directions, and the pair must not be confused
//!
//! [`Driver::wants_to_fire`] is for weapons leaving the **nose**,
//! [`Driver::wants_to_drop`] for those leaving the **tail** (the Mine and, when
//! it lands, the Bomb). They read different halves of [`crate::Field`] and
//! neither falls back to the other: a driver reeling somebody in has a rival
//! *ahead* and nobody behind, and a cluster laid then goes where there is
//! nobody to hit. Pinned by `a_driver_does_not_lay_mines_at_a_craft_ahead` and
//! its mirror (the forward gate rejects a craft astern only through the cone,
//! `Rival::cos_bearing` being `-1.0`, so the mirror test states it).
//!
//! **Both functions are ours.** `WeaponAi_Update` (`0x08851550`) decides this
//! in the original; its forward-weapon fire law is ported as
//! [`crate::weapon_ai`] and runs wherever `WeaponAIstats.xml` was read, so
//! [`Driver::wants_to_fire`] is the fallback for a race without it. See
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`. The two share one noise
//! stream, trigger rate and personality term, so a pilot that shoots readily
//! also lays mines readily.

use super::{Context, Driver, TRIGGER_RATE, WEAPON_STREAM};
use crate::noise::roll;

/// How far a forward weapon is worth firing, in units.
const WEAPON_RANGE: f32 = 200.0;

/// And how close is too close: `oag_weapons::projectile::blast` damages
/// **every** craft in radius, the firer included.
const WEAPON_MIN_RANGE: f32 = 20.0;

/// How far off the nose a target may sit, as a cosine (about twenty degrees):
/// `docs/architecture/determinism.md` forbids the transcendental, and
/// `Rival::cos_bearing` is a dot product the caller already had.
const WEAPON_CONE: f32 = 0.94;

/// How bent the road between here and the target may be before a shot is not
/// worth taking.
const WEAPON_CURVATURE: f32 = 1.0 / 400.0;

/// How close a rival behind has to be before laying mines is worth it.
///
/// **Ours, the mirror of [`WEAPON_RANGE`]**: a mine is not aimed, so the cone,
/// curvature and minimum-range gates do not transfer, and a mine laid with
/// somebody on your tail works. Shorter than the forward range because a
/// cluster covers about sixty units and stays put: a rival two hundred back
/// has time to go round it.
const MINE_RANGE: f32 = 120.0;

impl Driver {
    /// Whether this driver would put a forward weapon in the air this tick, and
    /// at whom. The target slot comes back although a Rocket is unguided,
    /// because the *decision* is worth testing and a guided weapon will want it.
    ///
    /// Five gates:
    ///
    /// 1. There is a craft ahead.
    /// 2. It is inside [`WEAPON_RANGE`] and outside [`WEAPON_MIN_RANGE`].
    /// 3. It is inside the cone, by [`Rival::cos_bearing`].
    /// 4. **The road between is straight enough**, by the `max_curvature` the
    ///    Turbo gate uses: a rocket round a corner is a rocket in a wall.
    /// 5. The trigger roll ([`TRIGGER_RATE`], a rate not a probability).
    ///
    /// **The roll does not touch the world's generator**: it is
    /// `noise::roll(seed, phase, WEAPON_STREAM)`, a pure function of this
    /// driver's seed and tick count (see `Personality::from_pilot`), or *which
    /// craft shoots* would depend on how many pickups had been handed out.
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
    /// fired** weapon right now (the Cannon, so far).
    ///
    /// **Chosen, not measured. No confidence score.** The original decides this
    /// with an *uninitialised* byte: `Cannon_UpdateReload` gates on the fire-held
    /// flag of the firing craft's control record, for an opponent `Ai + 0x1e`,
    /// which nothing in the image writes, in an object that is never
    /// zero-filled (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`).
    /// There is no design to be faithful to, and opponent behaviour is a design
    /// axis: **tuned for a good race, not for the heap.**
    ///
    /// A Cannon empties thirty rounds in a second and a half, so held down
    /// unconditionally it sprays an empty track the instant the pickup lands.
    /// This holds the trigger **only while somebody in front is worth hitting**,
    /// by [`Self::wants_to_fire`]'s aiming gates, so the rounds are spent where
    /// they make the race harder. Two differences from `wants_to_fire`:
    ///
    /// - **No trigger roll**: that is a delay before a single shot, and against
    ///   a per-tick reload countdown it would throttle the fire *rate* to a
    ///   twentieth of the authored one. The cadence is the disc's; only the
    ///   decision to hold is ours.
    /// - **No minimum range**: a Cannon round has no blast to catch the firer.
    #[must_use]
    pub fn holds_fire(&self, ctx: &Context<'_>) -> bool {
        if self.personality(ctx.pilot).trigger <= 0.0 {
            return false;
        }
        self.aimable(ctx, 0.0).is_some()
    }

    /// The rival ahead this driver could aim a forward weapon at: noticed, in
    /// range, inside the cone, with straight enough road between.
    ///
    /// **Noticed, not measured**: a driver cannot shoot at a craft it has not
    /// seen yet ([`Reflex`]; the clock advances in [`Self::drive`]).
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
    /// **Nothing about *when* an opponent drops mines is recovered**:
    /// `WeaponAi_Update` decides it and only its *table* has been read
    /// (`ai-stats.md`). So this is the forward weapon's rule minus the parts
    /// that do not apply:
    ///
    /// 1. Somebody **behind**, noticed ([`Reflex`]) rather than merely present.
    /// 2. That rival is inside [`MINE_RANGE`].
    /// 3. The trigger roll, off [`WEAPON_STREAM`] and [`TRIGGER_RATE`], scaled
    ///    by the same personality and provocation.
    ///
    /// **No cone, curvature or minimum range**, each absence the point: a mine
    /// is dropped not aimed, a corner is a *reason* to lay them, and a rival
    /// close enough to touch is the best moment. A driver just passed drops more
    /// readily, as `provoked` does for shooting. Returns the rival's slot like
    /// [`Self::wants_to_fire`], though a mine has no target.
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
