//! The pipelines that stamp a blended batch's glow byte into the mask.
//!
//! A blend state cannot write a constant alpha while the colour blend reads
//! the texel's own, so a transparent batch with the glow bits - which the
//! original stamps with the GE stencil under its blend - is submitted twice:
//! once through `TransparentPipelines` for its colour, and once through these
//! for its mask. See [`super::GlowMask::Stamped`] and
//! `docs/rendering/glow-mask.md`, "Transparent batches stamp".

use super::{Velocity, pipeline_cache, velocity_targets};

/// What [`pipelines`] shares with the pipelines [`super::build`] has already
/// made, borrowed so this does not grow `build`'s own argument list.
pub(super) struct Shared<'a> {
    pub device: &'a wgpu::Device,
    pub shader: &'a wgpu::ShaderModule,
    pub layout: &'a wgpu::PipelineLayout,
    pub vertex_buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    pub constants: &'a [(&'static str, f64)],
    pub velocity: Velocity,
    pub format: wgpu::TextureFormat,
    pub multisample: wgpu::MultisampleState,
    /// The blended pipelines' own: tested, never written.
    pub depth_stencil: &'a wgpu::DepthStencilState,
}

/// The stamp pipeline, `[0]` two-sided and `[1]` back-face culled like every
/// other blended pipeline, so a draw's `culled` bit picks the same face rule
/// for its mask as for its colour.
///
/// **Alpha only, no blend.** The write mask is `ALPHA`, so the colour the
/// fragment returns reaches nothing, and the blend is off so the alpha
/// channel is replaced rather than accumulated - the stencil's `REPLACE`. The
/// velocity attachment a race adds is masked empty, as it is for a blended
/// draw.
pub(super) fn pipelines(shared: &Shared<'_>) -> [wgpu::RenderPipeline; 2] {
    let entry = shared
        .velocity
        .entry("fs_main_stamp", "fs_main_stamp_velocity");
    let targets = velocity_targets(
        wgpu::ColorTargetState {
            format: shared.format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALPHA,
        },
        shared.velocity.target(true),
    );
    let make = |label: &str, cull: bool| {
        let primitive = wgpu::PrimitiveState {
            cull_mode: cull.then_some(wgpu::Face::Back),
            ..Default::default()
        };
        pipeline_cache::cached_pipeline(
            "vs_main",
            entry,
            &targets,
            primitive,
            Some(shared.depth_stencil.clone()),
            shared.multisample,
            shared.constants,
            || {
                shared
                    .device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some(label),
                        layout: Some(shared.layout),
                        vertex: wgpu::VertexState {
                            module: shared.shader,
                            entry_point: Some("vs_main"),
                            buffers: shared.vertex_buffers,
                            compilation_options: Default::default(),
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: shared.shader,
                            entry_point: Some(entry),
                            targets: &targets,
                            compilation_options: wgpu::PipelineCompilationOptions {
                                constants: shared.constants,
                                ..Default::default()
                            },
                        }),
                        primitive,
                        depth_stencil: Some(shared.depth_stencil.clone()),
                        multisample: shared.multisample,
                        multiview_mask: None,
                        cache: None,
                    })
            },
        )
    };
    [
        make("mesh glow stamp (two-sided)", false),
        make("mesh glow stamp (culled)", true),
    ]
}
