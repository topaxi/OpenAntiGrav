//! The two pipelines that give a circuit's blended foliage its own depth.
//!
//! The original draws list B (trees, translucent floor patches, glass) far to
//! near by one queue key per mesh and writes no depth, so which tree is over
//! which is a property of the key and the order a strip authors its planes in
//! (`docs/rendering/draw-order.md`, "Trees overlapping trees on Platinum Rush
//! White"). With `ScenerySort::Depth` a foliage draw is submitted twice:
//! [`Foliage::depth`] lays the depth of its solid texels down first, for every
//! foliage draw of the list, and [`Foliage::colour`] then draws the usual
//! alpha-over blend against that depth with `LessEqual` and no write, so a
//! texel is shown only where nothing nearer is solid.

use super::{pipeline_cache, velocity_targets};

/// What [`pipelines`] shares with the pipelines [`super::build`] has already
/// made, borrowed so this does not grow `build`'s own argument list.
pub(super) struct Shared<'a> {
    pub device: &'a wgpu::Device,
    pub shader: &'a wgpu::ShaderModule,
    pub layout: &'a wgpu::PipelineLayout,
    pub vertex_buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    pub constants: &'a [(&'static str, f64)],
    pub format: wgpu::TextureFormat,
    pub multisample: wgpu::MultisampleState,
    /// The alpha-over blend and the colour write mask of the blended pipeline.
    pub blend: wgpu::BlendState,
    pub write_mask: wgpu::ColorWrites,
    /// The second target a blended draw rides with, masked empty.
    pub velocity_target: Option<Option<wgpu::ColorTargetState>>,
    /// The scene's own depth comparison.
    pub depth_compare: wgpu::CompareFunction,
}

/// The foliage pair, each `[0]` two-sided and `[1]` back-face culled like every
/// other blended pipeline.
#[derive(Debug, Clone)]
pub struct Foliage {
    /// Depth only: the texels at or above half alpha, depth written, every
    /// colour target masked.
    pub depth: [wgpu::RenderPipeline; 2],
    /// The alpha-over colour pass: `LessEqual` against [`Self::depth`], none written.
    pub colour: [wgpu::RenderPipeline; 2],
}

impl Foliage {
    /// Whether a draw takes part: a circuit's list-B mesh (it carries the depth
    /// point the original's key is taken from) that blends alpha-over by the
    /// recovered class, not by a factor pair its file authors, and not
    /// additively. Translucent floor patches and glass fades take part, but write
    /// depth only where they are at least half opaque.
    #[must_use]
    pub fn takes_part(draw: &crate::mesh::DrawCall) -> bool {
        draw.depth_point.is_some()
            && draw.blend_state.is_none()
            && matches!(draw.blend, Some(oag_vex::vex::BlendClass::AlphaOver) | None)
    }
}

pub(super) fn pipelines(shared: &Shared<'_>) -> Foliage {
    let depth_state = |write: bool, compare: wgpu::CompareFunction| wgpu::DepthStencilState {
        format: super::DEPTH_FORMAT,
        depth_write_enabled: Some(write),
        depth_compare: Some(compare),
        stencil: Default::default(),
        bias: Default::default(),
    };
    let colour_target = |blend: Option<wgpu::BlendState>, write_mask: wgpu::ColorWrites| {
        velocity_targets(
            wgpu::ColorTargetState {
                format: shared.format,
                blend,
                write_mask,
            },
            shared.velocity_target.clone(),
        )
    };
    let make = |label: &str,
                entry: &'static str,
                targets: &[Option<wgpu::ColorTargetState>],
                depth: wgpu::DepthStencilState,
                cull: bool| {
        let primitive = wgpu::PrimitiveState {
            cull_mode: cull.then_some(wgpu::Face::Back),
            ..Default::default()
        };
        pipeline_cache::cached_pipeline(
            shared.shader,
            "vs_main",
            shared.vertex_buffers,
            entry,
            targets,
            primitive,
            Some(depth.clone()),
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
                            targets,
                            compilation_options: wgpu::PipelineCompilationOptions {
                                constants: shared.constants,
                                ..Default::default()
                            },
                        }),
                        primitive,
                        depth_stencil: Some(depth),
                        multisample: shared.multisample,
                        multiview_mask: None,
                        cache: None,
                    })
            },
        )
    };
    let masked = colour_target(None, wgpu::ColorWrites::empty());
    let blended = colour_target(Some(shared.blend), shared.write_mask);
    let depth = depth_state(true, shared.depth_compare);
    let colour = depth_state(false, wgpu::CompareFunction::LessEqual);
    Foliage {
        depth: [
            make(
                "mesh foliage depth (two-sided)",
                "fs_blend_cutout_depth",
                &masked,
                depth.clone(),
                false,
            ),
            make(
                "mesh foliage depth (culled)",
                "fs_blend_cutout_depth",
                &masked,
                depth,
                true,
            ),
        ],
        colour: [
            make(
                "mesh foliage colour (two-sided)",
                "fs_main_blend",
                &blended,
                colour.clone(),
                false,
            ),
            make(
                "mesh foliage colour (culled)",
                "fs_main_blend",
                &blended,
                colour,
                true,
            ),
        ],
    }
}
