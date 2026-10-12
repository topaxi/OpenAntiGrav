//! Wipeout Pure's SINGLE PLAYER chain over the menu stage: the screen the
//! player is on and the rows behind it. The flow that opens, answers and closes
//! it is `crate::session::pure_chain`; the state machine is
//! `oag_game::campaign::pure_chain`'s and the screen `oag_ui_screens::pure_chain`'s.
//!
//! Like [`crate::campaign_stage::CampaignStage`] it takes the tick whole while
//! open and is drawn over the page behind it.

use oag_game::campaign::pure_art::Player;
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
    art: Player,
    /// The screen and row the art was last asked for, so a tick that moved
    /// nothing does not rebuild the event lists to find the same key.
    art_for: Option<(Step, usize)>,
}

impl PureChainStage {
    /// The chain on its first screen, read against `records`.
    pub(crate) fn new(
        chain: Chain,
        strings: StringTable,
        title: &'static oag_title::Title,
        records: &Store,
        art: Player,
    ) -> Self {
        let view = pure_chain::present(&chain, &strings, title, records);
        let mut open = Self {
            choice: Choice::new(view.title, view.context, view.pitch, view.rows),
            chain,
            strings,
            title,
            art,
            art_for: None,
        };
        open.follow_art();
        open
    }

    /// Puts the art of the row under the cursor on the right-hand side, when the
    /// screen or the row changed since the last call.
    pub(crate) fn follow_art(&mut self) {
        let tag = self.choice.selected_row().map(|row| row.tag);
        let at = tag.map(|tag| (self.chain.at(), tag));
        if at.is_some() && at == self.art_for {
            return;
        }
        self.art_for = at;
        self.art.show(tag.and_then(|tag| self.chain.art_key(tag)));
    }

    pub(crate) fn art(&self) -> &Player {
        &self.art
    }

    pub(crate) fn art_mut(&mut self) -> &mut Player {
        &mut self.art
    }

    /// Rebuilds the screen from `records`, on the same screen when `same` and
    /// as a fresh one (cursor at the top, or on the class being played) when
    /// not.
    pub(crate) fn refresh(&mut self, records: &Store, same: bool) {
        let view = pure_chain::present(&self.chain, &self.strings, self.title, records);
        self.art_for = None;
        if same {
            self.choice.replace(view.rows);
            self.follow_art();
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
        self.follow_art();
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
