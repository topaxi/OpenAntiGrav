//! Which circuits the Race Box offers: the disc's own `<Unlock Grid="...">`
//! rows, evaluated against the campaign progress this build persists.
//!
//! **Pulse only.** `TrackSelection_PopulateList` (`0x088edf3c`) keeps a
//! `PI_Track` only when `Definition_IsUnlocked` (`0x0888e29c`) passes - a
//! locked circuit is **absent from the list, not greyed** (decompile; and the
//! 2026-09-29 live capture, where the list went from 3 circuits to 24 with the
//! dev byte set). A named `<Unlock Grid="GridN">` row evaluates
//! `Unlock_GridPointsMet` (`0x0888ebd8`): the named grid's own medal points
//! reach its `RequiredPoints`. See `docs/formats/race-setup.md`, "Two unlock
//! axes", and `docs/ui/selection-screens.md`.
//!
//! The predicate is [`oag_tables::race_campaign::grid_points_met`], called
//! directly rather than through the campaign screen's `grid_is_unlocked`: that
//! one answers "is this *grid* open" and short-circuits `true` for `grid0`
//! (shipped with no lock), which would open every `Grid0`-gated circuit on a
//! fresh profile.
//!
//! The mode-gated per-track byte at `+0x16e` that `TrackSelection_PopulateList`
//! tests when its cached `Mode == 6` is `availableInZone` (the definition loader
//! writes it from that attribute; 6 is `Zone`), which the Zone circuit list
//! already filters on, so this gate has nothing to add for it. **Not modelled,
//! and open:** the always-fails byte at `+0x99`.
//!
//! `--unlock-all` ([`Gate::open`]) is **ours**: a developer and capture escape
//! that mirrors, as context only, the original's own dev byte at profile
//! `+0x45f`. It is not a game feature.

use std::sync::atomic::{AtomicBool, Ordering};

use oag_tables::race_campaign::{self, Grid, Medal};

use crate::records::{self, Store};
use oag_raceplay::catalogue::{LoyaltyRow, Track};

/// A campaign medal as the campaign tables name it. The one conversion,
/// shared by the campaign screens and this gate.
#[must_use]
pub fn to_campaign_medal(medal: records::Medal) -> Medal {
    match medal {
        records::Medal::Gold => Medal::Gold,
        records::Medal::Silver => Medal::Silver,
        records::Medal::Bronze => Medal::Bronze,
    }
}

/// The best medal `records` holds for a campaign cell of `title`, in the
/// shape [`race_campaign::grid_points_met`] and `Grid::points_earned` take.
pub fn campaign_medal_of<'a>(
    records: &'a Store,
    title: &'a str,
) -> impl Fn(&str) -> Option<Medal> + 'a {
    move |cell| {
        records
            .campaign_medal(title, cell)?
            .best_medal
            .map(to_campaign_medal)
    }
}

static UNLOCK_ALL: AtomicBool = AtomicBool::new(false);

/// Turns every gate off for this process - `--unlock-all`. **Ours**, for
/// captures and tests; a process-wide switch because the live race box and the
/// `--menu-page` stills must agree without a flag threaded through both.
pub fn set_unlock_all(on: bool) {
    UNLOCK_ALL.store(on, Ordering::Relaxed);
}

/// Whether [`set_unlock_all`] is on.
#[must_use]
pub fn unlock_all() -> bool {
    UNLOCK_ALL.load(Ordering::Relaxed)
}

/// What gates a title's circuits.
#[derive(Debug, Clone, Default)]
pub struct Gate {
    /// The campaign grids the `<Unlock>` names resolve against. `None` means
    /// nothing gates: a title that authors no `<Unlock>` on its circuits, or
    /// `--unlock-all`.
    grids: Option<Vec<Grid>>,
}

impl Gate {
    /// Nothing gates: every circuit is offered.
    #[must_use]
    pub fn open() -> Self {
        Self { grids: None }
    }

    /// Circuits gate on `grids`' own medal points.
    #[must_use]
    pub fn on_grids(grids: Vec<Grid>) -> Self {
        Self { grids: Some(grids) }
    }

    /// Whether `track` is offered given `records` for the title `title`.
    #[must_use]
    pub fn offers(&self, track: &Track, records: &Store, title: &str) -> bool {
        let Some(grids) = self.grids.as_deref() else {
            return true;
        };
        if unlock_all() {
            return true;
        }
        match track.unlock_grid.as_deref() {
            None => true,
            Some(name) => {
                race_campaign::grid_points_met(grids, name, &campaign_medal_of(records, title))
            }
        }
    }
}

impl Gate {
    /// `tracks` cut to the circuits [`Self::offers`] passes, order kept.
    #[must_use]
    pub fn offered(&self, tracks: &[Track], records: &Store, title: &str) -> Vec<Track> {
        let keep = |track: &&Track| self.offers(track, records, title);
        tracks.iter().filter(keep).cloned().collect()
    }

    /// The gate `title`'s circuits answer to, read off its own archives.
    ///
    /// Pulse names its campaign grids in `<Unlock Grid="...">`; no other title
    /// authors a circuit `<Unlock>` this build reads (Pure has none, HD and
    /// Omega are not wired), so they get [`Self::open`]. A Pulse source whose
    /// grids will not read is logged and left open - there is nothing to gate
    /// against - rather than locking every circuit away.
    #[must_use]
    pub fn read(title: &oag_title::Title, archives: &mut oag_assets::Archives) -> Self {
        let campaign = title.campaign;
        let Some(entry) = campaign
            .definition_entry
            .filter(|_| campaign.circuit_unlocks)
        else {
            return Self::open();
        };
        match crate::campaign::read_grids(archives, entry, campaign.grid_archive) {
            Ok(grids) => Self::on_grids(grids),
            Err(error) => {
                log::warn!("{error:#} - no campaign grids, so no circuit is locked");
                Self::open()
            }
        }
    }
}

/// Whether a craft variant's `<Unlock>` rows pass: `Definition_IsUnlocked`'s
/// own combine, restricted to the one condition Pulse's variants author
/// (`loyalty`).
///
/// A row that is not `Exclusive` must hold together with every other such
/// row; an `Exclusive` row that holds unlocks the variant on its own, ahead
/// of anything before it; an `Exclusive` row that fails leaves the variant
/// locked unless a later one holds. No rows: unlocked. `Team="any"` is met by
/// the best single team ([`Store::loyalty_best`]), any other name by that
/// team's own total. [`unlock_all`] opens everything - ours, not the
/// original's.
#[must_use]
pub fn loyalty_unlocked(rows: &[LoyaltyRow], records: &Store, title: &str) -> bool {
    if unlock_all() {
        return true;
    }
    let met = |row: &LoyaltyRow| {
        row.loyalty != 0
            && row.loyalty
                <= if row.team.eq_ignore_ascii_case("any") {
                    records.loyalty_best(title)
                } else {
                    records.loyalty_total(title, &row.team)
                }
    };
    let mut unlocked = true;
    for row in rows {
        if row.exclusive {
            if met(row) {
                return true;
            }
            unlocked = false;
        } else if row.loyalty != 0 {
            unlocked = unlocked && met(row);
        }
    }
    unlocked
}

/// Whether `title` gates its craft variants at all: Pulse authors the
/// loyalty rows, and no other title's definition here carries them
/// ([`oag_title::Campaign::loyalty_unlocks`]).
#[must_use]
pub fn gates_variants(title: &oag_title::Title) -> bool {
    title.campaign.loyalty_unlocks
}

/// `tracks` as Track Select lists them from `archives`: gated when `kind` is Track
/// and a source is at hand, whole otherwise. The records are read from disk
/// the way every capture reads them.
#[must_use]
pub fn offered_on(
    kind: oag_ui_screens::picker::Kind,
    archives: Option<&mut oag_assets::Archives>,
    title: &oag_title::Title,
    tracks: &[Track],
) -> Vec<Track> {
    match archives {
        Some(archives) if kind == oag_ui_screens::picker::Kind::Track => {
            Gate::read(title, archives).offered(tracks, &records::load(), title.name)
        }
        _ => tracks.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(team: &str, loyalty: u32, exclusive: bool) -> LoyaltyRow {
        LoyaltyRow {
            team: team.to_string(),
            loyalty,
            exclusive,
        }
    }

    #[test]
    fn no_rows_means_unlocked_and_a_missed_exclusive_row_means_locked() {
        let store = Store::default();
        assert!(loyalty_unlocked(&[], &store, "t"));
        assert!(!loyalty_unlocked(&[row("a", 10, true)], &store, "t"));
    }

    #[test]
    fn exclusive_rows_are_alternatives_and_any_is_the_best_team() {
        let mut store = Store::default();
        store.record_loyalty("t", "b", 50);
        let rows = [row("a", 10, true), row("any", 50, true)];
        assert!(loyalty_unlocked(&rows, &store, "t"), "b alone reaches any");
        store.record_loyalty("t", "c", 30);
        let rows = [row("a", 10, true), row("any", 80, true)];
        assert!(
            !loyalty_unlocked(&rows, &store, "t"),
            "50 and 30 are not 80"
        );
    }

    #[test]
    fn plain_rows_must_all_hold() {
        let mut store = Store::default();
        store.record_loyalty("t", "a", 10);
        let rows = [row("a", 10, false), row("b", 5, false)];
        assert!(!loyalty_unlocked(&rows, &store, "t"));
        store.record_loyalty("t", "b", 5);
        assert!(loyalty_unlocked(&rows, &store, "t"));
    }
}
