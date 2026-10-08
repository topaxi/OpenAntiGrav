//! Ship dynamics: the rigid body, the controls that drive it, and the state the force
//! law carries between frames.
//!
//! The model is specified in `docs/physics/README.md`; the force law and integrator sit
//! on these types in [`crate::hover`], [`crate::airbrake`], [`crate::forces`] and
//! [`crate::integrate`].
//!
//! [`Body::mass`] and [`crate::params::Physical::mass`] are the same quantity held twice
//! (the force law reads the parameter set, the integrator the body). Keeping them equal
//! is the integration layer's job; nothing here copies one over the other, so a mismatch
//! stays visible.

use oag_core::math::{Quat, Vec3};

/// The largest delta the original will integrate, in seconds: `clamp(measured_dt, 0,
/// 0.06666)`. Never fires at the project's fixed 60 Hz; kept for variable-delta
/// comparison against a trace.
pub const MAX_DT: f32 = 0.066_66;

/// How many explicit Euler sub-steps one frame is integrated in. The count is fixed and
/// the sub-step size is `dt/3`; the original's craft path has no 1/60
/// (`docs/architecture/adr/0007-fixed-timestep-vs-original.md`).
pub const SUBSTEPS: u32 = 3;

/// A rigid body with a diagonal inertia tensor.
///
/// Forces and torques accumulate in world space over a frame and the integrator's
/// sub-steps read them, as in the original's `AddForceAtPoint`-shaped calls.
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
    /// Diagonal of the inertia tensor, on the body's own axes.
    /// Diagonal of the inertia tensor, on the body's own axes.
    ///
    /// [`crate::forces::ship_inertia`], `(15.6, 21.6, 15.6)` on `(right, up, forward)`,
    /// which is also [`Body::default`]'s value.
    ///
    /// **This crate applies it in body axes; the original applies the same three numbers
    /// in world axes** (measured exactly on three captures,
    /// `scripts/trace-inertia-frame-fit.py`, `docs/ghidra/functions/psp-pulse-usa/rigid-body.md`).
    /// `15.6` on right and forward makes the tensor yaw-invariant, so they agree on a
    /// level craft and diverge only under pitch and roll. Not changed: it would move the
    /// simulation, and whether it improves it is unmeasured. The contact denominators
    /// are already right: [`crate::pair`] and [`crate::wall`] apply this diagonal to a
    /// world-space `r x n`, as the original does.
    ///
    /// **The default is the ship's tensor, not [`Vec3::ONE`], on purpose.** Every craft
    /// shares it (a code literal at one call site), so
    /// `Body { position, mass, ..Body::default() }` is nearly always a craft, and a unit
    /// default would silently give `15.6x` the pitch authority and an unstable alignment
    /// oscillator. A test that wants unit inertia says so.
    ///
    /// It divides every angular term (drives, weathervane, alignment, probe lever arms)
    /// because accumulators hold **torque** and angular **momentum** is damped, as in the
    /// original. It must *not* be applied to angular damping, which reads `I * omega`;
    /// see [`crate::passive::angular_damping`].
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
    ///
    /// This textbook form *is* what both of the original's contact resolvers use.
    /// `Body_ResolveContact` (`0x0884e968`) and `Body_ResolveContactPair` (`0x0884ef30`)
    /// rotate `body+0x150` through the basis rows (`vtfm4.q`, `0x0884ea58`/`0x0884f038`)
    /// and cross with the lever arm on the left (`vcrsp.t`, `0x0884ea9c`/`0x0884f078`).
    /// `body+0x150` holds the rotation rate **negated and in body coordinates**, so the
    /// expression is `v + cross(r, -omega) == v + omega x r`. Measured:
    /// `scripts/omega-column-reading-fit.py` over four captures, `NegatedLocal` wins by
    /// two orders of magnitude (2 % residual against 200 % for a world-space reading);
    /// it is also the `w_game = -w_physics` contract of [`crate::integrate`].
    ///
    /// **Do not "correct" a resolver to `cross(r, R^T omega)`** against our own
    /// `angular_velocity`: that flips the sign of the dominant mode (a median 8-11 units/s
    /// of `vn` on a recorded lap) and costs `07_Track` its clean lap in
    /// `race_ground_truth`. `wall::tests` pins it with a yawing craft; the other wall
    /// tests start at `omega == 0`, where no convention is exercised.
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
    /// Not an accumulator on purpose: [`crate::step`] calls [`Self::clear_accumulators`]
    /// at its top, so a force added outside the step (a weapon blast, applied by
    /// `oag-gameplay`) would be dropped. An impulse writes the velocity, so it survives.
    ///
    /// `dv = J / m`; a zero or negative mass is ignored (a `Handling::ZERO` fixture has
    /// none, and an infinity takes the simulation with it). **Ours:** that `<Rocket
    /// blastforce>` is an impulse is this project's reading; see `oag_weapons::projectile`.
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
/// Analog axes are `-1.0..=1.0`, triggers `0.0..=1.0`. The simulation never sees a key or
/// pad: `oag-gameplay` owns the input snapshot (rule 1 of
/// `docs/architecture/workspace-layout.md`).
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
    /// An edge. The *direct* request: [`crate::airbrake::advance_sideshift`] arms the
    /// timer. The two gesture machines below fire the same timers without it, so a caller
    /// mapping real buttons leaves this [`Sideshift::None`].
    pub sideshift: Sideshift,
    /// The novice scheme's sideshift button (`OPT_CTRL_SS`, action 7) is **held**.
    ///
    /// While held, the steering axis returning inside
    /// [`crate::airbrake::SIDESHIFT_FLICK_THRESHOLD`] arms a flick and crossing back out
    /// fires one. See `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub shift_modifier: bool,
    /// The veteran scheme's left airbrake (`OPT_CTRL_LAB`, action 5) was **pressed** this
    /// tick (an edge).
    ///
    /// Two presses inside [`crate::airbrake::SIDESHIFT_TAP_WINDOW`] are a left sideshift.
    /// The original reads the pressed mask at `*(craft+0x78) + 0x20`, so a held airbrake
    /// does not repeat.
    pub shift_tap_left: bool,
    /// The veteran scheme's right airbrake (`OPT_CTRL_RAB`, action 6) was
    /// pressed this tick.
    pub shift_tap_right: bool,
    /// The `LEFT` d-pad was **pressed** this tick (an edge).
    ///
    /// One of the barrel roll's two tap sources, and the only button one: the original
    /// writes `1` into its tap history when `LEFT` is pressed *or* the steering axis
    /// crosses below `-90`. The axis leg is read off [`Self::steer_x`] in
    /// [`crate::barrel_roll::advance_gesture`], since a crossing needs the previous
    /// tick's axis (per-craft state). Scheme-independent: both schemes feed the history
    /// from the d-pad. See `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub roll_tap_left: bool,
    /// The `RIGHT` d-pad was pressed this tick, the `2` of [`Self::roll_tap_left`].
    pub roll_tap_right: bool,
    /// A barrel roll was decided on this tick, and which way.
    ///
    /// The *direct* request, like [`Self::sideshift`]: [`crate::barrel_roll::advance_gesture`]
    /// arms it through [`crate::barrel_roll::arm`], so it is gated on being airborne and
    /// `cost < shield` identically. A caller mapping real buttons leaves it `None` and
    /// arms through the three-tap gesture.
    ///
    /// `oag_ai::Driver` sets it, **a deliberate deviation**: the original reads the
    /// gesture from the human pad block only, so its opponents never roll. A field of its
    /// own rather than synthesised taps on [`Self::steer_x`], so the invented intent stays
    /// distinguishable from the recovered input path. See `docs/gameplay/ai.md`.
    pub roll_request: Option<crate::barrel_roll::TapDirection>,
    /// The fraction of the shield pool this craft keeps back rather than spending it on a
    /// barrel roll, `0.0..=1.0`.
    ///
    /// **The only field here with no counterpart in the original**: an invented AI-quality
    /// rule, on top of [`crate::barrel_roll::arm`]'s recovered `cost < shield` (confidence
    /// 90, not weakened). **Zero is the recovered behaviour exactly**, which a real pad
    /// and [`Default`] leave it at. `oag_ai` sets it from `Pilot::roll_floor`. It is a
    /// **hard** floor: a roll that would end below it is refused.
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
    /// Left airbrake, ramped toward its input at `gain` up and `falloff` down, on the
    /// original's `0..=100` scale (see [`crate::controls::CONTROL_RANGE`]).
    pub airbrake_left: f32,
    /// Right airbrake, same ramp and same scale.
    pub airbrake_right: f32,
    /// The throttle state, on the `0..=100` scale.
    ///
    /// **Not ramped.** The original's `<Engine gain/>` and `<Engine falloff/>` ramp is
    /// overwritten by the raw input before anything reads it
    /// ([`crate::params::Engine::falloff`]); this holds the raw input, as `craft+0x2b8`.
    pub thrust: f32,
    /// The brake state, `0..=100`, ramped at `<Brakes gain/>` and `<Brakes falloff/>`.
    ///
    /// There is no brake axis: it rises while **both** airbrake inputs are positive.
    pub brake: f32,
    /// The steering state, `-100..=100`, ramped toward the analog X axis.
    pub steer: f32,
    /// The steering lean's rate-limited follower, the original's `craft+0x848`.
    ///
    /// Chases `raw_stick * 0.01` at no more than `0.6 * 5.4` per second when pushing
    /// outward and `0.3 * 5.4` when returning ([`crate::controls::update_camera_lean`]).
    /// **Not hashed**: only the cockpit camera reads it, and hashing it would move every
    /// committed reference for a value no force reads.
    pub camera_lean_follower: f32,
    /// The steering lean the cockpit camera rolls by, `craft+0x844`:
    /// [`Self::camera_lean_follower`] through a `4/s` first-order filter, `-1..=1`. Not
    /// hashed, for the reason above.
    pub camera_lean: f32,
    /// How far steering is inverted: a **blend**, not a flag (0 normal, 0.5 dead, past 1
    /// fully inverted). Nothing in this crate drives it; a reverse-controls pickup would.
    pub reverse_controls: f32,
    /// Seconds left on the collision stun at `craft+0x290`.
    ///
    /// While it runs **the engine produces no thrust** and lateral grip is suppressed:
    /// `Ship_UpdateEngine` returns from its prologue without writing either accumulator,
    /// and `Ship_ApplyLateralGrip` returns after decrementing this. See
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The engine has an early return
    /// that produces no thrust at all"; confidence 88 on the gate, 85 on reading the field
    /// as a collision stun.
    ///
    /// Armed by [`crate::wall::apply_pending_impulse`] with
    /// [`crate::wall::STUN_PER_CONTACT`] (`craft+0x290 += 0.5` in
    /// `Ship_ApplyCollisionImpulse`), **added, not assigned**, so impacts accumulate.
    /// Nothing posts a pending impulse yet (a track contact does not; see
    /// [`crate::wall::STUN_PER_CONTACT`]), so it is armed correctly and never fires.
    ///
    /// Decremented in [`crate::forces::evaluate`] at the lateral-grip step, as the original
    /// does, so the engine sees the *pre*-decrement value in the same frame.
    pub stun_timer: f32,
    /// Whether last frame's [`crate::wall::resolve`] pushed the hull out of a wall.
    ///
    /// **Vestigial:** it once edge-triggered an earlier, wrong arming site of
    /// [`Self::stun_timer`] (see [`crate::wall::STUN_PER_CONTACT`]). `wall::resolve` still
    /// writes it every tick and nothing reads it. Kept because a hit-reaction animation
    /// or audio cue is likely to want the edge.
    pub wall_contact_prev: bool,
    /// The pending collision-impulse vector at `entity->0x4c + 0x110`.
    ///
    /// `Ship_ApplyCollisionImpulse` (`0x0883f274`) reads it every tick, projects it onto
    /// the ship's forward axis, applies the result, arms [`Self::stun_timer`], and zeroes
    /// it **unconditionally** on every path (gated only by being nonzero and by
    /// [`Self::shield_pickup_timer`]). [`crate::wall::apply_pending_impulse`] is the port;
    /// see `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
    ///
    /// **Nothing in this crate calls a producer yet, so it is always zero in practice.**
    /// The original has two: `Weapon_PostBlastImpulse_q` (ported as
    /// [`crate::wall::post_blast_impulse`], tested against supplied inputs, but its
    /// original caller is found and unread and there is no weapon trigger to drive it)
    /// and an unnamed second writer from what reads like a rival-contact path (no port).
    /// So `apply_pending_impulse` is a correct no-op until a producer is wired.
    pub pending_impulse: Vec3,
    /// Seconds left on the **weapon slowdown timer** at `craft+0x2e0`.
    ///
    /// While it runs the victim gets no engine thrust and a zeroed throttle state
    /// ([`crate::engine::engine`]), no lateral grip ([`crate::forces::evaluate`]) and a
    /// hover target lowered by `min(timer, 4.0)` ([`crate::hover::target_height`]): a
    /// craft hit by a rocket coasts, slides and sinks.
    ///
    /// A weapon impact credits the victim's pending slot (`entity+0x130`) with the
    /// weapon's `slowdown_time`, and each tick the craft update drains it into this timer
    /// through `Ship_AddSlowdown` (`0x08848690`), clamped to `<Global slowdown_limit>`.
    /// See [`crate::slowdown`] and `engine.md`, "The slowdown mechanic, recovered end to
    /// end"; confidence 85-92 per claim, all static. Armed here by
    /// [`crate::slowdown::add`]; the pending slot lives in `oag-gameplay`, which can see
    /// the weapon table.
    ///
    /// Counted down in [`crate::forces::evaluate`] and **not clamped to zero**; see
    /// [`crate::slowdown`] for why that differs from the sideshift timers.
    pub slowdown_timer: f32,
    /// How grounded the ship is, quantised to `{0.0, 0.5, 1.0}`: probes in contact over two.
    pub grounded: f32,
    /// Last frame's [`Self::grounded`], which **every control term reads**.
    ///
    /// Hover is step 8 of 15 in the original's craft update and clears the contact flag
    /// on entry, so the engine, brakes, quadratic drag, gravity and pitch see the
    /// *previous* frame's groundedness, while lateral grip and the weathervane (which run
    /// after hover) see this frame's. Confidence 84; see `engine.md`. Resolving contacts
    /// and then applying forces gets a different answer on every takeoff and landing
    /// frame in five terms at once.
    pub grounded_prev: f32,
    /// Seconds left on each sideshift, left then right.
    ///
    /// One timer per side (`entity+0x8a4`, `entity+0x8a8`); the fired one is set to
    /// [`crate::airbrake::SIDESHIFT_DURATION`], both count down by `dt` and each drives a
    /// craft flag, so both can run at once and the forces cancel. From
    /// `Ship_UpdateSideshiftInput_q` (`0x08846a54`); see `engine.md`.
    pub sideshift_timers: [f32; 2],
    /// Seconds left on each side's double-tap window, left then right
    /// (`entity+0x89c`, `entity+0x8a0`).
    ///
    /// The first airbrake press opens its side's window at
    /// [`crate::airbrake::SIDESHIFT_TAP_WINDOW`]; a second press inside it fires the
    /// shift. Veteran scheme only; see `input-bindings.md`.
    pub shift_tap_windows: [f32; 2],
    /// Whether a novice-scheme flick is armed (`entity+0x860 & 0x400`).
    ///
    /// Set while the sideshift button is held and the axis is inside
    /// [`crate::airbrake::SIDESHIFT_FLICK_THRESHOLD`], cleared when a flick fires;
    /// without it a held-over axis would fire every tick.
    pub shift_armed: bool,
    /// Seconds before another sideshift may be triggered (`entity+0x8ac`).
    ///
    /// Refreshed to [`crate::airbrake::SIDESHIFT_LOCKOUT`] every tick either shift timer
    /// runs, so it is a gap between the end of one shift and the start of the next. Gates
    /// both gesture machines, not the direct [`ShipControls::sideshift`].
    pub shift_lockout: f32,
    /// The barrel roll's three-entry tap-history buffer, oldest first
    /// (`entity+0x88c`/`+0x890`/`+0x894`).
    ///
    /// `0` is empty, `1` a `LEFT` tap, `2` a `RIGHT`. A completed alternation (`[2, 1, 2]`
    /// or `[1, 2, 1]`) arms the roll and clears this, so a fourth tap starts afresh. See
    /// [`crate::barrel_roll`] and `input-bindings.md`.
    pub roll_taps: [u8; 3],
    /// Seconds since the last tap recorded into [`Self::roll_taps`] (`entity+0x884`).
    ///
    /// Zeroed on every tap. A tap at `0.6` s or later does not shift the older entries;
    /// see [`crate::barrel_roll::INTER_TAP_TIMEOUT`].
    pub roll_tap_timer: f32,
    /// The barrel roll's signed phase, `-1.0..=1.0` (`entity+0x87c`). Ramps toward
    /// [`Self::roll_target`] at `<Special roll_speed>` and does not reverse on its own
    /// ([`crate::barrel_roll::advance_phase`]).
    pub roll_phase: f32,
    /// Where [`Self::roll_phase`] is ramping: `1.0` after a `[2, 1, 2]` arm, `-1.0` after
    /// `[1, 2, 1]`, `0.0` once released short of completion ([`crate::barrel_roll::release`]).
    pub roll_target: f32,
    /// Whether a roll is armed and not yet spent (`entity+0x860 & 0x100`/`& 0x80`): set by
    /// [`crate::barrel_roll::arm`], cleared by the landing in
    /// [`crate::barrel_roll::release`], so one roll pays out once. Hashed only while set,
    /// so a ship that never rolls keeps its committed hash.
    pub roll_armed: bool,
    /// Seconds left on the barrel roll's landing payout, ours: the original's
    /// `craft+0x1c0 & 0x400`, held for `<Special roll_turbotime>` after a completed roll
    /// lands. While it runs, [`crate::airbrake::lateral_grip`] is scaled 1.5x, the hover
    /// rebound coefficient is forced to `1.0`, and the engine grants the same uncapped
    /// turbo add as [`Self::turbo_timer`]. See `input-bindings.md`.
    pub roll_payout_timer: f32,
    /// Which side of [`crate::barrel_roll::AXIS_TAP_THRESHOLD`] the steering axis sat on
    /// at the end of last tick, or `None` for neither.
    ///
    /// **This crate's own resolution, not a traced field.** A crossing is an edge and
    /// needs last tick's side; where the original keeps that was not traced, so it lives
    /// with the gesture's per-craft state (as [`Self::shift_armed`] does). A tap fires
    /// whenever this changes *into* a side, so `LEFT` straight to `RIGHT` is one tap. See
    /// [`crate::barrel_roll::advance_gesture`].
    pub roll_axis_zone: Option<crate::barrel_roll::TapDirection>,
    /// The craft is in the original's **grid state**, `craft+0x2a4 == 0`: placed on the
    /// start line and held until the green light.
    ///
    /// `Race_PlaceGrid` puts every craft in state `0` and `Race_StartRacing` moves it to
    /// `1` (`Craft_SetState`, `0x08848590`). This gates `Ship_HoverTwoPoint`'s
    /// bank-to-yaw coupling (`if (craft+0x2a4 != 0)`, `0x0884ad2c`-`0x0884ad40`), which is
    /// why a craft on a banked grid holds its heading through the countdown and yaws at
    /// GO. Measured on PPSSPP 2026-10-01; see `docs/physics/grid-state.md`.
    ///
    /// Written by the race, which owns the countdown; physics only reads it. `false` for
    /// a craft never on a grid, so a bare test and a mid-race respawn see the racing law.
    ///
    /// **Measured on Pulse PSP only** (Time Trial and a Single Race grid, two circuits).
    /// Applying it to every title is **chosen, not measured**.
    ///
    /// State `0` changes more, not ported here: roll damping is `-5` not `-2`, the
    /// `rebound` base is `1.0`, and the control record's airbrakes are forced full. See
    /// `grid-state.md`.
    pub on_grid: bool,
    /// The craft has left the grid state, `craft+0x2a4 == 1`: `false` through the
    /// countdown **in every mode, Zone included**, `true` for a craft never on a grid.
    ///
    /// The same fact as `!`[`Self::on_grid`]. The launch boost's clock starts here
    /// ([`crate::launch`]); Zone's auto-speed uses the same multiplier. Written by the
    /// race, not hashed, for [`Self::on_grid`]'s reason.
    pub released: bool,
    /// The craft runs `Ship_HoverFourCorner` rather than `Ship_HoverTwoPoint`:
    /// `Ship_UpdateHover` (`0x0884870c`) picks it each frame on
    /// `g_game_mode == 6 && g_debug_mode_override == 0` (Zone). Today it changes only the
    /// bank-to-yaw gain (`50.0` against `30.0`,
    /// [`crate::hover::BANK_TO_YAW_GAIN_FOUR_CORNER`]); the four-corner probe layout and
    /// downforce law are not ported. Written by the race, not hashed.
    pub four_corner: bool,
    /// The grid clamp on the pre-scale hover target, `craft+0x348`: `Some` where the title holds
    /// the craft's hover low on the grid and releases it after the green light
    /// ([`crate::hover::capped_target_height`]). Written by the race from the countdown clock
    /// every tick, not hashed, for [`Self::on_grid`]'s reason. `None`: the ordinary target.
    pub hover_cap: Option<f32>,
    /// The hover probe set ([`crate::hover::Rig`]): Pulse's two-point law unless the title
    /// measured its own. Written by the race from the title, not hashed, for
    /// [`Self::on_grid`]'s reason.
    pub hover_rig: crate::hover::Rig,
    /// The launch boost, `craft+0x294`; see [`crate::launch`]. Idle unless the race
    /// supplies the disc's `<StartBoost>` (`Environment::start_boost`), and hashed only
    /// once it is not.
    pub launch: crate::launch::LaunchState,
    /// Seconds since the ship last touched down. Below 0.2 the suspension uses
    /// `landing_rebound` in place of `rebound`. **Armed in the air, not on touchdown**;
    /// see [`Self::time_airborne`].
    pub time_since_landing: f32,
    /// Seconds the ship has been off the ground, `craft+0x284`.
    ///
    /// `Ship_UpdateCraft` (`0x08849df0`) keeps this and its mirror `craft+0x288` as a
    /// pair: the true one accumulates `dt` while the other is zeroed. Confidence **95**;
    /// the grounded twin `+0x288` goes unmodelled because nothing else reads it.
    ///
    /// `Ship_HoverTwoPoint` zeroes [`Self::time_since_landing`] **while still airborne**,
    /// once this passes `Antigrav::rebound_jump_time`. A shorter hop never arms the
    /// landing response; only a real flight earns the `landing_rebound` bounce. See
    /// `engine.md`.
    pub time_airborne: f32,
    /// The 0-to-1 magstrip blend. At 1.0 the ordinary suspension is fully cancelled and the
    /// magnetic hold has taken over. Driven by [`crate::maglock::ramp`], `0.2` a frame.
    pub mag_lock_blend: f32,
    /// Seconds left on the speed-pad boost, `craft+0x298`.
    ///
    /// **Re-armed, not triggered.** `Ship_ApplySpeedupPad` (`0x08848f9c`) assigns the
    /// class's `<SpeedupPads time>` on *every* tick inside a pad volume and decrements
    /// further down the same function, so the boost runs for `time` from the last tick
    /// inside the pad, and a slow crossing boosts longer
    /// ([`crate::forces::Environment::pad_hit`] is `Some` every such tick).
    ///
    /// Counted down in [`crate::forces::evaluate`] **before** the force is read off it,
    /// the original's order. The two orders differ by one `dt` of ramp at the peak (about
    /// 6 %), which `docs/physics/cornering-ground-truth.md` measured precisely enough to
    /// tell apart.
    pub pad_timer: f32,
    /// The world-space direction the speed-pad boost pushes, `craft+0x1b0`.
    ///
    /// Refreshed on every tick inside a pad, then held while [`Self::pad_timer`] runs
    /// down, so the boost direction does not follow the ship. Unit length from a pad,
    /// zero on a ship that never touched one.
    pub pad_direction: Vec3,
    /// The **last** mag-floor hit the probe found, `craft+0x250` and `+0x260`.
    ///
    /// Carried between frames because the original does: those fields are a raycast
    /// out-parameter only a hit overwrites, and [`mag_lock_blend`](Self::mag_lock_blend)
    /// takes five frames to decay after the strip ends, so the hold's last five frames read
    /// a stale contact in the original too. `None` is a ship that never touched a strip.
    pub mag_contact: Option<crate::maglock::MagContact>,
    /// The energy pool, the original's `entity+0x88`.
    ///
    /// On the ship entity, one hop out at `craft+0x1c4`, **not** the craft the trace
    /// harness breaks on (whose `+0x88` is an orientation-matrix element). Settled live
    /// 2026-08-10 by the reciprocal pointer identity: the entity's `+0x94` points back at
    /// the craft. Recovered in `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    ///
    /// Bounded above by [`crate::Dimensions::shield`] and floored at zero, **the one place
    /// this crate knowingly departs from the original**: `Ship_SetShield` clamps above
    /// only, because `Ship_Damage` tests the signed result to enter the destroyed state.
    /// This engine has no destroyed state yet, so a negative pool would be a number
    /// nothing could act on. When that transition lands, this floor is what moves.
    ///
    /// In this crate and hashed, because the original keeps it on the craft: it is
    /// written from the contact response ([`crate::damage::apply_contact`]), and a pool
    /// outside [`ShipState`] would be a simulation field the determinism gate cannot see.
    pub shield: f32,
    /// Where the craft is in the destroyed sequence, the original's `entity+0x8c`; only
    /// the three states the energy pool reaches ([`crate::damage::CraftState`]).
    pub craft_state: crate::damage::CraftState,
    /// Seconds left on the current craft state, `entity+0x874`. Only
    /// [`crate::damage::CraftState::Destroyed`] runs it down; zero otherwise, which keeps
    /// it out of the hash on a race nobody dies in.
    pub state_timer: f32,
    /// Seconds left on a fired Turbo pickup.
    ///
    /// While positive the engine **adds `Engine.turbo` to thrust**, after the
    /// acceleration cap and before the fixed doubling, so the add is uncapped
    /// ([`crate::engine::engine`]).
    ///
    /// `Ship_UpdateEngine` (`0x0884c5c8`) gates the add on
    /// `((flags & 0x200) || (flags & 0x400)) && craft+0x2a4 == 1` (decompiler, 2026-08-11).
    /// `0x400` is held for `<Special roll_turbotime>` after a completed barrel roll lands
    /// (confidence 90; see [`Self::roll_payout_timer`], kept separate because `0x400` also
    /// drives a lateral-grip multiplier and a hover override that a Turbo pickup must not
    /// get). `0x200` is therefore the Turbo pickup, **but its writer has not been found**,
    /// so that half is inference, confidence 75. See `input-bindings.md`.
    ///
    /// **The timer itself is ours**: the original holds a bit, neither `craft+0x1c0`'s
    /// writer nor the pickup word was read. The **duration** is the disc's
    /// `<Weapon type="Turbo"><Stats time>` and the **magnitude** its `<Engine turbo>`.
    /// **The `craft+0x2a4 == 1` gate is not reproduced** (nothing decodes that enum), so
    /// a craft here can turbo where the original might not. **The boost lift is not
    /// implemented**: the branch adds `g_boost_lift * T` along body up while thrust is
    /// held, and that global was never read (`docs/gameplay/pickups.md`).
    ///
    /// In this crate and hashed, like [`Self::shield`]: written by gameplay, read by the
    /// force law. The *inventory* is not here, because it is an
    /// `oag_tables::weapons::Weapon` and this crate depends on nothing but `oag-core`.
    pub turbo_timer: f32,
    /// Seconds left on a fired Shield pickup.
    ///
    /// While positive the craft takes no damage: [`crate::damage::apply_contact`] returns
    /// early as it does for a craft already blowing up.
    ///
    /// Recovered 2026-08-19. `Shield_Fire` (`0x08861568`, fire bit `0x20`) loads the
    /// weapon-stats block's `+0x8c`, where `WeaponStats_ParseShield` (`0x0880ca2c`) writes
    /// `time`, into `craft+0x188` and raises running bit `0x10`; `Shield_Update`
    /// (`0x08861630`) counts it down. The bit is read by the weapon-damage drain, the
    /// contact loop's damage reaction and `Ship_ApplyCollisionImpulse`. See
    /// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`. It read as unfindable
    /// because Ghidra resolves no cross-reference to any string or constant in this
    /// image: "no reader found" was a property of the search.
    ///
    /// Refusing damage is right: both of the original's drains *discard* the amount on the
    /// shielded branch and arm the hit flash ([`crate::damage::Shield::absorbed`]).
    /// **Still ours:** nothing decrements `Self::shield` by the `absorb` half of
    /// `<Stats>`, and the original's absorb path is a separate button.
    /// [`crate::damage::Shield::depleted`] cannot fire while this runs, so a mode that
    /// ends on `depleted` can be held open by a pickup.
    ///
    /// Not named `shield_timer`: [`Self::shield`] is the energy pool and
    /// [`crate::damage::Shield`] the per-tick contact outcome.
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
            camera_lean_follower: 0.0,
            camera_lean: 0.0,
            reverse_controls: 0.0,
            stun_timer: 0.0,
            wall_contact_prev: false,
            pending_impulse: Vec3::ZERO,
            slowdown_timer: 0.0,
            grounded: 0.0,
            grounded_prev: 0.0,
            sideshift_timers: [0.0, 0.0],
            shift_tap_windows: [0.0, 0.0],
            shift_armed: false,
            shift_lockout: 0.0,
            roll_taps: [0, 0, 0],
            // At or past the timeout, so a cold-started ship's first tap never cascades a
            // stale all-zero history (`crate::barrel_roll::INTER_TAP_TIMEOUT`).
            roll_tap_timer: crate::barrel_roll::INTER_TAP_TIMEOUT,
            roll_phase: 0.0,
            roll_target: 0.0,
            roll_armed: false,
            roll_payout_timer: 0.0,
            roll_axis_zone: None,
            // `Ship_InitCraft` (`0x08849354`) sets `craft+0x2b4` to `10.0`, outside the 0.2 s
            // landing window as a fresh ship must be. See `engine.md`.
            time_since_landing: 10.0,
            on_grid: false,
            four_corner: false,
            hover_cap: None,
            hover_rig: crate::hover::Rig::TWO_POINT,
            released: true,
            launch: crate::launch::LaunchState::default(),
            time_airborne: 0.0,
            mag_lock_blend: 0.0,
            pad_timer: 0.0,
            pad_direction: Vec3::ZERO,
            mag_contact: None,
            // Zero, not a pool: a default ship has no `Dimensions` to take a maximum from.
            // `crate::damage::reset` fills it.
            shield: 0.0,
            craft_state: crate::damage::CraftState::Racing,
            state_timer: 0.0,
            turbo_timer: 0.0,
            shield_pickup_timer: 0.0,
        }
    }
}

impl ShipState {
    /// Whether any hover probe is in contact. Exact, since `grounded` only holds `0.0`,
    /// `0.5` or `1.0`.
    #[must_use]
    pub fn is_grounded(&self) -> bool {
        self.grounded > 0.0
    }

    /// Quantises a probe contact count to `{0.0, 0.5, 1.0}`: the count over two, the whole
    /// of the original's groundedness. The two-probe assumption is baked into the divisor;
    /// the original's four-corner variant has an unknown selection rule.
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
    /// The original computes `F x r` in both binaries: it integrates its basis as
    /// `e' = e x w` rather than `e' = w x e`, so its angular velocity is the textbook one
    /// negated. [`crate::integrate`] uses the textbook form, so every torque expression
    /// from the disassembly is negated exactly once on the way in: here, in
    /// [`crate::hover::ALIGNMENT_GAIN`] (`+400` against a read `-400`) and in
    /// [`crate::passive::WEATHERVANE_GROUND`] (`+0.1` against `-0.1`).
    ///
    /// **Flipping any one of those three without the others diverges unconditionally.**
    /// This asserts the direction, not mere non-zero: `a_force_off_axis_makes_torque`
    /// passes either way round. Angular damping is **not** in that list: `tau = -c * w`
    /// is right in either convention.
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

    /// The frame axis points **down** in `.vex` data and this one does not, so the
    /// difference is pinned rather than left to be rediscovered.
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
