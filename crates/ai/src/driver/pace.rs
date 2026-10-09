//! How fast the corner ahead allows, and the brake that gets the craft there.
//!
//! Split out of [`super`] for `scripts/check-file-size.py`. Everything here is
//! a **pure** function of the road, the speed and whatever [`Driver`](crate::Driver)
//! state the caller passes in, so it is testable without a craft.
//!
//! The chain: [`curvature_span`] (what the estimator measures over) ->
//! [`corner_target`] -> [`throttle`] -> [`track_peak_curvature`] (the mark the
//! caller stores) -> [`trail`] (the differential) -> [`airbrakes`] (the two
//! commands the physics reads).

use super::{Personality, Steer, Tuning};

/// The chord [`Line::curvature`](crate::Line::curvature) measures over, for a
/// driver looking `look` ahead: half the lookahead unless
/// [`Tuning::curvature_span`] caps it (reasoning and sweep there).
///
/// **All three callers share this** - [`Driver::drive`](crate::Driver::drive)'s
/// braking window, [`Driver::allows_speed`](crate::Driver::allows_speed) and
/// the rocket gate in [`super::weapons`] - or a corner a craft brakes for is
/// one it fires a rocket through, and the twelve-circuit sweep only reproduces
/// its reference with all three capped together.
pub(super) fn curvature_span(tuning: &Tuning, look: f32) -> f32 {
    let span = look * 0.5;
    match tuning.curvature_span {
        Some(cap) => span.min(cap),
        None => span,
    }
}

/// The chord a reading averages over, given the step [`curvature_span`] chose.
/// `None` keeps the two equal, see [`Tuning::curvature_chord`].
pub(super) fn curvature_chord(tuning: &Tuning, step: f32) -> f32 {
    tuning.curvature_chord.map_or(step, |chord| chord.min(step))
}

/// The fastest the corner ahead can be taken, or infinity where there is none.
///
/// **Two limits, the smaller binds.** `sqrt(lateral_accel / curvature)` is the
/// grip limit, scaled by [`Personality::commitment`] (under the root, so a few
/// per cent of commitment is half that in speed) so the field does not brake in
/// unison. `max_turn_rate / curvature` is **kinematics, not a tuned
/// constant**: yawing at `v * k` is required to hold a line, so a hull that
/// cannot rotate faster than `w` cannot hold it above `w / k` whatever its
/// grip. The craft leaves the line pointing the wrong way with slip flat.
///
/// # Why the kinematic limit was added
///
/// `07_Track`'s tightest arc (curvature 0.047, radius 21) admits **33 units/s**.
/// The hull's ceiling is `steer * Turning.amount / (5 * I_yy)` = `100 * 1.68 /
/// 108` = 1.556 rad/s on Pulse's craft, within a few per cent of the 1.42-1.51
/// `docs/ghidra/functions/psp-pulse-usa/engine.md` measures on the original.
/// The grip limit said 74, the craft arrived at 94 and shed **34-35 shield a
/// lap** on the outside wall. Dropping [`Tuning::lateral_accel`] 260 to 90
/// could not reach it (24.08 to 24.11 loss; `sqrt(90 / 0.047)` is 43.8).
/// Board-wide it is worth **51 shield across twelve circuits** for 1.2 s of
/// mean clean lap (`sweep_curvature_span`, `race_ground_truth.rs`). It binds
/// above `k = max_turn_rate^2 / lateral_accel`, about 0.0125, which is 20 per
/// cent of `07`'s samples and 26 per cent of `06`'s.
///
/// Infinity rather than an option, so a straight needs no branch of its own.
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
    // The smaller of what the hull can do ([`hull_yaw_ceiling`]) and what this
    // driver is allowed ([`Tuning::max_turn_rate`]): `Difficulty::tune` scales
    // the permission, and a Novice reading the hull directly would corner like
    // an Ace.
    let rate = hull_yaw_ceiling.map_or(tuning.max_turn_rate, |hull| hull.min(tuning.max_turn_rate));
    let yaw = rate / curvature;
    grip.min(yaw)
}

/// The yaw rate a craft's own hull can sustain, in radians per second.
///
/// **Authored data, not a tuned constant**: the steady state of `oag_physics`'s
/// yaw axis on the craft being flown.
///
/// ```text
/// omega = steer * Turning.amount / (damping * I_yy)
/// ```
///
/// `oag_physics::engine::steering` is `steer * Turning.amount` as a yaw drive
/// with no speed factor; `oag_physics::passive::YAW_DAMPING` is `-5`; `I_yy` is
/// `1 / oag_physics::forces::YAW_INVERSE_INERTIA`; full lock is
/// `oag_physics::controls::CONTROL_RANGE` (`100`).
///
/// # `I_yy` is the craft's own, per title, and `Turning.amount` is per team
///
/// `I_yy` is read off `body.inertia`, the tensor the race seated the craft
/// with, so the AI plans on the hull it actually flies. The inertia box
/// `(12, 8, 12)` is a code literal at one call site and the mass it is built
/// with is a per-title literal: Pulse passes `0.9` (`I_yy` 21.6,
/// [`oag_physics::forces::YAW_INVERSE_INERTIA`]), Wipeout HD `1.0` (`I_yy` 24,
/// `oag_title::craft_laws::CraftLaws`, measured on RPCS3), so HD's ceilings
/// are `0.9` of the table below. A title with no `CraftLaws` flies Pulse's.
/// `<Turning amount>` is **per team, constant across speed classes**,
/// measured by `crates/game/tests/ai_clean_lap_board.rs`. On Pulse:
///
/// | team | `amount` | ceiling | against `Tuning::max_turn_rate` 1.8 |
/// | --- | ---: | ---: | ---: |
/// | Feisar | 1.80 | 1.667 | -7.4 % |
/// | Assegai, AG Systems | 1.68 | 1.556 | -13.6 % |
/// | Qirex | 1.55 | 1.435 | -20.3 % |
/// | EGX, Goteki | 1.42 | 1.315 | -27.0 % |
/// | Triakis, Piranha | 1.30 | 1.204 | -33.1 % |
///
/// No craft reaches 1.8, so the old kinematic limit asked every one for a
/// corner speed its hull could not rotate at. The 1.556 row is the Outpost 7
/// thread's hand measurement. HD's Feisar reads `1.5`, the yaw rate RPCS3
/// measures (`docs/physics/hd-handling-ground-truth.md`).
///
/// The steering ramp is not modelled here: the steady state does not depend
/// on it, and the speed plan's own simulation steps the real ramp through
/// `oag_physics`, clamped where the craft's `ShipState::steer_ramp_clamped`
/// says so.
#[must_use]
pub fn hull_yaw_ceiling(handling: &oag_physics::Handling, body: &oag_physics::Body) -> f32 {
    oag_physics::controls::CONTROL_RANGE * handling.turning.amount
        / (-oag_physics::passive::YAW_DAMPING * body.inertia.y)
}

/// Thrust, and the brake held on **both** sides, from the speed the corner
/// ahead allows.
///
/// Past [`Tuning::brake_margin`] the command climbs from
/// [`Tuning::brake_floor`] with the overspeed rather than snapping.
///
/// **It does not ramp the deceleration**: `oag_physics::controls::update` gates
/// `ShipState::brake` on both inputs being strictly positive and never reads
/// their level. What climbs is `max(L, R)`, which the lateral-grip coefficient
/// reads, so this dials **how much cornering grip the deceleration is bought
/// with**.
pub(super) fn throttle(speed: f32, target: f32, tuning: &Tuning) -> (f32, f32) {
    if speed <= target {
        return (1.0, 0.0);
    }
    // Finite: `speed <= target` already returned for an infinite one.
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
/// One side alone yaws the nose toward it, pushes the body away, adds a little
/// forward speed and cuts lateral grip as hard as both sides would, with no
/// deceleration (that needs both). It buys **yaw authority, paid for in grip**,
/// so it is spent only where the steering loop has run out of authority.
///
/// # What used to gate this, and why it was replaced
///
/// The gate was `speed < target`, meant as "corner exit". **Measured** on
/// Talon's Junction (Pulse PSP) it also gated out corner *entry*: `command`
/// pinned at `+-1.0`, `rate_error` 0.5-0.6 rad/s for over ten ticks while speed
/// fell from 111 to 39.5 and `target` sat at 160-180. `!target.is_finite()` was
/// tried and reverted: no disc track's `target` is infinite (0 of 5,336 ticks
/// over two laps, against 303 on `tests/closed_loop.rs`'s collinear synthetic
/// straights), so it gated out nothing and ground one opponent's shield to
/// zero on
/// `opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`.
/// Account: `docs/gameplay/ai.md`, "Airbrakes, and what a differential one
/// actually does".
///
/// What discriminates entry from exit is `curvature`'s own *trend*, not
/// `target` or any `speed / target` ratio (an entry segment and an exit segment
/// both held `speed` a similar fraction below `target`): flat to rising while
/// working the corner, falling once it opens.
/// [`Driver::peak_curvature`](crate::Driver::peak_curvature) tracks the
/// high-water mark.
///
/// A second failure: a weapon hit **halves a craft's speed in one tick**, which
/// pure pursuit reads as a genuine corner on near-straight track (`command` and
/// `rate_error` both saturate). `oag_physics::forces::evaluate` skips lateral
/// grip while the timer runs, so a differential there buys nothing.
///
/// Four gates:
///
/// - **Recovering**: [`ShipState::slowdown_timer`] or [`ShipState::stun_timer`]
///   positive.
/// - **Curvature floor**: [`Tuning::trail_curvature_floor`]; below it the chord
///   estimate's noise (~5e-5 on Talon's Junction) cannot be told from a
///   straight, which `!target.is_finite()` wrongly assumed announces itself.
/// - **Exit**: `curvature < peak_curvature * `[`Tuning::trail_exit_decay`].
/// - **Saturation and deadband**: below [`Tuning::trail_saturation`] of full
///   lock the rate loop has its own authority; [`Tuning::trail_deadband`] keeps
///   this out of the small-signal regime `tests/closed_loop.rs` linearised
///   about.
///
/// No slew limiting: this is a *target* and `oag_physics::controls::update`
/// ramps the airbrake states at `Airbrake::gain`/`falloff`.
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
    // Positive `rate_error` wants to turn further right, and a nose-right yaw
    // needs `imbalance = L - R` negative, so the **right** side is braked.
    // `5ad69f3` shipped this backwards for months;
    // `the_differential_brakes_the_side_the_nose_is_turning_toward` pins it and
    // `oag_physics::airbrake`'s header settles the sign.
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
/// Runs every tick regardless of saturation, so a craft that saturates only
/// near the apex does not read as "exiting" for want of a mark.
///
/// # The mark leaks
///
/// "Straight" is [`Tuning::trail_curvature_floor`], and **on this disc's
/// geometry the line never goes straight by it**: on `07_Track`, lone Ace,
/// 6,000 ticks, the smallest windowed curvature is `0.00127` against `0.00100`,
/// so the reset fires **zero** times and the mark latches at `0.04566`. The
/// exit gate then meant "not the tightest corner seen this race" and rejected
/// **4,486** ticks (74.8 %); the differential fired on **227** (3.78 %).
/// Through `07`'s 2,150-2,199 hairpin steering sat at **full lock** with a rate
/// error of 0.22 rad/s against a windowed curvature of `0.0139` and a bound of
/// `0.0320`.
///
/// [`Tuning::trail_peak_decay`] makes the mark leaky; `1.0` is the old latch,
/// so the sweep includes the row it replaced.
pub(super) fn track_peak_curvature(curvature: f32, previous_peak: f32, tuning: &Tuning) -> f32 {
    if curvature <= tuning.trail_curvature_floor {
        curvature
    } else {
        curvature.max(previous_peak * tuning.trail_peak_decay)
    }
}

/// The two airbrake commands, from the symmetric brake and the differential.
///
/// The sides are an interval of width `differential` slid to sit as near the
/// symmetric brake as it will go, not the brake plus and minus half of it:
///
/// - **A craft braking flat out has no headroom above**: adding to one side
///   would clip at `1.0` and deliver nothing, exactly when needed.
/// - **The low side must stay strictly positive whenever the brake is on**:
///   both sides positive is what engages `ShipState::brake`. Below `floor` the
///   differential shrinks, never the brake.
///
/// With the brake off `floor` is zero and one side rises from nothing: yaw
/// authority, no deceleration.
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

#[cfg(test)]
mod tests {
    use super::hull_yaw_ceiling;
    use oag_physics::{Body, Handling};

    fn body(inertia: oag_core::math::Vec3) -> Body {
        Body {
            inertia,
            ..Body::default()
        }
    }

    /// A Pulse craft's ceiling is bit for bit the one Pulse's global `I_yy` gave before the
    /// craft's own inertia was read.
    #[test]
    fn a_pulse_craft_reads_pulses_ceiling_bit_for_bit() {
        let mut handling = Handling::default();
        for amount in [1.30f32, 1.42, 1.55, 1.68, 1.80] {
            handling.turning.amount = amount;
            let before = oag_physics::controls::CONTROL_RANGE * amount
                / (-oag_physics::passive::YAW_DAMPING
                    * (1.0 / oag_physics::forces::YAW_INVERSE_INERTIA));
            let seated = body(oag_physics::forces::ship_inertia_at(
                oag_physics::forces::INERTIA_MASS,
            ));
            assert_eq!(
                hull_yaw_ceiling(&handling, &seated).to_bits(),
                before.to_bits()
            );
            assert_eq!(
                hull_yaw_ceiling(&handling, &Body::default()).to_bits(),
                before.to_bits()
            );
        }
    }

    /// HD builds its inertia with mass `1.0`, so `I_yy` is 24 and its Feisar turns at `1.5`
    /// rad/s, not Pulse's `1.667`.
    #[test]
    fn an_hd_craft_reads_its_own_inertia() {
        let mut handling = Handling::default();
        handling.turning.amount = 1.80;
        let hd = hull_yaw_ceiling(&handling, &body(oag_physics::forces::ship_inertia_at(1.0)));
        assert!((hd - 1.5).abs() < 1e-5, "{hd}");
    }
}
