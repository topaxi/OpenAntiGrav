//! The depth prepass's two pipelines, for a model whose opaque list costs
//! more in overdraw than in drawing it twice: Wipeout HD's circuit.
//!
//! [`Prepass::depth`] lays the opaque list's depth down with nothing shaded,
//! every colour target's write mask empty and the fragment stage a constant,
//! and [`Prepass::shade`] then draws the same list with the real
//! fragment shader, comparing `LessEqual` against that depth and writing
//! none, so each sample is shaded by the one surface that owns it.
//!
//! **The caller draws the shaded half in reverse.** Under the prepass every
//! exactly coplanar surface passes `LessEqual`, so the last one shaded wins;
//! drawn in reverse that is the *first* in draw order, which is the surface
//! the plain `Less` pipeline lets win. So the prepass changes what is shaded
//! and not what is seen - provided the two pipelines place a vertex at the
//! same depth to the bit, which they do by sharing `vs_main` with no
//! pipeline constants on the vertex stage.

use super::pipeline_cache;

/// The pair a model built with a prepass carries.
#[derive(Debug, Clone)]
pub struct Prepass {
    /// Depth only: `Less`, depth written, every colour target masked.
    pub depth: wgpu::RenderPipeline,
    /// The opaque pipeline's own shading, `LessEqual` against the prepass's
    /// depth, depth not written.
    pub shade: wgpu::RenderPipeline,
    /// [`Self::shade`] from the `indirect_material` variant, which reads each draw's
    /// material slot from its first instance, so the half can go out as indirect
    /// multi-draws (`super::indirect::colour`). `None` elsewhere.
    pub shade_indirect: Option<wgpu::RenderPipeline>,
}

/// What [`pipelines`] shares with the opaque pipeline `build` has made.
pub(super) struct Shared<'a> {
    pub device: &'a wgpu::Device,
    pub shader: &'a wgpu::ShaderModule,
    /// The `indirect_material` variant, where [`Prepass::shade_indirect`] is wanted.
    pub indirect_shader: Option<&'a wgpu::ShaderModule>,
    pub layout: &'a wgpu::PipelineLayout,
    pub vertex_buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    pub constants: &'a [(&'static str, f64)],
    pub velocity: super::Velocity,
    /// The opaque pipeline's own targets and fragment entry.
    pub targets: &'a [Option<wgpu::ColorTargetState>],
    pub fragment_entry: &'static str,
    pub primitive: wgpu::PrimitiveState,
    pub multisample: wgpu::MultisampleState,
}

pub(super) fn pipelines(shared: &Shared<'_>) -> Prepass {
    let depth_state = |write: bool, compare: wgpu::CompareFunction| wgpu::DepthStencilState {
        format: super::DEPTH_FORMAT,
        depth_write_enabled: Some(write),
        depth_compare: Some(compare),
        stencil: Default::default(),
        bias: Default::default(),
    };
    let masked: Vec<Option<wgpu::ColorTargetState>> = shared
        .targets
        .iter()
        .map(|target| {
            target.clone().map(|t| wgpu::ColorTargetState {
                write_mask: wgpu::ColorWrites::empty(),
                ..t
            })
        })
        .collect();
    let make = |shader: &wgpu::ShaderModule,
                label: &str,
                entry: &'static str,
                targets: &[Option<wgpu::ColorTargetState>],
                depth: wgpu::DepthStencilState| {
        pipeline_cache::cached_pipeline(
            shader,
            "vs_main",
            shared.vertex_buffers,
            entry,
            targets,
            shared.primitive,
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
                            module: shader,
                            entry_point: Some("vs_main"),
                            buffers: shared.vertex_buffers,
                            compilation_options: Default::default(),
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: shader,
                            entry_point: Some(entry),
                            targets,
                            compilation_options: wgpu::PipelineCompilationOptions {
                                constants: shared.constants,
                                ..Default::default()
                            },
                        }),
                        primitive: shared.primitive,
                        depth_stencil: Some(depth),
                        multisample: shared.multisample,
                        multiview_mask: None,
                        cache: None,
                    })
            },
        )
    };
    // Only the two opaque entries load the draw's row (`mesh.wesl`).
    let indirect_shader = shared
        .indirect_shader
        .filter(|_| matches!(shared.fragment_entry, "fs_main" | "fs_main_velocity"));
    Prepass {
        depth: make(
            shared.shader,
            "mesh depth prepass",
            shared
                .velocity
                .entry("fs_depth_only", "fs_depth_only_velocity"),
            &masked,
            depth_state(true, wgpu::CompareFunction::Less),
        ),
        shade: make(
            shared.shader,
            "mesh prepassed shade",
            shared.fragment_entry,
            shared.targets,
            depth_state(false, wgpu::CompareFunction::LessEqual),
        ),
        shade_indirect: indirect_shader.map(|shader| {
            make(
                shader,
                "mesh prepassed shade, per-draw material",
                shared.fragment_entry,
                shared.targets,
                depth_state(false, wgpu::CompareFunction::LessEqual),
            )
        }),
    }
}
