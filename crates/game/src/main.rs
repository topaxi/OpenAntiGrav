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
    adapter, boot, capture, catalogue, display, menu, movie, perf, race, report, settings, source,
    upscale,
};
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
    /// `oag_assets::pulse::Layout`.
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

    /// Render one frame to a PNG and exit, without opening a window.
    #[arg(long)]
    screenshot: Option<std::path::PathBuf>,

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
    #[arg(long)]
    press: Option<String>,

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
    #[arg(long, default_value = race::DEFAULT_TEAM)]
    team: String,

    /// The speed class: venom, flash, rapier or phantom.
    #[arg(long, default_value = "venom")]
    class: String,

    /// The race mode: time_trial, speed_lap or zone.
    ///
    /// Needed on the `--race` path in particular, which skips the menus and so
    /// has no other way to pick one.
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
            "{:?} is not a race mode; try time_trial, speed_lap or zone",
            cli.mode
        )
    })?;
    // Resolved once, before anything opens it: both ways in need a source, and
    // "no disc image found" is a message about the command line, not something to
    // discover eight seconds of intro later.
    let source = source::resolve(cli.source.as_deref(), settings.source.image.as_deref())?;

    let race_options = race::Options {
        source: source.clone(),
        track: cli.track.clone(),
        team: cli.team.clone(),
        class,
        mode,
        ribbon: cli.ribbon,
        collision: cli.collision,
        lod: cli.lod.unwrap_or(settings.graphics.lod),
        pose: cli.pose.as_deref().map(parse_pose).transpose()?,
    };

    // Before `boot::load`, deliberately: the front end's load parses the front-end
    // XML, every language plugin and a string table, and may shell out to `ffmpeg`
    // to transcode the intro. Going straight to a race needs none of it.
    if cli.race {
        return run_race(&cli, race_options, &settings, anisotropy);
    }

    let leg = if cli.reel {
        frontend::Leg::DevPubReel
    } else {
        frontend::Leg::LogoFmv
    };
    let options = boot::Options {
        source: source.clone(),
        // The string table follows the saved language, so a player who picked
        // French once reads French from the next boot rather than only having
        // the picker skipped.
        language: settings.language.clone(),
        leg,
        movie: cli.movie.clone().unwrap_or_else(|| {
            match leg {
                frontend::Leg::LogoFmv => boot::DEFAULT_BOOT_MOVIE,
                frontend::Leg::DevPubReel => boot::DEVPUB_REEL,
            }
            .to_string()
        }),
        cache: cli.cache.clone().unwrap_or_else(boot::default_cache_dir),
        // Every frame by default: the movie the disc plays is 1200 frames long
        // and its last one is the Wipeout Pulse logo, so a cap would stop the
        // sequence before the thing it exists to show.
        extent: cli
            .movie_frames
            .map_or(movie::Extent::Whole, movie::Extent::Frames),
        no_video: cli.no_video,
    };

    let mut loaded = boot::load(&options)?;
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

    // A source with no intro reel at all - which is every PS2 source, whose
    // intro is an MPEG-2 program stream outside the archives - has no video
    // format either, and the front end draws without one.
    let video_format = loaded.movie.as_ref().and_then(VideoFormat::of);

    if let Some(path) = cli.screenshot {
        return capture::run(
            loaded,
            video_format,
            &capture::Options {
                path,
                until: cli.until.clone(),
                ticks: cli.ticks,
                held: button_mask(cli.hold.as_deref()),
                pressed: button_mask(cli.press.as_deref()),
                trace: cli.trace,
                race: Some(race_options),
                log_every: cli.log_every,
                size: parse_size(&cli.size)?,
                screen: cli.screen.clone(),
                menu_page: cli.menu_page.clone(),
                presented: cli.presented,
                settings: settings.clone(),
                anisotropy,
            },
        );
    }

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
    // The two lists that come off the disc rather than out of the definition:
    // what is raceable, and what languages exist. Both carry a label the player
    // reads and a value the settings file stores, and on a circuit those are
    // different strings - `16_Track` against its localised name, which is
    // shipped content and only ever lives in memory.
    let shell = Shell {
        definition,
        modes: menu::mode_choices(&loaded.strings),
        tracks: loaded
            .tracks
            .iter()
            .map(|track| {
                (
                    track.clone(),
                    loaded.strings.get_or_id(&track.id).to_string(),
                )
            })
            .collect(),
        languages: loaded
            .languages
            .iter()
            .map(|language| menu::Choice::labelled(&language.name, &language.native_name))
            .collect(),
        font: loaded.font.clone(),
        sprites: loaded.sprites.clone(),
    };

    println!("\n{MENU_KEYS}");

    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the intro is animated whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        boot: Some(loaded),
        race: None,
        video_format,
        race_options,
        trace: cli.trace,
        log_every: cli.log_every,
        anisotropy,
        settings,
        shell: Some(shell),
        state: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// Turns a comma-separated list of abstract button names into a mask.
///
/// Unknown names are skipped rather than fatal: `none` is a real value in the
/// game's own XML and means no button.
fn button_mask(names: Option<&str>) -> u32 {
    names.map_or(0, |list| {
        list.split(',')
            .filter_map(|name| input::button_from_name(name.trim()))
            .fold(0u32, |mask, index| mask | (1u32 << index))
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
     Q and E or the shoulders are the airbrakes, L2 is both";

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
fn run_race(
    cli: &Cli,
    options: race::Options,
    settings: &settings::Settings,
    anisotropy: Anisotropy,
) -> Result<()> {
    let loaded = race::load(&options)?;
    for line in &loaded.report {
        println!("{line}");
    }

    if cli.dry_run {
        return Ok(());
    }

    if let Some(path) = cli.screenshot.clone() {
        return race::capture(
            loaded,
            &race::CaptureOptions {
                path,
                ticks: cli.ticks,
                held: button_mask(cli.hold.as_deref()),
                size: parse_size(&cli.size)?,
                log_every: cli.log_every,
                aspect: settings.display.aspect,
                anisotropy,
                renderer: settings.graphics.renderer.clone(),
                fov: settings.graphics.fov,
                frustum_culling: settings.graphics.frustum_culling,
                pvs_culling: settings.graphics.pvs_culling,
                animated_textures: settings.graphics.animated_textures,
                boost_fov_kick: settings.graphics.boost_fov_kick,
                anti_aliasing: settings.graphics.anti_aliasing,
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
        );
    }

    println!("\n{RACE_KEYS}{ESC_QUITS}");

    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the simulation runs whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        boot: None,
        race: Some(loaded),
        video_format: None,
        race_options: options,
        trace: cli.trace,
        log_every: cli.log_every,
        anisotropy,
        settings: settings.clone(),
        // `--race` opens a window straight onto a track: no front end, so no
        // font and no sprite sheet, so no menu tree and no circuit list. The
        // settings still apply, they just cannot be changed from here - which
        // is what makes escape quit on this route rather than back out.
        shell: None,
        state: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// One application handler for both ways in, because there is one window.
struct App {
    /// The boot sequence, when the game boots into the front end.
    boot: Option<boot::Boot>,
    /// A race loaded before the window opened, which is what `--race` does.
    race: Option<race::Loaded>,
    video_format: Option<VideoFormat>,
    /// What a race started from `Launch Game` is flown on.
    race_options: race::Options,
    trace: bool,
    log_every: u32,
    anisotropy: Anisotropy,
    /// The persisted settings, which the menus edit and write straight back.
    settings: settings::Settings,
    /// What the menus need, absent on the `--race` path.
    shell: Option<Shell>,
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
        let (backdrop, backdrop_shape) =
            match self.boot.as_mut().and_then(|loaded| loaded.backdrop.take()) {
                Some(movie) => {
                    let shape = BackdropShape {
                        frame_rate: movie.frame_rate,
                        // Pillarboxed rather than stretched, because the PS2's cut is
                        // not the PSP's shape: an `.IPF` declares its own display
                        // aspect. The PSP's `.PMF` is already 480x272, so this is the
                        // full screen there and changes nothing.
                        rect: frontend::pillarbox(frontend::SCREEN, movie.display_aspect),
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
        let stage = if let Some(loaded) = self.race.take() {
            gpu.window.set_title(RACE_TITLE);
            Stage::race(
                &gpu,
                loaded,
                framebuffer.size(),
                self.anisotropy,
                self.settings.graphics.anti_aliasing,
                self.settings.graphics.boost_fov_kick,
            )?
        } else if let Some(loaded) = self.boot.take() {
            Stage::frontend(&gpu, loaded, self.video_format, self.trace)?
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
            clock: TickClock::new(TickRate::DEFAULT),
            last: std::time::Instant::now(),
            meter: perf::Meter::new(),
            overlay,
            stalled: true,
            next_frame: std::time::Instant::now(),
            race_options: self.race_options.clone(),
            log_every: self.log_every,
            anisotropy: self.anisotropy,
            launched: false,
            settings: self.settings.clone(),
            shell: self.shell.clone(),
            quit: false,
            backdrop,
            backdrop_shape,
        }))
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
        let config = surface
            .get_default_config(&chosen.adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by this adapter")?;
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
    /// The boot sequence: the intro reel, then the language picker.
    Frontend(Box<FrontendStage>),
    /// Our own menus, between the boot sequence and a race.
    Menu(Box<MenuStage>),
    /// A ship on a track.
    Race(Box<RaceStage>),
}

impl Stage {
    fn frontend(
        gpu: &Gpu,
        loaded: boot::Boot,
        video_format: Option<VideoFormat>,
        trace: bool,
    ) -> Result<Self> {
        let renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            video_format,
            loaded.font.clone(),
            &loaded.sprites,
        )?;
        // The store moves onto its own thread and the `Movie` around it is done
        // with: everything else it carried - the frame count, the rate, the
        // aspect - was read into the sequence and the renderer before this.
        // `repeat: false`, because the intro ends rather than starting again.
        let feed = loaded.movie.and_then(|movie| {
            let (width, height) = (movie.width, movie.height);
            movie
                .frames
                .map(|frames| movie::Feed::spawn(frames, false, width, height))
        });
        Ok(Self::Frontend(Box::new(FrontendStage {
            renderer,
            frontend: loaded.frontend,
            feed,
            shown: false,
            trace,
        })))
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
        anti_aliasing: display::AntiAliasing,
        boost_fov_kick: bool,
    ) -> Result<Self> {
        let race::Loaded {
            setup,
            hud,
            track_model,
            ship_model,
            collision_model,
            sky_model,
            pad_model,
            boost_model,
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
            boost_model,
            flare,
            noise,
            gpu.config.format,
            size,
            anisotropy,
            visibility,
            anti_aliasing,
            fog_volumes,
        )?;
        // Against the **surface** format, like every other renderer here, because
        // the HUD is composited into the offscreen target which shares it.
        let overlay = oag_game::hud::Overlay::new(&gpu.device, &gpu.queue, gpu.config.format, &hud)
            .context("building the HUD overlay")?;
        let mut race = race::Race::start(setup);
        race.set_boost_fov_kick(boost_fov_kick);
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
    /// Where the disc's looping backdrop has got to, and where it goes on
    /// screen. `None` when this source has no backdrop, and then the rows are
    /// drawn on black exactly as they were before it existed.
    ///
    /// The player lives here rather than in `Session` because it is part of what
    /// is on screen: a stage that is not running should not be advancing a
    /// movie, and putting it here makes that structural instead of remembered.
    backdrop: Option<Backdrop>,
}

/// The looping menu picture: a player, where it goes, and which frame is up.
struct Backdrop {
    player: movie::Player,
    rect: [f32; 4],
    /// Which frame is **actually in the renderer's planes**, or `None` while none
    /// is.
    ///
    /// Not "which frame we would like": the decode happens on another thread now,
    /// so for the first frame or two after the menus open there is genuinely no
    /// picture, and `None` is what stops the video quad being drawn over zeroed
    /// planes - which is a green rectangle, not a black one. It is also what the
    /// draw list reports, so the list names the frame on screen rather than one
    /// that may not have arrived. See [`MenuStage::render`].
    shown: Option<usize>,
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
                    self.renderer.upload_frame(&gpu.queue, &frame.bytes)?;
                    backdrop.shown = Some(frame.index);
                }
                // The frame on screen, not the one the playhead names.
                backdrop.shown.map(|frame| menu::Backdrop {
                    rect: backdrop.rect,
                    frame,
                })
            }
            // A backdrop whose frames could not be opened is no backdrop: the
            // planes hold nothing, and drawing them would be a green rectangle
            // over the menu rather than a missing picture.
            _ => None,
        };
        let list = menu::draw_list(&self.menu, &keys::bound_keys, shown);
        self.renderer
            .render(&gpu.device, &gpu.queue, encoder, view, &list, viewport);
        Ok(())
    }
}

struct FrontendStage {
    renderer: Renderer,
    frontend: Frontend,
    /// The intro, decoded on a worker thread. `None` on a source with no movie,
    /// under `--no-video`, and with no `ffmpeg` - and then the renderer was built
    /// with no video pipeline either, so the draw is skipped rather than green.
    feed: Option<movie::Feed>,
    /// Whether a picture has ever reached the planes. See
    /// [`FrontendStage::sync_video`].
    shown: bool,
    trace: bool,
}

impl FrontendStage {
    fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) -> Result<()> {
        let mut list = self.frontend.draw_list();
        self.sync_video(&gpu.queue, &mut list)?;
        self.renderer
            .render(&gpu.device, &gpu.queue, encoder, view, &list, viewport);
        Ok(())
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
    fn sync_video(&mut self, queue: &wgpu::Queue, list: &mut Vec<frontend::Draw>) -> Result<()> {
        let Some(wanted) = list.iter().find_map(|draw| match draw {
            frontend::Draw::Video { frame, .. } => Some(*frame),
            _ => None,
        }) else {
            return Ok(());
        };
        if let Some(feed) = self.feed.as_mut() {
            if let Some(reason) = feed.take_error() {
                bail!("decoding the intro movie: {reason}");
            }
            // `saturating_sub`, so an empty cache is a movie with no picture
            // rather than a panic on `0 - 1`.
            let position = wanted.min(feed.len().saturating_sub(1)) as u64;
            if let Some(frame) = feed.take_upto(position) {
                self.renderer.upload_frame(queue, &frame.bytes)?;
                self.shown = true;
            }
            if !self.shown {
                list.retain(|draw| !matches!(draw, frontend::Draw::Video { .. }));
            }
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
    backdrop: Option<movie::Feed>,
    /// The backdrop's frame rate and where on screen it goes, kept because the
    /// feed carries pixels and not presentation.
    backdrop_shape: Option<BackdropShape>,
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
}

impl Shell {
    /// Which circuit a stored `race.track` names, if this source has it.
    fn track(&self, id: &str) -> Option<&catalogue::Track> {
        self.tracks
            .iter()
            .map(|(track, _)| track)
            .find(|track| track.id == id)
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

    fn frame(&mut self) -> Result<()> {
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

        for _ in 0..steps {
            // Ends the devices' tick for both stages. A race reads the snapshot's
            // axes; the front end reads the button edges the same call computed,
            // through `buttons_mut`, because it needs `consume_press` and a
            // snapshot is a value.
            let snapshot = self.controls.snapshot();
            match &mut self.stage {
                Stage::Frontend(stage) => {
                    let events = stage.frontend.update(dt, self.controls.buttons_mut());
                    report(&events, stage.trace);
                    for note in stage.frontend.take_notes() {
                        println!("{note}");
                    }
                }
                Stage::Menu(stage) => {
                    stage.tick(dt);
                    let events = stage.menu.update(self.controls.buttons_mut());
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
        let scene_stats = match &mut self.stage {
            Stage::Frontend(stage) => {
                stage.render(&self.gpu, &mut encoder, target, inside)?;
                None
            }
            Stage::Menu(stage) => {
                stage.render(
                    &self.gpu,
                    &mut encoder,
                    target,
                    inside,
                    self.backdrop.as_mut(),
                )?;
                None
            }
            Stage::Race(stage) => Some(stage.render(
                &self.gpu,
                &mut encoder,
                target,
                inside,
                self.settings.graphics.fov,
                self.settings.graphics.frustum_culling,
                self.settings.graphics.pvs_culling,
                self.settings.graphics.animated_textures,
            )),
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
        // Supplied before seeding, because a value cannot be seeded onto a list
        // that is not there yet.
        let tracks: Vec<menu::Choice> = shell
            .tracks
            .iter()
            .map(|(track, name)| menu::Choice::labelled(&track.id, name))
            .collect();
        model.supply(menu::ValueSource::Tracks, &tracks);
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
        // **And the feed is put back to the start.** `Player::new` below begins at
        // frame zero every time the menus open, so the feed has to begin at
        // position zero with it or the second open compares this player's
        // positions against a worker that is hundreds of frames further on, takes
        // nothing, and freezes on the last picture of the first visit.
        let shape = self.backdrop_shape.filter(|_| self.backdrop.is_some());
        let format = self.backdrop.as_ref().map(VideoFormat::of_feed);
        if let Some(feed) = self.backdrop.as_mut() {
            feed.restart();
        }
        let frames = self.backdrop.as_ref().map_or(0, movie::Feed::len);
        let renderer = Renderer::new(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.config.format,
            format,
            shell.font,
            &shell.sprites,
        )?;
        self.stage = Stage::Menu(Box::new(MenuStage {
            renderer,
            menu: model,
            backdrop: shape.map(|shape| Backdrop {
                // `repeat`, which is the whole difference between this movie and
                // the intro: `FE Screen` sits under it for as long as a player
                // is in the menus, so it wraps rather than finishing on its last
                // frame. See `movie::Player`.
                player: movie::Player::new(frames, true, shape.frame_rate),
                rect: shape.rect,
                shown: None,
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
                self.race_options.team = self.settings.race.team.clone();
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
            self.settings.graphics.anti_aliasing,
            self.settings.graphics.boost_fov_kick,
        )?;
        self.gpu.window.set_title(RACE_TITLE);
        Ok(())
    }
}
