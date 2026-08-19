//! Which colour space a render target holds, told by its format.
//!
//! Split out of `mesh_render.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// Whether a render-target format holds linear light rather than
/// gamma-encoded colour.
///
/// **The format is the statement.** Wipeout HD's race draws into
/// [`crate::post::hd_bloom::SCENE_FORMAT`] because its bloom gate reads
/// pre-exposure luminance above 1.0, and every shader in this crate that can
/// draw into that target switches to linear output when built against it -
/// no per-drawable flag exists to fall out of step with the attachment.
#[must_use]
pub fn is_linear_target(format: wgpu::TextureFormat) -> bool {
    format == crate::post::hd_bloom::SCENE_FORMAT
}

/// The pipeline-constant list that switches a shader's `linear_out` override
/// for `format` - `&[]` on a gamma target, so a pipeline built against one
/// is bit-identical to what it was before linear targets existed.
#[must_use]
pub fn linear_constants(format: wgpu::TextureFormat) -> &'static [(&'static str, f64)] {
    if is_linear_target(format) {
        &[("linear_out", 1.0)]
    } else {
        &[]
    }
}

/// [`linear_constants`] as a whole fragment-stage options block, for the
/// pipelines whose only compilation option it is.
#[must_use]
pub fn fragment_options(format: wgpu::TextureFormat) -> wgpu::PipelineCompilationOptions<'static> {
    wgpu::PipelineCompilationOptions {
        constants: linear_constants(format),
        ..Default::default()
    }
}
