//! How hard a texture sampler filters at a grazing angle.
//!
//! Split out of `mesh_render.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// Anisotropic filtering level: the one texture-filtering knob modern
/// renderers expose to a user. Mip generation itself always runs (see
/// [`mip_chain`]) and is not a setting - every renderer just does it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Anisotropy {
    Off,
    X2,
    X4,
    X8,
    #[default]
    X16,
}

impl Anisotropy {
    /// The `wgpu::SamplerDescriptor::anisotropy_clamp` value this level maps to.
    pub(super) const fn clamp(self) -> u16 {
        match self {
            Self::Off => 1,
            Self::X2 => 2,
            Self::X4 => 4,
            Self::X8 => 8,
            Self::X16 => 16,
        }
    }
}

impl std::str::FromStr for Anisotropy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "1" | "1x" => Ok(Self::Off),
            "2" | "2x" => Ok(Self::X2),
            "4" | "4x" => Ok(Self::X4),
            "8" | "8x" => Ok(Self::X8),
            "16" | "16x" => Ok(Self::X16),
            other => Err(format!(
                "{other:?} is not an anisotropy level; try off, 2x, 4x, 8x or 16x"
            )),
        }
    }
}

impl std::fmt::Display for Anisotropy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Off => "off",
            Self::X2 => "2x",
            Self::X4 => "4x",
            Self::X8 => "8x",
            Self::X16 => "16x",
        })
    }
}
