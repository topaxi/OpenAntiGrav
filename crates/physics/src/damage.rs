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
//! - **Weapon damage and absorb**, which need weapons.
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
    /// `Ship_SetState`'s case 4 plays `_BLOWUP`, hides the HUD and puts the
    /// camera in mode 5 - **none of which this engine does**, because it has no
    /// explosion, no HUD hide and no camera mode. What it does have is the
    /// timer, which is what the transition below needs.
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
    // `Ship_Damage` refuses outright outside the racing states - a craft already
    // blowing up takes no further damage. The original's gate lists states 4, 5,
    // 6 and 2; the three this crate models collapse to "not racing".
    if state.craft_state != CraftState::Racing {
        return Shield::default();
    }

    let max = dimensions.shield;
    let before = state.shield;
    let lost = contact_damage(wall.impulse_sum, rules);

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

    regenerate(state, dimensions, rules, 0.0);

    Shield {
        lost,
        crossed_critical,
        depleted,
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
mod tests {
    use super::*;

    fn dimensions(shield: f32) -> Dimensions {
        Dimensions {
            shield,
            ..Dimensions::default()
        }
    }

    fn state(shield: f32) -> ShipState {
        ShipState {
            shield,
            ..ShipState::default()
        }
    }

    fn wall(impulse_sum: f32) -> WallResponse {
        WallResponse {
            impulse_sum,
            ..WallResponse::default()
        }
    }

    /// The headline law: `|p| * 0.05 * 0.7`.
    #[test]
    fn contact_damage_is_the_impulse_times_the_recovered_pair() {
        let amount = contact_damage(100.0, DamageRules::default());
        assert!(
            (amount - 3.5).abs() < 1e-5,
            "100 units of impulse should cost 3.5 energy, cost {amount}"
        );
    }

    /// The rule that reads like a bug and is not: no weapons halves *collision*
    /// damage. A time trial is exactly this case.
    #[test]
    fn turning_weapons_off_halves_what_a_wall_costs() {
        let rules = DamageRules {
            weapons: false,
            ..DamageRules::default()
        };
        assert!((contact_damage(100.0, rules) - 1.75).abs() < 1e-5);
    }

    #[test]
    fn reset_fills_the_pool_from_the_hull() {
        let mut s = state(0.0);
        reset(&mut s, &dimensions(300.0));
        assert_eq!(s.shield, 300.0);
    }

    #[test]
    fn a_contact_takes_energy_and_the_pool_stops_at_zero() {
        let d = dimensions(300.0);
        let mut s = state(2.0);
        let report = apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());
        assert_eq!(s.shield, 0.0, "the pool must not go negative");
        assert!(report.depleted, "reaching zero is the signal Zone needs");
        assert!((report.lost - 35.0).abs() < 1e-4);
    }

    /// `depleted` is an edge. A ship that is already at zero must not re-signal
    /// every tick it keeps scraping, or whatever ends the race ends it repeatedly.
    #[test]
    fn an_already_empty_pool_does_not_re_signal() {
        let d = dimensions(300.0);
        let mut s = state(0.0);
        let report = apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());
        assert!(!report.depleted);
    }

    /// The critical warning is a crossing, not a level - the original tests the
    /// percentage on both sides of the subtraction.
    #[test]
    fn the_critical_warning_fires_on_the_crossing_and_not_below_it() {
        let d = dimensions(100.0);

        // 25 % -> 15 %, one crossing of the 20 % line.
        let mut s = state(25.0);
        let crossing = apply_contact(&mut s, &d, &wall(10.0 / CONTACT_DAMAGE_SCALE), d_rules());
        assert!(crossing.crossed_critical);

        // 15 % -> 5 %, already below, so no second warning.
        let again = apply_contact(&mut s, &d, &wall(10.0 / CONTACT_DAMAGE_SCALE), d_rules());
        assert!(!again.crossed_critical, "a level test would fire here");
    }

    fn d_rules() -> DamageRules {
        DamageRules::default()
    }

    /// Damage off does not skip the subtraction; it floors the result at 20 and
    /// regenerates. A test that asserted "no damage" would pin the wrong reading.
    #[test]
    fn damage_off_still_subtracts_but_cannot_empty_the_pool() {
        let d = dimensions(300.0);
        let rules = DamageRules {
            damage: false,
            ..DamageRules::default()
        };
        let mut s = state(300.0);
        let report = apply_contact(&mut s, &d, &wall(1000.0), rules);
        assert!(report.lost > 0.0, "the subtraction still happens");
        assert_eq!(s.shield, 300.0 - 35.0);

        let mut empty = state(1.0);
        apply_contact(&mut empty, &d, &wall(1000.0), rules);
        assert_eq!(
            empty.shield, REGENERATION_FLOOR,
            "the floor is what makes a craft undestroyable, not a skipped subtraction"
        );
    }

    #[test]
    fn regeneration_ramps_at_four_a_second_and_stops_at_the_maximum() {
        let d = dimensions(50.0);
        let rules = DamageRules {
            damage: false,
            ..DamageRules::default()
        };
        let mut s = state(30.0);
        regenerate(&mut s, &d, rules, 1.0);
        assert_eq!(s.shield, 34.0);

        let mut full = state(49.0);
        regenerate(&mut full, &d, rules, 10.0);
        assert_eq!(full.shield, 50.0);
    }

    #[test]
    fn regeneration_does_nothing_while_damage_is_on() {
        let d = dimensions(300.0);
        let mut s = state(100.0);
        regenerate(&mut s, &d, DamageRules::default(), 1.0);
        assert_eq!(s.shield, 100.0);
    }

    /// The whole destroyed sequence, in the order the original runs it.
    #[test]
    fn emptying_the_pool_blows_the_craft_up_and_then_eliminates_it() {
        let d = dimensions(300.0);
        let mut s = state(1.0);
        let report = apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());

        assert!(report.depleted);
        assert_eq!(s.craft_state, CraftState::Destroyed);
        assert_eq!(s.state_timer, DESTROYED_DURATION);

        // The explosion runs for half a second and not a tick less.
        let dt = 1.0 / 60.0;
        for _ in 0..29 {
            advance_state(&mut s, dt);
            assert_eq!(s.craft_state, CraftState::Destroyed);
        }
        advance_state(&mut s, dt);
        assert_eq!(s.craft_state, CraftState::Eliminated);
        assert_eq!(s.state_timer, 0.0);
    }

    /// `Ship_Damage` refuses outright outside the racing states, so a craft
    /// already blowing up cannot be blown up again - and `depleted` cannot fire
    /// twice, which is what a mode ending on it depends on.
    #[test]
    fn a_destroyed_craft_takes_no_further_damage() {
        let d = dimensions(300.0);
        let mut s = state(1.0);
        apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());
        let pool = s.shield;

        let again = apply_contact(&mut s, &d, &wall(1000.0), DamageRules::default());
        assert_eq!(
            again,
            Shield::default(),
            "a destroyed craft kept taking hits"
        );
        assert_eq!(s.shield, pool);
        assert_eq!(s.state_timer, DESTROYED_DURATION, "the explosion restarted");
    }

    /// Nothing advances outside the explosion, so a racing craft's timer cannot
    /// drift and an eliminated one stays eliminated.
    #[test]
    fn only_the_explosion_runs_the_state_timer_down() {
        let mut racing = state(100.0);
        racing.state_timer = 5.0;
        advance_state(&mut racing, 1.0);
        assert_eq!(racing.state_timer, 5.0);

        let mut done = state(0.0);
        done.craft_state = CraftState::Eliminated;
        advance_state(&mut done, 1.0);
        assert_eq!(done.craft_state, CraftState::Eliminated);
    }

    /// Taking the grid clears the sequence as well as filling the pool - a
    /// restart must not leave the previous run's wreck in place.
    #[test]
    fn reset_puts_a_wrecked_craft_back_in_the_race() {
        let mut s = state(0.0);
        s.craft_state = CraftState::Eliminated;
        s.state_timer = 3.0;
        reset(&mut s, &dimensions(300.0));
        assert_eq!(s.craft_state, CraftState::Racing);
        assert_eq!(s.state_timer, 0.0);
        assert_eq!(s.shield, 300.0);
    }

    /// A ship whose `<Misc>` never loaded has a zero maximum, and the HUD divides
    /// by it. The original produces a NaN; this does not.
    #[test]
    fn a_zero_pool_reads_as_zero_percent_rather_than_nan() {
        assert_eq!(percent(0.0, 0.0), 0.0);
        assert!(percent(0.0, 0.0).is_finite());
    }
}
