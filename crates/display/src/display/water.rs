//! [`Water`]: which law draws the surface of a sea, a lake or a pool.

use serde::{Deserialize, Serialize};

/// Which law draws a classified water surface.
///
/// `[render_profiles.<title> (<platform>)] water`. A value other than
/// [`Self::Off`] and [`Self::Original`] borrows another title's law, so a menu
/// offers it only when that title's disc image is present and never on that
/// title itself (`docs/rendering/water.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Water {
    /// The surface's constant colour or texture, lit: no wave normal, no
    /// scroll, no reflection, no caustic pass.
    Off,
    /// The title's own law. The default.
    #[default]
    Original,
    /// Wipeout HD's sea law.
    Hd,
    /// Wipeout 2048's one-tap law.
    Vita2048,
    /// Wipeout Omega's law.
    Omega,
}

impl Water {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Original => "original",
            Self::Hd => "hd",
            Self::Vita2048 => "2048",
            Self::Omega => "omega",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 5] = [
        Self::Off,
        Self::Original,
        Self::Hd,
        Self::Vita2048,
        Self::Omega,
    ];

    /// Whether the law is another title's, so that title's disc must be present.
    #[must_use]
    pub fn is_borrowed(self) -> bool {
        matches!(self, Self::Hd | Self::Vita2048 | Self::Omega)
    }
}

impl std::str::FromStr for Water {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|water| water.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a water setting; try off, original, hd, 2048 or omega")
            })
    }
}

impl std::fmt::Display for Water {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for Water {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Water> for String {
    fn from(water: Water) -> Self {
        water.to_string()
    }
}
