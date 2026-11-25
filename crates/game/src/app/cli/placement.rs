//! Where the craft starts.

/// Where the craft starts.
#[derive(clap::Args, Debug)]
pub(crate) struct PlacementArgs {
    /// Start the craft at this world position instead of on its grid slot:
    /// `x,y,z` or `x,y,z,yaw`, `yaw` in degrees off the track's own direction
    /// there.
    ///
    /// **A capture aid, not a spawn.** Two circuits photographed from the same
    /// place, or one of our frames lined up with a position read out of an
    /// emulator's debugger, instead of running a guessed number of ticks and
    /// comparing whatever comes up. The position is used exactly as given -
    /// nothing lifts or snaps it - while attitude comes from the nearest spline
    /// sample, so a banked corner or a loop reads right rather than leaving the
    /// craft flat inside the geometry.
    #[arg(long, value_name = "X,Y,Z[,YAW]")]
    pub(crate) pose: Option<String>,

    /// Start the craft at a captured trace row's exact pose: position and full
    /// basis from the CSV, nothing recomputed from the spline.
    ///
    /// The other half of the comparison `--pose` was built for: an emulator
    /// frame and one of ours, from the same state. When the capture carries
    /// camera columns (`psp-trace.py --camera`), the frame is also rendered
    /// from the recorded camera pose; without them the chase camera frames the
    /// shot as usual.
    #[arg(long, value_name = "TRACE.CSV", conflicts_with = "pose")]
    pub(crate) pose_from: Option<std::path::PathBuf>,

    /// With `--pose-from` or `--pose` and `--camera-pose`: also put the opponents where the original had them on that tick. A CSV
    /// of one row per grid slot (`slot,team,left_x..,up_x..,fwd_x..,pos_x..`), what
    /// `scripts/psp-race-field.py` writes out of the emulator's RAM; each craft flies the team
    /// the row names and sits on the pose the row carries. A capture aid for a mid-race frame,
    /// single race only (the grid is what spawns the opponents).
    #[arg(long, value_name = "FIELD.CSV")]
    pub(crate) field_from: Option<std::path::PathBuf>,

    /// With `--field-from` and `--screenshot`: give every craft the exhaust history of a craft
    /// that has been flying at its captured velocity, so the frame has each craft's trail and
    /// a plume at the level a racing craft holds (a posed capture runs no tick and draws neither).
    /// The file needs `vel_x,vel_y,vel_z` columns and a slot 0 row for the player. Only the exhaust
    /// is stepped, the craft ending exactly on their captured poses; the straight path and full
    /// thrust are chosen, not measured (`Race::warm_up_field`).
    #[arg(long, requires = "field_from")]
    pub(crate) field_warmup: bool,

    /// With `--field-warmup`: a collision spark the original threw before the frame,
    /// `SLOT:NODE:AGE_SECONDS[:SEVERITY]`, repeatable. `NODE` is the locator's index in the hull's
    /// `Ship Collision Fx` list and `AGE_SECONDS` how long ago its gate stamp (`instance + 100`,
    /// minus `0.8`) says it fired (`scripts/psp-contact-fx-read.py`); the sparks are thrown
    /// that long before the frame and the effects run on to it, which a capture that runs no tick
    /// would otherwise never see. `SEVERITY` is the spawn scale (`intensity * 2 + 0.4`, `2.4`
    /// when absent: chosen, not measured).
    #[arg(
        long,
        value_name = "SLOT:NODE:AGE[:SEVERITY]",
        requires = "field_warmup"
    )]
    pub(crate) pose_spark: Vec<String>,
}
