//! `--wallpaper` as the desktop background of a Wayland compositor with `wlr-layer-shell` (niri,
//! sway, Hyprland, river): one `background`-layer surface per output, each drawn as a window of
//! the one session with a camera of its own (`docs/ui/attract-demo.md`, "A desktop background").
//!
//! winit has no layer shell, so this run has no winit window and no winit loop. Its own loop
//! (calloop over the Wayland socket) calls `Session::frame` as the window's redraw would, the
//! way `--drive` runs a session with no window (`crate::drive`). Each output's surface is a wgpu
//! surface over the raw `wl_surface` ([`crate::gpu::RawTarget`]), drawn only when the
//! compositor asks for a frame of it (its frame callback, [`Pacer`]) and at most
//! `--wallpaper-fps` times a second, so an output the compositor stops showing (and stops sending
//! callbacks to) costs nothing.
//!
//! A background takes no input (no keyboard focus, an empty input region): it ends on SIGINT or
//! SIGTERM, or when the compositor goes away. It holds no idle inhibitor: the screen still
//! blanks and locks over it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use log::{debug, info, warn};
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState, Region};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::{
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, registry_handlers,
};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::{Connection, Proxy, QueueHandle};
use winit::raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};

use crate::app::App;
use crate::gpu::{Gpu, Pace, RawTarget};
use crate::session::Session;

/// How long an output whose frame callback has not come back is left before it is drawn once
/// more: a compositor that withholds callbacks from a hidden surface sends the pending one when
/// it shows the surface again, so this is only a guard against one that never does.
const REPROBE: Duration = Duration::from_secs(2);

/// How many round trips the start waits for every output's first configure.
const FIRST_CONFIGURE_TRIES: u32 = 8;

/// The layer-shell namespace a compositor's rules can match the wallpaper by.
const NAMESPACE: &str = "oag-wallpaper";

/// Set by SIGINT or SIGTERM.
static STOP: AtomicBool = AtomicBool::new(false);

/// Runs `app` as the desktop background. `Ok(false)` when this desktop has no layer shell to
/// put it on (not Wayland, or a compositor without `zwlr_layer_shell_v1`), and the caller opens
/// the ordinary window instead.
///
/// # Errors
///
/// The device or the session would not open, or the Wayland connection failed while running.
pub(crate) fn run(app: &mut App) -> Result<bool> {
    if std::env::var("WINIT_UNIX_BACKEND").is_ok_and(|b| b == "x11") {
        return Ok(false);
    }
    if app.launcher.is_some() {
        warn!("--wallpaper: no disc image was chosen, so the picker opens in a window; name the disc image");
        return Ok(false);
    }
    let Ok(conn) = Connection::connect_to_env() else {
        return Ok(false);
    };
    let (globals, mut queue) = registry_queue_init::<State>(&conn).context("the Wayland registry")?;
    let qh = queue.handle();
    let Ok(layer_shell) = LayerShell::bind(&globals, &qh) else {
        warn!("--wallpaper: this compositor has no wlr-layer-shell, so the wallpaper is a window");
        return Ok(false);
    };
    let compositor = CompositorState::bind(&globals, &qh).context("wl_compositor")?;
    let mut state = State {
        session: None,
        surfaces: Vec::new(),
        primary_lost: false,
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        compositor,
        layer_shell,
        conn: conn.clone(),
        qh: qh.clone(),
    };
    // The outputs and their names, then every output's first configure.
    queue.roundtrip(&mut state).context("listing the outputs")?;
    for _ in 0..FIRST_CONFIGURE_TRIES {
        if !state.surfaces.is_empty() && state.surfaces.iter().all(|s| s.size.is_some()) {
            break;
        }
        queue.roundtrip(&mut state).context("configuring the backgrounds")?;
    }
    let Some(first) = state.surfaces.iter().position(|s| s.size.is_some()) else {
        anyhow::bail!("--wallpaper: the compositor configured no background (no outputs?)");
    };
    let target = state.surfaces[first].target(&conn, &qh);
    let gpu = Gpu::on_raw(target, &app.settings.graphics.renderer)?;
    state.surfaces[first].placed = true;
    state.surfaces.swap(0, first);
    let Some(session) = app.open_with(gpu)? else {
        return Ok(true);
    };
    state.session = Some(session);
    state.place_outputs();
    info!(
        "--wallpaper: the desktop background of {} output(s)",
        state.surfaces.iter().filter(|s| s.placed).count()
    );
    catch_signals();

    let mut event_loop: EventLoop<State> = EventLoop::try_new().context("the wallpaper's loop")?;
    WaylandSource::new(conn, queue)
        .insert(event_loop.handle())
        .map_err(|e| anyhow::anyhow!("watching the Wayland socket: {}", e.error))?;
    loop {
        let wait = state.wait();
        event_loop
            .dispatch(Some(wait), &mut state)
            .context("the Wayland connection")?;
        state.place_outputs();
        if STOP.load(Ordering::Relaxed) || state.session.as_ref().is_some_and(|s| s.quit) {
            break;
        }
        state.frame()?;
    }
    debug!("--wallpaper: leaving");
    if let Some(mut session) = state.session.take() {
        session.wind_down();
    }
    Ok(true)
}

/// SIGINT and SIGTERM end the wallpaper through its own way out, so what a run keeps on leaving
/// (`Session::wind_down`) is kept.
fn catch_signals() {
    extern "C" fn stop(_: libc::c_int) {
        STOP.store(true, Ordering::Relaxed);
    }
    let handler = stop as extern "C" fn(libc::c_int) as libc::sighandler_t;
    // SAFETY: the handler only stores to an atomic, which is async-signal-safe.
    #[allow(unsafe_code)]
    unsafe {
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGTERM, handler);
    }
}

/// One output's background.
struct Background {
    output: wl_output::WlOutput,
    layer: LayerSurface,
    /// The output's name (`DP-1`), for the log.
    name: String,
    /// Its configured size in surface coordinates, `None` until the first configure.
    size: Option<(u32, u32)>,
    /// The output's integer scale, which the buffer is drawn at.
    scale: i32,
    /// Whether the session draws it yet.
    placed: bool,
    gate: Arc<Mutex<Gate>>,
}

impl Background {
    fn pixels(&self) -> winit::dpi::PhysicalSize<u32> {
        let (width, height) = self.size.unwrap_or((1, 1));
        let scale = self.scale.max(1).unsigned_abs();
        winit::dpi::PhysicalSize::new(width.max(1) * scale, height.max(1) * scale)
    }

    fn target(&self, conn: &Connection, qh: &QueueHandle<State>) -> RawTarget {
        let surface = self.layer.wl_surface().clone();
        let display = std::ptr::NonNull::new(conn.backend().display_ptr().cast())
            .expect("a live Wayland connection has a display");
        let window = std::ptr::NonNull::new(surface.id().as_ptr().cast())
            .expect("a live wl_surface has a proxy");
        RawTarget {
            display: RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display)),
            window: RawWindowHandle::Wayland(WaylandWindowHandle::new(window)),
            size: self.pixels(),
            keep: Box::new((self.layer.clone(), conn.clone())),
            pace: Box::new(Pacer {
                surface,
                qh: qh.clone(),
                gate: Arc::clone(&self.gate),
            }),
        }
    }

    /// Whether the compositor would take a frame of it now.
    fn wants_frame(&self) -> bool {
        self.placed && self.gate.lock().is_ok_and(|gate| gate.open())
    }
}

/// An output's frame callback: when the last one was asked for, until it comes back.
#[derive(Default)]
struct Gate {
    asked: Option<Instant>,
}

impl Gate {
    fn open(&self) -> bool {
        self.asked.is_none_or(|at| at.elapsed() >= REPROBE)
    }
}

/// [`Pace`] for one background: a frame is drawn when the compositor answered the last one's
/// callback, and asks for the next callback as it is drawn (it rides the present's commit).
struct Pacer {
    surface: wl_surface::WlSurface,
    qh: QueueHandle<State>,
    gate: Arc<Mutex<Gate>>,
}

impl Pace for Pacer {
    fn ready(&mut self) -> bool {
        let Ok(mut gate) = self.gate.lock() else {
            return false;
        };
        if !gate.open() {
            return false;
        }
        self.surface.frame(&self.qh, self.surface.clone());
        gate.asked = Some(Instant::now());
        true
    }
}

struct State {
    /// Declared first, so it drops before the Wayland objects its surfaces were made over.
    session: Option<Session>,
    /// In the session's window order once placed: index 0 is window 1.
    surfaces: Vec<Background>,
    /// Every output went away and window 1 draws nothing until one comes back.
    primary_lost: bool,
    registry: RegistryState,
    outputs: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    conn: Connection,
    qh: QueueHandle<State>,
}

impl State {
    /// A background for `output`, configured by the compositor before it is drawn.
    fn add_output(&mut self, output: wl_output::WlOutput) {
        let info = self.outputs.info(&output);
        let name = info
            .as_ref()
            .and_then(|info| info.name.clone())
            .unwrap_or_else(|| format!("output {}", output.id().protocol_id()));
        let scale = info.as_ref().map_or(1, |info| info.scale_factor);
        let surface = self.compositor.create_surface(&self.qh);
        let layer = self.layer_shell.create_layer_surface(
            &self.qh,
            surface,
            Layer::Background,
            Some(NAMESPACE),
            Some(&output),
        );
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_size(0, 0);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        if let Ok(region) = Region::new(&self.compositor) {
            layer.set_input_region(Some(region.wl_region()));
        }
        let _ = layer.set_buffer_scale(scale.max(1).unsigned_abs());
        layer.commit();
        debug!("--wallpaper: a background on {name} (scale {scale})");
        self.surfaces.push(Background {
            output,
            layer,
            name,
            size: None,
            scale,
            placed: false,
            gate: Arc::default(),
        });
    }

    /// Hands every configured background the session does not draw yet to it.
    fn place_outputs(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let configured = |b: &Background| !b.placed && b.size.is_some();
        while let Some(index) = self.surfaces.iter().position(configured) {
            let target = self.surfaces[index].target(&self.conn, &self.qh);
            let opened = session.gpu.open_raw_output(target);
            let mut background = self.surfaces.remove(index);
            match opened {
                Ok(output) => {
                    background.placed = true;
                    let placed = self.surfaces.iter().filter(|b| b.placed).count();
                    debug!(
                        "--wallpaper: {} drawn ({}x{})",
                        background.name,
                        output.config.width,
                        output.config.height
                    );
                    if self.primary_lost {
                        self.primary_lost = false;
                        session.replace_primary(output);
                    } else if let Err(e) = session.add_background(output) {
                        warn!("--wallpaper: {}: {e:#}", background.name);
                        continue;
                    }
                    self.surfaces.insert(placed, background);
                }
                Err(e) => warn!("--wallpaper: {}: {e:#}", background.name),
            }
        }
    }

    /// Takes the background at `index` out of the session and drops it.
    fn remove(&mut self, index: usize) {
        let background = self.surfaces.remove(index);
        if background.placed
            && let Some(session) = self.session.as_mut()
        {
            if index == 0 && !self.surfaces.first().is_some_and(|b| b.placed) {
                self.primary_lost = true;
            }
            session.remove_background(index);
        }
        debug!("--wallpaper: {} gone", background.name);
    }

    /// One frame, when the limit allows it and some output would take it. An idle desktop
    /// whose every background is hidden ticks nothing.
    fn frame(&mut self) -> Result<()> {
        let Some(session) = self.session.as_mut() else {
            return Ok(());
        };
        let due = session.next_frame_at().is_none_or(|at| Instant::now() >= at);
        if due && self.surfaces.iter().any(Background::wants_frame) {
            session.frame()?;
        }
        Ok(())
    }

    /// How long the loop may sleep: until the next frame is due, or until a hidden output is
    /// tried again.
    fn wait(&self) -> Duration {
        let ready = self.surfaces.iter().any(Background::wants_frame);
        let until_due = self
            .session
            .as_ref()
            .and_then(Session::next_frame_at)
            .map_or(Duration::ZERO, |at| at.saturating_duration_since(Instant::now()));
        if ready { until_due } else { until_due.max(Duration::from_millis(250)) }
    }

    fn index_of(&self, surface: &wl_surface::WlSurface) -> Option<usize> {
        self.surfaces.iter().position(|b| b.layer.wl_surface() == surface)
    }
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        factor: i32,
    ) {
        let Some(index) = self.index_of(surface) else {
            return;
        };
        let background = &mut self.surfaces[index];
        if background.scale == factor {
            return;
        }
        background.scale = factor;
        let _ = background.layer.set_buffer_scale(factor.max(1).unsigned_abs());
        let (placed, size) = (background.placed, background.pixels());
        debug!("--wallpaper: {} at scale {factor}", background.name);
        if placed && let Some(session) = self.session.as_mut() {
            session.resize_background(index, size);
        }
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _: u32) {
        if let Some(index) = self.index_of(surface)
            && let Ok(mut gate) = self.surfaces[index].gate.lock()
        {
            gate.asked = None;
        }
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        self.add_output(output);
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        if let Some(info) = self.outputs.info(&output)
            && let Some(name) = info.name
            && let Some(background) = self.surfaces.iter_mut().find(|b| b.output == output)
        {
            background.name = name;
        }
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        if let Some(index) = self.surfaces.iter().position(|b| b.output == output) {
            self.remove(index);
        }
    }
}

impl LayerShellHandler for State {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        if let Some(index) = self.index_of(layer.wl_surface()) {
            self.remove(index);
        }
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let Some(index) = self.index_of(layer.wl_surface()) else {
            return;
        };
        let background = &mut self.surfaces[index];
        let size = Some(configure.new_size);
        if background.size == size {
            return;
        }
        background.size = size;
        let (placed, pixels) = (background.placed, background.pixels());
        debug!(
            "--wallpaper: {} configured {}x{} ({}x{} pixels)",
            background.name, configure.new_size.0, configure.new_size.1, pixels.width, pixels.height
        );
        if placed && let Some(session) = self.session.as_mut() {
            session.resize_background(index, pixels);
        }
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState];
}

delegate_compositor!(State);
delegate_output!(State);
delegate_layer!(State);
delegate_registry!(State);
