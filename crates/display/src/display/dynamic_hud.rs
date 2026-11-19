//! [`DynamicHud`]: whether the race draws its HUD, and whether it is bent.

use serde::{Deserialize, Serialize};

/// The player's `Dynamic HUD` choice, `[display] dynamic_hud`.
///
/// Pulse authors the list (PSP and PS2: `oag_title::HudArt::dynamic_hud`): `FE_DEFAULT`,
/// `FE_CONCAVE`, `FE_FLAT` and `FE_OFF`. `Hud_Update` turns `FE_CONCAVE` on the HUD's
/// vertex warp, `FE_FLAT` off, and `FE_DEFAULT` on in the cockpit view only
/// (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`). Every other title offers
/// `default` and `concave` alone, `concave` being Pulse's warp on a HUD that never had it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum DynamicHud {
    /// `FE_DEFAULT`, the disc's default: the HUD is drawn, bent in the cockpit view where the
    /// title's own default says so.
    #[default]
    Default,
    /// `FE_CONCAVE`: the HUD is drawn bent in every camera view.
    Concave,
    /// `FE_FLAT`: the HUD is drawn flat in every camera view.
    Flat,
    /// `FE_OFF`: the HUD is not drawn, and neither is the song marquee that rides on it.
    Off,
}

impl DynamicHud {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Concave => "concave",
            Self::Flat => "flat",
            Self::Off => "off",
        }
    }

    /// Every choice, for the menus and for error messages, in the disc's list order.
    pub const ALL: [Self; 4] = [Self::Default, Self::Concave, Self::Flat, Self::Off];

    /// Whether the HUD is drawn.
    #[must_use]
    pub fn shows_hud(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// Whether the HUD is drawn bent: `default_bends` is what the title's default does in the
    /// camera view in use (Pulse: bent in the cockpit view, flat outside it).
    #[must_use]
    pub fn bends(self, default_bends: bool) -> bool {
        match self {
            Self::Default => default_bends,
            Self::Concave => true,
            Self::Flat | Self::Off => false,
        }
    }
}

impl std::str::FromStr for DynamicHud {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|choice| choice.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a dynamic HUD choice; try default, concave, flat or off")
            })
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
    fn the_four_choices_round_trip_through_their_names() {
        for choice in DynamicHud::ALL {
            assert_eq!(choice.name().parse::<DynamicHud>(), Ok(choice));
            let text = String::from(choice);
            assert_eq!(DynamicHud::try_from(text), Ok(choice));
        }
        assert_eq!("OFF".parse::<DynamicHud>(), Ok(DynamicHud::Off));
        assert_eq!("Concave".parse::<DynamicHud>(), Ok(DynamicHud::Concave));
        assert!("convex".parse::<DynamicHud>().is_err());
    }

    #[test]
    fn only_off_hides_the_hud_and_default_is_what_a_fresh_profile_holds() {
        assert!(DynamicHud::default().shows_hud());
        assert!(DynamicHud::Concave.shows_hud());
        assert!(DynamicHud::Flat.shows_hud());
        assert!(!DynamicHud::Off.shows_hud());
    }

    /// `Hud_Update`: `FE_CONCAVE` sets the warp bit, `FE_FLAT` clears it, `FE_DEFAULT` copies the
    /// player craft's internal-view flag.
    #[test]
    fn concave_always_bends_flat_never_and_default_follows_the_title() {
        for cockpit in [false, true] {
            assert!(DynamicHud::Concave.bends(cockpit));
            assert!(!DynamicHud::Flat.bends(cockpit));
            assert!(!DynamicHud::Off.bends(cockpit));
            assert_eq!(DynamicHud::Default.bends(cockpit), cockpit);
        }
    }
}
