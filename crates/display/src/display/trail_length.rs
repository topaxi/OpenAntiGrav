//! [`TrailLength`]: how long a craft's engine ribbon is.

use serde::{Deserialize, Serialize};

/// How many samples a PSP-preset engine ribbon keeps.
///
/// **Only meaningful where the ribbon is the PSP preset** (Pulse and Pure);
/// HD/Fury, Omega and 2048 draw their own tube and offer no choice.
/// `[render_profiles.<title> (<platform>)] trail_length`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum TrailLength {
    /// The original's ribbon, and the default: ten samples, one a tick
    /// (`docs/ghidra/functions/psp-pulse-usa/exhaust.md`).
    #[default]
    Original,
    /// HD/Fury's length (54 samples, one a frame;
    /// `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`) on the PSP
    /// ribbon's own head width, fade and taper. An enhancement of this
    /// project's, a departure from the original.
    Hd,
}

impl TrailLength {
    /// The spelling used in a settings file, on a menu row and on `--trail-length`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Hd => "hd",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Original, Self::Hd];
}

impl std::str::FromStr for TrailLength {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|length| length.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a trail length; try original or hd"))
    }
}

impl std::fmt::Display for TrailLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for TrailLength {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<TrailLength> for String {
    fn from(length: TrailLength) -> Self {
        length.to_string()
    }
}
