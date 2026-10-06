//! The HD-style menu backdrop's GPU side: the scene drawn into a target
//! cleared to white, filtered to a grey drawing, blurred, and composited.
//!
//! `oag_ui::scene_backdrop` decides what a frame looks like and hands over a
//! [`Frame`]; this draws it. Per frame, before the page's own render pass:
//!
//! 1. **Scene** - the model, through its own camera at the frame's moment,
//!    into a target cleared to white (`0xffffff`, the clear `Render` sets).
//! 2. **Edge** - `FEBackgroundAnim_fp`'s Roberts cross and fill, into a grey
//!    target.
//! 3. **Blur** - the page's `blur` pixels at 1080 lines, two axes.
//!
//! Then, inside the page's pass where the draw list's
//! [`oag_ui::frontend::Draw::SceneBackdrop`] sits:
//!
//! 4. **Copy** - the grey over the page, opaque.
//!
//! The scene pass is [`crate::preview::Preview`]'s own, so the model is drawn
//! the way every other menu mesh is. **The blur kernel is chosen, not
//! measured**: the engine's blur (`0x003e3e50`) is a chain of offset passes
//! whose weights are unread, and this is a gaussian of the same radius.

use oag_core::math::Mat4;
use oag_display::space::Space;
use oag_mesh::mesh::Model;
use oag_mesh::mesh_render::Anisotropy;
use oag_ui::scene_backdrop::Frame;

use super::resources::{sampler_entry, texture_entry, uniform_entry};
use crate::preview::Preview;

/// The offscreen targets' format: what the scene is drawn in and the filter
/// reads, an 8-bit target as the original's.
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The lines the blur radius is authored at: `Update` scales `blur` by
/// `height / 1080` (the literal `1/1080` at `0x008ac0e8`).
const AUTHORED_LINES: f32 = 1080.0;

/// The most taps each side a blur takes, so a huge window cannot ask for a
/// loop the GPU would pay for every pixel.
const MAX_REACH: f32 = 24.0;

/// One pass's constants, `scene.wesl`'s `Params`.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    taps: [f32; 4],
    levels: [f32; 4],
}

struct Targets {
    size: (u32, u32),
    scene: wgpu::TextureView,
    grey: wgpu::TextureView,
    blurred: wgpu::TextureView,
}

/// The backdrop's GPU resources.
pub struct SceneBackdrop {
    preview: Preview,
    edge_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    copy_pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    edge_params: wgpu::Buffer,
    blur_params: [wgpu::Buffer; 2],
    targets: Option<Targets>,
    copy_bind: Option<wgpu::BindGroup>,
}

impl std::fmt::Debug for SceneBackdrop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneBackdrop")
            .field("model", &self.preview)
            .finish_non_exhaustive()
    }
}

impl SceneBackdrop {
    /// Puts `model` on the GPU and builds the three pipelines, the copy's for a
    /// page of `page_format`.
    ///
    /// # Errors
    ///
    /// The model's pipelines will not build.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        page_format: wgpu::TextureFormat,
        model: Model,
    ) -> anyhow::Result<Self> {
        let preview = Preview::new(device, queue, TARGET_FORMAT, Anisotropy::default(), model)?;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene backdrop"),
            entries: &[
                uniform_entry(0),
                texture_entry(1),
                sampler_entry(2, wgpu::SamplerBindingType::Filtering),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene backdrop"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene backdrop"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/scene.wgsl")).into(),
            ),
        });
        let pipeline = |label: &str, entry: &str, format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_quad"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let params = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: std::mem::size_of::<Params>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        Ok(Self {
            preview,
            edge_pipeline: pipeline("scene edge", "fs_edge", TARGET_FORMAT),
            blur_pipeline: pipeline("scene blur", "fs_blur", TARGET_FORMAT),
            copy_pipeline: pipeline("scene copy", "fs_copy", page_format),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("scene backdrop"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            }),
            layout,
            edge_params: params("scene edge"),
            blur_params: [params("scene blur x"), params("scene blur y")],
            targets: None,
            copy_bind: None,
        })
    }

    /// The three offscreen passes for `frame`, into targets sized to the
    /// viewport. Encoded before the page's own pass, which [`Self::composite`]
    /// then reads from.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        viewport: (u32, u32),
    ) {
        let size = (viewport.0.max(1), viewport.1.max(1));
        if self.targets.as_ref().is_none_or(|t| t.size != size) {
            self.targets = Some(targets_for(device, size));
        }
        let (width, height) = (size.0 as f32, size.1 as f32);
        let targets = self.targets.as_ref().expect("built above");

        // 1. The scene, on white. `Preview` loads the colour it is given, so the
        // clear is its own pass.
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.scene,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        }));
        self.preview.draw_matrices(
            device,
            queue,
            encoder,
            &targets.scene,
            (0.0, 0.0, width, height),
            size,
            Space {
                size: (width, height),
                display_aspect: width / height,
            },
            Mat4::from_cols_array_2d(&frame.view_projection),
            Mat4::IDENTITY,
            frame.seconds,
        );

        // 2. The edge drawing, one texel of the target being the tap spacing's
        // unit.
        let levels = frame.look.main;
        let (dx, dy) = (levels.edge_width / width, levels.edge_width / height);
        queue.write_buffer(
            &self.edge_params,
            0,
            bytemuck::bytes_of(&Params {
                taps: [dx, dy, -0.5 * dx, -0.5 * dy],
                levels: [
                    levels.edge_level,
                    1.0 - levels.fill_level,
                    levels.fill_level,
                    0.0,
                ],
            }),
        );
        let bind = |label: &str, params: &wgpu::Buffer, source: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            })
        };
        let pass = |encoder: &mut wgpu::CommandEncoder,
                    label: &str,
                    pipeline: &wgpu::RenderPipeline,
                    bind: &wgpu::BindGroup,
                    target: &wgpu::TextureView| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind, &[]);
            pass.draw(0..3, 0..1);
        };
        let edge_bind = bind("scene edge", &self.edge_params, &targets.scene);
        pass(
            encoder,
            "scene edge",
            &self.edge_pipeline,
            &edge_bind,
            &targets.grey,
        );

        // 3. The blur, when the page asks for one: the authored radius scaled
        // to this target's height, a gaussian of half that sigma.
        let radius = frame.look.blur * height / AUTHORED_LINES;
        if radius >= 0.5 {
            let reach = radius.ceil().min(MAX_REACH);
            let sigma = (radius * 0.5).max(0.5);
            for (buffer, step) in self
                .blur_params
                .iter()
                .zip([[1.0 / width, 0.0], [0.0, 1.0 / height]])
            {
                queue.write_buffer(
                    buffer,
                    0,
                    bytemuck::bytes_of(&Params {
                        taps: [0.0, 0.0, sigma, reach],
                        levels: [step[0], step[1], 0.0, 0.0],
                    }),
                );
            }
            let across = bind("scene blur x", &self.blur_params[0], &targets.grey);
            pass(
                encoder,
                "scene blur x",
                &self.blur_pipeline,
                &across,
                &targets.blurred,
            );
            let down = bind("scene blur y", &self.blur_params[1], &targets.blurred);
            pass(
                encoder,
                "scene blur y",
                &self.blur_pipeline,
                &down,
                &targets.grey,
            );
        }
        // The picture is in `grey` either way: the blur's second pass returns it there.
        self.copy_bind = Some(bind("scene copy", &self.edge_params, &targets.grey));
    }

    /// Draws the prepared picture over the page, opaque.
    pub fn composite(&self, pass: &mut wgpu::RenderPass<'_>) {
        let Some(bind) = &self.copy_bind else {
            return;
        };
        pass.set_pipeline(&self.copy_pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn targets_for(device: &wgpu::Device, size: (u32, u32)) -> Targets {
    let target = |label: &str| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TARGET_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    Targets {
        size,
        scene: target("scene backdrop scene"),
        grey: target("scene backdrop grey"),
        blurred: target("scene backdrop blurred"),
    }
}
