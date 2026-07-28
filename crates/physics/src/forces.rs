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
//! Confidence 84, from `docs/ghidra/functions/psp-pulse/engine.md`. The obvious
//! reimplementation - resolve contacts, then apply forces - gets a different answer
//! on every takeoff and landing frame, in five terms simultaneously. So [`evaluate`]
//! binds the two values to separately named locals and every call site takes one
//! explicitly; no term reads groundedness out of the state for itself.
//!
//! # What is here and what is deliberately absent
//!
//! Present: the control force law from
//! `docs/ghidra/functions/psp-pulse/engine.md` (engine, brakes, steering, pitch and
//! the passive terms), the two-probe air cushion from `docs/physics/README.md`, and
//! the airbrakes with lateral grip and the sideshift.
//!
//! Present too, and not a force: the magstrip hold ([`crate::maglock`]), which
//! runs inside step 8 and writes the body directly.
//!
//! **Absent**, each because implementing it would mean inventing a trigger nobody
//! has decoded: the track-section force (`FUN_08848f9c`, a guess at confidence 45),
//! the four-corner hover variant and the auto-speed law behind its selector, turbo,
//! and every flag-gated engine and steering variant. See
//! `docs/physics/README.md`'s "what is implemented" section for the full list.

use oag_core::math::Vec3;

use crate::airbrake::{self, AirbrakeForces};
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
/// which `docs/ghidra/functions/psp-pulse/rigid-body.md` establishes is `L`, and
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
    /// The per-speed-class scale on `normal_gravity`, from the table at
    /// `0x08ab0dcc`.
    ///
    /// **The table's values were not read**, so the identity is the default, and it
    /// is an input rather than a constant because it belongs to the speed class,
    /// which this crate does not otherwise know about. Confidence 78 that the table
    /// exists and is indexed by class; what is in it is a guess awaiting M3.
    pub class_gravity_scale: f32,
    /// The ship's own collider index, so its probes do not hit itself.
    pub self_collider: Option<u32>,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            track_sample: None,
            track_sample_next: None,
            class_gravity_scale: 1.0,
            self_collider: None,
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
}

/// Evaluates every force for one frame, at full `dt`, exactly once.
///
/// Mutates `state`: the control states, the contact bookkeeping, the accumulators on
/// the body, and the two things the original applies outside the accumulators
/// entirely - the sideshift's velocity change and the penetration-escape teleport.
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
    let engine_force = engine::engine(state, handling, control_grounded, cached_speed);
    acc.local_force += engine_force.as_local_force();

    // 2. Brakes. Called only while the contact flag is set, so groundedness gates the
    //    whole term rather than scaling it: braking does nothing in the air.
    let brakes = if control_contact {
        engine::brakes(state, handling)
    } else {
        Vec3::ZERO
    };
    acc.world_force += brakes;

    // 3. Airbrakes. The sideshift they can also produce goes straight to the body,
    //    below, rather than through any accumulator.
    let airbrake = airbrake::evaluate(state, input, handling, cached_speed);
    acc.world_force += airbrake.world_force;
    acc.local_angular += airbrake.local_angular;

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
    if control_grounded == 0.0 && state.grounded > 0.0 {
        state.time_since_landing = 0.0;
    } else {
        state.time_since_landing += dt;
    }
    let contact_grounded = state.grounded;
    let contact = contact_grounded > 0.0;

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
    //     deliberately not ported; step 15 is the track-section force, which is not
    //     implemented.
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

    drain(state, &acc);

    // Outside the accumulators, both of them deliberately.
    state.body.linear_velocity += airbrake::sideshift_velocity_delta(state, input, handling);
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
        weathervane,
        angular_damping,
        steering,
        pitch,
        forward_speed,
        control_grounded,
        contact_grounded,
        wall: crate::wall::WallResponse::default(),
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
mod tests {
    use super::*;
    use crate::collide::{CollisionWorld, Surface, TriangleSoup};
    use crate::params::{Antigrav, Brakes, Engine, Physical};
    use crate::ship::Body;

    fn flat_floor() -> CollisionWorld {
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [-500.0, 0.0, -500.0],
                [-500.0, 0.0, 500.0],
                [500.0, 0.0, 0.0],
            ],
            vec![[0, 1, 2]],
            Vec::new(),
            Surface::Floor,
            0,
        ));
        world
    }

    /// Arbitrary round numbers, in the scaled in-memory form. **Not recovered
    /// values.**
    fn test_handling() -> Handling {
        Handling {
            antigrav: Antigrav {
                ride_height: 6.0,
                rebound: 1.0,
                ..Antigrav::default()
            },
            physical: Physical {
                mass: 1.0,
                normal_gravity: 10.0,
                flight_gravity: 4.0,
                ..Physical::default()
            },
            ..Handling::ZERO
        }
    }

    fn ship_at(height: f32) -> ShipState {
        ShipState {
            body: Body {
                position: Vec3::new(0.0, height, 0.0),
                ..Body::default()
            },
            ..ShipState::default()
        }
    }

    /// The recovered tensor, pinned as the three numbers the binary computes.
    ///
    /// Not a test of this crate's own arithmetic: the diagonal `(15.6, 21.6, 15.6)`
    /// is what `scripts/trace-angular-fit.py` independently measured off two real
    /// captures as `~(15, 21..22, 14..16)`, so these three values are the point of
    /// contact between an instruction read and a hardware measurement. If someone
    /// edits [`INERTIA_BOX_X`] or [`INERTIA_MASS`] this is what says the tensor no
    /// longer matches what the original's own recordings show.
    #[test]
    fn the_recovered_inertia_tensor_is_a_solid_box() {
        let xx =
            12.0 / (INERTIA_MASS * (INERTIA_BOX_Y * INERTIA_BOX_Y + INERTIA_BOX_Z * INERTIA_BOX_Z));
        let zz =
            12.0 / (INERTIA_MASS * (INERTIA_BOX_X * INERTIA_BOX_X + INERTIA_BOX_Y * INERTIA_BOX_Y));

        // The box is square in plan, so pitch and roll inertia are equal and the
        // yaw one is the odd axis out - the `x == z` symmetry the fit also found.
        assert_eq!(xx, zz);
        assert!(
            YAW_INVERSE_INERTIA < xx,
            "yaw must be the hardest axis to turn"
        );

        // (15.6, 21.6, 15.6), to a tolerance far tighter than the fit's own spread.
        assert!((1.0 / xx - 15.6).abs() < 0.01, "I_xx was {}", 1.0 / xx);
        assert!(
            (1.0 / YAW_INVERSE_INERTIA - 21.6).abs() < 0.01,
            "I_yy was {}",
            1.0 / YAW_INVERSE_INERTIA
        );
    }

    /// The staleness invariant, asserted where it is observable: on the frame a ship
    /// first touches down, gravity must still be the **airborne** one even though the
    /// ship ends that frame grounded.
    #[test]
    fn the_frame_a_ship_lands_uses_airborne_gravity_and_grounded_grip() {
        let handling = test_handling();
        let world = flat_floor();
        let mut state = ship_at(4.0);
        state.body.linear_velocity = Vec3::new(3.0, -1.0, 0.0);
        state.grounded = 0.0;

        let evaluated = evaluate(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        assert_eq!(evaluated.control_grounded, 0.0);
        assert_eq!(evaluated.contact_grounded, 1.0);
        assert_eq!(state.grounded, 1.0);
        // `flight_gravity` is 4 and `normal_gravity` is 10, so this is unambiguous.
        assert_eq!(evaluated.gravity.y, -handling.physical.flight_gravity);
    }

    /// And the reverse on the frame it leaves the ground: gravity is still the
    /// grounded one.
    #[test]
    fn the_frame_a_ship_takes_off_uses_grounded_gravity() {
        let handling = test_handling();
        let world = flat_floor();
        // Above `ride_height`, so no probe can reach the floor this frame.
        let mut state = ship_at(handling.antigrav.ride_height + 5.0);
        state.grounded = 1.0;

        let evaluated = evaluate(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        assert_eq!(evaluated.control_grounded, 1.0);
        assert_eq!(evaluated.contact_grounded, 0.0);
        assert_eq!(evaluated.gravity.y, -handling.physical.normal_gravity);
    }

    /// The engine reads the same stale value, which is what makes a landing frame
    /// give a ship 20 % thrust even though it ends that frame on the ground.
    #[test]
    fn the_engine_reads_the_stale_groundedness_too() {
        let handling = Handling {
            engine: Engine {
                accelcap: 1000.0,
                amount: 0.4,
                ..Engine::default()
            },
            ..test_handling()
        };
        let world = flat_floor();
        let full_throttle = ShipControls {
            thrust: 1.0,
            ..ShipControls::default()
        };

        let mut landing = ship_at(4.0);
        landing.grounded = 0.0;
        let landing_eval = evaluate(
            &mut landing,
            &full_throttle,
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        let mut settled = ship_at(4.0);
        settled.grounded = 1.0;
        let settled_eval = evaluate(
            &mut settled,
            &full_throttle,
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        assert_eq!(landing_eval.contact_grounded, settled_eval.contact_grounded);
        assert!(
            landing_eval.engine.thrust < settled_eval.engine.thrust,
            "landing thrust {} was not below settled thrust {}",
            landing_eval.engine.thrust,
            settled_eval.engine.thrust
        );
    }

    /// Braking is gated on the stale contact flag, not scaled by it, so a ship that
    /// was airborne last frame gets no brake force at all this frame.
    #[test]
    fn braking_does_nothing_in_the_air() {
        let handling = Handling {
            brakes: Brakes {
                amount: -0.5,
                gain: 400.0,
                falloff: 200.0,
            },
            ..test_handling()
        };
        let world = flat_floor();
        let held = ShipControls {
            airbrake_left: 1.0,
            airbrake_right: 1.0,
            ..ShipControls::default()
        };

        let mut airborne = ship_at(4.0);
        airborne.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
        airborne.brake = 100.0;
        airborne.grounded = 0.0;
        let airborne_eval = evaluate(
            &mut airborne,
            &held,
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );
        assert_eq!(airborne_eval.brakes, Vec3::ZERO);

        let mut grounded = ship_at(4.0);
        grounded.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
        grounded.brake = 100.0;
        grounded.grounded = 1.0;
        let grounded_eval = evaluate(
            &mut grounded,
            &held,
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );
        assert!(grounded_eval.brakes.z > 0.0);
    }

    /// The leap timer suppresses lateral grip entirely while it runs.
    #[test]
    fn lateral_grip_is_suppressed_while_the_leap_timer_runs() {
        let handling = Handling {
            antigrav: Antigrav {
                grip_ground: 2.0,
                grip_air: 1.0,
                ..test_handling().antigrav
            },
            ..test_handling()
        };
        let world = flat_floor();

        let mut sliding = ship_at(4.0);
        sliding.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
        sliding.grounded = 1.0;
        let free = evaluate(
            &mut sliding,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );
        assert_ne!(free.lateral_grip, Vec3::ZERO);

        let mut leaping = ship_at(4.0);
        leaping.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
        leaping.grounded = 1.0;
        leaping.leap_timer = 1.0;
        let held = evaluate(
            &mut leaping,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );
        assert_eq!(held.lateral_grip, Vec3::ZERO);
    }

    /// The collision stun suppresses lateral grip too, and is what counts itself
    /// down - `Ship_ApplyLateralGrip` owns the decrement.
    #[test]
    fn the_collision_stun_suppresses_lateral_grip_and_counts_itself_down() {
        let handling = Handling {
            antigrav: Antigrav {
                grip_ground: 2.0,
                grip_air: 1.0,
                ..test_handling().antigrav
            },
            ..test_handling()
        };
        let world = flat_floor();
        let dt = 1.0 / 60.0;

        let mut stunned = ship_at(4.0);
        stunned.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
        stunned.grounded = 1.0;
        stunned.stun_timer = 0.5;

        let held = evaluate(
            &mut stunned,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            dt,
        );
        assert_eq!(held.lateral_grip, Vec3::ZERO);
        assert_eq!(stunned.stun_timer, 0.5 - dt);
    }

    /// The stun must actually expire, and stop at zero rather than going negative -
    /// a negative timer would read as "not stunned" but is a trap for anything that
    /// later tests the sign.
    #[test]
    fn the_collision_stun_stops_at_zero_and_grip_returns() {
        let handling = Handling {
            antigrav: Antigrav {
                grip_ground: 2.0,
                grip_air: 1.0,
                ..test_handling().antigrav
            },
            ..test_handling()
        };
        let world = flat_floor();
        let dt = 1.0 / 60.0;

        let mut ship = ship_at(4.0);
        ship.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
        ship.grounded = 1.0;
        ship.stun_timer = dt * 0.5;

        let during = evaluate(
            &mut ship,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            dt,
        );
        assert_eq!(during.lateral_grip, Vec3::ZERO);
        assert_eq!(ship.stun_timer, 0.0, "clamped rather than negative");

        ship.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
        ship.grounded = 1.0;
        let after = evaluate(
            &mut ship,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            dt,
        );
        assert_ne!(after.lateral_grip, Vec3::ZERO, "grip must come back");
    }

    /// The world force accumulator holds forces, not accelerations: gravity is the
    /// only term carrying `mass`, so doubling the mass must not double the drag.
    #[test]
    fn only_gravity_scales_with_mass() {
        let handling = test_handling();
        let world = flat_floor();

        let mut light = ship_at(4.0);
        light.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
        light.grounded = 1.0;
        let light_eval = evaluate(
            &mut light,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        let mut heavy = ship_at(4.0);
        heavy.body.mass = 2.0;
        heavy.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
        heavy.grounded = 1.0;
        let heavy_eval = evaluate(
            &mut heavy,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        assert_eq!(heavy_eval.gravity, light_eval.gravity * 2.0);
        assert_eq!(heavy_eval.drag, light_eval.drag);
        assert_eq!(heavy_eval.rolling_resistance, light_eval.rolling_resistance);
    }

    /// The hover target is built from `ride_height`, which was previously believed
    /// not to reach the force law at all.
    #[test]
    fn the_hover_target_follows_ride_height() {
        let mut handling = test_handling();
        let world = flat_floor();

        let mut low = ship_at(4.0);
        low.grounded = 1.0;
        let low_eval = evaluate(
            &mut low,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        handling.antigrav.ride_height *= 2.0;
        let mut high = ship_at(4.0);
        high.grounded = 1.0;
        let high_eval = evaluate(
            &mut high,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        // Same height and the same everything else, but a higher target means more
        // lift.
        assert!(
            high_eval.hover.probes[0].force.y > low_eval.hover.probes[0].force.y,
            "{:?} was not above {:?}",
            high_eval.hover.probes[0].force,
            low_eval.hover.probes[0].force
        );
    }

    /// The end-to-end version of the steering sign fix: holding right must swing
    /// the nose toward the ship's own right axis over real ticks of `evaluate` and
    /// `integrate`, not away from it. A unit-level assertion on
    /// `engine::steering`'s return value alone would not catch a sign error
    /// introduced anywhere downstream in how the accumulators are drained onto the
    /// body, which is exactly the layer `docs/ghidra/functions/psp-pulse/engine.md`
    /// left as an open question until it was traced for this fix.
    #[test]
    fn holding_right_turns_the_ship_toward_its_own_right_axis() {
        let handling = Handling {
            turning: crate::params::Turning {
                amount: 0.02,
                gain: 400.0,
                falloff: 200.0,
            },
            ..Handling::ZERO
        };
        let world = flat_floor();
        // High enough above the floor that no hover probe ever makes contact, so
        // only steering is in play.
        let mut state = ship_at(1000.0);
        let controls = ShipControls {
            steer_x: 1.0,
            ..ShipControls::default()
        };
        let initial_right = state.body.right();

        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            crate::integrate::step(
                &mut state,
                &controls,
                &handling,
                &Environment::default(),
                &world,
                dt,
            );
        }

        assert!(
            state.body.forward().dot(initial_right) > 0.0,
            "forward was {:?}, expected a component toward the initial right axis {initial_right:?}",
            state.body.forward()
        );
    }

    /// The airbrake twin of the test above, and the one that would have caught
    /// Task #34's bug the day the steering sign was fixed.
    ///
    /// **`steer_x` is deliberately zero.** The only committed scenario that
    /// exercises the airbrake yaw term,
    /// `verification/scenarios/airbrake-asymmetric.inputs`, holds the brake and
    /// the steering on the same side, and the steering drive is several times the
    /// larger, so an inverted airbrake yaw still curves the run the right way and
    /// shows up only as a wrong rate. Holding one brake alone is what separates
    /// them: nothing else in the term list can yaw a ship flying level in a
    /// straight line, so the sign of the result is this term's sign.
    ///
    /// The direction asserted is the original's, not a preference:
    /// `angularLocal.y += fs * Airbrake.turn * (R - L) * 1e-3` about an accumulator
    /// whose positive sense is nose-right (the `w_game = -w_physics` convention),
    /// so `L > R` turns the nose left - toward the braked side, which is also what
    /// the game plays like.
    #[test]
    fn braking_the_left_airbrake_alone_turns_the_ship_toward_its_own_left() {
        let handling = Handling {
            airbrake: crate::params::Airbrake {
                turn: 3.0,
                gain: 800.0,
                falloff: 400.0,
                ..crate::params::Airbrake::default()
            },
            ..Handling::ZERO
        };
        let world = flat_floor();
        // Far above the floor, so no hover probe, no contact and no alignment
        // torque: the airbrake yaw is the only thing that can rotate this ship.
        let mut state = ship_at(1000.0);
        // The three airbrake terms all carry `craft+0x2ec` as a factor, so a
        // stationary ship gets nothing at all.
        state.body.linear_velocity = state.body.forward() * 40.0;

        let controls = ShipControls {
            airbrake_left: 1.0,
            steer_x: 0.0,
            ..ShipControls::default()
        };
        let initial_right = state.body.right();

        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            crate::integrate::step(
                &mut state,
                &controls,
                &handling,
                &Environment::default(),
                &world,
                dt,
            );
        }

        assert!(
            state.body.forward().dot(initial_right) < 0.0,
            "forward was {:?}; the left brake must swing it away from the initial \
             right axis {initial_right:?}, not toward it",
            state.body.forward()
        );
    }

    /// And the mirror, so that a term which somehow yawed left whatever it was
    /// given could not pass the test above.
    #[test]
    fn braking_the_right_airbrake_alone_turns_the_ship_toward_its_own_right() {
        let handling = Handling {
            airbrake: crate::params::Airbrake {
                turn: 3.0,
                gain: 800.0,
                falloff: 400.0,
                ..crate::params::Airbrake::default()
            },
            ..Handling::ZERO
        };
        let world = flat_floor();
        let mut state = ship_at(1000.0);
        state.body.linear_velocity = state.body.forward() * 40.0;

        let controls = ShipControls {
            airbrake_right: 1.0,
            ..ShipControls::default()
        };
        let initial_right = state.body.right();

        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            crate::integrate::step(
                &mut state,
                &controls,
                &handling,
                &Environment::default(),
                &world,
                dt,
            );
        }

        assert!(
            state.body.forward().dot(initial_right) > 0.0,
            "forward was {:?}, expected a component toward the initial right axis \
             {initial_right:?}",
            state.body.forward()
        );
    }
}
