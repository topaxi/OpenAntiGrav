//! Ship Select once per seated person, on a split-screen race box.
//!
//! The join screen seats people ([`oag_input::lobby::Lobby`]); Track Select is
//! one pick for the race, any device driving it; then **Ship Select opens once
//! per seat, in seat order, and only that seat's device drives it** - seat 0 is
//! the player the settings belong to, and every later seat leaves the settings
//! alone and keeps its pick in [`Session::split_picks`] until the launch carries
//! them into `oag_raceplay::Options::seat_picks`.
//!
//! **Where this comes from.** Wipeout HD authors it: its `Team Selection
//! Player 1` leads to `Team Selection Player 2`, each titled by its own string
//! (`FE_SHIPSEL_P1`, `FE_SHIPSEL_P2`), and the second's back button is bound
//! to a pad (`InputPad`) in the file. Wipeout Pulse's PS2 disc authors a
//! different screen, both players at once on one page (`Team SelectionSplit`);
//! every other title authors neither, and this one-after-the-other flow is what
//! all of them get. See `docs/ui/selection-screens.md`.
//!
//! **Chosen, not measured:**
//!
//! - A seat opens on the team after the previous seat's, so a bare confirm from
//!   everyone races different teams. Two people may still take the same one.
//! - The pointer drives whichever seat's screen is open, whatever its device:
//!   only one screen is up at a time, so a click is unambiguous.
//! - Back on a later seat returns to the seat before; back on the first goes to
//!   Track Select, as it always did.
//! - A title that authors no `FE_SHIPSEL_P<n>` string reads its Ship Select
//!   title with `PLAYER <n>` after it.

use oag_gameplay::input::Input;
use oag_input::lobby::{Lobby, Press};
use oag_ui_screens::picker::{Entry, Kind, Layout};

use crate::session::Session;
use crate::stage::Stage;

/// One seat's finished pick: what the screen was on, to reopen it there, and
/// what the race is told.
#[derive(Debug, Clone)]
pub(crate) struct SeatChoice {
    /// The entry id the screen selected (a base team, not a combined variant).
    pub(crate) entry_id: String,
    /// The livery row's id (`""` is the baseline).
    pub(crate) livery: String,
    /// What the race build receives for this seat.
    pub(crate) pick: oag_raceplay::SeatPick,
}

/// The buttons `seat`'s device pressed this tick, as a menu input.
///
/// A device's presses are what the join screen already reads
/// (`Controls::device_presses`): the pad's own press events and the keyboard's
/// newly pressed bits, so a press shorter than a tick still counts and two pads
/// are told apart. A seat with no device in the lobby reads nothing.
pub(super) fn input<D: Copy + PartialEq>(
    lobby: Option<&Lobby<D>>,
    seat: usize,
    presses: &[Press<D>],
) -> Input {
    let device = lobby.and_then(|lobby| lobby.seats().get(seat)).copied();
    let mask = presses
        .iter()
        .filter(|press| Some(press.device) == device)
        .fold(0, |mask, press| mask | press.buttons);
    let mut input = Input::new();
    input.begin_frame(mask);
    input
}

impl Session {
    /// People seated in the race box now: `1` when it is an ordinary race.
    pub(super) fn split_count(&self) -> usize {
        if self.campaign_cell.is_some() {
            return 1;
        }
        self.split_lobby.as_ref().map_or(1, Lobby::count).max(1)
    }

    /// `(seat, seated)` for a Ship Select that belongs to a seat, `None` for an
    /// ordinary race's.
    pub(super) fn seat_of_picker(&self, seat: usize) -> Option<(usize, usize)> {
        let count = self.split_count();
        (count > 1).then_some((seat, count))
    }

    /// The team and livery a seat's screen opens on.
    ///
    /// Seat 0 is the settings' own (`None` livery: the caller's rule). A later
    /// seat reopens on what it picked before, or else on the team after the
    /// previous seat's.
    pub(super) fn seat_opening(&self, seat: usize, entries: &[Entry]) -> (String, Option<String>) {
        if seat == 0 {
            return (self.settings.race.team.clone(), None);
        }
        if let Some(choice) = self.split_picks.get(seat - 1) {
            return (choice.entry_id.clone(), Some(choice.livery.clone()));
        }
        let previous = match seat {
            1 => self.settings.race.team.as_str(),
            _ => self
                .split_picks
                .get(seat - 2)
                .map_or("", |choice| choice.entry_id.as_str()),
        };
        let next = entries
            .iter()
            .position(|entry| entry.id == previous)
            .map_or(0, |at| (at + 1) % entries.len().max(1));
        (
            entries
                .get(next)
                .map_or_else(String::new, |entry| entry.id.clone()),
            None,
        )
    }

    /// The two halves of the source's own `Team SelectionSplit`, when this race
    /// box has exactly two seats and the source authors one.
    pub(super) fn pair_layouts(&self) -> Option<[Layout; 2]> {
        if self.split_count() != 2 {
            return None;
        }
        self.shell.as_ref()?.ship_select_split.clone()
    }

    /// `layout` titled for the seat, when the screen is a seat's - and, on a
    /// shared page, told what to say under a seat that has confirmed.
    pub(super) fn seat_layout(&self, mut layout: Layout, seat: Option<(usize, usize)>) -> Layout {
        let (Some((index, _)), Some(shell)) = (seat, self.shell.as_ref()) else {
            return layout;
        };
        if self.pair_layouts().is_some() {
            // The authored page has one title and no per-seat one.
            layout.ready = shell.strings.get("OAG_PICK_READY").unwrap_or("READY").to_string();
            return layout;
        }
        let number = index + 1;
        layout.title = match shell.strings.get(&format!("FE_SHIPSEL_P{number}")) {
            Some(authored) => authored.to_string(),
            None => {
                let player = shell.strings.get("OAG_JOIN_PLAYER").unwrap_or("PLAYER");
                format!("{} - {player} {number}", layout.title)
            }
        };
        layout
    }

    /// Seat `index` confirmed: keep its pick, and open the next seat's screen or
    /// launch the race.
    pub(super) fn confirm_seat(&mut self, index: usize, count: usize) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(picker) = stage.picker.take() else {
            return;
        };
        let chosen = picker.model.selected().map(|entry| entry.id.clone());
        let livery = picker
            .model
            .variant()
            .map(|(id, _)| id.clone())
            .unwrap_or_default();
        let axis = picker.livery_axis;
        let fixed = picker.model.fixed_livery();
        if let Some(id) = &chosen {
            self.speak_name(Kind::Ship, id);
        }
        if index == 0 {
            super::livery::confirmed(&mut self.settings.race, fixed);
            self.split_picks.clear();
        } else if let Some(entry_id) = chosen {
            self.split_picks.truncate(index - 1);
            let pick = self.seat_pick(&entry_id, &livery, axis);
            self.split_picks.push(SeatChoice {
                entry_id,
                livery,
                pick,
            });
        }
        if index + 1 < count && self.open_ship_picker_for(index + 1) {
            return;
        }
        self.launch_from_settings();
    }

    /// A seat on the shared page confirmed (`partner` is the second seat): hold
    /// its pick, and launch once both are held.
    pub(super) fn confirm_shared(&mut self, partner: bool) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let held = if partner {
            stage.partner.as_deref_mut()
        } else {
            stage.picker.as_mut()
        };
        let Some(held) = held else {
            return;
        };
        if held.model.is_locked() {
            return;
        }
        held.model.lock();
        let chosen = held.model.selected().map(|entry| entry.id.clone());
        let livery = held
            .model
            .variant()
            .map(|(id, _)| id.clone())
            .unwrap_or_default();
        let axis = held.livery_axis;
        let fixed = held.model.fixed_livery();
        let both = stage.picker.as_ref().is_some_and(|p| p.model.is_locked())
            && stage
                .partner
                .as_deref()
                .is_some_and(|p| p.model.is_locked());
        if let Some(id) = &chosen {
            self.speak_name(Kind::Ship, id);
        }
        if partner {
            if let Some(entry_id) = chosen {
                let pick = self.seat_pick(&entry_id, &livery, axis);
                self.split_picks = vec![SeatChoice {
                    entry_id,
                    livery,
                    pick,
                }];
            }
        } else {
            super::livery::confirmed(&mut self.settings.race, fixed);
        }
        if both {
            if let Stage::Menu(stage) = &mut self.stage {
                stage.picker = None;
                stage.partner = None;
            }
            self.launch_from_settings();
        }
    }

    /// An unconfirmed seat leaves the shared page: both people go back to Track
    /// Select, as the first seat's back does on a page of its own. Run from the
    /// second seat directly, because the first seat's back handler would read the
    /// *first* seat's hold.
    pub(super) fn leave_shared(&mut self) {
        if let Stage::Menu(stage) = &mut self.stage {
            stage.picker = None;
            stage.partner = None;
        }
        self.split_picks.clear();
        self.campaign_difficulty.take();
        self.campaign_ai_skill_scale = None;
        self.abandon_tournament();
        self.open_track_picker();
    }

    /// Back on a seat that had confirmed on the shared page: take the confirm back.
    pub(super) fn unlock_shared(&mut self, partner: bool) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let held = if partner {
            stage.partner.as_deref_mut()
        } else {
            stage.picker.as_mut()
        };
        if let Some(held) = held {
            held.model.unlock();
        }
        if partner {
            self.split_picks.clear();
        }
    }

    /// An event of the second seat on the shared page.
    pub(crate) fn handle_partner(&mut self, event: oag_ui_screens::picker::Event) {
        use oag_ui_screens::picker::Event;
        self.play_navs([event.nav()]);
        match event {
            Event::Moved | Event::VariantChanged => {
                if let Stage::Menu(stage) = &mut self.stage
                    && let Some(partner) = stage.partner.as_deref_mut()
                {
                    partner.refresh_preview(&self.gpu);
                }
            }
            Event::Confirmed => self.confirm_shared(true),
            Event::Back => {
                let locked = matches!(&self.stage, Stage::Menu(stage)
                    if stage.partner.as_deref().is_some_and(|p| p.model.is_locked()));
                if locked {
                    self.unlock_shared(true);
                } else {
                    self.leave_shared();
                }
            }
        }
    }

    /// Back from a later seat: forget its pick and return to the seat before.
    pub(super) fn back_seat(&mut self, index: usize) {
        if let Stage::Menu(stage) = &mut self.stage {
            stage.picker = None;
        }
        self.split_picks.truncate(index.saturating_sub(1));
        self.open_ship_picker_for(index - 1);
    }

    /// What the race is told for a seat that picked `team` in `livery`: the same
    /// combination of team and variant the single-player launch makes
    /// ([`crate::session::menus::apply_race_team`]), on a copy of the settings
    /// so the first player's stay theirs.
    fn seat_pick(
        &self,
        team: &str,
        livery: &str,
        axis: crate::picker_stage::LiveryAxis,
    ) -> oag_raceplay::SeatPick {
        let Some(shell) = self.shell.as_ref() else {
            return oag_raceplay::SeatPick {
                team: team.to_string(),
                hull_variant: None,
                skin: None,
            };
        };
        let mut race = self.settings.race.clone();
        race.team = team.to_string();
        (race.variant, race.skin) = match axis {
            crate::picker_stage::LiveryAxis::Skin => (String::new(), livery.to_string()),
            crate::picker_stage::LiveryAxis::Variant => (livery.to_string(), String::new()),
        };
        let mut options = oag_raceplay::Options::default();
        for warning in crate::session::menus::team::apply_race_team(
            shell.title,
            &shell.teams,
            &race,
            &mut options,
        ) {
            log::warn!("{warning}");
        }
        oag_raceplay::SeatPick {
            team: options.team.unwrap_or_else(|| team.to_string()),
            hull_variant: options.hull_variant,
            skin: options.skin,
        }
    }

    /// Drops the split-screen race box: the lobby, the picks made in it and any
    /// the last launch carried.
    pub(crate) fn drop_split_lobby(&mut self) {
        self.split_lobby = None;
        self.split_picks.clear();
        if let Some(options) = self.race_options.as_mut() {
            options.seat_picks.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_gameplay::input::Button;
    use oag_input::lobby::Device;

    fn press(device: Device<u8>, button: Button) -> Press<u8> {
        Press {
            device,
            buttons: button.bit(),
        }
    }

    fn lobby() -> Lobby<u8> {
        let mut lobby = Lobby::new();
        lobby.join(Device::Pad(7));
        lobby.join(Device::Pad(9));
        lobby
    }

    /// Two pads press at once: each seat's screen hears its own pad and not the
    /// other's, which is what keeps one player's confirm off another's pick.
    #[test]
    fn a_seat_hears_only_its_own_device() {
        let presses = [
            press(Device::Pad(7), Button::Down),
            press(Device::Pad(9), Button::Cross),
        ];
        let lobby = lobby();
        let first = input(Some(&lobby), 0, &presses);
        assert!(first.is_pressed(Button::Down) && !first.is_pressed(Button::Cross));
        let second = input(Some(&lobby), 1, &presses);
        assert!(second.is_pressed(Button::Cross) && !second.is_pressed(Button::Down));
    }

    /// A device with no seat, the keyboard here, and a seat past the lobby read
    /// nothing at all.
    #[test]
    fn a_stranger_and_an_empty_seat_hear_nothing() {
        let presses = [press(Device::Keyboard, Button::Cross)];
        let lobby = lobby();
        assert!(!input(Some(&lobby), 0, &presses).is_pressed(Button::Cross));
        assert!(!input(Some(&lobby), 5, &presses).is_pressed(Button::Cross));
        assert!(!input::<u8>(None, 0, &presses).is_pressed(Button::Cross));
    }

    /// A seat a physical keyboard took on the join screen hears that keyboard's raw presses, which
    /// the session keeps feeding through Ship Select (`Session::device_key`), and not the
    /// anonymous keyboard's: the window's own key events are the same presses again once the
    /// keyboards are told apart. Fails if the seat's device stops matching its presses.
    #[test]
    fn a_named_keyboards_seat_hears_its_raw_presses_alone() {
        use oag_input::pad::KeyboardId;
        let (a, b) = (Device::Keys(KeyboardId(0)), Device::Keys(KeyboardId(1)));
        let mut lobby = Lobby::<u8>::new();
        lobby.join(a);
        lobby.join(b);
        let presses = [
            press(b, Button::Cross),
            press(Device::Keyboard, Button::Down),
        ];
        let first = input(Some(&lobby), 0, &presses);
        assert!(!first.is_pressed(Button::Cross) && !first.is_pressed(Button::Down));
        let second = input(Some(&lobby), 1, &presses);
        assert!(second.is_pressed(Button::Cross) && !second.is_pressed(Button::Down));
    }
}
