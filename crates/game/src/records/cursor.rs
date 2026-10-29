//! Where a title's campaign screens last stood, kept per campaign so the next
//! visit - this launch or the next - opens where the player left off.
//!
//! One row per campaign (filed under [`super::campaign_title`], or a mod
//! campaign's own records title, so a mod's campaigns never share a row with
//! the disc's). The grid is kept **by name**, never by index: a mod campaign
//! that reorders or drops grids simply misses the lookup and the screen opens
//! on its default. The cell is the `(x, y)` slot of the original's one shared
//! cursor (`docs/ui/campaign-screens.md`, "Where the cursor lives"), not a
//! cell name, for the same reason. Written when the campaign screens close or
//! step back a level (modern saves: no slot, no prompt), never per move, and
//! off the render thread ([`super::save_in_background`]).

use super::{CampaignCursorRecord, Store};

/// A campaign's remembered position.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CampaignCursor {
    /// The grid `Grid Selection` was last on, by name.
    pub grid: Option<String>,
    /// `Cell Selection`'s `(x, y)` slot.
    pub slot: Option<(u32, u32)>,
}

impl Store {
    /// The position last kept for `title`'s campaign; empty if none.
    #[must_use]
    pub fn campaign_cursor(&self, title: &str) -> CampaignCursor {
        let title = title.to_lowercase();
        self.campaign_cursor
            .iter()
            .find(|row| row.title == title)
            .map_or_else(CampaignCursor::default, |row| CampaignCursor {
                grid: row.grid.clone(),
                slot: row.slot.map(|[x, y]| (x, y)),
            })
    }

    /// Keeps `cursor` as `title`'s position; the rows stay sorted. An empty
    /// cursor writes nothing.
    pub fn set_campaign_cursor(&mut self, title: &str, cursor: &CampaignCursor) {
        if *cursor == CampaignCursor::default() {
            return;
        }
        let title = title.to_lowercase();
        let row = CampaignCursorRecord {
            grid: cursor.grid.clone(),
            slot: cursor.slot.map(|(x, y)| [x, y]),
            title,
        };
        match self
            .campaign_cursor
            .iter_mut()
            .find(|have| have.title == row.title)
        {
            Some(have) => *have = row,
            None => {
                self.campaign_cursor.push(row);
                self.campaign_cursor.sort_by(|a, b| a.title.cmp(&b.title));
            }
        }
    }
}
