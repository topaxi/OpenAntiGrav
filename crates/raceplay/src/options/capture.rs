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
    /// The shield as a percentage of the full pool (0 to 100), when the capture read it. A
    /// percentage rather than a pool value because the pool differs per title (HD's reads 140, ours
    /// is the handling file's own): the original's value over the original's maximum is what a frame
    /// shows.
    pub shield_percent: Option<f32>,
    /// The best lap so far in ticks, when the capture read it (`None` also when there is none).
    pub best_lap_ticks: Option<u32>,
    /// The same best lap in whole centiseconds, which is what the original stores (`entity+0x930`) and
    /// prints: `56.34` s is `3380.4` ticks, which no tick count reproduces, so `BestTime` reads this when
    /// set (`oag_hud::Readout::best_lap_centiseconds`).
    pub best_lap_centiseconds: Option<u32>,
    /// The player's exhaust boost timer over its length, `0..=1`, when the capture read it
    /// (`*(craft+0x78)+0xb8 / +0xb4` on Pulse PSP): a posed capture runs no ticks, so its flare has no
    /// timer, and the HUD's multi-copy boost draw runs on this (`oag_hud::boost_smear`).
    pub boost_phase: Option<f32>,
    /// Zone mode's counters as the original's frame showed them, when the capture read them.
    pub zone: Option<CaptureZone>,
}

/// Zone mode's own counters, read out of the original's RAM at the stop a Zone pair is taken at
/// (Pulse PSP: the mode object's `+0x1a10` zone number, `+0x1a1c` score and `+0x1a24` dwell timer,
/// `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`). They are written into the race's own state
/// before the first frame, so the HUD, the colour grade and the stage sphere all read the zone the
/// original was on; a posed capture runs no ticks, so nothing else would move them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureZone {
    /// The zone number.
    pub zone: u16,
    /// The score, when the capture read it.
    pub score: Option<i32>,
    /// Seconds into the current zone, when the capture read it.
    pub timer_seconds: Option<f32>,
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
