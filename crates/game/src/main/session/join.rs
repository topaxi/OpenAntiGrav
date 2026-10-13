//! Opens and drives the split-screen join screen over the menus.
//!
//! **The entry is `oag-extras`' SPLIT SCREEN row, chosen, not measured** - see
//! that row's comment. Each device that presses confirm takes the next seat;
//! START (from a seated device, or a click) goes on to the race box's own Track
//! and Ship Select exactly as the RACE page's START does, with the joined count
//! carried to the race; back leaves a seat and then the screen.
//!
//! The lobby rules are `oag_input::lobby`'s and the screen's drawing and pointer
//! are `oag_ui_screens::join`'s. What lives here is only the join between them
//! and the session: whose press it was, which sound it makes, and what Start
//! hands the race (`Session::split_lobby`, which the race build drains).

use oag_input::lobby::{Device, Outcome};
use oag_ui::menu::nav::Nav;
use oag_ui_screens::join::{Event, Labels};

use crate::join_stage::JoinStage;
use crate::stage::Stage;

use super::Session;

impl Session {
    /// The head count the race box's `NUMBER OF PLAYERS` row holds
    /// ([`oag_game::split_screen::authored_people`]).
    pub(crate) fn authored_people(&self) -> usize {
        self.shell.as_ref().map_or(1, |shell| {
            oag_game::split_screen::authored_people(
                shell.title,
                &shell.player_counts,
                &self.settings.race.players,
            )
        })
    }

    /// The cut the race box's `SPLIT SCREEN` row holds
    /// ([`oag_game::split_screen::authored_cut`]).
    fn authored_cut(&self) -> oag_render::viewport::Cut {
        self.shell.as_ref().map_or(Default::default(), |shell| {
            oag_game::split_screen::authored_cut(&shell.split_cuts, &self.settings.race.split_cut)
        })
    }

    /// The race box's START with the `NUMBER OF PLAYERS` row on two: the join screen of
    /// exactly that many seats first, which only claims the devices, and then Track Select
    /// and each seat's Ship Select as the generic entry does. Returns whether it opened.
    pub(crate) fn open_authored_split(&mut self) -> bool {
        let people = self.authored_people();
        if people < 2 {
            return false;
        }
        self.open_join(Some(people));
        true
    }

    /// Opens the screen with nobody seated: the generic SPLIT SCREEN row's, up to the
    /// grid's eight.
    pub(crate) fn open_split_screen(&mut self) {
        self.open_join(None);
    }

    /// The screen, for `people` seats all of which must be taken, or the grid's.
    fn open_join(&mut self, people: Option<usize>) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let say = |id: &str, english: &str| shell.strings.get(id).unwrap_or(english).to_string();
        let labels = Labels {
            title: say("OAG_MENU_SPLIT_SCREEN", "SPLIT SCREEN"),
            hint: say("OAG_JOIN_HINT", "EACH PLAYER PRESSES A BUTTON TO JOIN"),
            player: say("OAG_JOIN_PLAYER", "PLAYER"),
            empty: say("OAG_JOIN_EMPTY", "PRESS A BUTTON"),
            keyboard: say("OAG_JOIN_KEYBOARD", "KEYBOARD"),
            pad: say("OAG_JOIN_PAD", "GAMEPAD"),
            start: say("OAG_JOIN_START", "START"),
            back: say("OAG_MENU_BACK", "BACK"),
        };
        if let Stage::Menu(stage) = &mut self.stage {
            stage.join = Some(match people {
                Some(people) => JoinStage::for_people(labels, people),
                None => JoinStage::new(labels),
            });
        }
    }

    /// One tick of the open screen: every device's presses, then the pointer.
    pub(crate) fn tick_join(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let presses = self.controls.device_presses().to_vec();
        let connected = self.controls.connected_pads();
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.join.as_mut() else {
            return;
        };
        open.lobby_mut().retain_connected(&connected);
        let mut outcomes: Vec<Outcome> = presses
            .into_iter()
            .map(|press| open.lobby_mut().press(press))
            .collect();
        for event in open.screen_mut().pointer(pointer, &stage.skin) {
            outcomes.push(match event {
                Event::Moved => continue,
                Event::Join => open.lobby_mut().join(Device::Keyboard),
                Event::Leave(seat) => open.lobby_mut().leave(seat),
                Event::Start => Outcome::Start,
                Event::Back => Outcome::Back,
            });
        }
        self.sync_join_names();
        for outcome in outcomes {
            if !matches!(self.stage, Stage::Menu(ref stage) if stage.join.is_some()) {
                break;
            }
            self.handle_join(outcome);
        }
    }

    /// Tells the screen who sits where, by the names their devices go by.
    fn sync_join_names(&mut self) {
        let Stage::Menu(stage) = &self.stage else {
            return;
        };
        let Some(open) = stage.join.as_ref() else {
            return;
        };
        let names: Vec<String> = open
            .lobby()
            .seats()
            .iter()
            .map(|seat| self.seat_name(*seat))
            .collect();
        if let Stage::Menu(stage) = &mut self.stage
            && let Some(open) = stage.join.as_mut()
        {
            open.screen_mut().set_seats(names);
        }
    }

    /// What the screen calls a seat's device.
    fn seat_name(&self, seat: Device) -> String {
        let Stage::Menu(stage) = &self.stage else {
            return String::new();
        };
        let Some(open) = stage.join.as_ref() else {
            return String::new();
        };
        let labels = open.screen().labels();
        match seat {
            Device::Keyboard => labels.keyboard.clone(),
            Device::Pad(id) => self
                .controls
                .pad_name(id)
                .map_or_else(|| labels.pad.clone(), |name| name.to_uppercase()),
        }
    }

    /// What one outcome of the lobby does: a sound, and for START and BACK a
    /// change of screen.
    pub(crate) fn handle_join(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Joined(_) | Outcome::Left(_) => {
                let nav = if matches!(outcome, Outcome::Joined(_)) {
                    Nav::Accept
                } else {
                    Nav::Decline
                };
                self.play_navs(Some(nav));
            }
            Outcome::Full => self.play_navs(Some(Nav::Decline)),
            Outcome::Back => {
                if let Stage::Menu(stage) = &mut self.stage {
                    stage.join = None;
                }
                self.play_navs(Some(Nav::Decline));
            }
            Outcome::Start => {
                let Stage::Menu(stage) = &mut self.stage else {
                    return;
                };
                // The race box authored the head count: START waits for everyone.
                if stage.join.as_ref().is_some_and(|open| !open.screen().can_start()) {
                    self.play_navs(Some(Nav::Decline));
                    return;
                }
                let Some(open) = stage.join.take() else {
                    return;
                };
                if self.authored_people() >= 2 {
                    self.split_cut = self.authored_cut();
                }
                self.play_navs(Some(Nav::Accept));
                if open.lobby().count() > 1 {
                    self.split_lobby = Some(open.lobby().clone());
                }
                self.start_race_box();
            }
            Outcome::Nothing => {}
        }
    }

    /// Escape on the open screen: the keyboard pressing back, so it gives up
    /// its seat if it has one and leaves the screen if it has none.
    pub(crate) fn escape_join(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.join.as_mut() else {
            return;
        };
        let outcome = open.lobby_mut().press(oag_input::lobby::Press {
            device: Device::Keyboard,
            buttons: oag_gameplay::input::Button::Circle.bit(),
        });
        self.sync_join_names();
        self.handle_join(outcome);
    }

    /// Back on the menus after a split-screen race: one cursor, any device
    /// drives it. Not while a race is parked behind the pause menu - resuming
    /// it needs the seats it left.
    pub(crate) fn unseat_after_split_race(&mut self) {
        if self.controls.assignment().humans().is_some() && self.suspended_race.is_none() {
            self.controls.merge_devices();
        }
    }
}
