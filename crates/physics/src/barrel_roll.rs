//! The barrel roll: a three-tap gesture, a signed phase that completes itself, and a
//! shield-gated arm.
//!
//! Recovered in `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`, "The tap-history
//! path is the barrel roll", confidence 90; every tunable is authored `<Global><Special>`
//! XML read live off both shipped PSP pressings. State: [`ShipState::roll_taps`] and its
//! neighbours.
//!
//! # The release event is the landing
//!
//! The original arms a roll with one of two bits (`entity+0x860 & 0x100`/`& 0x80`) that ramp
//! [`ShipState::roll_phase`] toward `+1.0`/`-1.0`. Until 2026-09-06 what clears them was this
//! module's guess; reading `Ship_UpdateSideshiftInput_q` (`0x08846a54`) whole showed the clear
//! sits in the **airborne-to-grounded** branch (`craft+0x1c0 & 1` set this tick,
//! `craft+0x860 & 0x200` clear), in the same `if` that arms the payout off
//! `|entity+0x87c| > 0.5`. A second clear on the grounded-to-airborne transition
//! (`0x08846ab4`-`0x08846ac8`) is omitted: the grounded gate in [`advance_gesture`] makes an
//! armed roll on the ground unreachable.
//!
//! # The payout is gated on the arm, not the phase
//!
//! The landing's `|entity+0x87c| > 0.5` test sits inside an "armed" test and the landing
//! clears the arm bits ([`ShipState::roll_armed`]). Without it a completed roll left the
//! phase at `+-1.0` and every later landing paid again (a maintainer report from play: on a
//! wavy track, "insane boosts").

use crate::params::Dimensions;
use crate::ship::{ShipControls, ShipState};

/// How long a tap stays a candidate for extending the gesture, in seconds.
///
/// `entity+0x884` accumulates `dt` and is zeroed on every tap. A tap while it is below the
/// timeout shifts the two older entries of [`ShipState::roll_taps`] down; at or past it only
/// the newest slot is overwritten and the older two go stale, so a gap this long cannot
/// complete a pattern spanning it. Confidence 90.
pub const INTER_TAP_TIMEOUT: f32 = 0.6;

/// The magnitude [`ShipState::roll_phase`] must clear for [`release`] to treat the roll as
/// complete. **Not an authored `<Special>` attribute**: the literal the original's own
/// comparison uses, read off the disassembly (`input-bindings.md`).
pub const COMPLETION_SPLIT: f32 = 0.5;

/// Whether spending a roll's cost would leave the pool above
/// [`ShipControls::roll_shield_floor`].
///
/// **Invented, and the only invented rule this mechanic carries.** Nothing on the disc
/// authors it. Chosen by the maintainer on 2026-09-06 (the AI should avoid rolls when low
/// on energy, "like below 20% or something"; the original's behaviour unknown) as a design
/// decision, **not** a finding, so it carries no confidence score.
///
/// It sits on top of [`arm`]'s recovered `cost < shield` (confidence 90). A craft flown by
/// `oag_ai::Driver` needs both; a human craft leaves the floor at `0.0` and needs only the
/// recovered one. That asymmetry is the deviation, an AI-quality choice. The `0.20` this once
/// held as a bare constant is now the low end of `oag_ai::Pilot::BALANCED`'s `roll_floor`. If
/// the original's AI gate is recovered, this is **replaced** by it, not reconciled.
#[must_use]
pub fn within_budget(
    state: &ShipState,
    dimensions: &Dimensions,
    roll_cost: f32,
    shield_floor: f32,
) -> bool {
    // A hard floor: compared is the pool the roll would *leave*, not the one it starts from
    // (the other reading lets a craft exactly on its floor spend and land under it).
    state.shield - roll_cost * 0.01 * dimensions.shield >= shield_floor * dimensions.shield
}

/// How far the steering axis has to be pushed for a tap, normalised.
///
/// The original's tap history takes an entry when the axis "crosses below `-90`" or "above
/// `+90`" on the `0..=100` scale ([`crate::controls::CONTROL_RANGE`]); `0.9` is that on the
/// `-1..=1` scale [`ShipControls`] carries. Compared against the **raw axis**
/// [`ShipControls::steer_x`], not the ramped [`ShipState::steer`], as the original reads it
/// from the same input block as the d-pad bits.
pub const AXIS_TAP_THRESHOLD: f32 = 90.0 / crate::controls::CONTROL_RANGE;

/// One directional tap: the `LEFT` d-pad bit or the axis crossing below `-90`, or
/// `RIGHT`/above `+90`. Written as `1`/`2` into [`ShipState::roll_taps`], as the original.
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

/// Records one tap into [`ShipState::roll_taps`] and reports whether it completed a pattern.
///
/// `Some(1.0)` for `[2, 1, 2]`, `Some(-1.0)` for `[1, 2, 1]`, `None` otherwise. A match
/// clears the history to `[0, 0, 0]`, so the next window is fresh rather than a sliding one
/// that would match again on the next tap.
///
/// Does **not** charge a cost or write [`ShipState::roll_target`]; that is [`arm`], kept
/// separate so a shield-empty craft still completes the *gesture*, not the roll.
///
/// The caller must have advanced [`ShipState::roll_tap_timer`] by this tick's `dt`
/// ([`advance_tap_timer`]), so the timeout is measured from the *previous* tap to this one.
pub fn record_tap(state: &mut ShipState, direction: TapDirection) -> Option<f32> {
    shift_tap(state, direction);

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

/// Shifts one tap into [`ShipState::roll_taps`] and restarts the inter-tap timer, with no
/// pattern test. What the original does for a tap while a roll is already armed
/// ([`advance_gesture`], "One roll per flight").
fn shift_tap(state: &mut ShipState, direction: TapDirection) {
    if state.roll_tap_timer < INTER_TAP_TIMEOUT {
        state.roll_taps[0] = state.roll_taps[1];
        state.roll_taps[1] = state.roll_taps[2];
    }
    state.roll_taps[2] = direction as u8;
    state.roll_tap_timer = 0.0;
}

/// Advances [`ShipState::roll_tap_timer`] by one tick. Call exactly once a tick, tap or not:
/// a tick with no tap still widens the gap [`record_tap`] measures next.
pub fn advance_tap_timer(state: &mut ShipState, dt: f32) {
    state.roll_tap_timer += dt;
}

/// Charges [`Dimensions::shield`] for a completed gesture and arms [`ShipState::roll_target`],
/// or refuses.
///
/// `sign` is [`record_tap`]'s `Some` payload. The cost is `roll_cost` percent of the pool and
/// the original arms nothing "unless `cost < shield`", **strictly less than**, so a shield
/// exactly at the cost refuses too. Returns whether it armed.
#[must_use]
pub fn arm(state: &mut ShipState, dimensions: &Dimensions, roll_cost: f32, sign: f32) -> bool {
    let cost = roll_cost * 0.01 * dimensions.shield;
    if state.shield <= cost {
        return false;
    }
    state.shield -= cost;
    state.roll_target = sign;
    state.roll_armed = true;
    true
}

/// Ramps [`ShipState::roll_phase`] toward [`ShipState::roll_target`] at `roll_speed` per
/// second, clamping rather than overshooting. Call every tick, armed or not: a target of
/// `0.0` is what carries an unfinished roll's phase back down after [`release`].
pub fn advance_phase(state: &mut ShipState, roll_speed: f32, dt: f32) {
    let step = roll_speed * dt;
    if state.roll_phase < state.roll_target {
        state.roll_phase = (state.roll_phase + step).min(state.roll_target);
    } else if state.roll_phase > state.roll_target {
        state.roll_phase = (state.roll_phase - step).max(state.roll_target);
    }
}

/// Resolves the self-completing ramp: it does not reverse and runs on to the nearer end.
///
/// Call on the airborne-to-grounded transition (module docs). Past [`COMPLETION_SPLIT`] the
/// target stays (so [`advance_phase`] runs on to `+/-1.0`) and this returns `true`; at or short
/// of it the target drops to `0.0` and this returns `false`. A ship with no armed roll
/// ([`ShipState::roll_armed`] clear) is untouched and reports `false`, which stops a landing
/// after a completed roll paying again.
pub fn release(state: &mut ShipState) -> bool {
    // The original's landing clears both arm bits whether or not it paid and gates the payout
    // on them, so a roll pays at most once. `roll_target` stays at `+-1.0` on a completion.
    if !state.roll_armed {
        return false;
    }
    state.roll_armed = false;
    let completed = state.roll_phase.abs() > COMPLETION_SPLIT;
    if !completed {
        state.roll_target = 0.0;
    }
    completed
}

/// The hover spring's rebound coefficient while the landing payout runs: a literal `1.0` in
/// place of `ordinary`.
///
/// One of three consumers of `craft+0x1c0 & 0x400` (with [`crate::airbrake::ROLL_GRIP_MULTIPLIER`]
/// and the turbo add in `crate::engine::engine`).
///
/// **This crate's own reconnection of two facts read independently**, not a third traced
/// instance: `input-bindings.md` reads the override as "a hover scalar is forced from
/// `stats+0x4` to a literal `1.0`" without naming the field, and `engine.md` records that the
/// undecoded `craft+0x2a4` enum's `0` state "disables the `rebound` parameter" in this same
/// function. `stats+0x4` sits one field after `ride_height` (`HandlingXml_ParseAntigrav`:
/// `ride_height 0x94`, `rebound 0x98`), so both read as one mechanism on two gates.
/// Confidence 75 on the identification; 90 on the override's existence and `1.0` value.
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

/// Runs the whole gesture for one tick: the timer, both tap sources, the history and the
/// shield-gated arm. Reports whether a roll was armed.
///
/// The only thing [`crate::forces::evaluate`] needs to call; [`advance_tap_timer`],
/// [`record_tap`] and [`arm`] stay public for the tests that pin them one at a time. Calling
/// this *and* [`advance_tap_timer`] in one tick advances the timer twice and halves
/// [`INTER_TAP_TIMEOUT`].
///
/// # The two sources are one signal
///
/// The original writes `1` "when the `LEFT` d-pad bit is pressed **or** the steering axis
/// crosses below `-90`": one history entry either way. The `or` is load-bearing: our input
/// layer maps the d-pad onto the analog axis too (`oag_input::pad::larger`), so both land on
/// the *same* tick, and recording both would double the direction and never match.
/// `LEFT` is tested first when both arrive, as [`crate::airbrake::sideshift_force`] does.
///
/// # Why the axis leg is here
///
/// A crossing is an edge and needs last tick's side, per-craft state
/// ([`ShipState::roll_axis_zone`]). The roll is not scheme-dependent, unlike the sideshift.
///
/// # An AI craft can arm one; the original's opponents almost certainly cannot
///
/// This crate cannot tell a pilot from `oag_ai::Driver`, so an opponent's steering can
/// complete the alternation, and on the disc's circuits it routinely does. **The original
/// reads the gesture from the human pad block only** (confidence 85, 2026-09-06,
/// `input-bindings.md`, "The tap history is the player's pad, and it is cleared on the
/// ground"): the tap leg early-outs on `ship+0x78 == 0` (`0x08846be0`), and the axis edge
/// detector's previous sample is a **single global** at `0x08ae4cf0` used by this one function,
/// which cannot serve eight craft. Keeping it reachable by every craft is a maintainer ruling
/// of 2026-09-06 ("AI may barrel roll, if they have enough shield energy"), a **deliberate
/// deviation** made affordable by the grounded gate below, which is a port.
///
/// # The direct request, and why it is not synthesised taps
///
/// A ruling of the same day: *our* AI barrel-rolls on purpose, an `Ace` whenever its budget
/// allows. After the grounded gate landed an opponent armed **zero** rolls on all twelve
/// circuits by accident, so `oag_ai::Driver` decides and sends [`ShipControls::roll_request`].
/// It goes **through the same two gates as the gesture**: read below the grounded early-out
/// (only an airborne craft arms) and through [`arm`] (`cost < shield` refuses). It skips the
/// tap history on purpose, so invented intent is never laundered through the recovered input
/// path. The third gate, [`within_budget`], is invented and applies to both routes; a real pad
/// leaves [`ShipControls::roll_shield_floor`] at `0.0`.
///
/// # The grounded gate
///
/// The original **cannot arm a roll while in contact with the track**, enforced by zeroing the
/// three-slot tap history every tick the contact bit is set: `0x08846bd0` branches past all tap
/// handling when `craft+0x1c0 & 1` and `0x08847018`-`0x08847030` zero `+0x88c`/`+0x890`/`+0x894`.
/// A gesture cannot even span a takeoff. Confidence 88; same page.
///
/// `contact` is **last** frame's groundedness, as this runs before `Ship_UpdateHover` rebuilds
/// the bit (as [`crate::forces::evaluate`] and `crate::airbrake::sideshift_force`).
///
/// This fixed a regression `crates/game/tests/race_ground_truth.rs`'s
/// `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` caught on 2026-09-06: an Ace
/// armed four to eight rolls a race on ten of twelve circuits, spending 30 to 54 of its 95
/// shield, and on `07_Track` and `16_Track` (never airborne) every charge bought nothing since
/// [`release`] never ran.
///
/// # One roll per flight
///
/// While either arm bit (`craft+0x860 & 0x100`/`& 0x80`, [`ShipState::roll_armed`]) is set the
/// original branches past both pattern compares (`0x08846d14`-`0x08846d30` to `0x08846f58`):
/// no match, no cost, no charge, no phase levelling. Taps still shift into the history, because
/// that block runs before the branch. The bits clear only on the ground/air transitions
/// ([`release`]) and not when the phase finishes, so a roll in progress and a *finished* roll
/// in the same flight both refuse a new one. Confidence 95; `input-bindings.md`, "One roll per
/// flight". The direct request obeys the same gate.
///
/// # A completed pattern levels the phase
///
/// The original writes `0.0` to `+0x87c` on a completed alternation **while unarmed**
/// (`0x08846e5c`, `0x08846f54`), on the far side of the `cost < shield` test, so a refused
/// gesture levels the ship just as an accepted one does and a new roll starts from level
/// rather than from a previous roll's residue.
pub fn advance_gesture(
    state: &mut ShipState,
    input: &ShipControls,
    dimensions: &Dimensions,
    roll_cost: f32,
    contact: bool,
    dt: f32,
) -> bool {
    advance_tap_timer(state, dt);

    if contact {
        state.roll_taps = [0, 0, 0];
        // [`ShipState::roll_axis_zone`] is deliberately *not* refreshed here: the original's
        // edge detector sits inside the airborne branch, so its previous sample goes stale
        // across a grounded stretch and the first airborne tick compares against the side
        // before touchdown. Refreshing would be tidier and a different reading.
        return false;
    }

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

    if state.roll_armed {
        if let Some(direction) = direction {
            shift_tap(state, direction);
        }
        return false;
    }

    if let Some(direction) = direction
        && let Some(sign) = record_tap(state, direction)
    {
        // The invented budget sits *outside* `arm`, which holds only the original's
        // `cost < shield` ([`within_budget`]).
        let armed = within_budget(state, dimensions, roll_cost, input.roll_shield_floor)
            && arm(state, dimensions, roll_cost, sign);
        // Levelled whether or not the shield could pay; the budget refuses like a flat shield.
        state.roll_phase = 0.0;
        return armed;
    }

    // The direct request, last: a craft whose steering completed the alternation this tick
    // already paid, and two charges in one tick is not meant.
    let Some(direction) = input.roll_request else {
        return false;
    };
    let sign = match direction {
        TapDirection::Right => 1.0,
        TapDirection::Left => -1.0,
    };
    if !within_budget(state, dimensions, roll_cost, input.roll_shield_floor)
        || !arm(state, dimensions, roll_cost, sign)
    {
        return false;
    }
    // **Levelled only on success, unlike the gesture above**: levelling is the original's
    // response to a completed *pattern*, which a direct request is not, and a roll armed without
    // it would start from the previous roll's `+-1.0` residue, reach its target instantly and
    // collect the landing payout for nothing.
    state.roll_phase = 0.0;
    true
}

#[cfg(test)]
mod tests;
