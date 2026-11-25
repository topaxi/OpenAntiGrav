//! [`ReflectionResolution`]: how many texels the screen-space glass and sea target gets.

use serde::{Deserialize, Serialize};

use super::Scale;

/// How many texels the target a race draws ahead of the main view gets, on a title that draws
/// one: Wipeout HD's behind-the-glass target (Vineta K's tunnel glass) and its planar sea
/// reflection (Moa Therma, Ubermall), 2048's and Omega's refraction target.
///
/// The original keeps the target at a fixed size whatever the window's: 640x360 for the glass
/// and 960x540 for the sea on HD (`docs/rendering/water.md`, "The planar reflection, drawn").
/// `Scaled` sizes it from the render scale instead, so a lowered render scale shrinks the
/// target too, at the cost of a coarser picture through the glass and in the sea.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ReflectionResolution {
    /// The original's fixed size.
    #[default]
    Original,
    /// The original's size times the render scale, capped at the original's size.
    Scaled,
}

impl ReflectionResolution {
    /// The value's name in a settings file, on the command line and in the menu.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Scaled => "scaled",
        }
    }

    /// Every value, in menu order.
    pub const ALL: [Self; 2] = [Self::Original, Self::Scaled];

    /// The target's size in texels when the original's is `original` and the player renders at
    /// `scale`.
    ///
    /// `Scaled` multiplies each axis by the scale's factor, rounded, and never goes above
    /// `original`: a render scale over 100 buys no sharper glass than the original's, so the
    /// setting only ever saves cost. Each axis is at least one texel. A function of the scale
    /// alone, not of the window, so a race at one scale builds the same target on every machine.
    #[must_use]
    pub fn size(self, original: (u32, u32), scale: Scale) -> (u32, u32) {
        match self {
            Self::Original => original,
            Self::Scaled => {
                let factor = scale.factor().min(1.0);
                let axis = |texels: u32| ((texels as f32 * factor).round() as u32).max(1);
                (axis(original.0), axis(original.1))
            }
        }
    }
}

impl std::str::FromStr for ReflectionResolution {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|value| value.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a reflection resolution; try original or scaled")
            })
    }
}

impl std::fmt::Display for ReflectionResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for ReflectionResolution {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<ReflectionResolution> for String {
    fn from(value: ReflectionResolution) -> Self {
        value.to_string()
    }
}
