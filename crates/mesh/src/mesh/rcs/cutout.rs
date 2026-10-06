//! Wipeout HD's alpha **test**: `oag_rcs::rcsmodel::Transparency::Mode2`.
//!
//! # A different fixed-function feature, not a different equation
//!
//! `Material_ApplyRenderState` (`0x005d8f68` in `ps3-hdfury-eu`) reads the two
//! low bits of a material's state word into two separate RSX registers - bit 0
//! into `NV4097_SET_BLEND_ENABLE`, bit 1 into `NV4097_SET_ALPHA_TEST_ENABLE` -
//! and mode 2 is bit 1 alone. So a mode-2 surface blends *nothing*: it keeps
//! or discards each pixel by comparing the fragment's alpha against the
//! `alpha_func`/`alpha_ref` pair the same call programs into
//! `NV4097_SET_ALPHA_FUNC`/`SET_ALPHA_REF`. See
//! `docs/ghidra/functions/ps3-hdfury-eu/material-state.md`.
//!
//! [`super::surface`] drew these alpha-*blended* until this module existed,
//! because `Material::blend` answered the same `Blend::Factors` for mode 1 and
//! mode 2 and 211 of the disc's 212 mode-2 materials carry the same
//! `0302`/`0303` factor pair a blended one does. The two differ in the depth
//! buffer on every one of them - a blended draw writes no depth - and in
//! colour on the ones whose coverage is not a pure `0`/`255` cutout.
//!
//! # What the disc authors, measured
//!
//! `crates/render/examples/hd_mode2_alpha.rs` sweeps every circuit of all
//! three `PSARC` archives: **103 mode-2 material records under 8 distinct
//! names, every one of them `GL_GREATER` against `0.5`, every one taking its
//! coverage from the first texture's `a` channel.** Five of the eight are
//! effectively pure cutouts - under a quarter of a percent of their texels lie
//! strictly between `0` and `255` - and the test only changes their depth
//! behaviour. Two are not: `cf_alpha4glow` is **67 %** intermediate and
//! `jd_alphalambert_test` **20 %**, so on those the hard cutoff is a visible
//! change of picture and not only of sorting.
//!
//! | material | records | chunks | texels between 0 and 255 | kept by `> 0.5` | mean alpha |
//! | --- | --- | --- | --- | --- | --- |
//! | `cf_alpha4glow` | 15 | 38 | 67.07 % | 36.2 % | 0.3865 |
//! | `jd_alphalambert_test` | 5 | 10 | 19.92 % | 49.0 % | 0.4860 |
//! | `emissive_alpha_heathaze_test` | 4 | 54 | 0.23 % | 72.9 % | 0.7288 |
//! | `fence_alpha` | 11 | 11 | 0.13 % | 24.3 % | 0.2429 |
//! | `nr_crowd_bustle` | 33 | 1,123 | 0.11 % | 48.6 % | 0.4858 |
//! | `jd_alphalambert_alphatest` | 2 | 45 | 0.00 % | 44.7 % | 0.4469 |
//! | `lambert` | 32 | 100 | 0.00 % | 44.8 % | 0.4476 |
//! | `uv_anim_diffuse_alpha_emissive` | 1 | 65 | 0.00 % | 0.8 % | 0.0077 |

use oag_rcs::rcsmodel::{self, Blend};

use super::Report;

/// `GL_GREATER`: keep the fragment whose alpha is **above** the reference.
///
/// The only comparison any consumed mode-2 material on the disc uses - see the
/// census above, and `rcsmodel::Material::alpha_func` for the wider one that
/// found `GL_LESS` too, on materials where the field is never read.
pub const GL_GREATER: u32 = 0x0204;

/// The alpha-test reference one material's cutout compares against, or `None`
/// where it is not a cutout this module will draw.
///
/// **`None` for a comparison other than [`GL_GREATER`]**, which is the one
/// thing the shader hardcodes: `mesh.wesl` discards below the reference and has
/// no comparison-function switch, so a `GL_LESS` cutout drawn through it would
/// be inverted. Nothing on the disc reaches that, and a material that did would
/// draw opaque and be counted in [`Report::cutout_unread`] rather than drawn
/// wrongly.
#[must_use]
pub fn of(material: &rcsmodel::Material) -> Option<f32> {
    (material.blend() == Blend::AlphaTest && material.alpha_func == GL_GREATER)
        .then_some(material.alpha_ref)
}

/// The reference this whole model's cutout materials author, for the pipeline
/// override `mesh.wesl` reads.
///
/// **One per model, because the comparison is a pipeline constant** - see
/// `crate::mesh::Model::alpha_test_ref`. Every mode-2 material on the disc
/// authors `0.5`, so the first one answers for all of them; a model that
/// disagreed with itself would need a second pipeline, and this counts the
/// disagreement in [`Report::cutout_unread`] rather than silently drawing the
/// rest at the first one's reference.
pub fn reference(model: &rcsmodel::Model, report: &mut Report) -> Option<f32> {
    let mut chosen: Option<f32> = None;
    for material in &model.materials {
        if material.blend() != Blend::AlphaTest {
            continue;
        }
        match of(material) {
            Some(reference) => match chosen {
                None => chosen = Some(reference),
                Some(first) if first == reference => {}
                Some(_) => report.cutout_unread += 1,
            },
            None => report.cutout_unread += 1,
        }
    }
    chosen
}
