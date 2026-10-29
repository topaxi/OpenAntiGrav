//! Opens and drives Wipeout Pure's SINGLE PLAYER chain over the menus - the
//! shape `session::campaign` has for the grid titles. The order of the screens
//! is the disc's (`oag_tables::pure_flow`), the state machine is
//! `oag_game::campaign::pure_chain`'s, the screen is
//! `oag_ui_screens::pure_chain`'s, and what the last screen before
//! `Team Selection` starts is the launch every campaign cell takes
//! (`Session::launch_campaign_cell`): the cell is carried to the race, which
//! evaluates its medal, and the finished race records it
//! (`Store::record_campaign`) into the table the next opening of the chain reads
//! its gates from.

use log::warn;
use oag_game::campaign::pure_art::{Fills, Player};
use oag_game::campaign::pure_chain::{Chain, Chosen};
use oag_game::campaign::pure_list;
use oag_tables::pure_ai_stats::PureClass;
use oag_tables::pure_flow::Step;
use oag_ui::menu::nav::Nav;
use oag_ui_screens::pure_chain::Event;

use crate::pure_chain_stage::PureChainStage;
use crate::stage::Stage;

use super::Session;

impl Session {
    /// The main menu's campaign row on a title whose campaign is a chain:
    /// reads the disc's screens and events and opens `Single Player`. Called by
    /// [`Self::open_campaign`] with the archives it already opened.
    pub(super) fn open_pure_chain(&mut self, mut archives: oag_assets::Archives) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let title = shell.title;
        let strings = shell.strings.clone();
        let loaded = pure_list::load(&mut archives, title)
            .and_then(|campaign| Ok((campaign, pure_list::load_flow(&mut archives, title)?)));
        let (campaign, flow) = match loaded {
            Ok(loaded) => loaded,
            Err(error) => {
                warn!("{error:#} - SINGLE PLAYER has nothing to show");
                return;
            }
        };
        let class = PureClass::ALL[self.pure_chain_class.min(PureClass::ALL.len() - 1)];
        let art = Player::new(
            archives,
            shell.sprites.clone(),
            shell.globals.clone(),
            shell.strings.clone(),
            (
                title
                    .campaign
                    .flow
                    .map_or_else(String::new, |files| files.selection.to_string()),
                Step::Class.screen().to_string(),
            ),
            Fills::of(&campaign, title.name, self.records.rows()),
        );
        let open = PureChainStage::new(
            Chain::new(campaign, flow, class),
            strings,
            title,
            &self.records,
            art,
        );
        self.pure_classic_entries = open
            .chain()
            .campaign()
            .definition
            .tracks
            .iter()
            .filter_map(|track| {
                open.chain()
                    .campaign()
                    .track_entry(&track.name)
                    .map(|entry| (track.name.clone(), entry))
            })
            .collect();
        let mut open = open;
        // A relaunch reopens on the event last raced, as a session that raced it does.
        if let Some(name) = self
            .campaign_title()
            .and_then(|title| self.records.campaign_cursor(&title).grid)
        {
            put_on_event(&mut open, &name, &self.records);
        }
        if let Stage::Menu(stage) = &mut self.stage {
            stage.pure_chain = Some(open);
        }
    }

    /// The ship directories `Team Selection` must not offer yet: the teams
    /// the disc gates (`PI_Team`'s `<Unlock>` rows) that the profile has not
    /// opened. Empty on a title with no list campaign, and under `--unlock-all`.
    pub(crate) fn pure_locked_teams(&self) -> Vec<String> {
        let Some(shell) = self.shell.as_ref() else {
            return Vec::new();
        };
        // Before the source is opened: no list campaign, nothing to read.
        if shell.title.campaign.list_entry.is_none() || oag_game::unlock::unlock_all() {
            return Vec::new();
        }
        let Some(mut archives) = self.open_title_archives("Team Selection") else {
            return Vec::new();
        };
        pure_list::locked_teams_on(shell.title, &mut archives, &self.records)
    }

    /// Opens the chain again on the screen that picked the event named `name`,
    /// with the cursor on it: the race it was launched from has ended or been
    /// left.
    pub(crate) fn reopen_pure_chain(&mut self, name: &str) {
        self.open_campaign();
        if let Stage::Menu(stage) = &mut self.stage
            && let Some(open) = stage.pure_chain.as_mut()
        {
            put_on_event(open, name, &self.records);
        }
    }

    /// One tick of the open screen: pad and pointer, and what came of either.
    pub(crate) fn tick_pure_chain(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.pure_chain.as_mut() else {
            return;
        };
        let mut events = open.choice_mut().update(self.controls.buttons_mut());
        events.extend(open.choice_mut().pointer(pointer, &stage.skin));
        open.choice_mut().follow(&stage.skin);
        open.follow_art();
        open.art_mut().advance(1.0 / 60.0);
        open.advance_description(1.0 / 60.0);
        for event in events {
            self.handle_pure_chain(event);
        }
    }

    /// What one event of the open screen does. The pad path and the pointer path
    /// both end here, so a locked row refuses both alike.
    pub(crate) fn handle_pure_chain(&mut self, event: Event) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.pure_chain.as_mut() else {
            return;
        };
        match event {
            Event::Moved => self.play_navs(Some(Nav::Moved(None))),
            Event::Back => {
                self.pure_chain_class = class_index(open.chain().class());
                if open.chain_mut().back() {
                    open.refresh(&self.records, false);
                } else {
                    stage.pure_chain = None;
                }
                self.play_navs(Some(Nav::Decline));
            }
            Event::Confirmed(tag) => {
                let title = open.title();
                let chosen =
                    oag_game::campaign::pure_chain::choose(open.chain_mut(), tag, title, &self.records);
                self.pure_chain_class = class_index(open.chain().class());
                match chosen {
                    Chosen::Refused => self.play_navs(Some(Nav::Decline)),
                    Chosen::Advanced => {
                        open.refresh(&self.records, false);
                        self.play_navs(Some(Nav::Accept));
                    }
                    Chosen::Start(event) => {
                        self.play_navs(Some(Nav::Accept));
                        self.keep_event_cursor(&event.name);
                        self.launch_campaign_cell(event.cell, None);
                    }
                }
            }
        }
    }
}

/// Puts `open` on the screen that picked the event named `name`, with the cursor on it; an
/// event the disc no longer lists leaves the chain where it is.
fn put_on_event(open: &mut PureChainStage, name: &str, records: &oag_game::records::Store) {
    let Some(event) = open.chain().campaign().event_named(name) else {
        return;
    };
    open.chain_mut().return_to(&event);
    open.refresh(records, false);
    if let Some(tag) = open.chain().tag_of(&event) {
        open.choice_mut().select_tag(tag);
    }
}

fn class_index(class: PureClass) -> usize {
    PureClass::ALL
        .iter()
        .position(|c| *c == class)
        .unwrap_or_default()
}
