//! One frame of force evaluation, and nothing else.
//!
//! # Why this is separate from the integrator
//!
//! `docs/physics/README.md` specifies the craft integrator as
//!
//! ```text
//! dt = clamp(measured_dt, 0, 0.06666)
//! evaluate all forces once at full dt
//! integrate 3 explicit Euler sub-steps of dt/3
//! ```
//!
//! **Forces are evaluated once at full `dt`, not once per sub-step**, which is not what a
//! textbook sub-stepped integrator does. The split is structural: this module has no
//! sub-step loop, and [`crate::integrate`] only sees a [`Body`](crate::ship::Body).
//!
//! # Groundedness is stale for every control term
//!
//! The most likely thing to get wrong. Hover is **step 8 of 15** in the original and
//! clears the contact flag on entry, so terms before it see the *previous* frame's
//! groundedness and terms after it see this frame's:
//!
//! | Reads | Sees |
//! | --- | --- |
//! | engine, brakes, pitch, quadratic drag, gravity, hover's own load factor | last frame |
//! | lateral grip, weathervane torque | this frame |
//!
//! Confidence 84, from `docs/ghidra/functions/psp-pulse-usa/engine.md`. Resolving contacts
//! then applying forces gets a different answer on every takeoff and landing frame in
//! five terms. [`evaluate`] binds the two values to separately named locals and every call
//! site takes one explicitly; no term reads groundedness out of the state itself.
//!
//! # What is here and what is absent
//!
//! Present: the control force law (`engine.md`), the two-probe air cushion
//! (`docs/physics/README.md`), the airbrakes with lateral grip and sideshift, and the
//! magstrip hold ([`crate::maglock`], not a force: it runs inside step 8 and writes the
//! body directly).
//!
//! **Absent**, because each would mean inventing an undecoded trigger: the four-corner
//! hover variant and the auto-speed law behind its selector, turbo, every flag-gated engine
//! and steering variant, and two arms of the speed-pad boost
//! ([`crate::engine::speedup_pad`]). See `docs/physics/README.md`, "what is implemented".

use oag_core::math::Vec3;

use crate::airbrake::{self, AirbrakeForces};
use crate::barrel_roll;
use crate::collide::Raycaster;
use crate::hover::{self, Hover};
use crate::launch;
use crate::maglock;
use crate::params::Handling;
use crate::ship::{ShipControls, ShipState};
use crate::{controls, engine, passive};

/// The box the ship's inverse inertia tensor is built from, `x`, on the body's right
/// axis. See [`YAW_INVERSE_INERTIA`]. A code literal in the ship-entity constructor, the
/// same for every craft.
///
/// **The extents are the hull's body-axis ones; the original applies the resulting
/// diagonal in *world* axes** (`0x0884e380`-`0x0884e39c`, measured exactly by
/// `scripts/trace-inertia-frame-fit.py`). This crate applies it in body axes, which
/// differs only while a craft is pitched or rolled (`diag(a, b, a)` is yaw-invariant).
/// Unmeasured against play; see `docs/ghidra/functions/psp-pulse-usa/rigid-body.md`.
pub const INERTIA_BOX_X: f32 = 12.0;

/// The body-space inertia box, `y`. See [`INERTIA_BOX_X`].
pub const INERTIA_BOX_Y: f32 = 8.0;

/// The body-space inertia box, `z`. See [`INERTIA_BOX_X`].
pub const INERTIA_BOX_Z: f32 = 12.0;

/// The mass the inertia tensor is built with, **not** the mass the ship flies with.
///
/// `FUN_08840c74` calls `Body_SetMass(body, 0.9)` at `0x08841414` then `Body_SetBoxInertia`
/// at `0x08841488`, so the tensor captures `0.9`. `Ship_UpdateCraft` calls `Body_SetMass`
/// again **every frame** (`0x0884985c`, from `Physical.mass` at class `+0xf4`) and nothing
/// recomputes the tensor, so rotational inertia is frozen at construction and decoupled
/// from translational mass. The captures measure this ([`YAW_INVERSE_INERTIA`]).
pub const INERTIA_MASS: f32 = 0.9;

/// The yaw entry of the ship's inverse inertia tensor, `1 / I_yy`.
///
/// The tensor is a diagonal fixed in **world** axes (corrected 2026-09-10,
/// `docs/physics/cornering-ground-truth.md`); that changes the frame a caller applies it
/// in, not this value. This crate applies it body-locally, deliberately.
///
/// **Recovered, not fitted**: it replaced `YAW_DRIVE_CALIBRATION`, a fitted `0.0452`
/// (`0.046296` recovered, `2.4 %` apart, inside the fits' own spread).
///
/// # The writer
///
/// `Body_SetBoxInertia` (`0x0884e1ac`, 26 instructions) zeroes `body+0x40..0x80` from the
/// constant at `0x08a907e0` and writes three diagonal entries as
/// `12 / (m * (y*y + z*z))` and its two permutations, `I_xx = m (y^2 + z^2) / 12` being
/// the solid cuboid exactly (`lui a1,0x4140` is the `12.0f`; mass from `0x374(a0)`;
/// results stored to `0x40`, `0x54`, `0x68`). One caller, `World_SetBodyBoxInertia`
/// (`0x0884e900`), itself called only from the ship-entity constructor `FUN_08840c74`
/// at `0x08841488` with the literal box `(12, 8, 12)` built at `0x08841470`-`0x0884148c`
/// from `0x41400000`, `0x41000000`, `0x41400000`.
///
/// # One constant for every ship
///
/// The box is a **code literal at a single call site**: `Misc` `width`/`length`/`height`
/// reach the *collider* (scaled by `0.75` a few lines earlier) and not the inertia.
///
/// # Agreement with the captures picks out the mass
///
/// With `(12, 8, 12)` and `m = 0.9` the tensor is `I = (15.6, 21.6, 15.6)` on `(right, up,
/// forward)`. `scripts/trace-angular-fit.py` fitted the recorded `body+0x160` column on two
/// captures to `~(15, 21..22, 14..16)`: the same `x == z` symmetry and ratio (`1.385`
/// against `1.413`), from a measurement that knew nothing of this function.
///
/// | tensor built with | `I_yy` | against the fit's `21.2` |
/// | --- | ---: | ---: |
/// | `m = 0.9` (the constructor's) | `21.6` | `1.9 %` |
/// | `m = 1.0` (`Body_Init`'s default) | `24.0` | `13 %` |
///
/// So the captures confirm both `Body_SetMass(0.9)` and [`INERTIA_MASS`]'s claim that the
/// tensor is frozen at construction.
///
/// # Scaling the yaw drive is the exact form
///
/// The original damps **angular momentum**: `Ship_ApplyAngularDamping` (`0x08848ed0`) loads
/// `body+0x160` at `0x08848f08` (`L`, per `rigid-body.md`) and multiplies by
/// `(-pitch_damping, -5, -2)`, so yaw is `dL/dt = drive - 5 L`, `omega = c L` with
/// `c = YAW_INVERSE_INERTIA`, hence `domega/dt = c * drive - 5 * omega`: same equilibrium,
/// `0.2 s` time constant and transient as scaling the drive by `c` with damping on
/// `omega`. The old "inertia cancels at equilibrium" argument assumed damping read
/// angular *velocity*.
///
/// # The whole crate is on the momentum model
///
/// Earlier, pitch and roll were left about `15.6x` too strong because no capture of a
/// pitch or roll input existed. Now `verification/scenarios/pitch-both-ways.inputs` and
/// `pitch-hold-thrust.inputs` put the pitch entry at `-15.620` against `-15.6` (`0.13 %`,
/// on a capture with `speed/|velocity| == 1.0000` every tick) and measure
/// `body+0x160 = I * body+0x150` at `100.0 %` on the pitch axis
/// ([angular-velocity-column.md](../../../docs/physics/angular-velocity-column.md)).
/// So [`Accumulators`] holds **torque** on both angular axes, [`crate::ship::Body::inertia`]
/// carries [`ship_inertia`], and [`crate::integrate`] divides by it. This constant is no
/// longer *applied* anywhere; it is the yaw entry [`ship_inertia`] builds, and the two must
/// agree. Yaw behaviour is unchanged by the move (same equation as above), so **a yaw
/// regression is a bug in the change, not physics**.
///
/// # Confidence
///
/// **92.** The writer, its single call site, the literal arguments and the mass ordering
/// were read with the Allegrex module and agree to `2 %` with an independent two-capture
/// fit on the one axis it constrains. One binary; the PS2 was not checked. (An earlier
/// textbook box for the Assegai hull gave `16.6`, 33 % out: evidence about the inputs, since
/// the hull is not the box.)
pub const YAW_INVERSE_INERTIA: f32 =
    12.0 / (INERTIA_MASS * (INERTIA_BOX_X * INERTIA_BOX_X + INERTIA_BOX_Z * INERTIA_BOX_Z));

/// The pitch entry of the ship's inverse inertia tensor, `1 / I_xx`, on the body's right
/// axis.
///
/// The `Body_SetBoxInertia` computation of [`YAW_INVERSE_INERTIA`] on the two perpendicular
/// extents: `12 / (m * (y^2 + z^2))` = `1 / 15.6`, in the binary's own form so all three
/// entries are one expression.
///
/// **Measured, not only read.** Two captures of a held pitch input
/// (`verification/scenarios/pitch-both-ways.inputs`, `pitch-hold-thrust.inputs`) fit the
/// `body+0x160` column against the recorded basis rotation at `-15.620`, `0.13 %` from
/// `-15.6`, with `94 %` explained. Earlier data left this entry at `29 %` explained,
/// scattering from `-14` to `-24`.
pub const PITCH_INVERSE_INERTIA: f32 =
    12.0 / (INERTIA_MASS * (INERTIA_BOX_Y * INERTIA_BOX_Y + INERTIA_BOX_Z * INERTIA_BOX_Z));

/// The roll entry of the ship's inverse inertia tensor, `1 / I_zz` (world axes; see
/// [`YAW_INVERSE_INERTIA`]): `12 / (m * (x^2 + y^2))`.
///
/// Numerically equal to [`PITCH_INVERSE_INERTIA`] because the box is square in plan, but
/// written out because that is a property of the *box* and would stop holding if the
/// literals changed. The same captures put the measured roll entry at `-15.289`, `2 %` away.
pub const ROLL_INVERSE_INERTIA: f32 =
    12.0 / (INERTIA_MASS * (INERTIA_BOX_X * INERTIA_BOX_X + INERTIA_BOX_Y * INERTIA_BOX_Y));

/// The ship's body-space inertia tensor, a diagonal on `(right, up, forward)`:
/// `(15.6, 21.6, 15.6)`, the reciprocals of the three constants above, what
/// [`crate::ship::Body::inertia`] carries for every craft (`<Misc>` hull dimensions reach
/// the *collider*, not this).
///
/// A function only because `f32` division is not `const`.
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
    /// The track spline sample nearest the ship.
    ///
    /// What the original's `AiTrack_LocatePosition` (`0x0887ce78`) writes into the ship
    /// entity at `+0xaf0` and `+0xb60`. The only consumer is [`crate::maglock`]: the hold
    /// slaves the attitude to `unit(-down)` blended between the two samples, and the
    /// mag-floor probe casts along a direction built from the first one's `down`. Off a
    /// magstrip nothing reads them.
    ///
    /// It replaced `track_up`: the hold also needs the surface *point*, and the second
    /// sample makes the axis turn continuously along the track. Gravity still acts on
    /// world `.y` only; neither is a gravity direction.
    pub track_sample: Option<crate::maglock::TrackSample>,
    /// The spline sample after [`Self::track_sample`], when there is one. `None` is the
    /// original's `-1024.0` sentinel at the second record's `+0x40`, written by
    /// `AiTrack_UpdateCursor` when it has no second sample.
    pub track_sample_next: Option<crate::maglock::TrackSample>,
    /// The per-speed-class scale on `normal_gravity`, `g_class_gravity_scale`
    /// (`0x08ab0dcc`).
    ///
    /// Read off the disc: `<GlobalClass><GravityMul airborne/>` in the engine-wide
    /// `Data\XML\HandlingStats.xml`; the table has one writer and two readers, all read.
    /// Confidence **90**.
    ///
    /// **The attribute is named `airborne` and this scales the *grounded* term.** The
    /// gravity site's VFPU pair chain puts the table value on the lane carrying
    /// `normal_gravity * grounded` and a literal `1.0` on the airborne lane
    /// (`oag_tables::handling::GravityMul`). Do not "fix" this onto `flight_gravity`.
    ///
    /// An input because it belongs to the speed class, which this crate does not know.
    /// The default is the identity (what a caller that cannot read the file should pass; a
    /// zero leaves a grounded craft weightless).
    pub class_gravity_scale: f32,
    /// The ship's own collider index, so its probes do not hit itself.
    pub self_collider: Option<u32>,
    /// Zone mode's auto-speed target, replacing the throttle entirely.
    ///
    /// `Some` selects the four-corner branch of `Ship_UpdateEngine` (`0x0884c834`): used as
    /// the engine output directly, throttle unread, and the `0.5 * speed + accelcap` clamp
    /// does **not** apply. The caller computes `base + step * zone` from the disc's
    /// `<Zone start increment/>` (`oag_race::zone::thrust`).
    ///
    /// An input because the selector is a game mode this crate does not know. `None` is
    /// the ordinary path, bit-for-bit unchanged, which keeps the determinism reference.
    pub auto_speed: Option<f32>,
    /// The push direction of the speed pad the ship is inside this tick.
    ///
    /// `Some` on **every** tick the hull is inside a pad's trigger volume:
    /// `Ship_ApplySpeedupPad` (`0x08848f9c`) re-arms the timer each time, so an entry-only
    /// signal would cut the boost short ([`ShipState::pad_timer`]).
    ///
    /// The pad's own local `+Z` in world space (row 2 of its `.vex` node matrix); pads are
    /// authored aligned with the track. Expected unit length and not normalised here, so an
    /// unnormalised direction scales the boost.
    ///
    /// An input because the track's pad volumes are not known here; `oag_vex::pads` does
    /// the geometry and `oag_raceplay` runs the test.
    pub pad_hit: Option<Vec3>,
    /// The race's `Weapons` and `Damage` options, which decide what a wall costs the energy
    /// pool.
    ///
    /// An input because `Race_ReadSetupOptions` (`0x08896b84`) lowers them from race setup,
    /// which this crate does not know. The default is both on, the original's default for
    /// every game mode except three ([`crate::damage`]).
    pub damage_rules: crate::damage::DamageRules,
    /// The one-shot thrust scale a LeachBeam drain armed, `craft+0x31c`.
    ///
    /// `Ship_UpdateEngine` reads it once and writes `1.0` back ([`crate::engine::engine`]);
    /// its only writer is `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`), which copies the
    /// beam's `slowShipFactor` onto the victim every tick it drains. At or above `1.0` is
    /// neutral and what every other tick passes.
    ///
    /// An input rather than state, as [`Self::pad_hit`] is: re-armed every tick by a weapon
    /// this crate does not know and consumed when read. The composition root holds the
    /// armed value, like `oag_gameplay::world::Ship::pending_slowdown`. See
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    pub thrust_scale: f32,
    /// The disc's `<StartBoost>`, or `None` for a title that authors none and for callers
    /// that have not read it. `None` leaves [`crate::ship::ShipState::launch`] idle and the
    /// engine multiplier at `1.0` ([`crate::launch`]).
    pub start_boost: Option<crate::launch::StartBoost>,
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
            thrust_scale: 1.0,
            start_boost: None,
        }
    }
}

/// The four accumulators the original keeps on the craft, zeroed every frame.
///
/// `craft+0x320`, `+0x330`, `+0x340` and `+0x350`, drained into the rigid body at the
/// bottom of `Ship_UpdateCraft`. Kept as four vectors because which accumulator a term
/// writes is *evidence* (how the local and world frames were told apart).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Accumulators {
    /// Body-local force. Engine thrust and lift, and lateral grip.
    pub local_force: Vec3,
    /// World force. Brakes, the airbrake slide and lateral terms, gravity, drag,
    /// rolling resistance, the hover downforce and vertical damping.
    pub world_force: Vec3,
    /// Body-local **torque**. Steering yaw, pitch, airbrake yaw, bank-to-yaw and angular
    /// damping.
    ///
    /// Torque, settled rather than picked: this crate once treated both angular
    /// accumulators as angular *acceleration* ("a pick awaiting M3"). `Body_Integrate`
    /// settles it at confidence 88: `craft+0x340` reaches `body+0x120` and integrates into
    /// `body+0x160` with **no inertia or mass divide**, four instructions after the linear
    /// half scales by `invMass`. `body+0x160` is angular momentum and `dL/dt = torque`.
    ///
    /// Consequences: the damping term is `(-pitch_damping, -5, -2) * L`, not `* omega`
    /// (`Ship_ApplyAngularDamping` loads `body+0x160`), so
    /// [`crate::passive::angular_damping`] takes the body-local **momentum** and the
    /// "drive and damping share one accumulator so the inertia cancels" argument is
    /// retired; and the inertia tensor now reaches every term here.
    pub local_angular: Vec3,
    /// World-space **torque**: the surface-alignment and weathervane torques.
    ///
    /// `craft+0x350` reaches `body+0x130` and integrates into `body+0x160` through
    /// `basis^T * worldTorque` at `0x0884e35c`, the same path as the local drives plus a
    /// change of basis. [`drain`] folds it into the body frame and hands the sum over as
    /// one torque.
    pub world_angular: Vec3,
}

/// Everything one force evaluation produced, for inspection by a test or debug overlay.
/// The forces have already been accumulated onto the body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluated {
    /// The four accumulators, as drained onto the body.
    pub accumulators: Accumulators,
    /// The air cushion's output.
    pub hover: Hover,
    /// What the magstrip hold did, or `None` on ordinary track.
    /// What the magstrip hold did, or `None` on ordinary track. Not a force: a kinematic
    /// rewrite of the body, reported so a caller can see it ([`crate::maglock`]).
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
    /// Left at default by [`evaluate`], which runs before the body moves;
    /// [`crate::integrate::step`] fills it in. A caller driving `evaluate` directly sees
    /// "no wall response", which is true for that caller.
    pub wall: crate::wall::WallResponse,
    /// What this frame did to the energy pool, **after** the wall constraint ran. Left at
    /// default by [`evaluate`] for [`Self::wall`]'s reason. Both are edges, so reading this
    /// the tick after reads zeroes ([`crate::damage::Shield`]).
    pub shield: crate::damage::Shield,
    /// Whether a barrel roll was **armed** this frame, by either route.
    ///
    /// An edge, the arm and not the payout (the original plays its cue on landing, so the
    /// force law does not need it). Reported because our AI rolls on purpose and the
    /// original's does not, and a deviation that cannot be counted cannot be shown tuned
    /// right. [`crate::barrel_roll::arm`] charges the pool, `roll_cost` percent of it.
    pub roll_armed: bool,
}

/// Evaluates every force for one frame, at full `dt`, exactly once.
///
/// Mutates `state`: control states, contact bookkeeping, the body's accumulators, and the
/// one thing the original applies outside them, the penetration-escape teleport. The body's
/// accumulators are **not** cleared here; [`crate::integrate::step`] clears them at the top
/// of the frame so they still hold what was applied afterwards.
///
/// The term order follows `Ship_UpdateCraft`'s fifteen steps. Order does not change a sum
/// of forces but decides which groundedness each term sees, and keeps the code comparable
/// to the evidence page.
pub fn evaluate<R: Raycaster + ?Sized>(
    state: &mut ShipState,
    input: &ShipControls,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    dt: f32,
) -> Evaluated {
    // Last frame's contacts, bound once before anything can overwrite them.
    let control_grounded = state.grounded;
    state.grounded_prev = control_grounded;
    let control_contact = control_grounded > 0.0;

    controls::update(state, input, handling, dt);

    let forward = state.body.forward();
    let up = state.body.up();
    let velocity = state.body.linear_velocity;
    let forward_speed = velocity.dot(forward);

    // `craft+0x2ec`, the cached speed every term below reads, is the **absolute** value:
    // `Ship_UpdateCraft` runs `vdot.t` (`0x0884992c`) then `vabs.s` (`0x08849930`) before
    // its only store on a craft base. A no-op going forwards; a reversing ship gets drag
    // and rolling resistance that oppose it.
    let cached_speed = forward_speed.abs();

    let mut acc = Accumulators::default();

    // 1. Engine, into the local force accumulator.
    let engine_force = engine::engine(
        state,
        handling,
        control_grounded,
        cached_speed,
        env.auto_speed,
        env.thrust_scale,
    );
    acc.local_force += engine_force.as_local_force();
    // After the engine, which has just read last tick's multiplier: the grader and boost
    // writer run behind it in the original's frame (`crate::launch`).
    launch::advance(
        &mut state.launch,
        env.start_boost.as_ref(),
        state.released,
        input.thrust != 0.0,
        dt,
    );

    // 2. Brakes. Called only while the contact flag is set, so groundedness gates the
    //    whole term: braking does nothing in the air.
    let brakes = if control_contact {
        engine::brakes(state, handling)
    } else {
        Vec3::ZERO
    };
    acc.world_force += brakes;

    // 3. Airbrakes, and the sideshift in the same function's tail. The sideshift calls
    //    `Body_AddForceWorld` directly, which lands in the same place; gated on the
    //    contact flag, which hover has not rebuilt yet, so **last** frame's.
    let airbrake = airbrake::evaluate(state, input, handling, cached_speed);
    acc.world_force += airbrake.world_force;
    acc.local_angular += airbrake.local_angular;

    airbrake::advance_sideshift(state, input, dt);
    let sideshift = airbrake::sideshift_force(state, handling, control_grounded);
    acc.world_force += sideshift;

    // The barrel roll's gesture, ramp and payout countdown. The gesture reads the d-pad
    // edges and steering axis off the same `ShipControls`, so a real input snapshot arms a
    // roll here and nowhere else; `advance_gesture` advances the inter-tap timer itself.
    // Arming changes nothing in the force law (the original plays its cue on the
    // *payout*) and is reported on `Evaluated::roll_armed`; see `crate::barrel_roll`.
    //
    // `control_contact`, not `contact_grounded`: the original's `craft+0x1c0 & 1` read
    // here is last frame's, as hover has not rebuilt it. Same value the sideshift gets.
    let roll_armed = barrel_roll::advance_gesture(
        state,
        input,
        &handling.dimensions,
        handling.roll_cost,
        control_contact,
        dt,
    );
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

    // 8. Hover. Its load factor reads last frame's groundedness (`grounded_prev`) and it
    //    produces this frame's contact count.
    //
    //    The target height is bound *before* the hover call and reused by the mag lock
    //    below: the original builds `craft+0x2f0` once at the top of `Ship_UpdateCraft`
    //    from the previous frame's blend, and both consumers read that value. Recomputing
    //    it after the ramp would give the reposition a target the spring never saw.
    let target_height = hover::capped_target_height(
        handling,
        state.mag_lock_blend,
        state.slowdown_timer,
        state.hover_cap,
    );
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
    // **The landing response is armed in the air, not on touchdown**, and only once the
    // flight has outlasted `rebound_jump_time`: `Ship_UpdateCraft` (`0x08849df0`) keeps
    // `craft+0x284` as the airborne clock and `Ship_HoverTwoPoint` does the arming. A
    // shorter hop comes back down on the ordinary `rebound`, so the `landing_rebound`
    // bounce is an event, not a flicker response. Confidence 95; `ShipState::time_airborne`.
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

    // The barrel roll's landing payout: on this tick's airborne-to-grounded transition,
    // resolve the self-completing ramp and, if the roll finished, arm
    // [`ShipState::roll_payout_timer`] for `<Special roll_turbotime>`.
    //
    // Runs after hover, so on the landing tick the hover spring used the *pre*-arm timer and
    // sees the payout from the next tick; lateral grip below sees it immediately. Where the
    // original arms it relative to hover and lateral grip was not traced (see
    // `crate::barrel_roll` for why the release is tied to this transition).
    if contact && control_grounded <= 0.0 && barrel_roll::release(state) {
        state.roll_payout_timer = handling.roll_turbotime;
    }

    // Still step 8: `Ship_UpdateHover` runs one hover model then `Ship_UpdateMagLock`
    // unconditionally. **Not a force term**: it writes the body's position, velocity and
    // basis directly and touches no accumulator (see `crate::maglock` for why a torque
    // would be wrong).
    let mag_contact = maglock::probe(state, env, raycaster);
    let mag_lock = maglock::update(state, env, mag_contact, target_height);

    // The weapon slowdown timer's decay (`0x08849a24`), **after** the hover target was
    // computed from it and **before** the lateral-grip test, the original's order: a hit
    // costs its full first tick of engine and hover sink, and the grip test sees the
    // post-decrement value.
    //
    // **Gated, not clamped, unlike `airbrake::advance_sideshift`.** The original is
    // `if (t > 0.0f) t -= dt;` for both, landing one `dt` negative on expiry.
    // `advance_sideshift` clamps because every reader gates on `> 0.0`; **that premise is
    // false here**: `crate::slowdown::add` reads this field arithmetically (`timer +=
    // seconds` before its clamp), so the residue is observable in the next hit.
    // Outside the branch chain below: a stunned and weapon-slowed craft still counts this
    // timer down while the stun branch owns the grip.
    if state.slowdown_timer > 0.0 {
        state.slowdown_timer -= dt;
    }

    // 9. Lateral grip, into the *local* force accumulator, once both the collision stun
    //    and the weapon slowdown timer have expired.
    //
    //    `Ship_ApplyLateralGrip` opens with `if (craft+0x290 > 0) { craft+0x290 -= dt;
    //    return; }`, so the stun suppresses the grip and is counted down here, after the
    //    engine (step 2) read the pre-decrement value, as in the original
    //    (`ShipState::stun_timer`).
    let lateral_grip = if state.stun_timer > 0.0 {
        state.stun_timer = (state.stun_timer - dt).max(0.0);
        Vec3::ZERO
    } else if state.slowdown_timer > 0.0 {
        Vec3::ZERO
    } else {
        airbrake::lateral_grip(state, handling, contact_grounded)
    };
    acc.local_force += lateral_grip;

    // 11. Weathervane, 12. angular damping, 13. rolling resistance, 14. vertical damping.
    //     Step 10 is the dead in-air roll-levelling branch, deliberately not ported.
    let weathervane = passive::weathervane(forward, velocity, contact);
    // `Ship_ApplyAngularDamping` loads `body+0x160`, angular **momentum**, so it reads
    // `I * omega` in the body frame, not `omega`; that is why the inertia survives to the
    // observable (`Accumulators::local_angular`).
    let local_angular_velocity = state.body.orientation.inverse() * state.body.angular_velocity;
    let local_angular_momentum = local_angular_velocity * state.body.inertia;
    let angular_damping = passive::angular_damping(handling, local_angular_momentum);
    let rolling_resistance = passive::rolling_resistance(velocity, cached_speed);
    let vertical_damping = passive::vertical_damping(up, velocity, contact_grounded);

    acc.world_angular += weathervane;
    acc.local_angular += angular_damping;
    acc.world_force += rolling_resistance;
    acc.world_force += vertical_damping;

    // 15. The speed-pad boost, the last of the fifteen, on its own timer so it runs on
    //     after the tick that armed it. `up` is the body's own up row, which is
    //     `craft+0x160`, the hull up axis `<Special speedpad_jump>` tilts toward (dot
    //     `+1.000000` against `body+0x010` on a live capture).
    let speedup_pad = engine::speedup_pad(state, input, handling, up, env.pad_hit, dt);
    acc.world_force += speedup_pad;

    drain(state, &acc);

    // Outside the accumulators: the penetration-escape teleport moves the body without
    // touching its velocity.
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
        roll_armed,
    }
}

/// Drains the four accumulators onto the body.
///
/// The force accumulators hold **forces** (gravity carries `mass` explicitly and no other
/// term does, as in the original), so nothing multiplies by mass. The angular ones hold
/// **torque** ([`Accumulators::local_angular`]), so nothing multiplies by inertia either.
/// The world one is rotated into the body, summed with the local one, and the total goes
/// out as a world-space torque, [`crate::integrate`]'s input.
///
/// The inertia used to be applied here and divided out by the integrator, a round trip
/// that made [`crate::ship::Body::inertia`] a no-op for every accumulator term.
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
