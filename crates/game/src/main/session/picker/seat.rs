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
pub(super) fn input(lobby: Option<&Lobby>, seat: usize, presses: &[Press]) -> Input {
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

    /// `layout` titled for the seat, when the screen is a seat's.
    pub(super) fn seat_layout(&self, mut layout: Layout, seat: Option<(usize, usize)>) -> Layout {
        let (Some((index, _)), Some(shell)) = (seat, self.shell.as_ref()) else {
            return layout;
        };
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
        if let Some(id) = &chosen {
            self.speak_name(Kind::Ship, id);
        }
        if index == 0 {
            self.settings.race.variant_chosen = true;
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
