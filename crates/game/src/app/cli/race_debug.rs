//! Development and comparison switches for a race.

use super::detonator::{MineEnvelopes, MinePositions, parse_detonator_mines, parse_mine_envelopes};

/// Development and comparison switches for a race.
#[derive(clap::Args, Debug)]
pub(crate) struct RaceDebugArgs {
    /// Development override: pin the Zone colour grade to a stage for the
    /// whole run.
    ///
    /// **Both titles' ladders are recovered** - 2048's own zone-number
    /// table and HD/Fury's (`crates/hd/src/race.rs`'s
    /// `ZONE_STAGES`) - so a race normally escalates the grade on its own,
    /// re-deriving the stage from the zone counter every frame
    /// (`Scene::sync_zone_grade`). This flag exists for a **capture** rather
    /// than for seeing the look at all: without it, a stage 0 (`Start`) frame
    /// is not reachable at all, since even a zero-tick capture lands after
    /// the race's own opening-stage logic has already stepped past it (see
    /// `oag_raceplay::load::environment::zone_grade`), and there is otherwise
    /// no way to hold the grade still on a chosen rung long enough to compare
    /// it against the original frame for frame.
    ///
    /// Pins, not just sets once:
    /// [`oag_raceplay::zone_grade::ZoneGrade::pin_stage`] holds the stage
    /// against every later `show_zone` call, so the ladder does not
    /// overwrite it on the next frame the way a plain commit would.
    ///
    /// Clamped to the stages the loaded file names. Has no effect outside
    /// `--mode zone`, or on a title that ships no stage table.
    #[arg(long, value_name = "N")]
    pub(crate) zone_stage: Option<u32>,

    /// Comparison switch: hold Wipeout 2048's race-composite exposure at `E`
    /// instead of adapting it once a frame.
    ///
    /// **For the compare pairs only, never play.** Vita3K never writes the
    /// bloom gate's target back to the guest, so the original's exposure step
    /// reads zero and never runs there: every 2048 reference frame is drawn at
    /// the 1.15 `Exposure_Set` resets to, and `compare_env.ours_flags` passes
    /// `--hold-exposure 1.15` so a pair compares like with like. See
    /// `oag_raceplay::Options::exposure_hold`. No effect on a title without
    /// the chain.
    #[arg(long, value_name = "E")]
    pub(crate) hold_exposure: Option<f32>,

    /// Development override: draw the Zone visualiser glow from a fixed
    /// synthetic ramp instead of this project's own live audio spectrum.
    ///
    /// **The only way to see the glow in a capture at all.** `--screenshot`
    /// runs on the null audio backend, which never publishes a spectrum -
    /// so an ordinary capture shows no glow whatever the scene authors, and
    /// a real device's spectrum is live and non-deterministic, the wrong
    /// input for a comparison two runs are meant to agree on. See
    /// `crate::race_capture::spectrum::ZoneFeed`'s own doc comment.
    #[arg(long, conflicts_with = "zone_bands")]
    pub(crate) zone_spectrum_test: bool,

    /// Comparison input: lay the Detonator opening stage's mines at these world positions
    /// (`X,Y,Z;X,Y,Z;...`, the original's mine objects read in the frame's pause), each on the
    /// survey lane nearest to it, in place of the seeded layout.
    #[arg(
        long,
        value_name = "X,Y,Z;...",
        value_parser = parse_detonator_mines
    )]
    pub(crate) detonator_mines: Option<MinePositions>,

    /// Comparison input: hold each `--detonator-mines` mine's music envelope at the original's
    /// (`S,T;...`: `m+0xf4` and `m+0xf8`, same order), ours having no history of the music.
    #[arg(
        long,
        value_name = "S,T;...",
        value_parser = parse_mine_envelopes,
        requires = "detonator_mines"
    )]
    pub(crate) detonator_mine_envelopes: Option<MineEnvelopes>,

    /// Comparison input: start the Detonator EMP at this many charges (the gauge they give), as
    /// the original's charge was written for the capture it is compared with.
    #[arg(long, value_name = "N")]
    pub(crate) detonator_emp_charge: Option<u32>,

    /// Comparison input: draw the Zone and Detonator music-driven look from these
    /// sixteen held band levels, read off the original at the frame being compared
    /// (`+0x32f8` of its Environment stage block, comma-separated).
    ///
    /// The floor's visualiser reads these as they are, with no hold applied on top.
    /// Without `--zone-bands-raw` the Detonator mine's envelope reads them too.
    /// Replaces the live spectrum the way `--zone-spectrum-test` does, with
    /// the original's levels instead of a ramp.
    #[arg(
    long,
    value_name = "B0,...,B15",
    value_parser = crate::race_capture::spectrum::parse_bands
)]
    pub(crate) zone_bands: Option<crate::race_capture::spectrum::Bands>,

    /// The sixteen raw band levels (`+0x32b8` of the stage block) the Detonator mine's
    /// envelope reads; defaults to `--zone-bands`.
    #[arg(
        long,
        value_name = "R0,...,R15",
        requires = "zone_bands",
        value_parser = crate::race_capture::spectrum::parse_bands
    )]
    pub(crate) zone_bands_raw: Option<crate::race_capture::spectrum::Bands>,

    /// Development view: draw the driveable ribbon instead of the track's art
    /// meshes.
    ///
    /// The ribbon is the geometry the simulation actually spawns on and queries,
    /// so ship-plus-ribbon shows directly whether the ship is where the physics
    /// thinks it is - useful for a physics comparison and misleading about
    /// everything else. A race draws the map by default.
    #[arg(long)]
    pub(crate) ribbon: bool,

    /// Overlay the collision soup - the geometry the physics world is actually
    /// made of, the same view `oag-view --collision` draws - on top of the
    /// track model chosen above.
    #[arg(long)]
    pub(crate) collision: bool,

    /// Spawn the rest of the grid even though `--mode` races solo in the
    /// original.
    ///
    /// A verification aid - see `race::Options::opponents` - for looking at
    /// the measured grid layout without a mode that actually fields one.
    /// `time_trial`, `speed_lap` and `zone` all race with `AI DIFFICULTY`
    /// greyed to `N/A` on the real Custom Race screen, and this flag exists
    /// despite that rather than because of it.
    #[arg(long)]
    pub(crate) opponents: bool,

    /// Play the trail-hit sparks on the player continuously, whatever the
    /// trigger says.
    ///
    /// **A verification aid for the *drawing*, not for the trigger** - the two
    /// fail differently and this separates them. `WO_TRAIL_HITSHIP` fires when
    /// a craft flies into another's engine trail, which under `--autopilot`
    /// happens a handful of times in a whole race and almost never in front of
    /// the camera, so "I saw nothing" cannot tell a burst that never played
    /// from one that played and drew nothing. With this on, a burst re-ignites
    /// at the player's nozzle every half second: if the screen stays empty the
    /// fault is in the draw path, and if it does not, the trigger is what to
    /// look at. See `RaceView::advance_trail_hits`.
    #[arg(long)]
    pub(crate) trail_sparks: bool,
}
