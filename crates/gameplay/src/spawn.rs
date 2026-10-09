//! Putting a ship onto a track.
//!
//! The spline carries an orthonormal frame at every control point, so a spawn
//! pose is a conversion rather than a search. Two conventions from
//! `docs/formats/track.md` decide the whole of this module and both are easy to
//! get backwards, so each is a test below rather than only a comment:
//!
//! - **`down` is the surface normal negated.** It points into the track, so the
//!   ship's up axis is `-down`. The `.vex` frame axis was documented as `up` for
//!   a long time and it also points down; see `HANDOVER.md`.
//! - **`sample` does not renormalise the interpolated axes**, matching the
//!   original. A B-spline blend of unit vectors is not a unit vector, so
//!   anything building a rotation from them has to.
//!
//! Two ways in, and they claim different things.
//! [`Pose::from_start_position`] uses the track's own authored `Start Position`
//! node - a recovered value, and the one to prefer. [`Pose::from_sample`] puts a
//! ship on the spline, which is where a track with no such node has to go and
//! which claims only "on the track".
//!
//! What is still **not** here: grid slot *assignment*. Every PSP track ships
//! exactly one `Start Position`, so the other seven slots are laid out by code
//! that has not been read, and how a ship is assigned one remains an open
//! question tracked against M5 in `docs/overview/roadmap.md`. Deriving the rest
//! of a grid from the one authored slot would be a guess dressed as code.

use oag_core::math::{Mat3, Quat, Vec3, quat_from_axis_angle};
use oag_physics::Handling;
use oag_vex::track::{Sample, StartPosition};

use crate::world::Ship;

/// Where a ship sits and which way it faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// World-space position of the centre of mass.
    pub position: Vec3,
    /// Orientation, as a unit quaternion.
    pub orientation: Quat,
}

impl Pose {
    /// A pose from a spline sample, offset across the track and lifted off it.
    ///
    /// `lateral_offset` is measured along the sample's own `lateral` axis, which
    /// is the same axis `racing_line`, `ai_bound_left` and `ai_bound_right` are
    /// expressed in. Using that axis rather than a derived right vector is what
    /// made this correct while which way `lateral` points was still open; it now
    /// points to the driver's **right**, from the `Start Position` frame agreeing
    /// with it on all 40 shipped tracks - see `docs/formats/track.md`. So a
    /// positive `racing_line` is a racing line to the right, and this keeps
    /// using the stored axis anyway rather than restating that as a sign.
    ///
    /// `height` is measured along the ship's up axis from the surface line.
    /// Note that the height a ship actually settles at is emergent from the
    /// hover force law and is not this value; see `docs/physics/README.md`.
    /// This is only where it starts.
    #[must_use]
    pub fn from_sample(sample: &Sample, lateral_offset: f32, height: f32) -> Self {
        let up = -Vec3::from_array(sample.down);
        let forward = Vec3::from_array(sample.tangent);
        let lateral = Vec3::from_array(sample.lateral);
        let position = Vec3::from_array(sample.pos)
            + lateral.normalize_or_zero() * lateral_offset
            + up.normalize_or_zero() * height;

        Self {
            position,
            orientation: orientation_from_axes(forward, up),
        }
    }

    /// A pose at an arbitrary world position, taking its attitude from `sample`.
    ///
    /// **A capture and comparison aid, not a spawn.** Nothing in the original
    /// puts a ship at an arbitrary point; this exists so a frame can be
    /// reproduced from a position read out of a debugger, or so two circuits can
    /// be photographed from the same place. `position` is used exactly as given -
    /// no lift, no probe, no snapping to the surface - because a pose that
    /// silently moved would defeat the point.
    ///
    /// Attitude comes from the spline rather than from world up, so a banked
    /// corner or a loop reads correctly instead of leaving the ship flat inside
    /// the geometry. `yaw`, in radians about the sample's own up axis, turns the
    /// ship away from the sample's tangent; `0.0` faces the way the track runs.
    #[must_use]
    pub fn from_position_on_sample(sample: &Sample, position: Vec3, yaw: f32) -> Self {
        let up = -Vec3::from_array(sample.down);
        let forward = Vec3::from_array(sample.tangent);
        let forward = up
            .try_normalize()
            .map_or(forward, |axis| quat_from_axis_angle(axis, yaw) * forward);

        Self {
            position,
            orientation: orientation_from_axes(forward, up),
        }
    }

    /// A pose from the track's own authored grid slot, lifted off the surface.
    ///
    /// [`oag_vex::track::StartPosition`] is where the exporter put a ship and
    /// which way it faces, after the bind handler's fix-up - a recovered value
    /// rather than an index into a resampled spline, which is what makes this the
    /// one to prefer when a track has such a node.
    ///
    /// **It is one slot, not the grid.** Every PSP track ships exactly one, so
    /// where the other seven sit is still code nobody has read, and this offers
    /// no way to ask for slot `n`. Nor is it necessarily pole: on `16_Track` the
    /// authored slot sits about 138 units behind where a time trial actually
    /// starts. See `docs/formats/track.md`.
    ///
    /// # The slot's own height is not the one to start at, and that is measured
    ///
    /// `ground` is the world `y` of the collision surface under the slot, and
    /// `height` is measured up from *that* - not from the slot. The authored `y`
    /// looks like a ride height and is not one: across the 40 shipped PSP track
    /// files it sits anywhere from **1.03 to 7.36 units** above the surface
    /// beneath it. Starting a ship `height` above the authored value would
    /// therefore put it out of its own probes' reach on some tracks and inside
    /// the floor on others, which is a spawn that works on the track it was tried
    /// on. So the slot supplies where a ship stands and which way it points, and
    /// the height comes from where it comes from everywhere else - see
    /// [`spawn_height`].
    ///
    /// `None` keeps the authored `y`, for a caller with no collision geometry to
    /// ask. That is the honest fallback rather than the right answer.
    ///
    /// Both the lift and `ground` are along world up rather than the slot's own,
    /// because the bind has already forced that axis to `(0, 1, 0)`.
    #[must_use]
    pub fn from_start_position(slot: &StartPosition, ground: Option<f32>, height: f32) -> Self {
        let up = Vec3::from_array(slot.up);
        let forward = Vec3::from_array(slot.forward);
        let mut position = Vec3::from_array(slot.position);
        if let Some(ground) = ground {
            position.y = ground;
        }

        Self {
            position: position + up * height,
            orientation: orientation_from_axes(forward, up),
        }
    }

    /// This pose re-pointed along `forward`, keeping its position and its up
    /// axis.
    ///
    /// **For the case where the authored slot's heading is stale and the track
    /// itself is the better witness**, which is a real state on Wipeout HD and
    /// on no other title measured: nine of its 28 circuit files carry a `Start
    /// Position` whose rotation was never re-authored for the direction the
    /// spline runs. Where the slot is right - which is all 24 PSP track files
    /// and 19 of HD's 28 - nothing calls this. See `oag_raceplay::spawn`,
    /// which holds the measurement and the rule.
    ///
    /// The up axis is taken from the pose rather than passed in, so a caller
    /// cannot re-point a ship and tilt it in the same step. A `forward` that is
    /// zero, or parallel to up, leaves the pose alone rather than returning the
    /// identity: the authored heading is wrong but present, and an arbitrary one
    /// is worse.
    #[must_use]
    pub fn facing(self, forward: Vec3) -> Self {
        let up = self.orientation * Vec3::Y;
        match try_orientation_from_axes(forward, up) {
            Some(orientation) => Self {
                orientation,
                ..self
            },
            None => self,
        }
    }
}

/// The orientation of a ship lying in `sample`'s own frame: forward along the
/// track's tangent, up the surface normal.
///
/// **What Pulse PSP's grid does** (`Race_ComputeGridLayout`'s per-slot matrix,
/// `FUN_0882663c`, built from the located sample rather than from the authored
/// node): measured on `16_Track`, the eight craft's forward `z` reads `-0.0025`
/// at slot 1 down to `-0.0061` at slot 8 where the node's own heading is
/// `0.0000` and the track's sample tangent at the same places reads `-0.0033` to
/// `-0.0057` - see `docs/physics/grid-state.md`. **Superseded for Pulse PSP by
/// [`crate::grid_walk`] (2026-10-02)**: the original's heading is the direction the track's
/// two edges run, not the tangent, and the 0.045 degrees left here is gone there. This
/// stays the fallback where that walk refuses a node and for every other title.
#[must_use]
pub fn orientation_on_sample(sample: &Sample) -> Quat {
    orientation_from_axes(
        Vec3::from_array(sample.tangent),
        -Vec3::from_array(sample.down),
    )
}

/// Builds a rotation whose forward is `-Z` and whose up is `+Y`, matching
/// [`oag_physics::Body`]'s axes.
///
/// `up` is orthogonalised against `forward` rather than trusted, because the
/// spline's interpolated axes are neither unit length nor exactly perpendicular
/// once blended. Degenerate input (either axis zero, or the two parallel) yields
/// the identity, which is wrong but recoverable, where a `NaN` quaternion would
/// poison the state hash for the rest of the race.
pub(crate) fn orientation_from_axes(forward: Vec3, up: Vec3) -> Quat {
    try_orientation_from_axes(forward, up).unwrap_or(Quat::IDENTITY)
}

/// The same, saying `None` where the input is degenerate instead of answering
/// with the identity.
///
/// The two are not the same question, and [`Pose::facing`] is why: `forward =
/// -Z` with `up = +Y` *is* the identity, so a caller that took the identity as
/// "that did not work" would refuse the one heading it is most likely to be
/// handed.
fn try_orientation_from_axes(forward: Vec3, up: Vec3) -> Option<Quat> {
    let z = -forward.normalize_or_zero();
    let y = up - z * up.dot(z);
    let (z, y) = (z.try_normalize()?, y.try_normalize()?);
    Some(Quat::from_mat3(&Mat3::from_cols(y.cross(z), y, z)))
}

impl Ship {
    /// Places this ship at `pose` and clears everything the force law carries.
    ///
    /// Velocity, both airbrakes, thrust and the magstrip blend all reset;
    /// `time_since_landing` starts outside the 0.2 s landing window so a fresh
    /// ship does not spawn using `landing_rebound`. The handling parameters and
    /// the slot's active flag are left alone, because respawning mid-race must
    /// not silently change which ship this is.
    ///
    /// **The energy pool is carried across too**, for the same reason and a
    /// sharper one: this is the respawn path, and a respawn that refilled the
    /// shield would make wall damage free. The original charges a respawn rather
    /// than paying one out - `Ship_SetState`'s state-3 branch computes
    /// `clamp(shield - 1, 0, 5)`. Filling the pool is `oag_physics::damage::reset`
    /// and belongs to taking the grid, not to a pose.
    pub fn place_at(&mut self, pose: Pose) {
        let mass = self.physics.body.mass;
        let inertia = self.physics.body.inertia;
        // The title's craft laws are the craft's, not its state, so a reset keeps them as it
        // keeps the inertia (the speed plan's rescues place a craft through here).
        let hover_rig = self.physics.hover_rig;
        let steer_ramp_clamped = self.physics.steer_ramp_clamped;
        let shield = self.physics.shield;
        // **The launch boost is carried across**, like the shield. A respawn does
        // not replay the window in the original (its timer holds through state 3 and
        // the multiplier stays 1.0, watched live), and a default `LaunchState` on a
        // released craft would start a fresh window at `normalMul` after every
        // respawn. See `oag_physics::launch`.
        let launch = self.physics.launch;
        self.physics = oag_physics::ShipState::default();
        self.physics.launch = launch;
        self.physics.body.position = pose.position;
        self.physics.body.orientation = pose.orientation;
        self.physics.body.mass = mass;
        self.physics.body.inertia = inertia;
        self.physics.hover_rig = hover_rig;
        self.physics.steer_ramp_clamped = steer_ramp_clamped;
        self.physics.shield = shield;
        // **And the weapon slowdown a hit still owes this craft**, which is the
        // one piece of the force law that does not live on `physics` and so is
        // not cleared by the assignment above. Whether the original's own reset
        // path zeroes `entity+0x130` is **not read** - `Ship_SetState`'s state-3
        // branch was followed for the energy charge and not for this - so this
        // is chosen rather than recovered, on the narrow ground that the line
        // above already clears `slowdown_timer`: leaving the *refill* behind
        // would put a craft back on the racing line and take its engine away on
        // its first tick there, from a hit it took before the teleport. Half a
        // mechanic surviving a reset is the shape to distrust.
        self.pending_slowdown = 0.0;
        // The same argument for the beam's one-shot throttle: a link the
        // teleport just broke (`Beam::link_broken` tests `active`) should not
        // take the first tick back on the line.
        self.pending_thrust_scale = 1.0;
    }
}

/// How far above the track's surface line a ship starts: the height its own
/// suspension holds it at.
///
/// Three candidate heights exist and they are three different quantities. Which one
/// a ship starts at changes the first second of every race, so the choice is written
/// down here rather than left as a number in a constructor.
///
/// - [`oag_vex::track::HOVER_LIFT`] is where the load pass puts the **AI line**,
///   by lifting each control point three units off the surface. It says nothing
///   about ships. Starting there leaves the suspension compressed by the difference,
///   and on the observed data one frame of the spring at that compression throws the
///   ship clear of the track. Measured, not predicted, which is why this is not it.
/// - `<Antigrav ride_height>` is what this used to return, and it is **wrong for a
///   reason worth keeping**: it is simultaneously the spring's target and the length
///   of the probe raycast, so a ship starting there sits at exactly the limit of its
///   own reach. Measured against the real collision mesh at the spawn, the surface is
///   between **5.468 and 5.542** below the probes while the cast is **5.500** - so
///   whether a probe reports contact is decided by the fourth decimal place of the
///   track geometry, and it flickers from the first tick. That flicker is an
///   off-centre force every tick and a gravity term stepping between its grounded and
///   airborne values every tick, which is what threw the ship clear.
/// - [`oag_physics::hover::target_height`] is where the spring pulls, but not where it
///   rests: it has to carry the ship, so it settles a little below its target. That
///   settled height is what this returns, and it is the only one of the three at which
///   both probes are in contact with margin on the first frame.
///
/// # The derivation, which is the force law's and not a number picked here
///
/// Each of the **two** probes contributes `mass * 0.3 * (target - h) * HOVER_K *
/// load * (normal_gravity + track_gravity)` along the ship's up axis, and a
/// grounded ship is pulled down by gravity, `normal_gravity * mass`, **plus the
/// hover downforce, `track_gravity * mass`** ([`oag_physics::hover::DOWNFORCE_SCALE`]).
/// Setting them equal at `load = 1` - a grounded ship, which is what a ship on a
/// grid is - and solving for the probe height gives
///
/// ```text
/// rest = target - (normal_gravity + track_gravity)
///                 / (2 * 0.3 * HOVER_K * (normal_gravity + track_gravity))
///      = target - 1.25          whenever both gravities are non-zero
/// ```
///
/// and the ship's *centre* sits [`oag_physics::hover::PROBE_DROP_RAW`] *
/// [`oag_physics::hover::TARGET_GLOBAL_SCALE`] above that, because the probes hang
/// below it. `mass` cancels, which is why none appears; the per-class gravity
/// scale is `1.0` for all four classes (`0x08ab0dcc`, read), which is why it does
/// not appear either.
///
/// **Two corrections landed here together, and the second is what makes the
/// number check out.** The old expression divided by *one* probe's gradient and
/// omitted the downforce, so it computed a sag of `0.147` units - `2.7 %` of the
/// reach - and its own documentation flagged that as an open reading. With both
/// halves right the sag is `1.25` of a `4.125` reach, and the resulting spawn
/// height on the shipped values is **`4.0`** against the original's own resting
/// craft height, read live on the start line at **`4.002`-`4.009`**. Nothing in
/// the expression is fitted; that agreement is the check.
///
/// This is only where a ship *starts*: the height it settles at is emergent from the
/// force law, as `oag_gameplay::spawn::Pose::from_sample` says. The two now agree,
/// which is the point.
/// How many craft a Pulse grid holds.
pub const GRID_SLOTS: u8 = 8;

/// How far ahead of its successor each grid slot sits, along the slot's forward
/// axis.
///
/// **Read out of the original's code** (2026-09-16): the literal `0x419e6666`
/// in `Race_ComputeGridLayout` (`0x0882b3b0`), the step it walks along the
/// spline's tangent from one slot to the next, re-locating after each step -
/// `docs/ghidra/functions/psp-pulse-usa/grid.md`. It was first *measured*: eight
/// craft read out of PPSSPP's memory on the grid put the per-slot step between
/// `19.763` and `19.862`, mean `19.79`, which is this value to within the
/// curvature of the start straight. The literal replaces the mean.
pub const GRID_ROW_PITCH: f32 = 19.8;

/// How far the odd-numbered slots sit to one side of the even-numbered ones.
///
/// The grid is two staggered columns, and this is the whole of the lateral
/// pattern: even slots sit on the authored node's own line, odd slots sit
/// `GRID_COLUMN_OFFSET` to the craft's **left**. Measured the same way as
/// [`GRID_ROW_PITCH`] and much tighter - the four odd slots read `19.882`,
/// `19.993`, `20.035` and `20.016`, and the four even ones are within `0.05` of
/// zero - and it is: the original offsets every slot `10.0` to alternating
/// sides of the AI corridor's midpoint (`Race_ComputeGridLayout`,
/// `docs/ghidra/functions/psp-pulse-usa/grid.md`), so the column-to-column
/// distance is exactly `20.0`. This crate keeps the even column on the node's
/// own line and the odd one `20.0` across, which is the same pair of columns
/// with a different zero.
///
/// **Left, and that is a trap this got wrong once.** The measurement is a
/// projection onto the original's row 0, and the original's row 0 is the craft's
/// *left* where [`oag_physics::Body::right`] is its right - see
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`. Measured directly on the same
/// grid: the original's row 0 reads `(-0.006, -0.008, -0.9999)` where our own
/// right axis reads `(0, 0, 1)`, exactly opposite. Taking the offset along
/// `Body::right` instead put the front of the grid off the outside of the
/// track's first corner, which `the_whole_grid_lands_on_the_track` catches and
/// no unit test on an identity node could.
pub const GRID_COLUMN_OFFSET: f32 = 20.0;

/// The pose of one grid slot, given the track's authored `Start Position`.
///
/// **The authored node is slot 8**, the back of the grid, and that is measured
/// rather than assumed: this crate's spawn from the node lands **1.84 units**
/// from where the original puts its own eighth craft, against **139.7** from
/// where a time trial starts. It also retires the puzzle
/// [`track.md`](../../../docs/formats/track.md#start-position) recorded - the
/// node being "3.2-20.5 units off the centreline" is not an authoring quirk, it
/// is the lateral stagger, and "137.9 units behind where a time trial starts" is
/// the length of the grid.
///
/// `slot` is 1-based, front to back, and out-of-range values clamp rather than
/// panicking: the caller's slot comes from a race option in the original and
/// there is no reason for a bad one to take the process down.
///
/// **All eight craft share one heading** - the orientation is the node's,
/// unmodified, which fourteen samples agree on to four decimal places.
///
/// Full account, including how the ordering is chosen:
/// [`grid.md`](../../../docs/ghidra/functions/psp-pulse-usa/grid.md).
#[must_use]
pub fn grid_pose(node: Pose, slot: u8) -> Pose {
    let slot = slot.clamp(1, GRID_SLOTS);
    let forward = node.orientation * Vec3::NEG_Z;
    // The craft's **left**, which is the original's row 0 - see
    // [`GRID_COLUMN_OFFSET`] for why this is not `Vec3::X`.
    let left = node.orientation * Vec3::NEG_X;
    let back = f32::from(GRID_SLOTS - slot);
    let lateral = if slot % 2 == 1 {
        GRID_COLUMN_OFFSET
    } else {
        0.0
    };
    Pose {
        position: node.position + forward * (back * GRID_ROW_PITCH) + left * lateral,
        orientation: node.orientation,
    }
}

#[must_use]
pub fn spawn_height(handling: &Handling) -> f32 {
    capped_spawn_height(handling, None)
}

/// [`spawn_height`] for a craft whose hover target is clamped on the grid
/// ([`oag_physics::hover::capped_target_height`]): the same rest height of the same spring, about
/// `0.16` under the lower target, so the craft starts where its suspension already holds it.
#[must_use]
pub fn capped_spawn_height(handling: &Handling, cap: Option<f32>) -> f32 {
    let target = oag_physics::hover::capped_target_height(handling, 0.0, 0.0, cap);
    let load = handling.physical.normal_gravity + handling.physical.track_gravity;
    let gradient = 2.0 * 0.3 * oag_physics::hover::HOVER_K * load;
    let drop = oag_physics::hover::PROBE_DROP_RAW * oag_physics::hover::TARGET_GLOBAL_SCALE;

    if gradient > 0.0 {
        target - load / gradient + drop
    } else {
        // A parameter set with no gravity has no spring either, so there is no rest
        // height to compute; the target is the honest answer, and `Handling::ZERO`
        // still gives zero.
        target
    }
}

/// The ship's rotational inertia, as a body-space diagonal.
///
/// `(15.6, 21.6, 15.6)` on `(right, up, forward)`, the same tensor for every craft
/// in the game. It is [`oag_physics::forces::ship_inertia`] and this function is
/// here only so the spawn path keeps one name for "the inertia a craft is built
/// with"; the derivation, the instruction listing and the captures that confirm it
/// live on that constant.
///
/// # It used to be derived from `<Misc>`, and that was wrong
///
/// The previous version of this function built a textbook box tensor from the
/// ship's hull dimensions - `I = m * (b^2 + c^2) / 12` over
/// `<Misc width/height/length>` at `<Physical mass>` - and said so honestly: "what
/// is **not** claimed is that the original computes it this way: how it builds a
/// tensor, or whether it uses one at all, was not recovered". It has since been
/// recovered, and it is not this:
///
/// - `Body_SetBoxInertia` (`0x0884e1ac`) is called from the ship-entity
///   constructor with the **code-literal** box `(12, 8, 12)` at a mass of `0.9`
///   set two calls earlier, at a single call site. So the tensor is a constant of
///   the game, not a property of the craft.
/// - `<Misc>`'s dimensions do reach that same constructor - scaled by `0.75` - but
///   they go to the **collider**, not to the inertia.
/// - The old derivation is also `4.4x` out on roll for the observed hull
///   (`3.5` against `15.6`), which is the axis the surface-alignment torque acts
///   on, so this is not a cosmetic difference.
///
/// The argument the old comment made for replacing `(1, 1, 1)` still holds and is
/// worth keeping, because it is the reason a wrong tensor is worse than a loud
/// failure: the probes sit half a hull length apart and the spring's gradient is
/// about 34 per unit, so one probe alone is a rotational stiffness near
/// `6.5^2 * 34`; divided by a unit inertia that is a 38 rad/s oscillator, and at
/// the specified `1/180` sub-step explicit Euler grows it about 5 % per tick.
/// Measured at rest with no input, roll grew from 0.026 to 0.570 rad over 51 ticks
/// and the ship inverted and fell through the floor by tick 250.
///
/// `<Misc weight_distribution>` is a fore/aft mass bias that nothing here models,
/// and on this reading nothing should: the original's tensor does not vary per
/// craft at all.
#[must_use]
pub fn box_inertia() -> Vec3 {
    oag_physics::forces::ship_inertia()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_vex::track::Sample;

    /// A flat piece of track running along `-Z`, which is the identity pose.
    fn level_sample() -> Sample {
        Sample {
            pos: [0.0, 0.0, 0.0],
            tangent: [0.0, 0.0, -1.0],
            down: [0.0, -1.0, 0.0],
            lateral: [1.0, 0.0, 0.0],
            half_width_left: 10.0,
            half_width_right: 10.0,
            ai_bound_left: 8.0,
            ai_bound_right: 8.0,
            racing_line: 0.0,
            section_id: 0,
            flags: 0,
            light_scale: [0xff; 4],
        }
    }

    #[test]
    fn a_level_sample_gives_the_identity_orientation() {
        let pose = Pose::from_sample(&level_sample(), 0.0, 0.0);
        assert_eq!(pose.position, Vec3::ZERO);
        assert!(pose.orientation.angle_between(Quat::IDENTITY) < 1e-5);
    }

    /// `down` points into the track, so a ship lifted off it moves along `-down`
    /// and not along `down`. Getting this backwards buries the ship.
    #[test]
    fn height_lifts_the_ship_against_the_down_axis() {
        let pose = Pose::from_sample(&level_sample(), 0.0, 4.0);
        assert_eq!(pose.position, Vec3::new(0.0, 4.0, 0.0));
    }

    #[test]
    fn a_lateral_offset_moves_across_the_track() {
        let pose = Pose::from_sample(&level_sample(), 3.0, 0.0);
        assert_eq!(pose.position, Vec3::new(3.0, 0.0, 0.0));
    }

    /// `Path::sample` blends unit vectors, which does not produce unit vectors.
    /// A rotation built without renormalising would scale the ship.
    #[test]
    fn unnormalised_axes_still_give_a_unit_rotation() {
        let sample = Sample {
            tangent: [0.0, 0.0, -0.31],
            down: [0.0, -0.77, 0.0],
            ..level_sample()
        };
        let pose = Pose::from_sample(&sample, 0.0, 1.0);
        assert!(pose.orientation.is_normalized());
        // A tolerance rather than equality: normalising 0.77 and multiplying
        // back gives 0.99999994, so the lift is one unit to within a rounding
        // step and not exactly. Asserting equality here would be asserting that
        // f32 division is exact, which is a different and false claim.
        assert!(
            (pose.position - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-6,
            "the lift is one unit however long the down axis was, got {}",
            pose.position
        );
    }

    /// An axis pair the blend could plausibly produce at a junction, where two
    /// paths' frames disagree. Identity is wrong but finite; a NaN quaternion
    /// would poison every state hash after it.
    #[test]
    fn degenerate_axes_give_the_identity_rather_than_nan() {
        let sample = Sample {
            tangent: [0.0, 0.0, 0.0],
            down: [0.0, 0.0, 0.0],
            ..level_sample()
        };
        let pose = Pose::from_sample(&sample, 0.0, 0.0);
        assert_eq!(pose.orientation, Quat::IDENTITY);
        assert!(pose.position.is_finite());
    }

    #[test]
    fn an_uphill_sample_pitches_the_ship_up() {
        let sample = Sample {
            tangent: [0.0, 0.5, -0.5],
            ..level_sample()
        };
        let pose = Pose::from_sample(&sample, 0.0, 0.0);
        let forward = pose.orientation * Vec3::NEG_Z;
        assert!(forward.y > 0.0, "nose should rise, got {forward}");
        assert!(
            (pose.orientation * Vec3::Y).y > 0.0,
            "up should still be up"
        );
    }

    #[test]
    fn placing_a_ship_clears_its_motion_but_not_its_identity() {
        let mut ship = Ship {
            active: true,
            segment: 7,
            ..Ship::default()
        };
        ship.physics.body.linear_velocity = Vec3::new(100.0, 0.0, 0.0);
        ship.physics.body.mass = 3.0;
        ship.physics.grounded = 1.0;
        ship.physics.slowdown_timer = 1.0;
        ship.pending_slowdown = 1.0;

        ship.place_at(Pose::from_sample(&level_sample(), 0.0, 2.0));

        assert_eq!(ship.physics.body.linear_velocity, Vec3::ZERO);
        assert_eq!(ship.physics.grounded, 0.0);
        // **Both halves of the weapon slowdown**, and the second is the one that
        // needs asserting: `pending_slowdown` is not on `physics`, so it does
        // not fall out of the state reset the way the timer does, and a craft
        // put back with a live credit would lose its engine on its first tick
        // on the racing line.
        assert_eq!(ship.physics.slowdown_timer, 0.0);
        assert_eq!(ship.pending_slowdown, 0.0);
        assert_eq!(ship.physics.body.position, Vec3::new(0.0, 2.0, 0.0));
        assert_eq!(ship.physics.body.mass, 3.0, "mass is not motion");
        assert!(ship.active, "respawning is not despawning");
        assert_eq!(
            ship.segment, 7,
            "and it does not lose its place on the track"
        );
    }

    /// A fresh ship must not be inside the landing window, or its first frame
    /// uses `landing_rebound` in place of `rebound`.
    #[test]
    fn a_placed_ship_is_not_in_the_landing_window() {
        let mut ship = Ship::default();
        ship.place_at(Pose::from_sample(&level_sample(), 0.0, 0.0));
        assert!(ship.physics.time_since_landing >= 0.2);
    }

    /// The frame `16_Track` ships: facing `+X`, which is `-Z` rotated a quarter
    /// turn about world up.
    fn level_slot() -> StartPosition {
        StartPosition {
            position: [8.0, -50.0, -196.0],
            left: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            forward: [1.0, 0.0, 0.0],
        }
    }

    #[test]
    fn a_slot_faces_the_way_the_track_authored_it() {
        let pose = Pose::from_start_position(&level_slot(), None, 0.0);
        assert_eq!(pose.position, Vec3::new(8.0, -50.0, -196.0));

        let forward = pose.orientation * Vec3::NEG_Z;
        assert!((forward - Vec3::X).length() < 1e-6, "got {forward}");
        let up = pose.orientation * Vec3::Y;
        assert!((up - Vec3::Y).length() < 1e-6, "got {up}");
    }

    /// The bind has already levelled the slot's up axis, so the lift is
    /// vertical - the same direction on every slot of every track.
    #[test]
    fn a_slot_lifts_along_world_up() {
        let pose = Pose::from_start_position(&level_slot(), None, 4.0);
        assert_eq!(pose.position, Vec3::new(8.0, -46.0, -196.0));
    }

    /// The authored `y` is not a ride height, so a caller that knows where the
    /// floor is measures from the floor and the slot's own height is discarded.
    #[test]
    fn a_known_ground_height_replaces_the_authored_one() {
        let pose = Pose::from_start_position(&level_slot(), Some(-60.0), 4.0);
        assert_eq!(pose.position, Vec3::new(8.0, -56.0, -196.0));
    }

    /// The two constructors agree where the inputs agree, which is what makes
    /// falling back to the spline a fallback rather than a different convention.
    #[test]
    fn a_slot_and_a_sample_facing_the_same_way_give_the_same_orientation() {
        let slot = StartPosition {
            position: [0.0, 0.0, 0.0],
            left: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            forward: [0.0, 0.0, -1.0],
        };
        let from_slot = Pose::from_start_position(&slot, None, 0.0);
        let from_sample = Pose::from_sample(&level_sample(), 0.0, 0.0);
        assert!(
            from_slot.orientation.angle_between(from_sample.orientation) < 1e-5,
            "{:?} vs {:?}",
            from_slot.orientation,
            from_sample.orientation
        );
    }
}

#[cfg(test)]
mod launch_tests {
    use super::*;
    use oag_physics::launch::{Grade, LaunchState};

    /// A respawn after the window has run does not start another: the grade, the
    /// latch and the clock survive the reset, and the multiplier stays neutral.
    #[test]
    fn a_respawn_does_not_replay_the_launch_window() {
        let spent = LaunchState {
            multiplier: LaunchState::NEUTRAL,
            grade: Grade::Stall,
            latched: true,
            ticks: 400,
        };
        let mut ship = Ship::default();
        ship.physics.launch = spent;

        ship.place_at(Pose {
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
        });

        assert_eq!(ship.physics.launch, spent);
    }

    /// A title's craft laws survive a respawn, as the inertia does: HD's four-probe rig and
    /// clamped steering ramp are the craft's, and the AI's speed plan rescues through here.
    #[test]
    fn a_respawn_keeps_the_titles_craft_laws() {
        let rig = oag_physics::hover::Rig {
            spring_share: 0.15,
            along_normal: true,
            ..oag_physics::hover::Rig::TWO_POINT
        };
        let mut ship = Ship::default();
        ship.physics.hover_rig = rig;
        ship.physics.steer_ramp_clamped = true;

        ship.place_at(Pose {
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
        });

        assert_eq!(ship.physics.hover_rig, rig);
        assert!(ship.physics.steer_ramp_clamped);
    }
}

#[cfg(test)]
mod grid_tests {
    use super::*;

    /// The identity node, so a slot's offset reads directly as its own numbers.
    fn node() -> Pose {
        Pose {
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
        }
    }

    /// Slot 8 is the authored node itself - the measurement this whole layout
    /// hangs off. If this ever fails, the grid has been re-anchored and every
    /// other number here means something different.
    #[test]
    fn slot_eight_is_the_authored_node() {
        assert_eq!(grid_pose(node(), 8).position, Vec3::ZERO);
    }

    /// Two staggered columns: odd slots offset along row 0, even slots on the
    /// node's own line.
    #[test]
    fn the_columns_alternate() {
        for slot in 1..=GRID_SLOTS {
            // The identity node's left is `-X`, so an odd slot lands negative.
            let x = grid_pose(node(), slot).position.x;
            if slot % 2 == 1 {
                assert!(
                    (x + GRID_COLUMN_OFFSET).abs() < 1e-4,
                    "slot {slot} should sit on the offset column, sits at {x}"
                );
            } else {
                assert!(x.abs() < 1e-4, "slot {slot} should sit on the node's line");
            }
        }
    }

    /// Slot 1 leads and slot 8 trails, with a constant step between them. The
    /// original's own eight craft span 138.55 units nose to tail; ours must too.
    #[test]
    fn the_grid_runs_forward_from_the_node_at_a_constant_pitch() {
        let z: Vec<f32> = (1..=GRID_SLOTS)
            .map(|s| grid_pose(node(), s).position.z)
            .collect();
        for pair in z.windows(2) {
            assert!(
                (pair[1] - pair[0] - GRID_ROW_PITCH).abs() < 1e-3,
                "the pitch is not constant: {pair:?}"
            );
        }
        let span = z[0] - z[GRID_SLOTS as usize - 1];
        assert!(
            (span + 138.53).abs() < 0.1,
            "the grid should span the original's measured 138.55 units, spans {span}"
        );
    }

    /// Every craft on the grid faces the same way, which fourteen samples of the
    /// original agree on to four decimal places.
    #[test]
    fn every_slot_shares_the_nodes_heading() {
        let base = node();
        for slot in 1..=GRID_SLOTS {
            assert_eq!(grid_pose(base, slot).orientation, base.orientation);
        }
    }

    /// The slot comes from a race option in the original, so a bad one clamps
    /// rather than panicking.
    #[test]
    fn an_out_of_range_slot_clamps() {
        assert_eq!(grid_pose(node(), 0).position, grid_pose(node(), 1).position);
        assert_eq!(
            grid_pose(node(), 99).position,
            grid_pose(node(), GRID_SLOTS).position
        );
    }
}
