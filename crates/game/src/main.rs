//! Runs Wipeout Pulse from the user's own disc image.
//!
//! ```sh
//! oag-game data/images/pulse-psp-usa.chd
//! ```
//!
//! Boots the way the original does: the intro reel with its frame-counted pauses
//! and two-second holds, START to skip, then the Language Selection screen driven
//! by the disc's own front-end XML. Arrow keys move, Return or X selects, which
//! fires `Launch Game`.
//!
//! ```sh
//! # No display needed. Runs the sequence headless and writes one frame.
//! oag-game data/images/pulse-psp-usa.chd --screenshot /tmp/menu.png \
//!     --until "Language Selection" --hold start
//! ```
//!
//! `--race` is the other mode: a ship on a real track, arrow keys to steer, X to
//! thrust, Q and E for the airbrakes.
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
use oag_assets::pulse;
use oag_core::{TickClock, TickRate};

use oag_game::frontend::{self, Frontend};
use oag_game::input::{self, Input};
use oag_game::keys::map_key;
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
    #[arg(default_value = "data/images/pulse-psp-usa.chd")]
    source: String,

    /// Archive entry name of the movie the intro plays.
    #[arg(long, default_value = pulse::names::INTRO_MOVIE)]
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

    /// With `--screenshot`, run until this state is current before capturing.
    #[arg(long)]
    until: Option<String>,

    /// With `--screenshot`, run this many ticks before capturing.
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

    /// Fly a ship on a real track instead of booting the front end.
    ///
    /// Arrow keys steer, X or Return thrusts, Q and E are the airbrakes.
    #[arg(long)]
    race: bool,

    /// With `--race`, the track's `.vex` entry name in `Data.wad`.
    #[arg(long, default_value = race::DEFAULT_TRACK)]
    track: String,

    /// With `--race`, the team, which selects both the handling stats and the
    /// model.
    #[arg(long, default_value = race::DEFAULT_TEAM)]
    team: String,

    /// With `--race`, the speed class: venom, flash, rapier or phantom.
    #[arg(long, default_value = "venom")]
    class: String,

    /// With `--race`, draw the track's art meshes instead of its driveable ribbon.
    ///
    /// The ribbon is the default because it is the geometry the simulation spawns
    /// on, so it shows whether the ship is where the physics thinks it is.
    #[arg(long)]
    art: bool,

    /// With `--race`, print a telemetry line every this many ticks. Zero prints
    /// none.
    #[arg(long, default_value_t = 60)]
    log_every: u32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Before `boot::load`, deliberately: the front end's load parses the front-end
    // XML, every language plugin and a string table, and may shell out to `ffmpeg`
    // to transcode the intro. A race needs none of it.
    if cli.race {
        return run_race(&cli);
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

    let video_format = loaded.movie.frames.as_ref().map(|frames| VideoFormat {
        width: loaded.movie.width,
        height: loaded.movie.height,
        chroma_width: frames.chroma_width,
        chroma_height: frames.chroma_height,
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
            },
        );
    }

    println!("\narrow keys move, return or X selects, space skips, escape quits");

    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the intro is animated whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        boot: Some(loaded),
        video_format,
        trace: cli.trace,
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

/// The race view's size, windowed and captured alike.
///
/// Three times the PSP's screen, the same as the front end's window, so a
/// screenshot frames exactly what the window would have shown.
const RACE_SIZE: (u32, u32) = (1440, 816);

/// Loads a track and a ship and either captures one frame or opens a window.
fn run_race(cli: &Cli) -> Result<()> {
    let class = SpeedClass::from_name(&cli.class).with_context(|| {
        format!(
            "{:?} is not a speed class; try venom, flash, rapier or phantom",
            cli.class
        )
    })?;

    let loaded = race::load(&race::Options {
        source: cli.source.clone(),
        track: cli.track.clone(),
        team: cli.team.clone(),
        class,
        art: cli.art,
    })?;
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
                size: RACE_SIZE,
                log_every: cli.log_every,
            },
        );
    }

    println!("\narrow keys steer, X or return thrusts, Q and E are the airbrakes, escape quits");

    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the simulation runs whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = RaceApp {
        loaded: Some(loaded),
        log_every: cli.log_every,
        state: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct RaceApp {
    loaded: Option<race::Loaded>,
    log_every: u32,
    state: Option<RaceSession>,
}

/// Everything a race needs that only exists once there is a window.
struct RaceSession {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    scene: race::Scene,
    race: race::Race,
    keyboard: Keyboard,
    clock: TickClock,
    last: std::time::Instant,
    log_every: u32,
}

impl ApplicationHandler for RaceApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let Some(loaded) = self.loaded.take() else {
            event_loop.exit();
            return;
        };
        match RaceSession::new(event_loop, loaded, self.log_every) {
            Ok(session) => self.state = Some(session),
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
            session.window.request_redraw();
        }
    }
}

impl RaceSession {
    fn new(
        event_loop: &ActiveEventLoop,
        loaded: race::Loaded,
        log_every: u32,
    ) -> Result<RaceSession> {
        let attributes = Window::default_attributes()
            .with_title("OpenAntiGrav - race")
            .with_inner_size(winit::dpi::LogicalSize::new(RACE_SIZE.0, RACE_SIZE.1));
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
            label: Some("oag-game race"),
            ..Default::default()
        }))
        .context("requesting the device")?;

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by this adapter")?;
        surface.configure(&device, &config);

        let race::Loaded {
            setup,
            track_model,
            ship_model,
            ..
        } = loaded;
        let scene = race::Scene::new(
            &device,
            &queue,
            track_model,
            ship_model,
            config.format,
            (config.width, config.height),
        )?;

        Ok(RaceSession {
            window,
            device,
            queue,
            surface,
            config,
            scene,
            race: race::Race::start(setup),
            keyboard: Keyboard::new(),
            clock: TickClock::new(TickRate::DEFAULT),
            last: std::time::Instant::now(),
            log_every,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        // The depth attachment has to match the colour one, or the next pass is a
        // validation error.
        self.scene.resize(&self.device, (width, height));
    }

    fn frame(&mut self) -> Result<()> {
        // Fixed timestep, per ADR-0007: the simulation steps at exactly 1/60
        // whatever the window is doing.
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);

        for _ in 0..self.clock.advance(nanos) {
            let snapshot = self.keyboard.snapshot();
            self.race.tick(&snapshot);
            if self.log_every > 0 && self.race.world.tick % u64::from(self.log_every) == 0 {
                println!("{}", race::describe(&self.race.telemetry()));
            }
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
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
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race frame"),
            });
        self.scene.render(
            &self.queue,
            &mut encoder,
            &view,
            &self.race,
            (self.config.width, self.config.height),
        );
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }
}

struct App {
    boot: Option<boot::Boot>,
    video_format: Option<VideoFormat>,
    trace: bool,
    state: Option<Session>,
}

/// Everything that only exists once there is a window.
struct Session {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    frontend: Frontend,
    movie: movie::Movie,
    clock: TickClock,
    last: std::time::Instant,
    input: Input,
    held: u32,
    frame_bytes: Vec<u8>,
    uploaded: Option<usize>,
    trace: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let Some(loaded) = self.boot.take() else {
            event_loop.exit();
            return;
        };

        match Session::new(event_loop, loaded, self.video_format, self.trace) {
            Ok(session) => self.state = Some(session),
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

            WindowEvent::KeyboardInput { event, .. } => {
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && event.state == ElementState::Pressed
                {
                    event_loop.exit();
                    return;
                }
                session.key(&event.logical_key, event.state == ElementState::Pressed);
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
            session.window.request_redraw();
        }
    }
}

impl Session {
    fn new(
        event_loop: &ActiveEventLoop,
        loaded: boot::Boot,
        video_format: Option<VideoFormat>,
        trace: bool,
    ) -> Result<Self> {
        let attributes = Window::default_attributes()
            .with_title("OpenAntiGrav")
            // Three times the PSP's screen, so the 5x7 glyphs stay legible.
            .with_inner_size(winit::dpi::LogicalSize::new(1440, 816));
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

        let renderer = Renderer::new(&device, &queue, config.format, video_format)?;

        Ok(Self {
            window,
            device,
            queue,
            surface,
            config,
            renderer,
            frontend: loaded.frontend,
            movie: loaded.movie,
            clock: TickClock::new(TickRate::DEFAULT),
            last: std::time::Instant::now(),
            input: Input::new(),
            held: 0,
            frame_bytes: Vec::new(),
            uploaded: None,
            trace,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn key(&mut self, key: &Key, down: bool) {
        let Some(index) = map_key(key) else { return };
        let mask = 1u32 << index;
        if down {
            self.held |= mask;
        } else {
            self.held &= !mask;
        }
    }

    fn frame(&mut self) -> Result<()> {
        // Fixed timestep, per ADR-0007: the simulation steps at exactly 1/60
        // whatever the window is doing.
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        let steps = self.clock.advance(nanos);
        let dt = f64::from(self.clock.rate().dt());

        for _ in 0..steps {
            self.input.begin_frame(self.held);
            let events = self.frontend.update(dt, &mut self.input);
            report(&events, self.trace);
            for note in self.frontend.take_notes() {
                println!("{note}");
            }
        }

        let list = self.frontend.draw_list();
        self.sync_video(&list)?;

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
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
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        self.renderer.render(
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            &list,
            (self.config.width, self.config.height),
        );
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }

    /// Uploads the movie frame the draw list asks for, if it changed.
    fn sync_video(&mut self, list: &[frontend::Draw]) -> Result<()> {
        let Some(wanted) = list.iter().find_map(|draw| match draw {
            frontend::Draw::Video { frame, .. } => Some(*frame),
            _ => None,
        }) else {
            return Ok(());
        };
        if self.uploaded == Some(wanted) {
            return Ok(());
        }
        let Some(frames) = self.movie.frames.as_mut() else {
            return Ok(());
        };
        frames.read_frame(wanted.min(frames.len - 1), &mut self.frame_bytes)?;
        self.renderer.upload_frame(&self.queue, &self.frame_bytes)?;
        self.uploaded = Some(wanted);
        Ok(())
    }
}
