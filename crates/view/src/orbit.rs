//! Interactive orbit-camera window for a decoded model (`--mesh` / `--track`).
//!
//! Draws through the same pipeline [`mesh_render::build`] builds for the
//! offscreen capture path, aimed at a real surface instead of a readback
//! buffer, with yaw, pitch and zoom driven continuously by held keys instead
//! of one fixed angle passed on the command line.
//!
//! Only the window is here. The camera maths is
//! [`oag_mesh::orbit`], which knows nothing about `winit` and is
//! tested without one; this module just maps real keys onto its [`Held`] and
//! feeds it the elapsed time.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use log::{debug, error};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use oag_mesh::mesh::Model;
use oag_mesh::mesh_render::{self, Anisotropy, DEPTH_FORMAT, UNIFORMS_SIZE};
use oag_mesh::orbit::{Held, Orbit, PITCH_LIMIT, ZOOM_RANGE, advance};

/// What a drag of one window height turns the camera through, in radians.
///
/// A screen-relative rate rather than a per-pixel one, so the same gesture
/// covers the same arc whatever the window is: dragging the full height sweeps
/// most of a half-turn.
const DRAG_ROTATE: f32 = 2.5;

/// What a drag of one window height pans, in bounding-sphere radii.
///
/// Chosen so the model tracks the cursor roughly one-for-one at the default
/// framing, which is the only thing that makes a pan drag feel like dragging
/// rather than like nudging.
const DRAG_PAN: f32 = 1.6;

/// What one wheel notch does to the zoom multiplier.
const WHEEL_ZOOM: f32 = 0.1;

/// Pixels per wheel notch, for the trackpads that report a pixel delta rather
/// than a line count.
const PIXELS_PER_NOTCH: f32 = 40.0;

/// Which mouse button is dragging, and where the cursor was last seen.
#[derive(Debug, Default)]
struct Drag {
    /// The camera turns while the left button is down.
    rotating: bool,
    /// The look-at point moves while the right or middle button is down.
    panning: bool,
    /// The previous cursor position, so a move is a delta. `None` until the
    /// first move of a drag, because the position at button-down is not
    /// reported with the button.
    last: Option<(f32, f32)>,
}

/// Opens a window and orbits `model` under keyboard and mouse control until
/// closed.
///
/// Keys: Left/Right rotate, Up/Down pitch, `W`/`A`/`S`/`D` pan, `+`/`-` (or
/// PageUp/PageDown) zoom, `R` recentres, Escape quits.
///
/// Mouse: drag left to rotate, drag right or middle to pan, wheel to zoom.
pub fn run(model: Model, yaw: f32, pitch: f32, anisotropy: Anisotropy) -> Result<()> {
    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the camera moves continuously while a key is held.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        model: Some(model),
        orbit: Orbit {
            yaw,
            pitch: pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT),
            ..Orbit::default()
        },
        anisotropy,
        state: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    model: Option<Model>,
    orbit: Orbit,
    anisotropy: Anisotropy,
    state: Option<Session>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let Some(model) = self.model.take() else {
            event_loop.exit();
            return;
        };

        match Session::new(event_loop, model, self.orbit, self.anisotropy) {
            Ok(session) => self.state = Some(session),
            Err(e) => {
                error!("{e:#}");
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

            WindowEvent::MouseInput { button, state, .. } => {
                session.button(button, state == ElementState::Pressed);
            }

            WindowEvent::CursorMoved { position, .. } => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a cursor position in a window fits an f32 exactly"
                )]
                session.cursor(position.x as f32, position.y as f32);
            }

            WindowEvent::CursorLeft { .. } => session.drag_ended(),

            // A key held while the window loses focus is never seen to come up,
            // and the camera would orbit for ever while the user is elsewhere.
            // `oag-game` has fixed this class since `release_all` landed
            // (`crates/game/src/main/app.rs`); the viewer never got it -
            // finding U2 of the 2026-08-18 review. The drag goes with it, for
            // the same reason a `CursorLeft` ends one.
            WindowEvent::Focused(false) => {
                session.held = Held::default();
                session.drag_ended();
            }

            WindowEvent::MouseWheel { delta, .. } => session.wheel(delta),

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
                    error!("render error: {e:#}");
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

/// Everything that only exists once there is a window.
struct Session {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    alpha_test_pipeline: wgpu::RenderPipeline,
    /// One per alpha-test reference this model's own batches ask for - see
    /// `mesh_render::Built::cutout_pipelines`.
    cutout_pipelines: Vec<(f32, wgpu::RenderPipeline)>,
    blend_pipeline: [wgpu::RenderPipeline; 2],
    additive_pipeline: [wgpu::RenderPipeline; 2],
    unblended_pipeline: [wgpu::RenderPipeline; 2],
    /// One pair per equation this model's own file authors - see
    /// `mesh_render::Built::authored_pipelines`.
    authored_pipelines: Vec<(wgpu::BlendState, [wgpu::RenderPipeline; 2])>,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    texture_binds: Vec<wgpu::BindGroup>,
    /// Bind group 2. The viewer never fogs, so this stays at
    /// [`mesh_render::Fog::off`] for the session's lifetime.
    fog_bind: wgpu::BindGroup,
    /// Bind group 3, rewritten every frame from the model's authored
    /// texture-transform tracks.
    anim_bind: wgpu::BindGroup,
    anim_buffer: wgpu::Buffer,
    node_anim_buffer: wgpu::Buffer,
    model: Model,
    orbit: Orbit,
    held: Held,
    /// Which mouse button is dragging, and from where.
    drag: Drag,
    last: Instant,
    /// When this session opened, driving the texture animation.
    ///
    /// Wall clock, unlike the race, which drives the same tracks off
    /// `world.tick`. Nothing here is simulated or replayed, so there is no
    /// determinism claim to break - and an asset viewer that showed animated
    /// surfaces frozen would hide exactly what it exists to show.
    opened: Instant,
}

impl Session {
    fn new(
        event_loop: &ActiveEventLoop,
        model: Model,
        orbit: Orbit,
        anisotropy: Anisotropy,
    ) -> Result<Self> {
        let attributes = Window::default_attributes()
            .with_title(format!("oag-view - {}", model.label))
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 800));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("creating the window")?,
        );
        // **The cursor stays visible**, because the pointer is an input here:
        // dragging orbits and the wheel zooms, beside the keyboard's arrows,
        // `+`/`-` and PageUp/PageDown. This hid the pointer behind a comment
        // saying orbiting is keyboard-only, which stopped being true when drag
        // and wheel landed in this same file - finding U8 of the 2026-08-18
        // review. Hiding the thing the user is dragging with is the one
        // arrangement that cannot be right.

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .context("creating the surface")?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .context("no suitable GPU adapter (is a Vulkan driver installed?)")?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &oag_mesh::mesh_render::device_descriptor("oag-view orbit", &adapter),
        ))
        .context("requesting the device")?;

        let size = window.inner_size();
        // The viewer draws through the same pipeline the game does, so it
        // works in the same space: gamma, with nothing encoding on write. See
        // [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
        // A viewer left on an sRGB surface would show every mesh differently
        // from the game and stop being usable as a reference, which is the
        // whole point of it.
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by this adapter")?;
        config.format = config.format.remove_srgb_suffix();
        config.view_formats = vec![config.format];
        surface.configure(&device, &config);

        let mesh_render::Built {
            pipeline,
            alpha_test_pipeline,
            cutout_pipelines,
            blend_pipeline,
            additive_pipeline,
            unblended_pipeline,
            authored_pipelines,
            bind_group: placeholder_bind_group,
            vertex_buffer,
            index_buffer,
            texture_binds,
            fog_bind,
            fog_buffer: _,
            zone_vis_texture: _,
            zone_rebind: _,
            anim_bind,
            anim_buffer,
            node_anim_buffer,
            emissive_buffer: _,
            stamp_pipeline: _,
            prepass: _,
        } = mesh_render::build(
            &device,
            &queue,
            &model,
            config.format,
            anisotropy,
            1,
            mesh_render::Depth::Scene,
            mesh_render::TRANSPARENT_BLEND,
            // The asset viewer runs no bloom, so the mask is moot here, and
            // it draws into one target, so there is no velocity buffer.
            mesh_render::GlowMask::Protected,
            mesh_render::Velocity::None,
            // The asset viewer draws a model, not a race, so no Zone stage.
            &mesh_render::zone::StageArt::NONE,
            // And no shadow map, and nothing receiving one: a viewer shows a
            // model against nothing, which is what makes it a reference.
            mesh_render::ShadowMaps::NONE,
            mesh_render::ShadowReceiver::Never,
        )?;
        let _ = placeholder_bind_group;

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbit uniforms"),
            size: UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("orbit uniforms"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let depth_view = make_depth_view(&device, config.width, config.height);

        Ok(Self {
            window,
            device,
            queue,
            surface,
            config,
            depth_view,
            pipeline,
            alpha_test_pipeline,
            cutout_pipelines,
            blend_pipeline,
            additive_pipeline,
            unblended_pipeline,
            authored_pipelines,
            uniform_buffer,
            bind_group,
            vertex_buffer,
            index_buffer,
            texture_binds,
            fog_bind,
            anim_bind,
            anim_buffer,
            node_anim_buffer,
            model,
            orbit,
            held: Held::default(),
            drag: Drag::default(),
            last: Instant::now(),
            opened: Instant::now(),
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = make_depth_view(&self.device, width, height);
    }

    fn key(&mut self, key: &Key, down: bool) {
        match key {
            Key::Named(NamedKey::ArrowLeft) => self.held.yaw_neg = down,
            Key::Named(NamedKey::ArrowRight) => self.held.yaw_pos = down,
            Key::Named(NamedKey::ArrowUp) => self.held.pitch_pos = down,
            Key::Named(NamedKey::ArrowDown) => self.held.pitch_neg = down,
            Key::Named(NamedKey::PageUp) => self.held.zoom_in = down,
            Key::Named(NamedKey::PageDown) => self.held.zoom_out = down,
            Key::Character(c) => match c.to_ascii_lowercase().as_str() {
                "+" | "=" => self.held.zoom_in = down,
                "-" | "_" => self.held.zoom_out = down,
                // Panning on WASD rather than on more arrows: the arrows are
                // taken, and a viewer that opens a 2,370-unit circuit needs to
                // reach a corner of it without leaving the keyboard.
                "a" => self.held.pan_left = down,
                "d" => self.held.pan_right = down,
                "w" => self.held.pan_up = down,
                "s" => self.held.pan_down = down,
                // A way back. Without one a pan far enough to lose the model
                // leaves nothing on screen to steer by.
                "r" if down => self.orbit = self.orbit.recentred(),
                _ => {}
            },
            _ => {}
        }
    }

    fn button(&mut self, button: MouseButton, down: bool) {
        match button {
            MouseButton::Left => self.drag.rotating = down,
            MouseButton::Right | MouseButton::Middle => self.drag.panning = down,
            _ => return,
        }
        if !(self.drag.rotating || self.drag.panning) {
            self.drag_ended();
        }
    }

    /// Ends any drag in progress, so the next one starts from its own first
    /// move rather than from wherever the cursor was last seen.
    fn drag_ended(&mut self) {
        self.drag = Drag::default();
    }

    fn cursor(&mut self, x: f32, y: f32) {
        let previous = self.drag.last.replace((x, y));
        if !(self.drag.rotating || self.drag.panning) {
            return;
        }
        let Some((px, py)) = previous else {
            // The first move of a drag has no delta to take; it establishes the
            // origin the next one is measured from.
            return;
        };

        // Both deltas are in window heights, so a gesture means the same thing
        // whatever the window is - and using the *height* for both keeps a
        // diagonal drag diagonal instead of skewing with the aspect ratio.
        let height = self.config.height.max(1) as f32;
        let (dx, dy) = ((x - px) / height, (y - py) / height);

        if self.drag.rotating {
            // Dragging right turns the model right, which means moving the eye
            // the other way. Dragging down pitches the camera up, for the same
            // reason: the cursor holds the model, not the camera.
            self.orbit.yaw -= dx * DRAG_ROTATE;
            self.orbit.pitch =
                (self.orbit.pitch + dy * DRAG_ROTATE).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        }
        if self.drag.panning {
            // Same convention: the point under the cursor comes with it, so the
            // look-at point moves opposite the drag.
            self.orbit = self.orbit.panned(-dx * DRAG_PAN, dy * DRAG_PAN);
        }
    }

    fn wheel(&mut self, delta: MouseScrollDelta) {
        let notches = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a scroll delta in pixels is far inside f32"
            )]
            MouseScrollDelta::PixelDelta(p) => p.y as f32 / PIXELS_PER_NOTCH,
        };
        self.orbit.zoom =
            (self.orbit.zoom - notches * WHEEL_ZOOM).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
    }

    fn frame(&mut self) -> Result<()> {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;

        self.orbit = advance(self.orbit, &self.held, dt);

        let anims = mesh_render::TexAnims::sample(
            &self.model,
            now.duration_since(self.opened).as_secs_f32(),
        );
        self.queue
            .write_buffer(&self.anim_buffer, 0, bytemuck::bytes_of(&anims));
        // The viewer's clock moves a track's `Anim Transform` nodes too, so a
        // circuit opened in `oag-view` shows the same scenery motion a race
        // does rather than a frozen first frame.
        let nodes = mesh_render::NodeAnims::sample(
            &self.model,
            now.duration_since(self.opened).as_secs_f32(),
        );
        self.queue
            .write_buffer(&self.node_anim_buffer, 0, bytemuck::bytes_of(&nodes));

        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        mesh_render::write_uniforms(
            &self.queue,
            &self.uniform_buffer,
            &self.model,
            aspect,
            self.orbit,
        );

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            other => {
                debug!("skipping frame: {other:?}");
                return Ok(());
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("orbit frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("orbit frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.06,
                            g: 0.07,
                            b: 0.09,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // `Buffer::slice` panics on a zero-length buffer, so a model with no
            // geometry skips every command below and the frame is the clear
            // colour alone. Matches the guard in `mesh_render::capture_from`.
            if !self.model.vertices.is_empty() && !self.model.indices.is_empty() {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                // The asset viewer never fogs: it shows what is on the disc, and fog
                // is a property of the race the asset sits in, not of the asset.
                pass.set_bind_group(2, &self.fog_bind, &[]);
                pass.set_bind_group(3, &self.anim_bind, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                // One draw per material run. Slot 0 is the white fallback, so a
                // texture index of n binds slot n + 1. Matches `mesh_render::capture_from`.
                for draw in &self.model.draws {
                    let slot = draw.texture.map_or(0, |t| t + 1);
                    pass.set_bind_group(
                        1,
                        &self.texture_binds[slot.min(self.texture_binds.len() - 1)],
                        &[],
                    );
                    pass.draw_indexed(draw.range.clone(), 0, 0..1);
                }

                // Second pipeline group, same pass: alpha-tested batches,
                // cutout. One pipeline per reference the batches themselves
                // ask for, switched only when it changes - see
                // `mesh_render::cutout`.
                let cutouts = mesh_render::CutoutPipelines {
                    default: &self.alpha_test_pipeline,
                    by_reference: &self.cutout_pipelines,
                };
                let mut cutout_set: Option<&wgpu::RenderPipeline> = None;
                for draw in &self.model.alpha_tested_draws {
                    let cutout = cutouts.select(draw);
                    if !cutout_set.is_some_and(|set| std::ptr::eq(set, cutout)) {
                        pass.set_pipeline(cutout);
                        cutout_set = Some(cutout);
                    }
                    let slot = draw.texture.map_or(0, |t| t + 1);
                    pass.set_bind_group(
                        1,
                        &self.texture_binds[slot.min(self.texture_binds.len() - 1)],
                        &[],
                    );
                    pass.draw_indexed(draw.range.clone(), 0, 0..1);
                }

                // Third pipeline group, same pass: transparent batches,
                // blended, with the equation the batch's own `pass_mask` asks
                // for - see `oag_vex::vex::Batch::blend_class`. The viewer
                // selects per draw call exactly as the game does, because it
                // is only useful as a reference if it renders the same way.
                let pipelines = mesh_render::TransparentPipelines {
                    alpha_over: &self.blend_pipeline,
                    additive: &self.additive_pipeline,
                    unblended: &self.unblended_pipeline,
                    authored: &self.authored_pipelines,
                };
                let mut current: Option<&wgpu::RenderPipeline> = None;
                for draw in &self.model.transparent_draws {
                    let pipeline = pipelines.select(draw);
                    if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                        pass.set_pipeline(pipeline);
                        current = Some(pipeline);
                    }
                    let slot = draw.texture.map_or(0, |t| t + 1);
                    pass.set_bind_group(
                        1,
                        &self.texture_binds[slot.min(self.texture_binds.len() - 1)],
                        &[],
                    );
                    pass.draw_indexed(draw.range.clone(), 0, 0..1);
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }
}

fn make_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("orbit depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    depth.create_view(&wgpu::TextureViewDescriptor::default())
}
