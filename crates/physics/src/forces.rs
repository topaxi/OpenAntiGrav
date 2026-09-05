//! One frame of force evaluation, and nothing else.
//!
//! # Why this is a separate module from the integrator
//!
//! `docs/physics/README.md` specifies the craft integrator as
//!
//! ```text
//! dt = clamp(measured_dt, 0, 0.06666)
//! evaluate all forces once at full dt
//! integrate 3 explicit Euler sub-steps of dt/3
//! ```
//!
//! **Forces are evaluated once, at full `dt`, not once per sub-step.** That is
//! counter-intuitive - it is not what a textbook sub-stepped integrator does, and
//! it is not what anyone reaching for "more accuracy" would write - so the
//! split is structural rather than a matter of discipline: this module cannot
//! sub-step because it has no loop and no notion of a sub-step, and
//! [`crate::integrate`] cannot re-evaluate forces because it only sees a
//! [`Body`](crate::ship::Body).
//!
//! # Groundedness is stale for every control term
//!
//! This is the single most likely thing to get wrong, and it is worth more than a
//! comment. In the original, hover is **step 8 of 15** and it clears the contact
//! flag on entry, so the terms that run before it see the *previous* frame's
//! groundedness and the terms after it see this frame's:
//!
//! | Reads | Sees |
//! | --- | --- |
//! | engine, brakes, pitch, quadratic drag, gravity, hover's own load factor | last frame |
//! | lateral grip, weathervane torque | this frame |
//!
//! Confidence 84, from `docs/ghidra/functions/psp-pulse-usa/engine.md`. The obvious
//! reimplementation - resolve contacts, then apply forces - gets a different answer
//! on every takeoff and landing frame, in five terms simultaneously. So [`evaluate`]
//! binds the two values to separately named locals and every call site takes one
//! explicitly; no term reads groundedness out of the state for itself.
//!
//! # What is here and what is deliberately absent
//!
//! Present: the control force law from
//! `docs/ghidra/functions/psp-pulse-usa/engine.md` (engine, brakes, steering, pitch and
//! the passive terms), the two-probe air cushion from `docs/physics/README.md`, and
//! the airbrakes with lateral grip and the sideshift.
//!
//! Present too, and not a force: the magstrip hold ([`crate::maglock`]), which
//! runs inside step 8 and writes the body directly.
//!
//! **Absent**, each because implementing it would mean inventing a trigger nobody
//! has decoded: the four-corner hover variant and the auto-speed law behind its
//! selector, turbo, every flag-gated engine and steering variant, and two arms of
//! the speed-pad boost whose own term *is* here (see
//! [`crate::engine::speedup_pad`]). See
//! `docs/physics/README.md`'s "what is implemented" section for the full list.

use oag_core::math::Vec3;

use crate::airbrake::{self, AirbrakeForces};
use crate::barrel_roll;
use crate::collide::Raycaster;
use crate::hover::{self, Hover};
use crate::maglock;
use crate::params::Handling;
use crate::ship::{ShipControls, ShipState};
use crate::{controls, engine, passive};

/// The body-space box the ship's inverse inertia tensor is built from, `x`.
///
/// See [`YAW_INVERSE_INERTIA`]. A code literal in the ship-entity constructor, not
/// authored data: the same three numbers for every craft in the game.
pub const INERTIA_BOX_X: f32 = 12.0;

/// The body-space inertia box, `y`. See [`INERTIA_BOX_X`].
pub const INERTIA_BOX_Y: f32 = 8.0;

/// The body-space inertia box, `z`. See [`INERTIA_BOX_X`].
pub const INERTIA_BOX_Z: f32 = 12.0;

/// The mass the inertia tensor is built with, and **not** the mass the ship flies
/// with.
///
/// `FUN_08840c74` calls `Body_SetMass(body, 0.9)` at `0x08841414` and then
/// `Body_SetBoxInertia` at `0x08841488`, in that order, so the tensor captures
/// `0.9`. `Ship_UpdateCraft` then calls `Body_SetMass` again **every frame**
/// (`0x0884985c`, from `Physical.mass` at class `+0xf4`) and nothing recomputes
/// the tensor - `Body_SetMass` is six instructions and touches only mass and
/// inverse mass. So the ship's rotational inertia is frozen at construction and
/// decoupled from its translational mass for the rest of the race.
///
/// That is not a detail: it is what the captures measure. See
/// [`YAW_INVERSE_INERTIA`].
pub const INERTIA_MASS: f32 = 0.9;

/// The yaw entry of the ship's body-space inverse inertia tensor, `1 / I_yy`.
///
/// **Recovered, not fitted.** This replaces `YAW_DRIVE_CALIBRATION`, a fitted
/// `0.0452` that stood here while the tensor's writer was unknown. It is a
/// textbook solid-box inertia, and the numbers are literals in the binary.
///
/// # The writer
///
/// `Body_SetBoxInertia` (`0x0884e1ac`, 26 instructions) zeroes `body+0x40..0x80`
/// from the all-zero constant at `0x08a907e0` and writes three diagonal entries:
///
/// ```text
/// 0884e1ac  mul.s f13,f13,f13        ; y*y
/// 0884e1b4  mul.s f14,f14,f14        ; z*z
/// 0884e1c0  mul.s f12,f12,f12        ; x*x
/// 0884e1cc  add.s f15,f13,f14        ; y*y + z*z
/// 0884e1e4  add.s f14,f12,f14        ; x*x + z*z
/// 0884e1f8  add.s f12,f12,f13        ; x*x + y*y
/// 0884e1fc  lui   a1,0x4140          ; 12.0f
/// 0884e200  lwc1  f16,0x374(a0)      ; the body's mass
/// 0884e208  mul.s f15,f16,f15
/// 0884e20c  div.s f15,f17,f15        ; 12 / (m * (y*y + z*z))
/// 0884e218  swc1  f15,0x40(a0)       ; I^-1 [0][0]
/// 0884e224  swc1  f14,0x54(a0)       ; I^-1 [1][1]
/// 0884e22c  swc1  f12,0x68(a0)       ; I^-1 [2][2]
/// ```
///
/// `I_xx = m (y^2 + z^2) / 12` is the solid rectangular cuboid, exactly.
///
/// It has **one** caller, the thin world-level wrapper `World_SetBodyBoxInertia`
/// (`0x0884e900`), which itself has **one** caller: the ship-entity constructor
/// `FUN_08840c74`, at `0x08841488`, with the literal box `(12, 8, 12)` built at
/// `0x08841470`-`0x0884148c` from `0x41400000`, `0x41000000`, `0x41400000`.
///
/// # Why one constant for every ship - now answered rather than argued
///
/// The old note here defended a single global factor as "what the evidence
/// supports" and flagged that a per-ship inertia would break it. The box is a
/// **code literal at a single call site**, so every craft in the game genuinely
/// has the same tensor: `Misc` `width`/`length`/`height` reach the *collider*
/// (scaled by `0.75` a few lines earlier in the same constructor) and not the
/// inertia. The global constant is not a compromise; it is the mechanism.
///
/// # It agrees with the captures, and the agreement picks out the mass
///
/// With `(12, 8, 12)` and `m = 0.9` the tensor's inverse is
/// `I = (15.6, 21.6, 15.6)` on `(right, up, forward)`. `scripts/trace-angular-fit.py`
/// fitted the recorded `body+0x160` column against a body-local angular velocity
/// on two captures and got `~(15, 21..22, 14..16)` - the same `x == z` symmetry
/// and the same ratio (`1.385` recovered against `1.413` fitted), from a
/// measurement that knew nothing about this function.
///
/// The agreement is sharp enough to **discriminate the mass**, which is the part
/// worth keeping:
///
/// | tensor built with | `I_yy` | against the fit's `21.2` |
/// | --- | ---: | ---: |
/// | `m = 0.9` (the constructor's) | `21.6` | `1.9 %` |
/// | `m = 1.0` (`Body_Init`'s default) | `24.0` | `13 %` |
///
/// So the captures independently confirm both the `Body_SetMass(0.9)` read *and*
/// [`INERTIA_MASS`]'s claim that the tensor is frozen at construction: if it
/// tracked the runtime mass the fit would have found `24`.
///
/// Against the constant this replaces, `0.0452` fitted versus `0.046296`
/// recovered is `2.4 %` - inside the spread of the fits themselves
/// (`0.0454`/`0.0450` from the force-law fit, `0.04686`/`0.04506` from the
/// angular-column fit).
///
/// # Why scaling the yaw drive is the *exact* form of this, not an approximation
///
/// The original damps **angular momentum**, not angular velocity:
/// `Ship_ApplyAngularDamping` (`0x08848ed0`) loads `body+0x160` at `0x08848f08`,
/// which `docs/ghidra/functions/psp-pulse-usa/rigid-body.md` establishes is `L`, and
/// multiplies it by `(-pitch_damping, -5, -2)`. So the original's yaw axis is
///
/// ```text
/// dL/dt = drive - 5 L,   omega = c L   with c = YAW_INVERSE_INERTIA
/// ```
///
/// and differentiating the second into the first gives
/// `domega/dt = c * drive - 5 * omega`. That is precisely what this crate
/// computes when the yaw drive is scaled by `c` and the damping is left as an
/// angular acceleration: same equilibrium, same `0.2 s` time constant, same
/// transient. **This is the read law, in a different but equivalent
/// arrangement**, not a stand-in for it.
///
/// It also retires the old note's "the inertia cancels at equilibrium, so it
/// cannot be the missing factor". That argument assumed the damping read the
/// angular *velocity*. It reads the momentum, and that is exactly why the
/// inertia survives.
///
/// # The whole crate is on the momentum model now, so this is no longer a
/// special case
///
/// An earlier revision of this section listed two things the constant did *not*
/// fix - the world-angular terms bypassing it, and pitch and roll being about
/// `15.6x` too strong - and deferred both, because half-applying the momentum
/// model is the failure mode [`crate::integrate`]'s docs warn about and because
/// **no capture of a pitch or roll input existed** to validate the move against.
///
/// Both blockers are gone. `verification/scenarios/pitch-both-ways.inputs` and
/// `pitch-hold-thrust.inputs` are captures of a held pitch input, and they put
/// the pitch entry of the tensor at `-15.620` against this reading's `-15.6` -
/// `0.13 %`, on a capture that is `speed/|velocity| == 1.0000` on every tick.
/// The same captures record `body+0x150` alongside `body+0x160` and measure
/// `body+0x160 = I * body+0x150` directly, at `100.0 %` explained on the pitch
/// axis. See
/// [angular-velocity-column.md](../../../docs/physics/angular-velocity-column.md).
///
/// So [`Accumulators`] holds **torque** on both angular axes,
/// [`crate::ship::Body::inertia`] carries [`ship_inertia`], and
/// [`crate::integrate`] divides by it - which means every angular path now
/// carries `I^-1`, including the two that used to bypass it. This constant is
/// still here, still exact, and no longer *applied* anywhere: it is the yaw
/// entry of the tensor [`ship_inertia`] builds, and the two must agree.
///
/// **The yaw axis's observable behaviour is unchanged by that move, and that is
/// a check rather than a hope.** Scaling the summed yaw drive by `c` and leaving
/// the damping as an angular acceleration gives `domega/dt = c*drive - 5*omega`;
/// accumulating torque and damping momentum gives
/// `dL/dt = drive - 5*L` with `omega = c*L`, and differentiating the second into
/// the first is the same equation. A yaw regression after this change is a bug in
/// the change, not physics.
///
/// # Confidence
///
/// **92.** Every instruction of the writer, its single call site, the literal
/// arguments and the mass ordering were read with the Allegrex module, and the
/// result agrees with an independent two-capture fit to `2 %` on the one axis
/// the fit constrains, reproducing a symmetry (`x == z`) the fit found on its
/// own. One binary; the PS2 was not checked.
///
/// A methodology note worth keeping: an earlier revision of this constant
/// recorded "a textbook box tensor for the shipped Assegai hull gives `16.6`,
/// the right order but 33 % out - which is why it stays a lead". That lead was
/// right in **kind** and wrong in its **dimensions** - the hull is not the box;
/// a hard-coded `(12, 8, 12)` is. Being 33 % out was evidence about the inputs,
/// not about the shape.
pub const YAW_INVERSE_INERTIA: f32 =
    12.0 / (INERTIA_MASS * (INERTIA_BOX_X * INERTIA_BOX_X + INERTIA_BOX_Z * INERTIA_BOX_Z));

/// The pitch entry of the ship's body-space inverse inertia tensor, `1 / I_xx`.
///
/// The same `Body_SetBoxInertia` computation as [`YAW_INVERSE_INERTIA`], on the
/// two extents perpendicular to the body's right axis: `12 / (m * (y^2 + z^2))`,
/// which is `1 / 15.6`. Written in the binary's own form rather than as a
/// reciprocal so all three entries are the one expression the instructions
/// perform.
///
/// **Measured, not only read.** Two captures of a held pitch input
/// (`verification/scenarios/pitch-both-ways.inputs` and `pitch-hold-thrust.inputs`)
/// fit the recorded `body+0x160` column against the rotation the recorded basis
/// performs at `-15.620` on this axis, `0.13 %` from the `-15.6` the literals
/// give, with `94 %` of the column explained on a signal that is the dominant
/// axis of those captures. Everything before them left this entry at `29 %`
/// explained and scattering from `-14` to `-24`, which is why the crate's own
/// docs used to warn against reading a tensor into the per-axis fits.
pub const PITCH_INVERSE_INERTIA: f32 =
    12.0 / (INERTIA_MASS * (INERTIA_BOX_Y * INERTIA_BOX_Y + INERTIA_BOX_Z * INERTIA_BOX_Z));

/// The roll entry of the ship's body-space inverse inertia tensor, `1 / I_zz`.
///
/// `12 / (m * (x^2 + y^2))`. The box is square in plan (`x == z`), so this is
/// numerically equal to [`PITCH_INVERSE_INERTIA`] - and it is written out rather
/// than aliased, because the equality is a property of the *box* and would stop
/// holding the moment anyone revisited the literals. The same captures put the
/// measured roll entry at `-15.289`, `2 %` away.
pub const ROLL_INVERSE_INERTIA: f32 =
    12.0 / (INERTIA_MASS * (INERTIA_BOX_X * INERTIA_BOX_X + INERTIA_BOX_Y * INERTIA_BOX_Y));

/// The ship's body-space inertia tensor, as a diagonal on `(right, up, forward)`.
///
/// `(15.6, 21.6, 15.6)`, the reciprocals of the three constants above. This is
/// what [`crate::ship::Body::inertia`] carries for a craft, and the same tensor
/// for every craft in the game - the box is a code literal at a single call site,
/// so `<Misc>`'s hull dimensions do not enter it. They reach the *collider*
/// instead, scaled by `0.75` a few lines earlier in the same constructor.
///
/// A function rather than a constant only because `f32` division is not `const`;
/// it is a pure expression over three constants and folds away.
#[must_use]
pub fn ship_inertia() -> Vec3 {
    Vec3::new(
        1.0 / PITCH_INVERSE_INERTIA,
        1.0 / YAW_INVERSE_INERTIA,
        1.0 / ROLL_INVERSE_INERTIA,
    )
}

/// The world outside the ship, as the force law sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    /// The track spline sample nearest the ship, and the one after it.
    ///
    /// These are what the original's `AiTrack_LocatePosition` (`0x0887ce78`)
    /// writes into the ship entity at `+0xaf0` and `+0xb60`, and the only consumer
    /// is [`crate::maglock`]: the magstrip hold slaves the ship's attitude to
    /// `unit(-down)` blended between the two, and the mag-floor probe casts along
    /// a direction built from the first one's `down` axis. Off a magstrip nothing
    /// reads them, so a caller with no track data loses nothing else.
    ///
    /// **This replaces the old `track_up` field**, which held the same axis in a
    /// form the hold cannot use: the surface *point* is needed as well, and the
    /// second sample is what makes the axis turn continuously along the track
    /// rather than stepping between sections. Gravity still acts on world `.y`
    /// only and neither of these is a gravity direction.
    pub track_sample: Option<crate::maglock::TrackSample>,
    /// The spline sample after [`Self::track_sample`], when there is one.
    ///
    /// `None` is the original's `-1024.0` sentinel at the second record's `+0x40`,
    /// which `AiTrack_UpdateCursor` writes when it has no second sample to report.
    pub track_sample_next: Option<crate::maglock::TrackSample>,
    /// The per-speed-class scale on `normal_gravity`, `g_class_gravity_scale`
    /// (`0x08ab0dcc`).
    ///
    /// **Read off the disc now, not a placeholder.** It is
    /// `<GlobalClass><GravityMul airborne/>` in the engine-wide
    /// `Data\XML\HandlingStats.xml`, and the table has one writer and two
    /// readers, all three read. Confidence **90**.
    ///
    /// **The XML attribute is named `airborne` and this scales the *grounded*
    /// term.** The gravity site's VFPU pair chain puts the table value on the lane
    /// carrying `normal_gravity * grounded` and a literal `1.0` on the airborne
    /// lane; see `oag_formats::handling::GravityMul`, which reads the chain out
    /// instruction by instruction. Do not "fix" this onto `flight_gravity`.
    ///
    /// An input rather than something this crate looks up, for the same reason
    /// [`Self::auto_speed`] is: it belongs to the speed class, and this crate does
    /// not otherwise know classes exist. The default is the identity, which is
    /// also what a caller that cannot read the file should pass - a zero here
    /// leaves a grounded craft weightless.
    pub class_gravity_scale: f32,
    /// The ship's own collider index, so its probes do not hit itself.
    pub self_collider: Option<u32>,
    /// Zone mode's auto-speed target, replacing the throttle entirely.
    ///
    /// `Some` selects the four-corner branch of `Ship_UpdateEngine`
    /// (`0x0884c834`): the value is used as the engine's output directly, the
    /// throttle is not read, and the `0.5 * speed + accelcap` clamp does **not**
    /// apply. The caller computes it as `base + step * zone` from the disc's own
    /// `<Zone start increment/>`; see `oag_race::zone::thrust`.
    ///
    /// An input rather than something this crate derives, for the same reason
    /// [`Self::class_gravity_scale`] is: the selector is a game mode, and this
    /// crate does not know modes exist. `None` is the ordinary throttle path and
    /// is bit-for-bit what it always was, which is what keeps the determinism
    /// reference unmoved.
    pub auto_speed: Option<f32>,
    /// The push direction of the speed pad the ship is inside this tick.
    ///
    /// `Some` on **every** tick the hull is inside a pad's trigger volume, not
    /// only the tick it enters one: `Ship_ApplySpeedupPad` (`0x08848f9c`) re-arms
    /// the timer each time, so an entry-only signal would cut the boost short on
    /// anything but the fastest crossing. See [`ShipState::pad_timer`].
    ///
    /// The vector is the pad's own local `+Z` axis in world space - row 2 of its
    /// `.vex` node matrix, which is why the boost pushes along the track: the pads
    /// are authored aligned with it. Expected to be unit length; nothing here
    /// normalises it, so a caller that hands over an unnormalised direction gets a
    /// boost scaled by its length.
    ///
    /// An input rather than something this crate derives, for the same reason
    /// [`Self::auto_speed`] is: finding it means owning the track's pad volumes,
    /// and this crate does not know tracks exist. `oag_formats::pads` does the
    /// geometry and `oag_game::race` runs the test.
    pub pad_hit: Option<Vec3>,
    /// The race's `Weapons` and `Damage` options, which decide what a wall costs
    /// the energy pool.
    ///
    /// An input rather than something this crate derives, for the same reason
    /// [`Self::auto_speed`] is: `Race_ReadSetupOptions` (`0x08896b84`) lowers them
    /// out of the race setup, and this crate does not know races have setups. The
    /// default is both on, which is the original's default for every game mode
    /// except three. See [`crate::damage`].
    pub damage_rules: crate::damage::DamageRules,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            track_sample: None,
            track_sample_next: None,
            class_gravity_scale: 1.0,
            self_collider: None,
            auto_speed: None,
            pad_hit: None,
            damage_rules: crate::damage::DamageRules::default(),
        }
    }
}

/// The four accumulators the original keeps on the craft, zeroed every frame.
///
/// `craft+0x320`, `+0x330`, `+0x340` and `+0x350`, drained into the rigid body at the
/// bottom of `Ship_UpdateCraft`. Reproduced as four separate vectors rather than
/// collapsed into one, because which accumulator a term writes is *evidence*: it is
/// how the local and world frames were told apart in the first place, and collapsing
/// them would throw that away.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Accumulators {
    /// Body-local force. Engine thrust and lift, and lateral grip.
    pub local_force: Vec3,
    /// World force. Brakes, the airbrake slide and lateral terms, gravity, drag,
    /// rolling resistance, the hover downforce and vertical damping.
    pub world_force: Vec3,
    /// Body-local **torque**. Steering yaw, pitch, airbrake yaw, bank-to-yaw and
    /// angular damping.
    ///
    /// **Torque, and that is settled rather than picked.** An earlier revision of
    /// this crate treated both angular accumulators as angular *acceleration* and
    /// said so - "a pick awaiting M3" - on the grounds that nothing in any term
    /// visibly divides by an inertia, that `docs/physics/README.md` calls this
    /// `angAccelLocal`, and that the damping term's shape is dimensionally an
    /// acceleration. `Body_Integrate` settles it the other way at confidence 88:
    /// `craft+0x340` reaches `body+0x120` and integrates into `body+0x160` with
    /// **no inertia and no mass divide**, four instructions after the linear half
    /// does scale by `invMass`. That asymmetry is the tell, `body+0x160` is
    /// angular momentum, and `dL/dt = torque`.
    ///
    /// Two consequences worth having in one place, because they are what changes
    /// when this reading is applied rather than merely recorded:
    ///
    /// - The damping term is `(-pitch_damping, -5, -2) * L`, not `* omega` -
    ///   `Ship_ApplyAngularDamping` loads `body+0x160`. So
    ///   [`crate::passive::angular_damping`] takes the body-local **momentum**,
    ///   and the "drive and damping share one accumulator so the inertia
    ///   cancels" argument is retired: it cancels only if the damping reads
    ///   `omega`, and it does not.
    /// - The inertia tensor now reaches every term here, where under the
    ///   acceleration reading it reached none of them (the drain multiplied by
    ///   exactly what the integrator divided by).
    pub local_angular: Vec3,
    /// World-space **torque**. The surface-alignment torque and the weathervane
    /// torque.
    ///
    /// Torque for the same reason [`Accumulators::local_angular`] is: `craft+0x350`
    /// reaches `body+0x130` and integrates into `body+0x160` through
    /// `basis^T * worldTorque` at `0x0884e35c`, the same path the local drives
    /// take with one extra change of basis. [`drain`] folds it into the body frame
    /// and hands the sum to the body as one torque.
    pub world_angular: Vec3,
}

/// Everything one force evaluation produced, for inspection.
///
/// The forces have already been accumulated onto the body; this is what a test or a
/// debug overlay wants to look at afterwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluated {
    /// The four accumulators, as drained onto the body.
    pub accumulators: Accumulators,
    /// The air cushion's output.
    pub hover: Hover,
    /// What the magstrip hold did, or `None` on ordinary track.
    ///
    /// Not a force and not in the accumulators: it is a kinematic rewrite of the
    /// body, reported here so a caller can see it happened. See
    /// [`crate::maglock`].
    pub mag_lock: Option<maglock::Hold>,
    /// The airbrake path's output, excluding lateral grip and the sideshift.
    pub airbrake: AirbrakeForces,
    /// The engine's body-local contribution.
    pub engine: engine::EngineForce,
    /// The brake's world-space force.
    pub brakes: Vec3,
    /// Lateral grip, as a body-local force.
    pub lateral_grip: Vec3,
    /// Gravity, as a world-space force.
    pub gravity: Vec3,
    /// Quadratic drag, as a world-space force.
    pub drag: Vec3,
    /// Rolling resistance, as a world-space force.
    pub rolling_resistance: Vec3,
    /// Vertical damping, as a world-space force.
    pub vertical_damping: Vec3,
    /// The speed-pad boost, as a world-space force. Zero on most ticks.
    pub speedup_pad: Vec3,
    /// The weathervane torque, world angular.
    pub weathervane: Vec3,
    /// Angular damping, body-local angular.
    pub angular_damping: Vec3,
    /// Body-local yaw from the steering.
    pub steering: f32,
    /// Body-local pitch from the pitch axis.
    pub pitch: f32,
    /// `dot(velocity, forward)`, signed, which four terms branch on.
    pub forward_speed: f32,
    /// The groundedness every control term saw: **last** frame's.
    pub control_grounded: f32,
    /// The groundedness lateral grip and the weathervane saw: this frame's.
    pub contact_grounded: f32,
    /// What the wall constraint did, **after** the integrator ran.
    ///
    /// Left at its default by [`evaluate`], which runs before the body has moved
    /// and so cannot know: [`crate::integrate::step`] fills it in. A caller that
    /// drives `evaluate` directly therefore sees "no wall response", which is the
    /// truth for that caller rather than a missing value.
    pub wall: crate::wall::WallResponse,
    /// What this frame did to the energy pool, **after** the wall constraint ran.
    ///
    /// Left at its default by [`evaluate`] for the same reason [`Self::wall`] is:
    /// the pool is charged from the contact impulses, which do not exist until
    /// the body has moved. Both fields are edges, so a caller that reads this on
    /// the tick after the one that produced it reads zeroes. See
    /// [`crate::damage::Shield`].
    pub shield: crate::damage::Shield,
}

/// Evaluates every force for one frame, at full `dt`, exactly once.
///
/// Mutates `state`: the control states, the contact bookkeeping, the accumulators on
/// the body, and the one thing the original applies outside the accumulators
/// entirely - the penetration-escape teleport.
///
/// The body's accumulators are **not** cleared here. [`crate::integrate::step`]
/// clears them at the top of the frame, so that after a step they still hold what was
/// applied.
///
/// The term order below follows `Ship_UpdateCraft`'s fifteen steps. Order does not
/// change a sum of forces, and it is kept anyway: it is what decides which
/// groundedness each term sees, and a reader comparing this against the evidence page
/// should not have to reorder anything in their head.
pub fn evaluate<R: Raycaster + ?Sized>(
    state: &mut ShipState,
    input: &ShipControls,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    dt: f32,
) -> Evaluated {
    // Last frame's contacts. Every control term reads this, and it is bound here,
    // once, before anything can overwrite it.
    let control_grounded = state.grounded;
    state.grounded_prev = control_grounded;
    let control_contact = control_grounded > 0.0;

    controls::update(state, input, handling, dt);

    let forward = state.body.forward();
    let up = state.body.up();
    let velocity = state.body.linear_velocity;
    let forward_speed = velocity.dot(forward);

    // `craft+0x2ec`, the cached speed every term below reads, is the **absolute**
    // value: `Ship_UpdateCraft` runs `vdot.t` at `0x0884992c` and `vabs.s` at
    // `0x08849930` before storing it, and that store is the field's only writer on
    // a craft base. Binding it here, once, is what the original does. It is a
    // no-op for a ship moving forwards; what it changes is that a reversing ship
    // gets drag and rolling resistance that oppose it rather than assist it.
    let cached_speed = forward_speed.abs();

    let mut acc = Accumulators::default();

    // 1. Engine, into the local force accumulator.
    let engine_force = engine::engine(
        state,
        handling,
        control_grounded,
        cached_speed,
        env.auto_speed,
    );
    acc.local_force += engine_force.as_local_force();

    // 2. Brakes. Called only while the contact flag is set, so groundedness gates the
    //    whole term rather than scaling it: braking does nothing in the air.
    let brakes = if control_contact {
        engine::brakes(state, handling)
    } else {
        Vec3::ZERO
    };
    acc.world_force += brakes;

    // 3. Airbrakes, and the sideshift in the same function's tail. The sideshift
    //    calls `Body_AddForceWorld` directly rather than going through the craft
    //    accumulator, which lands in the same place; it is gated on the contact
    //    flag, and hover has not run yet, so that flag is **last** frame's.
    let airbrake = airbrake::evaluate(state, input, handling, cached_speed);
    acc.world_force += airbrake.world_force;
    acc.local_angular += airbrake.local_angular;

    airbrake::advance_sideshift(state, input, dt);
    let sideshift = airbrake::sideshift_force(state, handling, control_grounded);
    acc.world_force += sideshift;

    // The barrel roll's own timers. Nothing here yet turns a real d-pad press
    // or a steering-axis crossing into [`barrel_roll::record_tap`]/[`barrel_roll::arm`]
    // calls - `ShipControls` carries no such event - so [`ShipState::roll_taps`]
    // never advances past `[0, 0, 0]` today. The ramp and the payout countdown
    // still run every tick, so a caller that does start recording taps needs no
    // further change here. See `crate::barrel_roll`.
    barrel_roll::advance_tap_timer(state, dt);
    barrel_roll::advance_phase(state, handling.roll_speed, dt);
    state.roll_payout_timer = (state.roll_payout_timer - dt).max(0.0);

    // 4. Steering and 5. pitch, both body-local angular.
    let steering = engine::steering(state, handling);
    let pitch = engine::pitch(input, handling, control_contact);
    acc.local_angular.y += steering;
    acc.local_angular.x += pitch;

    // 6. Quadratic drag and 7. gravity.
    let drag = passive::quadratic_drag(velocity, cached_speed, control_contact);
    let gravity = passive::gravity(
        handling,
        state.body.mass,
        env.class_gravity_scale,
        control_grounded,
    );
    acc.world_force += drag;
    acc.world_force += gravity;

    // 8. Hover. Its load factor reads last frame's groundedness through
    //    `grounded_prev`, and it is what produces this frame's contact count.
    //
    //    The target height is bound *before* the hover call and reused by the mag
    //    lock below, because the original builds `craft+0x2f0` once at the top of
    //    `Ship_UpdateCraft` from the previous frame's blend and both consumers read
    //    that one value. Recomputing it after the ramp would give the reposition a
    //    target the spring never saw.
    let target_height = hover::target_height(handling, state.mag_lock_blend, state.leap_timer);
    let hover = hover::evaluate(state, handling, env, raycaster, target_height);
    for probe in &hover.probes {
        if probe.contact {
            state.body.add_force_at_point(probe.force, probe.point);
        }
    }
    acc.world_force += hover.downforce;
    acc.local_angular += hover.local_angular_torque;
    acc.world_angular += hover.alignment_torque;

    // From here on, this frame's contacts.
    state.grounded = ShipState::quantise_grounded(hover.contacts);
    // **The landing response is armed in the air, not on touchdown**, and only
    // once the flight has outlasted `rebound_jump_time`. `Ship_UpdateCraft`
    // (`0x08849df0`) keeps `craft+0x284` as the airborne clock, zeroed the
    // moment anything touches; `Ship_HoverTwoPoint` then does the arming. A hop
    // shorter than the parameter never arms it, so the craft comes back down on
    // the ordinary `rebound` - which is what makes the `landing_rebound` bounce
    // an event rather than something that fires on every flicker of contact.
    // Confidence 95, from the disassembly. See `ShipState::time_airborne`.
    if state.grounded > 0.0 {
        state.time_airborne = 0.0;
        state.time_since_landing += dt;
    } else {
        if handling.antigrav.rebound_jump_time < state.time_airborne {
            state.time_since_landing = 0.0;
        }
        state.time_airborne += dt;
    }
    let contact_grounded = state.grounded;
    let contact = contact_grounded > 0.0;

    // The barrel roll's landing payout: resolve the self-completing ramp on
    // this tick's airborne-to-grounded transition and, if the roll finished,
    // arm [`ShipState::roll_payout_timer`] for `<Special roll_turbotime>`.
    //
    // Runs after hover, so on the exact landing tick the hover spring above
    // already used the *pre*-arm value of the timer and only sees the payout
    // from next tick on; lateral grip below, which runs after this point,
    // sees it immediately. Which of the original's own steps this arming
    // happens in relative to hover and lateral grip was not traced - see
    // `crate::barrel_roll`'s module docs for why this crate ties the release
    // to this transition at all.
    if contact && control_grounded <= 0.0 && barrel_roll::release(state) {
        state.roll_payout_timer = handling.roll_turbotime;
    }

    // Still step 8: `Ship_UpdateHover` runs one of the two hover models and then
    // `Ship_UpdateMagLock` unconditionally. It is **not** a force term - it writes
    // the body's position, velocity and basis directly and touches no accumulator,
    // which is why it appears here as a statement rather than as a summand. See
    // `crate::maglock` for why routing it through a torque would be wrong.
    let mag_contact = maglock::probe(state, env, raycaster);
    let mag_lock = maglock::update(state, env, mag_contact, target_height);

    // 9. Lateral grip, into the *local* force accumulator, and only once both the
    //    collision stun and the leap timer have expired.
    //
    //    `Ship_ApplyLateralGrip` opens with `if (craft+0x290 > 0) { craft+0x290 -=
    //    dt; return; }`, so the stun both suppresses the grip and is what counts it
    //    down. Decrementing here rather than with the control ramps is deliberate:
    //    the engine ran at step 2 and has already read the pre-decrement value, which
    //    is the original's ordering. See `ShipState::stun_timer`.
    let lateral_grip = if state.stun_timer > 0.0 {
        state.stun_timer = (state.stun_timer - dt).max(0.0);
        Vec3::ZERO
    } else if state.leap_timer > 0.0 {
        Vec3::ZERO
    } else {
        airbrake::lateral_grip(state, handling, contact_grounded)
    };
    acc.local_force += lateral_grip;

    // 11. Weathervane, 12. angular damping, 13. rolling resistance, 14. vertical
    //     damping. Step 10 is the dead in-air roll-levelling branch and is
    //     deliberately not ported.
    let weathervane = passive::weathervane(forward, velocity, contact);
    // `Ship_ApplyAngularDamping` loads `body+0x160`, which is angular **momentum**,
    // and multiplies it by `(-pitch_damping, -5, -2)`. So what the damping reads is
    // `I * omega` in the body frame, not `omega` - and that difference is exactly
    // why the inertia survives to the observable instead of cancelling against the
    // drive. See `Accumulators::local_angular`.
    let local_angular_velocity = state.body.orientation.inverse() * state.body.angular_velocity;
    let local_angular_momentum = local_angular_velocity * state.body.inertia;
    let angular_damping = passive::angular_damping(handling, local_angular_momentum);
    let rolling_resistance = passive::rolling_resistance(velocity, cached_speed);
    let vertical_damping = passive::vertical_damping(up, velocity, contact_grounded);

    acc.world_angular += weathervane;
    acc.local_angular += angular_damping;
    acc.world_force += rolling_resistance;
    acc.world_force += vertical_damping;

    // 15. The speed-pad boost, the last of the fifteen. Its own timer, so it runs
    //     on long after the tick that armed it.
    //
    //     `up` is the body's own up row, bound at the top of this function and
    //     shared with the vertical damping term. It is `craft+0x160`, the hull up
    //     axis the original's `<Special speedpad_jump>` branch tilts toward - dot
    //     `+1.000000` against `body+0x010` on a live capture.
    let speedup_pad = engine::speedup_pad(state, input, handling, up, env.pad_hit, dt);
    acc.world_force += speedup_pad;

    drain(state, &acc);

    // Outside the accumulators, deliberately: the penetration-escape teleport,
    // which moves the body without touching its velocity.
    state.body.position += hover.escape;

    Evaluated {
        accumulators: acc,
        hover,
        mag_lock,
        airbrake,
        engine: engine_force,
        brakes,
        lateral_grip,
        gravity,
        drag,
        rolling_resistance,
        vertical_damping,
        speedup_pad,
        weathervane,
        angular_damping,
        steering,
        pitch,
        forward_speed,
        control_grounded,
        contact_grounded,
        wall: crate::wall::WallResponse::default(),
        shield: crate::damage::Shield::default(),
    }
}

/// Drains the four accumulators onto the body.
///
/// The two force accumulators hold **forces**: gravity carries `mass` explicitly and
/// no other term does, exactly as in the original, so nothing here multiplies by
/// mass. The two angular accumulators hold **torque** - see
/// [`Accumulators::local_angular`] - so nothing here multiplies by the inertia
/// either. The world one is rotated into the body, summed with the local one, and
/// the total is handed back out as a world-space torque, which is
/// [`crate::integrate`]'s input.
///
/// **The inertia used to be applied here and divided out again by the
/// integrator.** That round trip is what made [`crate::ship::Body::inertia`] a
/// no-op for every accumulator term, and removing it is what makes the tensor
/// mean something on every angular path at once rather than on the one axis a
/// constant was applied to.
fn drain(state: &mut ShipState, acc: &Accumulators) {
    let orientation = state.body.orientation;

    state.body.add_force(orientation * acc.local_force);
    state.body.add_force(acc.world_force);

    let mut local_angular = acc.local_angular;
    local_angular += orientation.inverse() * acc.world_angular;
    state.body.add_torque(orientation * local_angular);
}

#[cfg(test)]
mod tests;
