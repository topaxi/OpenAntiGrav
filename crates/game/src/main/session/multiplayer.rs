//! LOCAL MULTIPLAYER: the flow that replaces the per-title split-screen entries
//! (2026-10-14, the maintainer's plan; **ours, chosen, not measured**).
//!
//! 1. The **MULTIPLAYER** main-menu row opens the page whose LOCAL MULTIPLAYER row starts the
//!    flow; LAN and ONLINE are rows for what comes later ([`oag_ui::menu::Action::ComingLater`]).
//! 2. **Join** (`session::join`): each person takes a seat with their device; the devices
//!    seated earlier this session are seated again where they are still connected.
//! 3. **Layout** (`session::window_layout`): windows, monitors, the cut, fullscreen.
//!    Remembered on disk.
//! 4. **Seat assignment** (next lane, `mp-seat-assign`): [`Session::assign_seats`] is where it
//!    goes, between the layout and the settings. It hands the lobby back as it got it today.
//! 5. **Race settings**: the RACE page, remembered. The mode row marks a mode that takes fewer
//!    people than are seated ([`Session::resupply_race_modes`], `Mode::max_people`).
//! 6. **Track Select**, then **Ship Select** per seat, then the race (`session::picker`).
//!
//! **Quick paths.** A race again from the end screen keeps the seats and the windows and skips
//! 2 to 4 (the devices stay seated, `Session::seat_the_windows`). RETURN TO MENU after a
//! multiplayer race lands on step 5 with everyone still seated
//! ([`Session::reenter_multiplayer`]): change the track and race again. Leaving through BACK
//! to the MULTIPLAYER page, or any other page, ends the flow ([`Session::sync_multiplayer`]).
//! A fresh entry from the main menu pre-fills every step, so it is OK, OK, OK.

use oag_input::lobby::{Device, Lobby};

use crate::stage::Stage;

use super::Session;

impl Session {
    /// LOCAL MULTIPLAYER fired on the MULTIPLAYER page: the first step is the join screen.
    pub(crate) fn open_multiplayer_local(&mut self) {
        self.open_join();
    }

    /// The step after join and layout: seat assignment ([`Self::assign_seats`]), then the RACE
    /// page, with `lobby` carried to the race. A lobby of fewer than two is not multiplayer
    /// and goes no further.
    pub(crate) fn continue_multiplayer(&mut self, lobby: Lobby) {
        if lobby.count() < 2 {
            return;
        }
        if !matches!(self.stage, Stage::Menu(_)) {
            return;
        }
        self.remember_devices(&lobby);
        let lobby = self.assign_seats(lobby);
        self.split_lobby = Some(lobby);
        self.open_multiplayer_settings();
    }

    /// **The hook for seat assignment (`mp-seat-assign`)**: each person moving their own
    /// marker onto the pane they want, between the layout and the settings. Nothing yet: the
    /// people sit in the order they joined, slot `i` in seat `i`, dealt across the windows in
    /// order (`Session::seat_the_windows`, `split_screen::deal_windows`).
    fn assign_seats(&mut self, lobby: Lobby) -> Lobby {
        lobby
    }

    /// Keeps `lobby`'s devices for the next time the join screen opens this session. In memory
    /// only: a restart joins again.
    fn remember_devices(&mut self, lobby: &Lobby) {
        self.mp_devices = lobby.seats().to_vec();
    }

    /// The RACE page as the flow's settings step: pushed over the MULTIPLAYER page, so BACK
    /// from it lands there, with the mode row marked for the people seated.
    fn open_multiplayer_settings(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        if !stage.menu.push("race") {
            log::warn!("multiplayer: the menu has no RACE page to open");
            return;
        }
        self.multiplayer = true;
        self.resupply_race_modes();
    }

    /// The cut a layout screen that is not shown leaves (the web and the phones, which get
    /// the one surface the platform makes): the first window of the remembered layout, else
    /// the horizontal cut every split defaults to.
    pub(crate) fn remembered_cut(&self) -> oag_render::viewport::Cut {
        oag_game::split_screen::parse_window_seats(
            &self.settings.race.window_seats,
            oag_render::viewport::Cut::Horizontal,
        )
        .ok()
        .and_then(|layout| layout.first().map(|window| window.cut))
        .unwrap_or_default()
    }

    /// The MODE row's options with every mode that takes fewer people than are seated marked
    /// unusable ([`oag_ui::menu::cap_modes`]). Head to Head is the one today (`Mode::max_people`).
    pub(crate) fn resupply_race_modes(&mut self) {
        let people = self.split_lobby.as_ref().map_or(1, Lobby::count);
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let reason = shell
            .strings
            .get("OAG_MP_MODE_CAPPED")
            .unwrap_or("For %d people at most")
            .to_string();
        let modes = oag_ui::menu::cap_modes(&shell.modes, people, |most| {
            reason.replace("%d", &most.to_string())
        });
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.supply(oag_ui::menu::ValueSource::RaceModes, &modes);
        }
    }

    /// Ends the flow once the menu has left the RACE page: it is on some other page with none of
    /// the flow's own screens (join, layout, Track and Ship Select) open over it. The lobby the
    /// join screen made goes with it, and the MODE row is whole again.
    pub(crate) fn sync_multiplayer(&mut self) {
        if !self.multiplayer {
            return;
        }
        let Stage::Menu(stage) = &self.stage else {
            return;
        };
        if has_left_the_flow(
            &stage.menu.page().id,
            stage.picker.is_some(),
            stage.join.is_some() || stage.layout.is_some(),
        ) {
            self.multiplayer = false;
            self.controls.forget_lobby_keys();
            self.drop_split_lobby();
            self.resupply_race_modes();
        }
    }

    /// Back on the menus after a multiplayer race that was not left through the campaign: the
    /// RACE page again with the same people seated, so changing the track and racing again is
    /// one START and no join, layout or seat assignment. The devices are those the join screen
    /// last seated, if they are still there; fewer than two ends it as a plain menu.
    pub(crate) fn reenter_multiplayer(&mut self) {
        let connected = self.controls.connected_pads();
        let mut lobby = Lobby::new();
        for device in &self.mp_devices {
            let present = match device {
                Device::Pad(id) => connected.contains(id),
                Device::Keyboard | Device::Keys(_) => true,
            };
            if present {
                lobby.join(*device);
            }
        }
        if lobby.count() < 2 {
            return;
        }
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        if !(stage.menu.push("multiplayer") && stage.menu.push("race")) {
            return;
        }
        self.split_lobby = Some(lobby);
        self.multiplayer = true;
        self.resupply_race_modes();
    }
}

/// Whether the menu has left the multiplayer flow's RACE page: it is on some page but RACE, with
/// none of the flow's own screens (Track and Ship Select, the join and layout screens) open
/// over it.
fn has_left_the_flow(page: &str, picker_open: bool, setup_open: bool) -> bool {
    !picker_open && !setup_open && page != "race"
}

#[cfg(test)]
mod tests {
    use super::has_left_the_flow;

    #[test]
    fn the_flow_ends_off_the_race_page_and_not_under_its_own_screens() {
        assert!(!has_left_the_flow("race", false, false), "on the RACE page");
        assert!(
            !has_left_the_flow("multiplayer", true, false),
            "Track Select is open over it"
        );
        assert!(
            !has_left_the_flow("multiplayer", false, true),
            "the join screen is open over it"
        );
        assert!(
            has_left_the_flow("multiplayer", false, false),
            "BACK to the MULTIPLAYER page ends it"
        );
    }
}
