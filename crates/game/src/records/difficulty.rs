//! The campaign difficulty rung a player last browsed to, kept per title so
//! the next visit to that title's campaign opens on it.
//!
//! One row per campaign title (filed under [`super::campaign_title`], so a
//! title's two pressings with different campaigns stay apart). Not shared
//! across titles: each title's rung set and default differ. Written by the
//! campaign screen when the rung changes (modern saves: no slot, no prompt).

use super::{CampaignDifficultyRecord, Difficulty, Store};

impl Store {
    /// The rung last browsed on `title`'s campaign screen, if the player ever
    /// changed it. `title` is the string [`super::campaign_title`] builds.
    #[must_use]
    pub fn campaign_difficulty(&self, title: &str) -> Option<Difficulty> {
        let title = title.to_lowercase();
        self.campaign_difficulty
            .iter()
            .find(|row| row.title == title)
            .map(|row| row.difficulty)
    }

    /// Remembers `difficulty` as `title`'s browsed rung; the rows stay sorted.
    pub fn set_campaign_difficulty(&mut self, title: &str, difficulty: Difficulty) {
        let title = title.to_lowercase();
        match self
            .campaign_difficulty
            .iter_mut()
            .find(|row| row.title == title)
        {
            Some(row) => row.difficulty = difficulty,
            None => {
                self.campaign_difficulty
                    .push(CampaignDifficultyRecord { title, difficulty });
                self.campaign_difficulty
                    .sort_by(|a, b| a.title.cmp(&b.title));
            }
        }
    }
}
