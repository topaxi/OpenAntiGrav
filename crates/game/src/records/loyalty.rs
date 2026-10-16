//! The per-team loyalty half of [`Store`]: `Loyalty_AccumulateTotal`'s running
//! total and the reads `Definition_IsUnlocked` compares against.
//!
//! **One total per team, shared by every title that has the team** (maintainer,
//! 2026-10-16): a row is keyed by the team's shared identity
//! ([`oag_title::team`]), so Feisar's loyalty counts in Pulse, Pure, HD, 2048 and
//! Omega alike. How a race *earns* it stays each title's own law; each title's
//! unlock thresholds stay its own.

use oag_title::Title;

use super::{LoyaltyRecord, Store};

/// `Loyalty_AccumulateTotal`'s ceiling on a banked total.
const CAP: u32 = 100_000;

impl Store {
    /// The team's running total as `title` names it - `0` for a team never
    /// raced, matching the original's own fresh-profile reading (`EndRace
    /// Rewards`' `Total loyalty: 0`). Shared with every title that has the team.
    #[must_use]
    pub fn loyalty_total(&self, title: &Title, team: &str) -> u32 {
        self.identity_total(&title.team_identity(team))
    }

    fn identity_total(&self, identity: &str) -> u32 {
        self.loyalty
            .iter()
            .find(|row| row.matches(identity))
            .map_or(0, |row| row.total)
    }

    /// The largest running total any team **of `title`'s roster** holds - what an
    /// `<Unlock Team="any" loyalty="N">` row compares: it passes when **some
    /// one team** reached `N` (`Unlock_LoyaltyMet` loops the teams and stops at
    /// the first that does), not when the teams' totals add up to it. Scoped to the
    /// title's own teams: a team only Omega has (`Tigron`) does not open Pulse's rows.
    #[must_use]
    pub fn loyalty_best(&self, title: &Title) -> u32 {
        title
            .teams
            .iter()
            .map(|team| self.identity_total(&title.team_identity(team.id)))
            .max()
            .unwrap_or(0)
    }

    /// Adds `award` to the team's shared total, creating the row if this is the
    /// first race ever recorded for it, and returns the new total -
    /// `Loyalty_AccumulateTotal`'s own `record+8 += award`, capped the same
    /// place the original caps it: `100000`.
    pub fn record_loyalty(&mut self, title: &Title, team: &str, award: u32) -> u32 {
        let identity = title.team_identity(team);
        self.add_loyalty(&identity, award)
    }

    pub(super) fn add_loyalty(&mut self, identity: &str, award: u32) -> u32 {
        let row = match self.loyalty.iter().position(|row| row.matches(identity)) {
            Some(at) => &mut self.loyalty[at],
            None => {
                self.loyalty.push(LoyaltyRecord {
                    title: String::new(),
                    team: identity.to_owned(),
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
        self.loyalty.sort_by(|a, b| a.team.cmp(&b.team));
        total
    }
}

/// Folds the rows an earlier build kept per `(title, team)` into the shared per-team rows.
///
/// **Sum, capped at the ceiling after the fold.** The shared total is what the team would hold
/// had every title banked into one store from the start, and that is the sum of what each
/// title banked; taking the largest would silently drop a player's progress in every other
/// title, and "a player loses nothing" is the point of migrating. The cost is that a player
/// who earned 5000 in Pulse and 3000 in HD starts at 8000, which is what sharing them means.
/// A row whose title this build does not know folds under its case-folded team id and is
/// named in `notes`. Idempotent: a folded row has no title, [`super::parse`] only folds rows
/// that have one, and `save` writes only folded rows.
pub(super) fn fold_legacy_loyalty(
    rows: Vec<LoyaltyRecord>,
    notes: &mut Vec<String>,
) -> Vec<LoyaltyRecord> {
    let mut store = Store::default();
    let (legacy, current): (Vec<_>, Vec<_>) =
        rows.into_iter().partition(|row| !row.title.is_empty());
    for row in current {
        store.add_loyalty(&row.team.trim().to_ascii_lowercase(), row.total);
    }
    for row in legacy {
        let known = crate::settings::KNOWN_TITLES
            .iter()
            .find(|title| title.name.eq_ignore_ascii_case(row.title.trim()));
        let identity = match known {
            Some(title) => title.team_identity(&row.team),
            None => {
                notes.push(format!(
                    "loyalty row for unknown title {:?} folded under its team id {:?}",
                    row.title, row.team
                ));
                row.team.trim().to_ascii_lowercase()
            }
        };
        store.add_loyalty(&identity, row.total);
    }
    store.loyalty
}
