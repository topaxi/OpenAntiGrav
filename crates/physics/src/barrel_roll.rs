//! The barrel roll: a three-tap gesture, a signed phase that completes itself,
//! and a shield-gated arm.
//!
//! Recovered in `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`'s "The
//! tap-history path is the barrel roll" section, confidence 90 - every tunable
//! is authored `<Global><Special>` XML, read live off both shipped PSP
//! pressings. See [`ShipState::roll_taps`] and its neighbours for the state
//! this module advances.
//!
//! # What is this crate's own resolution, not a traced read
//!
//! The original arms a roll by setting one of two bits
//! (`entity+0x860 & 0x100`/`& 0x80`) that ramp [`ShipState::roll_phase`] toward
//! `+1.0`/`-1.0`, and "when they clear ... it continues to the nearer end" -
//! but **what clears them was not traced**. This module ties that release to
//! the airborne-to-grounded transition, because the *only* other traced fact
//! about the phase is that the landing payout reads `|entity+0x87c| > 0.5` at
//! exactly that transition - so one event explaining both the phase's release
//! rule and the payout's gate is the simplest reading consistent with what was
//! measured, not a second independent finding. If a future trace shows the
//! bits clearing on some other event (a timeout, a second gesture, a menu
//! transition), [`release`] is what moves.

use crate::params::Dimensions;
use crate::ship::{ShipControls, ShipState};

/// How long a tap stays a candidate for extending the gesture, in seconds.
///
/// `entity+0x884`, which accumulates `dt` and is zeroed on every tap. A tap
/// recorded while this is still below the timeout shifts the two older
/// entries in [`ShipState::roll_taps`] down; at or past it, only the newest
/// slot is overwritten and the older two are left stale - which is what makes
/// a gap this long unable to complete a pattern that spans it. Confidence 90;
/// see the module docs.
pub const INTER_TAP_TIMEOUT: f32 = 0.6;

/// The magnitude [`ShipState::roll_phase`] must clear for [`release`] to treat
/// the roll as complete. **Not an authored `<Special>` attribute** - it is the
/// literal the original's own comparison uses, read off the disassembly, not
/// out of the XML. See `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
pub const COMPLETION_SPLIT: f32 = 0.5;

/// How far the steering axis has to be pushed for a tap, normalised.
///
/// The original's tap history takes an entry when the axis "crosses below
/// `-90`" or "above `+90`", on the internal `0..=100` scale
/// [`crate::controls::CONTROL_RANGE`] documents; `0.9` is that threshold on the
/// `-1..=1` scale [`ShipControls`] carries. It is compared against the **raw
/// axis** on [`ShipControls::steer_x`] and not against the ramped
/// [`ShipState::steer`], because the original reads it out of the same input
/// block it reads the d-pad bits from, in the same function.
pub const AXIS_TAP_THRESHOLD: f32 = 90.0 / crate::controls::CONTROL_RANGE;

/// One directional tap: the `LEFT` d-pad bit or the steering axis crossing
/// below `-90`, or `RIGHT`/crossing above `+90`. Written as `1`/`2` into
/// [`ShipState::roll_taps`], matching the original's own encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapDirection {
    /// `LEFT`, encoded `1`.
    Left = 1,
    /// `RIGHT`, encoded `2`.
    Right = 2,
}

/// `RIGHT, LEFT, RIGHT` - arms the roll toward `+1.0`.
const RIGHT_LEFT_RIGHT: [u8; 3] = [2, 1, 2];
/// `LEFT, RIGHT, LEFT` - arms the roll toward `-1.0`.
const LEFT_RIGHT_LEFT: [u8; 3] = [1, 2, 1];

/// Records one tap into [`ShipState::roll_taps`] and reports whether it just
/// completed a pattern.
///
/// `Some(1.0)` for `[2, 1, 2]`, `Some(-1.0)` for `[1, 2, 1]`, `None`
/// otherwise. On a match the history is cleared back to `[0, 0, 0]`, so the
/// caller sees a genuinely fresh three-tap window afterwards rather than a
/// sliding one that would report a match again on the very next tap that
/// happens to complete the same alternation one slot later.
///
/// Does **not** charge any cost or write [`ShipState::roll_target`] - that is
/// [`arm`]'s job, deliberately kept separate so a shield-empty craft can still
/// be seen to complete the *gesture*, just not the roll.
///
/// The caller must have already advanced [`ShipState::roll_tap_timer`] by this
/// tick's `dt` - see [`advance_tap_timer`] - so the timeout this reads is
/// measured from the *previous* tap to this one, not from the start of the
/// tick.
pub fn record_tap(state: &mut ShipState, direction: TapDirection) -> Option<f32> {
    if state.roll_tap_timer < INTER_TAP_TIMEOUT {
        state.roll_taps[0] = state.roll_taps[1];
        state.roll_taps[1] = state.roll_taps[2];
    }
    state.roll_taps[2] = direction as u8;
    state.roll_tap_timer = 0.0;

    let matched = if state.roll_taps == RIGHT_LEFT_RIGHT {
        Some(1.0)
    } else if state.roll_taps == LEFT_RIGHT_LEFT {
        Some(-1.0)
    } else {
        None
    };

    if matched.is_some() {
        state.roll_taps = [0, 0, 0];
    }
    matched
}

/// Advances [`ShipState::roll_tap_timer`] by one tick. Call this exactly once
/// a tick, whether or not a tap is being recorded this tick - a tick with no
/// tap still has to widen the gap [`record_tap`] measures on the next one.
pub fn advance_tap_timer(state: &mut ShipState, dt: f32) {
    state.roll_tap_timer += dt;
}

/// Charges [`Dimensions::shield`] for a completed gesture and arms
/// [`ShipState::roll_target`], or refuses.
///
/// `sign` is [`record_tap`]'s `Some` payload - `1.0` or `-1.0`. The cost is
/// `roll_cost` percent of the shield pool, and the original "arms nothing at
/// all unless `cost < shield`" - **strictly less than**, so a shield sitting
/// exactly at the cost refuses too. Returns whether it armed.
#[must_use]
pub fn arm(state: &mut ShipState, dimensions: &Dimensions, roll_cost: f32, sign: f32) -> bool {
    let cost = roll_cost * 0.01 * dimensions.shield;
    if state.shield <= cost {
        return false;
    }
    state.shield -= cost;
    state.roll_target = sign;
    true
}

/// Ramps [`ShipState::roll_phase`] toward [`ShipState::roll_target`] at
/// `roll_speed` units per second, clamping there rather than overshooting.
///
/// Call every tick regardless of whether the roll is armed: a target of `0.0`
/// is what carries an unfinished roll's phase back down after [`release`]
/// decides it fell short.
pub fn advance_phase(state: &mut ShipState, roll_speed: f32, dt: f32) {
    let step = roll_speed * dt;
    if state.roll_phase < state.roll_target {
        state.roll_phase = (state.roll_phase + step).min(state.roll_target);
    } else if state.roll_phase > state.roll_target {
        state.roll_phase = (state.roll_phase - step).max(state.roll_target);
    }
}

/// Resolves the self-completing ramp: does not reverse, and runs on to
/// whichever end is nearer.
///
/// Call this on the airborne-to-grounded transition - see the module docs for
/// why that event is this crate's own choice of when the original's arming
/// flags clear. Past [`COMPLETION_SPLIT`], [`ShipState::roll_target`] is left
/// where it is (so [`advance_phase`] runs the rest of the way to `+/-1.0`) and
/// this returns `true`. At or short of it, the target drops to `0.0` (so the
/// phase ramps back down) and this returns `false`. A ship that was never
/// armed (`roll_target == 0.0` and `roll_phase == 0.0`) is left untouched and
/// reports `false`.
pub fn release(state: &mut ShipState) -> bool {
    if state.roll_target == 0.0 && state.roll_phase == 0.0 {
        return false;
    }
    let completed = state.roll_phase.abs() > COMPLETION_SPLIT;
    if !completed {
        state.roll_target = 0.0;
    }
    completed
}

/// The hover spring's rebound coefficient while the landing payout runs,
/// forced to a literal `1.0` in place of `ordinary`.
///
/// One of the three consumers of the original's `craft+0x1c0 & 0x400`,
/// alongside [`crate::airbrake::ROLL_GRIP_MULTIPLIER`] and the turbo add in
/// `crate::engine::engine`.
///
/// **This crate's own reconnection of two facts read independently, not a
/// third traced instance.** `docs/ghidra/functions/psp-pulse-usa/
/// input-bindings.md` reads the override only as "a hover scalar is forced
/// from `stats+0x4` to a literal `1.0`", without naming the field;
/// `engine.md`'s account of the undecoded `craft+0x2a4` enum separately
/// records that its `0` state "disables the `rebound` parameter" in this same
/// function. `stats+0x4` sitting one field after `ride_height` (`stats+0x0`,
/// by the offsets `HandlingXml_ParseAntigrav` stores - `ride_height 0x94`,
/// `rebound 0x98`) lines up with `rebound` exactly, so both overrides read as
/// the same mechanism applied on two different gates. Confidence 75 on the
/// identification; the override's *existence* and its `1.0` value are
/// confidence 90, off the read above.
#[must_use]
pub fn rebound_override(state: &ShipState, ordinary: f32) -> f32 {
    if state.roll_payout_timer > 0.0 {
        1.0
    } else {
        ordinary
    }
}

/// Which side of [`AXIS_TAP_THRESHOLD`] an axis reading sits on, if either.
#[must_use]
fn axis_zone(steer_x: f32) -> Option<TapDirection> {
    if steer_x > AXIS_TAP_THRESHOLD {
        Some(TapDirection::Right)
    } else if steer_x < -AXIS_TAP_THRESHOLD {
        Some(TapDirection::Left)
    } else {
        None
    }
}

/// Runs the whole gesture for one tick: the timer, both tap sources, the
/// history and the shield-gated arm. Reports whether a roll was armed.
///
/// This is the reachable half of the mechanic and the only thing
/// [`crate::forces::evaluate`] needs to call - [`advance_tap_timer`],
/// [`record_tap`] and [`arm`] stay public because they are what the unit tests
/// pin one at a time, not because a caller should sequence them itself. Calling
/// this *and* [`advance_tap_timer`] in the same tick would advance the timer
/// twice and halve [`INTER_TAP_TIMEOUT`].
///
/// # The two sources are one signal
///
/// The original writes `1` "when the `LEFT` d-pad bit is pressed **or** the
/// steering axis crosses below `-90`" - one history entry either way, never
/// two. That `or` is load-bearing here rather than incidental: this project's
/// input layer maps the d-pad onto the analog axis as well
/// (`oag_input::pad::larger`), so a d-pad press and an axis crossing land on the
/// *same* tick, and recording both would shift the history twice and leave it
/// holding a doubled direction that can never match an alternation.
///
/// `LEFT` is tested first when both directions somehow arrive at once, matching
/// the order the two fields are declared in - the same tie-break
/// [`crate::airbrake::sideshift_force`] takes.
///
/// # Why the axis leg is here and not in the input layer
///
/// A crossing is an edge and needs last tick's side of the threshold, which is
/// per-craft state: see [`ShipState::roll_axis_zone`]. Putting it here also
/// means the two schemes never enter into it - the barrel roll is not a
/// scheme-dependent gesture, unlike the sideshift.
///
/// # An AI craft can arm one, and that is measured rather than assumed
///
/// This crate does not know whether a craft is flown by a pilot or by
/// `oag_ai::Driver` - both arrive as [`ShipControls`] - and the axis leg has
/// no human-only gate the way the novice flick's
/// [`ShipControls::shift_modifier`] effectively is. So an opponent's own
/// steering can complete the alternation. **Deliberately left that way**: the
/// original's gesture reads whatever is in the craft's input block, and adding
/// a "human only" flag would be inventing a mechanism nothing was traced to.
///
/// How often it actually happens was measured against `oag-ai`'s own closed-loop
/// fixtures on 2026-09-06, eight seeded drivers for 3,600 ticks each:
/// **zero** arms on the ordinary oval, and **three arms across the eight** on
/// `oval_of(60.0, 300.0)` - the deliberately pathological corner
/// `a_differential_holds_a_corner_the_steering_alone_cannot` keeps because a
/// craft "genuinely cannot make it on the stick alone" and sits on the steering
/// stop for 300+ ticks. Not pinned by a test on purpose - the number moves
/// with any controller tuning, which is the same reason `closed_loop.rs` keeps
/// its own bounds loose.
///
/// **The synthetic ovals understate it by an order of magnitude, and the disc
/// says so.** Re-measured on 2026-09-06 against the real circuits - one lone
/// Ace craft, `Mode::SingleRace`, 18,000 ticks, every forward circuit - an
/// opponent arms **four to eight rolls in a three-lap race on ten of the
/// twelve**, spending **30 to 54 of its 95 shield**. On `07_Track` and
/// `16_Track` it never leaves the ground for a single tick, so [`release`] is
/// never called, [`ShipState::roll_payout_timer`] is never armed, and every
/// one of those charges buys nothing. On `07_Track` that is fatal: the craft
/// is destroyed on lap 2 where it used to bank a clean lap first, which is
/// what `crates/game/tests/race_ground_truth.rs`'s
/// `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` caught. So
/// ordinary AI cornering *does* arm rolls, routinely. The gesture is left
/// ungated all the same, for the reason above - what is missing is a trace,
/// not a rule invented to fit this measurement.
///
/// # No airborne gate
///
/// Nothing here checks that the craft is off the ground, because nothing in the
/// recovered chain does either: the gesture arms whenever it completes and it
/// is the *payout* that requires the airborne-to-grounded transition. A roll
/// armed on the ground therefore costs shield and ramps [`ShipState::roll_phase`]
/// like any other. If a trace ever shows the original refusing to arm on the
/// ground, this is where that gate goes.
pub fn advance_gesture(
    state: &mut ShipState,
    input: &ShipControls,
    dimensions: &Dimensions,
    roll_cost: f32,
    dt: f32,
) -> bool {
    advance_tap_timer(state, dt);

    let zone = axis_zone(input.steer_x);
    let crossed = if zone == state.roll_axis_zone {
        None
    } else {
        zone
    };
    state.roll_axis_zone = zone;

    let direction = if input.roll_tap_left || crossed == Some(TapDirection::Left) {
        Some(TapDirection::Left)
    } else if input.roll_tap_right || crossed == Some(TapDirection::Right) {
        Some(TapDirection::Right)
    } else {
        None
    };

    let Some(direction) = direction else {
        return false;
    };
    let Some(sign) = record_tap(state, direction) else {
        return false;
    };
    arm(state, dimensions, roll_cost, sign)
}

#[cfg(test)]
mod tests;
