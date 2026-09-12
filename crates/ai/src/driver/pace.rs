//! How fast the corner ahead allows, and the brake that gets the craft there.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`, and the seam is a real one rather than a
//! convenient cut: everything here is a **pure** function of the road, the
//! craft's speed, and whatever of [`Driver`](crate::Driver)'s own state the
//! caller chooses to pass in - never [`Driver`](crate::Driver) itself. That is
//! why all six are testable without a craft, and why the steering loop above -
//! which is stateful, and closed on a rate - stays where it is.
//!
//! The chain runs [`curvature_span`] (what the estimator measures over) ->
//! [`corner_target`] (how fast that corner allows) -> [`throttle`] (thrust and
//! the both-sides brake) -> [`track_peak_curvature`] (this tick's high-water
//! mark, which the caller stores) -> [`trail`] (the differential) ->
//! [`airbrakes`] (the two commands the physics actually reads).

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
pub(super) fn corner_target(
    curvature: f32,
    tuning: &Tuning,
    personality: &Personality,
    hull_yaw_ceiling: Option<f32>,
) -> f32 {
    if curvature <= f32::EPSILON {
        return f32::INFINITY;
    }
    let grip = (tuning.lateral_accel * personality.commitment / curvature).sqrt();
    // **The smaller of what the hull can do and what this driver is allowed**,
    // and the two are different questions. See [`hull_yaw_ceiling`] for why the
    // hull's number belongs here and [`Tuning::max_turn_rate`] for why the
    // permission still caps it: `Difficulty::tune` scales the permission, and a
    // Novice that read the hull directly would corner like an Ace.
    let rate = hull_yaw_ceiling.map_or(tuning.max_turn_rate, |hull| hull.min(tuning.max_turn_rate));
    let yaw = rate / curvature;
    grip.min(yaw)
}

/// The yaw rate a craft's own hull can actually sustain, in radians per second.
///
/// **Authored data, not a tuned constant**, which is the whole point: the
/// steady state of `oag_physics`'s own yaw axis, evaluated on the craft being
/// flown rather than on one global belief.
///
/// ```text
/// omega = steer * Turning.amount / (damping * I_yy)
/// ```
///
/// `oag_physics::engine::steering` is `steer * Turning.amount` as a body-local
/// yaw drive with no speed factor; `oag_physics::passive::YAW_DAMPING` damps
/// yaw *momentum* at `-5`; and `I_yy` is `1 /
/// oag_physics::forces::YAW_INVERSE_INERTIA`. `steer` runs to
/// `oag_physics::controls::CONTROL_RANGE`, so full lock is `100`.
///
/// # `I_yy` is global and `Turning.amount` is not
///
/// The inertia box `(12, 8, 12)` and the mass `0.9` it is built with are **code
/// literals at a single call site** in the ship-entity constructor - `Misc`
/// `width`/`length`/`height` reach the *collider*, not the tensor - so every
/// craft in the game has the same `I_yy` of `21.6`. See
/// [`oag_physics::forces::YAW_INVERSE_INERTIA`], which records the
/// disassembly. The only per-craft term is `<Turning amount>`, and that one is
/// **per team and constant across the four speed classes**, measured over the
/// whole disc by `crates/game/tests/ai_clean_lap_board.rs`:
///
/// | team | `amount` | ceiling | against `Tuning::max_turn_rate` 1.8 |
/// | --- | ---: | ---: | ---: |
/// | Feisar | 1.80 | 1.667 | -7.4 % |
/// | Assegai, AG Systems | 1.68 | 1.556 | -13.6 % |
/// | Qirex | 1.55 | 1.435 | -20.3 % |
/// | EGX, Goteki | 1.42 | 1.315 | -27.0 % |
/// | Triakis, Piranha | 1.30 | 1.204 | -33.1 % |
///
/// **No craft on the disc can reach 1.8**, so the kinematic half of
/// [`corner_target`] was asking every one of them for a corner speed its hull
/// could not rotate at - by 7 % on the best team and 33 % on the worst. The
/// 1.556 row is the one the Outpost 7 thread measured by hand, and
/// `docs/ghidra/functions/psp-pulse-usa/engine.md` measures the *original*
/// hull at 1.42-1.51 under full lock, so this arithmetic lands within a few per
/// cent of the original's own behaviour.
#[must_use]
pub fn hull_yaw_ceiling(handling: &oag_physics::Handling) -> f32 {
    let i_yy = 1.0 / oag_physics::forces::YAW_INVERSE_INERTIA;
    oag_physics::controls::CONTROL_RANGE * handling.turning.amount
        / (-oag_physics::passive::YAW_DAMPING * i_yy)
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
/// # What used to gate this, and why it was replaced
///
/// The original gate was `speed < target`: off below the corner's modelled
/// target speed, meant to read as "corner exit, where the grip is wanted for
/// accelerating". **Measured** on a real circuit (Talon's Junction, Pulse PSP)
/// it instead gated out corner *entry* too - `command` pinned at `+-1.0`,
/// `rate_error` 0.5-0.6 rad/s, for over ten consecutive ticks while speed
/// collapsed from 111 to 39.5 units/s and `target` sat at 160-180 throughout,
/// `speed < target` holding on every one of them. Replacing it with
/// `!target.is_finite()` was tried and reverted: `target` is a real number
/// everywhere on disc geometry - zero of 5,336 ticks over two laps of Talon's
/// Junction were infinite, against 303 on `tests/closed_loop.rs`'s exactly
/// collinear synthetic straights - so the replacement gated out nothing there
/// and ground one opponent's shield to zero on
/// `opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`.
/// Full account, including the attempt and its revert, in `docs/gameplay/ai.md`,
/// "Airbrakes, and what a differential one actually does".
///
/// **What actually discriminates entry from exit, measured on the same real
/// lap**: not `target`'s absolute value - two real segments, one still
/// tightening into the corner and one already accelerating out of it, both
/// held `speed` a similar fraction below `target` throughout, so no ratio of
/// the two separates them either. What differs is `curvature`'s own *trend*:
/// flat to rising while still working the corner, falling once the corner
/// opens up on exit. [`Driver::peak_curvature`](crate::Driver::peak_curvature)
/// tracks the high-water mark since the line last went straight, and
/// `curvature` falling meaningfully below it is what exit looks like.
///
/// A second, unrelated real-track failure mode surfaced by the same
/// measurement: a weapon hit **halves a craft's speed in a single tick**
/// (`ShipState::slowdown_timer` arms the tick after), which the pure-pursuit
/// loop reads as exactly the shape of a genuine corner - `command` and
/// `rate_error` both saturate correcting for the sudden mismatch between
/// heading and velocity - on track geometry that is nearly straight the whole
/// time. `oag_physics::forces::evaluate` already skips lateral grip entirely
/// while the timer runs, so a differential spent there buys nothing and
/// affects nothing the craft can use; gated out below, alongside
/// [`ShipState::stun_timer`] for the same reason once something arms it.
///
/// Four gates now, each doing a different job:
///
/// - **Recovering.** [`ShipState::slowdown_timer`] or
///   [`ShipState::stun_timer`] positive - the craft is not steering into
///   anything, it is coasting off a hit with no lateral grip to spend the
///   differential against.
/// - **Curvature floor.** [`Tuning::trail_curvature_floor`] - below it this is
///   a straight, or close enough that the chord estimate's own noise (~5e-5 on
///   Talon's Junction) cannot be told from one. `!target.is_finite()`'s
///   mistake was assuming a straight makes itself known this cleanly; it does
///   not, curvature does.
/// - **Exit.** `curvature < peak_curvature * `[`Tuning::trail_exit_decay`] -
///   the corner has opened up enough since its tightest point that this reads
///   as corner exit rather than still being fought through.
/// - **Saturation and deadband.** The gate logic is unchanged from before:
///   below [`Tuning::trail_saturation`] of full lock the rate loop still has
///   authority of its own, and [`Tuning::trail_deadband`] keeps this out of
///   the small-signal regime `tests/closed_loop.rs` linearised about. The
///   *values* moved on the same date as the three gates above, for an
///   unrelated reason - see [`Tuning::trail_saturation`]'s own doc.
///
/// No slew limiting here, and none needed: this is a *target*, and
/// `oag_physics::controls::update` ramps the airbrake states toward it at
/// `Airbrake::gain`/`falloff`. The plant is the rate limiter, so the caller's
/// own state - [`Driver::peak_curvature`](crate::Driver::peak_curvature) -
/// is the only memory this needs, and this function stays a pure scalar
/// function of it.
///
/// [`ShipState::slowdown_timer`]: oag_physics::ShipState::slowdown_timer
/// [`ShipState::stun_timer`]: oag_physics::ShipState::stun_timer
pub(super) fn trail(
    steer: &Steer,
    curvature: f32,
    peak_curvature: f32,
    recovering: bool,
    tuning: &Tuning,
    personality: &Personality,
) -> f32 {
    if recovering
        || curvature <= tuning.trail_curvature_floor
        || curvature < peak_curvature * tuning.trail_exit_decay
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

/// The next [`Driver::peak_curvature`](crate::Driver::peak_curvature): the
/// high-water mark since the line last went straight, or `curvature` itself
/// once it has.
///
/// Runs every tick regardless of saturation, so the mark is already current
/// the moment a corner does saturate the steering loop - a craft that enters a
/// bend below `trail_saturation` and only saturates near the apex must not
/// read as "exiting" on its first saturated tick for want of a mark taken this
/// tick.
pub(super) fn track_peak_curvature(curvature: f32, previous_peak: f32, tuning: &Tuning) -> f32 {
    if curvature <= tuning.trail_curvature_floor {
        curvature
    } else {
        curvature.max(previous_peak)
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
