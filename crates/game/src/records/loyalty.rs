//! The per-team loyalty half of [`Store`]: `Loyalty_AccumulateTotal`'s running
//! total and the reads `Definition_IsUnlocked` compares against.
//!
//! **One total per team, shared by every title that has the team**: a row is keyed by the team's shared identity
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

/// Folds read `[[loyalty]]` rows onto the table's own rule: **one row per team, keyed by
/// the trimmed, lower-cased spelling of its key**.
///
/// A row is found by key, so a file that holds two rows for the same team, or a key
/// spelled differently, is malformed: the first row would win every read, and the rest would
/// be read by nothing yet rewritten on every save forever. Summing what each row holds is the
/// reading the table means - a row is what has been banked for that team, so two rows for one
/// team are two bankings that were never brought together - capped at [`CAP`] after the fold,
/// the same way a single row is capped. Every fold is named in `notes`; a file written by this
/// build needs none, because [`Store::add_loyalty`] can only ever keep one row per key.
///
/// **Spelling only, never identity.** `parse` knows no title, so the fold can only trim and
/// lower-case: a key whose roster identity is spelled differently from the key itself stays under
/// its own spelling. A row keyed `Mantis` folds to `mantis`, not to Pulse's identity for that team
/// (`mirage`), so [`Store::loyalty_total`] still reads `0` for it - as it did before the fold.
/// Pinned by `crates/game/src/records/tests.rs::a_fold_normalises_spelling_but_cannot_apply_a_roster_identity`.
#[must_use]
pub(super) fn fold_keys(rows: Vec<LoyaltyRecord>, notes: &mut Vec<String>) -> Vec<LoyaltyRecord> {
    let mut store = Store::default();
    for row in rows {
        let key = row.team.trim().to_ascii_lowercase();
        if store.loyalty.iter().any(|kept| kept.matches(&key)) {
            notes.push(format!(
                "loyalty row for team {:?} folded into the one kept for {:?}: +{}",
                row.team, key, row.total
            ));
        } else if key != row.team {
            notes.push(format!(
                "loyalty row for team {:?} folded under its key {:?}",
                row.team, key
            ));
        }
        store.add_loyalty(&key, row.total);
    }
    store.loyalty
}
