//! Wipeout Pure's campaign list over the menu stage: what is on screen and
//! the events behind its rows. The flow that opens, answers and closes it is
//! `crate::session::pure_campaign`; the rows are
//! `oag_game::campaign::pure_list`'s and the screen `oag_ui_screens::pure_campaign`'s.
//!
//! Like [`crate::campaign_stage::CampaignStage`] it takes the tick whole while
//! open and is drawn over the page behind it.

use oag_game::campaign::pure_list::{self, PureCampaign};
use oag_game::records::Store;
use oag_tables::pure_ai_stats::PureClass;
use oag_tables::pure_campaign::Event;
use oag_ui::language::StringTable;
use oag_ui_screens::pure_campaign::List;

/// The campaign list, open.
pub(crate) struct PureCampaignStage {
    list: List,
    campaign: PureCampaign,
    /// The events behind the rows, which a row's tag indexes.
    events: Vec<Event>,
    class: usize,
    strings: StringTable,
    title: &'static oag_title::Title,
}

impl PureCampaignStage {
    /// The list for `campaign`, on `class` (an index into
    /// [`PureClass::ALL`]), read against `records`.
    pub(crate) fn new(
        campaign: PureCampaign,
        strings: StringTable,
        title: &'static oag_title::Title,
        class: usize,
        records: &Store,
    ) -> Self {
        let (labels, _) = pure_list::wording(&strings);
        let mut stage = Self {
            list: List::new(labels, String::new(), Vec::new()),
            campaign,
            events: Vec::new(),
            class: class.min(PureClass::ALL.len() - 1),
            strings,
            title,
        };
        stage.refresh(records);
        stage
    }

    /// Rebuilds the rows from `records`: after a class step, or a medal that
    /// may have opened a gate.
    pub(crate) fn refresh(&mut self, records: &Store) {
        let (rows, events, class_name) = pure_list::present(
            &self.campaign,
            &self.strings,
            self.title.name,
            PureClass::ALL[self.class],
            records,
        );
        self.list.replace(class_name, rows);
        self.events = events;
    }

    /// Moves the class selector, wrapping, and rebuilds.
    pub(crate) fn step_class(&mut self, by: i32, records: &Store) {
        let count = PureClass::ALL.len() as i32;
        self.class = (self.class as i32 + by).rem_euclid(count) as usize;
        self.refresh(records);
    }

    /// Puts the cursor on the event named `name`, switching to its class first.
    pub(crate) fn select(&mut self, name: &str, records: &Store) {
        let Some(event) = PureClass::ALL
            .iter()
            .flat_map(|class| self.campaign.events(*class))
            .find(|event| event.name == name)
        else {
            return;
        };
        if let Some(class) = event.class
            && let Some(index) = PureClass::ALL.iter().position(|c| *c == class)
            && index != self.class
        {
            self.class = index;
            self.refresh(records);
        }
        if let Some(tag) = self.events.iter().position(|e| e.name == name) {
            self.list.select_tag(tag);
        }
    }

    /// The class being played, as an index into [`PureClass::ALL`].
    pub(crate) fn class(&self) -> usize {
        self.class
    }

    pub(crate) fn list(&self) -> &List {
        &self.list
    }

    pub(crate) fn list_mut(&mut self) -> &mut List {
        &mut self.list
    }

    /// The event behind a row's tag.
    pub(crate) fn event(&self, tag: usize) -> Option<&Event> {
        self.events.get(tag)
    }

    /// The archive entries of the circuits the shell's catalogue does not list
    /// (the `Classic` ones), by `PI_Track` name.
    pub(crate) fn classic_entries(&self) -> std::collections::BTreeMap<String, String> {
        self.campaign
            .definition
            .tracks
            .iter()
            .filter_map(|track| {
                self.campaign
                    .track_entry(&track.name)
                    .map(|entry| (track.name.clone(), entry))
            })
            .collect()
    }
}
