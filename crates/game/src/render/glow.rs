//! The front end's frame-end bloom, over the glow mask its widgets stamp.
//!
//! Pulse PSP ends every front-end frame with `Bloom_Draw`, the same four passes the race runs
//! (`docs/ghidra/functions/psp-pulse-usa/bloom.md`, ported as `oag_post::bloom`). What differs is
//! the mask: a front-end widget stamps its quad's stencil reference into the framebuffer's alpha
//! (`Gu_StencilFunc(ALWAYS, ref)`, `REPLACE`, alpha test off), so every pixel of the quad takes
//! the reference, and the bright pass blooms `rgb * alpha`. Everything else holds the clear's `4`
//! (`docs/rendering/glow-mask.md`), which the blurs truncate to nothing.
//!
//! Ours draws the list a second time into a scratch target of the viewport's size, rewrites that
//! target's alpha to the mask ([`BACKGROUND`] then each [`Draw::GlowStamp`](oag_ui::frontend::Draw)
//! in list order, a pipeline that writes the alpha channel alone), runs `oag_post::bloom`'s bright
//! pass and blurs over it, and composites the result onto the real target after the list.
//!
//! Pulse PS2's front end blooms through its own chain over its own mask
//! ([`oag_title::MenuSkin::glow_by_texel`]): the alpha is cleared to `0`, a [`Stamp::Texel`] writes
//! the quad's texel alpha, and `oag_post::ps2_bloom` reads it.

use anyhow::Result;

use super::quad::{ATTRIBUTES, Quad};

/// What the frame clear leaves in the framebuffer's alpha: measured out of EDRAM on every Pulse
/// front-end screen (a `4` on 127,242 of the 130,560 visible pixels of Grid Selection).
pub(super) const BACKGROUND: u8 = 4;

/// What the PS2's frame clear leaves in the framebuffer's alpha (`docs/rendering/ps2-bloom.md`,
/// "The mask, measured": the PS2 has no floor of `4`).
pub(super) const PS2_BACKGROUND: u8 = 0;

/// The PS2 front end's frame, which its bloom composite's `-5, -5` is measured in.
const FRONT_END_FRAME: (f32, f32) = (640.0, 448.0);

/// A rectangle larger than any grid, so the background stamp covers the whole viewport.
const EVERYWHERE: [f32; 4] = [-65_536.0, -65_536.0, 131_072.0, 131_072.0];

/// The scratch frame and the bloom that reads it.
pub(super) struct Glow {
    pub(super) bloom: oag_post::bloom::Bloom,
    /// Pulse PS2's chain, built the first time a list's mask is the PS2's.
    pub(super) ps2: Option<oag_post::ps2_bloom::Ps2Bloom>,
    /// The ui vertex stage with `fs_stamp`, writing the alpha channel and nothing else.
    pipeline: wgpu::RenderPipeline,
    scratch: Option<Scratch>,
}

struct Scratch {
    #[expect(dead_code, reason = "held so the view stays valid")]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
}

impl Glow {
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui glow stamp"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/ui.wgsl")).into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui glow stamp"),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui glow stamp"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Quad>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &ATTRIBUTES,
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_stamp"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // Stencil REPLACE: the reference lands whatever is under it, and blending
                    // never writes the channel.
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALPHA,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            bloom: oag_post::bloom::Bloom::new(device, format)?,
            ps2: None,
            pipeline,
            scratch: None,
        })
    }

    /// The PS2's chain, built on first use.
    pub(super) fn ps2_bloom(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Result<&oag_post::ps2_bloom::Ps2Bloom> {
        if self.ps2.is_none() {
            self.ps2 = Some(oag_post::ps2_bloom::Ps2Bloom::with_frame(
                device,
                format,
                FRONT_END_FRAME,
            )?);
        }
        Ok(self.ps2.as_ref().expect("built above"))
    }

    /// The scratch target for a `size` viewport, rebuilt when the size changes.
    pub(super) fn scratch(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> &wgpu::TextureView {
        if self.scratch.as_ref().is_none_or(|s| s.size != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("ui glow scratch"),
                size: wgpu::Extent3d {
                    width: size.0.max(1),
                    height: size.1.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
                texture_binding_view_dimension: None,
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.scratch = Some(Scratch {
                texture,
                view,
                size,
            });
        }
        &self.scratch.as_ref().expect("built above").view
    }

    /// Rewrites the scratch frame's alpha to the mask: `quads` is the instance range
    /// [`mask_quads`] pushed, drawn in order.
    pub(super) fn stamp(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        quad_buffer: &wgpu::Buffer,
        range: std::ops::Range<u32>,
    ) {
        let Some(scratch) = &self.scratch else {
            return;
        };
        let stamp = oag_gpu::timing::stamp("hud glow mask");
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui glow mask"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &scratch.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: stamp.as_ref().map(oag_gpu::timing::Stamp::render_writes),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_vertex_buffer(0, quad_buffer.slice(..));
        pass.draw(0..6, range);
    }
}

/// One write into the glow mask.
#[derive(Clone, Copy, Debug)]
pub(super) enum Stamp {
    /// A rectangle at a stencil reference: the PSP's rule.
    Value([f32; 4], u8),
    /// A sprite's own quad at its texel alpha times its colour's alpha: the PS2's rule.
    Texel(Quad),
}

/// Whether `stamps` is a PS2 mask: one write by texel makes the whole frame's rule the PS2's.
pub(super) fn is_ps2_mask(stamps: &[Stamp]) -> bool {
    stamps.iter().any(|stamp| matches!(stamp, Stamp::Texel(_)))
}

/// The instances of a mask: the background, then each stamp in list order. A PS2 mask
/// ([`is_ps2_mask`]) clears to [`PS2_BACKGROUND`] and has no stencil references, so it drops the
/// [`Stamp::Value`]s.
pub(super) fn mask_quads(stamps: &[Stamp]) -> Vec<Quad> {
    let quad = |rect: [f32; 4], value: u8| Quad {
        rect,
        uv: [0.0; 4],
        color: [0.0, 0.0, 0.0, f32::from(value) / 255.0],
        border: [0.0; 4],
        mode: 0.0,
        rotation: 0.0,
        chamfer: [0.0; 2],
        tile: [0.0; 2],
        warp: [0.0; 4],
    };
    let ps2 = is_ps2_mask(stamps);
    let background = if ps2 { PS2_BACKGROUND } else { BACKGROUND };
    std::iter::once(quad(EVERYWHERE, background))
        .chain(stamps.iter().filter_map(|stamp| match *stamp {
            Stamp::Value(rect, value) => (!ps2).then(|| quad(rect, value)),
            Stamp::Texel(sprite) => Some(sprite),
        }))
        .collect()
}
