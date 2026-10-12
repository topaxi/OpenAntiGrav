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
use oag_game::campaign::pure_chain::{Chain, Chosen, Start};
use oag_game::campaign::pure_list;
use oag_tables::pure_ai_stats::PureClass;
use oag_ui::menu::nav::Nav;
use oag_ui_screens::pure_chain::Event;

use crate::pure_chain_stage::PureChainStage;
use crate::stage::Stage;

use super::Session;

impl Session {
    /// The main menu's campaign row on a title whose campaign is a chain:
    /// reads the disc's screens and events and opens `Single Player`. Called by
    /// [`Self::open_campaign`] with the archives it already opened.
    pub(super) fn open_pure_chain(&mut self, archives: &mut oag_assets::Archives) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let title = shell.title;
        let strings = shell.strings.clone();
        let loaded = pure_list::load(archives, title)
            .and_then(|campaign| Ok((campaign, pure_list::load_flow(archives, title)?)));
        let (campaign, flow) = match loaded {
            Ok(loaded) => loaded,
            Err(error) => {
                warn!("{error:#} - SINGLE PLAYER has nothing to show");
                return;
            }
        };
        let class = PureClass::ALL[self.pure_chain_class.min(PureClass::ALL.len() - 1)];
        let open = PureChainStage::new(Chain::new(campaign, flow, class), strings, title, &self.records);
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
        if shell.title.campaign.list_entry.is_none() || oag_game::unlock::unlock_all() {
            return Vec::new();
        }
        let Some(mut archives) = self.open_title_archives("Team Selection") else {
            return Vec::new();
        };
        match pure_list::load(&mut archives, shell.title) {
            Ok(campaign) => {
                let medal_of = oag_game::unlock::campaign_medal_of(&self.records, shell.title.name);
                pure_list::locked_teams(&campaign, &medal_of, "")
            }
            Err(error) => {
                warn!("{error:#} - no team is gated");
                Vec::new()
            }
        }
    }

    /// Opens the chain again on the screen that picked the event named `name`,
    /// with the cursor on it: the race it was launched from has ended or been
    /// left.
    pub(crate) fn reopen_pure_chain(&mut self, name: &str) {
        self.open_campaign();
        if let Stage::Menu(stage) = &mut self.stage
            && let Some(open) = stage.pure_chain.as_mut()
        {
            let found = PureClass::ALL
                .iter()
                .flat_map(|class| open.chain().campaign().events(*class))
                .find(|event| event.name == name);
            if let Some(event) = found {
                open.chain_mut().return_to(&event);
                open.refresh(&self.records, false);
                if let Some(tag) = open.chain().tag_of(&event) {
                    open.choice_mut().select_tag(tag);
                }
            }
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
                    Chosen::Start(Start::Event(event)) => {
                        self.play_navs(Some(Nav::Accept));
                        self.launch_campaign_cell(event.cell, None);
                    }
                    Chosen::Start(Start::FreePlay { track, class }) => {
                        self.play_navs(Some(Nav::Accept));
                        self.launch_free_play(&track, class);
                    }
                }
            }
        }
    }

    /// Free Play: a plain race on the circuit and class chosen, through the race
    /// box's own launch (`Team Selection`, then the race), with no cell and so no
    /// record. The disc names the mode and its results screen and authors no
    /// medal law for it (chosen, not measured).
    fn launch_free_play(&mut self, track: &str, class: PureClass) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let ladder = shell
            .title
            .race
            .speed_classes
            .map_or(&[][..], |classes| classes.names);
        let Some(class_name) = ladder
            .iter()
            .find(|name| name.eq_ignore_ascii_case(class.title()))
        else {
            warn!("Free Play: {} is not on this title's ladder", class.title());
            return;
        };
        let mode = oag_race::Mode::SingleRace;
        if shell.track(mode, track).is_none() {
            warn!("Free Play: {track} is not a circuit this source offers for a plain race");
            return;
        }
        self.settings.race.mode = mode.name().to_string();
        self.settings.race.track = track.to_string();
        self.settings.race.class = (*class_name).to_string();
        self.campaign_cell = None;
        if let Stage::Menu(stage) = &mut self.stage {
            stage.pure_chain = None;
        }
        if !self.open_ship_picker() {
            self.launch_from_settings();
        }
    }
}

fn class_index(class: PureClass) -> usize {
    PureClass::ALL
        .iter()
        .position(|c| *c == class)
        .unwrap_or_default()
}
