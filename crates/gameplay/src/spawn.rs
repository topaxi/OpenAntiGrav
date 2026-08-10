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

use oag_core::math::{Mat3, Quat, Vec3};
use oag_formats::track::{Sample, StartPosition};
use oag_physics::Handling;

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
            .map_or(forward, |axis| Quat::from_axis_angle(axis, yaw) * forward);

        Self {
            position,
            orientation: orientation_from_axes(forward, up),
        }
    }

    /// A pose from the track's own authored grid slot, lifted off the surface.
    ///
    /// [`oag_formats::track::StartPosition`] is where the exporter put a ship and
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
}

/// Builds a rotation whose forward is `-Z` and whose up is `+Y`, matching
/// [`oag_physics::Body`]'s axes.
///
/// `up` is orthogonalised against `forward` rather than trusted, because the
/// spline's interpolated axes are neither unit length nor exactly perpendicular
/// once blended. Degenerate input (either axis zero, or the two parallel) yields
/// the identity, which is wrong but recoverable, where a `NaN` quaternion would
/// poison the state hash for the rest of the race.
fn orientation_from_axes(forward: Vec3, up: Vec3) -> Quat {
    let z = -forward.normalize_or_zero();
    let y = up - z * up.dot(z);
    let (Some(z), Some(y)) = (z.try_normalize(), y.try_normalize()) else {
        return Quat::IDENTITY;
    };
    Quat::from_mat3(&Mat3::from_cols(y.cross(z), y, z))
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
        let shield = self.physics.shield;
        self.physics = oag_physics::ShipState::default();
        self.physics.body.position = pose.position;
        self.physics.body.orientation = pose.orientation;
        self.physics.body.mass = mass;
        self.physics.body.inertia = inertia;
        self.physics.shield = shield;
    }
}

/// How far above the track's surface line a ship starts: the height its own
/// suspension holds it at.
///
/// Three candidate heights exist and they are three different quantities. Which one
/// a ship starts at changes the first second of every race, so the choice is written
/// down here rather than left as a number in a constructor.
///
/// - [`oag_formats::track::HOVER_LIFT`] is where the load pass puts the **AI line**,
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
#[must_use]
pub fn spawn_height(handling: &Handling) -> f32 {
    let target = oag_physics::hover::target_height(handling, 0.0, 0.0);
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
    use oag_formats::track::Sample;

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

        ship.place_at(Pose::from_sample(&level_sample(), 0.0, 2.0));

        assert_eq!(ship.physics.body.linear_velocity, Vec3::ZERO);
        assert_eq!(ship.physics.grounded, 0.0);
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
