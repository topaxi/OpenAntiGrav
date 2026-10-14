//! Opens and drives the seat assignment screen, the step of LOCAL MULTIPLAYER between the
//! window layout and the RACE page (`session::multiplayer` has the flow it belongs to).
//!
//! **Ours, chosen, not measured.** The layout screen's START opens it when the layout has two or
//! more windows (the people are dealt across windows and there is a window to choose); with one
//! window, which is also every browser and phone, there is nothing to choose between and the
//! flow goes straight on with the people in join order. Each person moves their own marker with
//! their own device (`Controls::device_presses`, told apart by `Lobby::seat_of`), the mouse
//! picks a person up and puts them down on a pane, and START goes on once everybody has
//! confirmed. BACK returns to the layout screen with its choice as it was.
//!
//! **What START writes.** [`Session::pane_people`], who sits in each pane, which
//! `oag_game::split_screen::deal_windows` reads when a race is built (and again for a race
//! again and a RETURN TO MENU race, which skip this screen), and the profile's memory of it,
//! `[race] window_panes`, one device key per pane. A fresh screen opens on that memory where
//! the devices are still seated ([`oag_game::split_screen::remembered_panes`]). The grid order
//! is untouched: a person's craft is still the slot their join order gave them.

use oag_game::split_screen::{DeviceName, device_key, remembered_panes};
use oag_input::lobby::{Device, Lobby};
use oag_ui::menu::nav::Nav;
use oag_ui_screens::seat_assign::{Event, Labels, Pane, SeatAssign};

use crate::seat_assign_stage::SeatAssignStage;
use crate::stage::Stage;
use crate::window_layout_stage::WindowLayoutStage;

use super::Session;

impl Session {
    /// The layout screen's START with `layout` holding the windows and the people: opens the
    /// assignment screen over the menus when there is more than one window to choose between,
    /// else hands `layout` back for the flow to go on with the people in join order.
    pub(crate) fn open_seat_assign(&mut self, layout: WindowLayoutStage) -> Option<WindowLayoutStage> {
        let windows: Vec<Pane> = layout
            .screen()
            .windows()
            .iter()
            .map(|window| Pane {
                seats: window.seats,
                vertical: window.vertical,
            })
            .collect();
        if windows.len() < 2 || !matches!(self.stage, Stage::Menu(_)) {
            return Some(layout);
        }
        let Some(shell) = self.shell.as_ref() else {
            return Some(layout);
        };
        let labels = Labels::read(&shell.strings);
        let keys = self.device_keys(layout.join().lobby());
        let order = remembered_panes(&keys, &self.settings.race.window_panes);
        let screen = SeatAssign::new(labels, windows, Some(&order));
        // The windows already open are named for the assignment the screen starts on.
        self.pane_people = screen.pane_people();
        #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
        self.refresh_window_titles();
        if let Stage::Menu(stage) = &mut self.stage {
            stage.assign = Some(SeatAssignStage::new(layout, screen));
        }
        None
    }

    /// One key per seated person, in join order: what the profile remembers a device by.
    fn device_keys(&self, lobby: &Lobby) -> Vec<String> {
        let names: Vec<DeviceName> = lobby
            .seats()
            .iter()
            .map(|seat| match seat {
                Device::Keyboard => DeviceName::Keyboard(0),
                Device::Keys(id) => DeviceName::Keyboard(id.0 as usize),
                Device::Pad(id) => DeviceName::Pad(self.controls.pad_name(*id)),
            })
            .collect();
        (0..names.len()).map(|seat| device_key(&names, seat)).collect()
    }

    /// One tick of the open screen: every device's presses, each moving its own person's
    /// marker, then the pointer.
    pub(crate) fn tick_seat_assign(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let presses = self.controls.device_presses().to_vec();
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.assign.as_mut() else {
            return;
        };
        let lobby = open.layout().join().lobby().clone();
        let mut events = Vec::new();
        for press in presses {
            // A raw keyboard that is not seated by name is the anonymous keyboard a click seated.
            let seat = lobby.seat_of(press.device).or_else(|| match press.device {
                Device::Keys(_) => lobby.seat_of(Device::Keyboard),
                _ => None,
            });
            if let Some(person) = seat {
                events.extend(open.screen_mut().press(person, press.buttons));
            }
        }
        events.extend(open.screen_mut().pointer(
            pointer,
            &stage.skin,
            &oag_game::split_screen::seat_cells,
        ));
        for event in events {
            if !matches!(self.stage, Stage::Menu(ref stage) if stage.assign.is_some()) {
                break;
            }
            self.handle_seat_assign(event);
        }
    }

    /// What one event of the open screen does: a sound, and for START and BACK a change of
    /// screen.
    pub(crate) fn handle_seat_assign(&mut self, event: Event) {
        match event {
            Event::Moved | Event::Changed => self.play_navs(Some(Nav::Moved(None))),
            Event::Refused => self.play_navs(Some(Nav::Decline)),
            Event::Back => {
                if let Stage::Menu(stage) = &mut self.stage
                    && let Some(open) = stage.assign.take()
                {
                    stage.layout = Some(open.into_layout());
                }
                self.play_navs(Some(Nav::Decline));
            }
            Event::Start => self.finish_seat_assign(),
        }
    }

    /// START: the assignment is kept for the race and remembered, and the flow goes on
    /// ([`Session::continue_multiplayer`]) with the lobby the join screen made.
    fn finish_seat_assign(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.assign.take() else {
            return;
        };
        let lobby = open.layout().join().lobby().clone();
        let pane_people = open.screen().pane_people();
        let keys = self.device_keys(&lobby);
        self.settings.race.window_panes = pane_people.iter().map(|person| keys[*person].clone()).collect();
        if let Err(e) = crate::settings::save(&self.settings) {
            log::error!("could not save settings: {e:#}");
        }
        log::info!(
            "seat assignment: {}",
            pane_people
                .iter()
                .enumerate()
                .map(|(pane, person)| format!("pane {} = player {} ({})", pane + 1, person + 1, keys[*person]))
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.pane_people = pane_people;
        #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
        self.refresh_window_titles();
        self.play_navs(Some(Nav::Accept));
        self.continue_multiplayer(lobby);
    }
}
