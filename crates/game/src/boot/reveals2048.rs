//! Which hidden events of 2048's campaign map the profile has earned the right
//! to find, and which it has found (`oag_ui::frontend::campaign_reveal`).
//!
//! The rank gate is `Store::rank_reveals`: the profile's one XP total against
//! this title's own `XPRanks.xml` (each title ranks the shared total by its
//! own table). Reads the table afresh, as [`super::tiers2048::refresh_locks`]
//! reads the grids, so `boot` carries nothing the front end has no use for.

use oag_ui::frontend::{Frontend, Reveal};

/// Folds the profile into the map's hidden events: found ones from the save,
/// the rest [`Reveal::Revealable`] once the rank is met. A no-op on a map with
/// no rank-gated event.
pub fn refresh(
    frontend: &mut Frontend,
    source: &str,
    records: &crate::records::Store,
    campaign_title: &str,
) {
    if !frontend.campaign_events().iter().any(|event| event.rank_gated) {
        return;
    }
    let Ok(opened) = oag_source::title::open_source(source, Vec::new(), Vec::new()) else {
        return;
    };
    let mut archives = opened.archives;
    let ranks = oag_2048::campaign::xp_tables(&mut archives).map(|(_, ranks)| ranks);
    frontend.refresh_campaign_reveals(|event| {
        if records.campaign_found(campaign_title, &event.name) {
            Reveal::Found
        } else if ranks
            .as_ref()
            .is_some_and(|ranks| records.rank_reveals(ranks, event.rank_required))
        {
            Reveal::Revealable
        } else {
            Reveal::Hidden
        }
    });
}
