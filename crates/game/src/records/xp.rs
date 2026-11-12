//! The profile's XP total, the half of [`Store`] `Profile_BankRaceXP` writes
//! (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`).
//!
//! **One total per profile, not per title**: 2048 and Omega's 2048 mode add to
//! and read the same number. Each title reads its own rank table, so the same
//! total is a different rank in each. A file written before the total existed
//! has no `xp` key and reads as `0`.

use oag_tables::xp::Ranks;

use super::{Medal, Store};
use oag_raceplay::Campaign2048Progress;

impl Store {
    /// The profile's XP total.
    #[must_use]
    pub fn xp(&self) -> u32 {
        self.xp
    }

    /// Adds `award` to the total (`Profile_BankRaceXP`'s `total += tally`) and returns the new
    /// total. The original's 32-bit counter has no cap; this saturates rather than wraps.
    pub fn bank_xp(&mut self, award: u32) -> u32 {
        self.xp = self.xp.saturating_add(award);
        self.xp
    }

    /// The profile's rank in `ranks` (`Profile_GetXPRank`).
    #[must_use]
    pub fn xp_rank(&self, ranks: &Ranks) -> u32 {
        ranks.rank(self.xp)
    }

    /// Whether a hidden event that needs `rank_required` can be revealed
    /// (`GameModeBase_IsUnlockable`'s rank test).
    #[must_use]
    pub fn rank_reveals(&self, ranks: &Ranks, rank_required: Option<i32>) -> bool {
        ranks.meets(self.xp, rank_required)
    }

    /// Banks a finished 2048 event's race-end awards from `progress`: the completion award for
    /// the tier `medal` stands for (Gold is Elite, Bronze Pass, none pays no completion) and
    /// `Finished Nth` for `place`. Returns what was added.
    ///
    /// **Chosen, not measured**: the completion pays on every finish, not only the first, and
    /// Elite pays instead of Pass rather than as well; both are unread
    /// (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`, "Wired").
    pub fn bank_event_xp(
        &mut self,
        progress: &Campaign2048Progress,
        medal: Option<Medal>,
        place: Option<u8>,
    ) -> u32 {
        let Some(awards) = &progress.xp_awards else {
            return 0;
        };
        let completion = medal.map(|m| m == Medal::Gold);
        let award = awards.race_end(progress.xp_column, completion, place);
        self.bank_xp(award);
        award
    }
}
