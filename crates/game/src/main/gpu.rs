//! The surface, the device and the queue: one window's worth of wgpu.

use std::sync::Arc;

use anyhow::{Context, Result};
use log::{info, warn};

use oag_game::{adapter, settings};

use oag_present::perf;
use oag_ui::strings;

use oag_display::display;

use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use crate::hints;
#[cfg(target_os = "linux")]
use crate::window::APP_ID;
use crate::window::{
    centred_on, choose_monitor, frame_latency, fullscreen, present_modes, window_icon,
};

/// The window and the GPU objects, which both stages draw through.
///
/// One window and one device for the whole process: the front end reaching
/// `Launch Game` swaps what is drawn, not what it is drawn with.
pub(crate) struct Gpu {
    pub(crate) window: Arc<Window>,
    /// The instance the surface came from: a surface is only valid with the
    /// adapter and device of its own instance, so [`Gpu::recreate_surface`]
    /// needs this one and not a fresh `adapter::instance()`.
    instance: wgpu::Instance,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
    /// The present modes this surface actually has. See [`Gpu::present_mode`].
    offered: Vec<wgpu::PresentMode>,
    /// What the RENDERER row offers. See [`adapter::Chosen::offered`].
    pub(crate) adapters: Vec<String>,
    /// What the RENDERER row would have to say to describe this run, which is
    /// not necessarily what the settings file says: a named adapter that would
    /// not make a device fell back to the default, and the default is drawing
    /// with something in particular. See [`adapter::Chosen::in_use`] and the
    /// row's restart note.
    pub(crate) in_use: Vec<String>,
    /// Whether this adapter can run the temporal upscaler at all.
    ///
    /// **Asked once, here, because this is the only place the adapter exists.**
    /// [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
    /// requires that a missing capability *degrade* rather than fail to boot,
    /// and the degrade happens in `upscale::Framebuffer::resolve_scene` - which
    /// has a device and no adapter. Carrying the answer is cheaper than
    /// re-requesting an adapter to ask it a second time, which is the same
    /// argument `offered` above already makes.
    pub(crate) temporal: bool,
}

/// What one adapter has to hand over before it can be drawn with.
///
/// A struct rather than four statements inline because **all four have to
/// succeed or none of them count**: a named adapter that enumerates fine can
/// still refuse the device or the surface config, and the recovery for that is
/// to run the whole sequence again on a different adapter. See
/// [`Gpu::bring_up`].
pub(crate) struct BroughtUp {
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    offered: Vec<wgpu::PresentMode>,
    adapters: Vec<String>,
    in_use: Vec<String>,
    temporal: bool,
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
        #[cfg(target_arch = "wasm32")]
        let (chosen, device, queue) = {
            let _ = (instance, renderer);
            web::take()?
        };
        #[cfg(not(target_arch = "wasm32"))]
        let chosen = adapter::choose(instance, Some(surface), renderer)?;
        #[cfg(not(target_arch = "wasm32"))]
        let (device, queue) =
            pollster::block_on(chosen.adapter.request_device(&wgpu::DeviceDescriptor {
                // **Not the default, which is `MemoryHints::Performance`.**
                // `gpu-allocator` suballocates from device blocks rather than
                // allocating per resource, and `Performance` asks for blocks of
                // 128-256 MiB: on a heap with 200 MiB free that request fails
                // outright while a 4 MiB texture would have fitted many times
                // over. `MemoryUsage` asks for 8-64 MiB instead, which costs
                // some allocator churn and nothing anyone would see.
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                ..oag_mesh::mesh_render::device_descriptor("oag-game", &chosen.adapter)
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
        let temporal = oag_post::fsr3::supported(&chosen.adapter);
        if !temporal {
            warn!("no compute shaders on this adapter; UPSCALER fsr3 will run as fsr1");
        }
        // Said here, after the device and the surface config, so the line is
        // about an adapter that *worked*: an attempt that dies at either of them
        // prints its own failure and falls back, and one line naming the loser
        // and another naming the winner would leave a bug report to guess which
        // one drew the picture. It also has to be said at all - `default` names
        // nothing a player could look up, and a name this machine no longer has
        // silently becomes the default. The driver comes with it because "which
        // llvmpipe" and "which Mesa" are the next questions.
        let info = chosen.adapter.get_info();
        info!(
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
            temporal,
            device,
            queue,
            config,
            offered,
            adapters: chosen.offered,
            in_use: chosen.in_use,
        })
    }

    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        settings: &settings::Display,
        renderer: &display::Renderer,
        // The player's chosen language, so the window's own title can be
        // overridden the same way the loading screen's prose is - see
        // `crate::hints`. Its own parameter rather than folded into
        // `settings` above: that is `settings::Display`, one field of the
        // whole `Settings`, and `language` is a sibling field of it, not a
        // display preference.
        language: Option<&str>,
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
        let title_strings = strings::project_table(language);
        let mut attributes = Window::default_attributes()
            .with_title(hints::title(&title_strings))
            .with_inner_size(winit::dpi::LogicalSize::new(size.width, size.height))
            .with_resizable(borderless)
            .with_window_icon(Some(window_icon()))
            .with_fullscreen(fullscreen(settings.window_mode, monitor.clone()));
        // The Wayland/X11 app id - see `window::APP_ID`'s doc for why this is
        // the half of the icon that actually reaches a Wayland taskbar.
        // `WindowAttributesExtWayland` rather than the X11 twin because both
        // write the same underlying field, so importing either sets it on
        // both backends; cfg-gated because the trait only exists at all when
        // winit is built with a Linux backend.
        #[cfg(target_os = "linux")]
        {
            use winit::platform::wayland::WindowAttributesExtWayland;
            attributes = attributes.with_name(APP_ID, APP_ID);
        }
        // Borderless carries the choice in the fullscreen request; windowed has
        // nothing to carry it, so the window is placed on the screen instead.
        // Asked for at creation rather than moved afterwards, which would open
        // it on one monitor and jump it to another in view of the player.
        if let (false, Some(monitor)) = (borderless, &monitor) {
            attributes = attributes.with_position(centred_on(monitor, size));
        }
        #[cfg(target_arch = "wasm32")]
        {
            attributes = web::canvas(attributes);
        }
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("creating the window")?,
        );
        // The window system's own cursor is never shown over the game. The
        // menus answer a pointer, and draw their own in the title's own
        // palette - `oag_game::cursor`, painted by `Session::draw` - because
        // a compositor's cursor over a borderless game is whatever that
        // compositor feels like showing, which on a handheld is nothing.
        // Racing reads no pointer at all, and there the drawn one is left
        // out too.
        window.set_cursor_visible(false);

        #[cfg(target_arch = "wasm32")]
        let instance = web::instance()?;
        #[cfg(not(target_arch = "wasm32"))]
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
                warn!("renderer {renderer} could not be brought up: {e:#}");
                warn!(
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
            temporal,
        } = brought;

        let mut gpu = Self {
            temporal,
            window,
            instance,
            device,
            queue,
            surface,
            config,
            offered,
            adapters,
            in_use,
        };
        gpu.apply_vsync(settings.vsync);
        gpu.surface.configure(&gpu.device, &gpu.config);
        Ok(gpu)
    }

    /// A new window and surface on the same device, after the platform took
    /// the old ones away.
    ///
    /// Android destroys the native window on every `Suspended` (the app left
    /// the foreground, or the screen turned off) and hands out a new one on
    /// the next `Resumed`; a surface over the old one is dead. Desktop never
    /// calls this: winit only raises those events on mobile and the web.
    pub(crate) fn recreate_surface(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .context("recreating the window")?,
        );
        window.set_cursor_visible(false);
        let surface = self
            .instance
            .create_surface(window.clone())
            .context("recreating the surface")?;
        let size = window.inner_size();
        self.window = window;
        self.surface = surface;
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        self.surface.configure(&self.device, &self.config);
        Ok(())
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
        warn!(
            "this surface offers {:?}, so vsync {vsync} falls back to Fifo",
            self.offered
        );
        wgpu::PresentMode::Fifo
    }

    /// Puts `vsync` into effect.
    pub(crate) fn set_vsync(&mut self, vsync: perf::Vsync) {
        self.apply_vsync(vsync);
        self.surface.configure(&self.device, &self.config);
    }

    /// The present mode and queue depth `vsync` asks for, into the config.
    fn apply_vsync(&mut self, vsync: perf::Vsync) {
        self.config.present_mode = self.present_mode(vsync);
        self.config.desired_maximum_frame_latency =
            frame_latency(vsync, self.config.present_mode);
    }

    /// The viewport, which every stage draws into.
    pub(crate) fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
}

/// The three things a scene is built and drawn with: the device, the queue
/// and the surface format.
///
/// **A trait so a scene can be built off the frame thread.** [`Gpu`] also owns
/// the window and the surface, which stay with the event loop; a race scene
/// needs neither, so [`crate::race_build`] hands a worker a [`Handles`] - three
/// cheap clones - and the same functions the frame loop calls with a `&Gpu`
/// take it unchanged.
pub(crate) trait GpuContext {
    fn device(&self) -> &wgpu::Device;
    fn queue(&self) -> &wgpu::Queue;
    fn format(&self) -> wgpu::TextureFormat;
}

impl GpuContext for Gpu {
    fn device(&self) -> &wgpu::Device {
        &self.device
    }
    fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
}

/// [`Gpu`] without the window and the surface: what a worker thread may hold.
///
/// `wgpu::Device` and `wgpu::Queue` are reference-counted and `Send + Sync` in
/// wgpu 30, so these are the frame loop's own device and queue, not a second
/// pair.
#[derive(Clone)]
pub(crate) struct Handles {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) format: wgpu::TextureFormat,
}

impl Gpu {
    /// This window's [`Handles`].
    pub(crate) fn handles(&self) -> Handles {
        Handles {
            device: self.device.clone(),
            queue: self.queue.clone(),
            format: self.config.format,
        }
    }
}

impl GpuContext for Handles {
    fn device(&self) -> &wgpu::Device {
        &self.device
    }
    fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}

/// The browser's half of [`Gpu::new`]: WebGPU hands out its adapter and device
/// through promises, and the page's main thread may not block on one, so the
/// web entry point awaits [`web::prepare`] before the event loop starts and
/// [`Gpu::new`] takes what it left. There is no adapter to choose between on
/// the web (the browser picks one), so the RENDERER row offers nothing. See
/// docs/tools/web.md.
#[cfg(target_arch = "wasm32")]
pub(crate) mod web {
    use std::cell::RefCell;

    use anyhow::{Context, Result};
    use oag_game::adapter;

    /// The id of the `<canvas>` the page draws into.
    pub(crate) const CANVAS_ID: &str = "oag-canvas";

    struct Prepared {
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    }

    thread_local! {
        static PREPARED: RefCell<Option<Prepared>> = const { RefCell::new(None) };
    }

    /// Requests the adapter and the device, as [`super::Gpu::bring_up`] does
    /// on native with the same descriptor.
    pub(crate) async fn prepare() -> Result<()> {
        let instance = adapter::instance();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .context("no WebGPU adapter (does this browser have WebGPU enabled?)")?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                ..oag_mesh::mesh_render::device_descriptor("oag-game", &adapter)
            })
            .await
            .context("requesting the device")?;
        PREPARED.with(|cell| {
            *cell.borrow_mut() = Some(Prepared {
                instance,
                adapter,
                device,
                queue,
            });
        });
        Ok(())
    }

    /// The instance [`prepare`] made the adapter from: a surface must come
    /// from the same one.
    pub(crate) fn instance() -> Result<wgpu::Instance> {
        PREPARED.with(|cell| {
            cell.borrow()
                .as_ref()
                .map(|prepared| prepared.instance.clone())
                .context("the WebGPU device was not prepared before the window opened")
        })
    }

    /// The prepared adapter and device, once.
    pub(crate) fn take() -> Result<(adapter::Chosen, wgpu::Device, wgpu::Queue)> {
        let prepared = PREPARED
            .with(|cell| cell.borrow_mut().take())
            .context("the WebGPU device was not prepared, or was already taken")?;
        Ok((
            adapter::Chosen {
                adapter: prepared.adapter,
                offered: Vec::new(),
                in_use: Vec::new(),
            },
            prepared.device,
            prepared.queue,
        ))
    }

    /// Draws into the page's own canvas rather than one winit would make.
    pub(crate) fn canvas(
        attributes: winit::window::WindowAttributes,
    ) -> winit::window::WindowAttributes {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowAttributesExtWebSys;
        let canvas = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id(CANVAS_ID))
            .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok());
        attributes
            .with_canvas(canvas)
            .with_focusable(true)
            .with_prevent_default(true)
    }
}
