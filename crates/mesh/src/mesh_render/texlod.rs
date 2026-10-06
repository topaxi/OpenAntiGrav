//! The GE's texture level selection for a PSP `.vex` model.

use super::Anisotropy;
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

/// How far out a PSP `.vex` model keeps its finer texture levels - one
/// multiplier on the distance at which [`PSP_TEXLOD_SLOPE`]'s law steps to
/// the next level, so the recovered rule stays the only rule.
/// `[render_profiles.<title> (<platform>)] texture_detail`.
///
/// [`Self::Original`] is the recovered slope law; [`Self::High`] and
/// [`Self::Maximum`] are this project's own, for a machine that would rather
/// see the crisp level further out - **chosen, not measured**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum TextureDetail {
    /// The slope law as `Gfx_FlushRenderManager` programs it.
    #[default]
    Original,
    /// Every level step at double the depth.
    High,
    /// Never step: level 0 at every depth.
    Maximum,
}

impl TextureDetail {
    /// The spelling used in a settings file, on a menu row and on `--texture-detail`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::High => "high",
            Self::Maximum => "maximum",
        }
    }

    /// Every preset, for the menus and for error messages.
    pub const ALL: [Self; 3] = [Self::Original, Self::High, Self::Maximum];

    /// What the depth of every level step is multiplied by. Infinite for
    /// [`Self::Maximum`], which no finite depth reaches.
    #[must_use]
    pub fn scale(self) -> f32 {
        match self {
            Self::Original => 1.0,
            Self::High => 2.0,
            Self::Maximum => f32::INFINITY,
        }
    }

    /// [`Self::scale`] as `mesh.wesl` applies it: added to the level the slope
    /// law computes, `-log2(scale)`. A doubled distance is one level less at
    /// every depth, and `-64` is far past any chain, so it clamps to level 0.
    /// Zero for [`Self::Original`], which is what a zeroed scene buffer holds,
    /// so a viewer that never writes one draws the recovered law.
    #[must_use]
    pub fn level_shift(self) -> f32 {
        -self.scale().log2().min(64.0)
    }
}

impl std::str::FromStr for TextureDetail {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|detail| detail.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a texture detail preset; try original, high or maximum")
            })
    }
}

impl std::fmt::Display for TextureDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for TextureDetail {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<TextureDetail> for String {
    fn from(detail: TextureDetail) -> Self {
        detail.to_string()
    }
}

/// The pipeline constants `mesh.wesl` reads to select a level by the GE's slope
/// rule: `texlod_slope` and `texlod_bias`, plus `aniso_max` (the sampler's
/// clamp, so the explicit level keeps an anisotropic footprint), for a model
/// that carries the disc's own mip chains ([`Texels::Chain`], which is a PSP `.vex` model), and nothing
/// for every other.
///
/// Decided from the textures alone so no caller has to say which title it is
/// drawing; `crates/render/tests/psp_slope_lod.rs` fails if it stops.
pub(super) fn constants(model: &Model, anisotropy: Anisotropy) -> Vec<(&'static str, f64)> {
    let chained = model
        .textures
        .iter()
        .flatten()
        .any(|texture| matches!(texture.texels, Texels::Chain(_)));
    if chained {
        vec![
            ("texlod_slope", f64::from(PSP_TEXLOD_SLOPE)),
            ("texlod_bias", f64::from(PSP_TEXLOD_BIAS)),
            ("aniso_max", f64::from(anisotropy.clamp())),
        ]
    } else {
        Vec::new()
    }
}
