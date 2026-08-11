//! Runs Wipeout Pulse from the user's own disc image.
//!
//! ```sh
//! oag-game data/images/pulse-psp-usa.chd
//! ```
//!
//! Boots into `LogoFMV`, the screen the disc's own boot reaches: it plays
//! `Data\Movies\Intro.PMF` straight through, START or X skips it, and then the
//! Language Selection screen driven
//! by the disc's own front-end XML. Arrow keys move, Return or X selects, which
//! fires `Launch Game` - and `Launch Game` loads a track and a ship and hands the
//! window over to a race, in the same window and on the same GPU device.
//!
//! ```sh
//! # No display needed. Runs the sequence headless and writes one frame.
//! oag-game data/images/pulse-psp-usa.chd --screenshot /tmp/menu.png \
//!     --until "Language Selection" --hold start
//! ```
//!
//! `--race` is the shortcut into the second half: the same race, without booting
//! the front end first.
//!
//! ```sh
//! oag-game --race
//! oag-game --race --screenshot /tmp/race.png --ticks 600 --hold cross
//! ```
//!
//! This file is only the window and the event loop. Everything else is in
//! [`oag_game`], so it can be tested without a GPU.

use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use oag_core::{TickClock, TickRate};

use oag_game::frontend::{self, Frontend};
use oag_game::input;
use oag_game::keys;
use oag_game::render::{Renderer, VideoFormat};
use oag_game::{
    adapter, at3, audio, boot, capture, catalogue, display, loading, menu, movie, perf, prefetch,
    race, report, settings, source, upscale,
};
use oag_gameplay::ControlScheme;
use oag_gameplay::input::button;
use oag_input::Controls;
use oag_physics::SpeedClass;
use oag_render::mesh_render::Anisotropy;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[derive(Parser, Debug)]
#[command(
    name = "oag-game",
    about = "Run Wipeout Pulse from a disc image",
    version
)]
struct Cli {
    /// A disc image, or a directory extracted with `oag-unpack`.
    ///
    /// Either release: the archives are found by name, so
    /// `data/images/pulse-ps2-eu.chd` works as well as the PSP default. See
    /// `oag_assets::Layout`.
    ///
    /// Left out, it is searched for: `data/images/` in the current directory,
    /// then beside the AppImage, then `<data dir>/oag/images`. `oag_game::source`
    /// documents the whole order, and `$OAG_IMAGE` short-circuits it.
    source: Option<String>,

    /// Which movie to play: an archive entry name, or `hash:XXXXXXXX` for one of
    /// the reels whose name is not recovered.
    ///
    /// Defaults to `Data\Movies\Intro.PMF`, which is what the `LogoFMV` screen
    /// plays and the only movie the disc's own boot opens, or to the European
    /// cut of the dev/pub reel under `--reel`.
    #[arg(long)]
    movie: Option<String>,

    /// Boot into `Intro Screen->IntroMovie1` instead of `LogoFMV`.
    ///
    /// The code-side state with the frame-counted holds at 144, 231 and 260,
    /// playing the 260-frame dev/pub reel those counters describe. The disc's
    /// own boot never enters it - see `docs/architecture/frontend-boot.md` - so
    /// this is a way to watch a real, evidenced code path, not the boot order.
    #[arg(long)]
    reel: bool,

    /// Convert only the first this many frames of the movie, rather than all of
    /// them.
    ///
    /// The whole 40-second intro is 33 MiB of cache and about 80 seconds of
    /// `ffmpeg`, once. `--movie-frames 261` is enough for the reel leg.
    #[arg(long)]
    movie_frames: Option<usize>,

    /// Do not convert the movie at all. The sequence still plays, without a
    /// picture.
    #[arg(long)]
    no_video: bool,

    /// Where converted frames are cached.
    #[arg(long)]
    cache: Option<std::path::PathBuf>,

    /// Convert every movie and every sound on the disc up front, on a
    /// background thread, instead of one at a time on first use.
    ///
    /// **Off by default and deliberately so.** Converting lazily is what makes
    /// a cold boot half a second rather than ten minutes; this is the opposite
    /// trade, taken once, so that nothing afterwards ever waits. Measured on
    /// this workspace: 5.5 s for all 93 ATRAC3+ streams and about ten minutes
    /// for the ~13,000 movie frames, of which `Data\Movies\Intro.PMF` alone is
    /// 57 s.
    ///
    /// The game boots and runs normally while it happens, and progress goes to
    /// stdout. Interrupting is safe: both caches are keyed by content, so the
    /// next run picks up whatever finished. See `oag_game::prefetch`.
    #[arg(long)]
    prefetch: bool,

    /// Render one frame to a PNG and exit, without opening a window.
    #[arg(long)]
    screenshot: Option<std::path::PathBuf>,

    /// With `--screenshot`, draw the loading screen at a stated conversion
    /// state instead of the sequence: `DONE/TOTAL`, e.g. `37/115`.
    ///
    /// The same kind of debugging view `--screen` and `--menu-page` are, and it
    /// exists for the same reason plus one of its own: a warm cache passes
    /// through a real mid-conversion state in a fraction of a second and a cold
    /// one takes ten minutes to leave it, so neither is a way to look at this
    /// screen. What is drawn is the window's own layout with a stated input.
    ///
    /// `--ticks` chooses the frame, which is what picks the wave's beat and the
    /// tip on show: the heartbeat peaks at 5 and 11 of its 24 frames.
    #[arg(long, value_name = "DONE/TOTAL")]
    loading_screen: Option<String>,

    /// Write everything the mixer produced to a WAV instead of to a device.
    ///
    /// The audio counterpart of `--screenshot`, and for the same reason: this
    /// project's runs are headless and CI has no sound card, so a file is the
    /// only end-to-end evidence available that a sound was made at all.
    ///
    /// **Forces the null backend.** With a device attached `cpal`'s callback is
    /// already draining the mixer from its own thread, and pulling the same
    /// samples here would race it - so a run that dumps is silent on the
    /// speakers by construction rather than by accident.
    ///
    /// Length is a function of the tick count alone: `--ticks 600` is ten
    /// seconds whatever the machine's frame rate. **Requires `--screenshot`**:
    /// the buffer grows for as long as the run does, and a capture is the only
    /// route with an end.
    #[arg(long, value_name = "FILE")]
    dump_audio: Option<std::path::PathBuf>,

    /// With `--screenshot`, the image's size as `WIDTHxHEIGHT`.
    ///
    /// A window is not always given the size it asks for - a tiling compositor
    /// hands out whatever its layout has - and the field of view is derived from
    /// the viewport, so this is how a capture can show what a differently shaped
    /// window would have drawn.
    #[arg(long, default_value = DEFAULT_SIZE)]
    size: String,

    /// With `--screenshot`, run until this state is current before capturing.
    #[arg(long)]
    until: Option<String>,

    /// With `--screenshot`, draw one named screen out of the front-end XML and
    /// stop, instead of running the sequence.
    ///
    /// A debugging view of a screen the boot order does not reach yet, e.g.
    /// `--screen "Show Logo"`. Names are the XML's own, and a `Parent->Child`
    /// path works too.
    #[arg(long)]
    screen: Option<String>,

    /// With `--screenshot`, capture the frame the way a window presents it:
    /// through the render scale, the upscaler, the brightness/gamma grade and
    /// the aspect bars.
    ///
    /// Off by default, because an ungraded capture at exactly `--size` is the
    /// right thing for a bug report and a graded one would make every capture
    /// disagree with every other. Turn it on to see what a player sees - which
    /// is the only way to see `[graphics] upscaler` do anything at all, since
    /// the ordinary capture never reaches the blit.
    #[arg(long)]
    presented: bool,

    /// With `--screenshot`, run this many ticks before capturing.
    ///
    /// A capture that reaches `Launch Game` spends what is left of them on the
    /// race the front end hands off to.
    #[arg(long, default_value_t = 0)]
    ticks: u32,

    /// With `--screenshot`, hold these buttons on every tick.
    ///
    /// Comma-separated abstract button names: `start`, `cross`, `circle`, `up`,
    /// `down`, `activate`, `cancel`.
    #[arg(long)]
    hold: Option<String>,

    /// With `--screenshot`, press and release these buttons on alternating
    /// ticks.
    ///
    /// `--press start,cross` is how the capture reaches `Launch Game`: START
    /// skips the intro, then cross picks a language. A held button only ever
    /// produces one rising edge, so two presses need two edges.
    ///
    /// It reaches the race too, not only the front end, which is what makes an
    /// edge-triggered manoeuvre testable headlessly: `--race --hold cross
    /// --press l` double-taps the left airbrake and sideshifts, repeatedly - a
    /// tap every other tick is well inside the `0.25 s` window. Expect the ship
    /// to be slow as well as displaced, because `q` is one key for both the
    /// airbrake *axis* and the sideshift *button*, so pulsing it also pulses the
    /// aerodynamic brake. That is a real pilot's gesture, not an artefact.
    #[arg(long)]
    press: Option<String>,

    /// Control scheme: `veteran` or `novice`.
    ///
    /// Overrides `[controls] scheme` for this run without writing it back. The
    /// two differ only in how a sideshift is asked for - veteran double-taps an
    /// airbrake, novice holds the sideshift button and flicks the stick - and
    /// they are the original's own `Control_Type` values. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    #[arg(long)]
    scheme: Option<ControlScheme>,

    /// Draw a frame counter over the intro.
    ///
    /// On by default when there is no picture, since a black screen for eight
    /// seconds is otherwise indistinguishable from a hang.
    #[arg(long)]
    overlay: bool,

    /// Load the menu tree from this file instead of the one built into the
    /// binary.
    ///
    /// For editing `assets/ui/menu.toml` without a rebuild. Checked the same
    /// way the built-in one is, so a mistake in it is a startup error.
    #[arg(long, value_name = "FILE")]
    menu: Option<std::path::PathBuf>,

    /// With `--screenshot`, draw one page of our own menus instead of the
    /// sequence: a page id from `assets/ui/menu.toml`.
    ///
    /// For looking at a layout without launching the game and walking to it.
    /// Like `--screen`, it takes no input and runs no state machine.
    #[arg(long, value_name = "PAGE")]
    menu_page: Option<String>,

    /// With `--menu-page`, draw that page part-way through arriving.
    ///
    /// `0` is the instant a page change starts and `1` is the end of it. A
    /// still cannot otherwise show a transition at all: `--menu-page` runs no
    /// clock, and `--ticks` does nothing alongside it, so without this the only
    /// way to look at the effect is to play the game and watch.
    ///
    /// Shows the *arriving* half only. The page being left is whatever the
    /// player came from, which a one-page capture has no way to know.
    #[arg(long, value_name = "0..1")]
    menu_anim_phase: Option<f32>,

    /// Show the language picker even when a language is already chosen.
    ///
    /// Without this the picker is skipped once `settings.toml` names a
    /// language, which is what a player wants and what makes the second run
    /// shorter than the first.
    #[arg(long)]
    pick_language: bool,

    /// Print every state transition as it happens, exits included.
    #[arg(long)]
    trace: bool,

    /// Report what was loaded and exit, without rendering anything.
    #[arg(long)]
    dry_run: bool,

    /// Skip the front end and go straight to a ship on a track.
    ///
    /// Arrow keys steer, X or Return thrusts, Q and E are the airbrakes. The
    /// same race the front end's `Launch Game` starts.
    #[arg(long)]
    race: bool,

    /// The track's `.vex` entry name, for either way into a race. The same name
    /// on both releases.
    #[arg(long, default_value = race::DEFAULT_TRACK)]
    track: String,

    /// The team, which selects both the handling stats and the model.
    ///
    /// A team **id**, which is the folder under `Data\Ships\` and not always
    /// the name on screen - the Mirage pack's team is `Mantis`. Teams a mounted
    /// pack adds are accepted here like any other; see `--dlc`.
    #[arg(long, default_value = race::DEFAULT_TEAM)]
    team: String,

    /// A directory holding downloadable content: a pack's `.edat` files, or the
    /// `.zip` they were downloaded as. Repeatable.
    ///
    /// Left out, it is searched for: `data/dlc/` in the current directory, then
    /// beside the AppImage, then `<data dir>/oag/dlc`. `$OAG_DLC` short-circuits
    /// that, and `oag_game::source::resolve_dlc` documents the order.
    ///
    /// **Any pack works against any release.** The original locked a pack to
    /// its own territory's disc; this does not. See `docs/formats/dlc-pack.md`.
    #[arg(long)]
    dlc: Vec<String>,

    /// The speed class: venom, flash, rapier or phantom.
    #[arg(long, default_value = "venom")]
    class: String,

    /// The race mode: time_trial, speed_lap, zone or single_race.
    ///
    /// Needed on the `--race` path in particular, which skips the menus and so
    /// has no other way to pick one. `single_race` is the only one that races
    /// with weapons, so it is the only one where a `Weapon Pad` draws or hands
    /// anything out.
    #[arg(long, default_value = "time_trial")]
    mode: String,

    /// Development view: draw the driveable ribbon instead of the track's art
    /// meshes.
    ///
    /// The ribbon is the geometry the simulation actually spawns on and queries,
    /// so ship-plus-ribbon shows directly whether the ship is where the physics
    /// thinks it is - useful for a physics comparison and misleading about
    /// everything else. A race draws the map by default.
    #[arg(long)]
    ribbon: bool,

    /// Overlay the collision soup - the geometry the physics world is actually
    /// made of, the same view `oag-view --collision` draws - on top of the
    /// track model chosen above.
    #[arg(long)]
    collision: bool,

    /// Spawn the rest of the grid even though `--mode` races solo in the
    /// original.
    ///
    /// A verification aid - see `race::Options::opponents` - for looking at
    /// the measured grid layout without a mode that actually fields one.
    /// `time_trial`, `speed_lap` and `zone` all race with `AI DIFFICULTY`
    /// greyed to `N/A` on the real Custom Race screen, and this flag exists
    /// despite that rather than because of it.
    #[arg(long)]
    opponents: bool,

    /// Seed the world generator, instead of `race::SEED`.
    ///
    /// A verification aid - see `race::Options::seed`. The one thing in a race
    /// that draws from the generator today is which pickup a weapon pad hands
    /// over, so this is how a capture is made to show a chosen weapon.
    #[arg(long)]
    seed: Option<u64>,

    /// In a race, print a telemetry line every this many ticks. Zero prints
    /// none.
    #[arg(long, default_value_t = 60)]
    log_every: u32,

    /// Anisotropic filtering level for track and ship textures: off, 2x, 4x,
    /// 8x or 16x.
    ///
    /// Overrides `[graphics] anisotropy` in the settings file
    /// (`settings::path`) for this run only; the file on disk is not changed.
    #[arg(long)]
    anisotropy: Option<Anisotropy>,

    /// What an authored `LodGroup` draws: `both` children, the way the original
    /// does, or only the higher-detail `single` one.
    ///
    /// Overrides `[graphics] lod` in the settings file (`settings::path`) for
    /// this run only; the file on disk is not changed. Here for the same reason
    /// `--anisotropy` is: `both` means two differently-tessellated copies of the
    /// same surface occupy the same space, and two captures differing only by
    /// this flag are how you see what that costs.
    #[arg(long)]
    lod: Option<oag_render::mesh::Lod>,

    /// Which resampler carries the frame onto the surface: bilinear or fsr1.
    ///
    /// Overrides `[graphics] upscaler` in the settings file (`settings::path`)
    /// for this run only; the file on disk is not changed. Here for the same
    /// reason `--anisotropy` is, and for one more: two `--presented` captures
    /// differing only by this flag are how the resamplers get compared, and
    /// asking somebody to edit a settings file between them is how a comparison
    /// ends up differing by something else as well.
    #[arg(long)]
    upscaler: Option<crate::display::Upscaler>,

    /// What percentage of the displayed size the game is rendered at, 25 to
    /// 200.
    ///
    /// Overrides `[graphics] render_scale` for this run only. The companion to
    /// `--upscaler`: a resampler can only be judged at a scale where it has
    /// something to resample, and the two flags together are what let one
    /// command produce one image of a comparison.
    #[arg(long)]
    render_scale: Option<u32>,

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
    pose: Option<String>,

    /// Start the craft at a captured trace row's exact pose: position and full
    /// basis from the CSV, nothing recomputed from the spline.
    ///
    /// The other half of the comparison `--pose` was built for: an emulator
    /// frame and one of ours, from the same state. When the capture carries
    /// camera columns (`psp-trace.py --camera`), the frame is also rendered
    /// from the recorded camera pose; without them the chase camera frames the
    /// shot as usual.
    #[arg(long, value_name = "TRACE.CSV", conflicts_with = "pose")]
    pose_from: Option<std::path::PathBuf>,

    /// Which tick of `--pose-from` to take the pose off.
    #[arg(long, default_value_t = 0, requires = "pose_from")]
    pose_tick: u64,

    /// With `--pose-from`: ignore the capture's camera columns and keep the
    /// chase camera.
    #[arg(long, requires = "pose_from")]
    no_camera: bool,

    /// With `--pose-from`: render the recorded camera at this fov instead of
    /// the disc's authored value, in **vertical degrees** - the authored
    /// value's own unit, settled at confidence 94 on 2026-08-09.
    ///
    /// It was the calibration knob for settling that unit. It is now a
    /// *measurement instrument* rather than a leftover: `place_at` resets the
    /// body, so a posed craft has zero velocity and never receives the
    /// original's speed-dependent widen. **Every matched-pose comparison needs
    /// this flag**, set to `60 + 0.075 * dot(fwd, vel)` for the captured tick.
    /// See `docs/rendering/projection-vs-the-original.md`.
    #[arg(long, requires = "pose_from")]
    camera_fov: Option<f32>,

    /// With `--screenshot`: force the exhaust into the state it would hold this
    /// many seconds after entering a speed pad, at saturated intensity.
    ///
    /// The frame-comparison knob for the boost visuals: a posed capture
    /// (`--pose-from --ticks 0`) never crosses a pad, so without this the flare
    /// renders cold and the plume not at all, and there is nothing to compare
    /// against an emulator shot taken mid-boost. `0.0` is the entry tick.
    #[arg(long, value_name = "SECONDS")]
    pose_boost: Option<f32>,

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
    pose_intensity: Option<f32>,

    /// With `--pose-boost`: the speed in units/s to advance the exhaust at,
    /// instead of the default racing `120`.
    ///
    /// Only reaches the picture through `Exhaust_Update`'s speed ramp, which
    /// floors the boost accumulator at `clamp((kmh - 100) / 500) * 0.6` - so it
    /// matters when a capture's speed is far from `120`. Take it from the
    /// capture's `speed` column.
    #[arg(long, value_name = "UNITS_PER_S", requires = "pose_boost")]
    pose_speed: Option<f32>,

    /// With `--race --screenshot`: drive the run from a committed `.inputs`
    /// script (see `scripts/input_script.py` for the format) instead of
    /// `--hold`/`--press` - the same file `scripts/psp-trace.py --script`
    /// feeds the emulator, so one authored input produces both sides of a
    /// visual comparison. Ticks past the script's end hold its last state.
    #[arg(long, value_name = "FILE.inputs", requires = "race")]
    input_script: Option<std::path::PathBuf>,

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
    camera_view: Option<display::CameraView>,

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
    /// Combining it with `--screenshot` is refused rather than ignored: the
    /// screenshot route renders through `race::capture`, and threading a writer
    /// into that loop is a change to a file this flag deliberately does not
    /// touch. Run the two separately.
    #[arg(long, value_name = "FILE.csv", conflicts_with = "screenshot")]
    trace_out: Option<std::path::PathBuf>,
}

/// Parses `--pose`: three or four comma-separated numbers, the fourth a yaw in
/// degrees.
fn parse_pose(text: &str) -> Result<(oag_core::math::Vec3, f32)> {
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    ensure!(
        matches!(parts.len(), 3 | 4),
        "--pose takes x,y,z or x,y,z,yaw, not {} value(s)",
        parts.len()
    );
    let mut values = [0.0f32; 4];
    for (slot, text) in values.iter_mut().zip(&parts) {
        *slot = text
            .parse()
            .with_context(|| format!("--pose component {text:?} is not a number"))?;
    }
    Ok((
        oag_core::math::Vec3::new(values[0], values[1], values[2]),
        values[3].to_radians(),
    ))
}

/// Resolves `--pose-from`: one row of a capture into an exact ship pose, plus
/// the recorded camera when the row carries one.
///
/// The ship's basis goes through [`oag_trace::replay::orientation_of`] under
/// the measured reading - the capture's `right_*` columns are the ship's left,
/// and that reconciliation must happen in the one crate that owns it rather
/// than be restated here. The camera goes through
/// [`oag_trace::replay::camera_orientation_of`], which has its own, weaker
/// contract; see its docs.
fn pose_from_trace(
    path: &std::path::Path,
    tick: u64,
    no_camera: bool,
    camera_fov: Option<f32>,
) -> Result<(race::PoseRequest, Option<race::CameraOverride>)> {
    use oag_trace::replay::{self, Basis};

    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {} for --pose-from", path.display()))?;
    let trace = oag_trace::Trace::parse(&text)
        .with_context(|| format!("{} is not a trace", path.display()))?;
    let frame = trace
        .frames
        .iter()
        .find(|frame| frame.tick == tick)
        .with_context(|| {
            format!(
                "{} has no tick {tick}; it covers {:?}..={:?}",
                path.display(),
                trace.frames.first().map(|f| f.tick),
                trace.frames.last().map(|f| f.tick)
            )
        })?;

    let pose = race::PoseRequest::Exact(oag_gameplay::spawn::Pose {
        position: frame.position,
        orientation: replay::orientation_of(frame, Basis::LeftUpForward),
    });
    let camera = match (no_camera, replay::camera_orientation_of(frame)) {
        (true, _) | (false, None) => {
            if !no_camera {
                eprintln!(
                    "{}: no camera columns (captured without --camera); using the chase camera",
                    path.display()
                );
            }
            None
        }
        (false, Some(orientation)) => Some(race::CameraOverride {
            // `camera_orientation_of` answered, so the pose group is present
            // and the eye is too: the columns are all-or-nothing at parse.
            eye: frame
                .camera_position
                .context("a parsed camera pose has an eye")?,
            orientation,
            fov_deg: camera_fov,
        }),
    };
    Ok((pose, camera))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Loaded (and, on first run or a missing key, written back complete) before
    // anything else: a bad value in the file should fail immediately, not eight
    // seconds of intro later.
    let settings = settings::load()?;
    let anisotropy = cli.anisotropy.unwrap_or(settings.graphics.anisotropy);
    let render_scale = match cli.render_scale {
        Some(percent) => crate::display::Scale::try_from(percent)
            .map_err(|why| anyhow::anyhow!("--render-scale {percent}: {why}"))?,
        None => settings.graphics.render_scale,
    };
    let settings = settings::Settings {
        graphics: settings::Graphics {
            upscaler: cli.upscaler.unwrap_or(settings.graphics.upscaler),
            render_scale,
            ..settings.graphics
        },
        ..settings
    };

    // Parsed before anything is loaded, and for both ways in: the front end can
    // hand off to a race, so a misspelled class must not be discovered eight
    // seconds of intro later.
    let class = SpeedClass::from_name(&cli.class).with_context(|| {
        format!(
            "{:?} is not a speed class; try venom, flash, rapier or phantom",
            cli.class
        )
    })?;
    // Parsed here for the same reason as the speed class: `--race` goes straight
    // to a track, so a misspelled mode has to be a message about the command
    // line rather than a race that quietly runs under different rules.
    let mode = oag_race::Mode::from_name(&cli.mode).with_context(|| {
        format!(
            "{:?} is not a race mode; try time_trial, speed_lap, zone or single_race",
            cli.mode
        )
    })?;
    // Refused rather than tolerated, and the reason is memory rather than
    // tidiness: a dump accumulates every sample it renders, so a windowed run
    // that a player leaves open grows the buffer for as long as they play - a
    // quarter of a gigabyte in ten minutes, at 48 kHz stereo `f32`. A capture is
    // bounded by `--ticks`, which is the only route that ends.
    ensure!(
        cli.dump_audio.is_none() || cli.screenshot.is_some(),
        "--dump-audio needs --screenshot: the dump is as long as the run, and only \
         a capture has an end. Add --screenshot FILE --ticks N."
    );
    // Refused for the same reason `--screen` and `--menu-page` are capture-only:
    // there is no window route that draws a *stated* conversion state, and
    // silently ignoring the flag would look like a screen that does not work.
    ensure!(
        cli.loading_screen.is_none() || cli.screenshot.is_some(),
        "--loading-screen needs --screenshot: it draws one frame of the loading \
         screen at a stated state. A window shows the real one under --prefetch."
    );

    // Resolved once, before anything opens it: both ways in need a source, and
    // "no disc image found" is a message about the command line, not something to
    // discover eight seconds of intro later.
    let source = source::resolve(cli.source.as_deref(), settings.source.image.as_deref())?;
    // Resolved beside it, but it cannot fail: no DLC is the ordinary state of a
    // copy of the game.
    let dlc = source::resolve_dlc(&cli.dlc, &settings.source.dlc);

    // Opened before either way in, because both want sound and neither owns the
    // other. **Only the device, not the music itself** - `start_music` is
    // deferred to whichever tick loop is actually about to run, windowed or
    // headless, so that a player never hears the front end before the window
    // that shows it exists. See the two call sites below and
    // `App::open`.
    let audio = audio::Audio::open(&settings.audio, cli.dump_audio.clone());
    // Surveyed once, here, because it is what decides whether the AUDIO page
    // offers MUSIC SOURCE at all - and answering it means opening every disc
    // image on the search path, which is not something to do while a menu is on
    // screen. Skipped under `--dry-run` for the same reason the music is.
    let music_discs = if cli.dry_run {
        audio::MusicDiscs::default()
    } else {
        let discs = audio::MusicDiscs::survey(&source);
        println!("audio: music discs, {}", discs.describe());
        discs
    };

    let (pose, camera) = match &cli.pose_from {
        Some(path) => {
            let (pose, camera) =
                pose_from_trace(path, cli.pose_tick, cli.no_camera, cli.camera_fov)?;
            (Some(pose), camera)
        }
        None => (
            cli.pose
                .as_deref()
                .map(parse_pose)
                .transpose()?
                .map(|(position, yaw)| race::PoseRequest::SplineAligned { position, yaw }),
            None,
        ),
    };

    let race_options = race::Options {
        source: source.clone(),
        dlc: dlc.clone(),
        track: cli.track.clone(),
        team: cli.team.clone(),
        class,
        mode,
        ribbon: cli.ribbon,
        collision: cli.collision,
        lod: cli.lod.unwrap_or(settings.graphics.lod),
        opponents: cli.opponents,
        seed: cli.seed,
        pose,
        camera,
    };

    // Before `boot::load`, deliberately: the front end's load parses the front-end
    // XML, every language plugin and a string table, and may shell out to `ffmpeg`
    // to transcode the intro. Going straight to a race needs none of it.
    if cli.race {
        // `--prefetch` is a front-end thing, and saying so is better than
        // quietly doing nothing. `--race` exists to skip the boot sequence, and
        // hanging its exit on ten minutes of `ffmpeg` would invert the one
        // thing it is for.
        if cli.prefetch {
            println!(
                "--prefetch has no effect with --race: it converts the front end's movies and \
                 sounds, which a race never opens. Run it without --race once."
            );
        }
        // Same reasoning one screen along: the loading screen is what a player
        // waits on while that conversion runs, so on a route that does not run
        // it there is nothing for the screen to be about.
        if cli.loading_screen.is_some() {
            println!("--loading-screen has no effect with --race; run it without --race.");
        }
        return run_race(
            &cli,
            race_options,
            &settings,
            anisotropy,
            audio,
            music_discs,
        );
    }

    let leg = if cli.reel {
        frontend::Leg::DevPubReel
    } else {
        frontend::Leg::LogoFmv
    };
    let options = boot::Options {
        source: source.clone(),
        dlc: dlc.clone(),
        // The string table follows the saved language, so a player who picked
        // French once reads French from the next boot rather than only having
        // the picker skipped.
        language: settings.language.clone(),
        leg,
        // `None` when `--movie` was not given: which entry that is depends on
        // the source's own title, not known here yet, so `boot::load` resolves
        // it once the source is open. See `boot::Options::movie`.
        movie: cli.movie.clone(),
        cache: cli.cache.clone().unwrap_or_else(boot::default_cache_dir),
        audio_cache: boot::default_audio_cache_dir(),
        // Every frame by default: the movie the disc plays is 1200 frames long
        // and its last one is the Wipeout Pulse logo, so a cap would stop the
        // sequence before the thing it exists to show.
        extent: cli
            .movie_frames
            .map_or(movie::Extent::Whole, movie::Extent::Frames),
        no_video: cli.no_video,
    };

    // Every leg with no window loads the whole boot here and now, blocking, and
    // then leaves. Only a window has anywhere to *show* a wait, so only a
    // window earns the machinery below that defers one.
    if cli.dry_run || cli.screenshot.is_some() {
        return run_windowless(
            cli,
            &options,
            &settings,
            race_options,
            anisotropy,
            audio,
            music_discs,
        );
    }

    // **The cheap half of the boot, and the only part a window waits on.**
    // Measured on the EU disc: 0.05 s here against 4.9 s for the movies, which
    // `MediaWorker` now runs on a thread of its own while winit opens the
    // window and the loading screen goes up over it. Before this split the
    // whole 5 seconds ran before winit had been asked for a window at all, so
    // there was nothing on screen to say the game had started - which is the
    // bug this shape exists to fix. See `boot::Shell`.
    let (mut boot_shell, archives) = boot::load_shell(&options)?;
    // Drained rather than iterated: `boot::assemble` appends its own lines to
    // this same list, and the hand-off prints what it finds there. Leaving
    // these in would print the whole first half twice, seconds apart, which
    // reads as the disc having been opened again.
    for line in boot_shell.report.drain(..) {
        println!("{line}");
    }
    let media = boot::MediaWorker::spawn(archives, &boot_shell, &options);

    // Parsed here rather than when the menus open, so a broken definition is a
    // startup error and not something a player meets after the intro.
    let definition = match cli.menu.as_deref() {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            menu::Definition::parse(&text).with_context(|| format!("parsing {}", path.display()))?
        }
        None => menu::Definition::parse(menu::BUILT_IN)
            .context("parsing the built-in menu definition")?,
    };
    // The three lists that come off the disc rather than out of the definition:
    // what is raceable, who can be raced for, and what languages exist. All
    // carry a label the player reads and a value the settings file stores, and
    // on a circuit or a team those are different strings - `16_Track` and
    // `Mantis` against their localised names, which are shipped content and
    // only ever live in memory.
    let shell = Shell {
        definition,
        modes: menu::mode_choices(&boot_shell.strings),
        teams: boot_shell
            .teams
            .iter()
            .map(|team| menu::Choice::labelled(&team.id, boot_shell.strings.get_or_id(&team.id)))
            .collect(),
        tracks: boot_shell
            .tracks
            .iter()
            .map(|track| {
                (
                    track.clone(),
                    boot_shell.strings.get_or_id(&track.id).to_string(),
                )
            })
            .collect(),
        languages: boot_shell
            .languages
            .iter()
            .map(|language| menu::Choice::labelled(&language.name, &language.native_name))
            .collect(),
        font: boot_shell.font.clone(),
        menu_skin: boot_shell.menu_skin,
        menu_font: boot_shell.menu_font.clone(),
        sprites: boot_shell.sprites.clone(),
    };

    println!("\n{MENU_KEYS}");

    // Unconditional now, where it used to be `--prefetch` only. Its two extra
    // archive reads used to buy nothing on a boot that went straight to the
    // front end; every windowed boot now shows the loading screen while the
    // movies decode, so every windowed boot needs the tips and the glow strip.
    let loading_assets = {
        let assets = loading::Assets::load(&source, &boot_shell.strings);
        for note in &assets.notes {
            println!("{note}");
        }
        assets
    };

    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the intro is animated whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        boot_shell: Some(boot_shell),
        media: Some(media),
        boot_overlay: cli.overlay,
        pick_language: cli.pick_language,
        race: None,
        race_options,
        trace: cli.trace,
        log_every: cli.log_every,
        anisotropy,
        scheme: resolve_scheme(&cli, &settings),
        settings,
        shell: Some(shell),
        audio: Some(audio),
        music_discs,
        // Asked for, not started: see `App::prefetch`.
        prefetch: cli.prefetch.then(|| prefetch::Options {
            source: source.clone(),
            movies: options.cache.clone(),
            audio: boot::default_audio_cache_dir(),
        }),
        loading_assets,
        state: None,
    };
    event_loop.run_app(&mut app)?;
    app.finish_audio()?;
    app.finish_prefetch();
    Ok(())
}

/// Every leg that never opens a window: `--dry-run` and the three captures.
///
/// **Split off from `main` because the boot is loaded differently here.** A
/// window defers the movies onto a worker and covers the wait with the loading
/// screen (`boot::MediaWorker`); none of these legs has anywhere to show that,
/// and a capture must not race a worker for the frames it is about to draw, so
/// every one of them takes the whole boot in one blocking `boot::load` exactly
/// as the game always did. The body below is that former part of `main`,
/// unchanged.
fn run_windowless(
    cli: Cli,
    options: &boot::Options,
    settings: &settings::Settings,
    race_options: race::Options,
    anisotropy: Anisotropy,
    mut audio: audio::Audio,
    music_discs: audio::MusicDiscs,
) -> Result<()> {
    let mut loaded = boot::load(options)?;
    if cli.overlay {
        loaded.frontend.set_overlay(true);
    }
    // A language chosen on an earlier run skips the picker. Reported either
    // way: silently not asking is indistinguishable from a broken picker, and
    // silently asking again is indistinguishable from a setting that did not
    // save.
    match (cli.pick_language, settings.language.as_deref()) {
        (false, Some(name)) if loaded.frontend.preselect_language(name) => {
            println!("language {name} from settings, skipping the picker");
        }
        (false, Some(name)) => {
            eprintln!("this source does not offer {name:?}, so the picker is shown");
        }
        _ => {}
    }
    for line in &loaded.report {
        println!("{line}");
    }

    if cli.dry_run {
        return Ok(());
    }

    // **After `boot::load`, and that is the whole of why it is here.** The boot
    // sequence transcodes the intro and the backdrop itself, lazily, and the
    // prefetch worker would convert those same two movies - two `ffmpeg`
    // processes writing one cache file, which is a corrupt file rather than a
    // race that resolves. Started once boot has finished, they are already
    // cached and the worker's planning pass skips them by name.
    let mut prefetch = cli.prefetch.then(|| {
        prefetch::Prefetch::spawn(prefetch::Options {
            source: options.source.clone(),
            movies: options.cache.clone(),
            audio: boot::default_audio_cache_dir(),
        })
    });

    let scheme = resolve_scheme(&cli, settings);

    // A source with no movie at all - which is every PS2 source, whose intro is
    // an MPEG-2 program stream outside the archives - has no video format
    // either, and the front end draws without one. A source with *some* movie
    // gets a pipeline sized for it even when the first boot leg is not the one
    // that plays it; see `Boot::video_format`.
    let video_format = loaded.video_format();

    // Before the sequence's own capture, because it is a different picture
    // rather than a variation on that one: it runs no state machine, opens no
    // movie and reaches the GPU through `capture::loading`.
    if let (Some(path), Some(spec)) = (&cli.screenshot, &cli.loading_screen) {
        let progress = parse_progress(spec)?;
        let assets = loading::Assets::load(&options.source, &loaded.strings);
        for note in &assets.notes {
            println!("{note}");
        }
        capture::loading(
            &assets,
            loaded.font.clone(),
            &loaded.sprites,
            &capture::LoadingOptions {
                path: path.clone(),
                size: parse_size(&cli.size)?,
                ticks: cli.ticks,
                progress,
                renderer: settings.graphics.renderer.clone(),
                aspect: settings.display.aspect,
            },
        )?;
        if let Some(prefetch) = &mut prefetch {
            prefetch.join();
        }
        return Ok(());
    }

    if let Some(path) = cli.screenshot {
        // No `start_music` here: this leg runs the boot sequence, so the music
        // waits behind the intro exactly as the window's does, and `capture::run`
        // starts it from its own tick loop. See `Audio::start_music`.
        capture::run(
            loaded,
            video_format,
            &capture::Options {
                path,
                until: cli.until.clone(),
                ticks: cli.ticks,
                held: button_mask(cli.hold.as_deref()),
                pressed: button_mask(cli.press.as_deref()),
                scheme,
                trace: cli.trace,
                race: Some(race_options),
                log_every: cli.log_every,
                size: parse_size(&cli.size)?,
                screen: cli.screen.clone(),
                menu_page: cli.menu_page.clone(),
                menu_anim_phase: cli.menu_anim_phase,
                presented: cli.presented,
                // The clone is overridden rather than `settings` itself, so
                // `--camera-view` reaches the race this capture may hand over to
                // (`capture::run` builds its `CaptureOptions` from this block)
                // without ever being written back to the settings file. The same
                // one flag then covers both screenshot paths - this one and
                // `--race --screenshot` - and neither persists it.
                settings: settings::Settings {
                    graphics: settings::Graphics {
                        camera_view: cli.camera_view.unwrap_or(settings.graphics.camera_view),
                        ..settings.graphics.clone()
                    },
                    ..settings.clone()
                },
                music_discs: music_discs.clone(),
                anisotropy,
            },
            &mut audio,
        )?;
        // After both legs, and only here: the capture above may have handed off
        // to a race that went on filling the same buffer, so writing inside
        // either one would truncate the WAV to whichever leg wrote it.
        audio.finish()?;
        // A capture is over in seconds and the conversion is not, so this is
        // where `--prefetch` actually waits. Joined rather than dropped,
        // because a headless run is how the whole cache gets filled in the
        // first place.
        if let Some(prefetch) = &mut prefetch {
            prefetch.join();
        }
        return Ok(());
    }
    unreachable!("every branch above returns")
}

/// Turns a comma-separated list of abstract button names into a mask.
///
/// Unknown names are skipped rather than fatal: `none` is a real value in the
/// game's own XML and means no button.
/// The control scheme this run uses: `--scheme` over `[controls] scheme`.
///
/// An unrecognised token in the settings file is **reported and ignored**
/// rather than fatal, matching how `race.mode` and `race.class` behave: a
/// profile written by a build that had a scheme this one does not should still
/// boot. The flag cannot be unrecognised - clap rejects it at parse time
/// through `ControlScheme`'s `FromStr`, which is why its error message names
/// the valid values.
fn resolve_scheme(cli: &Cli, settings: &settings::Settings) -> ControlScheme {
    if let Some(scheme) = cli.scheme {
        return scheme;
    }
    let token = &settings.controls.scheme;
    ControlScheme::from_name(token).unwrap_or_else(|| {
        let fallback = ControlScheme::default();
        eprintln!("ignoring [controls] scheme = {token:?}; using {fallback}");
        fallback
    })
}

fn button_mask(names: Option<&str>) -> u32 {
    names.map_or(0, |list| {
        list.split(',')
            .filter_map(|name| input::button_from_name(name.trim()))
            .fold(0u32, |mask, index| mask | (1u32 << index))
    })
}

/// Parses `--loading-screen`'s `DONE/TOTAL`.
///
/// Only the two figures the bar is drawn from, because everything else on the
/// screen follows from them: `planning` is over by definition once there is a
/// total, and `finished` is `done == total`, which is also the state the fade
/// runs in. `cached` and `failed` are left at zero rather than invented - they
/// are counts of things that really happened, and a flag that made them up
/// would be drawing a run nobody had.
fn parse_progress(spec: &str) -> Result<prefetch::Progress> {
    let bad = || {
        anyhow::anyhow!("{spec:?} is not a conversion state; write it as DONE/TOTAL, e.g. 37/115")
    };
    let (done, total) = spec.split_once('/').ok_or_else(bad)?;
    let done: usize = done.trim().parse().map_err(|_| bad())?;
    let total: usize = total.trim().parse().map_err(|_| bad())?;
    ensure!(
        done <= total,
        "{done} converted of {total} is more than all of them"
    );
    Ok(prefetch::Progress {
        planning: false,
        total,
        done,
        cached: 0,
        failed: 0,
        // A real label, built the way `prefetch` builds one, so the line reads
        // as the thing it will read as in a window rather than as filler.
        current: (done < total).then(|| format!("Data.wad {}", oag_pulse::names::INTRO_MOVIE)),
        finished: done == total,
    })
}

/// A capture's default size, front end and race alike.
///
/// Three times the PSP's screen, so the 5x7 glyphs stay legible and a screenshot
/// frames what the window would have shown at its own default. The *window's*
/// size is `graphics.window_size` and is a setting - see
/// [`display::Size::default`], which is this same shape.
const DEFAULT_SIZE: &str = "1440x816";

/// Parses `WIDTHxHEIGHT`.
fn parse_size(text: &str) -> Result<(u32, u32)> {
    let bad = || anyhow::anyhow!("{text:?} is not a size; write it as WIDTHxHEIGHT, e.g. 1440x816");
    let (width, height) = text.split_once(['x', 'X']).ok_or_else(bad)?;
    let width: u32 = width.trim().parse().map_err(|_| bad())?;
    let height: u32 = height.trim().parse().map_err(|_| bad())?;
    if width == 0 || height == 0 {
        return Err(bad());
    }
    Ok((width, height))
}

/// The window's title while the front end is on screen.
const TITLE: &str = "OpenAntiGrav";

/// Printed before the front end opens: the picker's own keys.
///
/// Escape quits here and only here, because the boot sequence is the first
/// thing on screen and has nothing behind it to go back to.
const MENU_KEYS: &str = "arrow keys or the left stick move, return, X or cross \
     selects, space or start skips, escape quits";

/// And once the menus have the window.
const SHELL_TITLE: &str = "OpenAntiGrav - menu";

/// Printed when the menus open, which have one key the picker does not.
const SHELL_KEYS: &str = "up and down move, left and right change a setting, \
     return, X or cross selects, backspace, circle or escape goes back";

/// And once a race has taken it over.
const RACE_TITLE: &str = "OpenAntiGrav - race";

/// Printed whenever a race takes the window, by either route - and the two
/// routes differ in exactly one key, which is why what escape does is spelled
/// separately rather than assumed.
const RACE_KEYS: &str = "arrow keys or the left stick steer, X, return or R2 thrusts, \
     Q and E or the shoulders are the airbrakes, L2 is both, C fires a pickup and Z absorbs it";

/// What escape does from a race the menus started, and from one `--race` did.
///
/// Escape is "back one level" everywhere; the difference is only that `--race`
/// has no level behind it. See [`Session::escape`].
const ESC_TO_MENU: &str = ", escape returns to the menus";
const ESC_QUITS: &str = ", escape quits";

/// Loads a track and a ship and either captures one frame or opens a window.
///
/// **The settings apply here too**, even though this route never opens a menu
/// to change them with. It used to take only the aspect, which left a window
/// opened with `--race` ignoring the window size, the render scale and the
/// performance overlay that the same file was setting for every other route -
/// and the overlay is most wanted exactly here, where a track is on screen.
/// Nothing on this path writes the file back.
/// Our per-tick state, in the capture's own columns, written to `path`.
///
/// # Which of our values stands for which recovered field
///
/// This mapping is a **claim**, not a convenience: differencing two columns that
/// do not mean the same thing produces a number that looks like a measurement
/// and is not. Each row is one field of the original's flare object, at the
/// offset `scripts/psp_trace_fields.py`'s `FLARE_FIELDS` reads it from.
///
/// | Column | Flare offset | Ours |
/// | --- | --- | --- |
/// | `boost_timer` | `+0xb8` | [`Exhaust::boost_timer`] |
/// | `plume_timer` | `+0x88` | [`Exhaust::plume_timer`] |
/// | `intensity` | `+0xbc` | [`Exhaust::intensity`] |
/// | `half_size` | `+0xc4` | [`Exhaust::half_size`] |
/// | `engine_on` | `+0x94` | [`Exhaust::engine_on`] |
/// | `flare_speed_kmh` | `+0x8c` | [`Exhaust::speed_kmh`] |
/// | `speed_ramp` | `+0x90` | [`Exhaust::speed_ramp`] |
/// | `boost_accum` | `+0x60` | [`Exhaust::boost_accumulator`] |
///
/// Two of them are worth stating outright, because the obvious accessor is the
/// wrong one:
///
/// - **`plume_timer` is the reveal timer, not the boost timer.** It is *not*
///   [`Exhaust::plume_visible`], which is the bit the reveal sets; the column is
///   the seconds-since-reveal that decides when that bit clears again.
/// - **`engine_on` is written as `0` or `1` here, and the capture's column is
///   neither.** The original's `+0x94` is an integer that `psp-trace.py` reads
///   through `struct.unpack("<f", ..)`, so a set value arrives as the denormal
///   `3.601337e-43`. `oag_trace::Flare::engine_on_is_set` is what both sides are
///   compared through, so writing a plain `1.0` is correct and comparable - see
///   `oag_trace::trace::FLARE_COLUMNS`.
///
/// # Row alignment
///
/// One row per tick, then one final row, mirroring `race::capture`: a row is
/// emitted **before** the tick it labels, which is `scripts/psp-trace.py`'s own
/// alignment ("the craft as this frame's update found it"), and the last row is
/// the state a `--screenshot` of the same command line would draw - after
/// `--pose-boost` has been applied, which `capture` also does after its loop.
/// So `--ticks 0` writes exactly one row: the posed state, drawing nothing.
fn write_trace(
    loaded: race::Loaded,
    cli: &Cli,
    scheme: ControlScheme,
    path: &std::path::Path,
) -> Result<()> {
    use oag_trace::{Flare, Frame, Trace};

    let mut race = race::Race::start(loaded.setup);
    race.set_control_scheme(scheme);

    // Built from the world rather than from `Telemetry`, which carries a summary
    // for the console and not the columns a comparison needs.
    let frame_of = |race: &race::Race, dt: f32| {
        let ship = &race.world.ships[0].physics;
        let body = &ship.body;
        let exhaust = race.exhaust();
        Frame {
            tick: race.world.tick,
            dt,
            grounded: ship.grounded,
            throttle: ship.thrust,
            brake: ship.brake,
            steer: ship.steer,
            airbrake_left: ship.airbrake_left,
            airbrake_right: ship.airbrake_right,
            speed_cached: body.linear_velocity.length(),
            row0: body.right(),
            up: body.up(),
            forward: body.forward(),
            position: body.position,
            velocity: body.linear_velocity,
            speed: body.linear_velocity.length(),
            // The energy pool, so a comparison can put a number on the recovered
            // contact-damage law instead of watching the bar. `oag-trace
            // replay` writes it too; the capture side is
            // `scripts/psp_trace_fields.py`'s `shield` at `entity+0x88`.
            shield: Some(ship.shield),
            flare: Some(Flare {
                boost_timer: exhaust.boost_timer(),
                plume_timer: exhaust.plume_timer(),
                intensity: exhaust.intensity(),
                half_size: exhaust.half_size(),
                engine_on: f32::from(u8::from(exhaust.engine_on())),
                speed_kmh: exhaust.speed_kmh(),
                speed_ramp: exhaust.speed_ramp(),
                boost_accumulator: exhaust.boost_accumulator(),
            }),
            ..Frame::default()
        }
    };

    let dt = oag_core::tick::TickRate::DEFAULT.dt();
    let mut trace = Trace::default();
    let mut held = race::HeldButtons::new(button_mask(cli.hold.as_deref()));
    for tick in 0..cli.ticks {
        trace.frames.push(frame_of(&race, dt));
        held.pulse(
            button_mask(cli.press.as_deref()),
            button_mask(cli.hold.as_deref()),
            tick.is_multiple_of(2),
        );
        let snapshot = held.snapshot();
        race.tick(&snapshot);
    }
    if let Some(age) = cli.pose_boost {
        race.force_boost_state(age, cli.pose_intensity, cli.pose_speed);
    }
    trace.frames.push(frame_of(&race, dt));

    let csv = trace.to_csv();
    std::fs::write(path, &csv)
        .with_context(|| format!("writing {} for --trace-out", path.display()))?;
    println!(
        "trace: {} row(s), {} column(s) -> {}",
        trace.len(),
        trace.columns().len(),
        path.display()
    );
    Ok(())
}

fn run_race(
    cli: &Cli,
    options: race::Options,
    settings: &settings::Settings,
    anisotropy: Anisotropy,
    mut audio: audio::Audio,
    music_discs: audio::MusicDiscs,
) -> Result<()> {
    let loaded = race::load(&options)?;
    for line in &loaded.report {
        println!("{line}");
    }

    if cli.dry_run {
        return Ok(());
    }

    let scheme = resolve_scheme(cli, settings);

    if let Some(path) = cli.trace_out.clone() {
        write_trace(loaded, cli, scheme, &path)?;
        return audio.finish();
    }

    if let Some(path) = cli.screenshot.clone() {
        // Same reasoning as the front end's own capture branch: no window to
        // stay in step with, but the music has to be running before
        // `race::capture`'s first tick. The race playlist rather than
        // `start_music`, so `--race` plays the same music a race launched from
        // the menus does - there is no menu voice here to switch away from,
        // so this simply starts the list at its first (or resumed) track.
        audio.start_race_music(
            &music_discs,
            settings.audio.music_source,
            &boot::default_audio_cache_dir(),
        );
        race::capture(
            loaded,
            &race::CaptureOptions {
                path,
                ticks: cli.ticks,
                held: button_mask(cli.hold.as_deref()),
                pressed: button_mask(cli.press.as_deref()),
                input_script: cli
                    .input_script
                    .as_deref()
                    .map(|path| -> anyhow::Result<_> {
                        let text = std::fs::read_to_string(path)
                            .with_context(|| format!("reading {}", path.display()))?;
                        oag_trace::script::Script::parse(&text)
                            .map_err(|e| anyhow::anyhow!("parsing {}: {e}", path.display()))
                    })
                    .transpose()?,
                scheme,
                size: parse_size(&cli.size)?,
                log_every: cli.log_every,
                aspect: settings.display.aspect,
                anisotropy,
                renderer: settings.graphics.renderer.clone(),
                fov: settings.graphics.fov,
                frustum_culling: settings.graphics.frustum_culling,
                pvs_culling: settings.graphics.pvs_culling,
                animated_textures: settings.graphics.animated_textures,
                bloom: settings.graphics.bloom,
                boost_fov_kick: settings.graphics.boost_fov_kick,
                camera_view: cli.camera_view.unwrap_or(settings.graphics.camera_view),
                anti_aliasing: settings.graphics.anti_aliasing,
                pose_boost: cli.pose_boost,
                pose_intensity: cli.pose_intensity,
                pose_speed: cli.pose_speed,
                presented: cli.presented.then_some(race::Presented {
                    render_scale: settings.graphics.render_scale,
                    presentation: oag_game::upscale::Presentation {
                        upscaler: settings.graphics.upscaler,
                        sharpness: settings.graphics.upscale_sharpness.stops(),
                        anti_aliasing: settings.graphics.anti_aliasing,
                        brightness: settings.display.brightness,
                        gamma: settings.display.gamma,
                    },
                }),
            },
            &mut audio,
        )?;
        return audio.finish();
    }

    println!("\n{RACE_KEYS}{ESC_QUITS}");

    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the simulation runs whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        boot_shell: None,
        media: None,
        boot_overlay: false,
        pick_language: false,
        race: Some(loaded),
        race_options: options,
        trace: cli.trace,
        log_every: cli.log_every,
        anisotropy,
        scheme,
        settings: settings.clone(),
        // `--race` opens a window straight onto a track: no front end, so no
        // font and no sprite sheet, so no menu tree and no circuit list. The
        // settings still apply, they just cannot be changed from here - which
        // is what makes escape quit on this route rather than back out.
        shell: None,
        audio: Some(audio),
        music_discs,
        // `--race` says up front that it ignores `--prefetch` - that converts
        // the front end's movies and sounds, which a race never opens - so
        // there is nothing to wait on here and no loading screen to wait with.
        prefetch: None,
        // Nor anything to draw one with: this route never opens the archives
        // the tips and the glow strip come out of.
        loading_assets: loading::Assets::default(),
        state: None,
    };
    event_loop.run_app(&mut app)?;
    app.finish_audio()
}

/// One application handler for both ways in, because there is one window.
struct App {
    /// The cheap half of the boot, loaded before the window opened.
    ///
    /// Half a boot rather than a whole one because the other half - the movies -
    /// is still decoding on [`Self::media`] while this window opens. See
    /// `boot::Shell`.
    boot_shell: Option<boot::Shell>,
    /// The movies, arriving on a thread of their own.
    ///
    /// Joined by [`Session::finish_loading`], which is also where the two
    /// halves become a `boot::Boot`.
    media: Option<boot::MediaWorker>,
    /// `--overlay`, applied to the sequence once it exists.
    boot_overlay: bool,
    /// `--pick-language`: show the picker even when the settings name one.
    pick_language: bool,
    /// A race loaded before the window opened, which is what `--race` does.
    race: Option<race::Loaded>,
    /// What a race started from `Launch Game` is flown on.
    race_options: race::Options,
    trace: bool,
    log_every: u32,
    anisotropy: Anisotropy,
    /// The control scheme resolved once at startup: `--scheme` over
    /// `[controls] scheme`. See [`Session::scheme`].
    scheme: ControlScheme,
    /// The persisted settings, which the menus edit and write straight back.
    settings: settings::Settings,
    /// What the menus need, absent on the `--race` path.
    shell: Option<Shell>,
    /// The mixer and its device, waiting for the window that will step it.
    ///
    /// Taken by [`Session`] on the first resume, which is why it is an
    /// `Option`: there is one of these per run, not one per window, and winit
    /// may resume more than once.
    audio: Option<audio::Audio>,
    /// Which Pulse releases this machine has, surveyed once before the window
    /// opened.
    ///
    /// Beside `audio` because it is the same kind of thing - a property of the
    /// run rather than of what is on screen - and it is here rather than in
    /// `Shell` because `--race` has no shell and still has music.
    music_discs: audio::MusicDiscs,
    /// What `--prefetch` asked for, **not started yet**.
    ///
    /// It may not start until the boot's own movies are done: both convert
    /// through `ffmpeg` into the same cache directory, and two processes
    /// writing one file is a corrupt file rather than a race that resolves.
    /// [`Session::start_prefetch`] is where it actually spawns, the moment
    /// [`Self::media`] reports finished. `None` on every ordinary boot.
    prefetch: Option<prefetch::Options>,
    /// The wave's glow strip and the disc's tips.
    ///
    /// Read on every windowed boot now, not only under `--prefetch`: the
    /// loading screen covers the movie decode on all of them.
    loading_assets: loading::Assets,
    state: Option<Session>,
}

impl App {
    /// Opens the window and builds whichever stage the command line asked for.
    ///
    /// `Ok(None)` means there was nothing to show, which only happens if the event
    /// loop resumes twice after the loaded state has been taken.
    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<Option<Session>> {
        let gpu = Gpu::new(
            event_loop,
            &self.settings.display,
            &self.settings.graphics.renderer,
        )?;

        // Built before the stage, because a race's depth attachment has to match
        // this rather than the window.
        let target = upscale::target_size(
            display::viewport(gpu.size(), self.settings.display.aspect),
            self.settings.graphics.render_scale,
            gpu.device.limits().max_texture_dimension_2d,
        );
        let framebuffer = upscale::Framebuffer::new(&gpu.device, gpu.config.format, target)
            .context("building the upscale pipeline")?;

        // Taken out of the boot before the front end takes the rest: it belongs
        // to the menus, which outlive the sequence that loaded it, and `--race`
        // has neither.
        //
        // Split in two here, and this is the last place both halves are in one
        // hand: the frames move onto a decode thread and the presentation - the
        // rate, the rectangle - stays behind, because a `Feed` deals in pixels
        // and knows nothing about where they go. `repeat: true`, which is the
        // whole difference between this movie and the intro.
        // **The menu's grid, which is ours and is the PSP's - not the source's.**
        // This rect is drawn by `MenuStage`'s own renderer, and `open_menus`
        // builds that one fresh and never calls `set_space`, so its `screen`
        // uniform is `Space::PSP` whatever disc is mounted. The menu layout it
        // sits behind is this project's own, authored at 480x272, which is why
        // `capture.rs` pins `Space::PSP` on the same picture.
        //
        // Handing this the *source's* space instead was a regression: on a PS2
        // disc it built the rect in a 640x448 grid for a shader normalising
        // against 480x272, and `pillarbox_in` always fills one axis of the grid
        // it is given, so the backdrop overflowed the screen on every aspect.
        // Silent on the PSP, where the two grids are the same numbers, and
        // invisible to `--menu-page`, which goes through `capture.rs`.
        // Taken before the stage rather than after it, because building the
        // front end is what starts the intro's sound and that needs the mixer
        // in hand. Moved rather than cloned: there is one mixer per run, and a
        // second one would either fight the first for the device or split a
        // `--dump-audio` capture across two buffers.
        let Some(mut audio) = self.audio.take() else {
            return Ok(None);
        };
        let stage = if let Some(loaded) = self.race.take() {
            gpu.window.set_title(RACE_TITLE);
            // Straight away on this leg only: `--race` has no boot sequence, so
            // there is no intro for the music to wait behind. The front end's
            // legs below start it from the tick loop instead, the moment the
            // sequence leaves its movie - see `Audio::start_music`. Here rather
            // than back in `main` before the load, which used to run seconds of
            // parsing and transcoding with the music already looping and no
            // window yet on screen to account for it.
            //
            // The race playlist, not `start_music`: `--race` has no menu to
            // play under a race, and no shell to return to (`escape` quits
            // outright on this leg), so there is nothing to pause and resume -
            // just the same list a menu-launched race plays, started fresh.
            audio.start_race_music(
                &self.music_discs,
                self.settings.audio.music_source,
                &boot::default_audio_cache_dir(),
            );
            Stage::race(
                &gpu,
                loaded,
                framebuffer.size(),
                self.anisotropy,
                &self.settings,
                self.scheme,
            )?
        } else if let Some(shell) = self.boot_shell.take() {
            // **Always the loading screen**, where this used to be `--prefetch`
            // only. Every windowed boot now has something to wait for - the
            // movies, which are still decoding on `self.media` as this window
            // opens - and this is the frame that says so instead of a desktop
            // with nothing on it. `finish_loading` swaps in the front end the
            // moment they land.
            Stage::loading(
                &gpu,
                shell,
                self.media.take(),
                &self.loading_assets,
                self.trace,
            )?
        } else {
            return Ok(None);
        };

        let controls = Controls::new();
        for name in controls.pad().names() {
            println!("gamepad: {name}");
        }

        // Built now rather than when the setting is first turned on, and from
        // **our own** 5x7 glyphs rather than the disc's font, for the same
        // reason: the overlay has to work on every route, and `--race` never
        // loads a font at all. One pipeline and a 616-byte atlas, against a
        // fallible build and a borrow dance in the middle of `frame`.
        let overlay = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            None,
            oag_game::font::Atlas::build(),
            &oag_game::sprite::Sheet::default(),
        )
        .context("building the performance overlay")?;

        Ok(Some(Session {
            gpu,
            framebuffer,
            stage,
            controls,
            audio,
            music_discs: self.music_discs.clone(),
            clock: TickClock::new(TickRate::DEFAULT),
            last: std::time::Instant::now(),
            meter: perf::Meter::new(),
            overlay,
            stalled: true,
            next_frame: std::time::Instant::now(),
            race_options: self.race_options.clone(),
            log_every: self.log_every,
            scheme: self.scheme,
            anisotropy: self.anisotropy,
            launched: false,
            settings: self.settings.clone(),
            shell: self.shell.clone(),
            quit: false,
            // Both filled by `finish_loading`, off the movies the boot's own
            // worker is still decoding as this window opens. `--race` never
            // fills them at all.
            backdrop: None,
            backdrop_shape: None,
            // Nothing running yet on either line: the conversion may not start
            // until the boot's own movies are done with the cache, which is
            // what `start_prefetch` waits for.
            prefetch: None,
            prefetch_pending: self.prefetch.take(),
            boot_overlay: self.boot_overlay,
            pick_language: self.pick_language,
        }))
    }

    /// Waits for a conversion the window has outlived.
    ///
    /// Here rather than in `main` because the handle moved onto the [`Session`]
    /// at the first resume - see [`Session::prefetch`]. The window is gone by
    /// now, so a conversion still running has nothing to stay responsive for,
    /// but it does have work worth not throwing away, and it prints as it goes.
    fn finish_prefetch(&mut self) {
        let Some(prefetch) = self
            .state
            .as_mut()
            .and_then(|session| session.prefetch.as_mut())
        else {
            return;
        };
        let outstanding = prefetch.progress();
        if !outstanding.finished {
            println!(
                "waiting for --prefetch: {} of {} converted",
                outstanding.done, outstanding.total
            );
        }
        prefetch.join();
    }

    /// Writes the `--dump-audio` WAV once the event loop has returned.
    ///
    /// Here and not in a winit callback: `exiting` is not guaranteed to run on
    /// every platform, and a capture that silently produced no file would be
    /// indistinguishable from one that produced silence.
    ///
    /// Writes nothing today, because `--dump-audio` is refused without
    /// `--screenshot` and a capture never opens a window. It is wired anyway so
    /// that lifting that restriction is one edit rather than two, and so a
    /// windowed run cannot become the one path that quietly drops its dump.
    ///
    /// # Errors
    ///
    /// Propagates a file that cannot be written.
    fn finish_audio(&self) -> Result<()> {
        match &self.state {
            // The window opened, so `open` took the mixer and `Session` has it.
            Some(session) => session.audio.finish(),
            // It never did - a device that would not start, or a resume that
            // never came. There is nothing to write and no reason to complain.
            None => Ok(()),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        match self.open(event_loop) {
            Ok(Some(session)) => self.state = Some(session),
            Ok(None) => event_loop.exit(),
            Err(e) => {
                eprintln!("error: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(session) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => session.resize(size.width, size.height),

            // A key held while the window loses focus is never seen to come up, and
            // the ship would keep turning while the player is elsewhere.
            WindowEvent::Focused(false) => session.controls.release_all(),

            WindowEvent::KeyboardInput { event, .. } => {
                // **`repeat` matters now that escape navigates.** winit resends
                // `Pressed` while a key is held, and back-one-level repeated
                // thirty times a second walks out of the menus and quits. It
                // did not matter while escape exited on the first one.
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && event.state == ElementState::Pressed
                    && !event.repeat
                {
                    session.escape();
                    return;
                }
                session
                    .controls
                    .set_key(&event.logical_key, event.state == ElementState::Pressed);
            }

            WindowEvent::RedrawRequested => {
                if let Err(e) = session.frame() {
                    eprintln!("frame error: {e:#}");
                    event_loop.exit();
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // A menu asking to quit is handled here rather than where it is raised:
        // the tick loop has no event loop to call, and quitting from inside a
        // frame would leave that frame half-drawn.
        if self.state.as_ref().is_some_and(|session| session.quit) {
            event_loop.exit();
            return;
        }
        let Some(session) = &self.state else {
            return;
        };

        // **The frame limiter is here and not in `frame`**, because the way to
        // produce fewer frames is to ask for fewer, not to draw one and then
        // sleep holding a submitted command buffer. `WaitUntil` hands the
        // waiting to the platform's own timer; `Poll` is what it was before and
        // is still what an unlimited run does.
        match session.next_frame_at() {
            Some(deadline) if std::time::Instant::now() < deadline => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
            _ => {
                event_loop.set_control_flow(ControlFlow::Poll);
                session.gpu.window.request_redraw();
            }
        }
    }
}

/// What winit is asked for, for a mode and a chosen screen.
///
/// `Borderless(None)` means "the monitor this window is on", which is what
/// makes it unable to fail: exclusive fullscreen needs a `VideoMode` enumerated
/// off a monitor and can be refused, and this build does not offer it. See
/// [`display::WindowMode::ALL`]. Naming a monitor keeps that property - it is
/// still borderless, just on a screen this build has already confirmed exists.
fn fullscreen(
    mode: display::WindowMode,
    monitor: Option<winit::monitor::MonitorHandle>,
) -> Option<winit::window::Fullscreen> {
    match mode {
        display::WindowMode::Windowed => None,
        display::WindowMode::Borderless => Some(winit::window::Fullscreen::Borderless(monitor)),
    }
}

/// What each monitor is called on the menu and in the settings file.
///
/// A screen the platform has no name for is numbered instead, so the list has
/// no blank rows. That number is positional and a settings file holding one is
/// therefore as fragile as an index would have been - which is why it is the
/// fallback and not the scheme; see [`display::Monitor`].
fn monitor_names(monitors: &[winit::monitor::MonitorHandle]) -> Vec<String> {
    monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            monitor
                .name()
                .unwrap_or_else(|| format!("screen {}", index + 1))
        })
        .collect()
}

/// The monitor a setting names, or `None` for "let the compositor decide".
///
/// A name this machine does not have is a note and the default, not an error:
/// the ordinary way to get one is to unplug a screen, and refusing to open a
/// window over it would be punishing a player for their own desk. The note
/// lists what is there, because the next thing anyone wants is the spelling.
fn choose_monitor(
    monitors: Vec<winit::monitor::MonitorHandle>,
    setting: &display::Monitor,
) -> Option<winit::monitor::MonitorHandle> {
    let names = monitor_names(&monitors);
    if let Some(index) = setting.choose(&names) {
        return monitors.into_iter().nth(index);
    }
    if let Some(wanted) = setting.name() {
        eprintln!(
            "no monitor named {wanted:?}; using the default (this machine has: {})",
            names.join(", ")
        );
    }
    None
}

/// Where a windowed window goes to sit on `monitor`.
///
/// The arithmetic is [`display::centred`], which is tested; this is the part
/// that reads winit's own rectangle and cannot be. A monitor's scale factor is
/// what turns the setting's logical size into the physical pixels the position
/// is measured in - getting that wrong offsets the window by the difference on
/// any screen that is not at 100 %.
///
/// **Centred on the window's inner extent and applied to its outer one**, so a
/// decorated window sits high by about a title bar. Not corrected, because the
/// correction is not knowable before the window exists and the frame size is
/// the compositor's to decide anyway - this is a request it may refuse
/// outright, and being a title bar off "centred" is the smallest of the ways
/// that can go.
fn centred_on(
    monitor: &winit::monitor::MonitorHandle,
    size: display::Size,
) -> winit::dpi::PhysicalPosition<i32> {
    let scale = monitor.scale_factor();
    let physical = |value: u32| (f64::from(value) * scale).round().max(0.0) as u32;
    let origin = monitor.position();
    let area = monitor.size();
    let (x, y) = display::centred(
        (origin.x, origin.y),
        (area.width, area.height),
        (physical(size.width), physical(size.height)),
    );
    winit::dpi::PhysicalPosition::new(x, y)
}

/// What each vsync setting asks the surface for, best first.
///
/// Named modes rather than wgpu's `Auto` pair, because the three settings are
/// three specific behaviours and `AutoNoVsync` picks between two of them: it
/// prefers `Mailbox` and falls back to `Immediate`, so asking for it made
/// "off" mean *either* "tear for the lowest latency" or "never tear", driver
/// depending. That is exactly the distinction this row now exists to let a
/// player make.
///
/// The cost of naming them is that **only `Fifo` is guaranteed** - Vulkan
/// requires it and makes the other two optional - and configuring a surface
/// with a mode it does not offer is a panic, not an error. Hence a chain per
/// setting and [`Gpu::present_mode`] walking it against what the surface
/// actually reported.
fn present_modes(vsync: perf::Vsync) -> &'static [wgpu::PresentMode] {
    match vsync {
        // Second choice is `Mailbox` and not `Fifo`: what "off" is asked for is
        // a loop that is never blocked, and mailbox keeps that while fifo
        // destroys it.
        perf::Vsync::Off => &[wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox],
        perf::Vsync::On => &[wgpu::PresentMode::Fifo],
        perf::Vsync::Smooth => &[wgpu::PresentMode::Mailbox, wgpu::PresentMode::Fifo],
    }
}

/// The window and the GPU objects, which both stages draw through.
///
/// One window and one device for the whole process: the front end reaching
/// `Launch Game` swaps what is drawn, not what it is drawn with.
struct Gpu {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    /// The present modes this surface actually has. See [`Gpu::present_mode`].
    offered: Vec<wgpu::PresentMode>,
    /// What the RENDERER row offers. See [`adapter::Chosen::offered`].
    adapters: Vec<String>,
    /// What the RENDERER row would have to say to describe this run, which is
    /// not necessarily what the settings file says: a named adapter that would
    /// not make a device fell back to the default, and the default is drawing
    /// with something in particular. See [`adapter::Chosen::in_use`] and the
    /// row's restart note.
    in_use: Vec<String>,
}

/// What one adapter has to hand over before it can be drawn with.
///
/// A struct rather than four statements inline because **all four have to
/// succeed or none of them count**: a named adapter that enumerates fine can
/// still refuse the device or the surface config, and the recovery for that is
/// to run the whole sequence again on a different adapter. See
/// [`Gpu::bring_up`].
struct BroughtUp {
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    offered: Vec<wgpu::PresentMode>,
    adapters: Vec<String>,
    in_use: Vec<String>,
}

impl Gpu {
    /// Everything downstream of picking an adapter, so it can be *re*-run.
    ///
    /// Split out because `apply_setting` saves on every keypress: the moment a
    /// player nudges the RENDERER row, that adapter is in their settings file,
    /// and if it then cannot make a device the game would not start again. A
    /// setting you can change from inside the game must not be able to lock you
    /// out of it, which is the same promise `choose_monitor` makes about a
    /// screen that has been unplugged - one step further down, where the
    /// adapter is found but will not serve.
    fn bring_up(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'static>,
        size: winit::dpi::PhysicalSize<u32>,
        renderer: &display::Renderer,
    ) -> Result<BroughtUp> {
        let chosen = adapter::choose(instance, Some(surface), renderer)?;
        let (device, queue) =
            pollster::block_on(chosen.adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("oag-game"),
                ..Default::default()
            }))
            .context("requesting the device")?;
        let mut config = surface
            .get_default_config(&chosen.adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by this adapter")?;
        // **The window does not encode.** Every shader in this pipeline writes
        // gamma-space values - the GE blends stored bytes, so that is the space
        // the whole thing works in - and an sRGB surface would encode them a
        // second time. See
        // [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
        //
        // Forced rather than accepted: `get_default_config` returns whichever
        // format the adapter lists first, which is the sRGB variant on some
        // backends and not on others, so leaving it alone would make the
        // pipeline's colour space a property of the driver. `Renderer::new`
        // forks the sprite sheet's texture format on `format.is_srgb()` and now
        // always takes the raw side, which is what keeps authored sprite and
        // text colours reaching the screen as authored on every path.
        config.format = config.format.remove_srgb_suffix();
        config.view_formats = vec![config.format];
        // Kept because the adapter is not: every later change of the vsync row
        // has to be checked against this same list, and re-requesting an
        // adapter to ask would be a second answer to the same question.
        let offered = surface.get_capabilities(&chosen.adapter).present_modes;
        // Said here, after the device and the surface config, so the line is
        // about an adapter that *worked*: an attempt that dies at either of them
        // prints its own failure and falls back, and one line naming the loser
        // and another naming the winner would leave a bug report to guess which
        // one drew the picture. It also has to be said at all - `default` names
        // nothing a player could look up, and a name this machine no longer has
        // silently becomes the default. The driver comes with it because "which
        // llvmpipe" and "which Mesa" are the next questions.
        let info = chosen.adapter.get_info();
        println!(
            "renderer: {} (setting: {renderer}, driver: {} {})",
            adapter::label(info.backend, &info.name, info.device_type),
            if info.driver.is_empty() {
                "unnamed"
            } else {
                &info.driver
            },
            if info.driver_info.is_empty() {
                "-"
            } else {
                &info.driver_info
            },
        );
        Ok(BroughtUp {
            device,
            queue,
            config,
            offered,
            adapters: chosen.offered,
            in_use: chosen.in_use,
        })
    }

    fn new(
        event_loop: &ActiveEventLoop,
        settings: &settings::Display,
        renderer: &display::Renderer,
    ) -> Result<Self> {
        // **A windowed window is fixed size, and that is a measurement.** Setting
        // the minimum and maximum to the same thing is the signal a tiling
        // compositor floats a window on rather than squeezing it into a column -
        // measured under niri, which tiles it to a portrait slot without this and
        // honours the requested size with it. Borderless has to be resizable,
        // because the compositor is about to resize it to the monitor.
        //
        // The renderer does not depend on either: `Race::projection` fits the
        // field of view to whatever viewport it is given, and `display::viewport`
        // shapes that viewport, so any window still frames the track correctly.
        let size = settings.window_size;
        let borderless = settings.window_mode == display::WindowMode::Borderless;
        let monitor = choose_monitor(event_loop.available_monitors().collect(), &settings.monitor);
        let mut attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(size.width, size.height))
            .with_resizable(borderless)
            .with_fullscreen(fullscreen(settings.window_mode, monitor.clone()));
        // Borderless carries the choice in the fullscreen request; windowed has
        // nothing to carry it, so the window is placed on the screen instead.
        // Asked for at creation rather than moved afterwards, which would open
        // it on one monitor and jump it to another in view of the player.
        if let (false, Some(monitor)) = (borderless, &monitor) {
            attributes = attributes.with_position(centred_on(monitor, size));
        }
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("creating the window")?,
        );
        // Racing is keyboard/gamepad-only; the cursor has nothing to click on.
        window.set_cursor_visible(false);

        let instance = adapter::instance();
        let surface = instance
            .create_surface(window.clone())
            .context("creating the surface")?;
        let size = window.inner_size();

        let brought = match Self::bring_up(&instance, &surface, size, renderer) {
            Ok(brought) => brought,
            // A *named* adapter that will not serve is recoverable, and the
            // recovery has to happen here rather than being left to the player:
            // the settings file is the only other way back, and a player who
            // cannot start the game cannot be told that from inside it.
            Err(e) if !renderer.is_default() => {
                eprintln!("renderer {renderer} could not be brought up: {e:#}");
                eprintln!(
                    "falling back to the default; set graphics.renderer = \"{}\" to keep it there",
                    display::Renderer::DEFAULT
                );
                Self::bring_up(
                    &instance,
                    &surface,
                    size,
                    &display::Renderer::default_renderer(),
                )
                .context("the default renderer would not start either")?
            }
            Err(e) => return Err(e),
        };
        let BroughtUp {
            device,
            queue,
            config,
            offered,
            adapters,
            in_use,
        } = brought;

        let mut gpu = Self {
            window,
            device,
            queue,
            surface,
            config,
            offered,
            adapters,
            in_use,
        };
        gpu.config.present_mode = gpu.present_mode(settings.vsync);
        gpu.surface.configure(&gpu.device, &gpu.config);
        Ok(gpu)
    }

    /// The best present mode this surface offers for a vsync setting.
    ///
    /// Falls back to `Fifo`, which Vulkan requires every device to have, and
    /// says so once rather than silently: "vsync off did nothing" is otherwise
    /// a bug report about this build rather than a fact about the driver.
    fn present_mode(&self, vsync: perf::Vsync) -> wgpu::PresentMode {
        for wanted in present_modes(vsync) {
            if self.offered.contains(wanted) {
                return *wanted;
            }
        }
        println!(
            "note: this surface offers {:?}, so vsync {vsync} falls back to Fifo",
            self.offered
        );
        wgpu::PresentMode::Fifo
    }

    /// Puts `vsync` into effect.
    fn set_vsync(&mut self, vsync: perf::Vsync) {
        self.config.present_mode = self.present_mode(vsync);
        self.surface.configure(&self.device, &self.config);
    }

    /// The viewport, which every stage draws into.
    fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
}

/// What the window is showing.
///
/// Both variants are boxed: a race carries the whole `World`, eight ship slots
/// wide, and an enum is as large as its largest variant wherever it is stored.
enum Stage {
    /// The wave and the counts, while `--prefetch` converts the disc.
    Loading(Box<LoadingStage>),
    /// The boot sequence: the intro reel, then the language picker.
    Frontend(Box<FrontendStage>),
    /// Our own menus, between the boot sequence and a race.
    Menu(Box<MenuStage>),
    /// A ship on a track.
    Race(Box<RaceStage>),
}

impl Stage {
    /// The loading screen, holding the half-boot it will hand the window to.
    ///
    /// **The boot sequence is carried as data rather than built and paused.**
    /// [`Stage::frontend`] spawns the intro's decode thread, and starting that
    /// ten minutes before anything reads a frame would leave a worker filling a
    /// ring nobody drains. So the front end is built at the hand-off, in
    /// [`Session::finish_loading`], and until then this owns the two halves.
    ///
    /// `media` is the worker still decoding the movies, `None` only if it was
    /// already taken - which cannot happen today, the window opening once.
    fn loading(
        gpu: &Gpu,
        shell: boot::Shell,
        media: Option<boot::MediaWorker>,
        assets: &loading::Assets,
        trace: bool,
    ) -> Result<Self> {
        // The **disc's** font, not the built-in 5x7 set: the tips are the
        // disc's own prose in the player's own language, and `push_text` skips
        // a glyph the atlas has no cell for - so accented text would silently
        // lose characters rather than fail. No video pipeline, this screen
        // having no movie; there is no loading movie in either build.
        let renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            None,
            shell.font.clone(),
            &shell.sprites,
        )?;
        // Sample count 1, matching `upscale::Framebuffer`'s target, which is
        // what this draws into.
        let wave = oag_render::loading::Pipeline::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            &assets.strip,
            1,
        );
        Ok(Self::Loading(Box::new(LoadingStage {
            atlas: shell.font.clone(),
            renderer,
            wave,
            screen: loading::Screen::new(assets.tips.clone()),
            shell: Some(shell),
            media,
            trace,
        })))
    }

    /// `audio` is taken because this is the moment the intro's sound starts,
    /// and there are two ways here - straight off the boot, or out of the
    /// loading screen when its fade runs out. Passing the mixer in makes
    /// starting it part of *becoming* the front end rather than something each
    /// of those two has to remember, which is the difference between a missed
    /// call being a compile error and being a silent movie on one path only.
    fn frontend(
        gpu: &Gpu,
        mut loaded: boot::Boot,
        video_format: Option<VideoFormat>,
        trace: bool,
        audio: &mut audio::Audio,
    ) -> Result<Self> {
        // Before the renderer is built rather than after, so the sound and the
        // first frame of picture start on the same tick: `Session::frame` draws
        // before it ticks, and a movie whose voice started a frame late would
        // be a frame late for the whole reel.
        let renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            video_format,
            loaded.font.clone(),
            &loaded.sprites,
        )?;
        // The sequence's rects are in its source's grid, and the renderer maps
        // rects onto the viewport - so it has to be told which grid, or a PS2
        // screen's widgets are drawn a third again too big. See
        // `frontend::Space`.
        let mut renderer = renderer;
        renderer.set_space(loaded.frontend.space());
        // The store moves onto its own thread and the `Movie` around it is done
        // with: everything else it carried - the frame count, the rate, the
        // aspect - was read into the sequence and the renderer before this.
        // `repeat: false`, because the intro ends rather than starting again.
        let spawn = |movie: Option<movie::Movie>| {
            movie.and_then(|movie| {
                let (width, height) = (movie.width, movie.height);
                movie
                    .frames
                    .map(|frames| movie::Feed::spawn(frames, false, width, height))
            })
        };
        // **Every movie is keyed to the screen that plays it**, and installed on
        // the tick that screen is entered - including the first, whose screen is
        // the boot step on Pulse but *not* on Pure, where the reel plays two steps
        // in. Starting it at load would sound the reel under Pure's language
        // picker, which `--pick-language` makes plainly audible.
        //
        // Spawned now rather than lazily, because the `Movie` and the frames
        // inside it only exist here, on `loaded`.
        let states = loaded.frontend.movie_states();
        let mut pending: Vec<PendingMovie> = Vec::new();
        for (at, sound, film) in [
            (0usize, loaded.movie_sound.take(), loaded.movie),
            (
                1,
                loaded.after_language_movie_sound.take(),
                loaded.after_language_movie,
            ),
        ] {
            if let Some(state) = states.get(at).copied() {
                pending.push(PendingMovie {
                    state,
                    feed: spawn(film),
                    sound,
                });
            }
        }
        let mut stage = FrontendStage {
            renderer,
            frontend: loaded.frontend,
            feed: None,
            pending,
            shown: false,
            backdrop_shown: false,
            held_backdrop: None,
            backdrop_in_planes: false,
            trace,
        };
        // The boot screen gets no `Enter` event - it is where the machine starts -
        // so its own movie, if it has one, is installed here.
        if let Some(state) = stage.frontend.machine().current().map(str::to_string) {
            stage.install_movie(&state, audio);
        }
        Ok(Self::Frontend(Box::new(stage)))
    }

    /// `size` is the **framebuffer's**, not the window's: the scene's depth
    /// attachment has to match the colour one it will be drawn with, and that is
    /// the offscreen target rather than the surface. Getting it from the window
    /// is right only at a render scale of 100 % on an unshaped aspect, which is
    /// exactly the case that would let it ship looking correct.
    fn race(
        gpu: &Gpu,
        loaded: race::Loaded,
        size: (u32, u32),
        anisotropy: Anisotropy,
        // The whole settings block rather than the three values a race reads out
        // of it. Three separate parameters is what this used to be, and each new
        // display preference added a fourth: the values travel together, they all
        // come from one place, and none of them is ever overridden per race. The
        // exception is `anisotropy`, which stays its own parameter precisely
        // because `--anisotropy` *can* override it.
        settings: &settings::Settings,
        scheme: ControlScheme,
    ) -> Result<Self> {
        let race::Loaded {
            setup,
            hud,
            track_model,
            ship_model,
            collision_model,
            sky_model,
            pad_model,
            weapon_pad_model,
            boost_model,
            boost_uv_transform,
            fog_volumes,
            visibility,
            flare,
            noise,
            ..
        } = loaded;
        let scene = race::Scene::new(
            &gpu.device,
            &gpu.queue,
            track_model,
            ship_model,
            collision_model,
            sky_model,
            pad_model,
            weapon_pad_model,
            setup.mode,
            boost_model,
            boost_uv_transform,
            flare,
            noise,
            gpu.config.format,
            size,
            anisotropy,
            settings.graphics.bloom,
            visibility,
            settings.graphics.anti_aliasing,
            fog_volumes,
        )?;
        // Against the **surface** format, like every other renderer here, because
        // the HUD is composited into the offscreen target which shares it.
        let overlay = oag_game::hud::Overlay::new(&gpu.device, &gpu.queue, gpu.config.format, &hud)
            .context("building the HUD overlay")?;
        let mut race = race::Race::start(setup);
        race.set_boost_fov_kick(settings.graphics.boost_fov_kick);
        // Applied before the first tick, but unlike the kick this one is also
        // set again whenever the cycle button or the menu row moves it - see
        // `Session::cycle_camera_view`.
        race.set_camera_view(settings.graphics.camera_view);
        race.set_control_scheme(scheme);
        Ok(Self::Race(Box::new(RaceStage {
            scene,
            race,
            hud: overlay,
        })))
    }
}

/// The front end, and everything only it needs.
/// The menus, and a renderer of their own.
///
/// Built from the same font atlas and sprite sheet the front end draws with,
/// rather than taken over from it: the menus have to be openable from somewhere
/// that is not the front end - a pause menu, eventually - and a stage that can
/// only exist downstream of another one cannot be. It costs one pipeline build
/// at a moment already spent loading.
struct MenuStage {
    renderer: Renderer,
    menu: menu::Menu,
    /// Where every row goes and what colour it is. Built once, when the menus
    /// open, from the title's own table and the line height of the face that
    /// will draw the rows - the two halves `menu::Skin` exists to join.
    skin: menu::Skin,
    /// The page change in flight, if one is.
    ///
    /// **The page the player left is kept as a finished draw list, not as a
    /// `Menu` to re-draw.** The transition only ever scales and fades what was
    /// already on screen, so a snapshot is both cheaper and more honest than a
    /// second model that would keep answering input.
    change: Option<PageChange>,
    /// Where the disc's looping backdrop has got to, and where it goes on
    /// screen. `None` when this source has no backdrop, and then the rows are
    /// drawn on black exactly as they were before it existed.
    ///
    /// The player lives here rather than in `Session` because it is part of what
    /// is on screen: a stage that is not running should not be advancing a
    /// movie, and putting it here makes that structural instead of remembered.
    /// **It is usually not this stage's own player**: coming out of the boot
    /// sequence it is the one `Show Logo` was running, moved across rather than
    /// rebuilt, so the loop never restarts on the handoff. See
    /// [`menu_playhead`].
    backdrop: Option<Backdrop>,
}

/// A page change part-way through.
///
/// Held for as long as the tween runs and then dropped. See
/// [`menu::Layers::zoomed`] for the effect and `docs/ui/menus-original.md` for
/// the capture it came off.
struct PageChange {
    /// The page being left, as it looked on its last frame.
    leaving: menu::Layers,
    /// The clock both halves share.
    tween: oag_game::anim::Tween,
    /// How far each half travels.
    shape: menu::Transition,
}

/// The looping menu picture: a player, where it goes, and which frame is up.
struct Backdrop {
    player: movie::Player,
    rect: [f32; 4],
    /// Which frame is **actually in the renderer's planes**, or `None` while none
    /// is.
    ///
    /// Not "which frame we would like": the decode happens on another thread, so
    /// there can genuinely be no picture yet, and `None` is what stops the video
    /// quad being drawn over zeroed planes - which is a green rectangle, not a
    /// black one. It is also what the draw list reports, so the list names the
    /// frame on screen rather than one that may not have arrived. See
    /// [`MenuStage::render`].
    ///
    /// **Coming out of the boot sequence this starts `Some`**, seeded by
    /// `Session::open_menus` from the picture the front end had on screen. It
    /// used to start `None` on every path, and the one to three frames of
    /// menu-on-black that produced were the flicker on the START press. It still
    /// starts `None` on the `escape` -> menus path, which carries no playhead and
    /// no picture and so shows black until the restarted feed produces frame 0 -
    /// see [`menu_playhead`], which is where that whole path's judgement call is
    /// argued.
    shown: Option<usize>,
}

/// The playhead the menus open on: the one already running, when there is one.
///
/// # Coming out of the boot sequence, there always is
///
/// `Show Logo` and the menus are two screens in front of **one** playback of
/// `Data\Movies\Backdrop`, not two playbacks of it: `Show Logo` is a child of
/// the `FE Screen` that owns the movie, so on hardware pressing START changes
/// which widgets are drawn over a loop that never stops. Building a
/// [`movie::Player`] here instead put the picture back at frame zero on that
/// press, which is the jump a player who has run the original reported. So the
/// front end hands its playhead over - see
/// [`frontend::Frontend::take_backdrop`] - and this passes it straight through,
/// unmodified and un-rewound. There is one [`movie::Feed`] for the whole
/// session already, so nothing else has to move.
///
/// # Leaving a race is the other way in, and it does start over
///
/// A race replaces the menu stage, and its playhead goes with it: nothing
/// advances the backdrop while a race is on screen, and nothing takes frames
/// out of the feed either, so both stop where the menus left them. Rather than
/// resume mid-loop from a stage that no longer exists, `escape` gets a fresh
/// playhead and `Session::open_menus` restarts the feed to match - which is
/// what the original does too, `FE Screen` being torn down for a race and
/// rebuilt after it, with its `autostart` movie starting again. It is also the
/// cheap end of the trade: one decoder flush against carrying a position
/// through a stage that has no use for it.
///
/// **This is a judgement call and the docs do not settle it.** What is
/// confirmed against the original is the boot handoff above; nobody has
/// measured the loop's phase across a race. If it turns out to continue there
/// too, the change is to stash the playhead on [`Session`] when a race starts
/// and pass it back in here - the feed needs no restart for that, because it
/// parks at most four frames past where the menus stopped taking from it.
fn menu_playhead(
    carried: Option<movie::Player>,
    frames: usize,
    frame_rate: (u64, u64),
) -> movie::Player {
    carried.unwrap_or_else(|| {
        // `repeat`, which is the whole difference between this movie and the
        // intro: `FE Screen` sits under it for as long as a player is in the
        // menus, so it wraps rather than finishing on its last frame. See
        // `movie::Player`.
        movie::Player::new(frames, true, frame_rate)
    })
}

impl MenuStage {
    /// Advances the backdrop by one tick.
    ///
    /// Driven from the same fixed `dt` the simulation is, and for the same
    /// reason the front end's movie is: nothing here reads the wall clock. The
    /// player wraps rather than finishing - it is a loop, which is the whole
    /// difference between this movie and the intro.
    fn tick(&mut self, dt: f64) {
        if let Some(backdrop) = &mut self.backdrop {
            backdrop.player.update(dt);
        }
        // The same fixed `dt` the backdrop is stepped with, and for the same
        // reason: nothing on this stage reads the wall clock, so two runs of
        // the same `--ticks` produce the same picture. See `oag_game::anim`.
        if let Some(change) = &mut self.change {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a tick is milliseconds; f32 holds it exactly"
            )]
            change.tween.advance(dt as f32);
            if change.tween.done() {
                self.change = None;
            }
        }
    }

    /// Starts a page change, snapshotting the page being left.
    ///
    /// Called after the model has already moved, so `leaving` is passed in
    /// rather than drawn here - the page it holds no longer exists as far as
    /// the model is concerned.
    ///
    /// A transition already in flight is **replaced**, not queued: a player
    /// holding a direction moves faster than half a second a page, and queueing
    /// would run the menus behind the input by however long the player kept
    /// going.
    fn begin_change(&mut self, leaving: menu::Layers) {
        self.change = Some(PageChange {
            leaving,
            tween: oag_game::anim::Tween::new(self.skin.transition_secs()),
            shape: menu::Transition::default(),
        });
    }

    /// Draws the page, uploading whatever backdrop frame the decode thread has
    /// ready.
    ///
    /// # The decode used to be on this thread, and it cost up to 30 ms a frame
    ///
    /// **Measured on the PSP backdrop, release build: `FrameStore::read_frame`
    /// took 0.03 ms to 30 ms for one frame**, depending on where in the movie the
    /// loop had got to - about 5 ms through the quiet half, 12-30 ms through the
    /// busy one, with every other call nearly free because the decoder's frame
    /// delay is 1. The upload was 0.06 ms and never the problem. At 30 frame
    /// changes a second that was **roughly a quarter of every second spent
    /// decoding on the thread that also draws**, which capped a 240-limited loop
    /// near 180 and, unlimited, hid inside an average that looked fine while
    /// individual frames were tens of milliseconds long.
    ///
    /// It is now a [`movie::Feed`]: a worker thread decodes ahead into a small
    /// ring and this asks for the newest frame at or before the playhead. What is
    /// left on this thread is the upload, and only on a frame that changed. See
    /// [ADR-0010](../../docs/architecture/adr/0010-movie-decode-thread.md).
    ///
    /// # A frame that has not arrived is not a frame
    ///
    /// `take_upto` returning `None` is ordinary and means *keep what is on
    /// screen*. It happens for the first frame or two after the menus open, and
    /// it would happen again if the worker ever fell behind. Two rules follow,
    /// and both are load-bearing rather than defensive:
    ///
    /// - **Nothing is uploaded**, so the planes keep the last good picture. A
    ///   frame held one frame longer is invisible at 30 Hz.
    /// - **With no picture at all, the video draw is left out entirely.** Zeroed
    ///   I420 planes are not black - `Y=0, U=0, V=0` is green - so drawing the
    ///   quad before the first frame lands would flash green over the menu. The
    ///   rows draw on black instead, exactly as they do on a source with no
    ///   backdrop.
    fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        feed: Option<&mut movie::Feed>,
    ) -> Result<()> {
        let shown = match (&mut self.backdrop, feed) {
            (Some(backdrop), Some(feed)) => {
                // Surfaced before the frame it would have been, and once: the
                // feed hands a decode failure over exactly one time.
                if let Some(reason) = feed.take_error() {
                    bail!("decoding the menu backdrop: {reason}");
                }
                if let Some(frame) = feed.take_upto(backdrop.player.position()) {
                    self.renderer.upload_frame(&gpu.queue, &frame.picture)?;
                    backdrop.shown = Some(frame.index);
                }
                // The frame on screen, not the one the playhead names.
                backdrop.shown.map(|frame| menu::Backdrop {
                    rect: backdrop.rect,
                    frame,
                    position: backdrop.player.position(),
                })
            }
            // A backdrop whose frames could not be opened is no backdrop: the
            // planes hold nothing, and drawing them would be a green rectangle
            // over the menu rather than a missing picture.
            _ => None,
        };
        let arriving = menu::draw_list(&self.menu, &self.skin, &keys::bound_keys, shown);
        let list = match &self.change {
            // The page being left grows and fades out; the one arriving grows
            // into place from smaller and fades in. Both run off one tween, so
            // they cannot drift apart. The backdrop is drawn once, by the page
            // arriving, because it is the same looping movie either way.
            Some(change) => {
                let t = change.tween.eased();
                let shape = &change.shape;
                let going = change.leaving.clone().zoomed(
                    shape.origin,
                    1.0 + (shape.out_scale - 1.0) * t,
                    1.0 - t,
                );
                let coming =
                    arriving.zoomed(shape.origin, shape.in_scale + (1.0 - shape.in_scale) * t, t);
                let mut list = coming.backdrop.clone();
                list.extend(going.chrome);
                list.extend(going.body);
                list.extend(coming.chrome);
                list.extend(coming.body);
                list
            }
            None => arriving.flatten(),
        };
        self.renderer
            .render(&gpu.device, &gpu.queue, encoder, view, &list, viewport);
        Ok(())
    }
}

/// The loading screen: two passes, and the front end waiting behind it.
///
/// It holds no [`prefetch::Prefetch`], deliberately. The worker handle lives on
/// [`Session`], because this stage is *replaced* when the wait is over and a
/// handle stored here would be dropped by that replacement - which sets the
/// stop flag and silently abandons whatever conversion was still to do. What
/// arrives here instead is a [`prefetch::Progress`] snapshot, per frame, which
/// also leaves `loading::Screen` drivable from a test with no worker at all.
struct LoadingStage {
    renderer: Renderer,
    /// The wave's own pipeline: additive, screen-space, no depth.
    wave: oag_render::loading::Pipeline,
    screen: loading::Screen,
    /// The atlas the layout measures its wrapping and eliding with. [`Renderer`]
    /// owns a copy and does not lend it out.
    atlas: oag_game::font::Atlas,
    /// The cheap half of the boot, waiting for the other one. `None` once
    /// taken, which is also what stops the hand-off happening twice.
    shell: Option<boot::Shell>,
    /// The movies, still decoding. **What this screen is actually waiting for**
    /// on an ordinary boot - `--prefetch`, when it is on, is waited for as well.
    media: Option<boot::MediaWorker>,
    trace: bool,
}

impl LoadingStage {
    /// Whether the movies have landed, so the fade may start.
    fn media_ready(&self) -> bool {
        self.media
            .as_ref()
            .is_none_or(boot::MediaWorker::is_finished)
    }
}

impl LoadingStage {
    /// Draws the text, then the wave over it.
    ///
    /// In that order and in two passes because they are two different blends:
    /// the UI list is alpha-over and clears the frame, the wave is additive over
    /// what is already there. See [`capture::draw_wave`], which is the same
    /// second pass and is shared so the window and a capture cannot drift.
    fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        progress: &prefetch::Progress,
    ) {
        let quads = self.screen.quads();
        let vertices = oag_render::loading::vertices(&quads);
        self.wave.upload(
            &gpu.queue,
            [1.0, 1.0, 1.0, self.screen.opacity()],
            &vertices,
        );
        let list = self.screen.draw_list(progress, &self.atlas);
        self.renderer
            .render(&gpu.device, &gpu.queue, encoder, view, &list, viewport);
        capture::draw_wave(encoder, view, &self.wave, viewport);
    }
}

struct FrontendStage {
    renderer: Renderer,
    frontend: Frontend,
    /// The intro, decoded on a worker thread. `None` on a source with no movie,
    /// under `--no-video`, and with no `ffmpeg` - and then the renderer was built
    /// with no video pipeline either, so the draw is skipped rather than green.
    feed: Option<movie::Feed>,
    /// Every movie still waiting for the screen that plays it.
    ///
    /// Held rather than started up front because they share one voice:
    /// [`crate::audio::Audio::start_movie`] stops whatever was playing, so
    /// starting two at load would mean the second silenced the first. Each is
    /// installed on the tick its own screen is entered - see
    /// [`FrontendStage::install_movie`].
    pending: Vec<PendingMovie>,
    /// Whether a picture has ever reached the planes. See
    /// [`FrontendStage::sync_video`].
    shown: bool,
    /// The same, for the backdrop `Show Logo` sits on. Kept apart from `shown`
    /// because the two movies fill the planes at different points in the
    /// sequence, and either can be the one that has not arrived yet.
    backdrop_shown: bool,
    /// The newest backdrop picture taken from the feed, **kept after it has
    /// been used** rather than dropped.
    ///
    /// [`movie::Feed::take_upto`] pops: a frame handed over is gone from the
    /// ring, and asking again for the same position returns `None`. The pump
    /// below runs on every frame whether or not the backdrop is on screen (see
    /// [`FrontendStage::sync_video`]), so without this the frame it popped on a
    /// frame that drew the intro was thrown away, and the first frame that
    /// wanted to *draw* the backdrop found the ring already past it - one to
    /// three frames of `Show Logo` over black before the playhead reached the
    /// next decoded frame. Holding it costs one 480x272 picture and makes the
    /// handoff seamless, both into `Show Logo` and on into the menus, which take
    /// it through [`FrontendStage::held_backdrop`].
    held_backdrop: Option<HeldFrame>,
    /// Whether [`FrontendStage::held_backdrop`] is what the planes hold now.
    ///
    /// One set of planes serves both movies, so an intro upload displaces the
    /// backdrop and the next backdrop draw has to put it back even though no
    /// new frame arrived. Without this the flag would say "uploaded" about a
    /// picture the intro had since overwritten.
    backdrop_in_planes: bool,
    trace: bool,
}

/// One boot movie, waiting for the screen that plays it.
struct PendingMovie {
    /// The screen this movie belongs to, from the title's own chain.
    state: &'static str,
    /// Its frames, already decoding on a worker thread. `None` under
    /// `--no-video`, with no `ffmpeg`, or when the movie could not be read.
    feed: Option<movie::Feed>,
    /// Its own track. `None` whenever the movie should be silent.
    sound: Option<at3::Pcm>,
}

/// A decoded backdrop picture kept past the moment it was taken from the feed.
///
/// The index rather than the position, because that is what a draw list reports
/// and what [`Backdrop::shown`] holds.
struct HeldFrame {
    index: usize,
    picture: movie::VideoFrame,
}

impl FrontendStage {
    /// Puts `state`'s own movie on screen and in the mixer, if it has one.
    ///
    /// One set of I420 planes serves every movie the front end draws, and one
    /// voice serves every movie's sound, so this is a **handover** rather than an
    /// addition: the feed is replaced and `start_movie` stops whatever was
    /// sounding. The arrived-a-picture-yet bookkeeping resets with it, or the new
    /// movie's first frame would either show the previous movie's last picture or
    /// be skipped as already shown.
    ///
    /// Idempotent by construction - the entry is taken out of `pending` - so a
    /// state re-entered later does not restart its movie.
    fn install_movie(&mut self, state: &str, audio: &mut audio::Audio) {
        let Some(at) = self.pending.iter().position(|movie| movie.state == state) else {
            return;
        };
        let PendingMovie { feed, sound, .. } = self.pending.remove(at);
        self.feed = feed;
        self.shown = false;
        self.backdrop_in_planes = false;
        audio.start_boot_movie(state, sound);
    }

    fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        backdrop: Option<&mut movie::Feed>,
    ) -> Result<()> {
        let mut list = self.frontend.draw_list();
        self.sync_video(&gpu.queue, &mut list, backdrop)?;
        self.renderer
            .render(&gpu.device, &gpu.queue, encoder, view, &list, viewport);
        Ok(())
    }

    /// What is decoding the intro, for the `dev` performance overlay. `None`
    /// on a source with no movie, under `--no-video`, or with no `ffmpeg` -
    /// see [`FrontendStage::feed`].
    fn video_label(&self) -> Option<&'static str> {
        self.feed.as_ref().map(movie::Feed::decoder_label)
    }

    /// Uploads the newest decoded frame at or before the one the draw list asks
    /// for.
    ///
    /// **This is the older of the two blocking decodes and it has always done
    /// it**: the intro called `FrameStore::read_frame` straight from the render
    /// thread from the day it was written, and the menu backdrop only made the
    /// cost measurable. Both go through a [`movie::Feed`] now - see
    /// [`MenuStage::render`] for the numbers and
    /// [ADR-0010](../../docs/architecture/adr/0010-movie-decode-thread.md) for
    /// the reasoning.
    ///
    /// The intro does not loop, so its position and its frame index are the same
    /// number - clamped to the cache, because `--movie-frames` can make the cache
    /// shorter than the sequence and the last frame then holds for the rest of it.
    ///
    /// `list` is trimmed rather than only read: until a picture has reached the
    /// planes there is nothing to draw, and drawing the quad anyway would put
    /// green over the screen rather than black, zeroed I420 being green. This
    /// costs the first frame or two of a 40-second movie that fades up from black
    /// anyway.
    /// **Two movies reach this, and which one is named by the draw rather than
    /// inferred.** `Show Logo` sits on the looping backdrop, so the sequence's
    /// last screen asks for a different file than its first one does; taking the
    /// frame from the intro's feed instead would draw a picture rather than fail,
    /// which is the trap `crate::capture` already documents. The two share one
    /// set of planes, which is safe only because `boot::load` declines to set the
    /// backdrop at all unless its geometry matches the intro's.
    ///
    /// # The backdrop's feed is pumped on every frame, drawn or not
    ///
    /// The backdrop's playhead runs from the moment the sequence starts - the
    /// movie is `autostart` on `FE Screen`, which the boot opens long before
    /// `Show Logo` draws it - but only `Show Logo` puts it on screen. A feed
    /// decodes a fixed few frames ahead and then parks, so a feed nobody takes
    /// from sits four frames from the start while the playhead is
    /// hundreds of frames on; the moment `Show Logo` appeared it would then rush
    /// through the whole movie at decode speed catching up.
    ///
    /// So the frame at the playhead is taken every frame whatever is on screen,
    /// and only *uploaded* when the backdrop is the movie being drawn. That
    /// keeps the one feed and the one playhead in step from boot to the menus,
    /// which is what makes handing the playhead over at `Launch Game`
    /// ([`frontend::Frontend::take_backdrop`]) a continuation rather than a
    /// second playback. It costs the worker thread decoding a 480x272 movie at
    /// 30 Hz during the intro - which is what the hardware is doing at that
    /// moment too.
    fn sync_video(
        &mut self,
        queue: &wgpu::Queue,
        list: &mut Vec<frontend::Draw>,
        backdrop: Option<&mut movie::Feed>,
    ) -> Result<()> {
        // `find_map` is only correct because a list carries at most one video,
        // which `Frontend::insert_backdrop` is what guarantees - and the
        // renderer would take the *last* one rather than this first one, so the
        // two would disagree if that ever stopped holding. Asserted rather than
        // handled: a second video needs a second plane set and a second bind
        // group, which is a renderer feature and not something to paper over
        // here.
        debug_assert!(
            list.iter()
                .filter(|draw| matches!(draw, frontend::Draw::Video { .. }))
                .count()
                <= 1,
            "a draw list carries at most one Draw::Video; the renderer draws one \
             video quad from one plane set"
        );
        let drawn = list.iter().find_map(|draw| match draw {
            frontend::Draw::Video {
                position, source, ..
            } => Some((*position, *source)),
            _ => None,
        });
        // Copied out before the renderer is touched, both being fields of
        // `self`, and because the pump below runs on frames whose draw list
        // names no movie at all.
        let playhead = self.frontend.backdrop().map(movie::Player::position);

        let has_backdrop = backdrop.is_some();
        if let (Some(feed), Some(position)) = (backdrop, playhead) {
            if let Some(reason) = feed.take_error() {
                bail!("decoding the menu backdrop: {reason}");
            }
            // Kept rather than used-or-dropped: the take is a pop, and the
            // frame popped on a frame that draws the intro is the one
            // `Show Logo` asks for a moment later. See
            // [`FrontendStage::held_backdrop`].
            if let Some(frame) = feed.take_upto(position) {
                self.held_backdrop = Some(HeldFrame {
                    index: frame.index,
                    picture: frame.picture,
                });
                self.backdrop_in_planes = false;
            }
            // Uploaded only when the backdrop is the movie on screen - one
            // renderer, one set of planes, and the intro is in them until
            // `Show Logo` - and only when the planes do not already hold it, so
            // the steady state is the same one upload per decoded frame it
            // always was.
            if matches!(drawn, Some((_, frontend::Video::Backdrop)))
                && !self.backdrop_in_planes
                && let Some(held) = &self.held_backdrop
            {
                self.renderer.upload_frame(queue, &held.picture)?;
                self.backdrop_in_planes = true;
                self.backdrop_shown = true;
            }
        }

        // The intro, only on the frames that draw it. Unlike the backdrop it
        // does not loop and nothing carries it past the sequence, so a feed left
        // parked while another screen is up has nothing to fall behind.
        if let Some((position, frontend::Video::Intro)) = drawn
            && let Some(feed) = self.feed.as_mut()
        {
            if let Some(reason) = feed.take_error() {
                bail!("decoding the intro movie: {reason}");
            }
            // `saturating_sub`, so an empty cache is a movie with no picture
            // rather than a panic on `0 - 1`. The cache can be shorter than the
            // sequence - `--movie-frames` - and then the last frame holds.
            let position = position.min(feed.len().saturating_sub(1) as u64);
            if let Some(frame) = feed.take_upto(position) {
                self.renderer.upload_frame(queue, &frame.picture)?;
                self.shown = true;
                // The intro has displaced whatever backdrop picture was in the
                // planes, so the next backdrop draw has to upload again.
                self.backdrop_in_planes = false;
            }
        }

        // A quad with nothing in its planes is a green rectangle, zeroed I420
        // not being black - so a movie with no feed at all (a source without a
        // backdrop, `--no-video`, no `ffmpeg`) and one whose first picture has
        // not arrived yet are both dropped from the list rather than drawn.
        let ready = match drawn {
            Some((_, frontend::Video::Intro)) => self.feed.is_some() && self.shown,
            Some((_, frontend::Video::Backdrop)) => has_backdrop && self.backdrop_shown,
            None => return Ok(()),
        };
        if !ready {
            list.retain(|draw| !matches!(draw, frontend::Draw::Video { .. }));
        }
        Ok(())
    }
}

/// A race, and everything only it needs.
struct RaceStage {
    scene: race::Scene,
    race: race::Race,
    /// The HUD, or `None` when the disc's layout could not be read.
    hud: Option<oag_game::hud::Overlay>,
}

impl RaceStage {
    /// `&mut self` rather than `&self`, unlike the other stages: the HUD's
    /// renderers own a growable instance buffer, the same reason
    /// [`Renderer::overlay`] takes `&mut self`. The scene stays immutable behind
    /// its own `RefCell`.
    // The two culling flags are independent settings read from the same place,
    // and a struct to carry them past one call site would name the grouping
    // without clarifying it - the same call this file already makes for
    // `Scene::new`.
    #[allow(clippy::too_many_arguments)]
    fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        fov: display::Fov,
        cull: bool,
        pvs_cull: bool,
        animated_textures: bool,
    ) -> race::SceneStats {
        let stats = self.scene.render(
            &gpu.device,
            &gpu.queue,
            encoder,
            view,
            &self.race,
            viewport,
            fov,
            cull,
            pvs_cull,
            animated_textures,
        );

        // Over the scene and inside the same target, so the HUD is drawn at the
        // render scale the game is and lands in a `--screenshot` too.
        if let Some(hud) = &mut self.hud {
            let readout = self.race.readout();
            hud.draw(&gpu.device, &gpu.queue, encoder, view, &readout, viewport);
        }
        stats
    }
}

/// Everything that only exists once there is a window.
struct Session {
    gpu: Gpu,
    stage: Stage,
    /// One set of devices for both stages: they belong to the window rather than
    /// to what is on screen, so key state carries across the handoff and a focus
    /// loss releases everything whichever stage is running.
    controls: Controls,
    /// The mixer, and the device behind it when this run has one.
    ///
    /// Beside `controls` because it is the same kind of thing: a device that
    /// belongs to the run rather than to whatever is on screen, so a race
    /// starting does not restart the music. Stepped from inside the fixed
    /// timestep and never from the frame - see [`audio::Audio::tick`].
    audio: audio::Audio,
    /// Which Pulse releases this machine has. See [`App::music_discs`].
    music_discs: audio::MusicDiscs,
    clock: TickClock,
    last: std::time::Instant,
    /// Recent frame times, for the performance overlay.
    ///
    /// Fed from the same `elapsed` the fixed timestep is driven by, which is
    /// the only wall clock in the process. Nothing the simulation reads comes
    /// back out of it - see [`perf`].
    meter: perf::Meter,
    /// Draws the overlay over whatever the stage drew.
    ///
    /// A renderer of its own because a race has none: `race::Scene` draws
    /// meshes and knows nothing about text.
    overlay: Renderer,
    /// Set whenever the loop is about to stall on a load, so the frame that
    /// carries it is dropped from [`Session::meter`] rather than measured.
    ///
    /// Starts `true`: the first frame's `elapsed` reaches back to before the
    /// window existed.
    stalled: bool,
    /// When the next frame is due, under a frame limit.
    ///
    /// A schedule rather than a stopwatch - see
    /// [`Session::schedule_next_frame`], which is where the difference between
    /// asking for 240 and getting it lives.
    next_frame: std::time::Instant,
    /// What `Launch Game` starts, kept because the front end is loaded long
    /// before anyone knows whether a race will be asked for.
    race_options: race::Options,
    log_every: u32,
    anisotropy: Anisotropy,
    /// The control scheme every race this session starts is driven with.
    ///
    /// Resolved once, at startup, from `--scheme` over `[controls] scheme`, and
    /// not re-read afterwards. Deliberate: a scheme changed mid-race would leave
    /// a half-finished gesture armed in `ShipState`, so the `CONTROLS` row says
    /// the change applies to the next race the way `BOOST FOV KICK` does.
    scheme: ControlScheme,
    /// Set the first time `Launch Game` starts a race, so a load that fails is
    /// reported once rather than on every frame.
    launched: bool,
    /// The persisted settings, kept because the menus change them and every
    /// change is written straight back.
    settings: settings::Settings,
    /// Where every stage draws, before it is stretched onto the surface.
    ///
    /// On the session rather than on a stage because it outlives them: a race
    /// taking the window over does not want a fresh target, and the size it
    /// should be is a property of the window and the settings rather than of
    /// what happens to be on screen.
    framebuffer: upscale::Framebuffer,
    /// Set when a menu asks to quit, read by the event loop.
    quit: bool,
    /// What the menus need, when this run has menus at all.
    shell: Option<Shell>,
    /// The looping picture the menus are drawn on, when this source has one.
    ///
    /// On the session and not in [`Shell`], which is `Clone`d into every stage
    /// that needs it: a movie is a decoder and a multi-megabyte blob, held once.
    /// The menu stage borrows the feed for the one upload it needs and owns the
    /// playhead. See [`MenuStage::backdrop`].
    ///
    /// **It outlives the menu stage on purpose.** A player walking menu -> race
    /// -> escape -> menu would otherwise pay a fresh decoder and a fresh read of
    /// the cache file every time; instead the stage is rebuilt and the feed is
    /// only [`movie::Feed::restart`]ed, which is one decoder flush. It also means
    /// the worker is alive while a race is on screen - parked on a full ring,
    /// costing nothing, because nothing is taking frames out of it.
    ///
    /// **It also outlives the boot sequence, and that is the whole reason there
    /// is one movie here rather than two.** The front end draws out of this same
    /// feed under `Show Logo` and hands its playhead to the menus when it ends -
    /// see [`menu_playhead`] - so nothing is restarted on that path. The
    /// [`movie::Feed::restart`] above is for the race path only, where there is
    /// no playhead left to continue.
    backdrop: Option<movie::Feed>,
    /// The backdrop's frame rate and where on screen it goes, kept because the
    /// feed carries pixels and not presentation.
    backdrop_shape: Option<BackdropShape>,
    /// The `--prefetch` worker, when this run started one.
    ///
    /// **On the session and not on the loading stage.** The stage is replaced
    /// the moment the wait is over, and a handle owned by it would be dropped by
    /// that replacement - `Prefetch`'s `Drop` sets the stop flag, so a run whose
    /// loading screen finished planning first would silently abandon everything
    /// still to convert. Here it outlives every stage and is joined once, at the
    /// exit, by [`App::finish_prefetch`].
    prefetch: Option<prefetch::Prefetch>,
    /// What `--prefetch` asked for, until it is safe to start. See
    /// [`App::prefetch`] and [`Session::start_prefetch`].
    prefetch_pending: Option<prefetch::Options>,
    /// `--overlay`, applied when the sequence is assembled. Not to be confused
    /// with [`Self::overlay`] above, which is the performance overlay's own
    /// renderer: this one draws the intro's frame counter.
    boot_overlay: bool,
    /// `--pick-language`, read at the same moment.
    pick_language: bool,
}

/// What the menus need to know about the backdrop besides its pixels.
///
/// Read off the [`movie::Movie`] before its frames moved onto a decode thread,
/// because that is the last moment both are in one place.
#[derive(Clone, Copy)]
struct BackdropShape {
    /// The rate this cut presents frames at. A `.PMF` is 30000/1001; the PS2's
    /// `.IPF` cuts are 25/1 or 30000/1001 depending on which one it is.
    frame_rate: (u64, u64),
    /// Where the picture goes on the 480x272 screen, pillarboxed if the cut's
    /// display aspect is not the screen's.
    rect: [f32; 4],
}

/// Everything the menus need, gathered where it is loaded.
///
/// `None` on the `--race` path, which opens a window straight onto a track and
/// never loads a front end - so there is no font, no sprite sheet, and nothing
/// to draw a menu with. That is a real state rather than an oversight, and
/// [`Session::open_menus`] says so rather than unwrapping.
#[derive(Clone)]
struct Shell {
    definition: menu::Definition,
    /// Every raceable circuit and the name to show for it.
    tracks: Vec<(catalogue::Track, String)>,
    /// Every team, valued by its id and labelled off the disc's string table.
    ///
    /// A pack's teams are in here too, and indistinguishable from the disc's -
    /// which is the point: the front end has no concept of downloadable
    /// content, only of what this source offers.
    teams: Vec<menu::Choice>,
    /// Every language this source offers, valued by its English name and
    /// labelled in itself.
    languages: Vec<menu::Choice>,
    /// The race modes, valued by their token and labelled off the disc.
    ///
    /// Resolved once here rather than each time the menus open, the same way the
    /// circuits are: the strings do not change while the game runs, and the
    /// string table is not kept past boot.
    modes: Vec<menu::Choice>,
    font: oag_game::font::Atlas,
    sprites: oag_game::sprite::Sheet,
    /// How this title lays its menus out and colours them, carried from the
    /// serial that identified the source. See `boot::Shell::menu_skin`.
    menu_skin: &'static oag_title::MenuSkin,
    /// The face menu rows are drawn in, which is a bigger one than the rest
    /// of the front end uses. `None` draws them in `font`.
    menu_font: Option<oag_game::font::Atlas>,
}

impl Shell {
    /// Which circuit a stored `race.track` names, if this source has it.
    fn track(&self, id: &str) -> Option<&catalogue::Track> {
        self.tracks
            .iter()
            .map(|(track, _)| track)
            .find(|track| track.id == id)
    }

    /// Whether a stored `race.team` is one this source offers, and its id.
    ///
    /// The teams are already `menu::Choice`s here rather than
    /// `catalogue::Team`s, and the value side of a choice *is* the id - see
    /// where the shell is built.
    fn team(&self, id: &str) -> Option<&str> {
        self.teams
            .iter()
            .find(|choice| choice.value == id)
            .map(|choice| choice.value.as_str())
    }
}

impl Session {
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.gpu.config.width = width;
        self.gpu.config.height = height;
        self.gpu
            .surface
            .configure(&self.gpu.device, &self.gpu.config);
        // The depth attachment follows the *framebuffer*, not the window, and
        // `frame` resizes both together - so a resize only has to invalidate,
        // which configuring the surface above already did.
    }

    /// How long one frame is allowed to take at the least, or `None` when
    /// nothing is limiting.
    ///
    /// `None` under `vsync = "on"` alone, whatever the limit says: there the
    /// display is deciding, and a second limiter underneath it does not halve
    /// the frame rate, it beats against the refresh and turns an even 60 into
    /// an uneven one. That is why the menu greys the row out under that one
    /// value and not the other two - `off` and `smooth` both leave the loop
    /// free to run ahead, and under `smooth` the limit is the only thing
    /// stopping the GPU rendering frames that are then discarded.
    fn frame_period(&self) -> Option<std::time::Duration> {
        if self.settings.display.vsync.paces_itself() {
            return None;
        }
        self.settings.display.frame_limit.period()
    }

    /// The earliest the next frame may start, or `None` for as soon as
    /// possible.
    fn next_frame_at(&self) -> Option<std::time::Instant> {
        self.frame_period().map(|_| self.next_frame)
    }

    /// What a frame time is measured against in the overlay: the rate this
    /// build is actually trying to present at.
    ///
    /// **Not the tick rate**, which is what this used to pass and which stopped
    /// being right the moment a frame limiter existed. Against 60 Hz a 4 ms
    /// frame is a 4-pixel sliver and every column is green, so a 240-limited
    /// run - the default - drew a pacing graph that conveyed nothing. The rule
    /// line means "one target frame" and the target has to be the one in force.
    ///
    /// With vsync on the display is the target and this build cannot ask a
    /// surface what its refresh is, so it falls back to the simulation's own
    /// rate. On a 144 Hz panel that reads pessimistically - the numbers are
    /// still measured, only the graph's scale is off - and getting it right
    /// needs a refresh rate off the monitor, which is work of its own.
    fn presentation_hz(&self) -> u32 {
        self.frame_period()
            .and_then(|_| self.settings.display.frame_limit.hz())
            .unwrap_or_else(|| self.clock.rate().hz())
    }

    /// Puts the next frame on the schedule, one period after the last one was
    /// *due* rather than one period after now.
    ///
    /// **This is the difference between a 240 limit delivering 240 and
    /// delivering 220.** A timer wakes at or after its deadline, never before,
    /// and the platform's granularity is around a millisecond - which at a
    /// 4.17 ms period is a quarter of it. Measuring the next deadline from when
    /// the loop actually woke banks that overshoot into every frame and the
    /// error compounds into a systematically low frame rate; measuring it from
    /// the previous deadline puts the frames on a fixed grid, so a late wake-up
    /// is followed by an early-relative one and the *average* is the rate that
    /// was asked for.
    ///
    /// The schedule is resynchronised whenever it is more than one period away
    /// from now, in either direction: behind means the loop stalled on a load
    /// and must not pay it back as a burst of frames, ahead means the limit
    /// itself just changed and the old period is still on the clock.
    fn schedule_next_frame(&mut self, now: std::time::Instant) {
        let Some(period) = self.frame_period() else {
            self.next_frame = now;
            return;
        };
        self.next_frame += period;
        if self.next_frame < now || self.next_frame > now + period {
            self.next_frame = now + period;
        }
    }

    /// Starts `--prefetch`, now that the boot's own movies are out of the cache.
    ///
    /// **Not before.** Both convert through `ffmpeg` into the same directory,
    /// and the boot's two reels are on the worker's list too - two processes
    /// writing one file is a corrupt cache, not a race that resolves. The
    /// ordering used to come free from `boot::load` being blocking; now that
    /// the movies run on a thread, it is this call that keeps it. Taking the
    /// options is what makes it once-only.
    fn start_prefetch(&mut self) {
        let Some(options) = self.prefetch_pending.take() else {
            return;
        };
        self.prefetch = Some(prefetch::Prefetch::spawn(options));
    }

    /// How far the conversion has got, right now.
    ///
    /// A run with no worker reads as finished rather than as
    /// [`prefetch::Progress::default`], which would be "nothing done of
    /// nothing" - true, but it is `finished: false`, and a loading screen shown
    /// that would never freeze or hand the window on.
    ///
    /// **A worker that has been asked for and not yet started reads as
    /// unfinished**, which is the one case the sentence above does not cover:
    /// between the window opening and [`Self::start_prefetch`] there is no
    /// handle, and reading that as "nothing to wait for" would fade the loading
    /// screen out a moment before the conversion it exists for even began.
    fn prefetch_progress(&self) -> prefetch::Progress {
        if self.prefetch_pending.is_some() {
            return prefetch::Progress::default();
        }
        self.prefetch.as_ref().map_or(
            prefetch::Progress {
                finished: true,
                ..prefetch::Progress::default()
            },
            prefetch::Prefetch::progress,
        )
    }

    /// Hands the window to the front end once the loading screen's fade is out.
    ///
    /// **Also where the boot's two halves become one.** The movies have landed
    /// by now - the fade does not start until they have, see
    /// [`LoadingStage::media_ready`] - so joining here never actually waits,
    /// and `boot::assemble` is the tail of the load that could not run until
    /// the reels had been measured.
    ///
    /// Taking the `Shell` is what makes this idempotent: a second call finds
    /// `None` and does nothing, so a failed `Stage::frontend` cannot be retried
    /// once a frame for the rest of the run.
    fn finish_loading(&mut self) -> Result<()> {
        let (shell, mut media, trace) = match &mut self.stage {
            Stage::Loading(stage) if stage.screen.is_done() => {
                let Some(shell) = stage.shell.take() else {
                    return Ok(());
                };
                (shell, stage.media.take(), stage.trace)
            }
            _ => return Ok(()),
        };
        let media = media
            .as_mut()
            .map(boot::MediaWorker::join)
            .unwrap_or_default();
        for line in &media.report {
            println!("{line}");
        }
        let mut loaded = boot::assemble(shell, media);
        if self.boot_overlay {
            loaded.frontend.set_overlay(true);
        }
        // A language chosen on an earlier run skips the picker. Reported either
        // way: silently not asking is indistinguishable from a broken picker,
        // and silently asking again is indistinguishable from a setting that
        // did not save. Here rather than before the window, which is where it
        // used to be: the sequence this asks does not exist until the two
        // halves have met.
        match (self.pick_language, self.settings.language.as_deref()) {
            (false, Some(name)) if loaded.frontend.preselect_language(name) => {
                println!("language {name} from settings, skipping the picker");
            }
            (false, Some(name)) => {
                eprintln!("this source does not offer {name:?}, so the picker is shown");
            }
            _ => {}
        }
        for line in &loaded.report {
            println!("{line}");
        }
        // A source with no intro reel at all - which is every PS2 source, whose
        // intro is an MPEG-2 program stream outside the archives - has no video
        // format either, and the front end draws without one.
        let video_format = loaded.movie.as_ref().and_then(VideoFormat::of);

        // Taken out of the boot before the front end takes the rest: it belongs
        // to the menus, which outlive the sequence that loaded it.
        //
        // Split in two here, and this is the last place both halves are in one
        // hand: the frames move onto a decode thread and the presentation - the
        // rate, the rectangle - stays behind, because a `Feed` deals in pixels
        // and knows nothing about where they go. `repeat: true`, which is the
        // whole difference between this movie and the intro.
        //
        // **The menu's grid, which is ours and is the PSP's - not the
        // source's.** This rect is drawn by `MenuStage`'s own renderer, and
        // `open_menus` builds that one fresh and never calls `set_space`, so
        // its `screen` uniform is `Space::PSP` whatever disc is mounted. The
        // menu layout it sits behind is this project's own, authored at
        // 480x272, which is why `capture.rs` pins `Space::PSP` on the same
        // picture.
        //
        // Handing this the *source's* space instead was a regression: on a PS2
        // disc it built the rect in a 640x448 grid for a shader normalising
        // against 480x272, and `pillarbox_in` always fills one axis of the grid
        // it is given, so the backdrop overflowed the screen on every aspect.
        // Silent on the PSP, where the two grids are the same numbers, and
        // invisible to `--menu-page`, which goes through `capture.rs`.
        let space = frontend::Space::PSP;
        (self.backdrop, self.backdrop_shape) = match loaded.backdrop.take() {
            Some(movie) => {
                let shape = BackdropShape {
                    frame_rate: movie.frame_rate,
                    // Pillarboxed rather than stretched, because the PS2's cut is
                    // not the PSP's shape: an `.IPF` declares its own display
                    // aspect. The PSP's `.PMF` is already 480x272, so this is the
                    // full screen there and changes nothing.
                    rect: frontend::pillarbox_in(space, movie.display_aspect),
                };
                let (width, height) = (movie.width, movie.height);
                // No frames is no backdrop, and then there is no shape to keep
                // either: the two are `Some` and `None` together everywhere below.
                match movie.frames {
                    Some(frames) => (
                        Some(movie::Feed::spawn(frames, true, width, height)),
                        Some(shape),
                    ),
                    None => (None, None),
                }
            }
            None => (None, None),
        };
        // Building the front end spawns the intro's decode thread and uploads a
        // sprite sheet; that is a load, and a load is not a frame time.
        self.stalled = true;
        self.stage = Stage::frontend(&self.gpu, loaded, video_format, trace, &mut self.audio)?;
        Ok(())
    }

    fn frame(&mut self) -> Result<()> {
        // Before this frame's ticks, so the front end's own first frame is drawn
        // on the frame after the fade ended rather than a frame later still.
        if let Err(e) = self.finish_loading() {
            eprintln!("cannot open the front end: {e:#}");
            self.quit = true;
            return Ok(());
        }

        // Checked before this frame's ticks rather than after them, so the frame
        // that entered `Launch Game` is drawn once before the load stalls the
        // window.
        // `Launch Game` opens **our menus**, not a race. The original has a main
        // menu between the picker and a track and this build now has one too; it
        // is simply not the original's, which is why the state whose transition
        // gets us here is still spelled the way the disc spells it while what it
        // reaches is not a recovered screen at all. See `oag_game::menu`.
        if !self.launched
            && matches!(&self.stage, Stage::Frontend(stage) if stage.frontend.is_finished())
        {
            self.launched = true;
            println!("\n{}: opening the menus", frontend::states::LAUNCH_GAME);
            // Whatever the picker settled on, remembered for next time. Taken
            // here rather than in the picker because this is where the front
            // end is known to be finished with it, and because `menu.rs` and
            // `frontend.rs` both stay ignorant of where settings live.
            if let Stage::Frontend(stage) = &self.stage
                && let Some(language) = stage.frontend.chosen()
                && self.settings.language.as_deref() != Some(language)
            {
                self.settings.language = Some(language.to_string());
                if let Err(e) = settings::save(&self.settings) {
                    eprintln!("could not save the chosen language: {e:#}");
                }
            }
            if let Err(e) = self.open_menus() {
                eprintln!("cannot open the menus: {e:#}");
            } else {
                println!("\n{SHELL_KEYS}");
            }
        }

        // Fixed timestep, per ADR-0007: the simulation steps at exactly 1/60
        // whatever the window is doing. The clock's own catch-up cap is what keeps
        // the race load above from being paid back as a burst of ticks.
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        self.schedule_next_frame(now);
        // The presentation layer's only reader of the clock, and it reads the
        // same value the timestep does rather than taking its own.
        //
        // **A load is not a frame time.** Opening the menus or building a race
        // stalls the loop for a few hundred milliseconds, and that stall lands
        // in exactly one `elapsed` - the same one, whichever side of this the
        // stage change happened on, because a stage only ever changes above or
        // below here. Recording it would put one 1000 ms column across the
        // graph for the two seconds a player is most likely to be looking at
        // it, so the frame that carries a load is dropped instead.
        if self.stalled {
            self.meter.clear();
            self.stalled = false;
        } else {
            self.meter.record(elapsed.as_secs_f32());
        }
        let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        let steps = self.clock.advance(nanos);
        let dt = f64::from(self.clock.rate().dt());

        // Before the snapshot below, because it is what makes the snapshot
        // start meaning anything: `--prefetch` may not run until the boot's own
        // movies are done with the cache, so this is the frame that starts it.
        // A no-op on every frame but one, and on every run without the flag.
        if matches!(&self.stage, Stage::Loading(stage) if stage.media_ready()) {
            self.start_prefetch();
        }

        // One snapshot for the whole frame, taken outside the tick loop: it is a
        // lock and a clone, and the ticks in one frame cannot have seen the
        // worker at different points anyway.
        let progress = self.prefetch_progress();

        for _ in 0..steps {
            // Ends the devices' tick for both stages. A race reads the snapshot's
            // axes; the front end reads the button edges the same call computed,
            // through `buttons_mut`, because it needs `consume_press` and a
            // snapshot is a value.
            let snapshot = self.controls.snapshot();
            // The audio's whole tick, and it is inside this loop rather than
            // beside it on purpose. Cue emission and mixer control are driven by
            // the tick count, exactly as the exhaust and the chase camera are
            // (see `race::Race::tick`), so a headless capture and a window
            // produce the same sound at the same tick. There is deliberately no
            // per-frame counterpart: with a device attached `cpal` drains the
            // mixer from its own callback thread, and with none the offline
            // dump below is the only reader.
            //
            // The movie's playhead is read **before** that call, so that this
            // loop and `capture::run` pace the picture against the same
            // measurement - where the sound had got to at the end of the
            // previous tick - rather than differing by one tick depending on
            // which side of `tick` each happened to sit. See
            // `movie::Player::follow`.
            let movie_playhead = self.audio.movie_playhead();
            self.audio.tick();
            // The in-race camera cycle, read off the shared `Input` and consumed,
            // exactly as the front end and the menus consume their own presses -
            // a press seen on two devices is one press and there is one place to
            // clear it.
            //
            // Deliberately here and **not** inside `Race::tick`. The tick takes an
            // `InputSnapshot` by value and the selected view is not part of one, so
            // keeping the cycle outside is what makes "cycling the camera cannot
            // move a simulation bit" true by construction rather than by argument.
            // It also puts the settings file - which `Race` cannot see - in reach,
            // which is what persists the choice across a restart. Before the tick
            // rather than after, so the frame this tick produces is already drawn
            // from the new view.
            if matches!(self.stage, Stage::Race(_))
                && self.controls.buttons().is_pressed(button::SELECT)
            {
                self.controls.buttons_mut().consume_press(button::SELECT);
                self.cycle_camera_view();
            }
            match &mut self.stage {
                // Stepped in the tick loop with everything else, so the wave's
                // heartbeat runs at the simulation's fixed 60 Hz rather than at
                // whatever the window is managing. The original's own loading
                // thread ran it at 30; ours is one beat per 24 ticks either way,
                // and a frame-rate-dependent heartbeat is exactly the thing
                // ADR-0007 fixed the timestep to avoid.
                // Two waits, one screen: the boot's own movies, and the
                // `--prefetch` conversion when there is one. The fade starts
                // when both are done - `Screen::advance` holds at full opacity
                // until then - and `finish_loading` hands the window on when it
                // has run out.
                Stage::Loading(stage) => {
                    stage
                        .screen
                        .advance(progress.finished && stage.media_ready());
                }
                Stage::Frontend(stage) => {
                    let events =
                        stage
                            .frontend
                            .update(dt, self.controls.buttons_mut(), movie_playhead);
                    // One set of planes and one voice serve every movie the front
                    // end draws, so each is installed on the tick its own screen
                    // is entered - picture and sound together, which is what makes
                    // `movie_playhead` report *that* movie's position for the
                    // screen's own update to pace against.
                    //
                    // Every entered state is offered rather than one named screen:
                    // which screens play movies came out of the title's own chain,
                    // and Pure's two are neither its boot step nor the step after
                    // its picker.
                    for event in &events {
                        if let oag_game::state_machine::Event::Enter(name) = event {
                            stage.install_movie(name, &mut self.audio);
                        }
                    }
                    report(&events, stage.trace);
                    for note in stage.frontend.take_notes() {
                        println!("{note}");
                    }
                    // The movie's sound outlives neither leg, and a skip leaves
                    // the state without finishing the player - see
                    // `Frontend::is_playing_movie`.
                    //
                    if !stage.frontend.is_playing_movie() {
                        self.audio.stop_movie();
                    }
                    // The menu music's cue is **no movies left**, not "not in one
                    // right now". The two are the same question only on a title
                    // whose boot opens on its movie: Pure opens on its language
                    // picker, so "not in a movie" is true before its reel has
                    // played at all, and starting the loop there put it under the
                    // reel - which is the overlap this test was written to stop.
                    // `pending` empties as each movie is installed, so it is
                    // exactly "none still to come".
                    //
                    // Every tick rather than on the edge - the state is what is
                    // asked, not a transition - which `start_music` absorbs by
                    // being idempotent.
                    if !stage.frontend.is_playing_movie() && stage.pending.is_empty() {
                        self.audio.start_music(
                            &self.music_discs,
                            self.settings.audio.music_source,
                            &boot::default_audio_cache_dir(),
                        );
                    }
                }
                Stage::Menu(stage) => {
                    stage.tick(dt);
                    // Snapshotted *before* the input is consumed, because the
                    // page being left stops existing the moment the model
                    // moves. Compared by page id rather than by stack depth:
                    // `back` and `open` both change the page, and a jump
                    // between two pages at the same depth is still a change.
                    let before = stage.menu.page().id.clone();
                    let leaving =
                        menu::draw_list(&stage.menu, &stage.skin, &keys::bound_keys, None);
                    let events = stage.menu.update(self.controls.buttons_mut());
                    if stage.menu.page().id != before {
                        stage.begin_change(leaving);
                    }
                    for event in events {
                        self.handle_menu(&event);
                    }
                }
                Stage::Race(stage) => {
                    stage.race.tick(&snapshot);
                    if self.log_every > 0 && stage.race.world.tick % u64::from(self.log_every) == 0
                    {
                        println!("{}", race::describe(&stage.race.telemetry()));
                    }
                }
            }
        }

        let frame = match self.gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.gpu
                    .surface
                    .configure(&self.gpu.device, &self.gpu.config);
                return Ok(());
            }
            other => {
                eprintln!("skipping frame: {other:?}");
                return Ok(());
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        // Where the game goes on the surface, and how many pixels it is drawn
        // with. One rectangle for every stage, so a change moves the whole game
        // rather than only what happens to be on screen.
        let rect = display::viewport(self.gpu.size(), self.settings.display.aspect);
        let wanted = upscale::target_size(
            rect,
            self.settings.graphics.render_scale,
            self.gpu.device.limits().max_texture_dimension_2d,
        );
        if self.framebuffer.resize(&self.gpu.device, wanted) {
            // A depth attachment whose size does not match the colour one is a
            // validation error, so the race's has to follow.
            if let Stage::Race(stage) = &mut self.stage {
                stage.scene.resize(
                    &self.gpu.device,
                    self.gpu.config.format,
                    self.framebuffer.size(),
                );
            }
        }

        // Each stage fills the target, and the target *is* the game's
        // rectangle: the bars are the surface the blit does not cover.
        let size = self.framebuffer.size();
        let inside = (0.0, 0.0, size.0 as f32, size.1 as f32);
        let target = self.framebuffer.view();
        let (scene_stats, video_label) = match &mut self.stage {
            Stage::Loading(stage) => {
                stage.render(&self.gpu, &mut encoder, target, inside, &progress);
                (None, None)
            }
            Stage::Frontend(stage) => {
                // The same feed the menus will borrow, and it is alive from the
                // session opening rather than from the menus opening - so the
                // backdrop under `Show Logo` is the one already looping, not a
                // second decoder.
                stage.render(
                    &self.gpu,
                    &mut encoder,
                    target,
                    inside,
                    self.backdrop.as_mut(),
                )?;
                (None, stage.video_label())
            }
            Stage::Menu(stage) => {
                stage.render(
                    &self.gpu,
                    &mut encoder,
                    target,
                    inside,
                    self.backdrop.as_mut(),
                )?;
                (None, self.backdrop.as_ref().map(movie::Feed::decoder_label))
            }
            Stage::Race(stage) => (
                Some(stage.render(
                    &self.gpu,
                    &mut encoder,
                    target,
                    inside,
                    self.settings.graphics.fov,
                    self.settings.graphics.frustum_culling,
                    self.settings.graphics.pvs_culling,
                    self.settings.graphics.animated_textures,
                )),
                None,
            ),
        };

        // Over the stage and inside the offscreen target, so the overlay is
        // drawn at the render scale the game is - measuring a frame nobody is
        // presenting would be the one way to get this wrong. One pass, skipped
        // entirely when the setting is off.
        let list = perf::draw_list(
            &self.meter,
            self.settings.graphics.perf_overlay,
            self.presentation_hz(),
            scene_stats,
            video_label,
        );
        if !list.is_empty() {
            self.overlay.overlay(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                target,
                &list,
                inside,
            );
        }

        // The whole back half of the frame - the upscaler, the grade and the
        // blit - so that this and a `--presented` capture cannot drift apart.
        self.framebuffer.resolve(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &view,
            rect,
            &upscale::Presentation {
                upscaler: self.settings.graphics.upscaler,
                sharpness: self.settings.graphics.upscale_sharpness.stops(),
                anti_aliasing: self.settings.graphics.anti_aliasing,
                brightness: self.settings.display.brightness,
                gamma: self.settings.display.gamma,
            },
        );
        self.gpu.queue.submit(Some(encoder.finish()));
        self.gpu.queue.present(frame);
        Ok(())
    }

    /// Replaces the front end with the menus, seeded from the settings.
    ///
    /// The renderer moves across rather than being rebuilt - it holds the disc's
    /// font atlas and sprite sheet, already uploaded, and a menu row is text and
    /// a rectangle. A stage that is not the front end leaves the menus where
    /// they are, which is what makes this safe to call from the frame loop.
    fn open_menus(&mut self) -> Result<()> {
        let shell = self
            .shell
            .clone()
            .context("this run has no menus: nothing loaded a font or a sprite sheet")?;
        // Everything below this is a load, and a load is not a frame time.
        self.stalled = true;
        let mut model = menu::Menu::new(shell.definition.clone());
        // The skin decides the row pitch, and the pitch decides how many rows a
        // page shows - so the window has to be told before anything scrolls.
        // The face the rows will be drawn in, which is also the face whose
        // line height sets the pitch - so the two are chosen together or the
        // rows would be spaced for a font they are not drawn in.
        let rows_face = shell
            .menu_font
            .clone()
            .unwrap_or_else(|| shell.font.clone());
        let skin = menu::Skin::new(shell.menu_skin, rows_face.line_height);
        model.set_visible_rows(menu::visible_rows(&skin));
        // Supplied before seeding, because a value cannot be seeded onto a list
        // that is not there yet.
        let tracks: Vec<menu::Choice> = shell
            .tracks
            .iter()
            .map(|(track, name)| menu::Choice::labelled(&track.id, name))
            .collect();
        model.supply(menu::ValueSource::Tracks, &tracks);
        model.supply(menu::ValueSource::Teams, &shell.teams);
        model.supply(menu::ValueSource::Languages, &shell.languages);
        model.supply(menu::ValueSource::RaceModes, &shell.modes);
        // Enumerated every time the menus open rather than kept from startup,
        // because a screen can be plugged in while the game is running and the
        // row should show it without a restart.
        let monitors: Vec<menu::Choice> = display::Monitor::offered(&monitor_names(
            &self.gpu.window.available_monitors().collect::<Vec<_>>(),
        ))
        .into_iter()
        .map(menu::Choice::plain)
        .collect();
        model.supply(menu::ValueSource::Monitors, &monitors);
        // Kept from startup rather than enumerated, unlike the monitors above,
        // and the difference is what a fresh answer would be worth. A screen
        // plugged in now can be used now; an adapter plugged in now cannot,
        // because the device was made at boot. Listing one would be offering a
        // row that does nothing this run.
        let renderers: Vec<menu::Choice> = display::Renderer::offered(&self.gpu.adapters)
            .into_iter()
            .map(menu::Choice::plain)
            .collect();
        model.supply(menu::ValueSource::Renderers, &renderers);
        // Empty unless this machine has both Pulse discs, which draws the row
        // unusable rather than offering a swap that cannot happen. Kept from
        // the boot survey rather than re-derived here: answering it means
        // opening every image on the search path, and a menu opening is not
        // the moment for that. See `audio::MusicDiscs`.
        let music_sources: Vec<menu::Choice> = if self.music_discs.both() {
            audio::MusicSource::ALL
                .iter()
                .map(|source| menu::Choice::plain(source.name()))
                .collect()
        } else {
            Vec::new()
        };
        model.supply(menu::ValueSource::MusicSources, &music_sources);
        self.seed_menu(&mut model);
        // What the row is set to comes from the settings file, above; what the
        // game is *drawing with* can only come from here, and the RENDERER row's
        // restart note is the difference between the two. Told every time the
        // menus open rather than once, because the model is new every time.
        let in_use: Vec<menu::Value> = self
            .gpu
            .in_use
            .iter()
            .map(|name| menu::Value::Text(name.clone()))
            .collect();
        if !model.in_effect("graphics.renderer", &in_use) {
            eprintln!("note: nothing in the menus defers graphics.renderer");
        }
        // What the row is set to comes from the settings file; what a race on
        // screen is actually *drawing* with can only come from its own
        // `Scene`, built once when that race started. Silent on the front end
        // and the menus, where no race exists yet to compare against - see
        // `Restart::in_effect`.
        //
        // **Not a single value.** Unlike the renderer, only the MSAA half of
        // this row is baked into the scene's pipelines - `upscale::Framebuffer::resolve`
        // reads FXAA/SMAA/off fresh every frame, the same way it already
        // reads the upscaler. So every mode that shares the built scene's
        // sample count is equally "in effect": a race built at `off` can move
        // to `fxaa` or back with no restart note, and only moving to or
        // between an MSAA tier the scene was not built with earns one.
        if let Stage::Race(stage) = &self.stage {
            let built = stage.scene.anti_aliasing();
            let live_equivalent: &[display::AntiAliasing] = if built.msaa_samples() == 1 {
                &[
                    display::AntiAliasing::Off,
                    display::AntiAliasing::Fxaa,
                    display::AntiAliasing::Smaa,
                ]
            } else {
                std::slice::from_ref(&built)
            };
            let in_use: Vec<menu::Value> = live_equivalent
                .iter()
                .map(|mode| menu::Value::Text(mode.to_string()))
                .collect();
            if !model.in_effect("graphics.anti_aliasing", &in_use) {
                eprintln!("note: nothing in the menus defers graphics.anti_aliasing");
            }
            // Same story as MSAA above, but a single tier baked in at
            // `Race::start` rather than a range of equivalent modes.
            let in_use = [menu::Value::Text(stage.race.boost_fov_kick().to_string())];
            if !model.in_effect("graphics.boost_fov_kick", &in_use) {
                eprintln!("note: nothing in the menus defers graphics.boost_fov_kick");
            }
        }

        // **The movie planes are asked for only when there is a movie to put in
        // them.** This used to be unconditionally `None`, on the grounds that
        // wanting them would tie the menus to a stage that had played one; that
        // reasoning still holds and this does not break it. What the menus are
        // tied to is a *movie*, handed to them by whoever built the session, and
        // it is optional - the menus open with no backdrop on a source that has
        // none and on `--no-video`, and draw on black there.
        //
        // **Asked of the feed, not of a `Movie`.** The frames moved onto a decode
        // thread when the session was built, so a `Movie`'s `frames` is `None`
        // by now and testing it would build a renderer with no video pipeline and
        // draw the menus on black - with no error anywhere, and with
        // `--menu-page` still looking right, because that flag goes through the
        // headless capture path and its movie is a different one that kept its
        // frames. See `VideoFormat::of_feed`.
        //
        // **The playhead is carried over the front end's shoulder, not built
        // fresh.** See `menu_playhead`: coming out of the boot sequence there is
        // already one running, and rebuilding it at frame zero is what made the
        // picture jump back to the start of the loop the instant START was
        // pressed. The feed is only restarted when there is nothing to carry -
        // the two share an origin already, and restarting one of them is exactly
        // what would put them at odds.
        let shape = self.backdrop_shape.filter(|_| self.backdrop.is_some());
        let format = self.backdrop.as_ref().map(VideoFormat::of_feed);
        let frames = self.backdrop.as_ref().map_or(0, movie::Feed::len);
        let renderer = Renderer::new(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.config.format,
            format,
            rows_face,
            &shell.sprites,
        )?;
        // **The picture moves across as well as the playhead**, and it has to,
        // because the renderer does not. A fresh `Renderer` is fresh planes:
        // zeroed, which is green rather than black, so the first menu frame
        // would draw its rows on the black fill instead and the picture would
        // only appear once the playhead reached the next decoded frame. That is
        // one to three frames of menu-on-black on every boot - the flicker on
        // the START press. Seeding the planes here closes it, and `shown` is
        // seeded to match so the draw list names the frame that is really in
        // them. `escape` -> menus carries nothing, as it carries no playhead:
        // that path restarts the loop deliberately, see `menu_playhead`.
        let seed = match &self.stage {
            Stage::Frontend(stage) => stage
                .held_backdrop
                .as_ref()
                .map(|held| (held.index, held.picture.clone())),
            _ => None,
        };
        let seeded = match &seed {
            Some((index, picture)) => {
                renderer.upload_frame(&self.gpu.queue, picture)?;
                Some(*index)
            }
            None => None,
        };
        // Past the last fallible step, so a renderer that could not be built
        // leaves the front end holding its own backdrop rather than stripped of
        // one it is still drawing.
        let carried = match &mut self.stage {
            Stage::Frontend(stage) => stage.frontend.take_backdrop(),
            _ => None,
        };
        if carried.is_none()
            && let Some(feed) = self.backdrop.as_mut()
        {
            feed.restart();
        }
        self.stage = Stage::Menu(Box::new(MenuStage {
            renderer,
            menu: model,
            skin,
            // Opening the menus is not a page change: the front end's own
            // hand-off already had its moment, and starting a transition here
            // would zoom the first page in from nothing on every boot.
            change: None,
            backdrop: shape.map(|shape| Backdrop {
                player: menu_playhead(carried, frames, shape.frame_rate),
                rect: shape.rect,
                shown: seeded,
            }),
        }));
        self.gpu.window.set_title(SHELL_TITLE);
        Ok(())
    }

    /// Puts every setting the menus can edit onto the row that edits it.
    ///
    /// A key nothing edits is reported rather than ignored: it means a setting
    /// exists that a player has no way to change, which is a gap worth seeing in
    /// the log rather than a silent one.
    fn seed_menu(&self, model: &mut menu::Menu) {
        for (key, value) in settings::menu_seeds(&self.settings, self.anisotropy) {
            if !model.seed(key, &value) {
                eprintln!("note: nothing in the menus edits {key}");
            }
        }
    }

    /// Acts on one thing the menus did.
    fn handle_menu(&mut self, event: &menu::MenuEvent) {
        match event {
            menu::MenuEvent::Changed { setting, value } => self.apply_setting(setting, value),
            menu::MenuEvent::Fired(menu::Action::LaunchRace) => {
                // The stored id names a *race*, and only this source can say
                // which file that is. A source that no longer offers it keeps
                // whatever track the options already held rather than guessing
                // a path, which would fail at the archive with a message about
                // a missing entry instead of about a missing circuit.
                match self
                    .shell
                    .as_ref()
                    .and_then(|shell| shell.track(&self.settings.race.track))
                {
                    Some(track) => self.race_options.track = track.entry_name(),
                    None => eprintln!(
                        "this source does not offer {:?}, racing {} instead",
                        self.settings.race.track, self.race_options.track
                    ),
                }
                // The same shape as the circuit above, and needed for the same
                // reason now that the roster is what this source offers rather
                // than a fixed list: a stored team can be one only a
                // downloadable pack carries, and a boot that did not find the
                // pack drops it. `Menu::supply` resets the row silently when
                // that happens, so without this check the menu would show one
                // team and the race would attempt another - failing at the
                // archive with a message naming a team that is not on screen.
                match self
                    .shell
                    .as_ref()
                    .and_then(|shell| shell.team(&self.settings.race.team))
                {
                    Some(team) => self.race_options.team = team.to_string(),
                    None => eprintln!(
                        "this source does not offer team {:?}, racing as {} instead",
                        self.settings.race.team, self.race_options.team
                    ),
                }
                if let Some(class) = SpeedClass::from_name(&self.settings.race.class) {
                    self.race_options.class = class;
                }
                // Same shape as the speed class above: an unrecognised token
                // leaves the previous mode in place rather than substituting
                // one, so a settings file from a build with a mode this one
                // does not have still races.
                if let Some(mode) = oag_race::Mode::from_name(&self.settings.race.mode) {
                    self.race_options.mode = mode;
                }
                println!("\nloading {}", self.race_options.track);
                match self.launch_race() {
                    Ok(()) => println!("\n{RACE_KEYS}{ESC_TO_MENU}"),
                    // Reported rather than fatal: leaving the menus on screen
                    // lets the player pick something else, where a vanished
                    // window would just look like a crash.
                    Err(e) => eprintln!("cannot start a race: {e:#}"),
                }
            }
            // Backing out of the root page means the same thing as choosing
            // QUIT: there is nothing behind the menus to go back to.
            menu::MenuEvent::Fired(menu::Action::Quit) | menu::MenuEvent::Closed => {
                self.quit = true;
            }
        }
    }

    /// What escape does, which is **back one level** and not quit.
    ///
    /// One rule, three places it lands:
    ///
    /// - in the menus, it pops a page, exactly as circle does. On the root page
    ///   `Menu::back` raises `Closed`, which already means "there is nothing
    ///   behind the menus", so the last one still quits;
    /// - in a race, it hands the window back to the menus;
    /// - in the front end, or in a `--race` run that never had menus, there is
    ///   no level behind and it quits.
    ///
    /// **Leaving a race discards it.** There is no pause and no resume - the
    /// `World` is dropped and re-entering the race loads a fresh one - and
    /// building that is a milestone of its own, not something to half-do here.
    /// A player who backs out of a race expects to lose it; one who backs out
    /// and finds a *stale* race would not.
    fn escape(&mut self) {
        // Collected before anything else touches `self`: `handle_menu` takes
        // `&mut self` and the events borrow the stage.
        if let Stage::Menu(stage) = &mut self.stage {
            let events = stage.menu.back();
            for event in events {
                self.handle_menu(&event);
            }
            return;
        }

        if matches!(self.stage, Stage::Race(_)) && self.shell.is_some() {
            println!("\nleaving the race");
            // Before `open_menus`, not after: the menu voice this resumes has
            // to be sounding by the time the menus themselves draw. See
            // `Audio::pause_race_music`.
            self.audio.pause_race_music();
            match self.open_menus() {
                Ok(()) => println!("\n{SHELL_KEYS}"),
                // Reported rather than fatal, and then it quits: a race whose
                // menus cannot be rebuilt has nothing left to offer, but a
                // window that vanished with no message would read as a crash.
                Err(e) => {
                    eprintln!("cannot return to the menus: {e:#}");
                    self.quit = true;
                }
            }
            return;
        }

        self.quit = true;
    }

    /// Puts the window onto the monitor, and into the mode and size, that the
    /// settings now hold.
    ///
    /// Immediately, unlike anisotropy and the language: a player who picks
    /// borderless and sees nothing happen will assume it is broken. The resize
    /// event the compositor sends back is what reconfigures the surface, so
    /// nothing here touches it.
    ///
    /// The size and the position are asked for only in windowed mode, and
    /// **every request here may be refused** - a compositor is allowed to
    /// ignore all three, and a tiling one will ignore at least two. Nothing
    /// depends on any of them being honoured.
    fn apply_window(&mut self) {
        let wanted = self.settings.display.clone();
        let monitor = choose_monitor(
            self.gpu.window.available_monitors().collect(),
            &wanted.monitor,
        );
        self.gpu
            .window
            .set_fullscreen(fullscreen(wanted.window_mode, monitor.clone()));
        let windowed = wanted.window_mode == display::WindowMode::Windowed;
        self.gpu.window.set_resizable(!windowed);
        if windowed {
            let size = wanted.window_size;
            let _ = self
                .gpu
                .window
                .request_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
            // Moved before it is resized as far as the compositor is concerned,
            // both being requests it may reorder or drop; the position is
            // computed from the size the settings hold rather than the one the
            // window currently has, so the two do not have to agree yet.
            if let Some(monitor) = &monitor {
                self.gpu
                    .window
                    .set_outer_position(centred_on(monitor, size));
            }
        }
    }

    /// Applies a changed setting and writes it back.
    ///
    /// Persisted on every keypress rather than on the way out, because there is
    /// no way out that is guaranteed to run: a player quits with the window
    /// button as often as with the menu. The file is a few hundred bytes.
    fn apply_setting(&mut self, setting: &str, value: &menu::Value) {
        let text = value.to_string();
        match setting {
            // `display.monitor` never fails to parse - a monitor name is
            // whatever the platform says it is - so a name this machine does
            // not have is reported by `choose_monitor` when the window is
            // moved, not here. See `display::Monitor`.
            "display.monitor" => {
                self.settings.display.monitor = display::Monitor::from(text);
                self.apply_window();
            }
            "display.window_mode" => match text.parse::<display::WindowMode>() {
                Ok(mode) => {
                    self.settings.display.window_mode = mode;
                    self.apply_window();
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.window_size" => match text.parse::<display::Size>() {
                Ok(size) => {
                    self.settings.display.window_size = size;
                    self.apply_window();
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.aspect" => match text.parse::<display::Aspect>() {
                // Applied by the next frame, because every stage takes its
                // viewport from this on the way into its pass.
                Ok(aspect) => self.settings.display.aspect = aspect,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Both applied by the next frame: the blit pass reads them on its
            // way onto the surface, so a change shows on whatever is on screen
            // - including the menu the player is standing on, which is the
            // point of putting them there rather than behind a race.
            "display.brightness" => match text.parse::<display::Brightness>() {
                Ok(brightness) => self.settings.display.brightness = brightness,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.gamma" => match text.parse::<display::Gamma>() {
                Ok(gamma) => self.settings.display.gamma = gamma,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied on the spot rather than by the next frame: a bus gain is
            // read by whatever the mixer renders next, which on a device is
            // already in flight. That is what makes the row audible while the
            // player is standing on it, the way the two above are visible.
            "audio.music_volume" => match text.parse::<audio::Volume>() {
                Ok(volume) => {
                    self.settings.audio.music_volume = volume;
                    self.audio.apply(&self.settings.audio);
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied on the spot, and **seeked rather than restarted**: the
            // two releases' encodes of a track agree in length to 11 ms, so
            // carrying the playhead across lands in the same bar. The first
            // move onto a release reads it off its disc - measured at 2.0 s
            // for the PS2's 36 MiB of PCM and 0.4 s for the PSP's cached
            // decode - and every move after that is instant, because the sound
            // is held. See `audio::Audio::set_music_source`.
            "audio.music_source" => match text.parse::<audio::MusicSource>() {
                Ok(source) => {
                    self.settings.audio.music_source = source;
                    self.audio.set_music_source(
                        &self.music_discs,
                        source,
                        &oag_game::boot::default_audio_cache_dir(),
                    );
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // **Applied on the next launch**, and the only setting here that
            // cannot be applied at all this run: the device, every pipeline and
            // every uploaded mesh hang off the adapter chosen at boot, so
            // switching would mean tearing down the surface, the framebuffer
            // and whatever stage is on screen. Stored now, drawn with next time.
            // `Gpu::new` retries on the default if this one will not start, so
            // choosing an adapter that cannot serve is recoverable from inside
            // the game rather than only from the settings file.
            "graphics.renderer" => self.settings.graphics.renderer = display::Renderer::from(text),
            "graphics.render_scale" => match text.parse::<display::Scale>() {
                // Applied by the next frame: `frame` sizes the target from this
                // every time and rebuilds it when the answer changes.
                Ok(scale) => self.settings.graphics.render_scale = scale,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.upscaler" => match text.parse::<display::Upscaler>() {
                // Applied by the next frame. Choosing `fsr1` for the first time
                // builds its two pipelines inside that frame, which is a shader
                // compilation a player may notice once and never again.
                Ok(upscaler) => self.settings.graphics.upscaler = upscaler,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.upscale_sharpness" => match text.parse::<display::Sharpness>() {
                // Applied by the next frame: it is one float in the upscaler's
                // uniform, rewritten only when it moves.
                Ok(sharpness) => self.settings.graphics.upscale_sharpness = sharpness,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // `off`, `fxaa` and `smaa` are applied by the next frame, the
            // same as the upscaler: `Framebuffer::resolve` reads this fresh
            // and builds FXAA's pipeline lazily, the same way it does FSR 1's.
            // **Only moving to or between the two MSAA levels waits for the
            // next race**, because that is what rebuilds the scene pipelines
            // MSAA's sample count is baked into - see `race::Scene::new`. The
            // row's restart note distinguishes the two cases; see
            // `Session::open_menus`.
            "graphics.anti_aliasing" => match text.parse::<display::AntiAliasing>() {
                Ok(mode) => self.settings.graphics.anti_aliasing = mode,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.fov" => match text.parse::<display::Fov>() {
                // Applied by the next frame the race draws, which builds its
                // projection from this every time. Nothing else uses it: the
                // front end and the menus are drawn flat.
                Ok(fov) => self.settings.graphics.fov = fov,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.perf_overlay" => match text.parse::<perf::Overlay>() {
                // Applied by the next frame, which draws it or does not.
                Ok(mode) => self.settings.graphics.perf_overlay = mode,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.vsync" => match text.parse::<perf::Vsync>() {
                Ok(vsync) => {
                    self.settings.display.vsync = vsync;
                    // Applied immediately, by reconfiguring the surface. A
                    // player who turns vsync off and sees nothing change will
                    // assume it is broken, and the frame limiter that comes
                    // with it would then look broken too.
                    self.gpu.set_vsync(vsync);
                    // The frames either side of a surface reconfigure are not
                    // frames anyone is going to present at that rate.
                    self.stalled = true;
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.frame_limit" => match text.parse::<perf::FrameLimit>() {
                // Applied by the next `about_to_wait`, which is what waits.
                Ok(limit) => self.settings.display.frame_limit = limit,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.anisotropy" => match text.parse::<Anisotropy>() {
                Ok(level) => {
                    self.anisotropy = level;
                    self.settings.graphics.anisotropy = level;
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Baked into the race at `Race::start` (see `Self::race`), so a
            // change here has no effect on the one already running - the same
            // as `graphics.anti_aliasing`'s MSAA levels above.
            "graphics.boost_fov_kick" => match text.parse::<display::BoostFovKick>() {
                Ok(kick) => self.settings.graphics.boost_fov_kick = kick,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied to the race already running, unlike the two rows above:
            // the original binds this to a button precisely so it can be changed
            // while flying, and a row that deferred it to the next race would be
            // the odd one out rather than the careful one. Nothing it touches is
            // simulation state - see `race::Race::set_camera_view`.
            "graphics.camera_view" => match text.parse::<display::CameraView>() {
                Ok(view) => {
                    self.settings.graphics.camera_view = view;
                    if let Stage::Race(stage) = &mut self.stage {
                        stage.race.set_camera_view(view);
                    }
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // The scheme the *next* race starts with. Not applied to a race
            // already running: `Race::set_control_scheme` is called once before
            // the first tick, and swapping mid-race would leave a half-finished
            // gesture armed in `ShipState`.
            "controls.scheme" => match text.parse::<ControlScheme>() {
                Ok(scheme) => {
                    self.settings.controls.scheme = scheme.name().to_string();
                    self.scheme = scheme;
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "race.mode" => self.settings.race.mode = text,
            "race.class" => self.settings.race.class = text,
            "race.team" => self.settings.race.team = text,
            "race.track" => self.settings.race.track = text,
            "language" => self.settings.language = Some(text),
            other => {
                eprintln!("note: nothing applies {other}");
                return;
            }
        }
        if let Err(e) = settings::save(&self.settings) {
            eprintln!("could not save settings: {e:#}");
        }
    }

    /// Moves the in-race camera on one perspective and persists the choice.
    ///
    /// **This is the original's SELECT.** `Camera_UpdatePlayerView`
    /// (`0x0883c0cc`) tests abstract button index `0xf`, consumes the press,
    /// rotates its profile setting through three values and sets the profile's
    /// dirty byte - so the write back to the settings file here is a reproduction
    /// and not a convenience. Confidence **88** for the cycle and its order; see
    /// `docs/ghidra/functions/psp-pulse-usa/camera.md`.
    ///
    /// The order lives on [`display::CameraView::next`], so this function decides
    /// nothing about it: the button, the `CAMERA VIEW` menu row and the type's own
    /// test all walk one sequence.
    ///
    /// Saving on every press is deliberate. The alternative - saving on exit -
    /// loses the choice to a crash or a `kill`, and the file is a few hundred
    /// bytes written at most once per press of one button.
    fn cycle_camera_view(&mut self) {
        let next = self.settings.graphics.camera_view.next();
        self.settings.graphics.camera_view = next;
        if let Stage::Race(stage) = &mut self.stage {
            stage.race.set_camera_view(next);
        }
        // Nothing re-seeds the menu here: `settings::menu_seeds` reads
        // `self.settings` when the page opens, so the `CAMERA VIEW` row already
        // opens on whatever the player last flew with.
        if let Err(e) = settings::save(&self.settings) {
            eprintln!("could not save settings: {e:#}");
        }
    }

    /// Replaces whatever is on screen with the race the menus ask for.
    ///
    /// The window, the device and the surface are the ones already open, so the
    /// handoff costs a load and not a second window.
    fn launch_race(&mut self) -> Result<()> {
        self.stalled = true;
        let loaded = race::load(&self.race_options)?;
        for line in &loaded.report {
            println!("{line}");
        }
        self.stage = Stage::race(
            &self.gpu,
            loaded,
            self.framebuffer.size(),
            self.anisotropy,
            &self.settings,
            self.scheme,
        )?;
        // After the stage swap succeeds, not before: both loads above can fail
        // with `?`, and a failed launch must leave the menu music playing
        // rather than having already silenced it. See `Audio::start_race_music`.
        self.audio.start_race_music(
            &self.music_discs,
            self.settings.audio.music_source,
            &boot::default_audio_cache_dir(),
        );
        self.gpu.window.set_title(RACE_TITLE);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-row capture in exactly the shape `psp-trace.py --camera` writes:
    /// the required columns plus the all-or-nothing camera group. Hand-authored
    /// like `oag_trace::trace`'s own fixtures - a real capture cannot be
    /// committed.
    const FIXTURE: &str = "\
tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,\
cam_right_x,cam_right_y,cam_right_z,cam_up_x,cam_up_y,cam_up_z,\
cam_fwd_x,cam_fwd_y,cam_fwd_z,cam_pos_x,cam_pos_y,cam_pos_z
5,0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22,1,0,0,0,1,0,0,0,1,10,8,-45
6,0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,10,2.5,-29.6,0,0,22,22,1,0,0,0,1,0,0,0,1,10,8,-44.6
";

    fn fixture_file(name: &str, text: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("oag-game-pose-from-{name}.csv"));
        std::fs::write(&path, text).expect("a writable temp dir");
        path
    }

    #[test]
    fn a_pose_from_a_trace_row_is_exact_and_carries_the_camera() {
        let path = fixture_file("carries-camera", FIXTURE);
        let (pose, camera) = pose_from_trace(&path, 5, false, Some(70.0)).expect("the fixture row");
        let race::PoseRequest::Exact(pose) = pose else {
            panic!("--pose-from must not re-derive the pose from the spline");
        };
        assert_eq!(pose.position, oag_core::math::Vec3::new(10.0, 2.5, -30.0));
        let camera = camera.expect("the fixture has camera columns");
        assert_eq!(camera.eye, oag_core::math::Vec3::new(10.0, 8.0, -45.0));
        assert_eq!(camera.fov_deg, Some(70.0));
    }

    #[test]
    fn no_camera_keeps_the_chase_camera_even_when_the_capture_has_one() {
        let path = fixture_file("no-camera", FIXTURE);
        let (_, camera) = pose_from_trace(&path, 5, true, None).expect("the fixture row");
        assert!(camera.is_none());
    }

    /// One frame of the PSP's backdrop, to the nanosecond.
    const BACKDROP_FRAME: f64 = 1001.0 / 30_000.0;

    #[test]
    fn the_menus_open_on_the_playhead_the_front_end_was_running() {
        // The bug this is here for: the boot sequence handed over a backdrop
        // that was 400 frames into its loop and the menus started a new one at
        // zero, so the picture jumped back to the start of the movie the instant
        // START was pressed.
        let mut running = movie::Player::new(270, true, movie::FRAME_RATE);
        for _ in 0..400 {
            running.update(BACKDROP_FRAME);
        }
        assert_eq!(running.position(), 400);

        let opened = menu_playhead(Some(running), 270, movie::FRAME_RATE);
        assert_eq!(
            opened.position(),
            400,
            "the menus continue the playback rather than restarting it"
        );
        assert_eq!(opened.frame(), 400 % 270, "and it is mid-loop, not at zero");
        assert!(!opened.is_finished());
    }

    #[test]
    fn a_playhead_carried_across_keeps_running_from_where_it_was() {
        // Not just the position at the handoff: the next frame after it has to
        // be the next frame of the same playback, wrap included.
        let mut running = movie::Player::new(270, true, movie::FRAME_RATE);
        for _ in 0..269 {
            running.update(BACKDROP_FRAME);
        }
        let mut opened = menu_playhead(Some(running), 270, movie::FRAME_RATE);
        opened.update(BACKDROP_FRAME);
        assert_eq!(opened.position(), 270);
        assert_eq!(opened.frame(), 0, "it wraps rather than ending");
    }

    #[test]
    fn with_nothing_to_carry_the_menus_start_the_loop_themselves() {
        // Leaving a race: the stage that owned the playhead is gone, and
        // `open_menus` restarts the feed to match this. See `menu_playhead`.
        let fresh = menu_playhead(None, 270, movie::FRAME_RATE);
        assert_eq!(fresh.position(), 0);
        assert_eq!(fresh.frames(), 270);
        assert!(!fresh.is_finished(), "270 frames of loop are not an ending");
    }

    /// The two figures, and the three fields that follow from them.
    #[test]
    fn a_stated_conversion_state_is_read_as_the_worker_would_report_it() {
        let midway = parse_progress("37/115").expect("37 of 115");
        assert!(!midway.planning, "a stated total means planning is over");
        assert!(!midway.finished);
        assert_eq!((midway.done, midway.total), (37, 115));
        assert!(midway.current.is_some(), "something is converting");

        let done = parse_progress("115/115").expect("all of them");
        assert!(done.finished);
        assert_eq!(done.fraction(), 1.0);
        assert_eq!(done.current, None, "nothing is converting any more");
    }

    /// Every rejection names what to write instead, because the flag is typed
    /// by hand and the shape is not guessable.
    #[test]
    fn a_state_that_is_not_two_numbers_is_refused() {
        for spec in ["37", "37/", "a/b", "37 115", ""] {
            let error = parse_progress(spec).expect_err("{spec} is not a state");
            assert!(error.to_string().contains("DONE/TOTAL"), "{spec}: {error}");
        }
        let error = parse_progress("200/115").expect_err("more than all of them");
        assert!(error.to_string().contains("more than all"), "{error}");
    }

    #[test]
    fn a_missing_tick_is_an_error_that_names_the_range() {
        let path = fixture_file("missing-tick", FIXTURE);
        let error = pose_from_trace(&path, 99, false, None).expect_err("tick 99 is not there");
        assert!(error.to_string().contains("no tick 99"), "{error}");
    }
}
