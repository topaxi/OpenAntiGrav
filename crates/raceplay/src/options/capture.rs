//! The capture aids an [`Options`] can carry: a craft pose and a camera.
//!
//! Split out of `options.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// Where [`Options::pose`] puts the craft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoseRequest {
    /// A position plus a yaw off the track's own direction there; pitch and
    /// roll come from the nearest spline sample. What `--pose X,Y,Z[,YAW]`
    /// always meant - see
    /// [`oag_gameplay::spawn::Pose::from_position_on_sample`].
    SplineAligned {
        /// Where to put the craft, world space.
        position: Vec3,
        /// Radians off the spline tangent at that point.
        yaw: f32,
    },
    /// A full pose, applied verbatim - a captured trace row's position and
    /// basis, nothing recomputed from the spline.
    Exact(Pose),
}

/// The HUD's race state imposed from outside, for a frame of the original taken mid-race.
///
/// **A capture aid beside [`Options::pose`]**: a craft posed with `--pose-from` and no ticks run
/// reads lap 1, zero clocks and the grid place, where the original's frame says "place 6 of 8,
/// lap 1 of 3, 30.0 s". Those four numbers are read out of the original's RAM at the same stop as
/// the pose and handed in here; nothing is derived. `None` is every race.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureHud {
    /// The player's place, 1-based.
    pub place: u32,
    /// The lap the player is on, 1-based.
    pub lap: u32,
    /// Ticks since the lap began.
    pub lap_ticks: u64,
    /// Ticks of racing clock since the countdown released the craft.
    pub race_ticks: u64,
    /// The speed readout in km/h, when the capture read it: a posed craft is at rest, so its own
    /// reads zero.
    pub speed_kmh: Option<f32>,
    /// The shield pool, in the pool's own units, when the capture read it.
    pub shield: Option<f32>,
    /// The best lap so far in ticks, when the capture read it (`None` also when there is none).
    pub best_lap_ticks: Option<u32>,
}

/// A camera pose imposed from outside, replacing the chase camera.
///
/// The one seam is [`Race::view`]: [`Race::camera_position`] derives from the
/// view matrix, so the PVS culling eye and the fog eye follow the override
/// without knowing it exists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraOverride {
    /// The camera eye, world space.
    pub eye: Vec3,
    /// The camera's orientation, in the same convention as the ship body's:
    /// `-Z` is the look direction, `Y` is up. What
    /// [`oag_trace::replay::camera_orientation_of`] produces.
    pub orientation: Quat,
    /// Replace the disc's authored fov, in **vertical degrees**, with this
    /// value. `None` keeps the authored one.
    ///
    /// This was the calibration knob for settling the unit, and the unit is now
    /// settled (confidence 94). What it is *for* has changed rather than gone
    /// away: a matched-pose render still needs it, because
    /// [`oag_gameplay::spawn::Ship::place_at`] resets the body, so a posed craft
    /// has zero velocity and never gets the original's speed-dependent widen.
    /// Pass the fov computed from the captured tick's own forward velocity.
    pub fov_deg: Option<f32>,
}
