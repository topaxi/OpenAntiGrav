//! [`DynamicHud`]: whether the race draws its HUD at all.

use serde::{Deserialize, Serialize};

/// The player's `Dynamic HUD` choice, `[display] dynamic_hud`.
///
/// **Only a title whose own options list it offers it** (Pulse, on the PSP and the PS2:
/// `oag_title::HudArt::dynamic_hud`). The original's list has four entries, `FE_DEFAULT`,
/// `FE_CONCAVE`, `FE_FLAT` and `FE_OFF`; only the two that change what a frame holds are
/// offered here. `FE_CONCAVE` and `FE_FLAT` set a flag on the HUD's two panel groups whose
/// effect on the picture is not decoded, so they would be a choice that does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum DynamicHud {
    /// `FE_DEFAULT`, the disc's default: the HUD is drawn.
    #[default]
    Default,
    /// `FE_OFF`: the HUD is not drawn, and neither is the song marquee that rides on it.
    Off,
}

impl DynamicHud {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Off => "off",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Default, Self::Off];

    /// Whether the HUD is drawn.
    #[must_use]
    pub fn shows_hud(self) -> bool {
        matches!(self, Self::Default)
    }
}

impl std::str::FromStr for DynamicHud {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|choice| choice.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a dynamic HUD choice; try default or off"))
    }
}

impl std::fmt::Display for DynamicHud {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for DynamicHud {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<DynamicHud> for String {
    fn from(choice: DynamicHud) -> Self {
        choice.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_choices_round_trip_through_their_names() {
        for choice in DynamicHud::ALL {
            assert_eq!(choice.name().parse::<DynamicHud>(), Ok(choice));
            let text = String::from(choice);
            assert_eq!(DynamicHud::try_from(text), Ok(choice));
        }
        assert_eq!("OFF".parse::<DynamicHud>(), Ok(DynamicHud::Off));
        assert!("concave".parse::<DynamicHud>().is_err());
    }

    #[test]
    fn only_default_draws_the_hud_and_it_is_what_a_fresh_profile_holds() {
        assert!(DynamicHud::default().shows_hud());
        assert!(!DynamicHud::Off.shows_hud());
    }
}
