//! [`ScenePrecision`]: what a race's scene target keeps of each pixel.

use serde::{Deserialize, Serialize};

/// What a race's scene target keeps of each pixel, where the title's scene stores
/// encoded values (Wipeout HD's).
///
/// The original draws HD's race into an 8-bit `A8R8G8B8` target, sRGB on write, and
/// its post chain decodes it on fetch (`docs/rendering/hd-track-tone.md`, "HD's blends
/// combine encoded values"). A title that draws into a gamma target or a float one of
/// its own (Pulse, Pure, Omega, 2048) is not changed by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ScenePrecision {
    /// The original's 8-bit target: each draw stores its encoded byte, each blend is
    /// clamped to `[0, 1]`, and the bloom resolve reads it in one hardware tap.
    #[default]
    Original,
    /// A half-float target holding the same encoded values unrounded, clamped only
    /// where the post chain reads them.
    Float,
}

impl ScenePrecision {
    /// The value's name in a settings file, on the command line and in the menu.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Float => "float",
        }
    }

    /// Every value, in menu order.
    pub const ALL: [Self; 2] = [Self::Original, Self::Float];

    /// Whether the scene is drawn into the original's 8-bit target.
    #[must_use]
    pub fn is_eight_bit(self) -> bool {
        self == Self::Original
    }
}

impl std::str::FromStr for ScenePrecision {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|value| value.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a scene precision; try original or float"))
    }
}

impl std::fmt::Display for ScenePrecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for ScenePrecision {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<ScenePrecision> for String {
    fn from(value: ScenePrecision) -> Self {
        value.to_string()
    }
}
