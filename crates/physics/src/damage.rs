//! The energy pool, and what a wall costs it.
//!
//! Recovered in [`shield.md`], which this module ports. The original's shape:
//!
//! ```text
//! Ship_ResetShield(craft)          shield = misc.shield[skill]
//! Ship_Damage(amount, craft, ...)  if (!weapons) amount *= 0.5
//!                                  shield = min(shield - amount, max)
//!                                  if (shield <= 0) SetState(craft, Destroyed)
//! FUN_088418e0's contact loop       Ship_Damage(|p| * 0.05 * 0.7, craft, ...)
//! ```
//!
//! The collision leg and the two weapon legs ([`apply_weapon`], [`add`]) are here.
//! Everything that *adds* to the pool goes through `Ship_AddShield` (`0x0883ddc8`), whose
//! four callers are all read: the pickup absorb, the LeachBeam's repair, Zone's clean-zone
//! recharge and the Eliminator's per-lap refill, each built in `oag_gameplay` or the
//! composition root on top of [`add`]. There is no pit lane in this title.
//!
//! # Why the pool lives in `oag_physics`
//!
//! Nothing in the force law reads it, but the *writer* decides: the amount is a function of
//! the contact impulse, which only this crate computes, and the original keeps the pool on
//! the craft beside the timers this crate owns. In `oag_gameplay` it would need per-contact
//! impulses exported or damage refitted from a summary, and it would sit outside
//! `crate::probe`'s determinism hash ([`crate::ShipState::shield`]).
//!
//! # Not implemented, and not guessed at
//!
//! - **`Ship_Damage`'s `weapon_kind` sub-bucket** (nine cases, unmapped) and the
//!   absorb-spark its `source == 2` branch triggers; [`apply_weapon`] is that branch's
//!   damage half only.
//! - **The respawn cost**: `Ship_SetState`'s state-3 branch stores `clamp(shield - 1, 0, 5)`;
//!   its consumer is unread.
//! - **The `SkillLevel` race option**: the pool arrives resolved
//!   (`oag_gameplay::handling::DEFAULT_SKILL_LEVEL`).
//!
//! [`shield.md`]: ../../../docs/ghidra/functions/psp-pulse-usa/shield.md

use crate::{ShipState, params::Dimensions, wall::WallResponse};

/// The scale from a contact's impulse magnitude to energy lost, `0.05 * 0.7`.
///
/// Both literals are `FUN_088418e0`'s contact loop: `|p| * 0.05` for the reaction, `that *
/// 0.7` into `Ship_Damage`. Kept as the product so it stays recognisable against the
/// decompile (the same `0.05` and `0.7` appear in [`crate::wall`]'s scrape friction).
///
/// **Confidence 94, measured against the running original.** A breakpoint on `Ship_Damage`
/// (where the contact ring is still intact) puts `amount / |p|` at `0.035000` on 25 of 25
/// calls over amounts spanning a factor of 225, with **one call per ring record** (four
/// contacts, four calls): what [`crate::wall::WallResponse::impulse_sum`] sums for.
///
/// **Charging every tick of a sustained graze is faithful, not a bug**: the original's loop
/// runs on every tick the ring holds a record and gates nothing on duration. The
/// measurements (ring cap of 8, fixed-step comparison, per-tick loss distribution) and the
/// conclusion that `07_Track`'s Ace dying is a driving-line question are in
/// `docs/ghidra/functions/psp-pulse-usa/shield.md`, "Contact damage is charged every tick
/// of a sustained graze". This constant and the cadence both stay.
pub const CONTACT_DAMAGE_SCALE: f32 = 0.05 * 0.7;

/// What `Ship_Damage` multiplies the amount by when the race has weapons off:
/// `if (g_weapons_enabled == 0) amount *= 0.5`. It applies to *collision* damage too (the
/// option that removes weapons halves what a wall costs), hence a named constant.
pub const NO_WEAPONS_DAMAGE_SCALE: f32 = 0.5;

/// Where the energy-critical warning fires, as a percentage of the pool.
///
/// `DAT_08a7b6a4`, `20.0` in `.rodata`. The original tests the percentage before *and*
/// after the subtraction and warns on the downward crossing only
/// ([`Shield::crossed_critical`]); a level test would re-fire every tick of a scrape.
pub const CRITICAL_PERCENT: f32 = 20.0;

/// Where a craft is in the destroyed sequence.
///
/// The original keeps a state machine at `entity+0x8c`, read and written by `Ship_State`
/// (`0x0883e64c`) and `Ship_SetState` (`0x08844100`). **Only three of nine states are
/// modelled**, the ones the energy pool reaches; the rest (race start, respawn, the
/// Eliminator's re-insertion) would claim a sequence nobody has read. The numbers beside
/// each variant are the original's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CraftState {
    /// State 1. Racing, and the only state that takes damage.
    #[default]
    Racing,
    /// State 4. The explosion, which runs for [`DESTROYED_DURATION`].
    ///
    /// `Ship_SetState`'s case 4 plays `~BLOWUP`, hides the HUD and puts the camera in mode
    /// 5. **The sound is built** (`oag_sound::sfx`); the HUD and camera work goes through
    /// four unidentified globals, whose addresses are on
    /// [`zone-mode.md`](../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md).
    Destroyed,
    /// State 5. Out of the race.
    ///
    /// The state that matters to a game mode: `Ship_SetState`'s case 5 **sets bit 12
    /// (`0x1000`) of `entity+0x860`**, which `Zone_UpdateRacing` ends a run on
    /// ([`zone-mode.md`](../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md) had it
    /// as set by nothing findable).
    ///
    /// The original moves on after 1.5 s to state 6's respawn wait or the Eliminator's state
    /// 8. **This crate stops here** and the composition root decides: the player's race ends
    /// (`RaceState::eliminate`), an opponent or Eliminator craft is put back
    /// (`oag_raceplay::Race::tick_destroyed_craft`).
    Eliminated,
}

/// How long the explosion runs before the craft is out of the race, in seconds.
///
/// `Ship_SetState`'s case 4 writes `0.5` into `entity+0x874`; state 4's per-frame update
/// (`0x088404c8`) subtracts `dt` and at or below zero goes to state 5. Confidence **88**,
/// no runtime leg.
pub const DESTROYED_DURATION: f32 = 0.5;

/// The race options that reach the pool.
///
/// Two of the three globals `Race_ReadSetupOptions` (`0x08896b84`) lowers; `g_skill_level`
/// is consumed earlier (it picks which `<Misc>` slot becomes [`Dimensions::shield`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRules {
    /// `g_weapons_enabled` (`0x08b30fa8`). With weapons off every amount is scaled by
    /// [`NO_WEAPONS_DAMAGE_SCALE`].
    pub weapons: bool,
    /// `g_damage_enabled` (`0x08b30fa9`).
    ///
    /// **This does not gate the subtraction**: with it off the original runs `Ship_Damage`
    /// in full and separately regenerates the pool at [`REGENERATION_PER_SECOND`] to a floor
    /// of [`REGENERATION_FLOOR`]. A craft cannot be destroyed because the floor is above
    /// zero, not because nothing hurts it ([`regenerate`]).
    pub damage: bool,
}

impl Default for DamageRules {
    /// Weapons and damage both on: `Race_ReadSetupOptions`' default for every mode except
    /// three.
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
/// Returned rather than stored on [`ShipState`] because both fields are **edges**, which
/// would otherwise need clearing by the reader. Ignoring it loses a sound cue only.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Shield {
    /// Energy lost this tick, after every scale. Zero on a tick with no contact.
    pub lost: f32,
    /// Whether this tick took the pool from above [`CRITICAL_PERCENT`] to at or below it:
    /// the `energycritical` cue's trigger.
    pub crossed_critical: bool,
    /// Whether the pool reached zero this tick. [`subtract`] also moves `craft_state` to
    /// [`CraftState::Destroyed`] on this edge; this is the per-tick report for the caller,
    /// which is how [`Zone`](../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md) ends.
    pub depleted: bool,
    /// Whether a fired Shield pickup swallowed this tick's damage.
    ///
    /// **The original's own edge.** Both damage drains take the shield branch *instead of*
    /// subtracting and then call `ShipShield_Hit` (`0x0885eb04`), which flashes and bulges
    /// the shell; without this the shield is invisible on the tick a player looks.
    ///
    /// Set only when there was something to swallow: a zero-energy graze is not a hit, and
    /// would strobe the shell every scrape tick (`shield-pickup.md`).
    pub absorbed: bool,
}

impl Shield {
    /// Whether the hit got through `Ship_Damage`'s gate and was subtracted: racing, no fired
    /// Shield, positive amount. `lost` is the amount *asked for*, not the clamped delta, so
    /// the killing hit on a nearly empty pool still reads as landed; every refusal returns a
    /// zero `lost`, the absorbed one included.
    #[must_use]
    pub fn landed(&self) -> bool {
        self.lost > 0.0
    }
}

/// Advance the destroyed sequence, `0x088404c8`.
///
/// Separate from [`apply_contact`] because it runs every tick, not only with contacts, and a
/// caller driving the force law directly still wants the craft to finish blowing up.
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
/// [`crate::engine::advance_turbo`]'s twin: called once a tick from [`crate::step`] **after**
/// [`apply_contact`] has read the timer, so the tick a shield fires on is protected, and
/// floored at zero so `> 0.0` is the whole gate. It inherits the same one-tick-longer
/// character (a `0.75` s pickup protects 46 ticks at 60 Hz, not 45), pinned by
/// `a_fired_shield_refuses_damage_for_its_authored_duration`.
pub fn advance_shield_pickup(state: &mut ShipState, dt: f32) {
    state.shield_pickup_timer = (state.shield_pickup_timer - dt).max(0.0);
}

/// Fill the pool, `Ship_ResetShield` (`0x0883dd24`).
///
/// The only thing that sets the pool to its maximum outright; a clean-zone recharge *adds*
/// through `Ship_AddShield`. Call it when a ship takes the grid, not on every spawn pose.
pub fn reset(state: &mut ShipState, dimensions: &Dimensions) {
    state.shield = dimensions.shield;
    state.craft_state = CraftState::Racing;
    state.state_timer = 0.0;
    // Ours, like the timer: a shield left running from the previous race would protect a
    // craft that never collected one. `turbo_timer` is deliberately *not* cleared (a
    // separate open question; changing it here would bury a behaviour change in a shield).
    state.shield_pickup_timer = 0.0;
}

/// The energy a frame's contacts cost, before the pool is touched. Separate from
/// [`apply_contact`] so a test can assert the law without a `ShipState`.
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
/// The original regenerates near the top of the craft update and walks the contact ring
/// near the bottom, so regeneration there hits the *previous* frame's pool. Splitting the
/// pool across the tick would reproduce that for one frame of a 4-a-second ramp (`0.067`
/// energy at 60 Hz); doing it in this order is a knowing simplification.
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
/// The same function as [`apply_contact`], differing only in the amount: a contact scales an
/// impulse by [`CONTACT_DAMAGE_SCALE`], a weapon passes the disc's `<Stats damage>`
/// through. The state gate, weapons-off halving, clamp, edges and destroyed transition are
/// one shared [`subtract`].
///
/// **Does not regenerate**: [`apply_contact`] does because it runs inside [`crate::step`]
/// and the floor must land in the loss's tick; this is called from outside by whoever owns
/// the projectile.
///
/// `source == 2` is the original's weapon hit at confidence 75, resting on the telemetry
/// bucket layout, not a string, and the only evidenced part of the weapon-damage path. The
/// `weapon_kind` sub-bucket and absorb-spark are not reproduced
/// (`docs/ghidra/functions/psp-pulse-usa/shield.md`).
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
/// [`contact_damage`]'s twin: the authored amount, halved with weapons off. That halving is
/// unreachable today (a weapons-off race arms no pads) but `Ship_Damage` applies it to every
/// amount whatever the source, so it is reproduced for the day something else arrives.
#[must_use]
pub fn weapon_damage(amount: f32, rules: DamageRules) -> f32 {
    if rules.weapons {
        amount
    } else {
        amount * NO_WEAPONS_DAMAGE_SCALE
    }
}

/// `Ship_Damage`'s body: the state gate, clamp, two edges and destroyed transition, given an
/// already-scaled amount. Shared by [`apply_contact`] and [`apply_weapon`] so the gate, which
/// makes a shielded or destroyed craft immune to *both*, cannot drift.
fn subtract(state: &mut ShipState, dimensions: &Dimensions, lost: f32) -> Shield {
    // `Ship_Damage` refuses outside the racing states (the original lists states 4, 5, 6 and
    // 2; the three modelled collapse to "not racing").
    if state.craft_state != CraftState::Racing {
        return Shield::default();
    }

    // A fired Shield pickup refuses on the same edge (recovered 2026-08-19). The original's
    // two drains both gate on it: weapon damage is posted to `craft+0x120` and drained by
    // `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`), which calls `Ship_Damage` only when
    // `craft+0x1b8 & 0x10` is clear; contact damage is the contact loop's reaction #2,
    // replaced by reaction #3 under the same bit. The amount is *discarded*, not deferred,
    // and both arm `ShipShield_Hit` (`shield-pickup.md`).
    //
    // Refusing *here*, before the subtraction, is the original's order and load-bearing: it
    // suppresses `depleted` and `crossed_critical` too, so a shielded craft cannot be
    // destroyed or fire the energy-critical cue. The wall's own impulse is unchanged (the
    // shield suppresses only the *posted* one, [`crate::wall::STUN_PER_CONTACT`], which the
    // track path never posts): a shielded craft bounces as usual and pays nothing.
    if state.shield_pickup_timer > 0.0 {
        return Shield {
            absorbed: lost > 0.0,
            ..Shield::default()
        };
    }

    let max = dimensions.shield;
    let before = state.shield;

    // `Ship_SetShield` clamps above and never below; the floor is ours (`ShipState::shield`).
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
/// Separate because it runs whether or not the frame had a contact. `0.0` applies the floor
/// alone, which [`apply_contact`] wants: the floor makes the pool undepletable in that mode
/// and must land in the loss's tick.
pub fn regenerate(state: &mut ShipState, dimensions: &Dimensions, rules: DamageRules, dt: f32) {
    if rules.damage {
        return;
    }
    let regenerated = (state.shield + dt * REGENERATION_PER_SECOND).max(REGENERATION_FLOOR);
    state.shield = regenerated.min(dimensions.shield);
}

/// Adds energy to the pool, clamped at the ship's maximum.
///
/// `Ship_AddShield` (`0x0883ddc8`) routes a delta through `Ship_SetShield` (`0x0883e6f4`),
/// which takes `min(amount, max)` against the skill-indexed `<Misc>` maximum and floors
/// nothing (`shield.md`). The clamp is recovered, and is why callers should not write
/// `state.shield += delta`: an unclamped pool draws a `ShieldBar` past full.
///
/// The one caller today is the pickup absorb (`<Weapon><Stats absorb>`). Zone's
/// perfect-zone recharge, the original's other caller, predates this, writes the field
/// directly and should move here.
///
/// Returns how much landed, less than `delta` against a full pool. The original's sound
/// cue is not implemented.
pub fn add(state: &mut ShipState, dimensions: &Dimensions, delta: f32) -> f32 {
    let before = state.shield;
    state.shield = (state.shield + delta).min(dimensions.shield);
    state.shield - before
}

/// The pool as a percentage of its maximum, `0.0` when there is no maximum.
///
/// A ship whose `<Misc>` never loaded has a zero pool; the original divides by zero here
/// (`Ship_SetShield` pushes a NaN at the HUD). Zero is a deliberate departure: a NaN
/// percentage propagates into the bar's width and takes the HUD with it.
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
