//! The command line: every flag `oag-game` takes, and nothing else.
//!
//! It is pure declaration - `clap` derives the parser, and every consumer
//! (`main`, [`crate::app::headless`], [`crate::app::args`]) reads fields off it.
//!
//! **A new flag goes in the file of its area**, as a field of that area's
//! `*Args` struct (`boot`, `loading`, `audio`, `capture`, `race_setup`,
//! `render`, `placement`, `hud` and the others below), so two flags added
//! for different areas never edit the same lines. [`Cli`] holds the groups
//! and the few flags that belong to none. A group is flattened at its position
//! in [`Cli`], so `--help` lists the flags in declaration order; a flag's name,
//! and the id other flags `requires` it by, is its field's name.

use clap::Parser;

use oag_display::display;
use oag_gameplay::ControlScheme;

mod audio;
pub(crate) mod benchmark;
mod boot;
mod capture;
mod capture_view;
pub(crate) mod detonator;
mod diagnostics;
pub(crate) mod extra;
mod fury;
mod hud;
mod loading;
mod placement;
mod pose_draw;
mod race_debug;
mod race_run;
mod race_setup;
mod render;
mod render_overrides;

#[derive(Parser, Debug)]
#[command(
    name = "oag-game",
    about = "Run a Wipeout game (Pulse, Pure, HD/Fury, 2048 or Omega) from your own copy",
    // The commit too, so a deployed build can be told from the last one.
    version = oag_net::BUILD_VERSION
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub(crate) boot: boot::BootArgs,

    /// Render one frame to a PNG and exit, without opening a window.
    #[arg(long)]
    pub(crate) screenshot: Option<std::path::PathBuf>,
    #[command(flatten)]
    pub(crate) loading: loading::LoadingArgs,
    #[command(flatten)]
    pub(crate) audio: audio::AudioArgs,

    /// Ours: offer every circuit in the Race Box, whatever is cleared (`crate::unlock`).
    #[arg(long)]
    pub(crate) unlock_all: bool,

    /// Record what the **device** is handed to a WAV, while it plays.
    ///
    /// Not `--dump-audio`, which forces the null backend so that it can pull
    /// from the mixer without racing a callback that is already draining it.
    /// This one taps the callback itself, so it is the only way to see the
    /// signal a real sound card received - which is what a fault that only
    /// happens with hardware attached needs.
    ///
    /// Recording starts when the stream opens, so pair it with `--race` to skip
    /// the front end. The file is written once it is full, from the frame loop,
    /// so a run killed at the terminal still leaves it behind.
    #[arg(long, value_name = "FILE")]
    pub(crate) tap_audio: Option<std::path::PathBuf>,

    /// How many seconds `--tap-audio` records. Preallocated, so it costs about
    /// 384 KB a second.
    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 90.0,
        requires = "tap_audio"
    )]
    pub(crate) tap_seconds: f32,

    /// Pin the animation clock, in seconds, instead of deriving it from the
    /// tick.
    ///
    /// Three mechanisms run off one clock, which a race takes from the tick:
    /// the two trackside ones - the per-material texture transform and the
    /// `Anim Transform` node motion - and Wipeout HD's
    /// engine-flame surface scroll, which the original drives from the same
    /// kind of global clock (`time`, engine shader parameter slot 0). This
    /// overrides it, for a comparison harness that needs our phase matched to a
    /// still of the original rather than left where the tick put it. Two runs
    /// at two values are also the headless way to show that an animated surface
    /// moves at all - and two runs exactly one period apart are how the flame's
    /// scroll rate was checked, since they must come back identical. Same flag,
    /// same meaning as `oag-view --anim-seconds`.
    ///
    /// **This is deliberately not a settings-file option.** It was one -
    /// `[graphics] animated_textures`, a boolean - and it was a footgun: it
    /// dated from when the animation was a guess, it grew to freeze moving
    /// scenery as well as scrolling surfaces without its name saying so, and a
    /// `false` persisted from before the default flipped left circuits silently
    /// static. A harness knob belongs on the command line.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) anim_seconds: Option<f32>,
    #[command(flatten)]
    pub(crate) fury: fury::FuryArgs,
    #[command(flatten)]
    pub(crate) capture: capture::CaptureArgs,

    /// Control scheme: `veteran` or `novice`.
    ///
    /// Overrides `[controls] scheme` for this run without writing it back. The
    /// two differ only in how a sideshift is asked for - veteran double-taps an
    /// airbrake, novice holds the sideshift button and flicks the stick - and
    /// they are the original's own `Control_Type` values. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    #[arg(long)]
    pub(crate) scheme: Option<ControlScheme>,

    /// Draw a frame counter over the intro.
    ///
    /// On by default when there is no picture, since a black screen for eight
    /// seconds is otherwise indistinguishable from a hang.
    #[arg(long)]
    pub(crate) overlay: bool,

    /// With `--screenshot` or `--drive`: leave out what this build adds to the disc's own screens
    /// (2048's RACEBOX, a mod's race page and MULTIPLAYER tiles on `GameModeChoice`, and the pad focus
    /// on its touch grids; the PHOTO MODE row of a pause screen that authors none), so a frame can be compared against a capture of the original's menu definition. A player always has them.
    #[arg(long)]
    pub(crate) original_menu: bool,

    /// `--menu`, `--menu-page`, `--menu-anim-phase`, `--menu-picker-seconds`
    /// and `--menu-prompt` - see [`extra::MenuArgs`].
    #[command(flatten)]
    pub(crate) menu_args: extra::MenuArgs,

    /// `--demo`, `--wallpaper` - see [`extra::DemoArgs`].
    #[command(flatten)]
    pub(crate) demo: extra::DemoArgs,

    /// `--benchmark`, `--preset` - see [`benchmark::BenchmarkArgs`].
    #[command(flatten)]
    pub(crate) benchmark: benchmark::BenchmarkArgs,
    #[command(flatten)]
    pub(crate) diagnostics: diagnostics::DiagnosticsArgs,

    /// Skip the front end and go straight to a ship on a track.
    ///
    /// Arrow keys steer, X or Return thrusts, Q and E are the airbrakes. The
    /// same race the front end's `Launch Game` starts.
    #[arg(long)]
    pub(crate) race: bool,
    #[command(flatten)]
    pub(crate) race_setup: race_setup::RaceSetupArgs,
    #[command(flatten)]
    pub(crate) race_debug: race_debug::RaceDebugArgs,
    #[command(flatten)]
    pub(crate) detonator: detonator::DetonatorArgs,

    #[command(flatten)]
    pub(crate) driver: extra::DriverArgs,
    #[command(flatten)]
    pub(crate) race_run: race_run::RaceRunArgs,
    #[command(flatten)]
    pub(crate) render: render::RenderArgs,
    #[command(flatten)]
    pub(crate) placement: placement::PlacementArgs,
    #[command(flatten)]
    pub(crate) hud: hud::HudArgs,
    #[command(flatten)]
    pub(crate) capture_view: capture_view::CaptureViewArgs,
    #[command(flatten)]
    pub(crate) pose_draw: pose_draw::PoseDrawArgs,

    /// With `--race --screenshot` or `--race --trace-out`: drive the run from
    /// a committed `.inputs` script (see `scripts/input_script.py` for the
    /// format) instead of `--hold`/`--press` - the same file
    /// `scripts/psp-trace.py --script` feeds the emulator, so one authored
    /// input produces both sides of a visual or CSV comparison. Ticks past
    /// the script's end hold its last state.
    ///
    /// `--trace-out` reads it too: `HeldButtons::advance` is where both legs
    /// make the same choice, so they cannot diverge (a trace that ignored it
    /// would be a plausible CSV of a craft that never moved).
    #[arg(long, value_name = "FILE.inputs", requires = "race")]
    pub(crate) input_script: Option<std::path::PathBuf>,

    /// Which of the three in-race camera perspectives to fly with, overriding
    /// `[graphics] camera_view` for this run: `internal`, `close` or `far`.
    ///
    /// In-game the choice is cycled with SELECT (TAB on a keyboard) and persisted,
    /// so this flag exists for the case a keypress cannot reach: a **headless
    /// `--screenshot`**, which is the only way to get a frame of the cockpit view
    /// on a machine that never opens a window. Unlike the settings row, it does not
    /// persist.
    ///
    /// Gated on `--screenshot` the way `--camera-fov` is gated on `--pose-from`,
    /// and for the same reason: a windowed run reads the view from the settings
    /// file and cycles it from the button, so a flag accepted there would be
    /// silently dropped. Refusing it says so instead.
    #[arg(long, value_name = "VIEW", requires = "screenshot")]
    pub(crate) camera_view: Option<display::CameraView>,

    /// With `--race`: write our per-tick state to a CSV in the capture's own
    /// columns, so it can be differenced against one from
    /// `scripts/psp-trace.py`.
    ///
    /// **Simulation only** - no window, no GPU, no screenshot. The point is to
    /// stop verifying the exhaust by looking at pictures: `oag-trace compare
    /// data/traces/pad0-boost.csv ours.csv` puts a number on every column
    /// instead. Writes the eight `--flare` columns beside the ship state, so a
    /// capture taken with `psp-trace.py --flare` compares on all of them.
    ///
    /// Combine with `--input-script` to drive it from an authored `.inputs`
    /// file instead of `--hold`/`--press` - a second, non-rendering way to
    /// get a scripted run's full per-tick basis, more convenient than
    /// `oag-trace run` for anyone already working against this binary's own
    /// load path rather than a captured reference trace.
    ///
    /// Combining it with `--screenshot` is refused rather than ignored: the
    /// screenshot route renders through `crate::race_capture::capture`, and threading a writer
    /// into that loop is a change to a file this flag deliberately does not
    /// touch. Run the two separately.
    #[arg(long, value_name = "FILE.csv", conflicts_with = "screenshot")]
    pub(crate) trace_out: Option<std::path::PathBuf>,

    /// `--write-icon` and `--icon-size` - see [`extra::IconArgs`].
    #[command(flatten)]
    pub(crate) icon: extra::IconArgs,

    /// `--png-info` and `--from-png` - see [`extra::ShotArgs`].
    #[command(flatten)]
    pub(crate) shot: extra::ShotArgs,

    /// `--ghost` and `--record-ghost` - see [`extra::GhostArgs`].
    #[command(flatten)]
    pub(crate) ghost: extra::GhostArgs,

    /// `--measure-race-load` - see [`extra::MeasureArgs`].
    #[command(flatten)]
    pub(crate) measure: extra::MeasureArgs,

    /// `--force-wreck` and `--no-hull-wreck` - see [`extra::WreckArgs`].
    #[command(flatten)]
    pub(crate) wreck: extra::WreckArgs,
    #[command(flatten)]
    pub(crate) intro: extra::IntroArgs,
    /// `--preseat`, `--script` and `--frame-log` - see [`extra::DevArgs`].
    #[command(flatten)]
    pub(crate) dev: extra::DevArgs,
    /// `--connect` and its companions - see [`extra::NetArgs`].
    #[command(flatten)]
    pub(crate) net: extra::NetArgs,
    #[command(flatten)]
    pub(crate) log: oag_log::tool::LogArgs,
}

impl Cli {
    /// Whether a person is at this launch, as opposed to a capture, a script, a trace or a
    /// comparison: only then may the first-run name prompt open. An allowlist, so a flag added
    /// later is automation until it is listed here.
    pub(crate) fn is_interactive(&self) -> bool {
        let scripted = self.dev.preseat
            || self.dev.script.is_some()
            || self.dev.drive.is_some()
            || self.dev.drive_file.is_some();
        let captured = self.screenshot.is_some()
            || self.audio.dump_audio.is_some()
            || self.tap_audio.is_some()
            || self.capture.until.is_some()
            || self.capture.screen.is_some()
            || self.capture.press.is_some()
            || self.capture.hold.is_some()
            || self.loading.loading_screen.is_some();
        !(scripted
            || captured
            || self.original_menu
            || self.race
            || self.diagnostics.dry_run
            || self.diagnostics.trace
            || self.diagnostics.probe_pad
            || self.diagnostics.fetch_ffmpeg
            || self.boot.prefetch
            || self.net.connect.is_some()
            || self.net.online.is_some())
    }

    /// `--seed`; always `None` in a build without `dev-cli`.
    pub(crate) fn seed(&self) -> Option<u64> {
        #[cfg(feature = "dev-cli")]
        return self.race_run.seed;
        #[cfg(not(feature = "dev-cli"))]
        None
    }

    /// `--force-rocket-hit-velocity`; always `None` in a build without `dev-cli`.
    pub(crate) fn force_rocket_hit_velocity(&self) -> Option<&str> {
        #[cfg(feature = "dev-cli")]
        return self.wreck.force_rocket_hit_velocity.as_deref();
        #[cfg(not(feature = "dev-cli"))]
        None
    }

    /// `--pose-hold-launch`; always `None` in a build without `dev-cli`.
    pub(crate) fn pose_hold_launch(&self) -> Option<&str> {
        #[cfg(feature = "dev-cli")]
        return self.wreck.pose_hold_launch.as_deref();
        #[cfg(not(feature = "dev-cli"))]
        None
    }
    /// `--force-shot-at`; always `None` in a build without `dev-cli`.
    pub(crate) fn force_shot_at(&self) -> Option<&str> {
        #[cfg(feature = "dev-cli")]
        return self.wreck.force_shot_at.as_deref();
        #[cfg(not(feature = "dev-cli"))]
        None
    }

    /// `--force-rocket-hit-at`; always empty in a build without `dev-cli`.
    pub(crate) fn force_rocket_hit_at(&self) -> &[String] {
        #[cfg(feature = "dev-cli")]
        return &self.wreck.force_rocket_hit_at;
        #[cfg(not(feature = "dev-cli"))]
        &[]
    }

    /// `--give`; always `None` in a build without `dev-cli`.
    pub(crate) fn give(&self) -> Option<&str> {
        #[cfg(feature = "dev-cli")]
        return self.race_run.give.as_deref();
        #[cfg(not(feature = "dev-cli"))]
        None
    }
}
