//! The profile's XP total, the half of [`Store`] `Profile_BankRaceXP` writes
//! (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`).
//!
//! **One total per profile, not per title**: 2048 and Omega's 2048 mode add to
//! and read the same number. Each title reads its own rank table, so the same
//! total is a different rank in each. A file written before the total existed
//! has no `xp` key and reads as `0`.

use oag_tables::xp::Ranks;

use super::{Medal, Store};
use oag_raceplay::xp_tally::{Completion, XpTally};

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

    /// Banks a finished 2048 event: the race's `tally` (its in-race awards and chain bonuses)
    /// plus the race-end awards, which go through the same tally so they chain like any other:
    /// the completion awards when `medal` beats `prior`, the best before the race (Gold is
    /// Elite, any other medal Pass; see [`Completion::on_improvement`]), and `Finished Nth` for
    /// `place`. Read `prior` before the race's medal is recorded. Returns what was added.
    ///
    /// **Chosen, not measured**: `Finished Nth` pays on every finish, and the mode column is by
    /// event kind (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`, "Wired").
    pub fn bank_event_xp(
        &mut self,
        tally: &mut XpTally,
        prior: Option<Medal>,
        medal: Option<Medal>,
        place: Option<u8>,
    ) -> u32 {
        let elite = |m: Medal| m == Medal::Gold;
        tally.finish(
            Completion::on_improvement(prior.map(elite), medal.map(elite)),
            place,
        );
        let award = tally.take();
        self.bank_xp(award);
        award
    }
}
