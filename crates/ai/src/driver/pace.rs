//! How fast the corner ahead allows, and the brake that gets the craft there.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`, and the seam is a real one rather than a
//! convenient cut: everything here is a **scalar** function of the road and the
//! craft's speed, with no access to [`Driver`](crate::Driver)'s own state at all. That is why
//! all five are testable without a craft, and why the steering loop above -
//! which is stateful, and closed on a rate - stays where it is.
//!
//! The chain runs [`curvature_span`] (what the estimator measures over) ->
//! [`corner_target`] (how fast that corner allows) -> [`throttle`] (thrust and
//! the both-sides brake) -> [`trail`] (the differential) -> [`airbrakes`] (the
//! two commands the physics actually reads).

use super::{Personality, Steer, Tuning};

/// The chord [`Line::curvature`](crate::Line::curvature) measures over, for a driver looking `look`
/// ahead.
///
/// Half the lookahead unless [`Tuning::curvature_span`] caps it, which is where
/// the reasoning and the sweep behind the cap are written down.
///
/// **All three callers share this** - the braking window in [`Driver::drive`](crate::Driver::drive),
/// [`Driver::allows_speed`](crate::Driver::allows_speed)'s boost gate and the rocket gate in
/// [`super::weapons`] - because they are asking one estimator the same question
/// about the same road. A span that differed between them would mean a corner a
/// craft brakes for is one it will still fire a rocket through, and the
/// twelve-circuit sweep only reproduces its reference point with all three
/// capped together.
pub(super) fn curvature_span(tuning: &Tuning, look: f32) -> f32 {
    let span = look * 0.5;
    match tuning.curvature_span {
        Some(cap) => span.min(cap),
        None => span,
    }
}

/// The fastest the corner ahead can be taken, or infinity where there is no
/// corner.
///
/// **Two limits, and the smaller of them binds.**
///
/// `sqrt(lateral_accel / curvature)` is the ordinary cornering limit - how fast
/// the craft can go round before it slides. [`Personality::commitment`] scales
/// the grip a driver assumes it has, so the eight targets are eight different
/// numbers and the field does not lift off and brake in unison. It is under the
/// square root, so a spread of a few per cent in commitment is half that in
/// speed.
///
/// `max_turn_rate / curvature` is the other one, and it is **kinematics rather
/// than a tuned constant**: a craft at speed `v` on a line of curvature `k` has
/// to yaw at `v * k` to stay on it, so a hull that cannot rotate faster than
/// `w` cannot hold that line above `w / k` however much grip it has. Nothing to
/// do with sliding - the craft leaves the line pointing the wrong way, with slip
/// flat and every wheel on the ground.
///
/// # Why the second one was missing, and what it cost
///
/// `07_Track`'s tightest arc has curvature 0.047 - radius 21 - which admits
/// **33 units/s**. The hull's ceiling is `steer * Turning.amount / (5 * I_yy)` =
/// `100 * 1.68 / 108` = 1.556 rad/s, within a few per cent of the 1.42-1.51 that
/// `docs/ghidra/functions/psp-pulse-usa/engine.md` measures on the original. The
/// grip limit alone said 74 and the craft arrived at 94, left the line, and shed
/// **34-35 shield a lap** grinding down the outside wall.
///
/// The grip term cannot reach that corner from either end: dropping
/// [`Tuning::lateral_accel`] from 260 to 90 moved the loss from 24.08 to 24.11,
/// because `sqrt(90 / 0.047)` is still 43.8 against a yaw limit of 33.
///
/// Board-wide this is worth **51 shield across the twelve circuits** for 1.2 s
/// of mean clean lap, measured by `sweep_curvature_span` in
/// `race_ground_truth.rs`. It binds above `k = max_turn_rate^2 / lateral_accel`,
/// about 0.0125 at the defaults, which is 20 per cent of `07`'s samples and 26
/// per cent of `06`'s.
///
/// Infinity rather than an option, because every caller wants "is this speed
/// allowed", and `speed <= f32::INFINITY` is the right answer for a straight
/// without a branch of its own.
pub(super) fn corner_target(curvature: f32, tuning: &Tuning, personality: &Personality) -> f32 {
    if curvature <= f32::EPSILON {
        return f32::INFINITY;
    }
    let grip = (tuning.lateral_accel * personality.commitment / curvature).sqrt();
    let yaw = tuning.max_turn_rate / curvature;
    grip.min(yaw)
}

/// Thrust, and the brake held on **both** sides, from the speed the corner
/// ahead allows.
///
/// Past [`Tuning::brake_margin`] the command climbs from
/// [`Tuning::brake_floor`] with the overspeed rather than snapping to one.
///
/// **This does not ramp the deceleration.** `oag_physics::controls::update`
/// gates `ShipState::brake` on both inputs being strictly positive and its ramp
/// rate never reads their level, so a command of `0.35` and a command of `1.0`
/// slow the craft at exactly the same rate. What climbs with the command is
/// `max(L, R)`, which is what the lateral-grip coefficient reads. So this is a
/// dial on **how much cornering grip the deceleration is bought with**, and a
/// driver only a little over its target keeps the grip it is about to need.
pub(super) fn throttle(speed: f32, target: f32, tuning: &Tuning) -> (f32, f32) {
    if speed <= target {
        return (1.0, 0.0);
    }
    // Finite, because `speed <= target` already returned for an infinite one.
    let overspeed = speed / target - 1.0;
    if overspeed <= tuning.brake_margin {
        return (0.0, 0.0);
    }
    let brake = (tuning.brake_floor + (overspeed - tuning.brake_margin) * tuning.brake_gain)
        .clamp(tuning.brake_floor, 1.0);
    (0.0, brake)
}

/// The differential airbrake: how much harder one side is held than the other,
/// positive when the extra braking goes on the **right**.
///
/// # This is not a brake
///
/// Braking one side alone yaws the nose toward that side, pushes the body away
/// from it, adds a little forward speed, and cuts lateral grip exactly as hard
/// as holding both sides would - and it engages no deceleration at all, because
/// that needs both. Three of those four are the wrong sign for what "trail
/// braking" usually means. What it actually buys is **yaw authority, paid for
/// in grip**, which is why it is spent only where the steering loop has run out
/// of authority of its own.
///
/// Three gates, each doing a different job:
///
/// - **Saturation.** Below [`Tuning::trail_saturation`] of full lock the rate
///   loop still has authority, and a second path in parallel with it is the
///   oscillation this crate was rewritten to remove. Above it the loop is
///   asking for more than the steering input can deliver.
/// - **Deadband.** Keeps it out of the small-signal regime, so the loop
///   linearised about the line is provably the one `tests/closed_loop.rs`
///   measured.
/// - **Overspeed.** Never below the corner's target speed, which is corner
///   exit - where the grip is wanted for accelerating and where the forward
///   slide term is at its largest. On a straight `target` is infinite and this
///   gate is what keeps the differential off it.
///
/// # This also gates out corner *entry*, and that is a live, unfixed bug
///
/// Read literally, "never below the corner's target speed" sounds like it
/// only ever excludes corner exit. It does not: `speed < target` fires for as
/// long as the craft's speed has not yet crossed a target computed from
/// [`Tuning::lateral_accel`] - a model of the grip available, not a
/// measurement of it on this corner - which is equally true on the way *in*.
/// A craft understeering hard enough to saturate the steering loop while
/// still measurably below that assumed target gets no assistance at all:
/// exactly the case the paragraph above says this exists for.
///
/// **Measured** on a real circuit (Talon's Junction, Pulse PSP): `command`
/// pinned at `+-1.0`, `rate_error` 0.5-0.6 rad/s - both well past
/// `trail_saturation`/`trail_deadband` - for over ten consecutive ticks while
/// speed collapsed from 111 to 39.5 units/s and `target` sat at 160-180
/// throughout. `speed < target` held on every one of those ticks, so the
/// differential was zero on every one of them. See
/// `docs/gameplay/ai.md#airbrakes-and-what-a-differential-one-actually-does`.
///
/// **Tried and reverted**: replacing the condition with `!target.is_finite()`,
/// off only on a genuine straight where [`corner_target`] returns infinity
/// rather than below-target anywhere. That passed every `tests/closed_loop.rs`
/// case including the stability guard, because the `oval_of` fixture in
/// `tests/closed_loop.rs` builds its
/// straights from exactly collinear points and so they are genuinely
/// infinite. **A real racing line's three sampled points are never exactly
/// collinear**, so `target` is finite everywhere on a disc track - measured
/// as zero `f32::INFINITY` targets over a full lap of Talon's Junction,
/// against 303 on the closed-loop oval in the same run. The replacement gate
/// was therefore inert on real geometry: the differential fired on every
/// saturated tick anywhere, including off-line recovery mid-corner, which
/// `tests/closed_loop.rs` cannot see because its own recoveries only happen
/// on its exactly-straight segments. On a real seven-craft field
/// (`opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`)
/// this ground one opponent's shield to zero over a minute of ordinary
/// racing, against a `full * 0.45` floor the unmodified gate clears
/// comfortably. A fix needs a threshold that actually discriminates on real
/// track curvature - not a re-derivation of "is this a straight" - and that
/// is a tuning question, not a logic one: see `HANDOVER.md`.
///
/// No slew limiting here, and none needed: this is a *target*, and
/// `oag_physics::controls::update` ramps the airbrake states toward it at
/// `Airbrake::gain`/`falloff`. The plant is the rate limiter, so the driver
/// needs no state of its own to remember.
pub(super) fn trail(
    steer: &Steer,
    speed: f32,
    target: f32,
    tuning: &Tuning,
    personality: &Personality,
) -> f32 {
    if speed < target
        || steer.command.abs() < tuning.trail_saturation
        || steer.rate_error.abs() <= tuning.trail_deadband
    {
        return 0.0;
    }
    let past = steer.rate_error.abs() - tuning.trail_deadband;
    let magnitude = (past * tuning.trail_gain * personality.trail).min(tuning.trail_max);
    // Positive `rate_error` is a craft that wants to turn further right, and a
    // nose-right yaw needs `imbalance = L - R` negative - so the **right** side
    // is the one braked. Getting this backwards is what `5ad69f3` shipped for
    // months; `the_differential_brakes_the_side_the_nose_is_turning_toward`
    // pins it, and `oag_physics::airbrake`'s own header settles the sign.
    if steer.rate_error >= 0.0 {
        magnitude
    } else {
        -magnitude
    }
}

/// The two airbrake commands, from the symmetric brake and the differential.
///
/// The two sides are an interval of width `differential` slid to sit as near
/// the symmetric brake as it will go, rather than the brake plus and minus half
/// of it. Two reasons, and both are about not losing the yaw where it is most
/// needed:
///
/// - **A craft braking flat out has no headroom above.** Adding to one side
///   alone would clip against `1.0` and deliver nothing, exactly in the corner
///   the driver is most in trouble in. Sliding the interval down instead keeps
///   the imbalance the caller asked for.
/// - **The low side must stay strictly positive whenever the brake is on**,
///   because both sides positive is the only thing that engages
///   `ShipState::brake`. Dropping one to zero mid-corner would silently cancel
///   the deceleration. `floor` is the limit it may slide to; below it the
///   differential is what shrinks, never the brake.
///
/// With the brake off, `floor` is zero and one side rises from nothing: yaw
/// authority and no deceleration, which is what a differential airbrake
/// physically is.
pub(super) fn airbrakes(brake: f32, differential: f32, floor: f32) -> (f32, f32) {
    let limit = if brake > 0.0 { floor } else { 0.0 };
    let width = differential.abs().min(1.0 - limit);
    let low = brake.clamp(limit, 1.0 - width);
    let high = low + width;
    if differential >= 0.0 {
        (low, high)
    } else {
        (high, low)
    }
}
