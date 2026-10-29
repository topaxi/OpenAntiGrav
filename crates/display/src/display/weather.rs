//! [`Weather`]: whether a circuit's rain and snow stay as the original draws
//! them or gain this project's own extras.

use serde::{Deserialize, Serialize};

/// What the weather draws on a title whose weather is played (Pulse on the
/// PSP today).
///
/// `[render_profiles.<title> (<platform>)] weather`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Weather {
    /// The original's weather and nothing added: the particle field, the lens
    /// droplets and the mist overlay of `Weather_Update`
    /// (`docs/ghidra/functions/psp-pulse-usa/weather.md`). The default; the
    /// frame is byte-identical to a build without the setting.
    #[default]
    Original,
    /// The original's weather plus splashes where a drop meets the track and
    /// snowflakes that settle on it. **This project's own, chosen, not
    /// measured**: the original's particles never touch geometry.
    Enhanced,
}

impl Weather {
    /// The spelling used in a settings file, on a menu row and on `--weather`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Enhanced => "enhanced",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Original, Self::Enhanced];

    /// Whether drops and flakes meet the track.
    #[must_use]
    pub fn is_enhanced(self) -> bool {
        matches!(self, Self::Enhanced)
    }
}

impl std::str::FromStr for Weather {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|weather| weather.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a weather setting; try original or enhanced"))
    }
}

impl std::fmt::Display for Weather {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for Weather {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Weather> for String {
    fn from(weather: Weather) -> Self {
        weather.to_string()
    }
}
