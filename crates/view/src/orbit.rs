//! Interactive orbit-camera window for a decoded model (`--mesh` / `--track`).
//!
//! Draws through the same pipeline [`mesh_render::build`] builds for the
//! offscreen capture path, aimed at a real surface instead of a readback
//! buffer, with yaw, pitch and zoom driven continuously by held keys instead
//! of one fixed angle passed on the command line.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::mesh::Model;
use crate::mesh_render::{self, DEPTH_FORMAT, UNIFORMS_SIZE};

/// Radians per second the arrow keys rotate the camera.
const ROTATE_RATE: f32 = 1.2;

/// Zoom multiplier change per second while a zoom key is held.
const ZOOM_RATE: f32 = 0.8;

/// Keeps the camera short of straight down or up, where `look_at`'s up vector
/// degenerates and the orbit flips.
const PITCH_LIMIT: f32 = 1.5;

/// How close and how far the zoom multiplier is allowed to go. 1.0 is the
/// bounding-sphere framing [`mesh_render`] uses by default.
const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.3..=4.0;

/// Opens a window and orbits `model` under keyboard control until closed.
///
/// Left/Right rotate, Up/Down pitch, `+`/`-` (or PageUp/PageDown) zoom, Escape
/// quits.
pub fn run(model: Model, yaw: f32, pitch: f32) -> Result<()> {
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
        state: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// Which camera keys are currently down.
#[derive(Default)]
struct Held {
    yaw_neg: bool,
    yaw_pos: bool,
    pitch_neg: bool,
    pitch_pos: bool,
    zoom_in: bool,
    zoom_out: bool,
}

/// The orbit camera's yaw, pitch and zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Orbit {
    yaw: f32,
    pitch: f32,
    zoom: f32,
}

/// Advances `orbit` by `dt` seconds according to which keys are held.
///
/// Pitch clamps short of straight down or up, where `mesh_render`'s `look_at`
/// up vector degenerates and the orbit flips. Zoom clamps to [`ZOOM_RANGE`].
/// Pure so the clamping can be tested without a window or a GPU.
fn advance(orbit: Orbit, held: &Held, dt: f32) -> Orbit {
    let mut yaw = orbit.yaw;
    let mut pitch = orbit.pitch;
    let mut zoom = orbit.zoom;

    if held.yaw_neg {
        yaw -= ROTATE_RATE * dt;
    }
    if held.yaw_pos {
        yaw += ROTATE_RATE * dt;
    }
    if held.pitch_pos {
        pitch += ROTATE_RATE * dt;
    }
    if held.pitch_neg {
        pitch -= ROTATE_RATE * dt;
    }
    pitch = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);

    if held.zoom_in {
        zoom -= ZOOM_RATE * dt;
    }
    if held.zoom_out {
        zoom += ZOOM_RATE * dt;
    }
    zoom = zoom.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());

    Orbit { yaw, pitch, zoom }
}

struct App {
    model: Option<Model>,
    orbit: Orbit,
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

        match Session::new(event_loop, model, self.orbit) {
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
    fn new(event_loop: &ActiveEventLoop, model: Model, orbit: Orbit) -> Result<Self> {
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

        let (pipeline, placeholder_bind_group, vertex_buffer, index_buffer, texture_binds) =
            mesh_render::build(&device, &queue, &model, config.format)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn held(set: impl FnOnce(&mut Held)) -> Held {
        let mut held = Held::default();
        set(&mut held);
        held
    }

    #[test]
    fn no_keys_held_leaves_the_camera_still() {
        let start = Orbit {
            yaw: 0.1,
            pitch: 0.2,
            zoom: 1.5,
        };
        assert_eq!(advance(start, &Held::default(), 1.0), start);
    }

    #[test]
    fn zero_dt_leaves_the_camera_still_even_with_keys_held() {
        let start = Orbit {
            yaw: 0.1,
            pitch: 0.2,
            zoom: 1.5,
        };
        let held = held(|h| {
            h.yaw_pos = true;
            h.pitch_pos = true;
            h.zoom_in = true;
        });
        assert_eq!(advance(start, &held, 0.0), start);
    }

    #[test]
    fn yaw_left_and_right_move_opposite_ways() {
        let start = Orbit {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
        };
        let right = advance(start, &held(|h| h.yaw_pos = true), 1.0);
        let left = advance(start, &held(|h| h.yaw_neg = true), 1.0);
        assert!(right.yaw > start.yaw);
        assert!(left.yaw < start.yaw);
        assert_eq!(right.yaw, -left.yaw);
    }

    #[test]
    fn opposite_keys_held_together_cancel() {
        let start = Orbit {
            yaw: 0.3,
            pitch: 0.0,
            zoom: 1.0,
        };
        let held = held(|h| {
            h.yaw_neg = true;
            h.yaw_pos = true;
        });
        assert_eq!(advance(start, &held, 1.0), start);
    }

    #[test]
    fn pitch_clamps_short_of_vertical_however_long_held() {
        let start = Orbit {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
        };
        let up = advance(start, &held(|h| h.pitch_pos = true), 1000.0);
        assert_eq!(up.pitch, PITCH_LIMIT);
        let down = advance(start, &held(|h| h.pitch_neg = true), 1000.0);
        assert_eq!(down.pitch, -PITCH_LIMIT);
    }

    #[test]
    fn zoom_clamps_to_its_range_however_long_held() {
        let start = Orbit {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
        };
        let close = advance(start, &held(|h| h.zoom_in = true), 1000.0);
        assert_eq!(close.zoom, *ZOOM_RANGE.start());
        let far = advance(start, &held(|h| h.zoom_out = true), 1000.0);
        assert_eq!(far.zoom, *ZOOM_RANGE.end());
    }
}
