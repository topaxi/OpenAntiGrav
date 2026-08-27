//! The recorded side: one CSV of per-tick ship state, in the original's own
//! conventions.
//!
//! The format is exactly what `scripts/psp-trace.py` writes, and the column list
//! below is a transcription of that script's `CRAFT_FIELDS` and `BODY_FIELDS`
//! rather than a redesign of them. A trace is **derived game data**: it lives
//! under `data/traces/`, which `.gitignore` covers, and is never committed. See
//! `docs/reverse-engineering/verification-protocol.md`.
//!
//! # Two things the recorded numbers are not
//!
//! - **They are not the state after a tick.** The capture breaks on
//!   `Ship_UpdateCraft`'s *entry*, so a row is the craft as the update found it:
//!   the position the previous tick left behind, and control states the previous
//!   tick wrote. A simulated trace must therefore be sampled before its step, not
//!   after, or every field is compared one tick out of phase.
//! - **They are not full precision.** The capture writes `%.7g`, which is about
//!   what an `f32` carries, so a difference below roughly `1e-7` relative is the
//!   file's own rounding rather than a divergence.

use oag_core::math::Vec3;

/// Every column a current capture writes, in the order it writes them.
///
/// Parsing is by name rather than by position - the header is read and each
/// column located in it - so a capture that grows a column stays readable.
/// [`Trace::columns`] is what [`Trace::to_csv`] writes, which is this list
/// filtered down to the columns the trace in hand actually carries.
pub const COLUMNS: [&str; 61] = [
    "tick",
    "dt",
    "grounded",
    "throttle",
    "brake",
    "steer",
    "airbrake_l",
    "airbrake_r",
    "speed_cached",
    "stun_timer",
    "timer_2e0",
    "shield",
    "fov_intercept",
    "fov_additive",
    "ss_tap_window_l",
    "ss_tap_window_r",
    "ss_shift_l",
    "ss_shift_r",
    "ss_lockout",
    "right_x",
    "right_y",
    "right_z",
    "up_x",
    "up_y",
    "up_z",
    "fwd_x",
    "fwd_y",
    "fwd_z",
    "pos_x",
    "pos_y",
    "pos_z",
    "vel_x",
    "vel_y",
    "vel_z",
    "speed",
    "avel_x",
    "avel_y",
    "avel_z",
    "omega_x",
    "omega_y",
    "omega_z",
    "cam_right_x",
    "cam_right_y",
    "cam_right_z",
    "cam_up_x",
    "cam_up_y",
    "cam_up_z",
    "cam_fwd_x",
    "cam_fwd_y",
    "cam_fwd_z",
    "cam_pos_x",
    "cam_pos_y",
    "cam_pos_z",
    "boost_timer",
    "plume_timer",
    "intensity",
    "half_size",
    "engine_on",
    "flare_speed_kmh",
    "speed_ramp",
    "boost_accum",
];

/// The columns every trace must carry, which is what the capture wrote before
/// the angular-velocity and timer columns were added.
///
/// **Captures already taken are not reproducible.** Each one is a hand-driven
/// PPSSPP session, so a column added today can never be backfilled into a file
/// recorded yesterday, and a reader that demanded it would retire the whole
/// existing corpus. So the ones in [`OPTIONAL_COLUMNS`] are read when present and
/// left absent when not - absent, not zero: a zero angular velocity is a claim
/// that the ship was not rotating, and a capture that never measured it must not
/// be able to make that claim. See [`Frame::angular_velocity`].
pub const REQUIRED_COLUMNS: [&str; 25] = [
    "tick",
    "dt",
    "grounded",
    "throttle",
    "brake",
    "steer",
    "airbrake_l",
    "airbrake_r",
    "speed_cached",
    "right_x",
    "right_y",
    "right_z",
    "up_x",
    "up_y",
    "up_z",
    "fwd_x",
    "fwd_y",
    "fwd_z",
    "pos_x",
    "pos_y",
    "pos_z",
    "vel_x",
    "vel_y",
    "vel_z",
    "speed",
];

/// The columns a trace may carry and an older one does not.
pub const OPTIONAL_COLUMNS: [&str; 36] = [
    "stun_timer",
    "timer_2e0",
    "shield",
    "fov_intercept",
    "fov_additive",
    "ss_tap_window_l",
    "ss_tap_window_r",
    "ss_shift_l",
    "ss_shift_r",
    "ss_lockout",
    "avel_x",
    "avel_y",
    "avel_z",
    "omega_x",
    "omega_y",
    "omega_z",
    "cam_right_x",
    "cam_right_y",
    "cam_right_z",
    "cam_up_x",
    "cam_up_y",
    "cam_up_z",
    "cam_fwd_x",
    "cam_fwd_y",
    "cam_fwd_z",
    "cam_pos_x",
    "cam_pos_y",
    "cam_pos_z",
    "boost_timer",
    "plume_timer",
    "intensity",
    "half_size",
    "engine_on",
    "flare_speed_kmh",
    "speed_ramp",
    "boost_accum",
];

/// The three columns of the angular **momentum** at `body+0x160`, which are all
/// present or all absent: two thirds of a vector is a broken capture, not an old
/// one.
pub const ANGULAR_COLUMNS: [&str; 3] = ["avel_x", "avel_y", "avel_z"];

/// The three columns of the angular **velocity** at `body+0x150`, all present or
/// all absent for the same reason as [`ANGULAR_COLUMNS`].
pub const ANGULAR_RATE_COLUMNS: [&str; 3] = ["omega_x", "omega_y", "omega_z"];

/// The twelve columns of the player camera node, written by
/// `scripts/psp-trace.py --camera`: the node's three basis rows plus its eye
/// position, all present or all absent - a partial pose is a broken capture,
/// not an old one.
///
/// The rows are recorded as the node holds them; whether `cam_right` is the
/// camera's right or its left, and whether `cam_fwd` is the look direction or
/// its negation, is settled by rendering from a captured pose, not assumed
/// here. The eye is stored **un-negated**: the node's `+0x30` holds the negated
/// eye position (`docs/ghidra/functions/psp-pulse-usa/camera.md`), and the capture
/// flips the sign at write time so this file's `cam_pos_*` is a world position
/// like `pos_*`.
pub const CAMERA_COLUMNS: [&str; 12] = [
    "cam_right_x",
    "cam_right_y",
    "cam_right_z",
    "cam_up_x",
    "cam_up_y",
    "cam_up_z",
    "cam_fwd_x",
    "cam_fwd_y",
    "cam_fwd_z",
    "cam_pos_x",
    "cam_pos_y",
    "cam_pos_z",
];

mod flare;

pub use flare::{FLARE_COLUMNS, Flare};

/// One tick of ship state, as the original held it.
///
/// Every field is in the original's units and the original's frame. Nothing is
/// converted on the way in: a harness that quietly rescaled the recording would
/// be comparing against its own assumptions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Which tick of the capture this is. The capture numbers from zero.
    pub tick: u64,
    /// The frame delta the original was about to integrate, in seconds.
    ///
    /// Variable, mean `1/59.94` over the captures taken so far - which is the
    /// measurement behind ADR-0007 - so a comparison run should normally be fed
    /// this rather than a fixed 60 Hz step.
    pub dt: f32,
    /// Probes in contact over two: `0.0`, `0.5` or `1.0`. Compared exactly.
    pub grounded: f32,
    /// Throttle, on the original's `0..=100` control scale. Raw input, not ramped.
    pub throttle: f32,
    /// Brake, `0..=100`, ramped.
    pub brake: f32,
    /// Steering, `-100..=100`, ramped.
    pub steer: f32,
    /// Left airbrake, `0..=100`, ramped.
    pub airbrake_left: f32,
    /// Right airbrake, `0..=100`, ramped.
    pub airbrake_right: f32,
    /// The craft's cached speed at `craft+0x2ec`, which is **one tick stale**
    /// (199/199 ticks of the capture this was measured on).
    pub speed_cached: f32,
    /// Row 0 of the body's basis - the CSV's `right_*` columns.
    ///
    /// **Named `right` in the capture and measured to be the ship's left.**
    /// Holding left gives `steer = -96` and `+1.51 rad/s` about row 1, holding
    /// right the mirror of that, 199/199 consistent; so forward rotates *toward*
    /// row 0 on a left turn. Confidence 84, `docs/overview/roadmap.md`. The field
    /// is called `row0` here so that nothing in this crate can read a sign out of
    /// a name, and [`crate::replay::Basis`] is where the reconciliation happens -
    /// as a switch, because the finding is exactly the kind a trace comparison
    /// exists to falsify.
    pub row0: Vec3,
    /// Row 1 of the basis: the ship's up axis.
    pub up: Vec3,
    /// Row 2 of the basis: the ship's forward axis.
    pub forward: Vec3,
    /// Centre of mass in world space.
    pub position: Vec3,
    /// Linear velocity in world space.
    pub velocity: Vec3,
    /// The body's own speed field, which is the velocity's length.
    pub speed: f32,
    /// Angular **momentum** at `body+0x160`, **raw, in the original's own sign
    /// and frame**, or `None` for a capture taken before the column existed.
    ///
    /// The field keeps its name, which is now a misnomer, because it is the
    /// column every capture in `data/traces/` since the angular pass carries and
    /// every number quoted against it is quoted under this name. What it holds is
    /// `I * omega` in body coordinates, negated: `Body_Integrate` integrates
    /// torque into it with no inertia and no mass divide (confidence 88), and two
    /// captures with a held pitch input measure `body+0x160 = I * body+0x150`
    /// directly at `100.0 %` explained on the pitch axis. See
    /// [`Frame::angular_rate`], which is the plain rotation rate and the one to
    /// compare against.
    ///
    /// The offset rests on two legs, one per binary: PSP `Body_ClearVelocity`
    /// (`0x0884da5c`) zeroes `body+0x140` and `body+0x160` and nothing else, and
    /// `+0x140` is the linear velocity this capture already verifies against the
    /// position delta; PS2 `Ship_ApplyAngularDamping` (`0x0015c1b0`) reads
    /// `body+0x160` as the quantity it damps.
    ///
    /// Its sign and frame are **negated body-local**, settled by fit rather than
    /// by assumption - see [`AngularReading`] and [`Summary::angular_readings`].
    ///
    /// `None` means "this capture did not measure it", which is not
    /// `Some(Vec3::ZERO)`: the second is a claim that the ship was not rotating.
    /// Everything downstream treats the absence as *not compared* rather than as
    /// agreement.
    pub angular_velocity: Option<Vec3>,
    /// Angular **velocity** at `body+0x150`, raw, or `None` for a capture taken
    /// before the column existed.
    ///
    /// This is the rotation rate by definition - `Body_Integrate` advances the
    /// basis by `+0x150` - so it is the only angular quantity in a capture that
    /// is directly comparable with `oag_physics::Body::angular_velocity`, with no
    /// inertia and no fitted scale in between. Measured against the rotation the
    /// recorded basis performs it fits at `-1.0011`, `-0.9999` and `-0.9993` on
    /// the three body axes, so the reading is [`AngularReading::NegatedLocal`]
    /// like the momentum column and the `w_game = -w_physics` negation lives
    /// here rather than between the two stored columns.
    ///
    /// `None` is "not measured", exactly as above.
    pub angular_rate: Option<Vec3>,
    /// The collision stun timer at `craft+0x290`, in seconds, or `None` for a
    /// capture taken before the column existed.
    ///
    /// Above zero it means the frame produced **no thrust and no lateral grip**:
    /// `Ship_UpdateEngine`'s prologue returns having zeroed the throttle state
    /// (`0x0884c634`, confidence 88) and `Ship_ApplyLateralGrip` returns early
    /// (`0x08848b78`). `Ship_ApplyCollisionImpulse` (`0x0883f274`) arms it with
    /// `+= 0.5` on a hit, so it doubles as the capture's wall-contact indicator.
    pub stun_timer: Option<f32>,
    /// The second gate on the same early return, at `craft+0x2e0`, or `None`.
    ///
    /// Above zero with flag `0x10` clear it kills thrust exactly as the stun
    /// timer does. **What arms it has never been read**, so it carries its offset
    /// for a name rather than a guess: below 50 confidence nothing gets named
    /// (ADR-0005). Recorded because a capture is the only thing that can say
    /// which of the two gates fired.
    pub timer_2e0: Option<f32>,
    /// The energy pool at the original's `entity+0x88`, or `None` for a capture
    /// taken before the column existed.
    ///
    /// The field `Ship_Shield` reads and `Ship_SetShield` writes, clamped above
    /// by the skill-indexed `<Misc>` maximum and **not** floored at zero - see
    /// `docs/ghidra/functions/psp-pulse-usa/shield.md`. It is the only column
    /// that can test the recovered contact-damage law `|p| * 0.035`, and the
    /// test has to allow for `Ship_Damage` halving the amount whenever the race
    /// has weapons off, which a time trial does.
    ///
    /// **A flat column does not mean nothing happened.** With damage off the
    /// original regenerates the pool at 4 a second and floors it at 20, so a
    /// capture taken in that mode can absorb a real hit and read almost
    /// unchanged one tick later.
    pub shield: Option<f32>,
    /// `entity+0x7c`, `FUN_088455ec`'s still-unidentified fov intercept, or
    /// `None`. Measured `0` on every capture so far, wall contact included; see
    /// `docs/ghidra/functions/psp-pulse-usa/camera.md`.
    pub fov_intercept: Option<f32>,
    /// `entity+0x790` - `dot(fwd, vel) * 0.075 + fov_intercept`, `camera.md`'s
    /// `SPEED_FOV_GAIN_DEG` term - or `None`. Has a second writer, `Hud_Update`'s
    /// impact shake, so a nonzero value alone does not say which one fired.
    pub fov_additive: Option<f32>,
    /// The veteran double-tap's left tap window, `entity+0x89c`, or `None` for a
    /// capture taken before the column existed.
    ///
    /// Opens to `0.25` on a first left-airbrake tap and counts down by `dt`;
    /// a second tap while it is still positive is what fires [`Self::ss_shift_l`].
    /// See `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`, confirmed at
    /// runtime against a capture of `sideshift-double-tap.inputs`.
    pub ss_tap_window_l: Option<f32>,
    /// The mirror of [`Self::ss_tap_window_l`] for the right airbrake,
    /// `entity+0x8a0`.
    pub ss_tap_window_r: Option<f32>,
    /// The veteran double-tap's left shift force timer, `entity+0x8a4`, or
    /// `None`.
    ///
    /// Set to `0.2` on the tap window's second tap; the already-ported force
    /// itself lives in `crates/physics/src/airbrake.rs`.
    pub ss_shift_l: Option<f32>,
    /// The mirror of [`Self::ss_shift_l`] for the right airbrake,
    /// `entity+0x8a8`.
    pub ss_shift_r: Option<f32>,
    /// The lockout common to both sideshift schemes, `entity+0x8ac`, or `None`.
    ///
    /// Set to `1.0` on any tick either shift timer is positive and counted down
    /// by `dt` - one second between sideshifts, novice or veteran.
    pub ss_lockout: Option<f32>,
    /// Row 0 of the player camera node's basis, or `None` for a capture taken
    /// without `--camera`.
    ///
    /// Recorded as the node holds it; see [`CAMERA_COLUMNS`] for what is and is
    /// not claimed about its sign. The ship's `right_*` trap ([`Frame::row0`])
    /// does **not** transfer here: the camera node is a scene node, not the
    /// rigid body, and its handedness is settled by rendering from a captured
    /// pose.
    pub camera_row0: Option<Vec3>,
    /// Row 1 of the camera node's basis, or `None`.
    pub camera_up: Option<Vec3>,
    /// Row 2 of the camera node's basis, or `None`.
    pub camera_forward: Option<Vec3>,
    /// The camera eye in world space, or `None`.
    ///
    /// Already un-negated by the capture: the node's `+0x30` holds the negated
    /// eye and `scripts/psp-trace.py` flips it before writing, so this compares
    /// directly against a world position.
    pub camera_position: Option<Vec3>,
    /// The exhaust flare's eight fields, or `None` for a capture taken without
    /// `--flare`.
    ///
    /// Absent, not zeroed, for the same reason the angular columns are: a zero
    /// `boost_timer` is a claim that the ship was not boosting, and a capture
    /// that never looked at the flare must not be able to make it.
    pub flare: Option<Flare>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            tick: 0,
            dt: 0.0,
            grounded: 0.0,
            throttle: 0.0,
            brake: 0.0,
            steer: 0.0,
            airbrake_left: 0.0,
            airbrake_right: 0.0,
            speed_cached: 0.0,
            row0: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::Z,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            speed: 0.0,
            angular_velocity: None,
            angular_rate: None,
            stun_timer: None,
            timer_2e0: None,
            shield: None,
            fov_intercept: None,
            fov_additive: None,
            ss_tap_window_l: None,
            ss_tap_window_r: None,
            ss_shift_l: None,
            ss_shift_r: None,
            ss_lockout: None,
            camera_row0: None,
            camera_up: None,
            camera_forward: None,
            camera_position: None,
            flare: None,
        }
    }
}

impl Frame {
    /// Whether the basis is positively oriented under ordinary component
    /// arithmetic: `cross(row0, up) = forward`.
    ///
    /// Measured true on 200 of 200 ticks of the first real capture, which is what
    /// removed frame handedness as an explanation for the two disputed cross
    /// product signs in the force law. Kept here so any future trace is checked
    /// against the same invariant rather than the claim being taken on trust.
    #[must_use]
    pub fn basis_is_positively_oriented(&self, tolerance: f32) -> bool {
        (self.row0.cross(self.up) - self.forward).length() <= tolerance
    }

    /// The recorded basis rows as a triple, in the file's own order.
    #[must_use]
    pub fn rows(&self) -> (Vec3, Vec3, Vec3) {
        (self.row0, self.up, self.forward)
    }

    /// The world-space angular velocity that carries this tick's basis onto the
    /// next one, in the ordinary physical sign, or `None` for a zero `dt`.
    ///
    /// For an orthonormal triad carried by a rotation of angle `t` about `n`,
    /// Rodrigues gives `sum cross(r_i, r_i') = 2 * n * sin(t)` **exactly** - the
    /// `dot(n, r)` terms cancel over the triad and the third term sums to
    /// `n x n`. So half the summed cross product over the frame delta recovers
    /// `n * sin(t)/dt`, which underestimates the true `n * t/dt` by `t^2/6`. At
    /// 60 Hz and the 1.5 rad/s the captures show, `t` is about a fortieth of a
    /// radian and that is one part in ten thousand: 1.6e-4 rad/s, well inside any
    /// tolerance this is used at.
    ///
    /// This is what makes the recorded angular-velocity column checkable against
    /// the recording itself rather than against a simulation: see
    /// [`Summary::angular_readings`].
    #[must_use]
    pub fn basis_rotation_rate(&self, next: &Self) -> Option<Vec3> {
        // A NaN delta is not a positive one, so the comparison is written the way
        // round that answers `None` for it rather than dividing by it.
        if self.dt.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return None;
        }
        let sum =
            self.row0.cross(next.row0) + self.up.cross(next.up) + self.forward.cross(next.forward);
        Some(sum / (2.0 * self.dt))
    }

    fn value(&self, column: &str) -> Option<f64> {
        Some(f64::from(match column {
            "tick" => return Some(self.tick as f64),
            "dt" => self.dt,
            "grounded" => self.grounded,
            "throttle" => self.throttle,
            "brake" => self.brake,
            "steer" => self.steer,
            "airbrake_l" => self.airbrake_left,
            "airbrake_r" => self.airbrake_right,
            "speed_cached" => self.speed_cached,
            "right_x" => self.row0.x,
            "right_y" => self.row0.y,
            "right_z" => self.row0.z,
            "up_x" => self.up.x,
            "up_y" => self.up.y,
            "up_z" => self.up.z,
            "fwd_x" => self.forward.x,
            "fwd_y" => self.forward.y,
            "fwd_z" => self.forward.z,
            "pos_x" => self.position.x,
            "pos_y" => self.position.y,
            "pos_z" => self.position.z,
            "vel_x" => self.velocity.x,
            "vel_y" => self.velocity.y,
            "vel_z" => self.velocity.z,
            "speed" => self.speed,
            "avel_x" => self.angular_velocity?.x,
            "avel_y" => self.angular_velocity?.y,
            "avel_z" => self.angular_velocity?.z,
            "omega_x" => self.angular_rate?.x,
            "omega_y" => self.angular_rate?.y,
            "omega_z" => self.angular_rate?.z,
            "stun_timer" => self.stun_timer?,
            "timer_2e0" => self.timer_2e0?,
            "shield" => self.shield?,
            "fov_intercept" => self.fov_intercept?,
            "fov_additive" => self.fov_additive?,
            "ss_tap_window_l" => self.ss_tap_window_l?,
            "ss_tap_window_r" => self.ss_tap_window_r?,
            "ss_shift_l" => self.ss_shift_l?,
            "ss_shift_r" => self.ss_shift_r?,
            "ss_lockout" => self.ss_lockout?,
            "cam_right_x" => self.camera_row0?.x,
            "cam_right_y" => self.camera_row0?.y,
            "cam_right_z" => self.camera_row0?.z,
            "cam_up_x" => self.camera_up?.x,
            "cam_up_y" => self.camera_up?.y,
            "cam_up_z" => self.camera_up?.z,
            "cam_fwd_x" => self.camera_forward?.x,
            "cam_fwd_y" => self.camera_forward?.y,
            "cam_fwd_z" => self.camera_forward?.z,
            "cam_pos_x" => self.camera_position?.x,
            "cam_pos_y" => self.camera_position?.y,
            "cam_pos_z" => self.camera_position?.z,
            "boost_timer" => self.flare?.boost_timer,
            "plume_timer" => self.flare?.plume_timer,
            "intensity" => self.flare?.intensity,
            "half_size" => self.flare?.half_size,
            "engine_on" => self.flare?.engine_on,
            "flare_speed_kmh" => self.flare?.speed_kmh,
            "speed_ramp" => self.flare?.speed_ramp,
            "boost_accum" => self.flare?.boost_accumulator,
            _ => return None,
        }))
    }

    /// Whether the frame carries a column, which for an optional one is whether
    /// the capture it came from measured it at all.
    #[must_use]
    pub fn has(&self, column: &str) -> bool {
        match column {
            "avel_x" | "avel_y" | "avel_z" => self.angular_velocity.is_some(),
            "omega_x" | "omega_y" | "omega_z" => self.angular_rate.is_some(),
            "stun_timer" => self.stun_timer.is_some(),
            "timer_2e0" => self.timer_2e0.is_some(),
            "shield" => self.shield.is_some(),
            "fov_intercept" => self.fov_intercept.is_some(),
            "fov_additive" => self.fov_additive.is_some(),
            "ss_tap_window_l" => self.ss_tap_window_l.is_some(),
            "ss_tap_window_r" => self.ss_tap_window_r.is_some(),
            "ss_shift_l" => self.ss_shift_l.is_some(),
            "ss_shift_r" => self.ss_shift_r.is_some(),
            "ss_lockout" => self.ss_lockout.is_some(),
            "cam_right_x" | "cam_right_y" | "cam_right_z" => self.camera_row0.is_some(),
            "cam_up_x" | "cam_up_y" | "cam_up_z" => self.camera_up.is_some(),
            "cam_fwd_x" | "cam_fwd_y" | "cam_fwd_z" => self.camera_forward.is_some(),
            "cam_pos_x" | "cam_pos_y" | "cam_pos_z" => self.camera_position.is_some(),
            other if FLARE_COLUMNS.contains(&other) => self.flare.is_some(),
            other => REQUIRED_COLUMNS.contains(&other),
        }
    }

    fn set(&mut self, column: &str, value: f32) {
        match column {
            "tick" => self.tick = value as u64,
            "dt" => self.dt = value,
            "grounded" => self.grounded = value,
            "throttle" => self.throttle = value,
            "brake" => self.brake = value,
            "steer" => self.steer = value,
            "airbrake_l" => self.airbrake_left = value,
            "airbrake_r" => self.airbrake_right = value,
            "speed_cached" => self.speed_cached = value,
            "right_x" => self.row0.x = value,
            "right_y" => self.row0.y = value,
            "right_z" => self.row0.z = value,
            "up_x" => self.up.x = value,
            "up_y" => self.up.y = value,
            "up_z" => self.up.z = value,
            "fwd_x" => self.forward.x = value,
            "fwd_y" => self.forward.y = value,
            "fwd_z" => self.forward.z = value,
            "pos_x" => self.position.x = value,
            "pos_y" => self.position.y = value,
            "pos_z" => self.position.z = value,
            "vel_x" => self.velocity.x = value,
            "vel_y" => self.velocity.y = value,
            "vel_z" => self.velocity.z = value,
            "speed" => self.speed = value,
            // The three angular columns arrive one at a time, so the first of
            // them promotes the field out of `None`; `Trace::parse` has already
            // rejected a header carrying some of them and not others.
            "avel_x" => self.angular_velocity.get_or_insert(Vec3::ZERO).x = value,
            "avel_y" => self.angular_velocity.get_or_insert(Vec3::ZERO).y = value,
            "avel_z" => self.angular_velocity.get_or_insert(Vec3::ZERO).z = value,
            "omega_x" => self.angular_rate.get_or_insert(Vec3::ZERO).x = value,
            "omega_y" => self.angular_rate.get_or_insert(Vec3::ZERO).y = value,
            "omega_z" => self.angular_rate.get_or_insert(Vec3::ZERO).z = value,
            "stun_timer" => self.stun_timer = Some(value),
            "timer_2e0" => self.timer_2e0 = Some(value),
            "shield" => self.shield = Some(value),
            "fov_intercept" => self.fov_intercept = Some(value),
            "fov_additive" => self.fov_additive = Some(value),
            "ss_tap_window_l" => self.ss_tap_window_l = Some(value),
            "ss_tap_window_r" => self.ss_tap_window_r = Some(value),
            "ss_shift_l" => self.ss_shift_l = Some(value),
            "ss_shift_r" => self.ss_shift_r = Some(value),
            "ss_lockout" => self.ss_lockout = Some(value),
            "cam_right_x" => self.camera_row0.get_or_insert(Vec3::ZERO).x = value,
            "cam_right_y" => self.camera_row0.get_or_insert(Vec3::ZERO).y = value,
            "cam_right_z" => self.camera_row0.get_or_insert(Vec3::ZERO).z = value,
            "cam_up_x" => self.camera_up.get_or_insert(Vec3::ZERO).x = value,
            "cam_up_y" => self.camera_up.get_or_insert(Vec3::ZERO).y = value,
            "cam_up_z" => self.camera_up.get_or_insert(Vec3::ZERO).z = value,
            "cam_fwd_x" => self.camera_forward.get_or_insert(Vec3::ZERO).x = value,
            "cam_fwd_y" => self.camera_forward.get_or_insert(Vec3::ZERO).y = value,
            "cam_fwd_z" => self.camera_forward.get_or_insert(Vec3::ZERO).z = value,
            "cam_pos_x" => self.camera_position.get_or_insert(Vec3::ZERO).x = value,
            "cam_pos_y" => self.camera_position.get_or_insert(Vec3::ZERO).y = value,
            "cam_pos_z" => self.camera_position.get_or_insert(Vec3::ZERO).z = value,
            // Same promotion rule as the vectors above: the eight arrive one at
            // a time, so the first promotes the group out of `None`, and
            // `Trace::parse` has already rejected a header carrying some of them
            // and not the rest.
            "boost_timer" => self.flare.get_or_insert_with(Flare::default).boost_timer = value,
            "plume_timer" => self.flare.get_or_insert_with(Flare::default).plume_timer = value,
            "intensity" => self.flare.get_or_insert_with(Flare::default).intensity = value,
            "half_size" => self.flare.get_or_insert_with(Flare::default).half_size = value,
            "engine_on" => self.flare.get_or_insert_with(Flare::default).engine_on = value,
            "flare_speed_kmh" => self.flare.get_or_insert_with(Flare::default).speed_kmh = value,
            "speed_ramp" => self.flare.get_or_insert_with(Flare::default).speed_ramp = value,
            "boost_accum" => {
                self.flare
                    .get_or_insert_with(Flare::default)
                    .boost_accumulator = value;
            }
            _ => {}
        }
    }
}

/// A whole capture: the frames, in tick order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trace {
    /// One per tick, in the order they were recorded.
    pub frames: Vec<Frame>,
}

/// Why a CSV could not be read as a trace.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The file held no header row.
    #[error("empty trace: no header row")]
    Empty,
    /// The header is missing a column the comparison needs.
    #[error("trace header is missing the column {0:?}")]
    MissingColumn(String),
    /// A row has a different number of fields than the header.
    #[error("line {line}: expected {expected} field(s), found {found}")]
    Width {
        /// One-based line number in the file.
        line: usize,
        /// How many the header declared.
        expected: usize,
        /// How many the row held.
        found: usize,
    },
    /// A field would not parse as a number.
    #[error("line {line}, column {column:?}: {value:?} is not a number")]
    NotANumber {
        /// One-based line number in the file.
        line: usize,
        /// Which column it was in.
        column: String,
        /// What was there instead.
        value: String,
    },
}

impl Trace {
    /// Reads a trace out of CSV text.
    ///
    /// Blank lines and lines starting with `#` are skipped, which is for
    /// hand-annotated fixtures. It does **not** rescue a capture run with
    /// `2>&1`: `scripts/psp-trace.py` writes its `craft at 0x...` line to stderr
    /// unprefixed, so a merged file fails on the header rather than skipping it.
    /// Capture with `--out`, or keep the streams apart.
    ///
    /// Columns are located by header name, and every column in
    /// [`REQUIRED_COLUMNS`] must be present: silently defaulting a missing one
    /// would produce a comparison that passes because it never looked. The
    /// [`OPTIONAL_COLUMNS`] are read when the header has them and left absent
    /// when it does not, which is what keeps a capture taken before they existed
    /// readable - see [`REQUIRED_COLUMNS`] for why that matters.
    ///
    /// # Errors
    ///
    /// [`Error::Empty`] for a file with no header, [`Error::MissingColumn`] for a
    /// header that does not carry every column of [`REQUIRED_COLUMNS`] - or that
    /// carries part of an all-or-nothing group ([`ANGULAR_COLUMNS`],
    /// [`ANGULAR_RATE_COLUMNS`], [`CAMERA_COLUMNS`], [`FLARE_COLUMNS`]) without
    /// the rest - and
    /// [`Error::Width`] or [`Error::NotANumber`] for a malformed row.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.trim()))
            .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'));

        let (_, header) = lines.next().ok_or(Error::Empty)?;
        let header: Vec<&str> = header.split(',').map(str::trim).collect();
        for wanted in REQUIRED_COLUMNS {
            if !header.contains(&wanted) {
                return Err(Error::MissingColumn(wanted.to_owned()));
            }
        }
        // A vector arrives whole or not at all. A header with `avel_x` and no
        // `avel_y` is a truncated or edited file rather than an older capture,
        // and reading it as a rotation about one axis would be an invention.
        // The camera pose is one twelve-column group for the same reason: a
        // basis without its eye, or rows without a third, is not a pose. The
        // flare's eight are one group because they are one `--flare` read of one
        // object: a file with `boost_timer` and no `plume_timer` was edited.
        for group in [
            &ANGULAR_COLUMNS[..],
            &ANGULAR_RATE_COLUMNS[..],
            &CAMERA_COLUMNS[..],
            &FLARE_COLUMNS[..],
        ] {
            if group.iter().any(|c| header.contains(c)) {
                for wanted in group {
                    if !header.contains(wanted) {
                        return Err(Error::MissingColumn((*wanted).to_owned()));
                    }
                }
            }
        }

        let mut frames = Vec::new();
        for (line, row) in lines {
            let fields: Vec<&str> = row.split(',').map(str::trim).collect();
            if fields.len() != header.len() {
                return Err(Error::Width {
                    line,
                    expected: header.len(),
                    found: fields.len(),
                });
            }
            let mut frame = Frame::default();
            for (column, field) in header.iter().zip(&fields) {
                let value: f32 = field.parse().map_err(|_| Error::NotANumber {
                    line,
                    column: (*column).to_owned(),
                    value: (*field).to_owned(),
                })?;
                frame.set(column, value);
            }
            frames.push(frame);
        }
        Ok(Self { frames })
    }

    /// How many ticks the trace holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether the trace holds no ticks at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The columns this trace carries, in the capture's own order.
    ///
    /// [`REQUIRED_COLUMNS`] always, plus each of [`OPTIONAL_COLUMNS`] that
    /// *every* frame holds. All-or-nothing because a CSV column is: a file cannot
    /// have `stun_timer` on some rows and not others, so a trace where only some
    /// frames carry one was assembled in memory rather than read, and writing it
    /// out would have to invent a value for the rest.
    #[must_use]
    pub fn columns(&self) -> Vec<&'static str> {
        COLUMNS
            .into_iter()
            .filter(|column| {
                REQUIRED_COLUMNS.contains(column)
                    || (!self.frames.is_empty() && self.frames.iter().all(|f| f.has(column)))
            })
            .collect()
    }

    /// Writes the trace back out in the capture's own column order.
    ///
    /// Floats are written with Rust's shortest round-tripping form rather than
    /// the capture's `%.7g`, so a simulated trace written here and read back is
    /// the same trace. A *recorded* trace round-tripped through this will
    /// therefore not be byte-identical to the file it came from, only
    /// numerically identical to what that file said.
    ///
    /// Only the columns [`Self::columns`] reports are written, so a trace read
    /// from a capture that predates the angular-velocity column writes it back
    /// without one rather than filling the gap with a zero that would read as a
    /// measurement.
    #[must_use]
    pub fn to_csv(&self) -> String {
        let columns = self.columns();
        let mut out = String::new();
        out.push_str(&columns.join(","));
        out.push('\n');
        for frame in &self.frames {
            for (index, column) in columns.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if *column == "tick" {
                    out.push_str(&frame.tick.to_string());
                } else {
                    let value = frame.value(column).unwrap_or(f64::NAN) as f32;
                    out.push_str(&value.to_string());
                }
            }
            out.push('\n');
        }
        out
    }

    /// A one-paragraph description of what is in the trace.
    ///
    /// Cheap orientation before a comparison: how long it is, whether the deltas
    /// look like the vblank-locked frame time the captures show, how far the ship
    /// actually went - a stalled ship is the false-start trap, not a bug - and
    /// whether the recorded basis is a positively oriented set on every tick.
    #[must_use]
    pub fn summary(&self) -> Summary {
        let mut summary = Summary {
            ticks: self.len(),
            ..Summary::default()
        };
        let Some(first) = self.frames.first() else {
            return summary;
        };
        let last = self.frames.last().copied().unwrap_or(*first);

        summary.dt_min = f32::INFINITY;
        summary.speed_min = f32::INFINITY;
        let mut dt_total = 0.0f64;
        let mut distance = 0.0f64;
        let mut previous = first.position;
        for frame in &self.frames {
            summary.dt_min = summary.dt_min.min(frame.dt);
            summary.dt_max = summary.dt_max.max(frame.dt);
            dt_total += f64::from(frame.dt);
            summary.speed_min = summary.speed_min.min(frame.speed);
            summary.speed_max = summary.speed_max.max(frame.speed);
            distance += f64::from((frame.position - previous).length());
            previous = frame.position;
            if frame.basis_is_positively_oriented(BASIS_TOLERANCE) {
                summary.positively_oriented += 1;
            }
        }
        summary.dt_mean = (dt_total / self.len() as f64) as f32;
        summary.path_length = distance as f32;
        summary.displacement = (last.position - first.position).length();

        summary.angular_ticks = self
            .frames
            .iter()
            .filter(|f| f.angular_velocity.is_some())
            .count();
        if self.frames.iter().all(|f| f.stun_timer.is_some()) {
            summary.stunned_ticks = Some(
                self.frames
                    .iter()
                    .filter(|f| f.stun_timer > Some(0.0))
                    .count(),
            );
        }
        if self.frames.iter().all(|f| f.timer_2e0.is_some()) {
            summary.timer_2e0_ticks = Some(
                self.frames
                    .iter()
                    .filter(|f| f.timer_2e0 > Some(0.0))
                    .count(),
            );
        }
        summary.angular_readings = self.angular_readings(&mut summary.angular_rms);

        for frame in &self.frames {
            let magnitude = frame.velocity.length();
            let unmeasurable = magnitude < UNMEASURABLE_SPEED;
            if unmeasurable {
                summary.unmeasurable_ticks += 1;
            }
            if (frame.speed / magnitude - 1.0).abs() < CLEAN_TOLERANCE {
                summary.clean_ticks += 1;
            } else if summary.first_contact.is_none() && !unmeasurable {
                // A tick below the floor cannot *date* a contact even though it
                // still counts toward `clean_ticks` above: near zero speed the
                // ratio is undefined at exactly zero velocity (NaN, which never
                // compares clean) and unreliable just above it, so an early
                // near-rest tick must not poison the first-contact date the way
                // it is allowed to nudge the fraction. See `UNMEASURABLE_SPEED`.
                summary.first_contact = Some(frame.tick as usize);
            }
        }

        summary
    }

    /// Scores every [`AngularReading`] of the recorded column against the
    /// rotation the recorded basis itself performs, best first.
    ///
    /// The error is measured in the *recorded* column's own space - the basis
    /// derivative is mapped forward through each reading rather than the column
    /// being mapped back - so all four numbers are in rad/s and comparable to
    /// each other and to `signal_rms`, which is set to the recorded column's own
    /// root-mean-square magnitude.
    fn angular_readings(&self, signal_rms: &mut f32) -> Option<[AngularFit; 4]> {
        let mut squared = [0.0f64; 4];
        let mut signal = 0.0f64;
        let mut count = 0usize;
        for pair in self.frames.windows(2) {
            let (frame, next) = (&pair[0], &pair[1]);
            let (Some(recorded), Some(measured)) =
                (frame.angular_velocity, frame.basis_rotation_rate(next))
            else {
                continue;
            };
            let rows = frame.rows();
            for (slot, reading) in AngularReading::ALL.into_iter().enumerate() {
                let predicted = reading.to_recorded(measured, rows);
                squared[slot] += f64::from((predicted - recorded).length_squared());
            }
            signal += f64::from(recorded.length_squared());
            count += 1;
        }
        if count == 0 {
            return None;
        }
        let root = |total: f64| (total / count as f64).sqrt() as f32;
        *signal_rms = root(signal);
        let mut fits = std::array::from_fn(|slot| AngularFit {
            reading: AngularReading::ALL[slot],
            rms_error: root(squared[slot]),
        });
        // Best first, and a total order so a report is deterministic: `f32` has
        // no `Ord`, and a NaN error - which a poisoned capture would produce -
        // must sort last rather than wherever a partial comparison leaves it.
        fits.sort_by(|a: &AngularFit, b: &AngularFit| {
            a.rms_error
                .partial_cmp(&b.rms_error)
                .unwrap_or(std::cmp::Ordering::Greater)
        });
        Some(fits)
    }
}

mod summary;

pub use summary::{
    AngularFit, AngularReading, BASIS_TOLERANCE, CLEAN_TOLERANCE, Summary, UNMEASURABLE_SPEED,
};

#[cfg(test)]
mod tests;
