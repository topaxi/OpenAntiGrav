//! Wipeout Pure's SINGLE PLAYER chain over the menu stage: the screen the
//! player is on and the rows behind it. The flow that opens, answers and closes
//! it is `crate::session::pure_chain`; the state machine is
//! `oag_game::campaign::pure_chain`'s and the screen `oag_ui_screens::pure_chain`'s.
//!
//! Like [`crate::campaign_stage::CampaignStage`] it takes the tick whole while
//! open and is drawn over the page behind it.

use oag_game::campaign::pure_chain::{self, Chain};
use oag_game::records::Store;
use oag_tables::pure_flow::Step;
use oag_ui::language::StringTable;
use oag_ui_screens::pure_chain::Choice;

/// The chain, open.
pub(crate) struct PureChainStage {
    chain: Chain,
    choice: Choice,
    strings: StringTable,
    title: &'static oag_title::Title,
}

impl PureChainStage {
    /// The chain on its first screen, read against `records`.
    pub(crate) fn new(
        chain: Chain,
        strings: StringTable,
        title: &'static oag_title::Title,
        records: &Store,
    ) -> Self {
        let view = pure_chain::present(&chain, &strings, title, records);
        Self {
            choice: Choice::new(view.title, view.context, view.pitch, view.rows),
            chain,
            strings,
            title,
        }
    }

    /// Rebuilds the screen from `records`, on the same screen when `same` and
    /// as a fresh one (cursor at the top, or on the class being played) when
    /// not.
    pub(crate) fn refresh(&mut self, records: &Store, same: bool) {
        let view = pure_chain::present(&self.chain, &self.strings, self.title, records);
        if same {
            self.choice.replace(view.rows);
            return;
        }
        let mut choice = Choice::new(view.title, view.context, view.pitch, view.rows);
        if self.chain.at() == Step::Class
            && let Some(tag) = self
                .chain
                .class_tag()
        {
            choice.select_tag(tag);
        }
        self.choice = choice;
    }

    pub(crate) fn title(&self) -> &'static oag_title::Title {
        self.title
    }

    pub(crate) fn chain(&self) -> &Chain {
        &self.chain
    }

    pub(crate) fn chain_mut(&mut self) -> &mut Chain {
        &mut self.chain
    }

    pub(crate) fn choice(&self) -> &Choice {
        &self.choice
    }

    pub(crate) fn choice_mut(&mut self) -> &mut Choice {
        &mut self.choice
    }
}
