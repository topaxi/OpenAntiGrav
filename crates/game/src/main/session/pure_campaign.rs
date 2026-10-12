//! Opens and drives Wipeout Pure's campaign list over the menus - the shape
//! `session::campaign` has for the grid titles. The rows are
//! `oag_game::campaign::pure_list`'s, the screen is
//! `oag_ui_screens::pure_campaign`'s, and what a confirmed row does is the
//! launch every campaign cell takes (`Session::launch_campaign_cell`): the cell
//! is carried to the race, which evaluates its medal, and the finished race
//! records it (`Store::record_campaign`) into the table the next opening of
//! this list reads its gates from.

use log::warn;
use oag_game::campaign::pure_list;
use oag_ui::menu::nav::Nav;
use oag_ui_screens::pure_campaign::{Event, Status};

use crate::pure_campaign_stage::PureCampaignStage;
use crate::stage::Stage;

use super::Session;

impl Session {
    /// `RACE CAMPAIGN` on a title whose campaign is a list: reads the disc's
    /// events and opens the list on the class it was last left on. Called by
    /// [`Self::open_campaign`] with the archives it already opened.
    pub(super) fn open_pure_campaign(&mut self, archives: &mut oag_assets::Archives) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let title = shell.title;
        let strings = shell.strings.clone();
        let campaign = match pure_list::load(archives, title) {
            Ok(campaign) => campaign,
            Err(error) => {
                warn!("{error:#} - RACE CAMPAIGN has nothing to show");
                return;
            }
        };
        let open = PureCampaignStage::new(
            campaign,
            strings,
            title,
            self.pure_campaign_class,
            &self.records,
        );
        self.pure_classic_entries = open.classic_entries();
        if let Stage::Menu(stage) = &mut self.stage {
            stage.pure_campaign = Some(open);
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

    /// Opens the list again with the cursor on the event named `name`: the
    /// race it was launched from has ended or been left.
    pub(crate) fn reopen_pure_campaign(&mut self, name: &str) {
        self.open_campaign();
        if let Stage::Menu(stage) = &mut self.stage
            && let Some(open) = stage.pure_campaign.as_mut()
        {
            open.select(name, &self.records);
        }
    }

    /// One tick of the open list: pad and pointer, and what came of either.
    pub(crate) fn tick_pure_campaign(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.pure_campaign.as_mut() else {
            return;
        };
        let mut events = open.list_mut().update(self.controls.buttons_mut());
        events.extend(open.list_mut().pointer(pointer, &stage.skin));
        open.list_mut().follow(&stage.skin);
        for event in events {
            self.handle_pure_campaign(event);
        }
    }

    /// What one event of the list does. The pad path and the pointer path both
    /// end here, so a locked row refuses both alike.
    pub(crate) fn handle_pure_campaign(&mut self, event: Event) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.pure_campaign.as_mut() else {
            return;
        };
        match event {
            Event::Moved => self.play_navs(Some(Nav::Moved(None))),
            Event::ClassStep(by) => {
                open.step_class(by, &self.records);
                self.pure_campaign_class = open.class();
                self.play_navs(Some(Nav::Stepped(None)));
            }
            Event::Back => {
                self.pure_campaign_class = open.class();
                stage.pure_campaign = None;
                self.play_navs(Some(Nav::Decline));
            }
            Event::Confirmed(tag) => {
                let locked = open
                    .list()
                    .rows()
                    .iter()
                    .any(|row| !row.header && row.tag == tag && row.status == Status::Locked);
                let Some(cell) = open.event(tag).map(|e| e.cell.clone()) else {
                    return;
                };
                if locked {
                    self.play_navs(Some(Nav::Decline));
                    return;
                }
                self.pure_campaign_class = open.class();
                self.play_navs(Some(Nav::Accept));
                self.launch_campaign_cell(cell, None);
            }
        }
    }
}
