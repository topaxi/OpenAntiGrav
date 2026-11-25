//! How a posed craft is drawn: exhaust, lean and flaps.

/// How a posed craft is drawn: exhaust, lean and flaps.
#[derive(clap::Args, Debug)]
pub(crate) struct PoseDrawArgs {
    /// With `--screenshot`: force the exhaust into the state it would hold this
    /// many seconds after entering a speed pad, at saturated intensity.
    ///
    /// The frame-comparison knob for the boost visuals: a posed capture
    /// (`--pose-from --ticks 0`) never crosses a pad, so without this the flare
    /// renders cold and the plume not at all, and there is nothing to compare
    /// against an emulator shot taken mid-boost. `0.0` is the entry tick.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) pose_boost: Option<f32>,

    /// With `--pose-boost`: the intensity the exhaust had when the pad was
    /// entered, instead of saturating it.
    ///
    /// **Saturated is the wrong default for a comparison against a teleported
    /// capture, and this is what fixes it.** `Exhaust_Update`'s intensity ramp
    /// climbs at only `0.25`/s, so it needs four seconds of thrust to reach
    /// `1.0`; a craft put on a pad's approach by `psp-drive.py place` has had
    /// far less than that, and the first real pad capture measured `0.1334` at
    /// the entry tick. Intensity is not cosmetic there - it sets the flare's
    /// resting half-size through `(i * 0.6 + 0.4) * 2.5` (`1.2` at `0.13`
    /// against `2.5` saturated) and all three of the ribbon's staggered layer
    /// alphas - so comparing a saturated render against an unsaturated capture
    /// measures the difference in *state*, not in rendering.
    ///
    /// Take the value from a `psp-trace.py --flare` capture's `intensity`
    /// column at the entry tick. The ramp then keeps climbing through the
    /// posed age at the same rate the original's does, so one number matches
    /// the whole curve rather than just its start.
    #[arg(long, value_name = "0..1", requires = "pose_boost")]
    pub(crate) pose_intensity: Option<f32>,

    /// With `--pose-boost`: the speed in units/s to advance the exhaust at,
    /// instead of the default racing `120`.
    ///
    /// Only reaches the picture through `Exhaust_Update`'s speed ramp, which
    /// floors the boost accumulator at `clamp((kmh - 100) / 500) * 0.6` - so it
    /// matters when a capture's speed is far from `120`. Take it from the
    /// capture's `speed` column.
    #[arg(long, value_name = "UNITS_PER_S", requires = "pose_boost")]
    pub(crate) pose_speed: Option<f32>,

    /// With a posed craft (`--pose-from` or `--pose`): put the craft back on that pose for the
    /// frame `--screenshot` draws and the last row `--trace-out` writes (the row at `--ticks`),
    /// after the countdown and the clock have run.
    ///
    /// For a pair taken at race clock `T`: the original's pose has already turned for `T` seconds
    /// at rest (bank-to-yaw, `docs/physics/grid-state.md`), so running `T` more seconds from it
    /// turned ours twice. Position and orientation only; the clock, the GO board, the record
    /// panel and the exhaust run as they did.
    #[arg(long)]
    pub(crate) pose_hold: bool,

    /// With `--pose-hold` and `--screenshot`: also put the craft back on that pose, at rest,
    /// after every tick.
    ///
    /// For a weapon fired from a resting craft: the shot then leaves along the pose's own
    /// heading, where `--pose-hold` alone fires it from wherever the craft has turned to and
    /// only draws the hull back on the pose.
    #[arg(long, requires = "pose_hold")]
    pub(crate) pose_hold_ticks: bool,

    /// With `--pose-from`: also draw the craft with the lean and barrel-roll phase the pose's own trace row recorded
    /// (`steer_lean`, `roll_phase`), put on after the ticks.
    ///
    /// A posed capture runs no tick, so the hull is drawn flat whatever the original's stick was doing at the
    /// instant; the lean is display state the original's per-tick update writes (`entity+0x854`), not part of the pose.
    /// The row is the one `--pose-tick` poses from, and a trace without the columns is refused.
    #[arg(long, requires = "pose_from")]
    pub(crate) pose_lean: bool,

    /// With `--pose-from`: also draw the craft's airbrake flaps at the levels the pose's own trace row recorded
    /// (`airbrake_l`, `airbrake_r`, `0..=100`), put on after the ticks.
    ///
    /// A posed capture runs no tick that swings the flaps, so they draw stowed whatever the original's air brakes
    /// were doing at the instant. The row is the one `--pose-tick` poses from.
    #[arg(long, requires = "pose_from")]
    pub(crate) pose_flaps: bool,

    /// With `--race --screenshot`: draw every craft's flaps stowed through the grid state, which holds
    /// them fully out in play. For a comparison against an original that stows them, and for a frame
    /// of the flaps alone: the same race with and without this differs in the flaps and nothing else.
    /// A posed capture (`--pose-from`, `--field-from`, `--pose`) stows them without it.
    #[arg(long)]
    pub(crate) no_grid_flaps: bool,
}
