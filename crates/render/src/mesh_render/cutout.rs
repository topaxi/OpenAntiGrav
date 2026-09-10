//! The alpha-test reference a cutout draw is submitted under, and the pipelines
//! that carry it.
//!
//! # Why this is per draw and not per model
//!
//! `Gfx_BuildBatchStateList` (`0x0891f890`) programs a PSP/PS2 batch's alpha
//! test as `Gu_AlphaFunc(GU_GREATER, ref, 0xff)`, and `ref` is one of `0x7f`,
//! `0x10` or `0` chosen from **the batch's own two flag bits** - see
//! [`oag_vex::vex::Batch::alpha_test_reference`] for the branch and its
//! evidence. A circuit is one [`Model`](crate::mesh::Model) holding thousands
//! of batches and it mixes them freely, so the reference cannot live on the
//! model the way Wipeout HD's per-material one does.
//!
//! It is a *pipeline* constant rather than vertex or uniform data because
//! `mesh.wgsl` declares `alpha_test_ref` as an `override`, so this file does
//! what [`super::TransparentPipelines`] already does for `DrawCall::blend`:
//! build one pipeline per distinct value the model actually names, and pick
//! between them per draw. Two, on every disc measured.
//!
//! # What the reference costs, measured
//!
//! It is a real change and not a formality, and it moves in the direction where
//! things *vanish*, so the census is in
//! `crates/vex/tests/alpha_test_reference_ground_truth.rs` and the numbers are
//! these. On PSP Pulse, the 1,500 batches asking for `0x7f` are binary
//! cutouts (leaves, crowds, railings) and the strictest reference discards
//! **zero** of their texels; the 7,823 asking for `0x10` lose 431,576 of
//! 17,005,223 (2.54 %), all of them alpha `1..=16`.
//!
//! **Those are texels this project was drawing fully opaque.** A cutout draw
//! goes through `fs_main_alpha_test`, which returns alpha `1.0` and writes
//! depth, so a texel at alpha 3 that clears the old `1/255` is not painted
//! faintly - it is painted solid *and* occludes what is behind it. Pure's
//! `Z3_whiteblue_cloud_GLOW.tga` is the clearest case: a uniform alpha of 3
//! across 4,096 texels, 13 batches, drawn as solid cloud until now and
//! discarded outright by the reference the file itself asks for.

use crate::mesh::DrawCall;

/// One pipeline per distinct reference `draws` names, in first-seen order.
///
/// `make` is [`super::build`]'s own cutout-pipeline closure, which differs
/// from the default pipeline only in the `alpha_test_ref` override it pushes.
///
/// A linear scan rather than a map, for the reason
/// [`super::Built::authored_pipelines`] does the same: the count is two, `f32`
/// is not `Hash` or `Ord`, and first-seen order keeps a rebuild of the same
/// file byte-identical.
pub fn pipelines(
    draws: &[DrawCall],
    make: impl Fn(&str, f32) -> wgpu::RenderPipeline,
) -> Vec<(f32, wgpu::RenderPipeline)> {
    let mut out: Vec<(f32, wgpu::RenderPipeline)> = Vec::new();
    for reference in draws.iter().filter_map(|draw| draw.alpha_test_ref) {
        if out.iter().any(|(seen, _)| *seen == reference) {
            continue;
        }
        let label = format!("mesh alpha test (ref {:.4})", reference);
        let pipeline = make(&label, reference);
        out.push((reference, pipeline));
    }
    out
}

/// The pipelines one [`Model::alpha_tested_draws`](crate::mesh::Model) is
/// submitted through.
///
/// The sibling of [`super::TransparentPipelines`], and the same shape: a
/// default plus the per-draw table, selected by a field the batch's own file
/// authored.
#[derive(Debug, Clone, Copy)]
pub struct CutoutPipelines<'a> {
    /// [`super::Built::alpha_test_pipeline`] - the model-level reference, which
    /// is Wipeout HD's per-material value where it authors one and
    /// `mesh.wgsl`'s `ALPHA_TEST_THRESHOLD` otherwise.
    pub default: &'a wgpu::RenderPipeline,
    /// [`super::Built::cutout_pipelines`], one per reference a `.vex` batch
    /// asked for.
    pub by_reference: &'a [(f32, wgpu::RenderPipeline)],
}

impl<'a> CutoutPipelines<'a> {
    /// The pipeline one cutout draw is submitted through.
    ///
    /// A draw whose file authors no reference - every PS3 chunk, every
    /// synthetic draw - takes [`Self::default`], so the HD path is untouched by
    /// this table existing. A reference with no pipeline cannot happen for a
    /// model this `Built` was made from, and falls back to the default rather
    /// than panicking, for the caller that pairs a `Built` with another model.
    #[must_use]
    pub fn select(&self, draw: &DrawCall) -> &'a wgpu::RenderPipeline {
        let Some(reference) = draw.alpha_test_ref else {
            return self.default;
        };
        self.by_reference
            .iter()
            .find(|(seen, _)| *seen == reference)
            .map_or(self.default, |(_, pipeline)| pipeline)
    }
}
