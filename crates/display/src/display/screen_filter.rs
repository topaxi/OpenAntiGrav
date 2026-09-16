//! [`FilterStrength`]: how much of a screen filter's simulated display shows.
//!
//! Its own file rather than a block in `display.rs` for the 1,000-line rule
//! in `scripts/check-file-size.py`, the same reason `shadows.rs` is - and
//! written out rather than through that file's `percentage!` macro, which is
//! declared after the `mod` lines and so not in scope here.
//!
//! The filter *itself* is not a type here: it is named by a file on disk, so
//! its values are a list the game supplies at runtime - the same footing
//! `display.front_end_style` and `graphics.renderer` are on - and the
//! settings key is a plain string with `off` as its empty value. See
//! `oag_game::screen`.

use serde::{Deserialize, Serialize};

/// How much of a screen filter's output reaches the surface, as a percentage.
/// 100 is the filter as authored; 0 is the untouched frame, which is what
/// `screen_filter = "off"` also draws without running the pass at all.
///
/// A mix between the two rather than a parameter *inside* the preset,
/// because it then means the same thing for every preset, a player's own
/// included: `mix(frame, filtered, strength)` in the pass's own entry point,
/// after the preset has returned. A preset that wants finer control declares
/// its own tunables in its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct FilterStrength(u32);

impl FilterStrength {
    /// The filter exactly as its file draws it.
    pub const FULL: Self = Self(100);

    /// Bounded at both ends: below 0 and above 100 the mix extrapolates,
    /// which is a picture nobody asked for.
    pub const RANGE: std::ops::RangeInclusive<u32> = 0..=100;

    /// The percentages the menus offer.
    pub const OFFERED: [Self; 5] = [Self(25), Self(50), Self(75), Self(90), Self(100)];

    /// The mix factor this percentage means, `0.0..=1.0`.
    #[must_use]
    pub fn factor(self) -> f32 {
        self.0 as f32 / 100.0
    }

    /// The percentage itself.
    #[must_use]
    pub fn percent(self) -> u32 {
        self.0
    }
}

impl Default for FilterStrength {
    fn default() -> Self {
        Self::FULL
    }
}

impl TryFrom<u32> for FilterStrength {
    type Error = String;

    fn try_from(percent: u32) -> Result<Self, Self::Error> {
        if Self::RANGE.contains(&percent) {
            Ok(Self(percent))
        } else {
            Err(format!(
                "a screen filter strength of {percent} is outside {}-{}",
                Self::RANGE.start(),
                Self::RANGE.end()
            ))
        }
    }
}

impl std::str::FromStr for FilterStrength {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let percent: u32 = text
            .trim()
            .trim_end_matches('%')
            .parse()
            .map_err(|e| format!("{text:?} is not a screen filter strength: {e}"))?;
        Self::try_from(percent)
    }
}

impl From<FilterStrength> for u32 {
    fn from(value: FilterStrength) -> Self {
        value.0
    }
}

impl std::fmt::Display for FilterStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_range_is_closed_at_both_ends() {
        assert_eq!("0".parse::<FilterStrength>(), Ok(FilterStrength(0)));
        assert_eq!("100%".parse::<FilterStrength>(), Ok(FilterStrength(100)));
        assert!("101".parse::<FilterStrength>().is_err());
        assert!("-1".parse::<FilterStrength>().is_err());
    }

    #[test]
    fn every_offered_value_round_trips_through_its_text() {
        for offered in FilterStrength::OFFERED {
            assert_eq!(offered.to_string().parse::<FilterStrength>(), Ok(offered));
        }
        assert_eq!(FilterStrength::default(), FilterStrength::FULL);
        assert_eq!(FilterStrength(50).factor(), 0.5);
    }
}
