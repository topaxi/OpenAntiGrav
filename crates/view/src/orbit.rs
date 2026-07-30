//! Interactive orbit-camera window for a decoded model (`--mesh` / `--track`).
//!
//! Draws through the same pipeline [`mesh_render::build`] builds for the
//! offscreen capture path, aimed at a real surface instead of a readback
//! buffer, with yaw, pitch and zoom driven continuously by held keys instead
//! of one fixed angle passed on the command line.
//!
//! Only the window is here. The camera maths is
//! [`oag_render::camera::orbit`], which knows nothing about `winit` and is
//! tested without one; this module just maps real keys onto its [`Held`] and
//! feeds it the elapsed time.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use oag_render::camera::orbit::{Held, Orbit, PITCH_LIMIT, advance};
use oag_render::mesh::Model;
use oag_render::mesh_render::{self, Anisotropy, DEPTH_FORMAT, UNIFORMS_SIZE};

/// Opens a window and orbits `model` under keyboard control until closed.
///
/// Left/Right rotate, Up/Down pitch, `+`/`-` (or PageUp/PageDown) zoom, Escape
/// quits.
pub fn run(model: Model, yaw: f32, pitch: f32, anisotropy: Anisotropy) -> Result<()> {
    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the camera moves continuously while a key is held.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        model: Some(model),
        orbit: Orbit {
            yaw,
            pitch: pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT),
            zoom: 1.0,
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
                    eprintln!("render error: {e:#}");
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
    blend_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    texture_binds: Vec<wgpu::BindGroup>,
    model: Model,
    orbit: Orbit,
    held: Held,
    last: Instant,
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
            label: Some("oag-view orbit"),
            ..Default::default()
        }))
        .context("requesting the device")?;

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by this adapter")?;
        surface.configure(&device, &config);

        let mesh_render::Built {
            pipeline,
            alpha_test_pipeline,
            blend_pipeline,
            bind_group: placeholder_bind_group,
            vertex_buffer,
            index_buffer,
            texture_binds,
        } = mesh_render::build(&device, &queue, &model, config.format, anisotropy)?;
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
            blend_pipeline,
            uniform_buffer,
            bind_group,
            vertex_buffer,
            index_buffer,
            texture_binds,
            model,
            orbit,
            held: Held::default(),
            last: Instant::now(),
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
            Key::Character(c) => match c.as_str() {
                "+" | "=" => self.held.zoom_in = down,
                "-" | "_" => self.held.zoom_out = down,
                _ => {}
            },
            _ => {}
        }
    }

    fn frame(&mut self) -> Result<()> {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;

        self.orbit = advance(self.orbit, &self.held, dt);

        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        mesh_render::write_uniforms(
            &self.queue,
            &self.uniform_buffer,
            &self.model,
            aspect,
            self.orbit.yaw,
            self.orbit.pitch,
            self.orbit.zoom,
            0.0,
        );

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
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
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

            // Second pipeline, same pass: alpha-tested batches, cutout. See
            // `mesh_render::Built::alpha_test_pipeline`.
            pass.set_pipeline(&self.alpha_test_pipeline);
            for draw in &self.model.alpha_tested_draws {
                let slot = draw.texture.map_or(0, |t| t + 1);
                pass.set_bind_group(
                    1,
                    &self.texture_binds[slot.min(self.texture_binds.len() - 1)],
                    &[],
                );
                pass.draw_indexed(draw.range.clone(), 0, 0..1);
            }

            // Third pipeline, same pass: transparent batches, blended. See
            // `mesh_render::Built::blend_pipeline`.
            pass.set_pipeline(&self.blend_pipeline);
            for draw in &self.model.transparent_draws {
                let slot = draw.texture.map_or(0, |t| t + 1);
                pass.set_bind_group(
                    1,
                    &self.texture_binds[slot.min(self.texture_binds.len() - 1)],
                    &[],
                );
                pass.draw_indexed(draw.range.clone(), 0, 0..1);
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
