//! The two pipeline groups a pass draws after the opaque one.

use super::{Built, Cursor, CutoutPipelines, TransparentPipelines};
use crate::mesh::{DrawCall, Model};

/// Draws [`Model::alpha_tested_draws`] and then [`Model::transparent_draws`]
/// into `pass`, in that order, each draw through the pipeline its own batch
/// picks ([`CutoutPipelines::select`], [`TransparentPipelines::select`]).
///
/// `set_pipeline` is issued only when the pick changes, so a model with one
/// reference (every PS3 model) pays for a single call. Transparent draws are
/// order-dependent, so they are never grouped by class first. The caller has
/// already drawn [`Model::draws`] and bound groups 0, 2 and 3 and the index
/// buffer, and holds the `cursor` over the vertex buffers; `bind` is the group 1 texture bind for one draw.
pub fn draw_cutout_then_transparent<'a>(
    pass: &mut wgpu::RenderPass<'_>,
    cursor: &mut Cursor<'_>,
    model: &Model,
    cutouts: CutoutPipelines<'a>,
    transparent: TransparentPipelines<'a>,
    bind: impl Fn(&DrawCall) -> &'a wgpu::BindGroup,
) {
    let mut current: Option<&wgpu::RenderPipeline> = None;
    for draw in &model.alpha_tested_draws {
        let pipeline = cutouts.select(draw);
        if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
            pass.set_pipeline(pipeline);
            current = Some(pipeline);
        }
        pass.set_bind_group(1, bind(draw), &[]);
        cursor.draw(pass, &draw.range);
    }
    let mut current: Option<&wgpu::RenderPipeline> = None;
    for draw in &model.transparent_draws {
        let pipeline = transparent.select(draw);
        if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
            pass.set_pipeline(pipeline);
            current = Some(pipeline);
        }
        pass.set_bind_group(1, bind(draw), &[]);
        cursor.draw(pass, &draw.range);
    }
}

impl Built {
    /// The cutout pipelines of this build, as [`draw_cutout_then_transparent`] takes them.
    #[must_use]
    pub fn cutout_set(&self) -> CutoutPipelines<'_> {
        CutoutPipelines {
            default: &self.alpha_test_pipeline,
            by_reference: &self.cutout_pipelines,
        }
    }

    /// The transparent pipelines of this build, as [`draw_cutout_then_transparent`] takes them.
    #[must_use]
    pub fn transparent_set(&self) -> TransparentPipelines<'_> {
        TransparentPipelines {
            alpha_over: &self.blend_pipeline,
            additive: &self.additive_pipeline,
            unblended: &self.unblended_pipeline,
            authored: &self.authored_pipelines,
        }
    }
}
