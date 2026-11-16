//! The player's name and tag: asked once on a first interactive launch, and editable from the
//! OPTIONS page.
//!
//! One name is stored for every title (`settings.player`); a title's [`Profile`] says whether it
//! shows the name and whether it also uses a tag. Pulse plays its own `<TagInput>` cell row
//! (`pilot_editor::Session::pulse_tag_entry`); every other title, and a Pulse source that
//! cannot spell the text, gets this project's grid.
//!
//! [`Profile`]: oag_title::profile::Profile

use log::{debug, warn};

use oag_ui_screens::campaign::footer::NavigationLegend;
use oag_ui_screens::{prompt, tag_entry};

use crate::overlay::{Prompt, Purpose};
use crate::settings::player::{self, Ask};
use crate::stage::Stage;

use super::Session;

impl Session {
    /// Asks for what a first interactive launch of this title is missing, once per run.
    pub(crate) fn ask_player(&mut self) {
        let Some(shell) = &self.shell else {
            return;
        };
        // A wallpaper takes no input, so a prompt over it would stand there for good.
        if self.demo.cli == Some(true) {
            return;
        }
        let ask = player::what_to_ask(
            &shell.title.profile,
            &self.settings.player,
            self.interactive_launch,
            self.player_asked,
        );
        let purpose = match ask {
            Ask::Nothing => return,
            Ask::Name => Purpose::PlayerName,
            Ask::Tag => Purpose::PlayerTag,
        };
        self.player_asked = true;
        self.open_player_prompt(purpose);
    }

    /// The OPTIONS page's PLAYER NAME row.
    pub(crate) fn edit_player_name(&mut self) {
        self.open_player_prompt(Purpose::PlayerName);
    }

    /// The OPTIONS page's PLAYER TAG row.
    pub(crate) fn edit_player_tag(&mut self) {
        self.open_player_prompt(Purpose::PlayerTag);
    }

    fn open_player_prompt(&mut self, purpose: Purpose) {
        let Some(shell) = &self.shell else {
            return;
        };
        let profile = shell.title.profile;
        let (length, id, english, initial, keys) = match purpose {
            Purpose::PlayerName => (
                profile.name_length,
                "OAG_PLAYER_NAME_TITLE",
                "ENTER YOUR NAME",
                self.settings.player.name.clone(),
                player::NAME_KEYS,
            ),
            Purpose::PlayerTag => (
                profile.tag_length,
                "OAG_PLAYER_TAG_TITLE",
                "ENTER YOUR TAG",
                self.settings.player.tag.clone(),
                player::TAG_KEYS,
            ),
            _ => return,
        };
        let Some(length) = length else {
            return;
        };
        let limit = usize::from(length.value);
        let widget = if purpose == Purpose::PlayerName {
            "Name"
        } else {
            "Tag"
        };
        let title = self.say(id, english);
        let accept = self.say("OAG_KEYBOARD_ACCEPT", "OK");
        let entry_labels = tag_entry::Labels {
            title: title.clone(),
            confirm: accept.clone(),
            hint: self.say(
                "OAG_TAG_ENTRY_HINT",
                "LEFT/RIGHT CELL   UP/DOWN GLYPH   CROSS/START ACCEPT   CIRCLE CANCEL",
            ),
        };
        let accepts: fn(char) -> bool = if purpose == Purpose::PlayerName {
            player::allowed
        } else {
            player::allowed_in_tag
        };
        let prompt = match self.pulse_tag_entry(widget, &initial, entry_labels, accepts) {
            Ok(entry) => {
                debug!("player {widget}: using Pulse's own TagInput");
                Prompt::tagging(purpose, entry)
            }
            Err(reason) => {
                debug!("player {widget}: using the on-screen grid, not Pulse's TagInput - {reason}");
                let labels = prompt::Labels {
                    title,
                    delete: self.say("OAG_KEYBOARD_DELETE", "DEL"),
                    accept,
                    hint: self.say(
                        "OAG_KEYBOARD_HINT",
                        "CROSS TYPE   SQUARE DELETE   START ACCEPT   CIRCLE CANCEL",
                    ),
                };
                Prompt::typing(
                    purpose,
                    prompt::Keyboard::with_keys(labels, &initial, limit, keys),
                )
            }
        };
        if let Stage::Menu(stage) = &mut self.stage {
            stage.prompt = Some(prompt);
        }
    }

    fn say(&self, id: &str, english: &str) -> String {
        self.shell
            .as_ref()
            .and_then(|shell| shell.strings.get(id))
            .unwrap_or(english)
            .to_string()
    }

    /// A name or tag prompt was accepted: store it, and go on to the tag when this title uses
    /// one and none is stored. An empty answer changes nothing, so a stored name is never
    /// blanked by a prompt that was accepted without typing.
    pub(crate) fn player_entered(&mut self, purpose: &Purpose, text: &str) {
        match purpose {
            Purpose::PlayerName if self.settings.player.set_name(text) => {}
            Purpose::PlayerTag if self.settings.player.set_tag(text) => {}
            _ => return,
        }
        if let Err(e) = crate::settings::save(&self.settings) {
            warn!("cannot remember the player name: {e:#}");
        }
        if let Stage::Menu(stage) = &mut self.stage {
            if let Some(legend) = &mut stage.nav_legend {
                legend.set_tag(&self.settings.player.tag);
            }
            if let Some(campaign) = &mut stage.campaign {
                campaign.set_tag(&self.settings.player.tag);
            }
        }
        if *purpose == Purpose::PlayerName
            && self.shell.as_ref().is_some_and(|shell| {
                shell.title.profile.asks_for_tag() && self.settings.player.tag().is_none()
            })
        {
            self.open_player_prompt(Purpose::PlayerTag);
        }
    }
}

/// `legend` with the stored tag set where `title` uses one, for a stage about to draw it.
pub(crate) fn tagged(
    legend: Option<NavigationLegend>,
    player: &crate::settings::Player,
    title: &oag_title::Title,
) -> Option<NavigationLegend> {
    legend.map(|legend| legend.with_tag(player.footer_tag(&title.profile)))
}
