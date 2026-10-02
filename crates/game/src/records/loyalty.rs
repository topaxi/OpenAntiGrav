//! The per-team loyalty half of [`Store`]: `Loyalty_AccumulateTotal`'s running
//! total and the reads `Definition_IsUnlocked` compares against. Split out of
//! `records.rs` under the 1,000-line rule; a move.

use super::{LoyaltyRecord, Store};

impl Store {
    /// The team `(title, team)` names' own running total - `0` for a team
    /// never raced, matching the original's own fresh-profile reading
    /// (`EndRace Rewards`' `Total loyalty: 0`).
    #[must_use]
    pub fn loyalty_total(&self, title: &str, team: &str) -> u32 {
        let title = title.trim().to_ascii_lowercase();
        let team = team.trim().to_ascii_lowercase();
        self.loyalty
            .iter()
            .find(|row| row.matches(&title, &team))
            .map_or(0, |row| row.total)
    }

    /// The largest running total any team of `title` holds - what an
    /// `<Unlock Team="any" loyalty="N">` row compares: it passes when **some
    /// one team** reached `N` (`Unlock_LoyaltyMet` loops the teams and stops at
    /// the first that does), not when the teams' totals add up to it.
    #[must_use]
    pub fn loyalty_best(&self, title: &str) -> u32 {
        let title = title.trim().to_ascii_lowercase();
        self.loyalty
            .iter()
            .filter(|row| row.title == title)
            .map(|row| row.total)
            .max()
            .unwrap_or(0)
    }

    /// Adds `award` to `(title, team)`'s own running total, creating the row
    /// if this is the first race ever recorded for it, and returns the new
    /// total - `Loyalty_AccumulateTotal`'s own `record+8 += award`, capped
    /// the same place the original caps it: `100000`.
    pub fn record_loyalty(&mut self, title: &str, team: &str, award: u32) -> u32 {
        const CAP: u32 = 100_000;
        let title = title.trim().to_ascii_lowercase();
        let team = team.trim().to_ascii_lowercase();
        let row = match self
            .loyalty
            .iter_mut()
            .find(|row| row.matches(&title, &team))
        {
            Some(row) => row,
            None => {
                self.loyalty.push(LoyaltyRecord {
                    title,
                    team,
                    total: 0,
                });
                self.loyalty
                    .last_mut()
                    .expect("just pushed onto this exact vec")
            }
        };
        row.total = row.total.saturating_add(award).min(CAP);
        let total = row.total;

        // Stable, for the same reason `Self::record`/`Self::record_campaign`
        // sort their own tables.
        self.loyalty
            .sort_by(|a, b| (&a.title, &a.team).cmp(&(&b.title, &b.team)));
        total
    }
}
