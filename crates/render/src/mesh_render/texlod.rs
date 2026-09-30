//! The GE's texture level selection for a PSP `.vex` model.

use crate::mesh::{Model, Texels};

/// The GE's texture level slope, as Pulse programs it: `1/256`.
///
/// `Gfx_FlushRenderManager` calls `Gu_TexLevelMode(2, 1.0)` - slope mode, bias
/// `1.0` - and then `Gu_TexLodSlope(0x3b800000)` once per frame; nothing else in
/// the binary emits `TEXLODSLOPE`. In slope mode the level is
/// `log2(|z| * slope) + bias`, a function of **view depth** alone: level 0 up to
/// `z = 128`, level 1 at `z = 256`, level 2 at `z = 512`. Recovered from the
/// binary (confidence 85 on the two values, see `mesh-draw.md`), **not** measured
/// off the GE: `|z|` here is the vertex's view-space depth (`clip.w`), an
/// unverified reading of the GE's own `z`, and the game's per-texture
/// `Texture_BuildBindList` emits its own mode and bias, unread.
pub const PSP_TEXLOD_SLOPE: f32 = 1.0 / 256.0;

/// The bias of [`PSP_TEXLOD_SLOPE`]'s call, in levels.
pub const PSP_TEXLOD_BIAS: f32 = 1.0;

/// The pipeline constants `mesh.wgsl` reads to select a level by the GE's slope
/// rule: `texlod_slope` and `texlod_bias`, for a model that carries the disc's
/// own mip chains ([`Texels::Chain`], which is a PSP `.vex` model), and nothing
/// for every other.
///
/// Decided from the textures alone so no caller has to say which title it is
/// drawing; `crates/render/tests/psp_slope_lod.rs` fails if it stops.
pub(super) fn constants(model: &Model) -> Vec<(&'static str, f64)> {
    let chained = model
        .textures
        .iter()
        .flatten()
        .any(|texture| matches!(texture.texels, Texels::Chain(_)));
    if chained {
        vec![
            ("texlod_slope", f64::from(PSP_TEXLOD_SLOPE)),
            ("texlod_bias", f64::from(PSP_TEXLOD_BIAS)),
        ]
    } else {
        Vec::new()
    }
}
