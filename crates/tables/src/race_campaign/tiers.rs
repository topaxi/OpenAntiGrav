//! The medal tiers of a campaign: [`Medal`] is a position in a [`MedalTiers`] list, best
//! first. The disc campaigns of every title use the standard list ([`MedalTiers::standard`]:
//! gold 3, silver 2, bronze 1); a campaign mod may declare its own
//! (`schemas/v1/campaign.tosd`, `[[medal-tier]]`).

use std::sync::OnceLock;

/// A medal tier: its position in the campaign's [`MedalTiers`], `0` the best.
///
/// [`Ord`] follows the position, so the *smaller* value is the *better* medal (the
/// original's "0 is best"). **Not the in-race HUD's tier ordinal**, which runs `0 = BRONZE ..
/// 3 = RECORD` and is unread; see `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
/// "what is not determined".
///
/// The three associated constants are the standard list's positions. In a campaign that
/// declares its own tiers a position may name any tier: ask [`MedalTiers`], not the constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Medal(u8);

#[allow(
    non_upper_case_globals,
    reason = "the three variants this type used to be"
)]
impl Medal {
    /// The best tier of the standard list.
    pub const Gold: Self = Self(0);
    /// The middle tier of the standard list.
    pub const Silver: Self = Self(1);
    /// The worst tier of the standard list a race can still be awarded.
    pub const Bronze: Self = Self(2);

    /// The tier at `index` in a list, `0` the best.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        u8::try_from(index).ok().map(Self)
    }

    /// The position in the list, `0` the best.
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    /// `Cell_MedalPoints` (`0x088bf530`) on the standard list: gold 3, silver 2, bronze 1.
    /// With [`MedalTiers::standard`] it is what any cell is worth at best, always `3`. A
    /// campaign's own list answers through [`MedalTiers::points`].
    #[must_use]
    pub fn points(self, tiers: &MedalTiers) -> u32 {
        tiers.points(self)
    }
}

/// One tier of a campaign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MedalTier {
    /// What a record, a requirement and a target name it by, lower case: `gold`.
    pub name: String,
    /// What a cell earned at this tier adds to its grid's points.
    pub points: u32,
    /// The swatch colour, `0xAARRGGBB`; the screen replaces the alpha byte with its fade.
    pub argb: u32,
    /// The string id of the tier's name on screen (`IG_HUD_GOLD`); `None` shows [`Self::name`]
    /// upper-cased.
    pub string_id: Option<String>,
}

/// A campaign's medal tiers, best first. Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MedalTiers(Vec<MedalTier>);

/// Why a list of tiers is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TiersError {
    /// No tier at all.
    Empty,
    /// More tiers than a [`Medal`] can number.
    TooMany,
    /// A name used twice.
    Duplicate(String),
    /// A name with nothing in it.
    NoName,
}

impl std::fmt::Display for TiersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "a campaign needs at least one medal tier"),
            Self::TooMany => write!(f, "more than {} medal tiers", u8::MAX),
            Self::Duplicate(name) => write!(f, "medal tier {name:?} is declared twice"),
            Self::NoName => write!(f, "a medal tier has no name"),
        }
    }
}

impl std::error::Error for TiersError {}

impl MedalTiers {
    /// The disc campaigns' list, for every title: gold 3, silver 2, bronze 1. The three colours
    /// are `CellSelection_PopulateGrid`'s (`0x088d5f50..64`) and `GridSelection_PopulateTiles`'s
    /// (`0x088de8f4..908`) immediates; PS2 reads the same twelve bytes from `.data` (`0x2b34c8`,
    /// `0x2b48f8`).
    #[must_use]
    pub fn standard() -> &'static Self {
        static STANDARD: OnceLock<MedalTiers> = OnceLock::new();
        STANDARD.get_or_init(|| {
            let tier = |name: &str, points, argb, id: &str| MedalTier {
                name: name.to_string(),
                points,
                argb,
                string_id: Some(id.to_string()),
            };
            Self(vec![
                tier("gold", 3, 0xfffa_eb38, "IG_HUD_GOLD"),
                tier("silver", 2, 0xffda_e3e4, "IG_HUD_SILVER"),
                tier("bronze", 1, 0xffdf_942f, "IG_HUD_BRONZE"),
            ])
        })
    }

    /// A list of tiers, best first.
    ///
    /// # Errors
    ///
    /// [`TiersError`] when it is empty, too long, or names a tier twice (names compare lower
    /// case).
    pub fn new(tiers: Vec<MedalTier>) -> Result<Self, TiersError> {
        if tiers.is_empty() {
            return Err(TiersError::Empty);
        }
        if tiers.len() > usize::from(u8::MAX) {
            return Err(TiersError::TooMany);
        }
        for (index, tier) in tiers.iter().enumerate() {
            if tier.name.is_empty() {
                return Err(TiersError::NoName);
            }
            if tiers[..index]
                .iter()
                .any(|before| before.name.eq_ignore_ascii_case(&tier.name))
            {
                return Err(TiersError::Duplicate(tier.name.clone()));
            }
        }
        Ok(Self(tiers))
    }

    /// Whether this is the standard list, which is what every disc campaign has.
    #[must_use]
    pub fn is_standard(&self) -> bool {
        self == Self::standard()
    }

    /// How many tiers there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Never: a list always has a tier.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The tiers, best first.
    pub fn iter(&self) -> impl Iterator<Item = (Medal, &MedalTier)> {
        self.0
            .iter()
            .enumerate()
            .filter_map(|(index, tier)| Some((Medal::from_index(index)?, tier)))
    }

    /// The tier of `medal`; `None` for a position past the end.
    #[must_use]
    pub fn get(&self, medal: Medal) -> Option<&MedalTier> {
        self.0.get(medal.index())
    }

    /// The tier named `name`, case-insensitively.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<Medal> {
        let index = self
            .0
            .iter()
            .position(|tier| tier.name.eq_ignore_ascii_case(name.trim()))?;
        Medal::from_index(index)
    }

    /// The best tier's points: what any cell is worth at best.
    #[must_use]
    pub fn best_points(&self) -> u32 {
        self.0.first().map_or(0, |tier| tier.points)
    }

    /// `medal`'s points; `0` for a position past the end.
    #[must_use]
    pub fn points(&self, medal: Medal) -> u32 {
        self.get(medal).map_or(0, |tier| tier.points)
    }

    /// `medal`'s swatch colour, `0xAARRGGBB`; the last tier's for a position past the end.
    #[must_use]
    pub fn argb(&self, medal: Medal) -> u32 {
        self.get(medal)
            .or_else(|| self.0.last())
            .map_or(0xffff_ffff, |tier| tier.argb)
    }

    /// Whether `medal` is `least` or better. **Chosen, not measured:** with `least` gold, every
    /// tier better than gold counts as at least gold.
    #[must_use]
    pub fn at_least(&self, medal: Medal, least: Medal) -> bool {
        medal <= least
    }

    /// The standard tier (gold, silver or bronze) that a sound, a trophy or a result line of
    /// the title's own stands in for `medal`: the tier itself when it is named so; otherwise the
    /// first standard-named tier below it in the list, or the last one above it when none
    /// follows; gold when the list names none. **Chosen, not measured:** the title ships three
    /// of each and has nothing for a fourth.
    #[must_use]
    pub fn class(&self, medal: Medal) -> Medal {
        let standard = |tier: &MedalTier| match tier.name.to_ascii_lowercase().as_str() {
            "gold" => Some(Medal::Gold),
            "silver" => Some(Medal::Silver),
            "bronze" => Some(Medal::Bronze),
            _ => None,
        };
        let at = medal.index().min(self.0.len() - 1);
        self.0[at..]
            .iter()
            .find_map(standard)
            .or_else(|| self.0[..at].iter().rev().find_map(standard))
            .unwrap_or(Medal::Gold)
    }
}

impl Default for MedalTiers {
    fn default() -> Self {
        Self::standard().clone()
    }
}

/// The tier a result earns against `targets`, best tier first, one target per tier: the first
/// one `value` meets. `flipped` is a counting mode (`Zone`, `Elimination`), which wants a
/// *higher* value (`>=`), the others a *lower* one (`<=`: a better position or a faster time).
#[must_use]
pub fn evaluate_targets(targets: &[i64], value: i64, flipped: bool) -> Option<Medal> {
    targets
        .iter()
        .position(|&target| {
            if flipped {
                value >= target
            } else {
                value <= target
            }
        })
        .and_then(Medal::from_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tier(name: &str, points: u32) -> MedalTier {
        MedalTier {
            name: name.to_string(),
            points,
            argb: 0xff00_0000 | points,
            string_id: None,
        }
    }

    fn with_platinum() -> MedalTiers {
        MedalTiers::new(vec![
            tier("platinum", 4),
            tier("gold", 3),
            tier("silver", 2),
            tier("bronze", 1),
        ])
        .expect("a valid list")
    }

    #[test]
    fn the_standard_list_is_the_three_the_discs_have() {
        let standard = MedalTiers::standard();
        assert!(standard.is_standard());
        let named: Vec<_> = standard
            .iter()
            .map(|(medal, tier)| (medal, tier.name.as_str(), tier.points))
            .collect();
        assert_eq!(
            named,
            [
                (Medal::Gold, "gold", 3),
                (Medal::Silver, "silver", 2),
                (Medal::Bronze, "bronze", 1),
            ]
        );
        assert_eq!(standard.argb(Medal::Gold), 0xfffa_eb38);
        assert_eq!(standard.argb(Medal::Silver), 0xffda_e3e4);
        assert_eq!(standard.argb(Medal::Bronze), 0xffdf_942f);
    }

    #[test]
    fn a_list_is_looked_up_by_name_and_points_come_from_it() {
        let tiers = with_platinum();
        let platinum = tiers.find("Platinum").expect("named");
        assert_eq!(platinum, Medal::from_index(0).unwrap());
        assert_eq!(tiers.find("gold").map(Medal::index), Some(1));
        assert_eq!(tiers.points(platinum), 4);
        assert_eq!(tiers.best_points(), 4);
        assert_eq!(tiers.find("diamond"), None);
        assert!(tiers.at_least(platinum, tiers.find("gold").unwrap()));
        assert!(!tiers.at_least(tiers.find("silver").unwrap(), tiers.find("gold").unwrap()));
    }

    #[test]
    fn a_bad_list_is_refused_by_name() {
        assert_eq!(MedalTiers::new(vec![]), Err(TiersError::Empty));
        assert_eq!(
            MedalTiers::new(vec![tier("gold", 2), tier("Gold", 1)]),
            Err(TiersError::Duplicate("Gold".to_string()))
        );
        assert_eq!(MedalTiers::new(vec![tier("", 1)]), Err(TiersError::NoName));
    }

    #[test]
    fn a_tier_with_no_voice_of_its_own_takes_the_standard_one_below_it() {
        let tiers = with_platinum();
        assert_eq!(tiers.class(tiers.find("platinum").unwrap()), Medal::Gold);
        assert_eq!(tiers.class(tiers.find("silver").unwrap()), Medal::Silver);
        let odd = MedalTiers::new(vec![tier("diamond", 5), tier("ruby", 4)]).unwrap();
        assert_eq!(odd.class(Medal::from_index(1).unwrap()), Medal::Gold);
    }

    #[test]
    fn targets_are_met_best_tier_first_in_either_direction() {
        assert_eq!(
            evaluate_targets(&[1, 2, 3, 4], 2, false),
            Medal::from_index(1)
        );
        assert_eq!(evaluate_targets(&[1, 2, 3, 4], 5, false), None);
        assert_eq!(
            evaluate_targets(&[20, 17, 15], 16, true),
            Medal::from_index(2)
        );
    }
}
