//! Opens and drives the join screen, the first step of LOCAL MULTIPLAYER
//! (`session::multiplayer` has the flow it belongs to).
//!
//! **The join screen is this project's addition**, chosen, not measured: the PS2 seats two
//! pads in port order and HD and Omega's `FE_GUEST_JOIN` only asks player 2 to press START.
//! Each device that presses confirm (or START, as HD's text says) takes the next seat; the
//! devices seated last time this session are seated again if they are still connected
//! ([`Session::mp_devices`]); START (from a seated device, or a click) goes on to the
//! window layout and the rest of the flow, with the joined count carried to the race; back
//! leaves a seat and then the screen.
//!
//! **A row per keyboard** where the platform tells keyboards apart: X11 and Windows send a
//! raw key event naming the physical keyboard (`Controls::lobby_key`), and a second keyboard
//! takes a seat of its own ([`Device::Keys`]). Wayland and macOS name no keyboard, so there
//! every keyboard is the one anonymous [`Device::Keyboard`]; a Wayland compositor with a seat
//! per window seats its second keyboard by a press in that window after the layout screen
//! (`docs/rendering/multi-window.md`, "Who sits where").
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
    /// The screen, headed by LOCAL MULTIPLAYER, with the devices seated last time this
    /// session seated again where they are still there.
    pub(crate) fn open_join(&mut self) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let mut join = JoinStage::new(Labels::read(&shell.strings));
        let connected = self.controls.connected_pads();
        for device in &self.mp_devices {
            let present = match device {
                Device::Pad(id) => connected.contains(id),
                Device::Keyboard | Device::Keys(_) => true,
            };
            if present {
                join.lobby_mut().join(*device);
            }
        }
        self.controls.forget_lobby_keys();
        if let Stage::Menu(stage) = &mut self.stage {
            stage.join = Some(join);
        }
        self.sync_join_names();
    }

    /// One tick of the open screen: every device's presses, then the pointer.
    pub(crate) fn tick_join(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let presses = self.controls.device_presses().to_vec();
        let connected = self.controls.connected_pads();
        let told_apart = self.controls.keyboards_told_apart();
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.join.as_mut() else {
            return;
        };
        let named = open
            .lobby()
            .seats()
            .iter()
            .any(|seat| matches!(seat, Device::Keys(_)));
        let told_apart = told_apart || named;
        open.lobby_mut().retain_connected(&connected);
        let mut outcomes: Vec<Outcome> = Vec::new();
        for press in presses {
            // A keyboard that joined anonymously (a click, or a key before the platform named
            // any) turns out to be this one: it keeps its seat.
            if let Device::Keys(_) = press.device
                && open.lobby().seat_of(press.device).is_none()
                && open.lobby_mut().replace(Device::Keyboard, press.device)
            {
                continue;
            }
            outcomes.push(open.lobby_mut().press(press));
        }
        for event in open.screen_mut().pointer(pointer, &stage.skin) {
            outcomes.push(match event {
                Event::Moved => continue,
                // A mouse is not a keyboard: once the platform has told keyboards apart, a
                // click on the empty seat seats nobody (a keyboard press does).
                Event::Join if told_apart => Outcome::Nothing,
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
            // "KEYBOARD" alone while it is the only one told apart, "KEYBOARD 2" once there are more.
            Device::Keys(id) => {
                let keyboards = open
                    .lobby()
                    .seats()
                    .iter()
                    .filter(|seat| matches!(seat, Device::Keys(_)))
                    .count();
                if keyboards > 1 || id.0 > 0 {
                    format!("{} {}", labels.keyboard, id.0 + 1)
                } else {
                    labels.keyboard.clone()
                }
            }
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
                self.pane_people.clear();
                self.controls.forget_lobby_keys();
                self.play_navs(Some(Nav::Decline));
            }
            Outcome::Start => {
                let Stage::Menu(stage) = &mut self.stage else {
                    return;
                };
                // A race of one is not multiplayer: START waits for a second person.
                if stage.join.as_ref().is_some_and(|open| open.lobby().count() < 2) {
                    self.play_navs(Some(Nav::Decline));
                    return;
                }
                let Some(open) = stage.join.take() else {
                    return;
                };
                // The keyboards stay told apart (`Controls::lobby_key`) through the layout, the
                // RACE page and the seats' own Ship Select, which read a seat's device: until the
                // flow ends ([`Session::sync_multiplayer`]).
                self.mp_devices = open.lobby().seats().to_vec();
                self.play_navs(Some(Nav::Accept));
                // On a desktop the window layout screen comes next, and its START carries on
                // (`session::window_layout`); elsewhere the flow goes straight on.
                if let Some(open) = self.open_window_layout(open) {
                    self.split_cut = self.remembered_cut();
                    self.pane_people.clear();
                    self.continue_multiplayer(open.lobby().clone());
                }
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
        // Escape is the keyboard pressing back: the keyboard that sat last, else the anonymous one.
        let device = open
            .lobby()
            .seats()
            .iter()
            .rev()
            .find(|seat| matches!(seat, Device::Keyboard | Device::Keys(_)))
            .copied()
            .unwrap_or(Device::Keyboard);
        let outcome = open.lobby_mut().press(oag_input::lobby::Press {
            device,
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
