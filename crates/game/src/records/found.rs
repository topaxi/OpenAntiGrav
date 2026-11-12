//! The hidden campaign events a player has found, kept per campaign so a find
//! survives the next launch (`oag_ui::frontend::Reveal`).
//!
//! One row per (campaign, event), filed under [`super::campaign_title`] and
//! the event's name, never an index: a campaign that reorders or drops events
//! only misses the lookup. A file written before this existed has no
//! `[[campaign_found]]` table and reads as an empty set.

use super::{CampaignFoundRecord, Store};

impl Store {
    /// Whether `event` of `title`'s campaign has been found.
    #[must_use]
    pub fn campaign_found(&self, title: &str, event: &str) -> bool {
        let title = title.to_lowercase();
        self.campaign_found
            .iter()
            .any(|row| row.title == title && row.event == event)
    }

    /// Keeps `event` as found; the rows stay sorted. Returns whether it was
    /// new.
    pub fn record_found(&mut self, title: &str, event: &str) -> bool {
        if self.campaign_found(title, event) {
            return false;
        }
        self.campaign_found.push(CampaignFoundRecord {
            title: title.to_lowercase(),
            event: event.to_string(),
        });
        self.campaign_found
            .sort_by(|a, b| (&a.title, &a.event).cmp(&(&b.title, &b.event)));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Store, parse};

    #[test]
    fn a_found_set_round_trips_and_an_old_save_reads_empty() {
        let mut store = Store::default();
        assert!(store.record_found("Wipeout 2048", "A P Ship Challenge"));
        assert!(!store.record_found("wipeout 2048", "A P Ship Challenge"));
        let text = toml::to_string_pretty(&store).expect("serialises");
        let (parsed, notes) = parse(&text).expect("parses back");
        assert!(notes.is_empty(), "{notes:?}");
        assert!(parsed.campaign_found("wipeout 2048", "A P Ship Challenge"));
        assert!(!parsed.campaign_found("wipeout 2048", "B P Ship Challenge"));
        assert!(!parsed.campaign_found("omega", "A P Ship Challenge"));

        let (old, notes) = parse("xp = 5\n").expect("an old file parses");
        assert!(notes.is_empty(), "{notes:?}");
        assert!(!old.campaign_found("wipeout 2048", "A P Ship Challenge"));
    }
}
