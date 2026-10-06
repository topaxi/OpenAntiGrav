//! The two-probe air cushion, and the grounded-only terms that come with it.
//!
//! Force law specified in `docs/physics/README.md` at confidence **91**, from static
//! analysis plus the runtime reads cited per constant. Numbers are transcribed from that
//! page even where they look wrong: a transcription can be corrected against a trace, an
//! improvement cannot.
//!
//! Deliberate, and easy to "fix" by mistake:
//!
//! - **Stiffness carries no handling parameter.** It is
//!   `mass * 0.3 * (normal_gravity + track_gravity)`; ship-to-ship differences come from
//!   the gravity values.
//! - **`rebound` and `landing_rebound` scale damping only.** Damping multiplies the spring
//!   term, so where the spring is zero the damping is zero too.
//! - **The probe reach is the spring's own target**, built from `ride_height`
//!   (`Ship_CastHoverProbes` ends its ray at `probe - up * craft+0x2f0`;
//!   `docs/ghidra/functions/psp-pulse-usa/engine.md`). The suspension is therefore
//!   compression-only. See [`target_height`] and [`probe`].
//! - **The suspension carries two gravities.** [`DOWNFORCE_SCALE`] presses the craft onto
//!   the surface with `track_gravity * mass * grounded`, which the spring's
//!   `normal_gravity + track_gravity` calibration assumes. Removing it also removes most
//!   attitude damping, because the damper multiplies the spring magnitude.
//!
//! Nothing here pitches the ship to the track (on a magstrip that is
//! [`crate::maglock`]'s job). Pitch and roll response is emergent from two forces at two
//! points, and the alignment torque projects its pitch component out. An explicit
//! pitch-to-track term would change the whole model.
//!
//! # Resting travel
//!
//! A probe in contact reports a height in `0..=target`. The craft rests where the two
//! springs carry gravity **plus** the downforce:
//!
//! ```text
//! 2 * 0.3 * HOVER_K * (normal_gravity + track_gravity) * compression
//!     = normal_gravity * classScale + track_gravity
//! ```
//!
//! `mass` cancels. The gravities would cancel too, resting every craft at `1.25`, if
//! `classScale` were `1.0`; it is `<GlobalClass><GravityMul airborne/>`, four values for
//! the four speed classes. With the real scale the reference capture's class rests at
//! `1.2390` against the original's live `craft+0x308` of `2.8878` on a `4.125` target
//! (`1.237` compressed), `0.16 %` away. So the rest point is near half the travel for
//! every craft. See `docs/physics/angular-velocity-column.md`.
//!
use oag_core::math::Vec3;

use crate::barrel_roll;
use crate::collide::{Ray, Raycaster};
use crate::forces::Environment;
use crate::params::Handling;
use crate::ship::ShipState;

/// The global hover stiffness scale, `K`.
///
/// The static image holds `1.0`, but **ship construction overwrites it with `4/3`**
/// before any race runs. Runtime-verified: `0x08ab0e20` reads `1.3333333730697632`
/// (`0x3faaaaab`) at a breakpoint in `Ship_UpdateCraft` during a Time Trial. Confidence
/// **95**. The exact quotient is the original's value; the decimal `1.333_3` is
/// `0x3faaa64c`, a different `f32`.
pub const HOVER_K: f32 = 4.0 / 3.0;

/// How long after touchdown `landing_rebound` replaces `rebound`, in seconds.
pub const LANDING_WINDOW: f32 = 0.2;

/// How much a full magstrip lock raises the hover target height:
/// `target = (...) * (1 + 0.2 * magLockBlend) * K2`.
pub const TARGET_MAG_LOCK_GAIN: f32 = 0.2;

/// The global scale on the hover target height, `K2` at `0x08ab0e1c`.
///
/// `0.75`, read two independent ways off Pulse in PPSSPP during a Time Trial: the global
/// holds `0x3f400000`, and at a breakpoint in `Ship_UpdateCraft` the spring target
/// `craft+0x2f0` reads **4.125** against the handling block's `ride_height` of **5.5**
/// (`4.125 / 5.5 = 0.75`, with `craft+0x74` at `0.0` so no additive offset is in play).
///
/// Confidence **92**. Not 95: one ship, one class, one track, and nothing traced across a
/// magstrip or a leap, where the other two factors in [`target_height`] would move.
///
/// This is the suspension's headroom. With the identity a resting ship sat 0.147 units
/// (2.7 % of `ride_height`) below the top of its range; at `0.75` it sits about
/// **1.5 units** below the point where it loses the ground.
pub const TARGET_GLOBAL_SCALE: f32 = 0.75;

/// The largest reduction the weapon slowdown timer can make to the hover target:
/// `min(craft+0x2e0, 4.0)`. **Unreachable on the shipped disc**, see [`crate::slowdown`].
pub const SLOWDOWN_ADJUST_MAX: f32 = 4.0;

/// The hover spring's target height.
///
/// ```text
/// target = (ride_height + offset - min(slowdownTimer, 4.0)) * (1 + 0.2 * magLockBlend) * K2
/// ```
///
/// **`ride_height` is the primary term** (parser `+0x94`, `craft+0x70`, then
/// `Ship_UpdateCraft` builds `craft+0x2f0`, which `Ship_HoverTwoPoint` springs against;
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`, confidence 88). It is also the
/// raycast length, so one field serves both uses.
///
/// **The additive `offset` is taken as zero.** The slot is `craft+0x74` and nothing was
/// found that writes it ([`crate::params::Pitch::antigrav_height_adjust`] has no reader
/// in the craft path, confidence 50 for "parsed but never consumed"). A positive offset
/// would also leave the model with no fixed point: the target would exceed the probe
/// reach, the spring would push *up* at every height a probe can report, and the ship
/// at rest would leave probe range every cycle (measured with `ride_height` 5.5 and
/// offset 1.0). A negative offset is not substituted, because nothing recovered says what
/// it would be.
#[must_use]
pub fn target_height(handling: &Handling, mag_lock_blend: f32, slowdown_timer: f32) -> f32 {
    let slowdown_adjust = slowdown_timer.clamp(0.0, SLOWDOWN_ADJUST_MAX);
    let base = handling.antigrav.ride_height - slowdown_adjust;

    base * (1.0 + TARGET_MAG_LOCK_GAIN * mag_lock_blend) * TARGET_GLOBAL_SCALE
}

/// The probe height below which penetration escape teleports the body, in units.
///
/// Not part of the spring, which reads `targetHeight - h` at every height. See
/// [`HoverProbe::escape`].
pub const PENETRATION_LIMIT: f32 = 1.0;

/// The surface-alignment torque gain, grounded only.
///
/// Applied as `ALIGNMENT_GAIN * cross(up, avgNormal)` with the right-axis component
/// projected out, so it levels roll and yaw but deliberately not pitch. The torque
/// reaches the body through [`crate::forces::drain`] as a torque and
/// [`crate::integrate`] divides by the recovered inertia tensor (`15.6` on roll, `21.6`
/// on yaw).
///
/// The measurements and arithmetic behind the points below are in
/// `docs/physics/README.md`, "Alignment gain: the measurements behind the numbers".
///
/// - **Sign: `+400`, where the page writes `-400`.** The page also says the term levels
///   roll and yaw, and the two contradict: with `omega = up x n`,
///   `d(up)/dt = n - up * (n . up)` points from `up` toward `n`, so only
///   `+k * cross(up, n)` aligns. No magnitude can flip that. Behaviour is the stronger
///   claim, so the literal is the transcription error. Confidence **84**. Leading
///   explanation: a handedness difference in the original's frame, shared with
///   [`crate::passive::WEATHERVANE_GROUND`] (`engine.md` lists handedness as
///   undetermined); a prediction to check against the next cross product recovered.
/// - **Magnitude: `400` is the original's literal.** The PS2 build materialises
///   `0xC3C80000` (`-400.0f`) in `Ship_HoverFourCorner` (`0x0015a940`) and
///   `Ship_HoverTwoPoint` (`0x0015b978`), same sign and operand order as PSP.
/// - **Stability, as a bare angular acceleration (before the inertia divide): unstable, and
///   not a typo.** With the acceleration held across the three sub-steps the frame
///   `H = 1/60` was 2.22x too large for `k = 400`, `c = 2` ([`crate::passive::ROLL_DAMPING`]):
///   roll grew `1.020167` per tick, measured at 1.018 to 1.022. The torque is now divided by
///   the recovered tensor (`15.6` on roll), which makes the oscillator stable at about 13 %
///   above the measured stiffness (`hover::tests::the_roll_oscillator_is_stable_by_the_margin_that_was_measured`).
/// - **The original never showed that growth.** A 150-tick no-input trace holds its up axis
///   within 0.000476 rad with no oscillation, and a 0.25 rad roll step response gives a
///   closed-loop stiffness of about **20.2** (not 400) and damping about 1.6. Confidence
///   **80** (one ship, one track, hand-fitted). Nothing here is tuned to hide the gap;
///   changing a magnitude would destroy the measurement.
/// - **The projection is real.** Re-read instruction by instruction at confidence 92:
///   `cross(up, avgNormal)` at `0x0884ac44`, `-400.0f` at `0x0884ac5c`, a
///   `vdot.t`/`vscl.q`/`vsub.q` triple against `craft+0x170` (the right axis) at
///   `0x0884acac`-`0x0884acfc`, `vadd.t` into `craft+0x350` at `0x0884ad24`. The crate's
///   under-damped pitch is not this term; see `engine.md`, "The alignment torque: the
///   projection is real", and `docs/physics/angular-velocity-column.md`.
/// - **Failure mode is growth, not anti-alignment**, pinned independent of simulation by
///   `the_alignment_torque_points_from_the_ships_up_axis_toward_the_surface_normal`.
///   The `F x r` anomaly in `Body_AddForceAtPoint` is not what threw the ship off.
pub const ALIGNMENT_GAIN: f32 = 400.0;

// The fitted `ALIGNMENT_INERTIA` (`19.8`) that once divided this gain is gone: the
// accumulators hold torque, so the divisor is the recovered tensor. Its check stands.
// The torque acts on roll and yaw (`I_zz = 15.6`, `I_yy = 21.6`) and the fit sat between
// them. On roll alone the replacement is `400 / 15.6 = 25.6` against the measured
// closed-loop `20.2`, a natural frequency of `5.06` against `4.49` rad/s. That 13 %
// discrepancy is left standing, not tuned away; see
// `docs/physics/angular-velocity-column.md`.

/// The bank-to-yaw coupling gain, grounded only:
/// `angularLocal.y += 30 * right.y * (1 - magLockBlend)`.
///
/// The magstrip factor comes from `engine.md`'s per-component enumeration of the hover
/// epilogue's writes; `docs/physics/README.md` records the term without it.
pub const BANK_TO_YAW_GAIN: f32 = 30.0;

/// The grounded downforce, `-track_gravity * mass * grounded * (1 - magLockBlend)` along
/// the averaged contact normal.
///
/// Read off `Ship_HoverTwoPoint`'s epilogue, `0x0884ad78`-`0x0884ae18`: `track_gravity`
/// from `[handling+0x6c]`, `mass` from `body+0x374`, THIS frame's grounded fraction
/// `craft+0x2b0`, negated, scaled onto the averaged normal `craft+0x140`, times
/// `1 - craft+0x280`, then added to the world force accumulator `craft+0x330`. Confidence
/// **92**; the PS2 four-corner twin computes the same
/// ([ps2-pulse-eu/craft-update.md](../../../docs/ghidra/functions/ps2-pulse-eu/craft-update.md)).
/// The coefficient is `1.0`; this constant only keeps the name the tree uses.
///
/// It is the missing pitch damping. The spring's damper multiplies the spring magnitude
/// (`1 + rebound * clamp(-0.1 * vn, -1, 2)`), so rate feedback is proportional to the
/// load already carried. Without the downforce the probes carry `normal_gravity * mass`;
/// with it, `(normal_gravity + track_gravity) * mass`, a factor of **17** on shipped
/// values. The resting compression is
///
/// ```text
/// compression = (normal_gravity * classScale + track_gravity)
///             / (0.8 * (normal_gravity + track_gravity))
/// ```
///
/// (`0.8` is two probes times `0.3 * HOVER_K`), which the live start-line read measured
/// at **1.237** against `1.2390` predicted. See [`probe_offsets`] and
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
/// [`TARGET_GLOBAL_SCALE`]. `Ship_InitCraft` (`0x08849354`) writes `0xbfc00000` into the
/// `y` lane of both offsets at `0x08849480`-`0x088494b4`.
pub const PROBE_DROP_RAW: f32 = 1.5;

/// How far fore and aft the two probes sit, before [`TARGET_GLOBAL_SCALE`]: the `z`
/// immediates `0x40c00000` and `0xc0c00000` from the same two writes.
pub const PROBE_HALF_SPACING_RAW: f32 = 6.0;

/// Where the two probes sit in body space, `(0, -1.125, -/+4.5)`, front first.
///
/// **A code literal, identical for every craft.** `Ship_InitCraft` (`0x08849354`) writes
/// `(0, -1.5, +/-6)` and the ship-entity constructor scales both by the `0.75` of
/// [`TARGET_GLOBAL_SCALE`]. Confidence 92; see `engine.md`, "The probe geometry is a code
/// literal, like the inertia tensor". No handling parameter enters it; hull dimensions
/// go to the collider.
///
/// The `-1.125` drop and `+/-4.5` spacing were once refuted by wrong load arithmetic;
/// [`DOWNFORCE_SCALE`] fixed both. A craft carrying `normal_gravity` alone rests `0.147`
/// into a `4.125` reach, while one carrying both rests `1.25` into it (live read: `2.888`
/// against `4.125`, `1.237` compressed). The spacing also has an independent leg: the
/// pitch oscillator's measured `omega_n` of `9.32-9.72` rad/s against
/// `sqrt(2 * 0.4 * (ng + tg) * 4.5^2 / 15.6) = 9.4`, with nothing fitted.
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
/// Transcribed and internally odd, so kept literal and flagged for M3: the branch is
/// discontinuous at `t = 0.2` (the blend gives `0.1 * rebound`, the other arm
/// `rebound`) and `rebound * t` mixes a coefficient with a time. `1 - 5*t` reaches zero
/// exactly at the window's end, so a factor was most likely lost in transcription.
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
/// `target_height` comes from [`target_height`]; it stays a parameter because its
/// additive offset is a guess. `grounded_prev` is last frame's groundedness, which the
/// spring scales by `0.75 * grounded_prev + 0.25`: the new contact count does not exist
/// yet when the spring is evaluated.
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

    // Cast along the ship's own up axis, not world gravity: this is why magstrips and
    // inversions work. The reach is the spring's target, not `ride_height`
    // (`Ship_CastHoverProbes` at `0x0884a0b4`: `probe - up * craft+0x2f0`), so the
    // spring is compression-only; consistent with the capture because
    // [`DOWNFORCE_SCALE`] rests the craft `1.25` units into the reach.
    let ray = Ray::new(point, -up, target_height);

    let Some(hit) = raycaster.raycast(ray, env.self_collider, false) else {
        return HoverProbe::miss(point);
    };
    probe_from_hit(state, handling, point, &hit, target_height)
}

/// The half of [`probe`] that turns a hit into a force, without casting. Split out for
/// [`derived_hit`], which above [`FAST_PROBE_SPEED`] builds the rear hit arithmetically.
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

    // Penetration escape runs before the surface check, so on any surface:
    // `Ship_HoverTwoPoint` computes `craft+0x308`, escapes on it, and only then tests
    // the collider type, so a probe inside a `Wall` is extruded too. The gate is
    // `craft+0x208 == 1`, the `Floor` class, so escape is narrower than contact (a
    // magstrip holds kinematically, a wall is `crate::wall`'s business).
    let escape = if hit.surface == crate::collide::Surface::Floor && height < PENETRATION_LIMIT {
        up * (PENETRATION_LIMIT - height)
    } else {
        Vec3::ZERO
    };

    // Contact, and so the spring, only for surface types 1 (Floor) and 3 (Mag Floor).
    if !hit.surface.is_hoverable() {
        return HoverProbe {
            escape,
            ..HoverProbe::miss(point)
        };
    }

    // The page's `bodyLinearVel + M * cross(probeLocal, angularVel)` looks like the
    // negated point velocity, but it is not: `vcrsp.t` at `0x0884a970` crosses the local
    // offset with `body+0x150`, which is `-omega` under `w_game = -w_physics`, so the
    // result is the conventional `omega x r`. See `engine.md`, "The damper's point
    // velocity is the conventional `omega x r`".
    let velocity = body.velocity_at(point);
    let normal_velocity = velocity.dot(hit.normal);

    let gravity = handling.physical.normal_gravity + handling.physical.track_gravity;
    let load = 0.75 * state.grounded_prev + 0.25;
    // Grouped left to right: operation order is part of the result. No `mul_add`.
    let spring = handling.physical.mass * 0.3 * (target_height - height) * HOVER_K * load * gravity;

    let damping = (-0.1 * normal_velocity).clamp(-1.0, 2.0);
    let reb = rebound_coefficient(handling, state.time_since_landing);
    let rebound = barrel_roll::rebound_override(state, reb);

    // The magstrip blend fades the suspension out for `crate::maglock`'s kinematic hold
    // (ramped by `maglock::ramp`).
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
/// `Ship_CastHoverProbes` branches on `craft+0x2ec <= 50.0`, the cached
/// `|dot(velocity, forward)|` of the previous frame (recovered 199/199 against a capture;
/// `engine.md`). Below it two rays are cast. Above it one is, and a hit manufactures the
/// other probe's hit record.
pub const FAST_PROBE_SPEED: f32 = 50.0;

/// How far the derived hit is pushed along `up` per unit of forward-facing surface
/// normal (`vscl.q` by `0x40c00000`).
///
/// A flat `6.0` with no derivation; the exact answer over a 9-unit spacing would be
/// `-9 * dot(n, forward) / dot(n, up)`. Transcribed, not improved.
pub const DERIVED_HIT_SLOPE_GAIN: f32 = 6.0;

/// The rear probe's hit, manufactured from the front probe's.
///
/// This keeps a craft attached over a crest: above [`FAST_PROBE_SPEED`] the front hit
/// record is copied, its point translated by the vector between the probes and pushed
/// along `up` by `dot(normal, forward) * 6.0`. The hit flag is copied too, so a front
/// probe in contact guarantees a rear one and `grounded` cannot read `0.5` at speed.
///
/// Which probe is cast was **measured**: `Ship_InitCraft` (`0x08849480`) gives probe 0
/// `+6.0` and probe 1 `-6.0` raw, and nothing settles which is the nose. Casting
/// `offsets[1]` first and deriving `offsets[0]` took the twelve-circuit solo benchmark
/// from nine clean laps to two (`docs/gameplay/ai.md`), so the order below is right.
///
/// It is also a downforce: the derived hit is not ray-clamped, so on a falling slope it
/// can sit further below the probe than `target_height`, and the spring
/// `(target_height - height)` then pulls the craft **down**. No other term does.
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

/// Catches a hull that crossed a hoverable surface between ticks and puts it back on top.
///
/// **This one is ours**; nothing in the original does this (its analogue is in
/// `crate::wall`'s docs). The original's contact test only looks one way: [`probe`]
/// casts down along `-up` for `target_height`, so once a surface is above the hull no
/// cast finds it and the craft is gone. Instrumented on Fort Gale (`14_Track`), a craft
/// descends at 139 units/s (76 straight down while in contact), and on the first
/// airborne tick it is already 2.0 units below the surface and ballistic from there; a
/// human driving the same section falls through too (`docs/gameplay/ai.md`).
///
/// It sweeps each probe's motion segment, and if that crosses a hoverable face from the
/// front, places the craft back on the surface with its inward velocity removed. A
/// continuous-collision test, chosen deliberately. It runs **only when both probes found
/// nothing**, so normal contact never reaches it, and needs a surface on the segment, so
/// an authored jump is unaffected (`13_Track`'s jump still flies).
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

    // Earliest crossing, in probe order so a tie depends only on the offsets.
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
        // Only a face the craft is moving into; leaving a surface from underneath
        // (inside a loop) must not be caught.
        if travel.dot(hit.normal) >= 0.0 {
            continue;
        }
        if best.is_none_or(|(nearest, _, _)| hit.distance < nearest) {
            best = Some((hit.distance, hit.point, hit.normal));
        }
    }
    let (_, point, normal) = best?;

    // Back a clearance above the face along its normal, not `up`: keeps the
    // along-track motion and undoes only the part that went through.
    let arm = body.orientation * probe_offsets()[0];
    let depth = (body.position + arm - point).dot(normal);
    let correction = normal * (PENETRATION_LIMIT - depth);

    // Take the inward velocity off or the next tick repeats this. No restitution: the
    // suspension makes landings springy from the next tick.
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
    // Projecting the right axis out removes pitch and leaves roll and yaw.
    let right = state.body.right();
    alignment_torque -= right * alignment_torque.dot(right);

    // The load the spring is calibrated to carry, with THIS frame's groundedness (the
    // original accumulates `craft+0x2b0` in this function, half per contacting probe,
    // before the epilogue reads it). `mass` is the body's.
    let grounded = crate::ship::ShipState::quantise_grounded(contacts);
    let downforce = -average_normal
        * (handling.physical.track_gravity
            * state.body.mass
            * grounded
            * (1.0 - state.mag_lock_blend)
            * DOWNFORCE_SCALE);

    // Bank-to-yaw: body-local yaw proportional to how far the right axis has tipped out
    // of the horizontal, cancelled by a magstrip lock. Skipped on the grid
    // (`craft+0x2a4 != 0`, see `ShipState::on_grid`).
    let bank = bank::gain(state) * right.y * (1.0 - state.mag_lock_blend);
    let local_angular_torque = Vec3::new(0.0, if state.on_grid { 0.0 } else { bank }, 0.0);

    // Two penetrating probes would each ask for a teleport; applying both would move the
    // body twice as far. The larger correction is taken. The original applies them in
    // sequence against a moving position, which needs re-casting between probes. This is
    // a choice, not a finding.
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

mod bank;
pub use bank::BANK_TO_YAW_GAIN_FOUR_CORNER;
#[cfg(test)]
mod tests;
