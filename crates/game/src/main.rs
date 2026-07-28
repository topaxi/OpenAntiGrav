//! Runs Wipeout Pulse from the user's own disc image.
//!
//! ```sh
//! oag-game data/images/pulse-psp-usa.chd
//! ```
//!
//! Boots the way the original does: the intro reel with its frame-counted pauses
//! and two-second holds, START to skip, then the Language Selection screen driven
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
use oag_game::render::{Renderer, VideoFormat};
use oag_game::{INTRO_FRAMES_NEEDED, boot, capture, movie, race, report};
use oag_input::Keyboard;
use oag_physics::SpeedClass;

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
    #[arg(default_value = "data/images/pulse-psp-usa.chd")]
    source: String,

    /// Which movie the intro plays: an archive entry name, or `hash:XXXXXXXX`
    /// for one of the reels whose name is not recovered. Defaults to the
    /// European cut of the dev/pub reel; `Data\Movies\Intro.PMF` is the long
    /// intro the LogoFMV screen plays later.
    #[arg(long, default_value = boot::DEFAULT_INTRO_REEL)]
    movie: String,

    /// Convert every frame of the movie, not just the ones the intro shows.
    #[arg(long)]
    full_movie: bool,

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Parsed before anything is loaded, and for both ways in: the front end can
    // hand off to a race, so a misspelled class must not be discovered eight
    // seconds of intro later.
    let class = SpeedClass::from_name(&cli.class).with_context(|| {
        format!(
            "{:?} is not a speed class; try venom, flash, rapier or phantom",
            cli.class
        )
    })?;
    let race_options = race::Options {
        source: cli.source.clone(),
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
        return run_race(&cli, race_options);
    }

    let options = boot::Options {
        source: cli.source.clone(),
        movie: cli.movie.clone(),
        cache: cli.cache.clone().unwrap_or_else(boot::default_cache_dir),
        extent: if cli.full_movie {
            movie::Extent::Whole
        } else {
            movie::Extent::Frames(INTRO_FRAMES_NEEDED)
        },
        no_video: cli.no_video,
    };

    let mut loaded = boot::load(&options)?;
    if cli.overlay {
        loaded.frontend.set_overlay(true);
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
            },
        );
    }

    println!("\narrow keys move, return or X selects, space skips, escape quits");

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

/// The window's size, and a capture's default, front end and race alike.
///
/// Three times the PSP's screen, so the 5x7 glyphs stay legible and a screenshot
/// frames exactly what the window would have shown.
const WINDOW_SIZE: (u32, u32) = (1440, 816);

/// [`WINDOW_SIZE`] as `--size` spells it.
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

/// And once a race has taken it over.
const RACE_TITLE: &str = "OpenAntiGrav - race";

/// Printed whenever a race takes the window, by either route.
const RACE_KEYS: &str =
    "arrow keys steer, X or return thrusts, Q and E are the airbrakes, escape quits";

/// Loads a track and a ship and either captures one frame or opens a window.
fn run_race(cli: &Cli, options: race::Options) -> Result<()> {
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
            },
        );
    }

    println!("\n{RACE_KEYS}");

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
    state: Option<Session>,
}

impl App {
    /// Opens the window and builds whichever stage the command line asked for.
    ///
    /// `Ok(None)` means there was nothing to show, which only happens if the event
    /// loop resumes twice after the loaded state has been taken.
    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<Option<Session>> {
        let gpu = Gpu::new(event_loop)?;
        let stage = if let Some(loaded) = self.race.take() {
            gpu.window.set_title(RACE_TITLE);
            Stage::race(&gpu, loaded)?
        } else if let Some(loaded) = self.boot.take() {
            Stage::frontend(&gpu, loaded, self.video_format, self.trace)?
        } else {
            return Ok(None);
        };

        Ok(Some(Session {
            gpu,
            stage,
            keyboard: Keyboard::new(),
            clock: TickClock::new(TickRate::DEFAULT),
            last: std::time::Instant::now(),
            race_options: self.race_options.clone(),
            log_every: self.log_every,
            launched: false,
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
            WindowEvent::Focused(false) => session.keyboard.release_all(),

            WindowEvent::KeyboardInput { event, .. } => {
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && event.state == ElementState::Pressed
                {
                    event_loop.exit();
                    return;
                }
                session
                    .keyboard
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

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(session) = &self.state {
            session.gpu.window.request_redraw();
        }
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
    fn new(event_loop: &ActiveEventLoop) -> Result<Self> {
        // Fixed size on purpose, for now. It sets the window's minimum and maximum
        // to the same thing, which is the signal a tiling compositor floats a
        // window on rather than squeezing it into a column - measured under niri,
        // which tiles it to a portrait slot without this and honours 1440x816 with
        // it. The renderer does not depend on it: `Race::projection` fits the field
        // of view to whatever viewport it is given, so a tiled or fullscreen window
        // still frames the track correctly. This is the early-stages default, not a
        // decision that a game window should never resize.
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_SIZE.0, WINDOW_SIZE.1))
            .with_resizable(false);
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
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by this adapter")?;
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

    fn race(gpu: &Gpu, loaded: race::Loaded) -> Result<Self> {
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
            gpu.size(),
        )?;
        Ok(Self::Race(Box::new(RaceStage {
            scene,
            race: race::Race::start(setup),
        })))
    }
}

/// The front end, and everything only it needs.
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
    ) -> Result<()> {
        let list = self.frontend.draw_list();
        self.sync_video(&gpu.queue, &list)?;
        self.renderer
            .render(&gpu.device, &gpu.queue, encoder, view, &list, gpu.size());
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
    fn render(&self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.scene
            .render(&gpu.queue, encoder, view, &self.race, gpu.size());
    }
}

/// Everything that only exists once there is a window.
struct Session {
    gpu: Gpu,
    stage: Stage,
    /// One keyboard for both stages: a device belongs to the window rather than
    /// to what is on screen, so key state carries across the handoff and a focus
    /// loss releases everything whichever stage is running.
    keyboard: Keyboard,
    clock: TickClock,
    last: std::time::Instant,
    /// What `Launch Game` starts, kept because the front end is loaded long
    /// before anyone knows whether a race will be asked for.
    race_options: race::Options,
    log_every: u32,
    /// Set the first time `Launch Game` starts a race, so a load that fails is
    /// reported once rather than on every frame.
    launched: bool,
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
        // The depth attachment has to match the colour one, or the next pass is a
        // validation error.
        if let Stage::Race(stage) = &mut self.stage {
            stage.scene.resize(&self.gpu.device, (width, height));
        }
    }

    fn frame(&mut self) -> Result<()> {
        // Checked before this frame's ticks rather than after them, so the frame
        // that entered `Launch Game` is drawn once before the load stalls the
        // window.
        if !self.launched
            && matches!(&self.stage, Stage::Frontend(stage) if stage.frontend.is_finished())
        {
            self.launched = true;
            println!("\n{}: loading a race", frontend::states::LAUNCH_GAME);
            match self.launch_race() {
                Ok(()) => println!("\n{RACE_KEYS}"),
                // Reported rather than fatal: leaving the front end on screen is
                // more use than a window that vanishes.
                Err(e) => eprintln!("cannot start a race: {e:#}"),
            }
        }

        // Fixed timestep, per ADR-0007: the simulation steps at exactly 1/60
        // whatever the window is doing. The clock's own catch-up cap is what keeps
        // the race load above from being paid back as a burst of ticks.
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        let steps = self.clock.advance(nanos);
        let dt = f64::from(self.clock.rate().dt());

        for _ in 0..steps {
            // Ends the keyboard's tick for both stages. A race reads the snapshot's
            // axes; the front end reads the button edges the same call computed,
            // through `buttons_mut`, because it needs `consume_press` and a
            // snapshot is a value.
            let snapshot = self.keyboard.snapshot();
            match &mut self.stage {
                Stage::Frontend(stage) => {
                    let events = stage.frontend.update(dt, self.keyboard.buttons_mut());
                    report(&events, stage.trace);
                    for note in stage.frontend.take_notes() {
                        println!("{note}");
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
        match &mut self.stage {
            Stage::Frontend(stage) => stage.render(&self.gpu, &mut encoder, &view)?,
            Stage::Race(stage) => stage.render(&self.gpu, &mut encoder, &view),
        }
        self.gpu.queue.submit(Some(encoder.finish()));
        self.gpu.queue.present(frame);
        Ok(())
    }

    /// Replaces the front end with the race its `Launch Game` asks for.
    ///
    /// The window, the device and the surface are the ones already open, so the
    /// handoff costs a load and not a second window.
    fn launch_race(&mut self) -> Result<()> {
        let loaded = race::load(&self.race_options)?;
        for line in &loaded.report {
            println!("{line}");
        }
        self.stage = Stage::race(&self.gpu, loaded)?;
        self.gpu.window.set_title(RACE_TITLE);
        Ok(())
    }
}
