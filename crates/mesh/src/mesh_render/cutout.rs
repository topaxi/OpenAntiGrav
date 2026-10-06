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
//! `mesh.wesl` declares `alpha_test_ref` as an `override`, so this file does
//! what [`super::TransparentPipelines`] already does for `DrawCall::blend`:
//! build one pipeline per distinct value the model actually names, and pick
//! between them per draw. Two on a Pulse circuit, and up to three where a
//! model names the `0` as well - Pure's `Speedup Pad` does.
//!
//! # What the reference costs, measured
//!
//! It is a real change and not a formality, and it moves in the direction where
//! things *vanish*, so it is measured twice - once in texels and once in
//! pixels, because the two answer different questions and one of them is
//! misleading on its own.
//!
//! **In texels** (`crates/vex/tests/alpha_test_reference_ground_truth.rs`), on
//! PSP Pulse: the 1,500 batches asking for `0x7f` are binary cutouts - leaves,
//! crowds, railings - and the strictest reference discards **zero** of their
//! texels; the 7,823 asking for `0x10` lose 431,576 of 17,005,223 (2.54 %),
//! all alpha `1..=16`.
//!
//! **In pixels** (`crates/render/examples/threshold_probe.rs`, four yaws at
//! 1024x1024, each bucket isolated), the picture is the other way round:
//!
//! | | `01_Track` | `16_Track` |
//! | --- | ---: | ---: |
//! | pixels differing, both buckets | 1,298 | 6,170 |
//! | pixels differing, `0x7f` alone | 1,298 | 6,170 |
//!
//! **Every visible pixel of the change comes from the bucket that discards no
//! texels at all**, and that is not a contradiction: the shader samples the
//! texture *filtered and mipped*, so a `{0, 255}` edge arrives at the alpha
//! test as a ramp, and `0x7f` cuts it at half coverage where `1/255` cut it at
//! any. A leaf's silhouette tightens by about a pixel. The original's own
//! sampler filters the same way, which is why the strict reference is
//! authored on exactly this kind of texture.
//!
//! The `0x10` bucket's 431,576 texels are alpha `1..=16` on surfaces small
//! enough at whole-circuit framing to move no pixel there - but they are not
//! harmless, because a cutout draw goes through `fs_main_alpha_test`, which
//! returns alpha `1.0` and **writes depth**. A texel at alpha 3 that cleared
//! `1/255` was not painted faintly; it was painted solid and occluded what was
//! behind it. Pure's `Z3_whiteblue_cloud_GLOW.tga` is the case with no
//! ambiguity: uniform alpha 3 across 4,096 texels on 13 batches, drawn as
//! solid cloud until now.

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

/// The reference `Gfx_BuildBatchStateList` pairs with a `GEQUAL` depth test,
/// `0x10`: a batch whose `header_flags & 0x20` is set gets both from the
/// same branch - see [`depth_compare`].
pub const DECAL_REFERENCE: f32 = 16.0 / 255.0;

/// The depth test one cutout pipeline draws with.
///
/// **A `0x10` cutout passes on equal depth.** `Gfx_BuildBatchStateList`
/// (`0x0891f890`) sets `Gu_DepthFunc(7)` - `GEQUAL` on the PSP's reversed
/// depth range, `LessEqual` here - and `Gu_AlphaFunc(GREATER, 0x10)` in one
/// branch, taken for `header_flags & 0x20`; every other cutout gets
/// `Gu_DepthFunc(6)`, `GREATER`, which is the scene's `Less`. These are the
/// circuits' `_GLOW` overlays, authored coplanar with the wall they light: 174
/// of `16_Track`'s 270 cutout batches. Under `Less` every one of their
/// fragments ties with the wall and is discarded, so neither their colour nor
/// their glow-mask stamp ever reached the frame. Read on the PSP executable;
/// Pure and the PS2 port author the same pattern and are not read. See
/// `docs/rendering/glow-mask.md`.
#[must_use]
pub fn depth_compare(
    reference: Option<f32>,
    scene: wgpu::CompareFunction,
) -> wgpu::CompareFunction {
    if reference == Some(DECAL_REFERENCE) && scene == wgpu::CompareFunction::Less {
        wgpu::CompareFunction::LessEqual
    } else {
        scene
    }
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
    /// `mesh.wesl`'s `ALPHA_TEST_THRESHOLD` otherwise.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_decal_reference_passes_on_equal_depth() {
        use wgpu::CompareFunction::{Always, Less, LessEqual};
        assert_eq!(depth_compare(Some(DECAL_REFERENCE), Less), LessEqual);
        assert_eq!(depth_compare(Some(f32::from(0x7f_u8) / 255.0), Less), Less);
        assert_eq!(depth_compare(Some(0.0), Less), Less);
        assert_eq!(depth_compare(None, Less), Less);
        assert_eq!(depth_compare(Some(DECAL_REFERENCE), Always), Always);
    }
}
