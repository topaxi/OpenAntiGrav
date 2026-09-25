//! Flag groups `clap` flattens into [`super::Cli`], each its own struct so
//! `cli.rs` stays under the size ratchet.

use std::path::PathBuf;

/// Writing the window icon.
#[derive(clap::Args, Debug)]
pub(crate) struct IconArgs {
    /// Rasterise `assets/icons/64x64.svg` to a PNG at `--icon-size` and exit.
    ///
    /// Touches no disc, no settings file and no window - only `oag_game::icon`.
    /// What `just install-desktop-file` and `scripts/build-appimage.sh` both
    /// call, so the icon on disk and the one `main/window.rs` hands winit at
    /// startup come from the same rasterizer and never drift apart:
    ///
    /// ```sh
    /// cargo run -p oag-game -- --write-icon /tmp/oag-game.png --icon-size 256
    /// ```
    #[arg(long, value_name = "FILE.png")]
    pub(crate) write_icon: Option<PathBuf>,

    /// Edge length in pixels for `--write-icon`.
    ///
    /// The source is a vector, so any size re-renders cleanly rather than
    /// upscaling a raster; 256 is what the freedesktop hicolor icon theme
    /// wants for `apps/`.
    #[arg(long, requires = "write_icon", default_value_t = 256)]
    pub(crate) icon_size: u32,
}

/// Racing and recording a ghost in a headless `--race --screenshot` capture.
///
/// **Explicit files, never the config directory.** A capture reads
/// `records.toml` and never writes it (see `race::CaptureOptions::
/// previous_best`); a ghost is the same: `--ghost` names the file to race and
/// `--record-ghost` the file the run's best lap goes to, so a scripted capture
/// cannot overwrite a player's own best. See ADR-0055.
#[derive(clap::Args, Debug)]
pub(crate) struct GhostArgs {
    /// Race the ghost lap stored in this replay file.
    ///
    /// The file a session writes under `<config dir>/oag/ghosts/`, or one an
    /// earlier `--record-ghost` wrote. Its key is not checked against this
    /// race's - naming a file is the intent.
    #[arg(long, value_name = "FILE.oagr")]
    pub(crate) ghost: Option<PathBuf>,

    /// Record this run and write its best lap, as a ghost, to this file.
    ///
    /// Written once, after the last tick, when the run completed a lap -
    /// whatever is already there.
    ///
    /// ```sh
    /// cargo run -p oag-game -- --race --mode time_trial --autopilot \
    ///     --ticks 9000 --record-ghost data/scratch/lap.oagr --screenshot /dev/null
    /// ```
    #[arg(long, value_name = "FILE.oagr")]
    pub(crate) record_ghost: Option<PathBuf>,
}

/// Timing the loading screen's hand-off to a race, in a window.
#[derive(clap::Args, Debug)]
pub(crate) struct MeasureArgs {
    /// Skip the boot sequence, launch this many races from the menus one
    /// after another, print every main-thread frame of each load-to-race
    /// transition that went over a 60 Hz frame's budget, and quit.
    ///
    /// The windowed route a player takes - the menus' own `LAUNCH RACE`, the
    /// loading screen, the hand-off - rather than `--race`, which loads before
    /// the window opens and never shows the loading screen at all. The first
    /// run pays a cold disc cache and the second does not, so asking for two
    /// separates the two. Measure a release build: a debug one is several
    /// times slower on exactly the work this times.
    ///
    /// ```sh
    /// cargo run --release -p oag-game -- data/images/pulse-psp-eu.chd \
    ///     --no-audio --measure-race-load 2
    /// ```
    #[arg(long, value_name = "RUNS", num_args = 0..=1, default_missing_value = "1")]
    pub(crate) measure_race_load: Option<u32>,
}
