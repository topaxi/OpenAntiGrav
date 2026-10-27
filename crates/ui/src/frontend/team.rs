//! `Team_Definition.xml`'s `team` screen: which team and craft slot a
//! campaign race flies.
//!
//! # The grid is `TouchTeamGrid_Item.cpp`'s, read in Ghidra
//!
//! Evidence page: `docs/ghidra/functions/vita-2048-eu-v104/team-grid.md`.
//!
//! The XML authors only `TouchTeamGrid`'s anchor `(16, 94)`; the grid is
//! native code (`/2048/eboot-vita-2048-eu-v104.elf`, vtable `0x8150edc8`).
//! What is read off it (confidence 85, decompiled, not a trace):
//!
//! - **Layout.** The constructor `FUN_813d0254` stores a 5 (teams) by 4
//!   (craft) grid, cell `80 x 65` ([`CELL_W`], [`CELL_H`]), pitch `+2`
//!   ([`PITCH_X`], [`PITCH_Y`]). The update `FUN_8110772e` hit-tests the
//!   cells from `y = anchor.y + 64`, so the strip of five team badges
//!   sits in `anchor.y .. anchor.y + 64` and row 0 starts at `158`.
//! - **Order.** Columns are the loader's own (`FUN_81105710`): Feisar,
//!   AG Systems, Qirex, Auricom, Pir-hana ([`TEAM_COLUMNS`]); rows are
//!   `speed`, `combat`, `agility`, `prototype` (the string table beside
//!   it, [`ROW_CRAFT`]). `NATIVE_TEAMS` stays alphabetical: races use it.
//! - **Draw.** `FUN_8110700e` (the 2048 half of the draw): a cell is
//!   `Blue2048`, `Orange2048` where the cursor is, the same Blue at
//!   alpha `0x80` when locked; the craft icon (`teamgrid/*.gtf`) is
//!   centred in it tinted `White2048` (alpha `0x80` when locked); a locked
//!   cell also draws `lock.gtf` 32x32 at `(+74, +38)` from the cell's
//!   top-left, centred on the icon's own size. The badge is the team
//!   texture centred in its column.
//! - **Prototype row.** The fourth row's icon depends on the column:
//!   AG Systems `ProtoAgility`, Qirex and Auricom `ProtoCombat`, the
//!   others `ProtoSpeed` ([`craft_icon`]).
//! - **Start.** The cursor starts on column 0, row 0: Feisar Speed
//!   (the constructor), unless the `Team2048`/`PlayerLivery2048` globals
//!   name another.
//!
//! **Not drawn, and why.**
//!
//! - **Padlocks.** A cell's lock flag is `FUN_812d16f6` over the ship list
//!   `DAT_81967280`: unlocked when the ship record's `+0x3c` object's
//!   `+0x2d8` exceeds 2, or its `+0x40` rank minus one is below the save's
//!   unlock count (`FUN_810133da`, `DAT_81545468 + 0x3257c`). This build
//!   has no save and no ship-record source for that rank (the
//!   `<FE>` and `<Unlock>` data live in the handling files and SP.xml, not
//!   in a table this crate reads), so every cell is drawn unlocked and
//!   [`Frontend::team_cell_locked`] is the one place the answer will come
//!   from. The padlock draw path is written from the original's draw
//!   and not yet reachable.
//! - **The `NEW` badge** (`new_circle.gtf` plus `ONL_CON_NEW`, shown while
//!   a cell is unlocked and unseen): needs the same save.
//! - **The skin list.** `teamskin_touch` authors `Colour="0x00000000"` and
//!   `ArrowTouchOnly`; the original frame shows no list at all, so none is
//!   drawn (the labelled `FE_SKIN_*` stand-in is gone). Its three values
//!   still reach no setting.
//! - **The `<Model name="ShipModel">` hull preview.** No 2D front-end to
//!   3D mesh seam exists for 2048 yet; see the handover thread.
//! - **The statistics table and the tutorial bubble** (`TeamInfo`,
//!   `FE_SHIP_STATS_TUT`): open, see the handover thread.
//!
//! # Controls
//!
//! Left/Right moves the cursor along the columns; Up/Down along the rows;
//! Circle leaves the way `select_button`'s own tick does
//! (`redirect="PreviousScreen"`). Pointer: a click on a cell selects it outright; a click outside is a no-op the same way the
//! touch grids elsewhere in this crate treat one.
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
//! The skin list is not drawn and reaches nothing: `teamskin_touch`'s three
//! values (`1`/`2`/`3`) do not obviously map onto `race::Options::skin`'s
//! own vocabulary without more reverse-engineering, so wiring it is left as
//! a follow-up rather than guessed at.

use oag_2048::frontend::states as w2048;
use oag_2048::race::{NATIVE_TEAMS, SHIP_TYPES, TEAM_VARIANTS};

use crate::pointer::{Pointer, contains};

use super::*;

/// The grid's anchor: `TouchTeamGrid`'s authored `x`/`y`.
const GRID_X: f32 = 16.0;
const GRID_Y: f32 = 94.0;
/// The badge strip's height: the update's hit-test starts row 0 at
/// `anchor.y + 64`.
const BADGE_H: f32 = 64.0;
/// A cell and the pitch between cells, from the constructor's own floats.
const CELL_W: f32 = 80.0;
const CELL_H: f32 = 65.0;
const PITCH_X: f32 = CELL_W + 2.0;
const PITCH_Y: f32 = CELL_H + 2.0;
const ROWS: usize = 4;

/// The columns, left to right, as `NATIVE_TEAMS` names them: the order
/// `FUN_81105710` loads the badge textures in.
const TEAM_COLUMNS: [&str; 5] = [
    "Feisar2048",
    "AG_Systems2048",
    "Qirex2048",
    "Auricom2048",
    "Piranha2048",
];

/// Each column's badge, in [`TEAM_COLUMNS`]' order.
const TEAM_BADGES: [&str; 5] = [
    r"Data\FE\NewImages\teamgrid\Feisar.gtf",
    r"Data\FE\NewImages\teamgrid\AG_SYS.gtf",
    r"Data\FE\NewImages\teamgrid\Qirex.gtf",
    r"Data\FE\NewImages\teamgrid\Auricom.gtf",
    r"Data\FE\NewImages\teamgrid\Pir-Hana.gtf",
];

/// The six craft icons, in the order the loader stores them.
const CRAFT_ICONS: [&str; 6] = [
    r"Data\FE\NewImages\teamgrid\Speed.gtf",
    r"Data\FE\NewImages\teamgrid\Fighter.gtf",
    r"Data\FE\NewImages\teamgrid\Agility.gtf",
    r"Data\FE\NewImages\teamgrid\ProtoSpeed.gtf",
    r"Data\FE\NewImages\teamgrid\ProtoCombat.gtf",
    r"Data\FE\NewImages\teamgrid\ProtoAgility.gtf",
];

const LOCK_ICON: &str = r"Data\FE\NewImages\teamgrid\lock.gtf";

/// A row's craft, as an index into `SHIP_TYPES` (fighter, agility, speed,
/// prototype): the rows read speed, combat, agility, prototype.
const ROW_CRAFT: [usize; ROWS] = [2, 0, 1, 3];

/// Every texture the grid draws and no widget names, for the sprite
/// sheet's `extra` list - see `oag_game::boot`.
#[must_use]
pub fn team_grid_textures() -> Vec<&'static str> {
    TEAM_BADGES
        .iter()
        .chain(CRAFT_ICONS.iter())
        .copied()
        .chain([LOCK_ICON])
        .collect()
}

/// The icon in `row` of `column`: the prototype row's depends on the team.
fn craft_icon(column: usize, row: usize) -> &'static str {
    let index = match (row, column) {
        (0, _) => 0,
        (1, _) => 1,
        (2, _) => 2,
        (_, 1) => 5,
        (_, 2 | 3) => 4,
        _ => 3,
    };
    CRAFT_ICONS[index]
}

fn column_of(team: usize) -> usize {
    TEAM_COLUMNS
        .iter()
        .position(|name| *name == NATIVE_TEAMS[team])
        .unwrap_or(0)
}

fn team_of(column: usize) -> usize {
    NATIVE_TEAMS
        .iter()
        .position(|name| *name == TEAM_COLUMNS[column])
        .unwrap_or(0)
}

fn row_of(craft: usize) -> usize {
    ROW_CRAFT.iter().position(|c| *c == craft).unwrap_or(0)
}

/// Where the cursor starts: the first column, the first row.
fn start() -> (usize, usize) {
    (team_of(0), ROW_CRAFT[0])
}

fn cell_rect(column: usize, row: usize) -> [f32; 4] {
    [
        GRID_X + column as f32 * PITCH_X,
        GRID_Y + BADGE_H + row as f32 * PITCH_Y,
        CELL_W,
        CELL_H,
    ]
}

fn badge_rect(column: usize) -> [f32; 4] {
    [GRID_X + column as f32 * PITCH_X, GRID_Y, CELL_W, BADGE_H]
}

/// A cell's hit area: the original tests `83 x 68` on the 82x67 pitch
/// (`FUN_8110772e`), so the cells tile with no dead strip and the
/// later cell wins the one-unit overlap, as the update's loop does.
fn hit_rect(column: usize, row: usize) -> [f32; 4] {
    let [x, y, ..] = cell_rect(column, row);
    [x, y, PITCH_X + 1.0, PITCH_Y + 1.0]
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

    /// Whether the cell is locked. Always `false` for now: the original
    /// reads a save this build has none of - see the module docs.
    #[must_use]
    pub fn team_cell_locked(&self, _team: usize, _craft: usize) -> bool {
        false
    }

    pub(super) fn update_team(&mut self, input: &mut Input) {
        let (team, craft) = self.touch.team_choice.unwrap_or_else(start);
        let (mut column, mut row) = (column_of(team), row_of(craft));
        let mut moved = false;
        if input.is_pressed(Button::Right) {
            input.consume_press(Button::Right);
            column = (column + 1) % TEAM_COLUMNS.len();
            moved = true;
        } else if input.is_pressed(Button::Left) {
            input.consume_press(Button::Left);
            column = (column + TEAM_COLUMNS.len() - 1) % TEAM_COLUMNS.len();
            moved = true;
        }
        if input.is_pressed(Button::Down) {
            input.consume_press(Button::Down);
            row = (row + 1) % ROWS;
            moved = true;
        } else if input.is_pressed(Button::Up) {
            input.consume_press(Button::Up);
            row = (row + ROWS - 1) % ROWS;
            moved = true;
        }
        if moved && !self.team_cell_locked(team_of(column), ROW_CRAFT[row]) {
            let (team, craft) = (team_of(column), ROW_CRAFT[row]);
            self.touch.team_choice = Some((team, craft));
            self.notes.push(format!(
                "team: {} / {}",
                NATIVE_TEAMS[team], SHIP_TYPES[craft]
            ));
        }
        if input.is_pressed(Button::Circle) {
            input.consume_press(Button::Circle);
            self.redirect_touch(w2048::TEAM, "select_button", "PreviousScreen");
        }
    }

    /// The pointer on the team screen - a click on a cell selects its team
    /// and craft. Returns whether the
    /// screen was on to take the click at all, the same contract
    /// [`Self::touch_pointer`] carries.
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
        let cell = (0..TEAM_COLUMNS.len())
            .flat_map(|column| (0..ROWS).map(move |row| (column, row)))
            .filter(|&(column, row)| contains(hit_rect(column, row), at))
            .last();
        if let Some((column, row)) = cell {
            let (team, craft) = (team_of(column), ROW_CRAFT[row]);
            // The original ignores a tap on a locked cell.
            if !self.team_cell_locked(team, craft) {
                self.touch.team_choice = Some((team, craft));
                self.notes
                    .push(format!("team: {} tapped", NATIVE_TEAMS[team]));
                self.notes
                    .push(format!("team: {} tapped", SHIP_TYPES[craft]));
            }
        }
        true
    }

    /// The badge strip and the grid, over `team`'s own header and the
    /// authored tiles `draw_screen_at` already drew.
    pub(super) fn draw_team(&self, out: &mut Vec<Draw>) {
        let (team, craft) = self.touch.team_choice.unwrap_or_else(start);
        let (cursor_column, cursor_row) = (column_of(team), row_of(craft));
        let blue = self.global_colour("Blue2048");
        if let Some(screen) = self.screens.by_name(w2048::TEAM) {
            self.underlay_header_blocks(screen, blue, out);
        }
        let orange = self.global_colour("Orange2048");
        let white = self.global_colour("White2048");
        let half = |colour: [f32; 4]| [colour[0], colour[1], colour[2], colour[3] * 0.5];
        for (column, badge) in TEAM_BADGES.iter().enumerate() {
            if let Some(placed) = self.placed(badge) {
                out.push(Self::sprite_at(
                    centred(badge_rect(column), placed),
                    placed,
                    [1.0; 4],
                ));
            }
        }
        for column in 0..TEAM_COLUMNS.len() {
            for (row, &row_craft) in ROW_CRAFT.iter().enumerate() {
                let rect = cell_rect(column, row);
                let locked = self.team_cell_locked(team_of(column), row_craft);
                let fill = if (column, row) == (cursor_column, cursor_row) {
                    orange
                } else if locked {
                    half(blue)
                } else {
                    blue
                };
                out.push(Draw::Fill { rect, color: fill });
                let tint = if locked { half(white) } else { white };
                if let Some(placed) = self.placed(craft_icon(column, row)) {
                    out.push(Self::sprite_at(centred(rect, placed), placed, tint));
                }
                if locked
                    && let Some(placed) = self.placed(LOCK_ICON)
                    && let Some(icon) = self.placed(craft_icon(column, row))
                {
                    // The original centres the lock on the craft icon's own
                    // size and draws it 32x32 (`FUN_8110700e`).
                    out.push(Self::sprite_at(
                        [
                            rect[0] + 74.0 - icon.width as f32 * 0.5,
                            rect[1] + 38.0 - icon.height as f32 * 0.5,
                            32.0,
                            32.0,
                        ],
                        placed,
                        tint,
                    ));
                }
            }
        }
        // The confirm tick is the one authored button with an icon; `ar_view`
        // and the two `replay_unlock` hotspots are not drawn in the original
        // frame (the latter author `Colour="0x00000000"`).
        for tile in self
            .tiles(w2048::TEAM)
            .iter()
            .filter(|tile| tile.src.is_some())
        {
            self.draw_tile(tile, blue, false, orange, out);
        }
    }
}

/// `placed`'s own size, centred in `rect`.
fn centred(rect: [f32; 4], placed: Placed) -> [f32; 4] {
    let (w, h) = (placed.width as f32, placed.height as f32);
    [
        rect[0] + (rect[2] - w) * 0.5,
        rect[1] + (rect[3] - h) * 0.5,
        w,
        h,
    ]
}
