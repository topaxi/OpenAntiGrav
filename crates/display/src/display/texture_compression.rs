//! [`TextureCompression`]: what form a block-compressed texture takes on a device
//! that cannot sample BC.

use serde::{Deserialize, Serialize};

/// What a disc's BC texture becomes on a device without `TEXTURE_COMPRESSION_BC`
/// (a phone's GPU), where it cannot go up as the disc's own blocks.
///
/// `Original` is the default: the texture is decoded to RGBA8, every texel the
/// disc's. `Astc` re-encodes that same picture as ASTC 4x4 blocks, about a
/// quarter of the memory and measurably faster to sample, at a small loss: a
/// lossy re-encode of this project's own output, not a format any disc authors
/// (`docs/rendering/texture-compression.md`). Offered only on a device that
/// lacks BC and samples ASTC; inert everywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum TextureCompression {
    /// Decoded to RGBA8, as the disc's blocks decode.
    #[default]
    Original,
    /// Re-encoded as ASTC 4x4, cached on disk.
    Astc,
}

impl TextureCompression {
    /// The value's name in a settings file, on the command line and in the menu.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Astc => "astc",
        }
    }

    /// Every value, in menu order.
    pub const ALL: [Self; 2] = [Self::Original, Self::Astc];
}

impl std::str::FromStr for TextureCompression {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|value| value.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a texture compression; try original or astc"))
    }
}

impl std::fmt::Display for TextureCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for TextureCompression {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<TextureCompression> for String {
    fn from(value: TextureCompression) -> Self {
        value.to_string()
    }
}
