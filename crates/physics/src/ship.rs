//! Ship dynamics: the rigid body, the controls that drive it, and the state the
//! force law carries between frames.
//!
//! The model is specified in `docs/physics/README.md`. The types below are the
//! contract; the force law and the integrator are implemented on top of them, in
//! [`crate::hover`], [`crate::airbrake`], [`crate::forces`] and
//! [`crate::integrate`].
//!
//! [`Body::mass`] and [`crate::params::Physical::mass`] are the same quantity held
//! in two places: the force law reads the parameter set and the integrator reads
//! the body. Keeping them equal is the integration layer's job, and nothing here
//! copies one over the other, so a mismatch stays visible rather than silently
//! producing a ship whose suspension and inertia disagree.

use oag_core::math::{Quat, Vec3};

/// The largest delta the original will integrate, in seconds.
///
/// `dt = clamp(measured_dt, 0, 0.06666)`, so a frame longer than about 15 Hz is
/// simulated as though it were 15 Hz. At the project's fixed 60 Hz this clamp
/// never fires, and it is reproduced anyway because a variable-delta comparison
/// against a trace from the original will need it.
pub const MAX_DT: f32 = 0.066_66;

/// How many explicit Euler sub-steps one frame is integrated in.
///
/// The count is fixed at three and the sub-step *size* is `dt/3`, so it varies
/// with the frame. There is no 1/60 anywhere in the original's craft path
/// despite fourteen other systems using one; see
/// `docs/architecture/adr/0007-fixed-timestep-vs-original.md`.
pub const SUBSTEPS: u32 = 3;

/// A rigid body with a diagonal inertia tensor.
///
/// Forces and torques accumulate in world space over a frame and are consumed by
/// the integrator, which is how the original works: every force is applied at a
/// point via an `AddForceAtPoint`-shaped call and the accumulators are what the
/// sub-steps read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    /// Centre of mass in world space.
    pub position: Vec3,
    /// Orientation as a unit quaternion.
    pub orientation: Quat,
    /// Linear velocity in world space.
    pub linear_velocity: Vec3,
    /// Angular velocity in world space, radians per second.
    pub angular_velocity: Vec3,
    /// Accumulated world-space force for this frame.
    pub force: Vec3,
    /// Accumulated world-space torque for this frame.
    pub torque: Vec3,
    /// Mass, from `<Physical mass/>`.
    pub mass: f32,
    /// Diagonal of the body-space inertia tensor.
    ///
    /// [`crate::forces::ship_inertia`], `(15.6, 21.6, 15.6)` on
    /// `(right, up, forward)`, and that is also what [`Body::default`] carries.
    ///
    /// **The default is the ship's tensor rather than [`Vec3::ONE`] on purpose,
    /// and the purpose is a footgun.** Every craft in the game shares this tensor,
    /// because it is a code literal at a single call site rather than something
    /// authored per ship, so there is no per-craft value for a spawn path to
    /// compute - and a body written as
    /// `Body { position, mass, ..Body::default() }` is overwhelmingly a craft.
    /// Leaving the default at `1` would make that expression silently produce a
    /// ship with `15.6x` the pitch authority and an unstable surface-alignment
    /// oscillator, with nothing in the source to look at. A test that genuinely
    /// wants a unit inertia says so.
    ///
    /// # It reaches every angular path, which it did not used to
    ///
    /// An earlier revision of this comment explained why setting this field was a
    /// **no-op that looked like an implementation**: `forces::drain` multiplied
    /// the angular accumulators by it and [`crate::integrate`] divided by it
    /// again, so it cancelled exactly on every accumulator term and reached only
    /// the hover probes' `add_force_at_point`. That was a consequence of the
    /// crate treating the accumulators as angular *acceleration*, which the
    /// original does not - it accumulates **torque** and damps angular
    /// **momentum**.
    ///
    /// The crate is on the momentum model now, so the round trip is gone and this
    /// field divides every angular term: the drives, the weathervane, the surface
    /// alignment and the probes' lever arms. What it must *not* be applied to is
    /// the angular damping, which reads `I * omega` and therefore leaves the
    /// angular acceleration inertia-free by construction - see
    /// [`crate::passive::angular_damping`].
    pub inertia: Vec3,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            force: Vec3::ZERO,
            torque: Vec3::ZERO,
            mass: 1.0,
            inertia: crate::forces::ship_inertia(),
        }
    }
}

impl Body {
    /// The body's own right axis in world space.
    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.orientation * Vec3::X
    }

    /// The body's own up axis in world space.
    ///
    /// This is the axis the hover probes cast along, not world up, which is what
    /// lets magstrips and inversions work at all.
    #[must_use]
    pub fn up(&self) -> Vec3 {
        self.orientation * Vec3::Y
    }

    /// The body's own forward axis in world space.
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        self.orientation * Vec3::NEG_Z
    }

    /// Velocity of a world-space point rigidly attached to the body.
    #[must_use]
    pub fn velocity_at(&self, point: Vec3) -> Vec3 {
        self.linear_velocity + self.angular_velocity.cross(point - self.position)
    }

    /// Accumulates a world-space force applied at a world-space point.
    pub fn add_force_at_point(&mut self, force: Vec3, point: Vec3) {
        self.force += force;
        self.torque += (point - self.position).cross(force);
    }

    /// Accumulates a world-space force through the centre of mass.
    pub fn add_force(&mut self, force: Vec3) {
        self.force += force;
    }

    /// Accumulates a world-space torque.
    pub fn add_torque(&mut self, torque: Vec3) {
        self.torque += torque;
    }

    /// Applies a world-space impulse through the centre of mass, immediately.
    ///
    /// **Not an accumulator, and that is the whole reason it exists.**
    /// [`crate::step`] calls [`Self::clear_accumulators`] at its top, so a force
    /// added by a caller *outside* the step - which is where a weapon blast is
    /// applied, because only `oag-gameplay` knows what a weapon is - would be
    /// dropped before anything integrated it. An impulse writes the velocity
    /// instead, so it survives.
    ///
    /// `dv = J / m`, with a zero or negative mass ignored rather than dividing:
    /// a `Handling::ZERO` fixture has no mass, and an infinity in a velocity
    /// takes the whole simulation with it.
    ///
    /// **Ours.** The original applies `<Rocket blastforce>` through something
    /// unread; that it is an impulse rather than a force held over some duration
    /// is this project's reading. See `oag_gameplay::projectile`.
    pub fn apply_impulse(&mut self, impulse: Vec3) {
        if self.mass > 0.0 {
            self.linear_velocity += impulse / self.mass;
        }
    }

    /// Drops the frame's accumulated force and torque.
    pub fn clear_accumulators(&mut self) {
        self.force = Vec3::ZERO;
        self.torque = Vec3::ZERO;
    }
}

/// One frame of pilot intent, already mapped out of whatever produced it.
///
/// Analog axes are `-1.0..=1.0` and triggers `0.0..=1.0`. The simulation never
/// sees a key, a pad or a PSP button mask: `oag-gameplay` owns the input
/// snapshot and hands this down, which is rule 1 of
/// `docs/architecture/workspace-layout.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShipControls {
    /// Steering, positive to the right.
    pub steer_x: f32,
    /// Pitch, positive nose up.
    pub steer_y: f32,
    /// Thrust, `0.0..=1.0`.
    pub thrust: f32,
    /// Left airbrake, `0.0..=1.0`.
    pub airbrake_left: f32,
    /// Right airbrake, `0.0..=1.0`.
    pub airbrake_right: f32,
    /// A sideshift was requested this frame, and to which side.
    ///
    /// An edge and not a level. This is the *direct* request: a caller that has
    /// already decided a shift should fire sets it, and
    /// [`crate::airbrake::advance_sideshift`] arms the timer. The two gesture
    /// machines below fire the same timers without going through it, so a
    /// caller that maps real buttons leaves this at [`Sideshift::None`] and a
    /// test or a probe that wants a shift on a given tick sets it.
    pub sideshift: Sideshift,
    /// The novice scheme's sideshift button (`OPT_CTRL_SS`, action 7) is
    /// **held**.
    ///
    /// While it is, the steering axis returning inside
    /// [`crate::airbrake::SIDESHIFT_FLICK_THRESHOLD`] arms a flick and crossing
    /// back out of it fires one. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub shift_modifier: bool,
    /// The veteran scheme's left airbrake (`OPT_CTRL_LAB`, action 5) was
    /// **pressed** this tick - an edge, not a level.
    ///
    /// Two presses inside [`crate::airbrake::SIDESHIFT_TAP_WINDOW`] are a left
    /// sideshift. The original reads this off the pressed mask at
    /// `*(craft+0x78) + 0x20`, which is why a held airbrake does not repeat.
    pub shift_tap_left: bool,
    /// The veteran scheme's right airbrake (`OPT_CTRL_RAB`, action 6) was
    /// pressed this tick.
    pub shift_tap_right: bool,
    /// The `LEFT` d-pad was **pressed** this tick - an edge, not a level.
    ///
    /// One of the barrel roll's two tap sources, and the only one that is a
    /// button: the original writes `1` into its tap history "when the `LEFT`
    /// d-pad bit is pressed *or* the steering axis crosses below `-90`", and
    /// the axis leg is read off [`Self::steer_x`] inside
    /// [`crate::barrel_roll::advance_gesture`] rather than here, because
    /// detecting a *crossing* needs the previous tick's axis and that is
    /// per-craft state. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    ///
    /// Not scheme-dependent, unlike the sideshift's fields: the tap history is
    /// fed by the d-pad in both schemes, which spend `L` and `R` and never the
    /// d-pad.
    pub roll_tap_left: bool,
    /// The `RIGHT` d-pad was pressed this tick, the `2` half of
    /// [`Self::roll_tap_left`]'s encoding.
    pub roll_tap_right: bool,
    /// A barrel roll was decided on this tick, and which way it goes.
    ///
    /// **The *direct* request, exactly as [`Self::sideshift`] is**: a caller
    /// that has already decided a roll should happen sets it, and
    /// [`crate::barrel_roll::advance_gesture`] arms it - through the same
    /// [`crate::barrel_roll::arm`] the gesture reaches, so it is gated on
    /// being airborne and on `cost < shield` identically. A caller that maps
    /// real buttons leaves this `None` and arms through the three-tap gesture
    /// instead.
    ///
    /// `oag_ai::Driver` is the one thing that sets it, and **that is a
    /// deliberate deviation**: the original reads the gesture out of the human
    /// player's pad block and nothing else, so its opponents never roll. A
    /// field of its own rather than synthesised taps on [`Self::steer_x`], so
    /// the invented intent stays distinguishable from the recovered input path
    /// it would otherwise be laundered through. See `docs/gameplay/ai.md`.
    pub roll_request: Option<crate::barrel_roll::TapDirection>,
    /// The fraction of the shield pool this craft keeps back rather than
    /// spending it on a barrel roll, `0.0..=1.0`.
    ///
    /// **The only field on this struct the original has no counterpart for**,
    /// and it is an invented AI-quality rule rather than a recovered
    /// behaviour: an opponent that spends its last energy on a manoeuvre and is
    /// then destroyed by one wall is a worse opponent. It sits *on top of*
    /// [`crate::barrel_roll::arm`]'s own `cost < shield`, which is recovered at
    /// confidence 90 and is not weakened by it.
    ///
    /// **Zero is the recovered behaviour exactly**, which is what a real pad
    /// leaves it at and what [`Default`] gives, so the human player and every
    /// test that does not mention it keep the original's rule and nothing
    /// else. `oag_ai` sets it from `Pilot::roll_floor`, which is where the
    /// `0.20` this replaced now lives - it was a bare constant in
    /// `crate::barrel_roll` until 2026-09-06.
    ///
    /// It is a **hard** floor: a roll that would leave the pool below it is
    /// refused, rather than one started from above it and allowed to end
    /// beneath.
    pub roll_shield_floor: f32,
}

/// Which way a one-shot sideshift goes, if any.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Sideshift {
    /// No sideshift this frame.
    #[default]
    None,
    /// Shift left.
    Left,
    /// Shift right.
    Right,
}

/// Everything the force law carries from one frame to the next.
///
/// Plain data with no allocation and no handles, so a whole race snapshots in one
/// `memcpy`-shaped operation; see `docs/architecture/adr/0003-no-ecs.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipState {
    /// The rigid body.
    pub body: Body,
    /// Left airbrake, ramped toward its input at `gain` up and `falloff` down.
    ///
    /// On the original's internal `0..=100` scale, not `0..=1`; see
    /// [`crate::controls::CONTROL_RANGE`].
    pub airbrake_left: f32,
    /// Right airbrake, same ramp and same scale.
    pub airbrake_right: f32,
    /// The throttle state, on the `0..=100` scale.
    ///
    /// **Not ramped.** `<Engine gain/>` and `<Engine falloff/>` compute a ramp in
    /// the original and it is overwritten by the raw input before anything reads
    /// it; see [`crate::params::Engine::falloff`]. This field holds the raw input,
    /// which is exactly what the original stores at `craft+0x2b8`.
    pub thrust: f32,
    /// The brake state, `0..=100`, ramped at `<Brakes gain/>` and `<Brakes falloff/>`.
    ///
    /// There is no brake axis: it rises while **both** airbrake inputs are positive
    /// and falls otherwise. Its ramp is live, unlike the engine's.
    pub brake: f32,
    /// The steering state, `-100..=100`, ramped toward the analog X axis.
    pub steer: f32,
    /// How far steering is inverted, `0..=1` and beyond.
    ///
    /// A **blend**, not a flag: at 0 steering is normal, at 0.5 it is dead, past 1
    /// it is fully inverted. Nothing in this crate drives it; a reverse-controls
    /// pickup would. Treating it as a boolean would snap where the original ramps.
    pub reverse_controls: f32,
    /// Seconds left on the collision stun at `craft+0x290`.
    ///
    /// While it runs, **the engine produces no thrust at all** and lateral grip is
    /// suppressed: a struck ship coasts and slides. `Ship_UpdateEngine` returns from
    /// its prologue without writing to either accumulator, and
    /// `Ship_ApplyLateralGrip` returns after decrementing this. See
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The engine has an early return
    /// that produces no thrust at all"; confidence 88 on the gate, 85 on reading the
    /// field as a collision stun.
    ///
    /// Armed by [`crate::wall::apply_pending_impulse`] with
    /// [`crate::wall::STUN_PER_CONTACT`], the original's `craft+0x290 += 0.5` in
    /// `Ship_ApplyCollisionImpulse`. It is **added, not assigned**, so successive
    /// impacts accumulate rather than refreshing to a fixed value.
    ///
    /// **This crate does not yet produce anything for
    /// [`Self::pending_impulse`] to hold** - a track contact does not, and is not
    /// supposed to; see [`crate::wall::STUN_PER_CONTACT`]'s own doc for the two
    /// bugs an earlier, wrong arming site cost before that was found. So this
    /// timer is armed correctly whenever something posts a pending impulse, and
    /// nothing does yet.
    ///
    /// Decremented in [`crate::forces::evaluate`] at the lateral-grip step rather
    /// than with the control ramps, because that is where the original decrements it
    /// and it is what makes the engine see the *pre*-decrement value in the same
    /// frame.
    pub stun_timer: f32,
    /// Whether last frame's [`crate::wall::resolve`] pushed the hull out of a wall.
    ///
    /// **Vestigial.** This existed to make an earlier, wrong arming site
    /// edge-triggered - `wall::resolve` used to arm [`Self::stun_timer`] directly
    /// on the first frame of every inward contact, and this field was how it knew
    /// "first frame" from "still touching". That arming site is gone (see
    /// [`crate::wall::STUN_PER_CONTACT`]'s own doc for why), so nothing reads this
    /// field for anything any more - `wall::resolve` still writes it every tick,
    /// faithfully, to nowhere. Left in rather than removed because a hull contact
    /// is still a fact a future consumer (a hit-reaction animation, an audio cue)
    /// is likely to want the edge of; removing it now would be guessing that
    /// nothing will.
    pub wall_contact_prev: bool,
    /// The pending collision-impulse vector at `entity->0x4c + 0x110`.
    ///
    /// `Ship_ApplyCollisionImpulse` (`0x0883f274`) reads this every tick,
    /// projects it onto the ship's own forward axis, applies the result, arms
    /// [`Self::stun_timer`], and zeroes this **unconditionally** on every path -
    /// gated only by whether it started nonzero and by
    /// [`Self::shield_pickup_timer`]. [`crate::wall::apply_pending_impulse`] is
    /// the port; see its own doc and
    /// `docs/ghidra/functions/psp-pulse-usa/contact-response.md` for the full
    /// instruction-level read.
    ///
    /// **Nothing in this crate calls a producer yet, so this field is still
    /// always zero in practice.** Two producers are found in the original -
    /// `Weapon_PostBlastImpulse_q` (a weapon blast) and an unnamed second
    /// writer reached from what reads like a rival-contact path.
    /// `Weapon_PostBlastImpulse_q` has a Rust port,
    /// [`crate::wall::post_blast_impulse`] - correct and tested against
    /// directly-supplied source position, radius and power - but nothing in
    /// this crate calls it: the original's own caller is found but unread, and
    /// this crate has no weapon trigger or stats table to drive it from. The
    /// second writer has no port at all. So `apply_pending_impulse` still runs
    /// every tick against a field nothing in this crate ever sets, a correct
    /// no-op until a producer is wired to something, the same shape as
    /// [`Self::pad_direction`] before a probe script crosses a pad.
    pub pending_impulse: Vec3,
    /// Seconds left on the timer at `craft+0x2e0`.
    ///
    /// While it runs, the hover target height is reduced by `min(timer, 4.0)` and
    /// lateral grip is suppressed entirely. **What arms it is not known** - a leap,
    /// a respawn and a race start are all plausible - so nothing in this crate sets
    /// it and a caller may. It counts down by `dt` and stops at zero.
    pub leap_timer: f32,
    /// How grounded the ship is, quantised to `{0.0, 0.5, 1.0}` - the count of
    /// probes in contact, over two.
    pub grounded: f32,
    /// Last frame's [`Self::grounded`], which **every control term reads**.
    ///
    /// Not only the hover spring's load factor. Hover is step 8 of 15 in the
    /// original's craft update and it clears the contact flag on entry, so the
    /// engine, the brakes, quadratic drag, gravity and pitch all see the *previous*
    /// frame's groundedness, while lateral grip and the weathervane torque - which
    /// run after hover - see this frame's. Confidence 84; see
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md`. The obvious reimplementation,
    /// resolving contacts and then applying forces, gets a different answer on
    /// every takeoff and landing frame in five terms at once.
    pub grounded_prev: f32,
    /// Seconds left on each sideshift, left then right.
    ///
    /// The original keeps one timer per side (`entity+0x8a4` and `entity+0x8a8`),
    /// sets the fired one to [`crate::airbrake::SIDESHIFT_DURATION`], counts both
    /// down by `dt`, and drives one craft flag from each - so both can run at once
    /// and the two forces then cancel. Recovered from
    /// `Ship_UpdateSideshiftInput_q` (`0x08846a54`); see
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
    pub sideshift_timers: [f32; 2],
    /// Seconds left on each side's double-tap window, left then right.
    ///
    /// The original's `entity+0x89c` and `entity+0x8a0`. The first press of an
    /// airbrake opens its side's window at
    /// [`crate::airbrake::SIDESHIFT_TAP_WINDOW`]; a second press while the
    /// window is still open fires the shift instead of reopening it. Veteran
    /// scheme only. See `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub shift_tap_windows: [f32; 2],
    /// Whether a novice-scheme flick is armed (`entity+0x860 & 0x400`).
    ///
    /// Set while the sideshift button is held and the steering axis is inside
    /// [`crate::airbrake::SIDESHIFT_FLICK_THRESHOLD`], cleared when a flick
    /// fires. Without it a held-over axis would fire a shift every tick.
    pub shift_armed: bool,
    /// Seconds before another sideshift may be triggered (`entity+0x8ac`).
    ///
    /// Refreshed to [`crate::airbrake::SIDESHIFT_LOCKOUT`] on every tick either
    /// sideshift timer is running, and gates *both* gesture machines - so it is
    /// a second between the end of one shift and the earliest start of the
    /// next, not a second between starts. It does not gate
    /// [`ShipControls::sideshift`], which is a direct request rather than a
    /// gesture.
    pub shift_lockout: f32,
    /// The barrel roll's three-entry tap-history gesture buffer, oldest first
    /// (`entity+0x88c`/`+0x890`/`+0x894`).
    ///
    /// `0` is an empty slot, `1` a `LEFT` tap and `2` a `RIGHT`. A completed
    /// three-tap alternation - `[2, 1, 2]` or `[1, 2, 1]` - arms the roll and
    /// clears this back to `[0, 0, 0]`, so a fourth tap starts a fresh gesture
    /// rather than immediately re-arming off the sliding window. See
    /// [`crate::barrel_roll`] and
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub roll_taps: [u8; 3],
    /// Seconds since the last tap recorded into [`Self::roll_taps`]
    /// (`entity+0x884`).
    ///
    /// Accumulates `dt` every tick and is zeroed on every tap. A tap recorded
    /// at `0.6` s or later does not shift the two older entries down - see
    /// [`crate::barrel_roll::INTER_TAP_TIMEOUT`].
    pub roll_tap_timer: f32,
    /// The barrel roll's signed phase, `-1.0..=1.0` (`entity+0x87c`).
    ///
    /// Ramps toward [`Self::roll_target`] at `<Special roll_speed>` and does not
    /// reverse on its own - see [`crate::barrel_roll::advance_phase`].
    pub roll_phase: f32,
    /// Where [`Self::roll_phase`] is currently ramping toward: `1.0` after a
    /// `[2, 1, 2]` arm, `-1.0` after `[1, 2, 1]`, or `0.0` once the roll has
    /// released short of completion. See [`crate::barrel_roll::release`].
    pub roll_target: f32,
    /// Seconds left on the barrel roll's landing payout, ours - the original's
    /// `craft+0x1c0 & 0x400`, held for `<Special roll_turbotime>` after a
    /// completed roll touches down. While it runs, [`crate::airbrake::lateral_grip`]
    /// is scaled 1.5x, the hover spring's rebound coefficient is forced to
    /// `1.0`, and the engine grants the same uncapped turbo add
    /// [`Self::turbo_timer`] grants for a Turbo pickup. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub roll_payout_timer: f32,
    /// Which side of [`crate::barrel_roll::AXIS_TAP_THRESHOLD`] the steering
    /// axis sat on at the end of last tick, or `None` for neither.
    ///
    /// **This crate's own resolution, not a traced field.** The original feeds
    /// its tap history from "the steering axis crossing below `-90`" / "above
    /// `+90`", and a crossing is an edge: it needs last tick's side of the
    /// threshold to be distinguishable from this tick's. Where the original
    /// keeps that was not traced, so it is kept here, with the rest of the
    /// gesture's per-craft state, rather than in the input layer - the same
    /// division [`Self::shift_armed`] already makes for the novice flick.
    ///
    /// A tap fires whenever this changes *into* a side, so a thumb rolled
    /// straight from `LEFT` to `RIGHT` with no neutral tick between them is one
    /// tap and not none. See [`crate::barrel_roll::advance_gesture`].
    pub roll_axis_zone: Option<crate::barrel_roll::TapDirection>,
    /// Seconds since the ship last touched down, in seconds.
    ///
    /// Below 0.2 the suspension uses `landing_rebound` in place of `rebound`.
    ///
    /// **Armed in the air, not on touchdown** - see [`Self::time_airborne`].
    pub time_since_landing: f32,
    /// Seconds the ship has been off the ground, `craft+0x284`.
    ///
    /// `Ship_UpdateCraft` (`0x08849df0`) keeps this and its mirror
    /// `craft+0x288` as a pair: whichever of grounded and airborne is true this
    /// tick accumulates `dt` while the other is zeroed. The disassembly is
    /// unambiguous, so confidence **95**; only `+0x288`, the grounded twin,
    /// goes unmodelled, because nothing else reads it.
    ///
    /// It exists for one job. `Ship_HoverTwoPoint` zeroes
    /// [`Self::time_since_landing`] **while the craft is still in the air**,
    /// and only once this has passed `Antigrav::rebound_jump_time`. So a hop
    /// shorter than that never arms the landing response and the craft touches
    /// back down on the ordinary `rebound`; only a real flight earns the
    /// `landing_rebound` bounce. See `docs/ghidra/functions/psp-pulse-usa/engine.md`.
    pub time_airborne: f32,
    /// The 0-to-1 magstrip blend. At 1.0 the ordinary suspension is fully
    /// cancelled and the magnetic hold has taken over.
    ///
    /// Driven by [`crate::maglock::ramp`], `0.2` a frame in either direction.
    pub mag_lock_blend: f32,
    /// Seconds left on the speed-pad boost, `craft+0x298`.
    ///
    /// **Re-armed, not triggered.** `Ship_ApplySpeedupPad` (`0x08848f9c`) assigns
    /// the class's `<SpeedupPads time>` on *every* tick the craft is inside a pad
    /// volume, then decrements by `dt` further down the same function. So the
    /// boost runs for `time` seconds counted from the last tick inside the pad,
    /// not from the tick it was entered, and a ship crossing a pad slowly is
    /// boosted for longer. [`crate::forces::Environment::pad_hit`] being `Some`
    /// every such tick is what reproduces that.
    ///
    /// Counted down in [`crate::forces::evaluate`] **before** the force is read
    /// off it, which is the original's ordering and is worth a sentence because
    /// the two orderings differ by one `dt` of ramp at the peak - about 6 % - and
    /// `docs/physics/cornering-ground-truth.md` measured the pad crossings
    /// precisely enough to tell them apart.
    pub pad_timer: f32,
    /// The world-space direction the speed-pad boost pushes, `craft+0x1b0`.
    ///
    /// Refreshed from the pad on every tick inside it, and then held while
    /// [`Self::pad_timer`] runs down - so leaving a pad does not change where the
    /// remaining boost pushes, even as the ship turns. Unit length when it comes
    /// from a pad; zero on a ship that has never touched one.
    pub pad_direction: Vec3,
    /// The **last** mag-floor hit the probe found, `craft+0x250` and `+0x260`.
    ///
    /// Carried between frames rather than recomputed, because the original does:
    /// those two fields are a raycast's out-parameter, only a hit overwrites them,
    /// and [`mag_lock_blend`](Self::mag_lock_blend) takes five frames to decay
    /// after the strip ends - so the hold's last five frames read a stale contact
    /// in the original too. `None` is a ship that has never touched a magstrip.
    pub mag_contact: Option<crate::maglock::MagContact>,
    /// The energy pool, the original's `entity+0x88`.
    ///
    /// **The ship entity, one hop out at `craft+0x1c4`** - *not* the craft the
    /// trace harness breaks on, whose `+0x88` is an orientation-matrix element.
    /// This comment named the wrong object for one commit and a capture taken
    /// there read a direction cosine, which looks exactly like a quantity going
    /// about its business. Settled live 2026-08-10 by the reciprocal pointer
    /// identity: the entity's `+0x94` points back at the craft.
    ///
    /// Recovered in `docs/ghidra/functions/psp-pulse-usa/shield.md`. Bounded
    /// above by [`crate::Dimensions::shield`] and floored at zero here, which is
    /// the one place this crate knowingly departs from the original:
    /// `Ship_SetShield` clamps above and **not** below, because `Ship_Damage`
    /// tests the signed result against zero to transition the craft into its
    /// destroyed state. There is no destroyed state in this engine yet, so a pool
    /// allowed to run negative would be a number nothing could act on and the HUD
    /// would have to special-case. When the destroyed transition lands, this floor
    /// is what moves.
    ///
    /// **In this crate, and hashed by the determinism gate, because the original
    /// keeps it on the craft.** It is gameplay-facing rather than dynamical -
    /// nothing in the force law reads it - but it is written from the contact
    /// response, and a pool that lived outside [`ShipState`] would be a
    /// simulation field the gate could not see. See
    /// [`crate::damage::apply_contact`].
    pub shield: f32,
    /// Where the craft is in the destroyed sequence, the original's `entity+0x8c`.
    ///
    /// Only the three states the energy pool reaches; see
    /// [`crate::damage::CraftState`].
    pub craft_state: crate::damage::CraftState,
    /// Seconds left on the current craft state, the original's `entity+0x874`.
    ///
    /// Only [`crate::damage::CraftState::Destroyed`] runs it down. Zero
    /// otherwise, which keeps it out of the way of the determinism hash on every
    /// tick of a race nobody dies in.
    pub state_timer: f32,
    /// Seconds left on a fired Turbo pickup.
    ///
    /// While it is positive the engine **adds `Engine.turbo` to thrust**, after
    /// the acceleration cap and before the fixed doubling - so the add is
    /// uncapped, which is what makes a turbo a turbo. See
    /// [`crate::engine::engine`].
    ///
    /// # What the original gates that add on, and why this is a timer instead
    ///
    /// `Ship_UpdateEngine` (`0x0884c5c8`) runs the add under
    /// `((flags & 0x200) || (flags & 0x400)) && craft+0x2a4 == 1`, read from the
    /// decompiler 2026-08-11. **Two bits, either of which turns the same term
    /// on**, which is the shape of one effect with two sources - and the second
    /// source is now identified rather than inferred: `0x400` is held for
    /// `<Special roll_turbotime>` seconds after a completed barrel roll lands,
    /// confidence 90. See [`Self::roll_payout_timer`], which is this crate's
    /// own timer for that source - kept separate from this field rather than
    /// merged into it, because `0x400` also drives two effects `0x200` does
    /// not (a lateral-grip multiplier and a hover override), and merging the
    /// two timers would apply those to an ordinary Turbo pickup too. That
    /// leaves `0x200` as the other way to be turboing, which is what a Turbo
    /// pickup is for. **The writer of `0x200` has not been found**, so this
    /// half is still an inference from the pair rather than a traced path -
    /// confidence 75. See `docs/ghidra/functions/psp-pulse-usa/
    /// input-bindings.md`.
    ///
    /// **The timer itself is ours.** The original holds a bit and this holds
    /// seconds, because neither `craft+0x1c0`'s writer nor the pickup word
    /// behind it has been read. The **duration** is the disc's own
    /// `<Weapon type="Turbo"><Stats time>` and the **magnitude** is its own
    /// `<Engine turbo>`, so both numbers are real even though the field holding
    /// them is not the original's shape.
    ///
    /// **The `craft+0x2a4 == 1` gate is not reproduced**: nothing decodes that
    /// enum. Treating it as satisfied means a craft here can turbo in a state
    /// where the original might not.
    ///
    /// **The boost lift is not implemented.** The same branch adds
    /// `g_boost_lift * T` along body up while the thrust button is held, and
    /// that global was never read. See `docs/gameplay/pickups.md`.
    ///
    /// **In this crate rather than in `oag-gameplay`, and hashed**, for the same
    /// reason [`Self::shield`] is: it is written by gameplay and read by the
    /// force law, and a simulation field the determinism gate cannot see is a
    /// replay divergence nobody notices. The *inventory* - which pickup is held,
    /// if any - is not here, because it is an `oag_formats::weapons::Weapon` and
    /// this crate deliberately depends on nothing but `oag-core`.
    pub turbo_timer: f32,
    /// Seconds left on a fired Shield pickup.
    ///
    /// While it is positive the craft takes no damage:
    /// [`crate::damage::apply_contact`] returns early the same way it does for a
    /// craft that is already blowing up.
    ///
    /// # Recovered as of 2026-08-19, and this comment used to say the opposite
    ///
    /// It said "ours all the way down", on the grounds that `<Weapon
    /// type="Shield"><Stats time>` joined to no recovered code path. It joins to
    /// three. `Shield_Fire` (`0x08861568`), the handler for fire bit `0x20`,
    /// loads the weapon-stats block's `+0x8c` - which is exactly where
    /// `WeaponStats_ParseShield` (`0x0880ca2c`) writes `time` - into
    /// `craft+0x188` and raises running bit `0x10`; `Shield_Update`
    /// (`0x08861630`) counts it down and lowers the bit. That bit is then read
    /// by the weapon-damage drain, by the contact loop's damage reaction and by
    /// `Ship_ApplyCollisionImpulse`. See
    /// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
    ///
    /// **The reason it read as unfindable is worth keeping**: Ghidra resolves no
    /// cross-reference to any string or constant in this image, so "no reader
    /// found" was a property of the search, not of the binary.
    ///
    /// **Damage refused rather than reduced turns out to be right**, and for a
    /// better reason than the one originally given ("unambiguous to test"). Both
    /// of the original's damage drains *discard* the amount on the shielded
    /// branch rather than scaling or deferring it, and both then arm the
    /// shield's own hit flash - which is what
    /// [`crate::damage::Shield::absorbed`] carries out of the tick.
    ///
    /// **What is still ours**: nothing yet decrements `Self::shield` by the
    /// `absorb` half of `<Stats>`, and the original's own absorb path is a
    /// separate button rather than part of the shield.
    ///
    /// **A consequence worth stating.** [`crate::damage::Shield::depleted`] is
    /// the edge Zone's unbuilt end condition is waiting for, and it cannot fire
    /// while this timer runs. That is intended - a shielded craft is not being
    /// destroyed - but it means a mode that ends on `depleted` can be held open
    /// by a pickup.
    ///
    /// **Not named `shield_timer`**, because [`Self::shield`] is the energy pool
    /// and [`crate::damage::Shield`] is the per-tick contact outcome. A third
    /// bare `shield` here would read as one of those two.
    pub shield_pickup_timer: f32,
}

impl Default for ShipState {
    fn default() -> Self {
        Self {
            body: Body::default(),
            airbrake_left: 0.0,
            airbrake_right: 0.0,
            thrust: 0.0,
            brake: 0.0,
            steer: 0.0,
            reverse_controls: 0.0,
            stun_timer: 0.0,
            wall_contact_prev: false,
            pending_impulse: Vec3::ZERO,
            leap_timer: 0.0,
            grounded: 0.0,
            grounded_prev: 0.0,
            sideshift_timers: [0.0, 0.0],
            shift_tap_windows: [0.0, 0.0],
            shift_armed: false,
            shift_lockout: 0.0,
            roll_taps: [0, 0, 0],
            // At or past the timeout, so a cold-started ship's first tap never
            // cascades a stale, all-zero history - see
            // `crate::barrel_roll::INTER_TAP_TIMEOUT`.
            roll_tap_timer: crate::barrel_roll::INTER_TAP_TIMEOUT,
            roll_phase: 0.0,
            roll_target: 0.0,
            roll_payout_timer: 0.0,
            roll_axis_zone: None,
            // `Ship_InitCraft` (`0x08849354`) sets `craft+0x2b4` to `10.0` -
            // recovered, replacing an invented `1.0` that was chosen for the
            // same reason the original's value serves: a freshly spawned ship
            // must not begin inside the 0.2 s landing window. Both are outside
            // it, so this is faithfulness rather than a behaviour change. See
            // `docs/ghidra/functions/psp-pulse-usa/engine.md`.
            time_since_landing: 10.0,
            time_airborne: 0.0,
            mag_lock_blend: 0.0,
            pad_timer: 0.0,
            pad_direction: Vec3::ZERO,
            mag_contact: None,
            // Zero, not a pool: a default ship has no `Dimensions` to take a
            // maximum from, and inventing one here would make every test that
            // builds a `ShipState::default()` silently start a race full.
            // `crate::damage::reset` is what fills it.
            shield: 0.0,
            craft_state: crate::damage::CraftState::Racing,
            state_timer: 0.0,
            turbo_timer: 0.0,
            shield_pickup_timer: 0.0,
        }
    }
}

impl ShipState {
    /// Whether any hover probe is in contact.
    ///
    /// A convenience over comparing [`Self::grounded`] to zero, which is exact:
    /// the field only ever holds `0.0`, `0.5` or `1.0`, all of which are
    /// representable.
    #[must_use]
    pub fn is_grounded(&self) -> bool {
        self.grounded > 0.0
    }

    /// Quantises a probe contact count to `{0.0, 0.5, 1.0}`.
    ///
    /// Contact count over two, which is the whole of the original's
    /// groundedness. Stated as a function because the two-probe assumption is
    /// baked into the divisor and a four-corner variant exists in the original
    /// whose selection rule is not known.
    #[must_use]
    pub fn quantise_grounded(contacts: u32) -> f32 {
        (contacts.min(2) as f32) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groundedness_takes_exactly_three_values() {
        assert_eq!(ShipState::quantise_grounded(0), 0.0);
        assert_eq!(ShipState::quantise_grounded(1), 0.5);
        assert_eq!(ShipState::quantise_grounded(2), 1.0);
        // A four-corner probe set would otherwise report 1.5 and 2.0 here.
        assert_eq!(ShipState::quantise_grounded(3), 1.0);
        assert_eq!(ShipState::quantise_grounded(4), 1.0);
    }

    #[test]
    fn a_force_at_the_centre_of_mass_makes_no_torque() {
        let mut body = Body::default();
        body.add_force_at_point(Vec3::Y, body.position);
        assert_eq!(body.force, Vec3::Y);
        assert_eq!(body.torque, Vec3::ZERO);
    }

    #[test]
    fn a_force_off_axis_makes_torque() {
        let mut body = Body::default();
        body.add_force_at_point(Vec3::Y, Vec3::X);
        assert_ne!(body.torque, Vec3::ZERO);
    }

    /// The torque is `r x F`, and **that sign is load-bearing across the whole crate**.
    ///
    /// The original computes this cross product the other way round, as `F x r`, in
    /// both binaries. That is not a bug in either: the engine integrates its basis as
    /// `e' = e x w` rather than the textbook `e' = w x e`, so its angular velocity is
    /// the negation of the textbook one and `F x r` there means `r x F` here.
    /// [`crate::integrate`] uses the textbook form, so every torque expression taken
    /// from the disassembly is negated exactly once on the way in - here, in
    /// [`crate::hover::ALIGNMENT_GAIN`] (`+400` against a read `-400`) and in
    /// [`crate::passive::WEATHERVANE_GROUND`] (`+0.1` against a read `-0.1`).
    ///
    /// **Flipping any one of those three without the others is unconditional
    /// divergence**, not a subtle drift, which is why this is asserted on the direction
    /// and not merely on being non-zero: `a_force_off_axis_makes_torque` above passes
    /// either way round, so it cannot catch a "correction" back to the literal reading.
    ///
    /// Angular damping is deliberately **not** in that list: `tau = -c * w` is a
    /// negative multiple of `w` in either convention, so it is already right and must
    /// not be flipped with the others.
    #[test]
    fn a_force_at_a_point_makes_torque_the_textbook_way_round() {
        let mut body = Body::default();
        // r = +X, F = +Y. Textbook `r x F` is +Z; the original's `F x r` would be -Z.
        body.add_force_at_point(Vec3::Y, Vec3::X);
        assert_eq!(
            body.torque,
            Vec3::Z,
            "torque came out {:?}; a flip to the original's own `F x r` convention \
             without also flipping the alignment and weathervane gains diverges \
             unconditionally",
            body.torque
        );
    }

    /// The frame axis points **down** in `.vex` data and this one does not, so
    /// the difference is pinned rather than left to be rediscovered.
    #[test]
    fn body_axes_are_a_right_handed_set() {
        let body = Body::default();
        assert_eq!(body.up(), Vec3::Y);
        assert_eq!(body.right(), Vec3::X);
        assert_eq!(body.forward(), Vec3::NEG_Z);
        assert!((body.right().cross(body.up()) - -body.forward()).length() < 1e-6);
    }

    #[test]
    fn a_spinning_body_moves_its_extremities() {
        let body = Body {
            angular_velocity: Vec3::Y,
            ..Body::default()
        };
        let at = body.position + Vec3::X;
        assert_ne!(body.velocity_at(at), Vec3::ZERO);
        assert_eq!(body.velocity_at(body.position), Vec3::ZERO);
    }
}
