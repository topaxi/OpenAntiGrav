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

use std::fmt;

use oag_core::math::Vec3;

/// Every column a current capture writes, in the order it writes them.
///
/// Parsing is by name rather than by position - the header is read and each
/// column located in it - so a capture that grows a column stays readable.
/// [`Trace::columns`] is what [`Trace::to_csv`] writes, which is this list
/// filtered down to the columns the trace in hand actually carries.
pub const COLUMNS: [&str; 53] = [
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
pub const OPTIONAL_COLUMNS: [&str; 28] = [
    "stun_timer",
    "timer_2e0",
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

/// The eight exhaust-flare columns, written by `scripts/psp-trace.py --flare`:
/// all present or all absent, for the same reason as [`CAMERA_COLUMNS`].
///
/// Each is one `f32` read straight out of the flare object the capture walks to
/// as `craft+0x1c4` -> `+0x78`, at the offsets `scripts/psp_trace_fields.py`'s
/// `FLARE_FIELDS` lists. Recording the offset each column came from matters more
/// than usual here, because four of the eight have names that could plausibly
/// belong to a different field:
///
/// | Column | Flare offset | What it is |
/// | --- | --- | --- |
/// | `boost_timer` | `+0xb8` | seconds left on the pad boost; `ExhaustFlare_OnSpeedupPad` stores `0.8` here |
/// | `plume_timer` | `+0x88` | seconds since the `<Team>boost.vex` plume was **revealed** - not the boost timer |
/// | `intensity` | `+0xbc` | the `0..=1` engine ramp the size and layer alphas read |
/// | `half_size` | `+0xc4` | flare quad half-extent, world units, already flickered |
/// | `engine_on` | `+0x94` | see below - an integer field, not a float |
/// | `flare_speed_kmh` | `+0x8c` | craft speed in km/h |
/// | `speed_ramp` | `+0x90` | `((flare_speed_kmh - 100) / 500)` clamped to `0..=1` |
/// | `boost_accum` | `+0x60` | the throttle-charge accumulator |
///
/// **`engine_on` is an integer read through a float lens, and is compared as a
/// predicate rather than by value.** The capture `struct.unpack("<f", ..)`s all
/// eight, but `+0x94` holds a small integer, so the column comes out as a
/// denormal: every non-zero row of `data/traces/pad0-boost.csv` reads
/// `3.601337e-43`, which is the `f32` whose bit pattern is `257`. The two
/// distinct values across that whole capture are exactly `0` and that denormal.
/// Stored here as written rather than normalised - this crate converts nothing
/// on the way in - so anything reading it must ask whether it is zero, never
/// what it equals. See [`Flare::engine_on_is_set`].
pub const FLARE_COLUMNS: [&str; 8] = [
    "boost_timer",
    "plume_timer",
    "intensity",
    "half_size",
    "engine_on",
    "flare_speed_kmh",
    "speed_ramp",
    "boost_accum",
];

/// The exhaust flare's eight fields for one tick.
///
/// One struct rather than eight `Option<f32>` on [`Frame`], because the group is
/// all-or-nothing: a capture either ran `--flare` or it did not, and eight
/// independently-optional fields would make "half a flare" representable when it
/// is not a state any file can be in. The angular and camera groups get the same
/// guarantee from [`Vec3`] being indivisible; this one has to spell it out.
///
/// Field order matches [`FLARE_COLUMNS`], which matches the capture's own header
/// order, so a row written from this reads back into the same struct.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Flare {
    /// Seconds left on the pad boost, `flare+0xb8`.
    pub boost_timer: f32,
    /// Seconds since the plume was revealed, `flare+0x88`. **Not** the boost
    /// timer: the reveal is edge-triggered, so this restarts at a reveal and
    /// then runs its own span regardless of how long the boost lasts.
    pub plume_timer: f32,
    /// The `0..=1` engine intensity ramp, `flare+0xbc`.
    pub intensity: f32,
    /// Flare quad half-extent in world units, `flare+0xc4`, flicker included.
    pub half_size: f32,
    /// Whether the engine counts as lit, `flare+0x94`.
    ///
    /// An integer field the capture reads as a float - see [`FLARE_COLUMNS`].
    /// Read it through [`Self::engine_on_is_set`] rather than comparing it.
    pub engine_on: f32,
    /// Craft speed in km/h, `flare+0x8c`.
    pub speed_kmh: f32,
    /// The clamped speed ramp, `flare+0x90`.
    pub speed_ramp: f32,
    /// The throttle-charge accumulator, `flare+0x60`.
    pub boost_accumulator: f32,
}

impl Flare {
    /// Whether the engine-on field is set, which is the only question its raw
    /// value can answer - see [`FLARE_COLUMNS`] for why it is not a `bool` here
    /// and not comparable as a number.
    #[must_use]
    pub fn engine_on_is_set(&self) -> bool {
        self.engine_on != 0.0
    }
}

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

/// How far `speed / |velocity|` may sit from `1.0` and still count as clean
/// (free) flight.
///
/// `speed` and `|velocity|` agree to `1e-6` in free flight - the captures write
/// seven significant digits - so this is a wide margin rather than a tight fit,
/// matching the `1e-4` already used ad hoc in
/// `crates/trace/tests/wall_contact_ground_truth.rs` before this existed.
pub const CLEAN_TOLERANCE: f32 = 1e-4;

/// Below this speed, `speed / |velocity|` stops measuring contact at all and
/// starts measuring rounding.
///
/// The normal impulse dominates at low speed and restitution can push
/// `|velocity|` above `speed` outright, which is what produced a `-380 %`
/// "friction" reading on a craft wedged at 0.15 units/s on the lap capture -
/// see `HANDOVER.md`. Split two ways in [`Summary`], deliberately: it does
/// **not** change [`Summary::clean_ticks`], which counts every tick with no
/// floor to match the fraction already on record, but it **does** stop a tick
/// below it from being named by [`Summary::first_contact`] - at exactly zero
/// velocity the ratio is `0/0` (`NaN`), and a fresh capture starting from a
/// dead stop would otherwise report tick 0 as the first contact on every run.
pub const UNMEASURABLE_SPEED: f32 = 1.0;

/// How far `cross(row0, up)` may sit from `forward` and still count as a
/// positively oriented basis.
///
/// The capture writes seven significant digits, so an exactly orthonormal basis
/// arrives with about `1e-6` of rounding in it.
pub const BASIS_TOLERANCE: f32 = 1e-4;

/// What the recorded angular-velocity column means.
///
/// Two independent binary questions, so four readings, and **neither is settled
/// by static reading**:
///
/// - **Sign.** `docs/ghidra/functions/ps2-pulse-eu/craft-update.md` derives
///   `w_game = -w_physics` from the PS2 integrator on three independent legs, so
///   the engine's angular velocity is the negative of the physical one. That is a
///   result about the *accumulators*; whether the stored velocity carries the
///   same convention is the obvious reading and not a measured one.
/// - **Frame.** The same page names `body+0x160` `angularVelocityLocal` and then
///   lists "which frame `body+0x150` and `body+0x160` are each expressed in" as
///   unresolved, and `docs/ghidra/functions/psp-pulse-usa/engine.md` caps the
///   local/world split of the angular accumulators at confidence 74.
///
/// So this is a switch, for the same reason [`crate::replay::Basis`] is: it is
/// exactly the kind of finding a capture exists to settle rather than a constant
/// to be chosen. Unlike the basis question it needs **no simulation at all** -
/// the recorded rows differentiate into an angular velocity of their own, and
/// [`Summary::angular_readings`] scores all four against it.
///
/// The local reading takes the recorded rows to be the body's axes in world
/// coordinates, which is what the rest of this crate already reads them as; it is
/// independent of [`crate::replay::Basis`], which is about mapping those rows
/// onto [`oag_physics::Body`]'s own axes rather than about the game's frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AngularReading {
    /// The column is the physical angular velocity, in world space.
    World,
    /// Body-local components, negated: `w_game = -w_physics` applied to the
    /// column the PS2 page calls `angularVelocityLocal`. The reading the sign
    /// result and the name point at together, and so the default.
    #[default]
    NegatedLocal,
    /// Body-local components, in the physical sign.
    Local,
    /// World space, negated.
    NegatedWorld,
}

impl AngularReading {
    /// Every reading, in report order.
    pub const ALL: [Self; 4] = [
        Self::NegatedLocal,
        Self::Local,
        Self::NegatedWorld,
        Self::World,
    ];

    /// The reading's name in a report.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::NegatedWorld => "negated-world",
            Self::Local => "local",
            Self::NegatedLocal => "negated-local",
        }
    }

    /// The physical, world-space angular velocity a recorded column describes,
    /// given the recorded basis rows of the same tick.
    #[must_use]
    pub fn to_world(self, recorded: Vec3, rows: (Vec3, Vec3, Vec3)) -> Vec3 {
        let (row0, row1, row2) = rows;
        match self {
            Self::World => recorded,
            Self::NegatedWorld => -recorded,
            Self::Local => row0 * recorded.x + row1 * recorded.y + row2 * recorded.z,
            Self::NegatedLocal => -(row0 * recorded.x + row1 * recorded.y + row2 * recorded.z),
        }
    }

    /// The column a physical, world-space angular velocity would be recorded as.
    /// The inverse of [`Self::to_world`] for an orthonormal basis.
    #[must_use]
    pub fn to_recorded(self, world: Vec3, rows: (Vec3, Vec3, Vec3)) -> Vec3 {
        let (row0, row1, row2) = rows;
        let local = || Vec3::new(world.dot(row0), world.dot(row1), world.dot(row2));
        match self {
            Self::World => world,
            Self::NegatedWorld => -world,
            Self::Local => local(),
            Self::NegatedLocal => -local(),
        }
    }
}

/// How well one reading of the angular-velocity column matches the recorded
/// basis's own rotation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AngularFit {
    /// The reading scored.
    pub reading: AngularReading,
    /// Root-mean-square difference between the recorded column and what the
    /// basis derivative says it should hold, in rad/s.
    pub rms_error: f32,
}

/// What [`Trace::summary`] found.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Summary {
    /// Ticks in the trace.
    pub ticks: usize,
    /// Smallest frame delta.
    pub dt_min: f32,
    /// Largest frame delta.
    pub dt_max: f32,
    /// Mean frame delta.
    pub dt_mean: f32,
    /// Slowest recorded speed.
    pub speed_min: f32,
    /// Fastest recorded speed.
    pub speed_max: f32,
    /// Distance travelled along the path, summed tick to tick.
    pub path_length: f32,
    /// Straight-line distance from the first position to the last.
    pub displacement: f32,
    /// How many ticks satisfy `cross(row0, up) = forward`.
    pub positively_oriented: usize,
    /// How many ticks carry an angular velocity. Zero for a capture taken before
    /// the column existed, which is not an error - see [`REQUIRED_COLUMNS`].
    pub angular_ticks: usize,
    /// Root-mean-square magnitude of the recorded angular velocity, in rad/s.
    /// The scale the fits below are to be read against: an `rms_error` of 0.01
    /// means nothing without knowing whether the signal is 1.5 or 0.01.
    pub angular_rms: f32,
    /// Every reading of the column scored against the recorded basis's own
    /// rotation, best first, or `None` when the trace has no angular velocity or
    /// is too short to differentiate.
    ///
    /// **This is the measurement that settles [`AngularReading`]**, and it needs
    /// no simulation: an orthonormal basis recorded on two consecutive ticks
    /// determines the rotation between them, so the column can be checked against
    /// the rows that sit beside it in the same file. A capture where one reading
    /// scores far below the other three has answered both open questions at once.
    pub angular_readings: Option<[AngularFit; 4]>,
    /// How many ticks were inside the collision-stun window, where the original
    /// produces no thrust and no lateral grip. `None` when the capture has no
    /// `stun_timer` column.
    ///
    /// The headline number for the force-balance question: a straight-line
    /// capture with a nonzero count here is a *stunned* ship coasting, and its
    /// speed is not an equilibrium of the engine force law at all.
    pub stunned_ticks: Option<usize>,
    /// How many ticks had the second engine gate at `craft+0x2e0` above zero.
    pub timer_2e0_ticks: Option<usize>,
    /// How many ticks pass the free-flight contact detector:
    /// `speed / |velocity|` within [`CLEAN_TOLERANCE`] of `1.0`.
    ///
    /// Counted over **every** tick with no speed floor, matching how the
    /// **32.7 %**/**171** figures in `HANDOVER.md` for the reference lap
    /// capture were first read by hand - reproduced exactly
    /// (`1029/3146`, first contact `171`) once this existed, which is what
    /// pins the convention here rather than the floor-excluding one
    /// [`Self::unmeasurable_ticks`] might suggest. The working rule this
    /// exists to make mechanical: *"Check a capture is clean before fitting
    /// anything to it"* - `speed` and `|velocity|` agree to `1e-6` in free
    /// flight, so a mismatch is a free contact detector. Two reference
    /// captures once stood as the M4 blocker for a session and a half because
    /// nobody ran this check.
    pub clean_ticks: usize,
    /// The first tick that fails the clean test at or above
    /// [`UNMEASURABLE_SPEED`], or `None` if every such tick passes.
    ///
    /// A tick below the floor is skipped when *searching* for this - it still
    /// contributes to [`Self::clean_ticks`] above, which counts every tick
    /// with no floor, but it cannot be the tick this field names. Two reasons:
    /// at exactly zero velocity the ratio is `0/0` (`NaN`, which never compares
    /// clean and would otherwise poison this field with tick 0 on any capture
    /// that starts from a dead stop), and just above zero it is real but
    /// unreliable in the same way `unmeasurable_ticks` describes. Verified
    /// against both anchors this module reproduces: neither the lap capture's
    /// **171** nor the standing start's **186** moves, because in both files
    /// nothing below the floor precedes the real first contact.
    pub first_contact: Option<usize>,
    /// How many ticks sit below [`UNMEASURABLE_SPEED`].
    ///
    /// Excluded from being **named** by [`Self::first_contact`] (see its docs)
    /// but **not excluded** from [`Self::clean_ticks`]'s count, which counts
    /// every tick with no floor - folding the floor into that fraction would
    /// produce a different number from the one already on record.
    ///
    /// Below a few units per second the ratio stops measuring contact at all:
    /// the normal impulse dominates and restitution can push `|velocity|`
    /// above `speed` outright, which is what produced a `-380 %` "friction"
    /// reading on a craft wedged at 0.15 units/s on the lap capture. A high
    /// count here alongside a low `clean_ticks` fraction is a hint to look at
    /// `speed_min` before trusting the headline number, not a reason to
    /// recompute it with a floor.
    pub unmeasurable_ticks: usize,
}

impl Summary {
    /// The reading of the angular-velocity column that best matches the recorded
    /// basis's own rotation.
    #[must_use]
    pub fn best_angular_reading(&self) -> Option<AngularFit> {
        self.angular_readings.map(|fits| fits[0])
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} tick(s)", self.ticks)?;
        writeln!(
            f,
            "dt: min {:.6}, mean {:.6} ({:.2} Hz), max {:.6}",
            self.dt_min,
            self.dt_mean,
            if self.dt_mean > 0.0 {
                1.0 / self.dt_mean
            } else {
                0.0
            },
            self.dt_max
        )?;
        writeln!(f, "speed: {:.3} to {:.3}", self.speed_min, self.speed_max)?;
        writeln!(
            f,
            "travelled {:.3} unit(s) along the path, {:.3} from start to finish",
            self.path_length, self.displacement
        )?;
        writeln!(
            f,
            "cross(row0, up) = forward on {}/{} tick(s)",
            self.positively_oriented, self.ticks
        )?;

        if self.angular_ticks == 0 {
            writeln!(
                f,
                "angular velocity: not in this capture (recorded before the column existed)"
            )?;
        } else {
            writeln!(
                f,
                "angular velocity: {}/{} tick(s), rms {:.4} rad/s",
                self.angular_ticks, self.ticks, self.angular_rms
            )?;
            if let Some(fits) = self.angular_readings {
                writeln!(
                    f,
                    "  against the basis derivative, best first: {}",
                    fits.iter()
                        .map(|fit| format!("{} {:.4}", fit.reading.as_str(), fit.rms_error))
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
            }
        }

        writeln!(
            f,
            "clean (speed / |velocity| ~= 1.0): {}/{} tick(s), first contact {} \
             ({} tick(s) below {UNMEASURABLE_SPEED} unit/s: counted in the \
             fraction, skipped when dating first contact - see the field docs)",
            self.clean_ticks,
            self.ticks,
            self.first_contact
                .map_or_else(|| "none".to_string(), |tick| tick.to_string()),
            self.unmeasurable_ticks
        )?;

        match self.stunned_ticks {
            None => write!(f, "stun timer: not in this capture"),
            Some(0) => write!(f, "stun timer: never armed"),
            Some(ticks) => write!(
                f,
                // "the run" rather than "the original": a `Summary` is printed
                // for our own traces too - `oag-trace drive` prints one - and a
                // report that calls our output the original's is a lie in the
                // one place a reader is least likely to check.
                "stun timer: armed on {}/{} tick(s) - no thrust and no lateral \
                 grip on those",
                ticks, self.ticks
            ),
        }?;
        match self.timer_2e0_ticks {
            None => Ok(()),
            Some(0) => write!(f, "; craft+0x2e0 never above zero"),
            Some(ticks) => write!(
                f,
                "; craft+0x2e0 above zero on {}/{} tick(s)",
                ticks, self.ticks
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two ticks in exactly the shape `scripts/psp-trace.py` writes: the same
    /// header, in the same order, with `%.7g`-shaped values. Hand-authored, not
    /// captured - a real trace is derived game data and cannot be committed.
    pub(crate) const FIXTURE: &str = "\
tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
stun_timer,timer_2e0,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,avel_x,avel_y,avel_z,omega_x,omega_y,omega_z,\
cam_right_x,cam_right_y,cam_right_z,cam_up_x,cam_up_y,cam_up_z,\
cam_fwd_x,cam_fwd_y,cam_fwd_z,cam_pos_x,cam_pos_y,cam_pos_z,\
boost_timer,plume_timer,intensity,half_size,engine_on,flare_speed_kmh,speed_ramp,boost_accum
0,0.016683,1,100,0,0,0,0,21.98,0,0,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,10,8,-45,0.8,0.25,0.5,3.5,1,79.2,0.3125,0.1875
1,0.016683,1,100,0,0,0,0,22,0,0,1,0,0,0,1,0,0,0,1,10,2.5,-29.63301,0,0,22.1,22.1,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,10,8,-44.6,0.78,0.2666,0.52,3.55,1,79.56,0.3187,0.19
";

    /// The same two ticks as a capture taken **before** the angular-velocity and
    /// timer columns existed - which is every trace under `data/traces/` today.
    ///
    /// This fixture is the backwards-compatibility contract in one constant. It
    /// is not a stale copy of [`FIXTURE`] to be updated alongside it: it is a
    /// frozen record of a file format that real, unreproducible captures are
    /// already written in, and it must keep parsing unchanged forever.
    pub(crate) const LEGACY_FIXTURE: &str = "\
tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed
0,0.016683,1,100,0,0,0,0,21.98,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22
1,0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,10,2.5,-29.63301,0,0,22.1,22.1
";

    #[test]
    fn the_fixture_header_is_the_capture_script_s_header() {
        // If `scripts/psp_trace_fields.py` grows a column, this is what fails first.
        let header = FIXTURE.lines().next().unwrap();
        assert_eq!(header, COLUMNS.join(","));
    }

    /// The header this crate expects, read out of the capture script itself.
    ///
    /// The doc comment on [`COLUMNS`] says the list is a transcription of
    /// `scripts/psp_trace_fields.py`'s `CRAFT_FIELDS` and `BODY_FIELDS`. A
    /// transcription that nothing checks is a copy waiting to drift, and the
    /// failure mode is quiet: a column added on one side only makes every trace
    /// unreadable, or worse, shifts what a name means. So the script is the
    /// source of truth and this reads it.
    ///
    /// The tables moved out of `psp-trace.py` itself when `psp-autopilot.py`
    /// started writing the same columns; this test is what noticed.
    fn header_the_capture_script_writes() -> Vec<String> {
        const SCRIPT: &str = include_str!("../../../scripts/psp_trace_fields.py");
        let mut columns = vec!["tick".to_owned()];
        for list in [
            "CRAFT_FIELDS = [",
            "BODY_FIELDS = [",
            "CAMERA_FIELDS = [",
            "FLARE_FIELDS = [",
        ] {
            let start = SCRIPT
                .find(list)
                .unwrap_or_else(|| panic!("{list} is not in scripts/psp_trace_fields.py"))
                + list.len();
            let body = &SCRIPT[start..];
            let end = body.find("\n]").expect("an unterminated field list");
            // Every entry is `("name", 0x...)`, so the names are exactly the
            // quoted strings inside the list.
            columns.extend(body[..end].split('"').skip(1).step_by(2).map(str::to_owned));
        }
        columns
    }

    /// The one test that can catch the capture script and this crate drifting
    /// apart, which no fixture can: a fixture is written by hand to match
    /// whatever this file already says.
    #[test]
    fn the_column_list_is_the_capture_script_s_own() {
        assert_eq!(header_the_capture_script_writes(), COLUMNS.to_vec());
    }

    #[test]
    fn the_legacy_fixture_is_exactly_the_required_columns() {
        let header = LEGACY_FIXTURE.lines().next().unwrap();
        assert_eq!(header, REQUIRED_COLUMNS.join(","));
    }

    /// The optional columns are the difference between the two, and every one of
    /// them is in the full header. A column added to `COLUMNS` and forgotten in
    /// `OPTIONAL_COLUMNS` would be silently unwritable by [`Trace::to_csv`].
    #[test]
    fn every_column_is_either_required_or_optional() {
        for column in COLUMNS {
            assert!(
                REQUIRED_COLUMNS.contains(&column) ^ OPTIONAL_COLUMNS.contains(&column),
                "{column} is in neither list, or in both"
            );
        }
        assert_eq!(
            COLUMNS.len(),
            REQUIRED_COLUMNS.len() + OPTIONAL_COLUMNS.len()
        );
        for column in ANGULAR_COLUMNS {
            assert!(OPTIONAL_COLUMNS.contains(&column));
        }
        for column in CAMERA_COLUMNS {
            assert!(OPTIONAL_COLUMNS.contains(&column));
        }
    }

    #[test]
    fn a_capture_shaped_csv_parses() {
        let trace = Trace::parse(FIXTURE).unwrap();
        assert_eq!(trace.len(), 2);
        let first = trace.frames[0];
        assert_eq!(first.tick, 0);
        assert_eq!(first.dt, 0.016_683);
        assert_eq!(first.grounded, 1.0);
        assert_eq!(first.throttle, 100.0);
        assert_eq!(first.speed_cached, 21.98);
        assert_eq!(first.row0, Vec3::X);
        assert_eq!(first.up, Vec3::Y);
        assert_eq!(first.forward, Vec3::Z);
        assert_eq!(first.position, Vec3::new(10.0, 2.5, -30.0));
        assert_eq!(first.velocity, Vec3::new(0.0, 0.0, 22.0));
        assert_eq!(trace.frames[1].tick, 1);
    }

    #[test]
    fn columns_are_located_by_name_not_by_position() {
        let reordered = "\
speed,vel_z,vel_y,vel_x,pos_z,pos_y,pos_x,fwd_z,fwd_y,fwd_x,up_z,up_y,up_x,\
right_z,right_y,right_x,speed_cached,airbrake_r,airbrake_l,steer,brake,throttle,\
grounded,dt,tick
22,22,0,0,-30,2.5,10,1,0,0,0,1,0,0,0,1,21.98,0,0,0,0,100,1,0.016683,0
";
        let trace = Trace::parse(reordered).unwrap();
        assert_eq!(
            trace.frames[0],
            Trace::parse(LEGACY_FIXTURE).unwrap().frames[0]
        );
    }

    /// The whole backwards-compatibility contract: every capture under
    /// `data/traces/` predates these columns, none of them can be re-taken
    /// without a hand-driven PPSSPP session, and all of them must keep working.
    #[test]
    fn a_capture_without_the_new_columns_still_parses() {
        let trace = Trace::parse(LEGACY_FIXTURE).unwrap();
        assert_eq!(trace.len(), 2);
        assert_eq!(trace.frames[0].speed_cached, 21.98);
        assert_eq!(trace.frames[0].position, Vec3::new(10.0, 2.5, -30.0));
    }

    /// Absent, not zero. A zero angular velocity is a claim that the ship was not
    /// rotating; a capture that never measured it must not be able to make that
    /// claim, because that is exactly how a comparison passes without looking.
    #[test]
    fn an_absent_column_is_absent_rather_than_zero() {
        let frame = Trace::parse(LEGACY_FIXTURE).unwrap().frames[0];
        assert_eq!(frame.angular_velocity, None);
        assert_eq!(frame.stun_timer, None);
        assert_eq!(frame.timer_2e0, None);
        assert_eq!(frame.camera_position, None);
        assert_ne!(frame.angular_velocity, Some(Vec3::ZERO));
    }

    /// Writing a legacy trace back out must not grow it a column, or the absence
    /// would be laundered into a zero by one round trip through this crate.
    #[test]
    fn a_legacy_trace_round_trips_without_growing_a_column() {
        let trace = Trace::parse(LEGACY_FIXTURE).unwrap();
        let csv = trace.to_csv();
        assert_eq!(csv.lines().next().unwrap(), REQUIRED_COLUMNS.join(","));
        assert_eq!(Trace::parse(&csv).unwrap(), trace);
        assert_eq!(trace.columns(), REQUIRED_COLUMNS.to_vec());
    }

    #[test]
    fn a_current_trace_round_trips_with_every_column() {
        let trace = Trace::parse(FIXTURE).unwrap();
        let csv = trace.to_csv();
        assert_eq!(csv.lines().next().unwrap(), COLUMNS.join(","));
        assert_eq!(Trace::parse(&csv).unwrap(), trace);
    }

    #[test]
    fn the_new_columns_are_read_when_they_are_there() {
        let text = FIXTURE
            .replacen(
                ",22,22,0,0,0,0,0,0,",
                ",22,22,0.25,-1.5,0.125,0.0625,-0.5,0.03125,",
                1,
            )
            .replacen("21.98,0,0,", "21.98,0.5,0.75,", 1);
        let frame = Trace::parse(&text).unwrap().frames[0];
        assert_eq!(frame.angular_velocity, Some(Vec3::new(0.25, -1.5, 0.125)));
        assert_eq!(frame.angular_rate, Some(Vec3::new(0.0625, -0.5, 0.031_25)));
        assert_eq!(frame.stun_timer, Some(0.5));
        assert_eq!(frame.timer_2e0, Some(0.75));
    }

    #[test]
    fn the_camera_columns_are_read_when_they_are_there() {
        let frame = Trace::parse(FIXTURE).unwrap().frames[0];
        assert_eq!(frame.camera_row0, Some(Vec3::X));
        assert_eq!(frame.camera_up, Some(Vec3::Y));
        assert_eq!(frame.camera_forward, Some(Vec3::Z));
        assert_eq!(frame.camera_position, Some(Vec3::new(10.0, 8.0, -45.0)));
    }

    #[test]
    fn the_flare_columns_are_read_when_they_are_there() {
        let frame = Trace::parse(FIXTURE).unwrap().frames[0];
        let flare = frame.flare.expect("the fixture carries the flare group");
        assert_eq!(flare.boost_timer, 0.8);
        assert_eq!(flare.plume_timer, 0.25);
        assert_eq!(flare.intensity, 0.5);
        assert_eq!(flare.half_size, 3.5);
        assert_eq!(flare.speed_kmh, 79.2);
        assert_eq!(flare.speed_ramp, 0.3125);
        assert_eq!(flare.boost_accumulator, 0.1875);
        assert!(flare.engine_on_is_set());
    }

    /// The mirror of [`the_legacy_fixture_is_exactly_the_required_columns`]: a
    /// capture taken before `--flare` existed must keep parsing, with the group
    /// **absent** rather than zeroed - a zero `boost_timer` would be a claim
    /// that the ship was not boosting.
    #[test]
    fn a_trace_without_the_flare_columns_has_no_flare_rather_than_zeroes() {
        let frame = Trace::parse(LEGACY_FIXTURE).unwrap().frames[0];
        assert_eq!(frame.flare, None);
        for column in FLARE_COLUMNS {
            assert!(!frame.has(column), "{column} reported present");
        }
        assert!(
            !Trace::parse(LEGACY_FIXTURE)
                .unwrap()
                .columns()
                .iter()
                .any(|c| FLARE_COLUMNS.contains(c))
        );
    }

    /// The mirror of [`a_partial_angular_velocity_is_an_error`]: half a flare is
    /// an edited file, not an older capture.
    #[test]
    fn a_partial_flare_group_is_an_error() {
        let text = FIXTURE.replace(",plume_timer", "");
        assert_eq!(
            Trace::parse(&text),
            Err(Error::MissingColumn("plume_timer".to_owned()))
        );
    }

    /// `engine_on` is an integer field the capture reads as a float, so the
    /// value that actually appears in `data/traces/pad0-boost.csv` is the
    /// denormal whose bit pattern is `257`. It must read as *set*, and it must
    /// survive a round trip - a reader that treated it as noise and zeroed it
    /// would silently flip the only bit the column carries.
    #[test]
    fn the_engine_on_denormal_reads_as_set_and_round_trips() {
        let raw = f32::from_bits(257);
        let text = FIXTURE.replacen(",3.5,1,79.2,", &format!(",3.5,{raw:e},79.2,"), 1);
        let trace = Trace::parse(&text).expect("the edited fixture");
        let flare = trace.frames[0].flare.expect("the flare group");
        assert!(flare.engine_on_is_set(), "{raw:e} must read as engine-on");
        assert_eq!(flare.engine_on.to_bits(), 257);
        assert_eq!(Trace::parse(&trace.to_csv()).unwrap(), trace);
    }

    /// Two thirds of a vector is a truncated or edited file, not an older
    /// capture, and reading it as a rotation about one axis would be an
    /// invention.
    #[test]
    fn a_partial_angular_velocity_is_an_error() {
        // The header check fires before any row is read, so only the header
        // needs editing; the rows' widths never come into it.
        let text = FIXTURE.replace(",avel_z", "");
        assert_eq!(
            Trace::parse(&text),
            Err(Error::MissingColumn("avel_z".to_owned()))
        );
    }

    /// A basis without its eye is not a pose, and reading one as a camera
    /// would silently frame every comparison shot from the wrong place.
    #[test]
    fn a_partial_camera_pose_is_an_error() {
        let text = FIXTURE.replace(",cam_pos_z", "");
        assert_eq!(
            Trace::parse(&text),
            Err(Error::MissingColumn("cam_pos_z".to_owned()))
        );
    }

    #[test]
    fn a_missing_column_is_an_error_not_a_default() {
        let text = FIXTURE.replacen("speed_cached", "not_the_column", 1);
        assert_eq!(
            Trace::parse(&text),
            Err(Error::MissingColumn("speed_cached".to_owned()))
        );
    }

    #[test]
    fn a_short_row_is_an_error() {
        let text = format!("{FIXTURE}2,0.016683\n");
        assert_eq!(
            Trace::parse(&text),
            Err(Error::Width {
                line: 4,
                expected: COLUMNS.len(),
                found: 2
            })
        );
    }

    #[test]
    fn a_non_numeric_field_names_itself() {
        let text = FIXTURE.replacen("21.98", "nan-ish", 1);
        assert_eq!(
            Trace::parse(&text),
            Err(Error::NotANumber {
                line: 2,
                column: "speed_cached".to_owned(),
                value: "nan-ish".to_owned()
            })
        );
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let text = format!("# craft at 0x08c00000\n\n{FIXTURE}");
        assert_eq!(Trace::parse(&text).unwrap().len(), 2);
    }

    #[test]
    fn an_empty_file_is_an_error() {
        assert_eq!(Trace::parse(""), Err(Error::Empty));
    }

    #[test]
    fn a_trace_round_trips_through_csv() {
        let trace = Trace::parse(FIXTURE).unwrap();
        assert_eq!(Trace::parse(&trace.to_csv()).unwrap(), trace);
    }

    #[test]
    fn the_summary_measures_what_the_ship_did() {
        let summary = Trace::parse(FIXTURE).unwrap().summary();
        assert_eq!(summary.ticks, 2);
        assert_eq!(summary.positively_oriented, 2, "cross(x, y) = z");
        assert!((summary.displacement - 0.366_99).abs() < 1e-4);
        assert!((summary.dt_mean - 0.016_683).abs() < 1e-6);
    }

    /// How far the ship is rolled in [`turning`], in radians.
    ///
    /// **Not zero, and not a right angle.** A ship rotating about world `+y` with
    /// its own up axis along `+y` records the same three numbers under the local
    /// and the world reading, so a level fixture can separate the *sign* question
    /// and is blind to the *frame* one. Rolling it puts the rotation axis across
    /// all three body axes and separates all four readings.
    const ROLL: f32 = 0.5;

    /// The recorded yaw rate the captures actually show, in rad/s: holding left
    /// gives `+1.51` about row 1 on 199 of 199 ticks, per
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md`. Used as the fixture's signal
    /// so the numbers in these tests are the size of the real ones.
    const YAW_RATE: f32 = 1.51;

    /// A ship turning steadily about world `+y` at [`YAW_RATE`], rolled by
    /// [`ROLL`], with the angular-velocity column written under `reading`.
    ///
    /// The basis is `Ry(theta) * B` for a fixed rolled triad `B`, so the rotation
    /// carrying one tick onto the next is `Ry(omega * dt)` in world space on every
    /// tick: the world angular velocity is exactly `(0, YAW_RATE, 0)` and the
    /// recorded column is whatever `reading` says that is.
    fn turning(ticks: usize, dt: f32, reading: AngularReading) -> Trace {
        let (roll_sin, roll_cos) = ROLL.sin_cos();
        let base = (
            Vec3::new(roll_cos, roll_sin, 0.0),
            Vec3::new(-roll_sin, roll_cos, 0.0),
            Vec3::Z,
        );
        let world = Vec3::new(0.0, YAW_RATE, 0.0);
        Trace {
            frames: (0..ticks)
                .map(|tick| {
                    let (sin, cos) = (YAW_RATE * tick as f32 * dt).sin_cos();
                    let yaw =
                        |v: Vec3| Vec3::new(v.x * cos + v.z * sin, v.y, -v.x * sin + v.z * cos);
                    let rows = (yaw(base.0), yaw(base.1), yaw(base.2));
                    Frame {
                        tick: tick as u64,
                        dt,
                        row0: rows.0,
                        up: rows.1,
                        forward: rows.2,
                        angular_velocity: Some(reading.to_recorded(world, rows)),
                        ..Frame::default()
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn the_turning_fixture_is_a_positively_oriented_basis_like_a_real_one() {
        for frame in turning(4, 1.0 / 60.0, AngularReading::default()).frames {
            assert!(frame.basis_is_positively_oriented(1e-5));
        }
    }

    /// The recorded rows differentiate into the rotation that carries one onto
    /// the next, which is what makes the new column checkable against the file it
    /// arrived in rather than against a simulation.
    #[test]
    fn the_basis_derivative_recovers_the_rate_the_fixture_turns_at() {
        let trace = turning(4, 1.0 / 60.0, AngularReading::default());
        let measured = trace.frames[0]
            .basis_rotation_rate(&trace.frames[1])
            .expect("a nonzero dt");
        assert!(
            (measured - Vec3::new(0.0, YAW_RATE, 0.0)).length() < 1e-3,
            "{measured}"
        );
    }

    #[test]
    fn a_zero_delta_has_no_rotation_rate_rather_than_an_infinite_one() {
        let frame = Frame::default();
        assert_eq!(frame.basis_rotation_rate(&Frame::default()), None);
    }

    /// [`AngularReading::ALL`] is in report order rather than declaration order,
    /// and [`Trace::angular_readings`] indexes its accumulators against it. A
    /// variant added to the enum and forgotten in `ALL` would simply never be
    /// scored, and the report would look complete while missing a candidate.
    #[test]
    fn every_reading_is_in_the_list_exactly_once() {
        for reading in [
            AngularReading::World,
            AngularReading::NegatedWorld,
            AngularReading::Local,
            AngularReading::NegatedLocal,
        ] {
            assert_eq!(
                AngularReading::ALL
                    .iter()
                    .filter(|r| **r == reading)
                    .count(),
                1,
                "{reading:?}"
            );
        }
        assert_eq!(AngularReading::ALL.len(), 4);
    }

    /// Every reading must be invertible, or seeding a run from a recorded column
    /// and writing our own back out would not be the same transformation twice.
    /// The same property [`crate::replay::Basis`] is pinned on.
    #[test]
    fn every_reading_round_trips() {
        let frame = turning(1, 1.0 / 60.0, AngularReading::default()).frames[0];
        let rows = frame.rows();
        let w = Vec3::new(0.3, -1.51, 0.07);
        for reading in AngularReading::ALL {
            let there_and_back = reading.to_world(reading.to_recorded(w, rows), rows);
            assert!(
                (there_and_back - w).length() < 1e-5,
                "{reading:?}: {there_and_back} is not {w}"
            );
        }
    }

    /// **The measurement the column exists to make.** A capture written under one
    /// reading must name that reading and no other, with the three wrong ones an
    /// order of magnitude worse - and this is decided by the recording alone, so
    /// the first real capture settles both open questions without a run.
    #[test]
    fn a_capture_names_the_reading_it_was_written_under() {
        for reading in AngularReading::ALL {
            let summary = turning(40, 1.0 / 60.0, reading).summary();
            let fits = summary.angular_readings.expect("40 ticks of it");
            assert_eq!(summary.angular_ticks, 40);
            assert_eq!(fits[0].reading, reading, "{reading:?}: {summary}");
            assert!(
                fits[0].rms_error < 1e-2,
                "{reading:?} should fit itself: {}",
                fits[0].rms_error
            );
            assert!(
                fits[1].rms_error > 10.0 * fits[0].rms_error.max(1e-3),
                "{reading:?} is not distinguished from {:?}: {} vs {}",
                fits[1].reading,
                fits[0].rms_error,
                fits[1].rms_error
            );
            assert!((summary.angular_rms - YAW_RATE).abs() < 1e-3);
        }
    }

    #[test]
    fn a_capture_without_the_column_scores_no_readings_and_says_so() {
        let summary = Trace::parse(LEGACY_FIXTURE).unwrap().summary();
        assert_eq!(summary.angular_ticks, 0);
        assert_eq!(summary.angular_readings, None);
        assert_eq!(summary.best_angular_reading(), None);
        assert_eq!(summary.stunned_ticks, None);
        assert!(summary.to_string().contains("not in this capture"));
    }

    /// The stun timer is the capture's wall-contact indicator, and a straight-line
    /// capture that spent ticks inside the window is a *stunned* ship coasting -
    /// not an equilibrium of the engine force law. So the count is in the summary
    /// rather than only in a comparison.
    #[test]
    fn the_summary_counts_the_ticks_the_original_spent_stunned() {
        let mut trace = Trace::parse(FIXTURE).unwrap();
        assert_eq!(trace.summary().stunned_ticks, Some(0));
        trace.frames[1].stun_timer = Some(0.5);
        let summary = trace.summary();
        assert_eq!(summary.stunned_ticks, Some(1));
        assert_eq!(summary.timer_2e0_ticks, Some(0));
        assert!(summary.to_string().contains("armed on 1/2"), "{summary}");
    }

    /// A synthetic contact tick and a synthetic unmeasurably-slow tick, checked
    /// against the mechanical version of the `speed / |velocity|` check that
    /// used to be computed ad hoc - see [`Summary::clean_ticks`], which this
    /// pins by construction rather than by reproducing a real capture (that
    /// reproduction lives in `HANDOVER.md`'s own numbers: 1029/3146 and first
    /// contact 171 on the reference lap capture, 186/300 and first contact 186
    /// on the standing start, both reproduced exactly once this existed).
    #[test]
    fn the_cleanliness_report_dates_the_first_contact_and_tallies_the_unmeasurable() {
        let clean = Frame {
            tick: 0,
            velocity: Vec3::new(0.0, 0.0, 10.0),
            speed: 10.0,
            ..Frame::default()
        };
        let contact = Frame {
            tick: 1,
            velocity: Vec3::new(0.0, 0.0, 10.0),
            speed: 8.0,
            ..Frame::default()
        };
        // Below `UNMEASURABLE_SPEED`, and would also fail the raw ratio test -
        // both counts see it, neither excludes it.
        let unmeasurable = Frame {
            tick: 2,
            velocity: Vec3::new(0.0, 0.0, 0.5),
            speed: 100.0,
            ..Frame::default()
        };
        let trace = Trace {
            frames: vec![clean, contact, unmeasurable],
        };

        let summary = trace.summary();
        assert_eq!(summary.first_contact, Some(1), "{summary:?}");
        assert_eq!(summary.clean_ticks, 1, "{summary:?}");
        assert_eq!(summary.unmeasurable_ticks, 1, "{summary:?}");
        assert!(
            summary.to_string().contains("1/3 tick(s), first contact 1"),
            "{summary}"
        );
    }

    /// A trace with nothing under [`UNMEASURABLE_SPEED`] and nothing that ever
    /// diverges is clean start to finish, with no contact to date.
    #[test]
    fn a_fully_clean_trace_reports_no_first_contact() {
        let frame = Frame {
            velocity: Vec3::new(0.0, 0.0, 10.0),
            speed: 10.0,
            ..Frame::default()
        };
        let trace = Trace {
            frames: vec![frame, frame],
        };

        let summary = trace.summary();
        assert_eq!(summary.first_contact, None, "{summary:?}");
        assert_eq!(summary.clean_ticks, 2, "{summary:?}");
        assert_eq!(summary.unmeasurable_ticks, 0, "{summary:?}");
    }

    /// A dead stop at tick 0 - `velocity == Vec3::ZERO`, so the ratio is `0/0`
    /// (`NaN`, which never compares clean) - must not become `first_contact`.
    /// This is exactly the shape a fresh capture's grid start takes, and is
    /// why `first_contact` skips ticks below `UNMEASURABLE_SPEED` rather than
    /// trusting whatever a near-zero-velocity ratio happens to read.
    #[test]
    fn a_dead_stop_at_the_start_does_not_poison_the_first_contact_date() {
        let rest = Frame {
            tick: 0,
            velocity: Vec3::ZERO,
            speed: 0.0,
            ..Frame::default()
        };
        let clean = Frame {
            tick: 1,
            velocity: Vec3::new(0.0, 0.0, 10.0),
            speed: 10.0,
            ..Frame::default()
        };
        let contact = Frame {
            tick: 2,
            velocity: Vec3::new(0.0, 0.0, 10.0),
            speed: 8.0,
            ..Frame::default()
        };
        let trace = Trace {
            frames: vec![rest, clean, contact],
        };

        let summary = trace.summary();
        assert_eq!(summary.first_contact, Some(2), "{summary:?}");
        assert_eq!(summary.unmeasurable_ticks, 1, "{summary:?}");
    }

    #[test]
    fn a_left_handed_basis_is_reported_as_such() {
        let frame = Frame {
            forward: -Vec3::Z,
            ..Frame::default()
        };
        assert!(!frame.basis_is_positively_oriented(BASIS_TOLERANCE));
    }
}
