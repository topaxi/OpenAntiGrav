//! [`Medal`]/[`Difficulty`] - split out of `records.rs` under the
//! 1,000-line rule (`scripts/check-file-size.py`), a self-contained cluster
//! (two small enums, no dependency on anything else in this module bar each
//! other's doc links) with no change to either type's own path
//! (`oag_game::records::Medal`/`Difficulty` still resolve, via the `pub use`
//! in `records.rs`).

use serde::{Deserialize, Serialize};

// Every doc comment below cites `CampaignRecord`/`Store`/`laps_completed`/
// `Observation` by name to resolve its own intra-doc links - the import is
// what makes those resolve, there is no code here that reads any of them.
#[allow(unused_imports)]
use super::{CampaignRecord, Observation, Store, laps_completed};

/// A campaign medal tier, [`oag_tables::race_campaign::Cell::evaluate_medal`]'s
/// own three-value law restated here rather than imported - see that
/// function's doc for `Cell_EvaluateMedal` (`0x088bf620`) and
/// [`oag_tables::race_campaign::Medal::points`] for `Cell_MedalPoints`
/// (`0x088bf530`), gold 3 / silver 2 / bronze 1.
///
/// **Duplicated, not imported, on purpose.** [`Observation`]'s own doc
/// already keeps this module free of `oag-race`/`oag-gameplay`/`crate::race`
/// so [`Store::record`] and [`laps_completed`] stay testable with no disc, no
/// GPU and no simulation step; pulling in `oag-formats` here for one enum
/// would trade that guarantee for a single shared type. [`laps_completed`]
/// already makes the identical trade for `oag_raceplay::scoreboard::build`'s
/// arithmetic - see its own doc.
///
/// Serialized as a lowercase word (`medal = "gold"`), never the bare
/// ordinal: the in-race HUD carries a *different* medal-tier ordinal running
/// the opposite direction (`0 = BRONZE`, per
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "what is not
/// determined" section), so a bare integer in this file would be a standing
/// invitation to misread which convention it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Medal {
    /// The best tier. Declared first so `Ord`'s derived order makes `Gold`
    /// the smallest value - [`Medal::better`] relies on this to pick the
    /// better of two medals with a plain `min`.
    Gold,
    /// The middle tier.
    Silver,
    /// The worst tier a race can still be awarded.
    Bronze,
}

impl Medal {
    /// `Cell_MedalPoints`'s own table: gold 3, silver 2, bronze 1.
    #[must_use]
    pub fn points(self) -> u32 {
        match self {
            Self::Gold => 3,
            Self::Silver => 2,
            Self::Bronze => 1,
        }
    }

    /// The better of two medals. Named rather than inlined as `self.min(other)`
    /// at the call site, so a reader of [`Store::record`] does not have to
    /// work out for themselves why `min` means "best" here - see this type's
    /// own doc for the `Ord` direction that makes it true.
    #[must_use]
    pub(super) fn better(self, other: Self) -> Self {
        self.min(other)
    }
}

/// **Wipeout HD/Fury only.** The rung a [`CampaignRecord::best_medal`] was
/// earned at - `oag_tables::race_campaign::Cell::targets_for_difficulty`'s
/// own three rungs, restated here for the same reason [`Medal`] is: this
/// module stays free of `oag-tables`, and a caller converts at the boundary
/// (`crate::main::campaign_stage::to_campaign_difficulty` and its own
/// inverse), the same shape `to_campaign_medal` already gives [`Medal`].
///
/// **Serialized as a lowercase word for the identical reason [`Medal`]'s
/// own doc gives, not merely by analogy**: `Cell::targets_for_difficulty`'s
/// own index convention (`0` easy .. `2` hard) is a plain `u8` everywhere
/// else in this codebase (`CellSelection::difficulty`,
/// `Cell::skill_for_difficulty`, `oag_ui_screens::campaign::hd::hd_medal_frame`) -
/// deliberately left that way there, since changing an established,
/// widely-used index convention to chase type safety in code that already
/// bounds-checks it (`difficulty.min(2)`) would be exactly the abstraction
/// this project's own house rules say not to add. A saved file is a
/// different case: nothing bounds-checks a hand-edited `records.toml`
/// against a comment, and this project already has one HUD ordinal that
/// runs a different direction from the one this rung's own name suggests
/// (see [`Medal`]'s own doc) - a bare `0`/`1`/`2` here would be the same
/// standing invitation to misread it, for a file meant to outlive any one
/// session's memory of which convention was live when it was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    /// `0` - Novice on `Elimination`/`NitroBattle` cells.
    Easy,
    /// `1` - Skilled. Also what [`oag_tables::race_campaign::Cell::gold`]/
    /// `silver`/`bronze` themselves mean on a cell with no per-difficulty
    /// targets at all - see that field's own doc.
    Medium,
    /// `2` - Elite. What
    /// `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s
    /// `SaveData_MigrateCellMedalsToHardElite` credits a pre-existing,
    /// difficulty-less medal at.
    Hard,
}

impl Difficulty {
    /// `Cell::targets_for_difficulty`'s own index, `0` easy .. `2` hard -
    /// the inverse of [`Self::from_index`]. Declared in ascending
    /// easy-to-hard order, so `Ord`'s derived direction already makes
    /// `Hard` the greatest [`Difficulty`] with no inversion trick - unlike
    /// [`Medal::better`], which needs one.
    #[must_use]
    pub fn index(self) -> u8 {
        match self {
            Self::Easy => 0,
            Self::Medium => 1,
            Self::Hard => 2,
        }
    }

    /// [`Self::index`]'s own inverse, clamping `2` and up to [`Self::Hard`]
    /// the same way `Cell::targets_for_difficulty` itself clamps any index
    /// past its own three rungs.
    #[must_use]
    pub fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Easy,
            1 => Self::Medium,
            _ => Self::Hard,
        }
    }
}
