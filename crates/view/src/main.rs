//! Views assets decoded straight from a Wipeout disc image.
//!
//! ```sh
//! oag-view data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad
//! ```
//!
//! Left and right arrows change asset, Escape quits. Nothing is written to
//! disk, and the disc image is opened read-only.
//!
//! This is the first thing in the project with a window, and it exists to prove
//! the whole pipeline end to end: CHD, ISO 9660, WAD, LZSS, texture decode,
//! GPU. If it draws the right picture, every layer beneath it is right.

mod assets;
mod mesh;
mod mesh_render;
mod offscreen;
mod track;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[derive(Parser, Debug)]
#[command(
    name = "oag-view",
    about = "View Wipeout assets from a disc image",
    version
)]
struct Cli {
    /// `<image>:<path-on-disc>`, for example
    /// `data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad`.
    archive: String,

    /// A file of candidate names, one per line, used to label assets.
    #[arg(long)]
    names: Option<PathBuf>,

    /// Render one asset to a PNG and exit, without opening a window.
    ///
    /// Runs headless, so it works over SSH and in CI, and gives the render
    /// path something a test can assert on.
    #[arg(long)]
    screenshot: Option<PathBuf>,

    /// Which asset the screenshot uses.
    #[arg(long, default_value_t = 0)]
    index: usize,

    /// Render a `.vex` model instead of textures, by its name in the archive.
    #[arg(long)]
    mesh: Option<String>,

    /// Render a track's driveable spline instead of its art meshes, by the
    /// `.vex` name in the archive.
    ///
    /// Draws the track surface from the spline's own half-widths, coloured per
    /// visibility section, with the racing line and AI corridor at hover height
    /// above it.
    #[arg(long)]
    track: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(name) = &cli.track {
        let (ai, label) = track::load(&cli.archive, name)?;
        println!(
            "{label}: version {:#x}, {} paths, {} junctions, {} control points",
            ai.version,
            ai.paths.len(),
            ai.junctions.len(),
            ai.point_count()
        );
        for (i, path) in ai.paths.iter().enumerate() {
            println!(
                "  path {i}: {} points, longest gap {:.2}, junctions {:?} -> {:?}",
                path.points.len(),
                path.max_spacing,
                path.entry,
                path.exit
            );
        }
        let model = track::build_model(&label, &ai);
        let path = cli
            .screenshot
            .clone()
            .context("--track currently requires --screenshot")?;
        // Tracks are flat and wide, so look down at them rather than along.
        mesh_render::capture_from(&model, &path, 1280, 960, 0.9, 1.15)?;
        println!("wrote {}", path.display());
        return Ok(());
    }

    if let Some(name) = &cli.mesh {
        let model = mesh::load(&cli.archive, name)?;
        println!(
            "{}: {} meshes, {} vertices, {} triangles, radius {:.2}",
            model.label,
            model.mesh_count,
            model.vertices.len(),
            model.indices.len() / 3,
            model.radius
        );
        let path = cli
            .screenshot
            .clone()
            .context("--mesh currently requires --screenshot")?;
        mesh_render::capture(&model, &path, 960, 720)?;
        println!("wrote {}", path.display());
        return Ok(());
    }

    let assets = assets::load(&cli.archive, cli.names.as_deref())?;
    println!("{} texture(s) loaded from {}", assets.len(), cli.archive);

    if let Some(path) = cli.screenshot {
        let asset = assets
            .get(cli.index)
            .with_context(|| format!("index {} of {}", cli.index, assets.len()))?;
        offscreen::capture(asset, &path)?;
        println!(
            "wrote {} ({}x{})",
            path.display(),
            asset.width,
            asset.height
        );
        return Ok(());
    }

    println!("left/right to change, escape to quit");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        assets,
        current: 0,
        state: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    assets: Vec<assets::Asset>,
    current: usize,
    state: Option<Renderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("oag-view")
            .with_inner_size(winit::dpi::LogicalSize::new(960, 640));

        match event_loop
            .create_window(attributes)
            .context("creating the window")
            .and_then(|window| Renderer::new(Arc::new(window)))
        {
            Ok(mut renderer) => {
                renderer.show(&self.assets[self.current]);
                self.state = Some(renderer);
            }
            Err(e) => {
                eprintln!("error: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(renderer) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                renderer.resize(size.width, size.height);
                renderer.window.request_redraw();
            }

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let count = self.assets.len();
                let step = match event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                        return;
                    }
                    // Wrapping both ways, so browsing never dead-ends.
                    Key::Named(NamedKey::ArrowRight) => 1,
                    Key::Named(NamedKey::ArrowLeft) => count - 1,
                    _ => return,
                };

                self.current = (self.current + step) % count;
                renderer.show(&self.assets[self.current]);
                renderer.window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                if let Err(e) = renderer.render() {
                    eprintln!("render error: {e:#}");
                    event_loop.exit();
                }
            }

            _ => {}
        }
    }
}

/// Uniforms shared with `shader.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    scale: [f32; 2],
    surface: [f32; 2],
}

struct Renderer {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
    /// Aspect ratio of the current texture, for letterboxing.
    aspect: f32,
}

impl Renderer {
    fn new(window: Arc<Window>) -> Result<Self> {
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
            label: Some("oag-view"),
            ..Default::default()
        }))
        .context("requesting the device")?;

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by this adapter")?;
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewer"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewer"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewer"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewer"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(config.format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Nearest filtering: these are small pixel-art textures, and smoothing
        // them would hide exactly the decoding errors this tool exists to find.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("viewer"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewer uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            window,
            device,
            queue,
            surface,
            config,
            pipeline,
            layout,
            sampler,
            uniform_buffer,
            bind_group: None,
            aspect: 1.0,
        })
    }

    /// Uploads an asset and makes it current.
    fn show(&mut self, asset: &assets::Asset) {
        let size = wgpu::Extent3d {
            width: asset.width,
            height: asset.height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("asset"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Srgb: the palettes hold non-linear colour, so a linear format
            // would render everything washed out.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            texture.as_image_copy(),
            &asset.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(asset.width * 4),
                rows_per_image: Some(asset.height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("asset"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));

        self.aspect = asset.width as f32 / asset.height as f32;
        self.window.set_title(&format!(
            "oag-view - {}  {}x{}",
            asset.label, asset.width, asset.height
        ));
        println!("{}  {}x{}", asset.label, asset.width, asset.height);
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&mut self) -> Result<()> {
        let Some(bind_group) = &self.bind_group else {
            return Ok(());
        };

        // Letterbox: shrink along whichever axis would otherwise stretch, so
        // the texture keeps its proportions whatever the window is doing.
        let surface_aspect = self.config.width as f32 / self.config.height as f32;
        let scale = if surface_aspect > self.aspect {
            [self.aspect / surface_aspect, 1.0]
        } else {
            [1.0, surface_aspect / self.aspect]
        };

        let uniforms = Uniforms {
            scale,
            surface: [self.config.width as f32, self.config.height as f32],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // A frame can legitimately be unavailable: the surface may be occluded,
        // outdated after a resize, or timed out. None of those are errors, so
        // skip the frame rather than tearing the window down.
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

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }
}
