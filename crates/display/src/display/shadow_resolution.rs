//! [`ShadowResolution`]: how many texels a shadow map gets.

use serde::{Deserialize, Serialize};

/// How many texels the shadow maps get, as a multiple of the title's own sizes.
///
/// It scales the maps a tier renders into: Wipeout HD's per-craft maps under
/// [`Shadows::Original`](super::Shadows::Original) and the depth map under
/// [`Shadows::Mapped`](super::Shadows::Mapped). A tier that draws geometry
/// (Pulse's projected hulls, a blob quad) has no map and is not changed by it.
/// What each value measures against the original is in
/// `docs/rendering/shadows.md`, "Shadow resolution".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ShadowResolution {
    /// The original's edge sharpness: the title's own map sizes at its own
    /// texels per world unit.
    #[default]
    Original,
    /// Twice the texels a side.
    High,
    /// Four times the texels a side.
    Ultra,
}

impl ShadowResolution {
    /// The value's name in a settings file, on the command line and in the menu.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::High => "high",
            Self::Ultra => "ultra",
        }
    }

    /// Every value, in menu order.
    pub const ALL: [Self; 3] = [Self::Original, Self::High, Self::Ultra];

    /// The multiple of the title's own map side.
    #[must_use]
    pub fn scale(self) -> u32 {
        match self {
            Self::Original => 1,
            Self::High => 2,
            Self::Ultra => 4,
        }
    }
}

impl std::str::FromStr for ShadowResolution {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|value| value.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a shadow resolution; try original, high or ultra")
            })
    }
}

impl std::fmt::Display for ShadowResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for ShadowResolution {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<ShadowResolution> for String {
    fn from(value: ShadowResolution) -> Self {
        value.to_string()
    }
}
