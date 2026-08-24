//! The energy pool, and what a wall costs it.
//!
//! Recovered in [`shield.md`], which this module is the port of. The original's
//! shape, in three functions:
//!
//! ```text
//! Ship_ResetShield(craft)          shield = misc.shield[skill]
//! Ship_Damage(amount, craft, ...)  if (!weapons) amount *= 0.5
//!                                  shield = min(shield - amount, max)
//!                                  if (shield <= 0) SetState(craft, Destroyed)
//! FUN_088418e0's contact loop       Ship_Damage(|p| * 0.05 * 0.7, craft, ...)
//! ```
//!
//! Only the collision leg is here. Weapon damage, weapon absorb and the pit-lane
//! recharge are the other three things that reach this pool in the original, and
//! none of them has anything to hand out yet - see the module's open list at the
//! bottom of this comment.
//!
//! # Why the pool lives in `oag_physics`
//!
//! It is not a dynamical quantity: nothing in the force law reads it, and a race
//! runs identically with it pinned at full. What decides the crate is the
//! *writer* rather than the reader - the amount is a function of the contact
//! impulse, which only this crate computes, and the original keeps the pool on
//! the craft beside the timers this crate already owns. Putting it in
//! `oag_gameplay` would mean either exporting per-contact impulses across the
//! crate boundary or refitting the damage from a reported summary, and it would
//! put a simulation field outside what `crate::probe`'s determinism hash can
//! reach. See [`crate::ShipState::shield`].
//!
//! # What is not implemented, and is not guessed at
//!
//! - **The destroyed transition.** `Ship_Damage` sets craft state 4 at zero or
//!   below; this engine has no such state, so [`apply_contact`] floors the pool
//!   instead and [`Shield::depleted`] is the signal a caller can act on. That
//!   signal is what [Zone](../../../docs/gameplay/race-modes.md) needs for its
//!   own missing end condition.
//! - **`Ship_Damage`'s `weapon_kind` sub-bucket**, nine cases, unmapped - and
//!   the absorb-spark effect its `source == 2` branch triggers. [`apply_weapon`]
//!   is that branch's damage half and nothing else.
//! - **The respawn cost.** `Ship_SetState`'s state-3 branch computes
//!   `clamp(shield - 1, 0, 5)` and stores it; what consumes it is unread.
//! - **The `SkillLevel` race option.** The pool arrives already resolved for a
//!   skill level - see `oag_gameplay::handling::DEFAULT_SKILL_LEVEL`.
//!
//! [`shield.md`]: ../../../docs/ghidra/functions/psp-pulse-usa/shield.md

use crate::{ShipState, params::Dimensions, wall::WallResponse};

/// The scale from a contact's impulse magnitude to energy lost, `0.05 * 0.7`.
///
/// Both literals are the original's, from `FUN_088418e0`'s contact loop: it
/// computes `|p| * 0.05` for the reaction and passes `that * 0.7` to
/// `Ship_Damage`. Kept as the product of the two rather than as `0.035` so the
/// pair stays recognisable against the decompile - and because the same `0.05`
/// and `0.7` appear in [`crate::wall`]'s scrape friction, which is not a
/// coincidence and is the point of naming them the same way twice.
///
/// **Confidence 94, measured against the running original.** A breakpoint on
/// `Ship_Damage` - where the contact ring is still intact, unlike at
/// `Ship_UpdateCraft`'s entry - puts `amount / |p|` at `0.035000` on 25 of 25
/// calls, minimum and maximum identical to six figures over amounts spanning a
/// factor of 225. The same 25 calls also confirm **one call per ring record**:
/// a tick with four contacts produced four calls, each matching a different
/// record. That is what [`crate::wall::WallResponse::impulse_sum`] sums for.
pub const CONTACT_DAMAGE_SCALE: f32 = 0.05 * 0.7;

/// What `Ship_Damage` multiplies the amount by when the race has weapons off.
///
/// `if (g_weapons_enabled == 0) amount *= 0.5`, and it applies to *collision*
/// damage as much as to weapon damage - the option that removes weapons also
/// halves what a wall costs. A recovered rule and a counter-intuitive one, which
/// is why it is a named constant rather than a `0.5` inline.
pub const NO_WEAPONS_DAMAGE_SCALE: f32 = 0.5;

/// Where the energy-critical warning fires, as a percentage of the pool.
///
/// `DAT_08a7b6a4`, read from `.rodata` as `20.0`. The original tests the
/// percentage before *and* after the subtraction and warns only on the downward
/// crossing, which is what [`Shield::crossed_critical`] reproduces - a level test
/// would re-fire on every tick of a scrape.
pub const CRITICAL_PERCENT: f32 = 20.0;

/// Where a craft is in the destroyed sequence.
///
/// The original keeps a small state machine on the entity at `+0x8c`, read and
/// written by `Ship_State` (`0x0883e64c`) and `Ship_SetState` (`0x08844100`).
/// **Only three of its nine states are modelled here**, because they are the
/// three the energy pool reaches; the rest are the race start, the respawn and
/// the Eliminator's re-insertion, and inventing them would be claiming a
/// sequence nobody has read.
///
/// The numbers beside each variant are the original's own, so a future pass can
/// line them up without re-deriving the mapping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CraftState {
    /// State 1. Racing, and the only state that takes damage.
    #[default]
    Racing,
    /// State 4. The explosion, which runs for [`DESTROYED_DURATION`].
    ///
    /// `Ship_SetState`'s case 4 plays `~BLOWUP`, hides the HUD and puts the
    /// camera in mode 5. **The sound is built** - `oag_game::audio::sfx` holds
    /// it for exactly this state - and the other two are not: case 4's HUD and
    /// camera work goes through four globals none of which is identified. The
    /// addresses are on
    /// [`zone-mode.md`](../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md)
    /// so the next reader starts from them rather than from a summary.
    Destroyed,
    /// State 5. Out of the race.
    ///
    /// This is the state that matters to a game mode: `Ship_SetState`'s case 5
    /// **sets bit 12 (`0x1000`) of `entity+0x860`**, which is the bit
    /// `Zone_UpdateRacing` ends a run on -
    /// [`zone-mode.md`](../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md)
    /// recorded that bit as set by nothing findable, and this is what sets it.
    ///
    /// The original moves on from here after 1.5 s, into a respawn or the
    /// Eliminator's kill bookkeeping. **This engine stops here**, because a race
    /// with one craft in it has nothing to move on to.
    Eliminated,
}

/// How long the explosion runs before the craft is out of the race, in seconds.
///
/// `Ship_SetState`'s case 4 writes `0.5` into the state timer at `entity+0x874`,
/// and state 4's own per-frame update (`0x088404c8`) is four lines: subtract
/// `dt`, and at or below zero go to state 5. Confidence **88** - both halves are
/// unambiguous, and neither has a runtime leg.
pub const DESTROYED_DURATION: f32 = 0.5;

/// The race options that reach the pool.
///
/// Two of the three globals `Race_ReadSetupOptions` (`0x08896b84`) lowers; the
/// third, `g_skill_level`, is consumed before this crate sees anything (it picks
/// which of `<Misc>`'s three slots becomes [`Dimensions::shield`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRules {
    /// `g_weapons_enabled` (`0x08b30fa8`). With weapons off, every amount is
    /// scaled by [`NO_WEAPONS_DAMAGE_SCALE`].
    pub weapons: bool,
    /// `g_damage_enabled` (`0x08b30fa9`).
    ///
    /// **This does not gate the subtraction**, which is the trap worth carrying
    /// in the type: with it off the original still runs `Ship_Damage` in full and
    /// separately regenerates the pool at [`REGENERATION_PER_SECOND`] with a
    /// floor of [`REGENERATION_FLOOR`]. A craft cannot be destroyed because the
    /// floor is above zero, not because nothing hurts it. See [`regenerate`].
    pub damage: bool,
}

impl Default for DamageRules {
    /// Weapons and damage both on, which is what
    /// `Race_ReadSetupOptions`' per-mode default is for every mode except three.
    fn default() -> Self {
        Self {
            weapons: true,
            damage: true,
        }
    }
}

/// Energy a second the pool regains while damage is off.
pub const REGENERATION_PER_SECOND: f32 = 4.0;

/// The floor that regeneration raises the pool to while damage is off.
pub const REGENERATION_FLOOR: f32 = 20.0;

/// What one tick did to the pool.
///
/// Returned rather than signalled through [`ShipState`] because both fields are
/// **edges**, and an edge stored on the state would have to be cleared by
/// whoever read it. A caller that ignores the return value loses a sound cue and
/// nothing else.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Shield {
    /// Energy lost this tick, after every scale. Zero on a tick with no contact.
    pub lost: f32,
    /// Whether this tick took the pool from above [`CRITICAL_PERCENT`] to at or
    /// below it. The `energycritical` cue's own trigger.
    pub crossed_critical: bool,
    /// Whether the pool reached zero this tick.
    ///
    /// The original transitions the craft into its destroyed state here. Nothing
    /// does yet; this is the signal to build that on, and the one
    /// [`Zone`](../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md)'s
    /// unimplemented end condition has been waiting for.
    pub depleted: bool,
    /// Whether a fired Shield pickup swallowed this tick's damage.
    ///
    /// **The original's own edge, not a convenience.** Both of its damage drains
    /// take the shield branch *instead of* subtracting, and both then call
    /// `ShipShield_Hit` (`0x0885eb04`) - so absorbing a hit is what makes the
    /// shell flash and bulge. Without this the shield is invisible on the one
    /// tick a player is looking at it.
    ///
    /// Set only when there was something to swallow: a zero-energy graze against
    /// a shield is not a hit, and firing the flash on it would strobe the shell
    /// for every tick of a scrape. See
    /// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
    pub absorbed: bool,
}

/// Advance the destroyed sequence, `0x088404c8`.
///
/// Separate from [`apply_contact`] because it runs on every tick rather than on
/// a tick with contacts, and because a caller driving the force law directly
/// still wants the craft to finish blowing up.
pub fn advance_state(state: &mut ShipState, dt: f32) {
    if state.craft_state != CraftState::Destroyed {
        return;
    }
    state.state_timer -= dt;
    if state.state_timer <= 0.0 {
        state.state_timer = 0.0;
        state.craft_state = CraftState::Eliminated;
    }
}

/// Counts a fired Shield pickup down.
///
/// [`crate::engine::advance_turbo`]'s twin, and deliberately identical to it:
/// called once a tick from [`crate::step`] **after** [`apply_contact`] has read
/// the timer, so the tick a shield is fired on is protected rather than skipped,
/// and floored at zero so `> 0.0` is the whole of the gate.
///
/// It therefore inherits the same one-tick-longer character `advance_turbo`
/// documents and measures - a `0.75` second pickup protects for 46 ticks at
/// 60 Hz, not 45, because the timer is read before it is decremented. Pinned by
/// `a_fired_shield_refuses_damage_for_its_authored_duration` so that reordering
/// the call fails a test rather than moving a number nobody is watching.
pub fn advance_shield_pickup(state: &mut ShipState, dt: f32) {
    state.shield_pickup_timer = (state.shield_pickup_timer - dt).max(0.0);
}

/// Fill the pool, `Ship_ResetShield` (`0x0883dd24`).
///
/// The only thing on the disc that sets the pool to its maximum outright; a
/// clean-zone recharge *adds* through `Ship_AddShield` instead. Call it when a
/// ship takes the grid, not on every spawn pose - the pool is a race-scale
/// quantity.
pub fn reset(state: &mut ShipState, dimensions: &Dimensions) {
    state.shield = dimensions.shield;
    state.craft_state = CraftState::Racing;
    state.state_timer = 0.0;
    // Ours, like the timer itself: taking the grid is a race-scale event and a
    // shield left running from the previous race would protect a craft that
    // never collected one. `turbo_timer` is deliberately *not* cleared here -
    // that is a separate open question recorded in `HANDOVER.md`, and changing
    // it in this pass would bury a behaviour change inside a shield.
    state.shield_pickup_timer = 0.0;
}

/// The energy a frame's contacts cost, before the pool is touched.
///
/// Separated from [`apply_contact`] so a test can assert the law without a
/// `ShipState`, and so a future weapon path can reuse the scaling half.
#[must_use]
pub fn contact_damage(impulse_sum: f32, rules: DamageRules) -> f32 {
    let amount = impulse_sum * CONTACT_DAMAGE_SCALE;
    if rules.weapons {
        amount
    } else {
        amount * NO_WEAPONS_DAMAGE_SCALE
    }
}

/// Apply one tick of wall contact to the pool, then the damage-off regeneration.
///
/// The ordering is the original's: `FUN_088418e0` regenerates near the top of the
/// craft update and walks the contact ring near the bottom, so within one frame
/// the regeneration is applied to the *previous* frame's pool. Reproducing that
/// exactly would need the pool split across the tick, and the difference is one
/// frame of a 4-a-second ramp - `0.067` energy at 60 Hz. Doing it in this order
/// instead is a knowing simplification, recorded here rather than left to be
/// rediscovered as a discrepancy.
pub fn apply_contact(
    state: &mut ShipState,
    dimensions: &Dimensions,
    wall: &WallResponse,
    rules: DamageRules,
) -> Shield {
    let report = subtract(state, dimensions, contact_damage(wall.impulse_sum, rules));
    regenerate(state, dimensions, rules, 0.0);
    report
}

/// Apply one weapon's damage to the pool, `Ship_Damage` with `source == 2`.
///
/// The same function [`apply_contact`] is, differing only in where the amount
/// comes from: a contact scales an impulse by [`CONTACT_DAMAGE_SCALE`], and a
/// weapon passes the disc's own `<Stats damage>` through unchanged. Everything
/// after that - the state gate, the weapons-off halving, the clamp, the two
/// edges and the destroyed transition - is shared, which is the point of it
/// being one [`subtract`] rather than two similar bodies.
///
/// **Does not regenerate.** [`apply_contact`] does, because it is called from
/// inside [`crate::step`] and the floor has to land in the same tick as the
/// loss; this is called from outside the step by whoever owns the projectile,
/// and the tick's own [`regenerate`] has already run or is about to.
///
/// `source == 2` is what the original calls a weapon hit, at confidence 75 -
/// it rests on the telemetry bucket layout rather than on a string, and it is
/// the *only* part of the weapon-damage path with any evidence behind it. The
/// `weapon_kind` sub-bucket - nine cases, unmapped - is not reproduced, and
/// neither is the absorb-spark effect that branch triggers. See
/// `docs/ghidra/functions/psp-pulse-usa/shield.md`.
pub fn apply_weapon(
    state: &mut ShipState,
    dimensions: &Dimensions,
    amount: f32,
    rules: DamageRules,
) -> Shield {
    subtract(state, dimensions, weapon_damage(amount, rules))
}

/// The energy one weapon hit costs, before the pool is touched.
///
/// [`contact_damage`]'s twin: the authored amount, halved when the race has
/// weapons off. That halving cannot be reached today - a weapons-off race arms
/// no pads and hands out no weapon - and it is applied anyway because
/// `Ship_Damage` applies it to *every* amount regardless of source, which is the
/// recovered rule. Reproducing it here rather than skipping it means the day
/// something else reaches this path, the rule is already right.
#[must_use]
pub fn weapon_damage(amount: f32, rules: DamageRules) -> f32 {
    if rules.weapons {
        amount
    } else {
        amount * NO_WEAPONS_DAMAGE_SCALE
    }
}

/// `Ship_Damage`'s body: the state gate, the clamp, the two edges and the
/// destroyed transition, given an amount that is already scaled.
///
/// Shared by [`apply_contact`] and [`apply_weapon`] so the two cannot drift -
/// the gate in particular, which is what makes a shielded or already-destroyed
/// craft immune to *both* and not just to whichever one was written first.
fn subtract(state: &mut ShipState, dimensions: &Dimensions, lost: f32) -> Shield {
    // `Ship_Damage` refuses outright outside the racing states - a craft already
    // blowing up takes no further damage. The original's gate lists states 4, 5,
    // 6 and 2; the three this crate models collapse to "not racing".
    if state.craft_state != CraftState::Racing {
        return Shield::default();
    }

    // A fired Shield pickup refuses on the same edge, and **that is recovered as
    // of 2026-08-19** - this comment used to say the refusal was ours.
    //
    // The original has two damage drains and the Shield gates both. Weapon
    // damage is posted to `craft+0x120` and drained by
    // `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`), which calls `Ship_Damage`
    // only when `craft+0x1b8 & 0x10` is clear; contact damage is the contact
    // loop's reaction #2, which the same bit replaces with reaction #3. In both,
    // the amount is *discarded* rather than deferred, and both then arm
    // `ShipShield_Hit`. See
    // `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
    //
    // Refusing *here* rather than after the subtraction is what the original
    // does too, and it is load-bearing: it suppresses `depleted` and
    // `crossed_critical` with it, so a shielded craft cannot be destroyed and
    // cannot fire the energy-critical cue.
    //
    // What the shield does **not** do is change the wall's own impulse. It
    // suppresses the *posted* one - see [`crate::wall::STUN_PER_CONTACT`] - and
    // nothing on the track path posts that. A shielded craft still bounces off a
    // wall exactly as an unshielded one does; it simply pays nothing for it.
    if state.shield_pickup_timer > 0.0 {
        return Shield {
            absorbed: lost > 0.0,
            ..Shield::default()
        };
    }

    let max = dimensions.shield;
    let before = state.shield;

    // `Ship_SetShield` clamps above and never below; the floor is ours and
    // `ShipState::shield` says why.
    state.shield = (before - lost).clamp(0.0, max);

    let depleted = state.shield <= 0.0 && before > 0.0;
    if depleted {
        // `Ship_Damage`'s `if (new <= 0.0) Ship_SetState(entity, 4)`.
        state.craft_state = CraftState::Destroyed;
        state.state_timer = DESTROYED_DURATION;
    }
    let crossed_critical =
        percent(before, max) > CRITICAL_PERCENT && percent(state.shield, max) <= CRITICAL_PERCENT;

    Shield {
        lost,
        crossed_critical,
        depleted,
        absorbed: false,
    }
}

/// The damage-off regeneration, `shield += dt * 4` floored at `20`.
///
/// Split out and called with `dt` from [`apply_contact`]'s caller rather than
/// folded in, because it runs whether or not the frame had a contact. Passing
/// `0.0` applies the floor alone, which is what [`apply_contact`] wants: the
/// floor is what makes the pool undepletable in that mode, and it must land in
/// the same tick as the loss rather than one later.
pub fn regenerate(state: &mut ShipState, dimensions: &Dimensions, rules: DamageRules, dt: f32) {
    if rules.damage {
        return;
    }
    let regenerated = (state.shield + dt * REGENERATION_PER_SECOND).max(REGENERATION_FLOOR);
    state.shield = regenerated.min(dimensions.shield);
}

/// Adds energy to the pool, clamped at the ship's maximum.
///
/// `Ship_AddShield` (`0x0883ddc8`) adds a delta and routes it through
/// `Ship_SetShield` (`0x0883e6f4`), which takes `min(amount, max)` against the
/// skill-indexed `<Misc>` maximum and floors nothing - see
/// `docs/ghidra/functions/psp-pulse-usa/shield.md`. **So the clamp above is
/// recovered**, and it is the reason this exists rather than callers writing
/// `state.shield += delta`: an unclamped pool draws a `ShieldBar` past full and
/// reports over 100 %.
///
/// The one caller today is absorbing a pickup, which pays back
/// `<Weapon><Stats absorb>`. Zone's perfect-zone recharge is the original's
/// other `Ship_AddShield` caller and predates this; it writes the field
/// directly and should move here.
///
/// Returns how much actually landed, which is less than `delta` against a full
/// pool. A caller that wants to say "absorbed" only when something happened
/// reads it; the sound cue the original plays is not implemented either way.
pub fn add(state: &mut ShipState, dimensions: &Dimensions, delta: f32) -> f32 {
    let before = state.shield;
    state.shield = (state.shield + delta).min(dimensions.shield);
    state.shield - before
}

/// The pool as a percentage of its maximum, `0.0` when there is no maximum.
///
/// A ship whose `<Misc>` never loaded has a zero pool, and the original would
/// divide by zero here - `Ship_SetShield` does exactly that and pushes a NaN at
/// the HUD. Returning zero instead is a departure, and a deliberate one: a NaN
/// percentage propagates into the bar's width and takes the whole HUD with it.
#[must_use]
pub fn percent(shield: f32, max: f32) -> f32 {
    if max > 0.0 {
        (shield / max) * 100.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests;
