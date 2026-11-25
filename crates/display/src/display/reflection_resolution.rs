//! [`ReflectionResolution`]: how many texels the screen-space glass and sea target gets.

use serde::{Deserialize, Serialize};

/// How many texels the target a race draws ahead of the main view gets, on a title that draws
/// one: Wipeout HD's behind-the-glass target (Vineta K's tunnel glass) and its planar sea
/// reflection (Moa Therma, Ubermall), 2048's and Omega's refraction target.
///
/// The original keeps the target at a fixed size whatever the window's: 640x360 for the glass
/// and 960x540 for the sea on HD (`docs/rendering/water.md`, "The planar reflection, drawn").
/// `Half` and `Quarter` take a fixed fraction of that size on each axis, whatever the render
/// scale, so the cost saved and the coarser picture through the glass and in the sea are the
/// same on every machine. `Auto` starts at the original's size and lets the frame loop step it
/// to `Half` and `Quarter` while the timed pass is a large share of a frame that misses its
/// budget (`oag_present::drs::reflection`); it is never a size of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ReflectionResolution {
    /// Stepped at runtime by the target's measured cost, between `Original` and `Quarter`.
    #[default]
    Auto,
    /// The original's fixed size.
    Original,
    /// Half the original's size on each axis.
    Half,
    /// A quarter of the original's size on each axis.
    Quarter,
}

impl ReflectionResolution {
    /// The value's name in a settings file, on the command line and in the menu.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Original => "original",
            Self::Half => "half",
            Self::Quarter => "quarter",
        }
    }

    /// Every value, in menu order.
    pub const ALL: [Self; 4] = [Self::Auto, Self::Original, Self::Half, Self::Quarter];

    /// The fixed steps `Auto` moves between, largest first.
    pub const STEPS: [Self; 3] = [Self::Original, Self::Half, Self::Quarter];

    /// Whether the frame loop picks the step.
    #[must_use]
    pub fn is_auto(self) -> bool {
        self == Self::Auto
    }

    /// How many times smaller than the original's each axis is. `Auto` is the original's until
    /// the frame loop steps it.
    #[must_use]
    pub fn divisor(self) -> u32 {
        match self {
            Self::Auto | Self::Original => 1,
            Self::Half => 2,
            Self::Quarter => 4,
        }
    }

    /// The target's size in texels when the original's is `original`.
    ///
    /// Each axis is the original's divided by [`Self::divisor`], rounded down, and at least one
    /// texel. A function of the original's size alone, not of the window or the render scale, so
    /// a race builds the same target on every machine.
    #[must_use]
    pub fn size(self, original: (u32, u32)) -> (u32, u32) {
        let axis = |texels: u32| (texels / self.divisor()).max(1);
        (axis(original.0), axis(original.1))
    }
}

impl std::str::FromStr for ReflectionResolution {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|value| value.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!(
                    "{text:?} is not a reflection resolution; try auto, original, half or quarter"
                )
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
