//! The barrel roll: a three-tap gesture, a signed phase that completes itself,
//! and a shield-gated arm.
//!
//! Recovered in `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`'s "The
//! tap-history path is the barrel roll" section, confidence 90 - every tunable
//! is authored `<Global><Special>` XML, read live off both shipped PSP
//! pressings. See [`ShipState::roll_taps`] and its neighbours for the state
//! this module advances.
//!
//! # The release event is now read, and it is the landing
//!
//! The original arms a roll by setting one of two bits
//! (`entity+0x860 & 0x100`/`& 0x80`) that ramp [`ShipState::roll_phase`] toward
//! `+1.0`/`-1.0`. What clears them was this module's own guess until
//! 2026-09-06, when `Ship_UpdateSideshiftInput_q` (`0x08846a54`) was read
//! whole: the clear sits in the **airborne-to-grounded** branch
//! (`craft+0x1c0 & 1` set this tick, `craft+0x860 & 0x200` clear, meaning it
//! was not set last tick), in the same `if` that arms the payout off
//! `|entity+0x87c| > 0.5`. So [`release`]'s event was the right guess and is
//! now a reading. A second clear runs on the opposite, grounded-to-airborne
//! transition (`0x08846ab4`-`0x08846ac8`); this port omits it because the
//! grounded gate in [`advance_gesture`] makes an armed roll on the ground
//! unreachable, so it would clear nothing.
//!
//! # The payout is gated on the arm, not on the phase
//!
//! The landing's `|entity+0x87c| > 0.5` test sits inside an "armed" test, and the
//! landing clears the arm bits. [`ShipState::roll_armed`] is that pair. Without
//! it a completed roll left the phase at `+-1.0` and every later landing paid
//! again (maintainer report from play: on a wavy track, "insane boosts").

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

/// Whether spending a roll's cost would leave the pool above
/// [`ShipControls::roll_shield_floor`].
///
/// # This gate is invented, and it is the only invented rule this mechanic
/// carries
///
/// Nothing on the disc authors it and nothing in the recovered chain gates on
/// it. It was chosen by the maintainer on 2026-09-06 - "I think AI should still
/// avoid barrel rolls if they are low on energy, like below 20% or something,
/// unsure what the original does here, but I'd be good with inventing a value
/// here too" - and it is recorded as a design decision, **not** as a finding.
/// It deliberately carries no confidence score: a score would let a later
/// reader cite a choice as evidence.
///
/// It sits *on top of* the original's own gate, which is [`arm`]'s
/// `cost < shield` and is recovered at confidence 90. A craft flown by
/// `oag_ai::Driver` needs both; a human craft leaves the floor at `0.0` and
/// needs only the recovered one. **That asymmetry is the deviation**, and it is
/// an AI-quality choice rather than a claim about how the original's craft
/// behave.
///
/// The `0.20` this used to hold as a bare `AI_ROLL_SHIELD_FLOOR` constant is
/// now the low end of `oag_ai::Pilot::BALANCED`'s `roll_floor`, so it is a
/// per-pilot number a player can retune in a file. If the original's own AI
/// gate is ever recovered, this is **replaced** by it rather than reconciled
/// with it.
#[must_use]
pub fn within_budget(
    state: &ShipState,
    dimensions: &Dimensions,
    roll_cost: f32,
    shield_floor: f32,
) -> bool {
    // A hard floor: what is compared is the pool the roll would *leave*, not
    // the one it starts from. The other reading lets a craft sitting exactly on
    // its floor spend anyway and land under it, which is the shape of "an
    // opponent that rolled itself down to nothing".
    state.shield - roll_cost * 0.01 * dimensions.shield >= shield_floor * dimensions.shield
}

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
    state.roll_armed = true;
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
/// phase ramps back down) and this returns `false`. A ship with no armed
/// roll ([`ShipState::roll_armed`] clear) is left untouched and reports
/// `false`, which is what stops a landing after a completed roll paying again.
pub fn release(state: &mut ShipState) -> bool {
    // The original's landing clears both arm bits whether or not it paid, and
    // the payout is gated on them, so a roll pays at most once. `roll_target`
    // is left at `+-1.0` on a completion so the phase still runs on.
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
/// # An AI craft can arm one, and the original's opponents almost certainly cannot
///
/// This crate does not know whether a craft is flown by a pilot or by
/// `oag_ai::Driver` - both arrive as [`ShipControls`] - and the axis leg has
/// no human-only gate the way the novice flick's
/// [`ShipControls::shift_modifier`] effectively is. So an opponent's own
/// steering can complete the alternation, and on the disc's own circuits it
/// routinely does.
///
/// **The original reads this gesture out of the human player's pad block and
/// nothing else**, recovered 2026-09-06 at confidence 85 - see
/// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`, "The tap history is
/// the player's pad, and it is cleared on the ground". Two legs: the whole tap
/// leg early-outs on `ship+0x78 == 0` (`0x08846be0`), and the axis edge
/// detector's previous-sample store is a **single global** at `0x08ae4cf0`,
/// referenced from this one function and nowhere else - a per-process scalar
/// cannot serve eight craft at once.
///
/// This project keeps the gesture reachable by every craft all the same, on a
/// maintainer's ruling of 2026-09-06 ("AI may barrel roll, if they have enough
/// shield energy") rather than as a port of the original. It is a **deliberate
/// deviation**, recorded so the next reader does not mistake it for a finding;
/// what makes it affordable is the grounded gate below, which is a port.
///
/// # The direct request, and why it is not synthesised taps
///
/// A ruling of the same day went further: *our* AI barrel-rolls on purpose,
/// with an `Ace` rolling whenever its energy budget allows. Leaving that to an
/// accidental alternation of the driver's own steering would not have produced
/// it - after the grounded gate below landed, an opponent armed **zero** rolls
/// on all twelve circuits - so the decision is taken in `oag_ai::Driver` and
/// arrives here on [`ShipControls::roll_request`].
///
/// That request is honoured **through the same two gates the gesture is**: it
/// is read below the grounded early-out, so an airborne craft is the only kind
/// that can arm one, and it reaches [`arm`], so `cost < shield` still refuses
/// it. What it skips is the tap history, which is the point - an invented
/// intent routed back through the recovered input path would be
/// indistinguishable from the recovered path a year from now.
///
/// The third gate, [`within_budget`], is invented and applies to both routes.
/// It reads [`ShipControls::roll_shield_floor`], which a real pad leaves at
/// `0.0`, so a human keeps the recovered behaviour exactly.
///
/// # The grounded gate
///
/// The original **cannot arm a roll while the craft is in contact with the
/// track**, and it enforces that by zeroing the whole three-slot tap history
/// every tick the contact bit is set rather than by refusing at the arm:
/// `0x08846bd0` branches past all tap handling when `craft+0x1c0 & 1` is set
/// and `0x08847018`-`0x08847030` write zero to `+0x88c`/`+0x890`/`+0x894`.
/// So a gesture cannot even span a takeoff, let alone complete on the ground.
/// Confidence 88; same evidence page.
///
/// `contact` is **last** frame's groundedness, because that is what the
/// original reads: this function runs before `Ship_UpdateHover` rebuilds the
/// bit, the same ordering [`crate::forces::evaluate`] keeps and the same value
/// `crate::airbrake::sideshift_force` is handed.
///
/// This is the fix for the regression `crates/game/tests/race_ground_truth.rs`'s
/// `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` caught on
/// 2026-09-06: an Ace opponent was arming four to eight rolls a race on ten of
/// the twelve circuits and spending 30 to 54 of its 95 shield, and on
/// `07_Track` and `16_Track` - which it never leaves the ground on - every one
/// of those charges bought nothing, because [`release`] never ran. With the
/// gate, a craft that never flies never pays.
///
/// # A completed pattern levels the phase
///
/// The original writes `0.0` to `+0x87c` on **any** completed alternation
/// (`0x08846e5c`, `0x08846f54`), on the far side of the `cost < shield` test,
/// so a refused gesture levels the ship just as an accepted one does and a new
/// roll always starts from level rather than from a previous roll's residue.
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
        // [`ShipState::roll_axis_zone`] is deliberately *not* refreshed here.
        // The original's edge detector sits inside the airborne branch, so its
        // previous sample goes stale across a grounded stretch and the first
        // airborne tick compares against whichever side the axis was on before
        // touchdown. Refreshing it would be the tidier reading and a different
        // one.
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

    if let Some(direction) = direction
        && let Some(sign) = record_tap(state, direction)
    {
        // The invented budget sits *outside* `arm`, which holds the original's
        // own `cost < shield` and nothing else. See [`within_budget`].
        let armed = within_budget(state, dimensions, roll_cost, input.roll_shield_floor)
            && arm(state, dimensions, roll_cost, sign);
        // Levelled whether or not the shield could pay - see the section above.
        // The budget refuses the same way a flat shield does, so it levels too.
        state.roll_phase = 0.0;
        return armed;
    }

    // The direct request, last: a craft whose own steering happened to complete
    // the alternation this tick has already spent the pool on that, and a
    // second charge in one tick is not a thing either route means.
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
    // **Levelled only on success, unlike the gesture above**, and the
    // difference is not cosmetic. Levelling is the original's response to a
    // completed *pattern*, which a direct request is not; and a roll armed
    // without it would start from the previous roll's `+-1.0` residue, reach
    // its target instantly and collect the landing payout for nothing.
    state.roll_phase = 0.0;
    true
}

#[cfg(test)]
mod tests;
