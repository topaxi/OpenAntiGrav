//! The passive terms: drag, resistance, damping and gravity.
//!
//! Small, always on and easy to overlook, and together what makes the ship handle: there is
//! **no speed-dependent steering**, so the drag, the weathervane torque and the angular
//! damping here carry that load.
//!
//! From `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The passive terms" and the gravity
//! paragraph. Confidence 74 to 80, decompilation only, **except the four drag
//! coefficients**, read from `Ship_ApplyQuadraticDrag` (`0x08848e28`) as the immediates
//! `0xbf666666` (`-0.9`, the `craft+0x2a4 == 0` branch), `0xbdcccccd` ([`DRAG_REVERSING`]),
//! `0xbba3d70a` ([`DRAG_GROUND`]) and `0xbb03126f` ([`DRAG_AIR`]) against the `0xbe4ccccd`
//! (`-0.2`) threshold: confidence **95**. [`ROLLING_RESISTANCE`]'s magnitude is confirmed
//! too: `Ship_ApplyRollingResistance` (`0x08848f4c`) builds `-v/|v|` (`vrsq.s`, a `vpfxs`
//! negation on `vscl.t`) then `vadd.t C610,C610,C610`, a **self-add that doubles it**, so the
//! magnitude is exactly `2` with no parameter. Confidence 92.
//!
//! # The airbrake drag term lives in [`crate::airbrake`]
//!
//! `Ship_UpdateAirbrakes` reads `Airbrake.drag` (class `+0xe8`, `+0x54` through `craft+0x70`)
//! at `0x0884cce8`; the term is in [`crate::airbrake::evaluate`] (scale `1e-5`, two literals
//! `0x3c23d70a` and `0x3a83126f` on one vector; it **accelerates** along `+forward` and a
//! reversing ship is not excluded). Recorded here because this is where one would look.
//!
//! It is not the missing speed-proportional resistance that
//! `docs/physics/force-balance-ground-truth.md` tracked as the M4 blocker: **there is no
//! missing resistance**. This crate's force law reproduces a standing-start capture from
//! `fs = 0.56` to `fs = 47.67` at rms `0.127` units of force, and the `~52`-unit deficit in
//! the two older captures is a per-frame *velocity* reduction in the collision path, outside
//! every force accumulator.
//!
//! # `craft+0x2ec` is `|dot(velocity, forward)|`, which makes one branch dead
//!
//! `Ship_UpdateCraft` computes the cached speed at `0x0884992c`-`0x08849938` as `vdot.t`
//! **then `vabs.s`**, and `search_instructions` over every `0x2ec(` displacement finds one
//! writer on a craft base, so every passive term reads a non-negative value. Confidence 90
//! on the `vabs.s`, 88 on the sole-writer negative (the scan cannot see a store through a
//! rebased pointer, the caveat `engine.md` records). Consequences, both implemented:
//!
//! - `Ship_ApplyQuadraticDrag`'s `craft+0x2ec < -0.2` test never fires, so [`DRAG_REVERSING`]
//!   is **unreachable in the original**; kept because the `-0.1` immediate is in the PS2
//!   build too.
//! - The drag is `k * |forwardSpeed| * velocity` with `k < 0`, **dissipative in both
//!   directions of travel**. The worry that the literal form adds energy while reversing
//!   came from feeding it a signed speed.

use oag_core::math::Vec3;

use crate::params::Handling;

/// Quadratic drag coefficient while reversing, `forwardSpeed < -0.2`: twenty to fifty times
/// the forward coefficients, so the ship is reluctant to fly backwards.
pub const DRAG_REVERSING: f32 = -0.1;

/// Quadratic drag coefficient while grounded.
pub const DRAG_GROUND: f32 = -0.005;

/// Quadratic drag coefficient while airborne. **Smaller than [`DRAG_GROUND`]**, not larger;
/// a reimplementation that "fixed" the intuitive ordering would change how far a ship
/// carries off a jump.
pub const DRAG_AIR: f32 = -0.002;

/// Forward speed below which the reversing drag coefficient applies.
pub const DRAG_REVERSE_THRESHOLD: f32 = -0.2;

/// The always-on rolling resistance magnitude, along `-unit(velocity)`: constant, independent
/// of speed and every handling parameter. **Gated on `forwardSpeed > 0`**, so it does not act
/// on a reversing or purely sideways ship, which also keeps `unit(velocity)` from being asked
/// for at rest.
pub const ROLLING_RESISTANCE: f32 = 2.0;

/// Weathervane coefficient while grounded.
///
/// # Why `+0.1` where the page writes `-0.1`
///
/// `engine.md` writes `angularWorld += cross(forward, velocity) * (grounded ? -0.1 : -0.3)`
/// and two lines later calls it "what turns the nose toward the direction of travel". In
/// **this crate's** right-handed frame those are incompatible, by the identity that settled
/// the alignment torque:
///
/// ```text
/// omega = f x v
/// d(f)/dt = omega x f = (f x v) x f = v * (f . f) - f * (v . f) = v - f * (v . f)
/// ```
///
/// which points from `f` toward `v`: a torque along `+cross(forward, velocity)` turns the
/// nose toward travel and a negative coefficient turns it away, whatever the magnitude. An
/// anti-weathervaning term would spin every ship out of every corner, so the stated
/// behaviour is implemented and the written sign is not.
///
/// This is the second term with that pattern (with `-400 * cross(up, avgNormal)`, see
/// [`crate::hover::ALIGNMENT_GAIN`]). Two independent transcription errors of one shape is
/// a poor explanation; a **systematic handedness difference** between the original's frame
/// and ours is a good one (the page lists handedness as undetermined), and predicts any
/// further cross-product term from this path needs the same flip.
///
/// Confidence 84 on the direction, the rubric's ceiling for decompilation-only evidence.
/// **The magnitude is unverified** and stays M3 work.
pub const WEATHERVANE_GROUND: f32 = 0.1;

/// Weathervane coefficient while airborne, three times the grounded one; sign as in
/// [`WEATHERVANE_GROUND`].
pub const WEATHERVANE_AIR: f32 = 0.3;

/// Yaw angular damping, hard-coded for every craft in the game.
pub const YAW_DAMPING: f32 = -5.0;

/// Roll angular damping, hard-coded for every craft in the game.
///
/// `-5.0` in the undecoded `craft+0x2a4` mode 0 and `-2.0` otherwise; mode 0 also disables
/// `rebound` and multiplies drag by about 180, so it reads as a front-end or display mode.
/// The racing value is used and the mode is not implemented (interpreting that enum is a
/// guess at confidence 40).
pub const ROLL_DAMPING: f32 = -2.0;

/// Vertical damping coefficient, along the ship's own up axis. **Airborne only**: scaled by
/// `1 - grounded` ([`vertical_damping`]), so zero on a fully grounded craft.

/// Quadratic drag, as a world-space force.
///
/// ```text
/// k = forwardSpeed < -0.2 ? -0.1 : grounded ? -0.005 : -0.002
/// worldForce += velocity * forwardSpeed * k
/// ```
///
/// `grounded` is the **previous** frame's contact flag. `forward_speed` is the cached
/// `craft+0x2ec`, `|dot(velocity, forward)|` (module docs), so with `k < 0` the power
/// delivered, `|v|^2 * forwardSpeed * k`, is negative both ways: dissipative while reversing
/// too. The `forward_speed < -0.2` arm cannot be taken by the original; it is kept because
/// the immediate is in both binaries, and a caller passing a signed speed will exercise it
/// and diverge.
///
/// Shape, from `Ship_ApplyQuadraticDrag` (`0x08848e28`): the coefficient is selected first (a
/// four-way branch, `craft+0x2a4 == 0` taking `-0.9`), then `vscl.t C600,C200,S310` scales
/// the **whole velocity vector** by the scalar speed and `vscl.t C600,C600,S610` by the
/// coefficient. Lateral velocity is dragged too; projected onto `forward` it is `k * fs^2`,
/// which is why the along-track balance sees a quadratic.
#[must_use]
pub fn quadratic_drag(velocity: Vec3, forward_speed: f32, grounded: bool) -> Vec3 {
    let k = if forward_speed < DRAG_REVERSE_THRESHOLD {
        DRAG_REVERSING
    } else if grounded {
        DRAG_GROUND
    } else {
        DRAG_AIR
    };

    velocity * forward_speed * k
}

/// Rolling resistance, as a world-space force.
/// Rolling resistance, as a world-space force. Zero unless `forward_speed > 0`, as in the
/// original, which also keeps this finite at rest.
#[must_use]
pub fn rolling_resistance(velocity: Vec3, forward_speed: f32) -> Vec3 {
    if forward_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let speed = velocity.length();
    if speed <= 0.0 {
        return Vec3::ZERO;
    }

    -(velocity / speed) * ROLLING_RESISTANCE
}

/// The weathervane torque, in the world angular accumulator.
///
/// ```text
/// angularWorld += cross(forward, velocity) * (grounded ? -0.1 : -0.3)
/// ```
///
/// **This, not any control term, turns the nose toward the direction of travel**, three
/// times stronger in the air. `grounded` is **this** frame's contact flag (the weathervane
/// runs after hover). The sign is flipped relative to the page; see [`WEATHERVANE_GROUND`].
///
/// "Travel" is three-dimensional: a ship falling straight down gets a nose-down pitch,
/// because `cross(forward, velocity)` has a pitch component whenever the velocity leaves the
/// ship's horizontal plane. A property of the term as recovered, not an artefact.
#[must_use]
pub fn weathervane(forward: Vec3, velocity: Vec3, grounded: bool) -> Vec3 {
    let k = if grounded {
        WEATHERVANE_GROUND
    } else {
        WEATHERVANE_AIR
    };

    forward.cross(velocity) * k
}

/// Angular damping, as a body-local torque.
///
/// ```text
/// angularLocal += (-pitch_damping, -5.0, -2.0) * bodyAngularMomentumLocal
/// ```
///
/// Only the pitch axis is tunable per ship. `pitch_damping` is negated here and expected
/// positive in the data, unlike `Brakes.amount`, whose sign is in the parameter.
///
/// # It reads the momentum, not the velocity
///
/// `Ship_ApplyAngularDamping` (`0x08848ed0`) loads `body+0x160` at `0x08848f08`, angular
/// **momentum** ([`crate::forces::Accumulators::local_angular`]), so the argument is
/// `I * omega` in the body frame and the result a torque, dimensionally consistent with the
/// drives it shares an accumulator with.
///
/// A version taking angular *velocity* let the old "drive and damping share one accumulator,
/// so the inertia cancels" argument look sound. Under `-c * L` it does **not** cancel, and
/// that surviving factor is the `22x` the yaw axis was missing for two passes. On a constant
/// diagonal tensor `I^-1 * (-c * I * omega)` is `-c * omega`, so the damping rate is
/// unchanged; what changes is that the drives are divided by `I` and the damping is not, the
/// original's arrangement.
#[must_use]
pub fn angular_damping(handling: &Handling, local_angular_momentum: Vec3) -> Vec3 {
    let coefficients = Vec3::new(-handling.pitch.pitch_damping, YAW_DAMPING, ROLL_DAMPING);

    coefficients * local_angular_momentum
}

/// Vertical damping, as a world-space force.
///
/// ```text
/// worldForce += up * dot(up, velocity) * -0.25 * (1 - grounded)
/// ```
///
/// Along the ship's own up axis, not world up.
///
/// # The scale is `1 - grounded`, not `1 - magLockBlend`
///
/// Inlined in `Ship_UpdateCraft` (step 14); once transcribed with the magstrip blend, which
/// is a different field. Read at instruction level in the PSP `BOOT.BIN` (confidence **92**):
///
/// ```text
/// 08849c60  lwc1   f12,0x2b0(s0)     ; craft+0x2b0 == the 0/0.5/1 grounded fraction
/// 08849c6c  sub.s  f12,f13,f12       ; 1 - grounded   (f13 = 1.0f)
/// 08849c78  mul.s  f12,f12,f14       ; * -0.25f (0xbe800000)
/// 08849ca0  vdot.t S601,C110,C200    ; dot(up, velocity); C110 == craft+0x90 == up
/// 08849ca8  vscl.t C610,C610,S600    ; * -0.25 * (1 - grounded)
/// 08849cac  vadd.t C300,C300,C610    ; craft+0x330, the world force accumulator
/// ```
///
/// `craft+0x280` is the magstrip blend (ramped by `Ship_UpdateMagLock`, scaled by the hover
/// spring); `craft+0x2b0` is the grounded fraction hover rebuilds each frame.
///
/// **Corroborated on the PS2 build** (`Ship_ApplyVerticalDamping`, `0x00159e78`, outlined
/// rather than inlined): same `-0.25`, `dot(up, velocity)` and `1 - grounded`
/// (`docs/ghidra/functions/ps2-pulse-eu/craft-update.md`). One difference, where this crate
/// follows the **PSP**: the PS2 *returns early* when `grounded != 0`, so a half contact gets
/// nothing there, where the PSP scales continuously. Both give a fully grounded craft zero,
/// so the term acts only in the air or on a half contact; applying it while grounded would
/// add a suspension damper the original lacks.
///
/// `grounded` is **this** frame's fraction: the term runs at step 14, after hover rewrote
/// `craft+0x2b0` at step 8.
#[must_use]
pub fn vertical_damping(up: Vec3, velocity: Vec3, grounded: f32) -> Vec3 {
    up * up.dot(velocity) * VERTICAL_DAMPING * (1.0 - grounded)
}

/// Gravity, as a world-space force.
///
/// ```text
/// worldForce.y += -( normal_gravity * classScale * mass * grounded
///                  + flight_gravity             * mass * (1 - grounded) )
/// ```
///
/// Four things `docs/physics/README.md` guessed wrong before the term was read:
///
/// - **It acts on world `.y` only**; `track_gravity` appears in the hover spring's
///   calibration and nowhere else.
/// - `normal_gravity` applies on the ground and `flight_gravity` in the air, blended by the
///   0/0.5/1 fraction, and **only `normal_gravity` gets the per-class scale**.
/// - `grounded` is the **previous** frame's fraction.
/// - `mass` is the rigid body's (`body+0x374`), not [`crate::params::Physical::mass`]; how
///   the two relate was not traced.
///
/// The negation lives in the code, checkably without trusting the VFPU prefix: the hover
/// spring multiplies by `normal_gravity + track_gravity` and must push up, so both are
/// positive in the data and a downward force needs the sign here.
#[must_use]
pub fn gravity(handling: &Handling, mass: f32, class_gravity_scale: f32, grounded: f32) -> Vec3 {
    let ground = handling.physical.normal_gravity * class_gravity_scale * mass * grounded;
    let air = handling.physical.flight_gravity * mass * (1.0 - grounded);

    Vec3::new(0.0, -(ground + air), 0.0)
}

#[cfg(test)]
mod tests;
