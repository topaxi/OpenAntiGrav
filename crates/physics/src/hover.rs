//! The two-probe air cushion, and the grounded-only terms that come with it.
//!
//! Specified in `docs/physics/README.md` at confidence **91** for the force law,
//! from static analysis that **has never been run**. Every number in this module
//! is transcribed from that page, including the parts that look wrong, because a
//! transcription can be corrected against a trace in M3 and an improvement
//! cannot.
//!
//! The three things most likely to be "fixed" by mistake, all of them deliberate:
//!
//! - **Stiffness carries no handling parameter.** It is
//!   `mass * 0.3 * (normal_gravity + track_gravity)`, calibrated against gravity.
//!   Ship-to-ship differences come from the gravity values, not from a spring
//!   constant, and there is no spring constant to look for.
//! - **`rebound` and `landing_rebound` scale damping only**, never the spring.
//!   Damping is a *multiplier* on the spring term rather than a summand, so at
//!   the height where the spring is zero the damping is zero too.
//! - **The reach is the spring's own target, and the target comes from
//!   `ride_height`.** `docs/physics/README.md` said `ride_height` "never appears
//!   in the force law"; `docs/ghidra/functions/psp-pulse-usa/engine.md` traced the
//!   offset chain from the parser through `craft+0x70` and `craft+0x2f0` into the
//!   spring, and `Ship_CastHoverProbes` ends its ray at
//!   `probe - up * craft+0x2f0`. So the probes reach exactly as far as the height
//!   they hold and the suspension is **compression-only**. See [`target_height`]
//!   and [`probe`].
//! - **The suspension carries two gravities, not one.** [`DOWNFORCE_SCALE`]
//!   presses the craft onto the surface with `track_gravity * mass * grounded`,
//!   which is exactly what the spring's `normal_gravity + track_gravity`
//!   calibration is calibrated for. Removing it does not merely lift the craft:
//!   because the damper multiplies the spring magnitude, it removes most of the
//!   craft's attitude damping with it.
//!
//! And one absence: **nothing here pitches the ship to the track**, and on a
//! magstrip that is [`crate::maglock`]'s job rather than a gap. Pitch and
//! roll response is entirely emergent from applying two forces at two points,
//! and the surface-alignment torque explicitly projects its pitch component out.
//! Adding an explicit pitch-to-track term would change the character of the whole
//! model.
//!
//! # How much travel the suspension has, which used to be an open question
//!
//! A probe in contact reports a height in `0..=target`, and the craft rests where
//! the two springs carry the load - gravity **plus** the downforce:
//!
//! ```text
//! 2 * 0.3 * HOVER_K * (normal_gravity + track_gravity) * compression
//!     = normal_gravity * classScale + track_gravity
//! ```
//!
//! `mass` cancels. The gravities **almost** cancel: they would exactly, and every
//! craft in the game would rest at `1.25`, if `classScale` were `1.0`. It is not -
//! it is `<GlobalClass><GravityMul airborne/>`, four different values for the four
//! speed classes, and this module's docs said `1.0` until that was decoded. With
//! the real scale the class the reference capture was taken in rests at `1.2390`,
//! against the original's own live `craft+0x308` reading of `2.8878` on a `4.125`
//! target - `1.237` compressed, which is `0.16 %` away where the `1.25` was
//! `1.05 %` away.
//!
//! So the resting point is *near* half the travel for every craft rather than
//! exactly half, and the symmetry that makes the recovered geometry work survives.
//! See `docs/physics/angular-velocity-column.md`.
//!
//! Two earlier readings of this same question are kept as recorded history,
//! because both were wrong in instructive ways. The first had the target equal to
//! the reach *and* `TARGET_GLOBAL_SCALE` at the identity, which left a resting
//! ship hard against the top of its range; that was settled by reading the scale
//! at `0.75`. The second computed the sag from `normal_gravity` alone - one probe
//! at that - and got `0.147` units, which made the recovered probe geometry look
//! refuted by the captures. It was the load that was wrong, not the geometry.
//!
use oag_core::math::Vec3;

use crate::barrel_roll;
use crate::collide::{Ray, Raycaster};
use crate::forces::Environment;
use crate::params::Handling;
use crate::ship::ShipState;

/// The global hover stiffness scale, `K`.
///
/// The static image holds this global as `1.0`, but **ship construction
/// overwrites it with `4/3` before any race runs**, so `1.0` is never the value
/// the force law sees.
///
/// **Runtime-verified**, which is the confirmation the previous note asked for:
/// `0x08ab0e20` reads `1.3333333730697632`, bit pattern `0x3faaaaab`, off a
/// breakpoint in `Ship_UpdateCraft` during a real Time Trial. Confidence **95**.
///
/// That measurement also **corrects the literal**. This was written as `1.333_3`,
/// with a note saying the original holds that decimal rather than `4.0 / 3.0`
/// "which is a different `f32`". The note had the right concern and the wrong way
/// round: `1.333_3` is `0x3faaa64c` and the game holds `0x3faaaaab`, so the exact
/// quotient is what the original has and the decimal was the approximation.
pub const HOVER_K: f32 = 4.0 / 3.0;

/// How long after touchdown `landing_rebound` replaces `rebound`, in seconds.
pub const LANDING_WINDOW: f32 = 0.2;

/// How much a full magstrip lock raises the hover target height.
///
/// `target = (...) * (1 + 0.2 * magLockBlend) * K2`.
pub const TARGET_MAG_LOCK_GAIN: f32 = 0.2;

/// The global scale on the hover target height, `K2` at `0x08ab0e1c`.
///
/// **`0.75`, and it is now read rather than guessed.** This was the identity, with
/// a note that its value had never been recovered and that the multiplication was
/// kept visible so somebody could fill it in. This is that reading, taken two
/// independent ways off Pulse running in PPSSPP during a Time Trial:
///
/// - The global itself. `0x08ab0e1c` holds `0.75`, bit pattern `0x3f400000`.
/// - The result it produces. At a breakpoint in `Ship_UpdateCraft`, the spring
///   target `craft+0x2f0` reads **4.125** while the handling block's
///   `ride_height` reads **5.5**, and `4.125 / 5.5 = 0.75` exactly, with
///   `craft+0x74` reading `0.0` so no additive offset is in play.
///
/// Confidence **92**: the global's address was already identified by static
/// analysis, both readings agree to the bit, and the arithmetic admits no other
/// factor. Not 95, because it was read on one ship in one class on one track and
/// nothing was traced across a magstrip or a leap, where the other two factors in
/// [`target_height`] would move.
///
/// # This is the headroom the suspension was missing
///
/// With the identity, the target equalled `ride_height`, which is *also* the probe
/// cast length, so the ship rested a spring-sag below the very top of its own reach:
/// 0.147 units of travel on the observed data, 2.7 % of the ride height. Any
/// disturbance past that dropped both probes. At `0.75` the target is `4.125`
/// against a `5.5` reach, so a resting ship sits about **1.5 units** below the point
/// where it loses the ground, which is ten times the margin and is what a suspension
/// is supposed to have.
///
/// That retires the open question this module recorded three candidate answers for.
/// It was **not** the additive `craft+0x74` offset (measured `0.0`) and **not** the
/// cast length (a longer cast measurably made things worse); it was this scale.
pub const TARGET_GLOBAL_SCALE: f32 = 0.75;

/// The largest reduction the weapon slowdown timer can make to the hover target.
///
/// `min(craft+0x2e0, 4.0)`, subtracted from the target while that timer runs,
/// and **unreachable on the shipped disc** - see [`crate::slowdown`].
pub const SLOWDOWN_ADJUST_MAX: f32 = 4.0;

/// The hover spring's target height.
///
/// ```text
/// target = (ride_height + offset - min(slowdownTimer, 4.0)) * (1 + 0.2 * magLockBlend) * K2
/// ```
///
/// **`ride_height` is the primary term.** `docs/physics/README.md` said it "never
/// appears in the force law"; `docs/ghidra/functions/psp-pulse-usa/engine.md` traced the
/// chain - the parser stores it at `+0x94`, `craft+0x70` points at it,
/// `Ship_UpdateCraft` builds `craft+0x2f0` from it, and `Ship_HoverTwoPoint` springs
/// against that - at confidence 88. It is still *also* the raycast length, so a ship
/// cannot probe further than the height it is trying to hold, and the two uses are
/// deliberately kept as one field rather than split.
///
/// **The additive `offset` is taken as zero, and that is a change from the first
/// reading.** The slot is `craft+0x74` and **nothing was found that writes it**.
/// [`crate::params::Pitch::antigrav_height_adjust`] was the obvious candidate by name
/// and was passed here originally, but a search for readers of that field in the craft
/// path found none - a weak negative, at confidence 50 for "parsed but never
/// consumed". It is no longer read here, for a reason that is arithmetic rather than
/// a re-reading of the binary:
///
/// **A positive offset leaves the model with no fixed point.** `ride_height` is also
/// the raycast length ([`probe`]), so the probes see ground only within `ride_height`
/// of the hull. With any `offset > 0` the target exceeds that reach, the spring is
/// pushing *up* at every height a probe can report, and there is no height at which
/// the ship rests while still in contact. Measured on the observed data - `ride_height`
/// 5.5 against an offset of 1.0, so a target of 6.5 - the ship at rest with no input
/// leaves the probe range every cycle, and the gravity term's grounded-to-airborne
/// step then pumps the bounce until it is thrown clear.
///
/// `docs/physics/README.md` records the same conclusion in its open-questions list
/// ("with the offset at zero the target equals the raycast length ... a negative offset
/// would give the probes headroom"). Zero is where that leaves it: **an unverified
/// guess is removed rather than a value invented**, and a negative offset is not
/// substituted for it, because nothing recovered says what one would be.
///
/// What this does *not* fix is recorded with it: at zero the target equals the reach
/// exactly, so the usable spring travel is only the sag needed to carry the ship's
/// weight,
///
/// ```text
/// sag = normal_gravity / (0.3 * HOVER_K * (normal_gravity + track_gravity))
/// ```
///
/// which on the observed data is **0.147 units, 2.7 % of `ride_height`** - because the
/// spring calibrates against `normal_gravity + track_gravity` while carrying only
/// `normal_gravity`. A disturbance larger than that still drops the probes. See
/// `crate::hover`'s module documentation and the M3 notes.
#[must_use]
pub fn target_height(handling: &Handling, mag_lock_blend: f32, slowdown_timer: f32) -> f32 {
    let slowdown_adjust = slowdown_timer.clamp(0.0, SLOWDOWN_ADJUST_MAX);
    let base = handling.antigrav.ride_height - slowdown_adjust;

    base * (1.0 + TARGET_MAG_LOCK_GAIN * mag_lock_blend) * TARGET_GLOBAL_SCALE
}

/// The probe height, in units, below which penetration escape teleports the body.
///
/// Not a tunable and not part of the spring, which reads `targetHeight - h` at
/// every height. See [`HoverProbe::escape`].
pub const PENETRATION_LIMIT: f32 = 1.0;

/// The surface-alignment torque gain, grounded only.
///
/// Applied as `ALIGNMENT_GAIN * cross(up, avgNormal)` with the right-axis
/// component projected out, so it levels roll and yaw but deliberately not pitch.
///
/// # Why this is `+400` where the page writes `-400`
///
/// `docs/physics/README.md` writes `-400 * cross(up, avgNormal)` and, in the same
/// sentence, describes a term "which levels roll and yaw". **Those two statements
/// contradict each other**, and the contradiction is settled by arithmetic rather
/// than by reading the prose more carefully - which is the lesson `HANDOVER.md`
/// draws from every other disagreement this project has had: look for the
/// relation that must hold.
///
/// Let `omega = up x n`. A torque along `omega` grows angular velocity along
/// `omega`, and the up axis then moves as
///
/// ```text
/// d(up)/dt = omega x up = (up x n) x up
///          = n * (up . up) - up * (n . up)      [ (a x b) x c = b(a.c) - a(b.c) ]
///          = n - up * (n . up)                  [ up is a unit vector ]
/// ```
///
/// which is exactly the component of `n` perpendicular to `up`: it points *from*
/// `up` *toward* `n`. So `+k * cross(up, n)` aligns and `-k * cross(up, n)`
/// diverges, and **no magnitude can change that**. Not one of the gain, the
/// inertia tensor, the probe offsets or the target height can flip the sign of
/// that derivation, whatever their values - which is the whole argument, and it
/// never depended on their being unknown. (This sentence used to call all four
/// unrecovered. Two of them since were: the divisor is the recovered box
/// inertia from `Body_SetBoxInertia`, and the target-height scale is the
/// recovered `0.75`.) The page's own prose asserts
/// levelling as part of a force law scored at confidence 91, so it is the literal
/// `-400` that has to be the transcription error: either the operands were
/// transposed when the page was written, `cross(avgNormal, up)`, or the sign was.
/// Behaviour is the stronger of the two claims, being what a trace would actually
/// show, where operand order is a detail of how somebody wrote down a decompiled
/// expression.
///
/// Simulation corroborates it. With the literal `-400`, a ship placed on a flat
/// floor with a 0.02 rad roll grows to a full tumble in under a second of
/// simulated time at 60 Hz - the signature of anti-alignment, not of a mistuned
/// gain.
///
/// **A third explanation has since become the leading one: handedness.**
/// [`crate::passive::WEATHERVANE_GROUND`] turned out to have exactly the same shape -
/// a negative coefficient on a cross product that must be positive for the behaviour
/// the same evidence page describes - and two independent transcription errors of one
/// shape is a poor explanation where a systematic frame difference is a good one.
/// `docs/ghidra/functions/psp-pulse-usa/engine.md` lists handedness as not determined. If
/// that is what this is, the original's expression is correct as written *in its own
/// frame*, nothing is a typo, and this crate is simply obliged to flip every cross
/// product it inherits from that path. Which is a prediction, and a cheap one to
/// check against the next such term recovered.
///
/// # This gain makes the ship unstable, and the margin is 2.2x rather than 11 %
///
/// Roll under this term is a damped oscillator with stiffness `k = 400` here and
/// damping `c = 2` from [`crate::passive::ROLL_DAMPING`]. The margin by which the
/// integrator fails to hold it was previously recorded as 11 %, from `h <= c/k =
/// 0.005` against a `1/180` sub-step. **That reading is superseded**, because it
/// assumes the force is re-evaluated every sub-step and
/// [`crate::integrate`] does not do that: the angular acceleration is computed once
/// from the frame's starting state and held across all three sub-steps.
///
/// Propagating one frame of the scheme the integrator actually runs - `a` fixed,
/// then three sub-steps of `h = H/3` - gives
///
/// ```text
/// theta'  = theta + H*theta_dot + (H^2/3)*a
/// dtheta' = theta_dot + H*a                       a = -k*theta - c*theta_dot
///
/// det = (1 - k*H^2/3)(1 - c*H) + k*H^2 - c*k*H^3/3
/// ```
///
/// so the governing step is the **frame** `H = 1/60`, not the sub-step, and the
/// stability condition is
///
/// ```text
/// c >= (2/3) * k * H     <=>     H <= 3c / (2k) = 0.0075 s
/// ```
///
/// At `k = 400`, `c = 2` and `H = 1/60` that needs `c >= 4.444` and it has `2`: the
/// frame is **2.22x** too large, and `det = 1.040741`, so roll grows by
/// `sqrt(det) = 1.020167` per tick. **Measured, not just derived**: a ship at rest on
/// a flat floor with an initial roll of 0.01 rad grows its peaks at 1.018 to 1.022 per
/// tick over 200 ticks, with a period of 19 ticks against the `sqrt(400) = 20 rad/s`
/// the gain predicts.
///
/// That measurement corroborates a second finding independently. This torque reaches
/// the body through [`crate::forces::drain`], which multiplies by the inertia tensor
/// that [`crate::integrate`] then divides out, so it acts as an angular *acceleration*
/// and never sees the inertia. Roll inertia for a real hull is about 3.1, so a torque
/// that *were* divided by it would give `k_eff = 129` and a visibly different growth
/// rate. The measured rate matches the undivided `k = 400`, which is the round trip
/// confirmed from the outside.
///
/// **The consequence is sharper than the old reading's.** An 11 % margin is
/// consistent with an original that is marginally unstable and never sits in that
/// state; a 2.22 x margin is not - a ship on the grid would visibly tumble. So one of
/// `k = 400`, `c = 2`, the sub-step count, or the once-per-frame evaluation is
/// misread, and a trace distinguishes them. **Nothing here is tuned to hide it**;
/// see `HANDOVER.md`'s note that changing a magnitude would destroy the measurement.
///
/// ## The original does not do this, and that is now measured rather than assumed
///
/// The question "is the original marginally unstable and simply never sits in that
/// state" has an answer, and it is **no**. Pulse was driven into a Time Trial under
/// PPSSPP and a 150-tick trace taken with **no input at all**, through
/// `scripts/psp-trace.py`:
///
/// | Quantity | Original, 150 ticks at rest |
/// | --- | --- |
/// | `grounded` | `1.0` on every tick, no flicker |
/// | distance travelled | 0.0087 units |
/// | `pos_y` range | 0.00019 units |
/// | up-axis deviation from its own mean | max **0.000476 rad** (0.027 deg) |
///
/// A ship sitting on a real, banked track surface, where this torque is certainly
/// non-zero, holds its attitude to within half a milliradian and shows **no
/// oscillation at all** - let alone one growing 2 % a tick at the 20 rad/s this gain
/// implies. So the instability is an artefact of the reading, not a property of the
/// game, and the four candidates above are now a search for *which* is wrong rather
/// than a question of whether any is.
///
/// **The gain itself was hunted for and not found.** The hover constant pool around
/// `0x08ab0e00` holds [`TARGET_GLOBAL_SCALE`], [`HOVER_K`] and
/// [`BANK_TO_YAW_GAIN`] - `0x08ab1098` reads `30.0` - but contains no `400.0`, and no
/// `-2.0`/`-5.0` pair for the damping. Whatever this term's real magnitudes are, they
/// live elsewhere. That is the next measurement, and it is the last thing between the
/// simulation and a ship that stays on a track.
///
/// # The literal is 400 and confirmed twice; the behaviour it produces is not
///
/// `docs/physics/README.md` transcribes this gain as `400`, and everything above
/// settles only its **sign**. The magnitude is now confirmed as well, at instruction
/// level and in a **second binary**: the PS2 build materialises `0xC3C80000`, exactly
/// `-400.0f`, as an immediate in both `Ship_HoverFourCorner` (`0x0015a940`) and
/// `Ship_HoverTwoPoint` (`0x0015b978`), with the same sign and operand order the PSP
/// reading has. **So `400` is not a transcription error and this constant keeps it.**
///
/// What is wrong is the behaviour `400` produces when it reaches the body as a bare
/// angular acceleration, and that was measured directly, by
/// perturbing the original and watching it recover - a step response, which is the
/// standard way to identify a second-order system and needs no disassembly at all.
///
/// Pulse was driven into a Time Trial under PPSSPP, broken in `Ship_UpdateCraft`, and
/// the rigid body's basis rows were **rewritten** through `memory.write_u32` to roll
/// the ship by `0.25 rad` about its own forward axis. The recovery was then traced per
/// frame. Note the craft update runs once per ship, so the breakpoint fires about
/// eight times a frame on a full grid; the distinct values are the frames.
///
/// ```text
/// roll, rad, from the 0.25 perturbation, one value per frame:
/// 0.2500 0.2493 0.2466 0.2424 0.2367 0.2295 0.2210 0.2113 0.2004 0.1886
/// 0.1758 0.1621 0.1476 0.1329 0.1175 0.1014 0.0855 0.0693 0.0531 0.0369
/// 0.0210 0.0055 -0.0097 -0.0244 -0.0384 -0.0518 -0.0646 -0.0762 ...
/// ```
///
/// It is a clean damped cosine: velocity starts at zero, the curve is flat at the top,
/// it crosses zero at **frame 21** and overshoots. So
///
/// ```text
/// quarter period = 21 frames at 60 Hz  ->  T = 1.4 s  ->  omega = 4.49 rad/s
/// stiffness  = omega^2                                 ~= 20.2
/// envelope decay over 0.583 s gives zeta               ~= 0.18
/// damping    = 2 * zeta * omega                        ~= 1.6
/// ```
///
/// Two things fall out. The damping is **about 1.6 against the transcribed `2.0`** in
/// [`crate::passive::ROLL_DAMPING`] - itself confirmed at `-2.0` in the PS2 build's
/// `Ship_ApplyAngularDamping` (`0x0015c1b0`) for racing modes - so that constant is
/// right and is kept. The closed-loop stiffness is **about 20 against a literal 400**,
/// a factor of twenty, and that factor is what made this crate's ship tumble: at `400`
/// the oscillator runs at `sqrt(400) = 20 rad/s` and the stability bound below is
/// violated 2.22-fold, while at `20.2` it runs at 4.49 rad/s, `det = 0.9704`, and roll
/// **decays** about 1.5 % a tick instead of growing 2 %. The factor is carried by
/// [`ALIGNMENT_INERTIA`], which exists precisely so the literal and the measurement can
/// both be stated without either being edited to suit the other.
///
/// **What the factor of twenty actually is remains open**; see [`ALIGNMENT_INERTIA`]
/// for the hypothesis, the candidate mechanism and what would retire it.
///
/// One thing the measurement does settle, and it is worth stating because a second
/// sign anomaly has since been found in `Body_AddForceAtPoint` (torque as `F x r`
/// rather than `r x F`, in both binaries): **this crate's failure mode is growth, not
/// anti-alignment.** A term with the wrong sign diverges from the first tick; what was
/// measured here is a ship holding still for about three hundred ticks and then
/// growing an oscillation exponentially from numerical noise, and
/// `the_alignment_torque_points_from_the_ships_up_axis_toward_the_surface_normal`
/// pins the direction as aligning independently of any simulation. So whatever the
/// `F x r` anomaly turns out to be, it is not what threw the ship off this track.
///
/// # The projection is re-read and confirmed, at 92
///
/// The whole term was re-read instruction by instruction to answer whether the
/// right-axis projection is real or a transcription artefact hiding a
/// pitch-restoring component: `cross(up, avgNormal)` at `0x0884ac44`, the
/// literal `-400.0f` materialised at `0x0884ac5c`, a `vdot.t`/`vscl.q`/`vsub.q`
/// triple against `craft+0x170` (the **right** axis) at
/// `0x0884acac`-`0x0884acfc`, and `vadd.t` into `craft+0x350`, the world
/// accumulator, at `0x0884ad24`. **Every element of what this module implements
/// is what the binary has**, at confidence 92, so the crate's under-damped pitch
/// is not this term and no change to it can be the fix. See
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The alignment torque: the
/// projection is real", and `docs/physics/angular-velocity-column.md` for where
/// the ringing actually comes from.
///
/// Confidence **80** on the step response: it is unambiguous and reproducible, but was
/// taken on one ship on one track and fitted by hand from a quarter period and one
/// overshoot. Confidence **84** on the direction, unchanged:
/// the arithmetic is compulsory, but the decompilation as recorded is
/// self-contradictory, no call site was re-read, and nothing is runtime-verified.
/// See the resolved-contradiction note in `docs/physics/README.md` for what would
/// raise it.
pub const ALIGNMENT_GAIN: f32 = 400.0;

// `ALIGNMENT_INERTIA`, a fitted `19.8`, used to divide the gain here.
//
// Its own documentation named the condition under which it should disappear: "if
// the original divides this torque by a real inertia tensor, this constant is that
// tensor's roll component and should be replaced by `Body::inertia` - which today
// it cannot be, because `forces::drain` multiplies the angular accumulators by the
// inertia that `integrate` divides out, so the term never sees it. That round trip
// is the thing to change, and this constant disappears when somebody does."
//
// The round trip is gone - the accumulators hold torque now - so the constant is
// gone with it, and the divisor is the recovered tensor. Two things worth keeping
// from the arithmetic it recorded, because they are a check on the replacement
// rather than a repetition of it:
//
// - The alignment torque has its pitch component projected out below, so what it
//   acts on is roll and yaw: `I_zz = 15.6` and `I_yy = 21.6`. The fitted `19.8`
//   sits **between** them, which is what a term spread across two axes should
//   look like and is not something the fit could have known.
// - On the roll axis alone the replacement is `400 / 15.6 = 25.6` against the
//   measured closed-loop stiffness of `20.2`, i.e. a natural frequency of `5.06`
//   rad/s against the original's measured `4.49`. **That is a 13 % discrepancy
//   and it is left standing rather than tuned away**: a recovered tensor that is
//   13 % out on one axis is worth more than a fitted scalar that is exact on it,
//   and the residual is a real open question about how the two axes share the
//   term. See `docs/physics/angular-velocity-column.md`.

/// The bank-to-yaw coupling gain, grounded only.
///
/// `angularLocal.y += 30 * right.y * (1 - magLockBlend)`. The magstrip factor comes
/// from `docs/ghidra/functions/psp-pulse-usa/engine.md`'s per-component enumeration of
/// the hover epilogue's writes; `docs/physics/README.md` records the term without it.
pub const BANK_TO_YAW_GAIN: f32 = 30.0;

/// The grounded downforce, `-track_gravity * mass * grounded * (1 - magLockBlend)`
/// along the averaged contact normal.
///
/// # This used to be a shape with a deliberately zero coefficient, and the
/// magnitude was recorded all along
///
/// The old text here said `docs/physics/README.md` "does not record its
/// magnitude", so the term was implemented as `spring_along_up * 0.0` - a shape
/// that scaled with the spring rather than with the load, multiplied by nothing.
/// **Both halves were wrong**, and the correction is a read, not a fit.
/// `Ship_HoverTwoPoint`'s epilogue, `0x0884ad78`-`0x0884ae18`:
///
/// ```text
/// 0884ad78  a0 = craft+0x1cc                 ; the rigid body
/// 0884ad7c  f12 = body+0x374                 ; mass, the field gravity also reads
/// 0884ad80  a0 = craft+0x70                  ; the handling block
/// 0884ad84  f13 = [a0+0x6c]                  ; track_gravity
/// 0884ad88  mul.s f12,f13,f12                ; track_gravity * mass
/// 0884ad8c  f14 = craft+0x2b0                ; THIS frame's grounded fraction
/// 0884ad90  mul.s f12,f12,f14
/// 0884ad94  neg.s f12,f12                    ; the sign is here, not at the use site
/// 0884ada4  C500 = craft+0x140               ; the averaged contact normal
/// 0884ada8  vscl.q C300,C500,S400            ; * that scalar
/// 0884add0  f12 = craft+0x280                ; the magstrip blend
/// 0884add4  sub.s f12,f20,f12                ; 1 - blend
/// 0884adec  vscl.q C300,C500,S400            ; * (1 - blend)
/// 0884ae08  a1 = craft+0x330                 ; the WORLD force accumulator
/// 0884ae14  vadd.t C300,C300,C600            ; += it
/// ```
///
/// Confidence **92** on the PSP instructions, and it has a second binary: the PS2
/// build's four-corner twin computes `-track_gravity * mass` along the same
/// averaged normal through a different base pointer
/// ([ps2-pulse-eu/craft-update.md](../../../docs/ghidra/functions/ps2-pulse-eu/craft-update.md)).
/// So the coefficient is `1.0` - there is no scale - and this constant exists
/// only to keep the name the rest of the tree refers to.
///
/// # It is the missing pitch damping, and that is arithmetic rather than a hope
///
/// The spring's damper is a **multiplier on the spring magnitude**
/// (`1 + rebound * clamp(-0.1 * vn, -1, 2)`), so the rate feedback it produces is
/// `0.1 * rebound * S`, proportional to the spring force `S` the craft is
/// *already* carrying. Without this downforce the two probes carry only
/// `normal_gravity * mass`; with it they carry `(normal_gravity + track_gravity) *
/// mass`, and on the shipped values that is a factor of **17**. The resting
/// compression the spring settles at,
///
/// ```text
/// compression = (normal_gravity * classScale + track_gravity)
///             / (0.8 * (normal_gravity + track_gravity))
/// ```
///
/// (`0.8` is two probes times `0.3 * HOVER_K`), is what the live probe read on
/// the start line measured at **1.237** - a number that made no sense while the
/// downforce was zero, and which is why the recovered probe geometry looked
/// refuted.
///
/// **`classScale` is not `1.0`**, which this doc comment claimed until
/// `g_class_gravity_scale` (`0x08ab0dcc`) was decoded out of
/// `<GlobalClass><GravityMul airborne/>`. With the real value the prediction is
/// `1.2390` rather than `1.25`, i.e. `0.16 %` from the measurement instead of
/// `1.05 %`. See [`probe_offsets`] and
/// `docs/physics/angular-velocity-column.md`.
pub const DOWNFORCE_SCALE: f32 = 1.0;

/// What one probe found and what it contributed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoverProbe {
    /// Where the probe sits, in world space.
    pub point: Vec3,
    /// The force to apply at [`Self::point`], already including damping and the
    /// magstrip blend. Zero when the probe found no hoverable surface.
    pub force: Vec3,
    /// Whether the probe found a [hoverable](crate::collide::Surface::is_hoverable)
    /// surface within `ride_height`.
    pub contact: bool,
    /// The hit surface's normal, or [`Vec3::ZERO`] without contact.
    pub normal: Vec3,
    /// `dot(probeWorld - rayHit, up)`: how far the probe sits above the surface
    /// measured along the ship's own up axis, not along the surface normal.
    pub height: f32,
    /// Position correction for the penetration-escape constraint, or
    /// [`Vec3::ZERO`].
    ///
    /// When `h < 1.0` the original **teleports** the body by `up * (1 - h)` with
    /// no velocity change. That is penetration escape for the last unit only; it
    /// is not the hover mechanism and must not be mistaken for one.
    ///
    /// **On `Floor` only** - narrower than contact, which also accepts
    /// `MagFloor`. The original's gate is `craft+0x208 == 1`, the `Floor`
    /// class, and it is applied before the collider's type is looked at, so
    /// this can be set on a probe that reports no contact.
    pub escape: Vec3,
}

impl HoverProbe {
    /// A probe that found nothing.
    fn miss(point: Vec3) -> Self {
        Self {
            point,
            force: Vec3::ZERO,
            contact: false,
            normal: Vec3::ZERO,
            height: 0.0,
            escape: Vec3::ZERO,
        }
    }
}

/// Everything the air cushion produced this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hover {
    /// The two probes, front first.
    pub probes: [HoverProbe; 2],
    /// How many probes are in contact, `0..=2`.
    pub contacts: u32,
    /// Mean of the normals under the probes in contact, normalised, or
    /// [`Vec3::ZERO`] when none are.
    pub average_normal: Vec3,
    /// The grounded-only surface-alignment torque, world space, pitch projected
    /// out.
    pub alignment_torque: Vec3,
    /// The grounded-only downforce along the ground normal, world space.
    pub downforce: Vec3,
    /// The grounded-only bank-to-yaw coupling, as a body-local angular
    /// acceleration.
    pub local_angular_torque: Vec3,
    /// The penetration-escape position correction for the whole body.
    pub escape: Vec3,
}

impl Hover {
    /// A frame with both probes in the air.
    fn airborne(probes: [HoverProbe; 2]) -> Self {
        Self {
            probes,
            contacts: 0,
            average_normal: Vec3::ZERO,
            alignment_torque: Vec3::ZERO,
            downforce: Vec3::ZERO,
            local_angular_torque: Vec3::ZERO,
            escape: Vec3::ZERO,
        }
    }
}

/// How far below the centre of mass the two probes hang, before
/// [`TARGET_GLOBAL_SCALE`].
///
/// `Ship_InitCraft` (`0x08849354`) writes the immediate `0xbfc00000` into the `y`
/// lane of both offsets at `0x08849480`-`0x088494b4`.
pub const PROBE_DROP_RAW: f32 = 1.5;

/// How far fore and aft the two probes sit, before [`TARGET_GLOBAL_SCALE`].
///
/// The `z` immediates from the same two writes, `0x40c00000` and `0xc0c00000`.
pub const PROBE_HALF_SPACING_RAW: f32 = 6.0;

/// Where the two probes sit in body space, `(0, -1.125, -/+4.5)`, front first.
///
/// **A code literal, identical for every craft in the game.** `Ship_InitCraft`
/// (`0x08849354`) writes `(0, -1.5, +/-6)` as immediates and scales both by the
/// same `0.75` global that [`TARGET_GLOBAL_SCALE`] is, in the ship-entity
/// constructor that calls it. Confidence 92; see
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The probe geometry is a code
/// literal, like the inertia tensor". No handling parameter enters it, so the
/// old `<Misc length>`-derived `+/-6.5` was deriving the spacing from the wrong
/// thing whatever factor it picked - the hull dimensions go to the *collider*,
/// exactly as they do for the inertia tensor.
///
/// # Why this could not be landed before the downforce, and why it can now
///
/// Two separate refutations used to stand against this placement, and
/// [`DOWNFORCE_SCALE`] dissolved both at once rather than either being wrong on
/// its own terms:
///
/// - **"Landing the spacing makes pitch worse."** True while the suspension
///   carried only `normal_gravity`: damping through the probes falls with `d^2`
///   and the crate was already `3.4x` short, so shortening the arm took the
///   pitch-rate error from `134 %` to `1005 %`. With the downforce the spring
///   carries seventeen times the load and the damper - a *multiplier* on the
///   spring - scales with it, so the shorter arm now lands on the original's own
///   step response instead of under it.
/// - **"The `-1.125` drop and a reach equal to the target are refuted by the
///   capture, which holds `grounded` at 1.0."** That refutation computed the
///   resting probe height with the load wrong: a craft carrying `normal_gravity`
///   alone rests `0.147` into a `4.125` reach and any disturbance drops it,
///   while a craft carrying `normal_gravity + track_gravity` rests **`1.25`**
///   into it and has that much travel in both directions. The live read on the
///   start line measured the probe distance at `2.888` against a `4.125` target -
///   `1.237` compressed, agreeing with the equilibrium this model now has to
///   `1 %`.
///
/// The `+/-4.5` half-spacing also has an independent runtime leg that never
/// depended on either: the pitch oscillator's own frequency. The original's
/// `omega_n` measures `9.32-9.72` rad/s and two probes at `+/-4.5` on the
/// recovered tensor predict `sqrt(2 * 0.4 * (ng + tg) * 4.5^2 / 15.6)`, which is
/// `9.4`. Nothing in that prediction was fitted.
#[must_use]
pub fn probe_offsets() -> [Vec3; 2] {
    let drop = PROBE_DROP_RAW * TARGET_GLOBAL_SCALE;
    let half = PROBE_HALF_SPACING_RAW * TARGET_GLOBAL_SCALE;

    // Body forward is -Z; see `Body::forward`. The front probe is first.
    [Vec3::new(0.0, -drop, -half), Vec3::new(0.0, -drop, half)]
}

/// The damping multiplier's rebound coefficient for this frame.
///
/// ```text
/// reb = timeSinceLanding >= 0.2
///         ? rebound
///         : 0.5 * (rebound * t + landing_rebound * (1 - 5*t))
/// ```
///
/// **This is transcribed and it is internally odd.** Taking `t` as
/// `time_since_landing` in seconds, the branch is discontinuous at `t = 0.2`,
/// where the blend evaluates to `0.1 * rebound` and the other arm to `rebound`,
/// and `rebound * t` mixes a coefficient with a time. `1 - 5*t` does reach zero
/// exactly at the window's end, which is the one part that reads as intended, so
/// the most likely explanation is a factor lost when the expression was written
/// down rather than in the original. Recorded, kept literal, flagged for M3.
#[must_use]
pub fn rebound_coefficient(handling: &Handling, time_since_landing: f32) -> f32 {
    if time_since_landing >= LANDING_WINDOW {
        handling.antigrav.rebound
    } else {
        let t = time_since_landing;
        0.5 * (handling.antigrav.rebound * t + handling.antigrav.landing_rebound * (1.0 - 5.0 * t))
    }
}

/// Runs one probe.
///
/// `target_height` is the height the spring pulls toward, from [`target_height`]. It
/// stays a parameter rather than a lookup because one term of it, the additive
/// offset, is still a guess.
///
/// `grounded_prev` is last frame's groundedness, which the spring scales by
/// `0.75 * grounded_prev + 0.25`. Last frame's, not this frame's: the spring is
/// evaluated before the new contact count exists.
#[must_use]
pub fn probe<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    local_offset: Vec3,
    target_height: f32,
) -> HoverProbe {
    let body = &state.body;
    let up = body.up();
    let point = body.position + body.orientation * local_offset;

    // Cast along the ship's **own** up axis rather than world gravity. This is
    // the whole reason magstrips and inversions work at all.
    //
    // The reach is the spring's own target, not `ride_height`:
    // `Ship_CastHoverProbes` builds the ray's far end at `0x0884a0b4` as
    // `probe - up * craft+0x2f0`, the same field `Ship_HoverTwoPoint` springs
    // against. A probe therefore reaches exactly as far as the height it holds,
    // so the spring is compression-only - which is consistent with the capture
    // precisely because [`DOWNFORCE_SCALE`] rests the craft `1.25` units into
    // that reach rather than `0.147`.
    let ray = Ray::new(point, -up, target_height);

    let Some(hit) = raycaster.raycast(ray, env.self_collider, false) else {
        return HoverProbe::miss(point);
    };
    probe_from_hit(state, handling, point, &hit, target_height)
}

/// The half of [`probe`] that turns a hit into a force, without casting.
///
/// Split out for [`derived_hit`], which above [`FAST_PROBE_SPEED`] produces the
/// rear probe's hit arithmetically rather than by casting - see there for why.
#[must_use]
fn probe_from_hit(
    state: &ShipState,
    handling: &Handling,
    point: Vec3,
    hit: &crate::collide::RaycastHit,
    target_height: f32,
) -> HoverProbe {
    let body = &state.body;
    let up = body.up();

    let height = (point - hit.point).dot(up);

    // **Penetration escape happens before the surface check, and therefore on
    // any surface at all.** `Ship_HoverTwoPoint` computes `craft+0x308`, escapes
    // on it, and *then* asks whether the collider's type is 1 or 3 - so a probe
    // a hand's breadth inside a `Wall` is pushed out of it just as a probe
    // inside a `Floor` is, even though no spring force follows. Reading it the
    // other way round, which this crate did until 2026-08-12, leaves a craft
    // free to sink into wall geometry the original would extrude it from.
    //
    // The gate on it is `craft+0x208 == 1`, and `1` is the **`Floor` class** -
    // read that way independently by an earlier pass, which recorded it on
    // `docs/physics/README.md` ("when `h < 1.0` on a floor surface") and as an
    // open divergence in `HANDOVER.md`, since this module escaped on `Floor` or
    // `MagFloor`. So the escape is narrower than contact, not wider: a magstrip
    // holds a craft kinematically and has no use for a hard constraint, and a
    // wall is `crate::wall`'s business.
    let escape = if hit.surface == crate::collide::Surface::Floor && height < PENETRATION_LIMIT {
        up * (PENETRATION_LIMIT - height)
    } else {
        Vec3::ZERO
    };

    // Contact - and so the spring - is accepted only for surface types 1
    // (Floor) and 3 (Mag Floor).
    if !hit.surface.is_hoverable() {
        return HoverProbe {
            escape,
            ..HoverProbe::miss(point)
        };
    }

    // The page writes this as `bodyLinearVel + M * cross(probeLocal, angularVel)`,
    // which reads like the negation of the conventional rigid-body point
    // velocity. **It is not, and that is now settled rather than assumed.** The
    // original's `vcrsp.t` at `0x0884a970` crosses the local probe offset with
    // `body+0x150`, which is `-omega` under the `w_game = -w_physics`
    // convention, so `basis * (r x -omega)` is `basis * (omega x r)` - the
    // conventional form, which is what this line computes. The recorded sign and
    // the recorded frame cancel. See
    // `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The damper's point velocity
    // is the conventional `omega x r`".
    let velocity = body.velocity_at(point);
    let normal_velocity = velocity.dot(hit.normal);

    let gravity = handling.physical.normal_gravity + handling.physical.track_gravity;
    let load = 0.75 * state.grounded_prev + 0.25;
    // Grouped left to right, deliberately: the operation order is part of the
    // result. No `mul_add` here or anywhere.
    let spring = handling.physical.mass * 0.3 * (target_height - height) * HOVER_K * load * gravity;

    let damping = (-0.1 * normal_velocity).clamp(-1.0, 2.0);
    let reb = rebound_coefficient(handling, state.time_since_landing);
    let rebound = barrel_roll::rebound_override(state, reb);

    // The magstrip blend fades the ordinary suspension out for `crate::maglock`'s
    // kinematic hold, ramped by `maglock::ramp` from the mag-floor probe and read
    // here rather than made up for.
    let force = up * spring * (1.0 + rebound * damping) * (1.0 - state.mag_lock_blend);

    HoverProbe {
        point,
        force,
        contact: true,
        normal: hit.normal,
        height,
        escape,
    }
}

/// Above this forward speed the rear probe is **derived rather than cast**.
///
/// `Ship_CastHoverProbes` branches on `craft+0x2ec <= 50.0` at its head, and
/// `craft+0x2ec` is the cached `|dot(velocity, forward)|` of the previous frame,
/// recovered at 199/199 against a capture; see
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`. Below the threshold it casts
/// two independent rays. Above it, it casts **one**, and if that ray hits, it
/// manufactures the other probe's hit record from it.
pub const FAST_PROBE_SPEED: f32 = 50.0;

/// How far the derived hit is pushed along `up` per unit of forward-facing
/// surface normal, `vscl.q` by `0x40c00000` at the tail of the fast path.
///
/// A slope-following correction with no derivation behind it: the exact answer
/// over a 9-unit probe spacing would be `-9 * dot(n, forward) / dot(n, up)`,
/// and the original uses a flat `6.0`. Transcribed, not improved.
pub const DERIVED_HIT_SLOPE_GAIN: f32 = 6.0;

/// The rear probe's hit, manufactured from the front probe's.
///
/// **This is the mechanism that keeps a craft attached over a crest**, and it is
/// why the original does not shed half its suspension the moment one probe
/// overruns a lip. Above [`FAST_PROBE_SPEED`] the second ray is never cast: the
/// first probe's whole hit record is copied, its point translated by the
/// world-space vector between the two probes, and then pushed along `up` by
/// `dot(normal, forward) * 6.0` to follow the slope. The hit *flag* is copied
/// too, so a front probe in contact guarantees a rear probe in contact and
/// `grounded` cannot read `0.5` at speed.
///
/// # Which probe is cast, settled by measurement
///
/// `Ship_InitCraft` writes the offsets at `0x08849480`: probe 0 is
/// `(0, -1.5, +6.0)` and probe 1 is `(0, -1.5, -6.0)`, raw, before the `0.75`
/// this module folds in. Whether `+6.0` is the *nose* depends on whether the
/// original's craft basis faces its local `+Z`, which this crate mirrors - and
/// nothing recovered settles it, so it was **measured instead**: casting
/// `offsets[1]` first and deriving `offsets[0]` takes the twelve-circuit solo
/// benchmark from nine clean laps to **two**, with respawn counts of twenty to
/// fifty a circuit against nought to seven. The order below is the right way
/// round by a very wide margin.
///
/// # Why it is also a downforce
///
/// Worth seeing, because it is not obvious from the shape: the derived hit is
/// *not* clamped by a ray, so on a falling slope it can sit further below the
/// probe than `target_height`. The spring is `(target_height - height)`, so a
/// derived probe past its target pulls the craft **down** onto the surface
/// rather than pushing it off. That is the only term in the model that does,
/// and it exists only above [`FAST_PROBE_SPEED`].
#[must_use]
fn derived_hit(
    state: &ShipState,
    front_point: Vec3,
    rear_point: Vec3,
    front: &crate::collide::RaycastHit,
) -> crate::collide::RaycastHit {
    let up = state.body.up();
    let forward = state.body.forward();
    let slope = front.normal.dot(forward);
    crate::collide::RaycastHit {
        point: front.point + (rear_point - front_point) + up * slope * DERIVED_HIT_SLOPE_GAIN,
        ..*front
    }
}

/// Casts the pair, taking whichever of the two paths the speed selects.
#[must_use]
fn probe_pair<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    offsets: [Vec3; 2],
    target_height: f32,
) -> [HoverProbe; 2] {
    let body = &state.body;
    let speed = body.linear_velocity.dot(body.forward()).abs();
    if speed <= FAST_PROBE_SPEED {
        return [
            probe(state, handling, env, raycaster, offsets[0], target_height),
            probe(state, handling, env, raycaster, offsets[1], target_height),
        ];
    }

    let up = body.up();
    let front_point = body.position + body.orientation * offsets[0];
    let rear_point = body.position + body.orientation * offsets[1];
    let Some(front) = raycaster.raycast(
        Ray::new(front_point, -up, target_height),
        env.self_collider,
        false,
    ) else {
        // The front ray found nothing, so the original casts the rear one for
        // real - the derivation has nothing to derive from.
        return [
            HoverProbe::miss(front_point),
            probe(state, handling, env, raycaster, offsets[1], target_height),
        ];
    };
    let rear = derived_hit(state, front_point, rear_point, &front);
    [
        probe_from_hit(state, handling, front_point, &front, target_height),
        probe_from_hit(state, handling, rear_point, &rear, target_height),
    ]
}

/// Catches a hull that crossed a hoverable surface between one tick and the
/// next, and puts it back on top of it.
///
/// # This one is ours
///
/// **Nothing in the original does this in this form** (its analogue, pass 1,
/// is in `crate::wall`'s docs); the original's contact test - and so ours - can only
/// ever look *one way*. [`probe`] casts down from the hull along `-up` for
/// `target_height`. Once the surface is above the hull no cast of any length
/// finds it, penetration escape has no downward hit to measure, and the craft
/// is gone for good.
///
/// That is not hypothetical. Instrumented on Fort Gale (`14_Track`), a craft
/// rides a descent at 139 units/s, which at that gradient is **76 units/s
/// straight down while still in contact**. The ramp ends; on the very first
/// airborne tick the craft is already 2.0 units *below* the surface with
/// nothing beneath it at all, and from there it is ballistic - 119 units/s
/// down after a second, 160 after two, with the track it left receding
/// overhead. A player driving the same section by hand falls through it too, so
/// it is not an opponent's line that is wrong. See `docs/gameplay/ai.md`.
///
/// A longer ray cannot fix a one-sided test, and a faster contact rate only
/// moves the speed at which it fails. What fixes it is asking the question the
/// discrete test cannot: *did the hull pass through the surface during this
/// tick?* So this sweeps each probe's own motion segment - where it was before
/// the integrator ran, to where it is now - and if that segment crosses a
/// hoverable face from the front, the craft is placed back on the surface with
/// its inward velocity removed. That is an ordinary continuous-collision test,
/// deliberately chosen over reproducing something the original does not have.
///
/// Two properties that keep it from changing anything it should not:
///
/// - It runs **only when both probes found nothing**, so a craft in normal
///   contact never reaches it and the recovered force law is untouched.
/// - It needs a surface *on the segment*. An authored jump is a gap with
///   nothing to cross, so a craft flying one is unaffected - `13_Track`'s jump
///   still flies and still lands on its own.
///
/// Returns the position correction and the new velocity, or `None`.
#[must_use]
pub fn sweep<R: Raycaster + ?Sized>(
    state: &ShipState,
    env: &Environment,
    raycaster: &R,
    before: Vec3,
) -> Option<(Vec3, Vec3)> {
    let body = &state.body;
    let travel = body.position - before;
    let distance = travel.length();
    if distance <= 0.0 {
        return None;
    }
    let direction = travel / distance;
    let up = body.up();

    // The earliest crossing of the two probes' segments, in probe order so a
    // tie cannot depend on anything but the offsets.
    let mut best: Option<(f32, Vec3, Vec3)> = None;
    for offset in probe_offsets() {
        let arm = body.orientation * offset;
        let from = before + arm;
        let Some(hit) = raycaster.raycast(
            Ray::new(from, direction, distance),
            env.self_collider,
            false,
        ) else {
            continue;
        };
        if !hit.surface.is_hoverable() {
            continue;
        }
        // Only a face the craft is moving *into*. Leaving a surface from
        // underneath - which a craft on the inside of a loop does every tick -
        // must not be caught.
        if travel.dot(hit.normal) >= 0.0 {
            continue;
        }
        if best.is_none_or(|(nearest, _, _)| hit.distance < nearest) {
            best = Some((hit.distance, hit.point, hit.normal));
        }
    }
    let (_, point, normal) = best?;

    // Put the probe that crossed back a clearance above the face, along the
    // face's own normal rather than along `up`: that keeps the along-track
    // motion the craft earned and only undoes the part that went through.
    let arm = body.orientation * probe_offsets()[0];
    let depth = (body.position + arm - point).dot(normal);
    let correction = normal * (PENETRATION_LIMIT - depth);

    // And take the inward velocity off, or the next tick simply repeats this.
    // No restitution: the suspension is what makes a landing springy, and it
    // gets to do that from the tick after this one.
    let into = body.linear_velocity.dot(normal);
    let velocity = if into < 0.0 {
        body.linear_velocity - normal * into
    } else {
        body.linear_velocity
    };
    let _ = up;
    Some((correction, velocity))
}

/// Runs both probes and the grounded-only terms.
#[must_use]
pub fn evaluate<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    target_height: f32,
) -> Hover {
    let offsets = probe_offsets();
    let probes = probe_pair(state, handling, env, raycaster, offsets, target_height);

    let contacts = probes.iter().filter(|probe| probe.contact).count() as u32;
    if contacts == 0 {
        return Hover::airborne(probes);
    }

    let mut normal_sum = Vec3::ZERO;
    let up = state.body.up();
    for probe in &probes {
        if probe.contact {
            normal_sum += probe.normal;
        }
    }
    let average_normal = (normal_sum / (contacts as f32)).normalize_or_zero();

    let mut alignment_torque = up.cross(average_normal) * ALIGNMENT_GAIN;
    // Project the right-axis component out, which is what removes pitch and
    // leaves roll and yaw. Written as a subtraction of the projection rather
    // than as a basis change, so there is one operation to check.
    let right = state.body.right();
    alignment_torque -= right * alignment_torque.dot(right);

    // The load the spring is calibrated to carry, pressing the craft onto the
    // surface: `-track_gravity * mass * grounded * (1 - magLockBlend)` along the
    // averaged normal, with **this** frame's groundedness (the original
    // accumulates `craft+0x2b0` inside this same function, half per contacting
    // probe, before the epilogue reads it). `mass` is the body's, which is the
    // field `Ship_HoverTwoPoint` and the gravity term both read.
    let grounded = crate::ship::ShipState::quantise_grounded(contacts);
    let downforce = -average_normal
        * (handling.physical.track_gravity
            * state.body.mass
            * grounded
            * (1.0 - state.mag_lock_blend)
            * DOWNFORCE_SCALE);

    // Bank-to-yaw: a body-local yaw term proportional to how far the right axis
    // has tipped out of the world horizontal, and cancelled by a magstrip lock
    // like the rest of the suspension.
    let local_angular_torque = Vec3::new(
        0.0,
        BANK_TO_YAW_GAIN * right.y * (1.0 - state.mag_lock_blend),
        0.0,
    );

    // Two probes both penetrating would each ask for a teleport, and applying
    // both would move the body twice as far as either wanted. The larger
    // correction is taken instead. The original applies them in sequence against
    // a position that moves as it goes, which cannot be reproduced without
    // re-casting between probes; this is a choice, not a finding.
    let mut escape = Vec3::ZERO;
    for probe in &probes {
        if probe.escape.length_squared() > escape.length_squared() {
            escape = probe.escape;
        }
    }

    Hover {
        probes,
        contacts,
        average_normal,
        alignment_torque,
        downforce,
        local_angular_torque,
        escape,
    }
}

#[cfg(test)]
mod tests;
