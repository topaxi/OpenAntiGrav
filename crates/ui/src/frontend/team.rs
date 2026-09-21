//! `Team_Definition.xml`'s `team` screen: which team and craft slot a
//! campaign race flies.
//!
//! # What is authored, what is chosen, and what is not drawn at all
//!
//! **Authored** (confidence 92, read straight off the XML): the header
//! image/text and `ONL_TEAM_SELECT` title (drawn generically by
//! `Frontend::draw_screen_at`, nothing here); `TouchTeamGrid`'s own anchor
//! `(16, 94)`; `teamskin_touch`'s rect `(430, 15, 528, 261)` and its three
//! entries (`FE_SKIN_NORMAL`/`FE_SKIN_CHROME`/`FE_SKIN_BLUE`); `TeamInfo`'s
//! own `(16, 307)`, which this module's own grid is bounded above; the
//! `select_button` tick (`redirect="PreviousScreen"`).
//!
//! **`TouchTeamGrid` itself carries no per-team layout** - no `<Entry>` list,
//! no per-cell rect, unlike `teamskin_touch` beside it. `NEWGUI/teamgrid/`'s
//! own texture set (`Feisar.gxt`, `AG_SYS.gxt`, `Auricom.gxt`, `Pir-hana.gxt`,
//! `Qirex.gxt` - one per [`oag_2048::race::NATIVE_TEAMS`] entry - plus
//! `Fighter.gxt`/`Agility.gxt`/`Speed.gxt`/three `Proto*.gxt` variants, and
//! `lock.gxt`/`new_circle.gxt`) says the real grid is teams-by-craft-type
//! with locked/new cell states, populated by native code the same way
//! `FE3DCanvas_Add{HD,Fury}CampaignEventButtons` populates the campaign map's
//! own hotspots (`docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`),
//! unresolved this pass.
//!
//! **Drawn here as labelled tiles instead of those icons.** The icon files
//! are never named by any widget, so wiring them into this build's sprite
//! sheet (`oag_game::boot::sprites`, which only packs textures a widget's
//! own `src` names) is its own piece of work, left for whoever measures the
//! real grid's cell size and spacing next to it. Locked/new cell states are
//! not drawn either, on the same grounds
//! `handover/frontend/2048s-front-end-is-read-and-not-wired.md` already
//! gives the campaign map: every event is offered, there being no save.
//!
//! **Chosen, not measured**: the two rows' own geometry. One row of five
//! team tiles and one row of four craft-slot tiles, both left-aligned under
//! `TouchTeamGrid`'s `(16, 94)` anchor and both kept inside `x` 16..414 - the
//! width `teamskin_touch`'s own `x="430"` leaves free - and above `y="307"`,
//! where `TeamInfo` starts. The gap between tiles is this build's own; do
//! not read it as a measurement of the original's grid.
//!
//! **The `<Model name="ShipModel">` preview is not drawn at all.** Placing a
//! 3D craft on a 2D front-end screen has no renderer seam yet - the same gap
//! `oag_ui::picker::slideshow`'s own `Model` sits on ("read and carried, not
//! yet drawn from"), which is Wipeout Pulse's Track Creation screen hitting
//! the identical wall. `TeamInfo`'s own stat panel is not drawn either: it
//! is native-populated the same way `ProfileTouchMain`/`TouchTeamGrid` are.
//!
//! # Controls
//!
//! Left/Right cycles the highlighted team; Up/Down the highlighted craft
//! slot; Cross cycles the skin list (drawn, not wired to any setting - see
//! below); Circle leaves the way `select_button`'s own tick does
//! (`redirect="PreviousScreen"`). Pointer: a click on a team or craft tile
//! selects it outright, a click on a skin row cycles it, a click outside any
//! tile is a no-op the same way the touch grids elsewhere in this crate
//! treat one.
//!
//! # What feeds the campaign launch
//!
//! [`Frontend::team_choice`] is `None` until the player actually moves off
//! the screen's own starting point - the same "untouched means unchanged"
//! contract [`Frontend::chosen`] (the language picker) already carries -
//! so a player who never opens `Home` never has `settings.race.team`
//! overwritten from underneath a RACE BOX choice made a previous session.
//! Once it is `Some`, `oag_game`'s composition root writes
//! [`oag_2048::race::NATIVE_TEAMS`]`[team]` into `settings.race.team` and
//! [`oag_title::TeamVariant::suffix`] into `settings.race.variant` at the
//! same point it already applies the picked language - see
//! `crate::main::session::frame`'s own comment for why that is the one
//! place both can land before the menus first read either. From there the
//! existing [`oag_title::RaceDefaults::team_variants_for`]/`combine_variant`
//! machinery `Session::launch_race` and the RACE/REMIX pages already use
//! settles it into a real `race::Options::team`, unchanged by this module.
//! The skin list is drawn and cycled and reaches nothing: `teamskin_touch`'s
//! three values (`1`/`2`/`3`) do not obviously map onto
//! `race::Options::skin`'s own vocabulary (a `PI_ModelSkin` name such as
//! `Alternative`/`Eliminator`) without more reverse-engineering than this
//! pass did, so wiring it is left as a follow-up rather than guessed at.

use oag_2048::frontend::states as w2048;
use oag_2048::race::{NATIVE_TEAMS, SHIP_TYPE_LABELS, SHIP_TYPES, TEAM_VARIANTS};

use crate::pointer::{Pointer, contains};

use super::*;

/// Left edge of both rows, and the right bound neither may cross -
/// `teamskin_touch`'s own `x="430"`. Chosen - see the module docs.
const GRID_LEFT: f32 = 16.0;
const GRID_RIGHT: f32 = 414.0;
const GAP: f32 = 12.0;

/// The team row's own rect origin - `TouchTeamGrid`'s authored anchor.
const TEAM_ROW_Y: f32 = 94.0;
const TEAM_ROW_HEIGHT: f32 = 64.0;
/// Below the team row, above `TeamInfo`'s own `y="307"`.
const CRAFT_ROW_Y: f32 = 174.0;
const CRAFT_ROW_HEIGHT: f32 = 56.0;

const SKIN_LABELS: [&str; 3] = ["FE_SKIN_NORMAL", "FE_SKIN_CHROME", "FE_SKIN_BLUE"];
/// `teamskin_touch`'s own authored rect.
const SKIN_RECT: [f32; 4] = [430.0, 15.0, 528.0, 261.0];

fn tile_rect(row_y: f32, row_height: f32, count: usize, index: usize) -> [f32; 4] {
    let width = (GRID_RIGHT - GRID_LEFT - GAP * (count as f32 - 1.0)) / count as f32;
    [
        GRID_LEFT + index as f32 * (width + GAP),
        row_y,
        width,
        row_height,
    ]
}

impl Frontend {
    /// The team and craft slot the player has moved to, once they have moved
    /// off the screen's own starting point - see the module docs for why
    /// `None` is the contract, not a missing feature.
    #[must_use]
    pub fn team_choice(&self) -> Option<(&'static str, &'static str)> {
        let (team, craft) = self.touch.team_choice?;
        Some((NATIVE_TEAMS[team], TEAM_VARIANTS.variants[craft].suffix))
    }

    pub(super) fn update_team(&mut self, input: &mut Input) {
        let (mut team, mut craft) = self.touch.team_choice.unwrap_or((0, 2));
        let mut moved = false;
        if input.is_pressed(Button::Right) {
            input.consume_press(Button::Right);
            team = (team + 1) % NATIVE_TEAMS.len();
            moved = true;
        } else if input.is_pressed(Button::Left) {
            input.consume_press(Button::Left);
            team = (team + NATIVE_TEAMS.len() - 1) % NATIVE_TEAMS.len();
            moved = true;
        }
        if input.is_pressed(Button::Down) {
            input.consume_press(Button::Down);
            craft = (craft + 1) % SHIP_TYPES.len();
            moved = true;
        } else if input.is_pressed(Button::Up) {
            input.consume_press(Button::Up);
            craft = (craft + SHIP_TYPES.len() - 1) % SHIP_TYPES.len();
            moved = true;
        }
        if moved {
            self.touch.team_choice = Some((team, craft));
            self.notes.push(format!(
                "team: {} / {}",
                NATIVE_TEAMS[team], SHIP_TYPES[craft]
            ));
        }
        if input.is_pressed(Button::Cross) {
            input.consume_press(Button::Cross);
            self.touch.skin_index = (self.touch.skin_index + 1) % SKIN_LABELS.len();
        }
        if input.is_pressed(Button::Circle) {
            input.consume_press(Button::Circle);
            self.redirect_touch(w2048::TEAM, "select_button", "PreviousScreen");
        }
    }

    /// The pointer on the team screen - a click on a team/craft tile selects
    /// it, a click on a skin row cycles it. Returns whether the screen was on
    /// to take the click at all, the same contract [`Self::touch_pointer`]
    /// carries.
    pub(super) fn team_pointer(&mut self, pointer: &Pointer) -> bool {
        if !self.machine.is(w2048::TEAM) {
            return false;
        }
        let Some(at) = pointer.at else {
            return true;
        };
        if !pointer.clicked {
            return true;
        }
        if let Some(team) = (0..NATIVE_TEAMS.len()).find(|&i| {
            contains(
                tile_rect(TEAM_ROW_Y, TEAM_ROW_HEIGHT, NATIVE_TEAMS.len(), i),
                at,
            )
        }) {
            let craft = self.touch.team_choice.map_or(2, |(_, c)| c);
            self.touch.team_choice = Some((team, craft));
            self.notes
                .push(format!("team: {} tapped", NATIVE_TEAMS[team]));
        } else if let Some(craft) = (0..SHIP_TYPES.len()).find(|&i| {
            contains(
                tile_rect(CRAFT_ROW_Y, CRAFT_ROW_HEIGHT, SHIP_TYPES.len(), i),
                at,
            )
        }) {
            let team = self.touch.team_choice.map_or(0, |(t, _)| t);
            self.touch.team_choice = Some((team, craft));
            self.notes
                .push(format!("team: {} tapped", SHIP_TYPES[craft]));
        } else if contains(SKIN_RECT, at) {
            self.touch.skin_index = (self.touch.skin_index + 1) % SKIN_LABELS.len();
        }
        true
    }

    /// The two rows and the skin list, over `team`'s own header and the
    /// authored tiles `draw_screen_at` already drew.
    pub(super) fn draw_team(&self, out: &mut Vec<Draw>) {
        let (team, craft) = self.touch.team_choice.unwrap_or((0, 2));
        let cursor = self.global_colour("Orange2048");
        for (i, name) in NATIVE_TEAMS.iter().enumerate() {
            self.draw_labelled_tile(
                tile_rect(TEAM_ROW_Y, TEAM_ROW_HEIGHT, NATIVE_TEAMS.len(), i),
                self.strings.get_or_id(name),
                i == team,
                cursor,
                out,
            );
        }
        for (i, label) in SHIP_TYPE_LABELS.iter().enumerate() {
            self.draw_labelled_tile(
                tile_rect(CRAFT_ROW_Y, CRAFT_ROW_HEIGHT, SHIP_TYPES.len(), i),
                self.strings.get_or_id(label),
                i == craft,
                cursor,
                out,
            );
        }
        let [x, y, w, h] = SKIN_RECT;
        let row = h / SKIN_LABELS.len() as f32;
        for (i, label) in SKIN_LABELS.iter().enumerate() {
            self.draw_labelled_tile(
                [x, y + row * i as f32, w, row],
                self.strings.get_or_id(label),
                i == self.touch.skin_index,
                cursor,
                out,
            );
        }
    }

    fn draw_labelled_tile(
        &self,
        rect: [f32; 4],
        label: &str,
        selected: bool,
        cursor: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        let colour = self.global_colour("Blue2048");
        out.push(Draw::Fill {
            rect,
            color: colour,
        });
        if selected {
            self.draw_cursor_ring(rect, cursor, out);
        }
        out.push(Draw::Text {
            x: rect[0] + rect[2] * 0.5,
            y: rect[1] + rect[3] * 0.5 - 8.0,
            scale: 0.5,
            color: [1.0, 1.0, 1.0, 1.0],
            border: None,
            align: Align::Centre,
            text: label.to_string(),
            wrap_width: Some(rect[2] - 4.0),
        });
    }
}
