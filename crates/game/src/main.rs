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

use anyhow::{Context, Result};
use clap::Parser;
use oag_core::{TickClock, TickRate};

use oag_game::frontend::{self, Frontend};
use oag_game::input;
use oag_game::keys;
use oag_game::render::{Renderer, VideoFormat};
use oag_game::{
    boot, capture, catalogue, display, menu, movie, perf, race, report, settings, source, upscale,
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

    /// Draw the track's art meshes instead of its driveable ribbon.
    ///
    /// The ribbon is the default because it is the geometry the simulation spawns
    /// on, so it shows whether the ship is where the physics thinks it is.
    #[arg(long)]
    art: bool,

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Loaded (and, on first run or a missing key, written back complete) before
    // anything else: a bad value in the file should fail immediately, not eight
    // seconds of intro later.
    let settings = settings::load()?;
    let anisotropy = cli.anisotropy.unwrap_or(settings.graphics.anisotropy);

    // Parsed before anything is loaded, and for both ways in: the front end can
    // hand off to a race, so a misspelled class must not be discovered eight
    // seconds of intro later.
    let class = SpeedClass::from_name(&cli.class).with_context(|| {
        format!(
            "{:?} is not a speed class; try venom, flash, rapier or phantom",
            cli.class
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
        art: cli.art,
        collision: cli.collision,
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
    let video_format = loaded.movie.as_ref().and_then(|movie| {
        movie.frames.as_ref().map(|frames| VideoFormat {
            width: movie.width,
            height: movie.height,
            chroma_width: frames.chroma_width,
            chroma_height: frames.chroma_height,
        })
    });

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
                aspect: settings.graphics.aspect,
                anisotropy,
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
        let gpu = Gpu::new(event_loop, &self.settings.graphics)?;
        let graphics = &self.settings.graphics;

        // Built before the stage, because a race's depth attachment has to match
        // this rather than the window.
        let target = upscale::target_size(
            display::viewport(gpu.size(), graphics.aspect),
            graphics.render_scale,
            gpu.device.limits().max_texture_dimension_2d,
        );
        let framebuffer = upscale::Framebuffer::new(&gpu.device, gpu.config.format, target)
            .context("building the upscale pipeline")?;

        let stage = if let Some(loaded) = self.race.take() {
            gpu.window.set_title(RACE_TITLE);
            Stage::race(&gpu, loaded, framebuffer.size(), self.anisotropy)?
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

/// What winit is asked for, for a mode.
///
/// `Borderless(None)` means "the monitor this window is on", which is what
/// makes it unable to fail: exclusive fullscreen needs a `VideoMode` enumerated
/// off a monitor and can be refused, and this build does not offer it. See
/// [`display::WindowMode::ALL`].
fn fullscreen(mode: display::WindowMode) -> Option<winit::window::Fullscreen> {
    match mode {
        display::WindowMode::Windowed => None,
        display::WindowMode::Borderless => Some(winit::window::Fullscreen::Borderless(None)),
    }
}

/// What the surface is configured with, for a vsync setting.
///
/// The `Auto` pair rather than `Fifo` and `Immediate` by name: those two are
/// not both supported everywhere, and a present mode a surface does not offer
/// is a panic at configure time. `AutoNoVsync` takes the best unthrottled mode
/// the adapter has and falls back to `Fifo` when it has none, which is the
/// honest answer on a platform that will not tear - the frame limiter then
/// still does what it says, because it is not the surface doing the waiting.
fn present_mode(vsync: bool) -> wgpu::PresentMode {
    if vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::AutoNoVsync
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
}

impl Gpu {
    fn new(event_loop: &ActiveEventLoop, graphics: &settings::Graphics) -> Result<Self> {
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
        let size = graphics.window_size;
        let borderless = graphics.window_mode == display::WindowMode::Borderless;
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(size.width, size.height))
            .with_resizable(borderless)
            .with_fullscreen(fullscreen(graphics.window_mode));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("creating the window")?,
        );

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .context("creating the surface")?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .context("no suitable GPU adapter (is a Vulkan driver installed?)")?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("oag-game"),
            ..Default::default()
        }))
        .context("requesting the device")?;

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by this adapter")?;
        config.present_mode = present_mode(graphics.vsync);
        surface.configure(&device, &config);

        Ok(Self {
            window,
            device,
            queue,
            surface,
            config,
        })
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
        Ok(Self::Frontend(Box::new(FrontendStage {
            renderer,
            frontend: loaded.frontend,
            movie: loaded.movie,
            frame_bytes: Vec::new(),
            uploaded: None,
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
    ) -> Result<Self> {
        let race::Loaded {
            setup,
            track_model,
            ship_model,
            collision_model,
            ..
        } = loaded;
        let scene = race::Scene::new(
            &gpu.device,
            &gpu.queue,
            track_model,
            ship_model,
            collision_model,
            gpu.config.format,
            size,
            anisotropy,
        )?;
        Ok(Self::Race(Box::new(RaceStage {
            scene,
            race: race::Race::start(setup),
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
}

impl MenuStage {
    fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) {
        let list = menu::draw_list(&self.menu, &keys::bound_keys);
        self.renderer
            .render(&gpu.device, &gpu.queue, encoder, view, &list, viewport);
    }
}

struct FrontendStage {
    renderer: Renderer,
    frontend: Frontend,
    movie: Option<movie::Movie>,
    frame_bytes: Vec<u8>,
    uploaded: Option<usize>,
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
        let list = self.frontend.draw_list();
        self.sync_video(&gpu.queue, &list)?;
        self.renderer
            .render(&gpu.device, &gpu.queue, encoder, view, &list, viewport);
        Ok(())
    }

    /// Uploads the movie frame the draw list asks for, if it changed.
    fn sync_video(&mut self, queue: &wgpu::Queue, list: &[frontend::Draw]) -> Result<()> {
        let Some(wanted) = list.iter().find_map(|draw| match draw {
            frontend::Draw::Video { frame, .. } => Some(*frame),
            _ => None,
        }) else {
            return Ok(());
        };
        if self.uploaded == Some(wanted) {
            return Ok(());
        }
        let Some(frames) = self.movie.as_mut().and_then(|movie| movie.frames.as_mut()) else {
            return Ok(());
        };
        frames.read_frame(wanted.min(frames.len - 1), &mut self.frame_bytes)?;
        self.renderer.upload_frame(queue, &self.frame_bytes)?;
        self.uploaded = Some(wanted);
        Ok(())
    }
}

/// A race, and everything only it needs.
struct RaceStage {
    scene: race::Scene,
    race: race::Race,
}

impl RaceStage {
    fn render(
        &self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) {
        self.scene
            .render(&gpu.queue, encoder, view, &self.race, viewport);
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
    /// `None` while vsync is on, whatever the limit says: the display is
    /// already deciding, and a second limiter underneath it does not halve the
    /// frame rate, it beats against the refresh and turns an even 60 into an
    /// uneven one. That is why the menu greys the row out rather than leaving
    /// both live - the two settings are not independent, and pretending they
    /// are produces exactly the stutter the other half of `perf` exists to
    /// show.
    fn frame_period(&self) -> Option<std::time::Duration> {
        if self.settings.graphics.vsync {
            return None;
        }
        self.settings.graphics.frame_limit.period()
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
            .and_then(|_| self.settings.graphics.frame_limit.hz())
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
        let graphics = &self.settings.graphics;
        let rect = display::viewport(self.gpu.size(), graphics.aspect);
        let wanted = upscale::target_size(
            rect,
            graphics.render_scale,
            self.gpu.device.limits().max_texture_dimension_2d,
        );
        if self.framebuffer.resize(&self.gpu.device, wanted) {
            // A depth attachment whose size does not match the colour one is a
            // validation error, so the race's has to follow.
            if let Stage::Race(stage) = &mut self.stage {
                stage
                    .scene
                    .resize(&self.gpu.device, self.framebuffer.size());
            }
        }

        // Each stage fills the target, and the target *is* the game's
        // rectangle: the bars are the surface the blit does not cover.
        let size = self.framebuffer.size();
        let inside = (0.0, 0.0, size.0 as f32, size.1 as f32);
        let target = self.framebuffer.view();
        match &mut self.stage {
            Stage::Frontend(stage) => stage.render(&self.gpu, &mut encoder, target, inside)?,
            Stage::Menu(stage) => stage.render(&self.gpu, &mut encoder, target, inside),
            Stage::Race(stage) => stage.render(&self.gpu, &mut encoder, target, inside),
        }

        // Over the stage and inside the offscreen target, so the overlay is
        // drawn at the render scale the game is - measuring a frame nobody is
        // presenting would be the one way to get this wrong. One pass, skipped
        // entirely when the setting is off.
        let list = perf::draw_list(
            &self.meter,
            self.settings.graphics.perf_overlay,
            self.presentation_hz(),
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

        self.framebuffer.present(&mut encoder, &view, rect);
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
        self.seed_menu(&mut model);

        let renderer = Renderer::new(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.config.format,
            // No video: a menu draws text and rectangles, and asking for the
            // movie planes would tie the menus to a stage that played one.
            None,
            shell.font,
            &shell.sprites,
        )?;
        self.stage = Stage::Menu(Box::new(MenuStage {
            renderer,
            menu: model,
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

    /// Puts the window into the mode and size the settings now hold.
    ///
    /// Immediately, unlike anisotropy and the language: a player who picks
    /// borderless and sees nothing happen will assume it is broken. The resize
    /// event the compositor sends back is what reconfigures the surface, so
    /// nothing here touches it.
    ///
    /// The size is asked for only in windowed mode, and the request may be
    /// refused - a compositor is allowed to ignore it, and a tiling one will.
    /// Nothing depends on it being honoured.
    fn apply_window(&mut self) {
        let graphics = &self.settings.graphics;
        self.gpu
            .window
            .set_fullscreen(fullscreen(graphics.window_mode));
        let windowed = graphics.window_mode == display::WindowMode::Windowed;
        self.gpu.window.set_resizable(!windowed);
        if windowed {
            let size = graphics.window_size;
            let _ = self
                .gpu
                .window
                .request_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
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
            "graphics.aspect" => match text.parse::<display::Aspect>() {
                // Applied by the next frame, because every stage takes its
                // viewport from this on the way into its pass.
                Ok(aspect) => self.settings.graphics.aspect = aspect,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.window_mode" => match text.parse::<display::WindowMode>() {
                Ok(mode) => {
                    self.settings.graphics.window_mode = mode;
                    self.apply_window();
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.window_size" => match text.parse::<display::Size>() {
                Ok(size) => {
                    self.settings.graphics.window_size = size;
                    self.apply_window();
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.render_scale" => match text.parse::<display::Scale>() {
                // Applied by the next frame: `frame` sizes the target from this
                // every time and rebuilds it when the answer changes.
                Ok(scale) => self.settings.graphics.render_scale = scale,
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
            "graphics.vsync" => {
                // A flag, so it is read off the value rather than parsed: the
                // row is a toggle and `Value::Flag` is what it emits.
                let menu::Value::Flag(on) = value else {
                    eprintln!("ignoring {setting} = {text:?}: expected on or off");
                    return;
                };
                self.settings.graphics.vsync = *on;
                // Applied immediately, by reconfiguring the surface. A player
                // who turns vsync off and sees nothing change will assume it is
                // broken, and the frame limiter that comes with it would then
                // look broken too.
                self.gpu.config.present_mode = present_mode(*on);
                self.gpu
                    .surface
                    .configure(&self.gpu.device, &self.gpu.config);
                // The frames either side of a surface reconfigure are not
                // frames anyone is going to present at that rate.
                self.stalled = true;
            }
            "graphics.frame_limit" => match text.parse::<perf::FrameLimit>() {
                // Applied by the next `about_to_wait`, which is what waits.
                Ok(limit) => self.settings.graphics.frame_limit = limit,
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
        self.stage = Stage::race(&self.gpu, loaded, self.framebuffer.size(), self.anisotropy)?;
        self.gpu.window.set_title(RACE_TITLE);
        Ok(())
    }
}
