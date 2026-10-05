//! [`MedalTargets`]/[`DifficultyTargets`]/[`Difficulty`] - the per-difficulty
//! target shape [`super::Cell::difficulty_targets`] carries and
//! [`super::Cell::targets_for_difficulty`]/`skill_for_difficulty`/
//! `evaluate_medal_for_difficulty` key on. Split out of `race_campaign.rs`
//! under the 1,000-line rule (`scripts/check-file-size.py`), the same
//! `mod <name>;` idiom `race_campaign::unlock` already uses for a self-
//! contained cluster - not a new module boundary, `pub use` below keeps
//! every external path (`oag_tables::race_campaign::Difficulty` and
//! siblings) unchanged.

// Every doc comment below cites `Cell`/`Medal` by name to resolve its own
// intra-doc links - the import is what makes those resolve, there is no code
// here that reads either.
#[allow(unused_imports)]
use super::{Cell, Medal};

/// One gold/silver/bronze target triple - [`Cell::gold`]/[`Cell::silver`]/
/// [`Cell::bronze`] at a single difficulty, and each rung of
/// [`DifficultyTargets`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MedalTargets {
    /// The gold-medal target.
    pub gold: i64,
    /// The silver-medal target.
    pub silver: i64,
    /// The bronze-medal target.
    pub bronze: i64,
}

/// [`Cell::difficulty_targets`]: one [`MedalTargets`] triple per difficulty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifficultyTargets {
    /// `<EasyGold>`/`<EasySilver>`/`<EasyBronze>`.
    pub easy: MedalTargets,
    /// `<MediumGold>`/`<MediumSilver>`/`<MediumBronze>`. Also
    /// [`Cell::gold`]/[`Cell::silver`]/[`Cell::bronze`]'s own value - see
    /// those fields' docs.
    pub medium: MedalTargets,
    /// `<HardGold>`/`<HardSilver>`/`<HardBronze>`.
    pub hard: MedalTargets,
}

/// The rung [`Cell::skill_for_difficulty`]/[`Cell::targets_for_difficulty`]/
/// [`Cell::evaluate_medal_for_difficulty`] all key on - Wipeout HD/Fury's
/// own `Easy`/`Medium`/`Hard` ≡ `Novice`/`Skilled`/`Elite` pairing, measured
/// in `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s "`HARD` ≡
/// `ELITE`" section. Replaces a bare `u8` index across this crate and
/// `oag-ui`'s own HD campaign draw code - the same "a bare ordinal is a
/// standing invitation to misread it" reasoning `oag_game::records::Medal`'s
/// own doc already gives, applied here rather than left as a convention
/// three call sites each restate in a doc comment (`0` easy .. `2` hard).
///
/// **A third, separate type from [`oag_game::records::Difficulty`]**, not
/// reused - the same duplication `oag_game::records::Medal` already carries
/// against this module's own [`Medal`], and for the identical reason:
/// `oag_game::records` stays free of this crate so its own store logic
/// keeps testing with no disc, no GPU and no simulation step. A caller
/// crossing the boundary converts explicitly (`crate::main::campaign_stage`'s
/// own `to_campaign_difficulty` and its inverse), the same shape
/// `to_campaign_medal` already gives [`Medal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difficulty {
    /// Novice, on `Elimination`/`NitroBattle` cells. Declared first so
    /// `Ord`'s derived direction already makes `Hard` the hardest
    /// [`Difficulty`] with no inversion trick, unlike [`Medal`].
    Easy,
    /// Skilled. Also what [`Cell::gold`]/[`Cell::silver`]/[`Cell::bronze`]
    /// themselves mean on a cell with no [`Cell::difficulty_targets`] at
    /// all - see those fields' own docs.
    Medium,
    /// Elite.
    Hard,
}

impl Difficulty {
    /// Cycles `Easy -> Medium -> Hard -> Easy` -
    /// `oag_ui_screens::campaign::CellSelection::cycle_difficulty`'s own step.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Easy => Self::Medium,
            Self::Medium => Self::Hard,
            Self::Hard => Self::Easy,
        }
    }
}
