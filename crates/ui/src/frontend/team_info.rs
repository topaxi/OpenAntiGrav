//! `TeamInfo`: the statistics panel under the Select Team grid.
//!
//! Evidence page: `docs/ghidra/functions/vita-2048-eu-v104/team-grid.md`
//! ("The statistics panel"). The widget is `Frontend/Items/TeamInfo_Item.cpp`
//! (`/2048/eboot-vita-2048-eu-v104.elf`: constructor `0x813ceab0`, draw
//! `0x810e0bf6`, row draw `0x810e071a`, header icons `0x81061808`, ability
//! icons `0x810614aa`); `Team_Definition.xml` authors only its anchor,
//! `<TeamInfo name="teamscreeninfo"><Values x="16" y="307">`. Everything
//! below is read off that decompile and its call sites' registers, at rest
//! (the original counts the bars up on entry and slides the panel in; neither
//! is drawn here).
//!
//! # What is drawn
//!
//! - **The table** (`0x810e071a`, four calls from `0x810e19f0`): four rows
//!   at `x = anchor.x + 416`, `y = anchor.y + 30 * n`, each a 512 x 27
//!   `Transparent2048` strip, its label (`RC_SPEED`, `RC_HANDLING`,
//!   `RC_HEALTH`, `RC_FIREPOWER`) at `(+9, +5)` in `Blue2048` and its number
//!   centred at `+478`, then ten 22-wide cells at `+200 + 25 i`. Cell `i` is
//!   `Orange2048` when `rating - i - 0.5` is positive (alpha
//!   `clamp((rating - i - 0.5) * 2)`), else the literal grey `0x808080`
//!   (`FUN_8105bc14(1.0, 0x808080, 0xff808080)`). A rating of 0 or 11 prints
//!   `--` (the literals at `0x81453e50`/`0x81453e54`).
//! - **The numbers** are `<FE>` of the craft's `PI_TeamModel` in
//!   `Data\Plugins\teams\Definition.xml` (`<FE speed thrust handling shield
//!   length firepower description>`), which is the ship record the screen's
//!   update (`0x8113a3b0`) hands the panel: it passes `+0x1c4`, `+0x1cc`,
//!   `+0x1d0` and `+0x1d8` of it, in row order `speed`, `handling`, `shield`
//!   (the HEALTH label) and `firepower`. In the HD/Fury mode the same call
//!   passes `+0x1c8` (`thrust`) as the fourth row and labels the rows
//!   `RC_SHIELD` and `RC_THRUST`. **Not** `HandlingStats`' own `<FE>`, which
//!   agrees on Feisar Speed (8, 7, 4, 6) and differs elsewhere (Feisar Fighter
//!   speed 5 against 3).
//! - **The header**: a `Transparent2048` band `(anchor.x + 510, anchor.y - 44)`
//!   418 x 44, the team icon (`Icon_Team_*`, 64 x 64) at `(anchor.x + 446,
//!   anchor.y - 54)`, the craft-class icon (`Icon_Ship_*`, 64 x 64) 40 to its
//!   left, and the team name and craft word at `(anchor.x + 530, anchor.y -
//!   39)` in the `Default` face at 0.82.
//! - **The two ability icons** (`0x810614aa`, called with `(anchor.x + 870,
//!   anchor.y - 83)`): 64 x 64, centred there and 80 to its left, naming what
//!   the craft cannot carry (`no_bomb`, `no_mines`, `no_quake`) or, for two
//!   prototypes, what it has instead, tinted `Blue2048` (the call's colour is
//!   the one the label uses; the glyphs are white on disc).
//!
//! # What is chosen, not read
//!
//! - The craft word is drawn in the same face and weight as the team name in
//!   one string (`FEISAR SPEED`); the original draws the second word in a
//!   lighter face at the first word's measured width plus 10. This front end
//!   has no glyph advances, so the gap is the face's own space.
//! - Prototype craft draw a scrolling description in place of the rows
//!   (`strcmp(craft, "prototype") == 0` selects that branch of `0x810e0bf6`);
//!   it is not drawn, and the rows are not drawn for a prototype either.
//! - The `FE_SHIP_STATS_TUT` bubble and the ability icons' touch tooltips
//!   (`Ship_Select_No_*`) are not drawn.
//!
//! **Omega: checked, applies, not wired.** `eboot.bin` carries the same item
//! class; no `Team_Definition.xml` ships there, so no screen instantiates it.

use super::team::{NATIVE_TEAMS, SHIP_TYPE_LABELS, SHIP_TYPES, TEAM_COLUMNS};
use super::*;

/// `<TeamInfo>`'s authored anchor in `Team_Definition.xml`.
const ANCHOR: [f32; 2] = [16.0, 307.0];
/// The table's left edge from the anchor, `0x810e0bf6`'s `+ 416.0`.
const TABLE_X: f32 = 416.0;
/// The panel's width, `0x810e071a`'s `+ 512`.
const PANEL_W: f32 = 512.0;
/// A row's height, `0x810e071a`'s `+ 27`.
const ROW_H: f32 = 27.0;
/// The step between rows, `0x810e0bf6`'s `+ 0x1e`.
const ROW_STEP: f32 = 30.0;
/// The panel under the rows: `anchor.y .. anchor.y + 117`.
const PANEL_H: f32 = 117.0;
/// The label's offset into its row, `0x810e071a`.
const LABEL_AT: [f32; 2] = [9.0, 5.0];
/// The label's scale in the `NEOSANS_BOLD` face, the literal `0x3f333333`.
const LABEL_SCALE: f32 = 0.7;
/// The number's centre from the row's left edge, `0x810e071a`'s `+ 0x1de`.
const NUMBER_AT: f32 = 478.0;
/// Where the ten cells start in a row and their pitch and width.
const CELLS_AT: f32 = 200.0;
const CELL_PITCH: f32 = 25.0;
const CELL_W: f32 = 22.0;
const CELLS: usize = 10;
/// The empty cell: `FUN_8105bc14(1.0, 0x808080, 0xff808080)`.
const EMPTY_CELL: [f32; 4] = [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0];

/// The header band's left edge and width from the anchor, `0x810e0bf6`.
const HEADER_X: f32 = 510.0;
const HEADER_W: f32 = 418.0;
const HEADER_H: f32 = 44.0;
/// The team icon's offset from the anchor, `0x810e15a6`: `+ 446`, `- 54`.
const TEAM_ICON_AT: [f32; 2] = [446.0, -54.0];
/// An icon's edge, `FUN_81061808`'s literal `64.0`.
const ICON: f32 = 64.0;
/// The craft icon sits this far left of the team icon.
const CLASS_ICON_DX: f32 = -40.0;
/// The header text's offset, `0x810e0bf6`: `+ 446 + 84`, `- 54 + 15`.
const NAME_AT: [f32; 2] = [530.0, -39.0];
/// Its scale, the literal `0x3f51eb85`.
const NAME_SCALE: f32 = 0.82;
/// The ability icons' centre from the anchor and the gap between them.
const ABILITY_AT: [f32; 2] = [870.0, -83.0];
const ABILITY_DX: f32 = 80.0;

/// The team icons, in [`TEAM_COLUMNS`]' order (`FUN_8105dcb8`'s loader).
const TEAM_ICONS: [&str; 5] = [
    r"Data\FE\NewImages\Team_Logos\Icon_Team_Feisar.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Team_AG-SYS.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Team_Qirex.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Team_Auricom.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Team_Pirhana.gtf",
];

/// The craft-class icons: `SHIP_TYPES`' order (fighter, agility, speed), then
/// the three prototype ones.
const SHIP_ICONS: [&str; 6] = [
    r"Data\FE\NewImages\Team_Logos\Icon_Ship_Combat.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Ship_Agility.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Ship_Racer.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Ship_Racer_proto.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Ship_Combat_proto.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Ship_Agility_proto.gtf",
];

const NO_BOMB: &str = r"Data\FE\NewImages\weapons\no_bomb.gtf";
const NO_MINES: &str = r"Data\FE\NewImages\weapons\no_mines.gtf";
const NO_QUAKE: &str = r"Data\FE\NewImages\weapons\no_quake.gtf";
const NO_DEFENSIVE: &str = r"Data\FE\NewImages\weapons\defensive_off.gtf";
const MINIGUNS: &str = r"Data\FE\NewImages\weapons\cannon.gtf";

/// The craft index of `prototype` in [`SHIP_TYPES`].
const PROTOTYPE: usize = 3;

/// The row labels, as string ids (`0x810e19f0` and its three siblings).
const ROW_LABELS: [&str; 4] = ["RC_SPEED", "RC_HANDLING", "RC_HEALTH", "RC_FIREPOWER"];

/// One craft's `<FE>` ratings: what the panel's four rows show.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeamRating {
    /// The team, as [`NATIVE_TEAMS`] spells it.
    pub team: &'static str,
    /// The craft, as an index into [`SHIP_TYPES`].
    pub craft: usize,
    /// `speed`, `handling`, `shield` (HEALTH) and `firepower`, in row order.
    pub rows: [f32; 4],
}

/// Every texture the panel draws that no widget names, for the sprite
/// sheet's `extra` list.
#[must_use]
pub fn team_info_textures() -> Vec<&'static str> {
    TEAM_ICONS
        .iter()
        .chain(SHIP_ICONS.iter())
        .copied()
        .chain([NO_BOMB, NO_MINES, NO_QUAKE, NO_DEFENSIVE, MINIGUNS])
        .collect()
}

/// The craft-class icon of `craft` for `team`'s column: the prototype's
/// depends on the team (`FUN_81061808`).
fn ship_icon(column: usize, craft: usize) -> &'static str {
    let index = match (craft, column) {
        (0, _) => 0,
        (1, _) => 1,
        (2, _) => 2,
        (_, 1) => 5,
        (_, 2 | 3) => 4,
        _ => 3,
    };
    SHIP_ICONS[index]
}

/// The two ability icons `(first, second)` of a craft, `FUN_810614aa`: the
/// first at the anchor's `x`, the second 80 to its left.
fn abilities(team: &str, craft: usize) -> (Option<&'static str>, Option<&'static str>) {
    let prototype = craft == PROTOTYPE;
    let first = match (prototype, team) {
        (true, "Auricom2048") => NO_DEFENSIVE,
        (true, "Qirex2048") => MINIGUNS,
        (true, "AG_Systems2048") => NO_BOMB,
        (false, _) if SHIP_TYPES[craft] == "fighter" => NO_MINES,
        _ => NO_BOMB,
    };
    let special = prototype && matches!(team, "Auricom2048" | "Qirex2048" | "AG_Systems2048");
    let second = (!special && (SHIP_TYPES[craft] == "speed" || prototype)).then_some(NO_QUAKE);
    (Some(first), second)
}

impl Frontend {
    /// Gives the Select Team panel its numbers: every craft's `<FE>` row, as
    /// the composition root read off the disc. Empty draws no rows (never a
    /// zero bar).
    pub fn set_team_ratings(&mut self, ratings: Vec<TeamRating>) {
        self.touch.team_ratings = ratings;
    }

    fn team_rating(&self, team: usize, craft: usize) -> Option<[f32; 4]> {
        self.touch
            .team_ratings
            .iter()
            .find(|rating| rating.team == NATIVE_TEAMS[team] && rating.craft == craft)
            .map(|rating| rating.rows)
    }

    /// The panel for the craft the cursor is on.
    pub(super) fn draw_team_info(&self, team: usize, craft: usize, out: &mut Vec<Draw>) {
        let [ax, ay] = ANCHOR;
        let blue = self.global_colour("Blue2048");
        let veil = self.global_colour("Transparent2048");
        let column = TEAM_COLUMNS
            .iter()
            .position(|name| *name == NATIVE_TEAMS[team])
            .unwrap_or(0);
        let x = ax + TABLE_X;

        if craft != PROTOTYPE
            && let Some(ratings) = self.team_rating(team, craft)
        {
            out.push(Draw::Fill {
                rect: [x, ay, PANEL_W, PANEL_H],
                color: veil,
            });
            for (row, (&rating, label)) in ratings.iter().zip(ROW_LABELS).enumerate() {
                self.draw_rating_row(x, ay + row as f32 * ROW_STEP, label, rating, out);
            }
        }

        out.push(Draw::Fill {
            rect: [ax + HEADER_X, ay - HEADER_H, HEADER_W, HEADER_H],
            color: veil,
        });
        let white = [1.0; 4];
        let team_icon = self.placed(TEAM_ICONS[column]);
        let class_icon = self.placed(ship_icon(column, craft));
        for (icon, dx) in [(team_icon, 0.0), (class_icon, CLASS_ICON_DX)] {
            if let Some(placed) = icon {
                let rect = [ax + TEAM_ICON_AT[0] + dx, ay + TEAM_ICON_AT[1], ICON, ICON];
                out.push(Self::sprite_at(rect, placed, white));
            }
        }
        out.push(Draw::Text {
            x: ax + NAME_AT[0],
            y: ay + NAME_AT[1],
            scale: NAME_SCALE,
            color: blue,
            border: None,
            align: Align::Left,
            text: format!(
                "{} {}",
                self.strings.get_or_id(NATIVE_TEAMS[team]),
                self.strings.get_or_id(SHIP_TYPE_LABELS[craft])
            ),
            wrap_width: None,
        });

        let (first, second) = abilities(NATIVE_TEAMS[team], craft);
        let centre = [ax + ABILITY_AT[0], ay + ABILITY_AT[1]];
        for (icon, dx) in [(first, 0.0), (second, -ABILITY_DX)] {
            if let Some(placed) = icon.and_then(|name| self.placed(name)) {
                let rect = [
                    centre[0] + dx - ICON * 0.5,
                    centre[1] - ICON * 0.5,
                    ICON,
                    ICON,
                ];
                out.push(Self::sprite_at(rect, placed, blue));
            }
        }
    }

    fn draw_rating_row(&self, x: f32, y: f32, label: &str, rating: f32, out: &mut Vec<Draw>) {
        let blue = self.global_colour("Blue2048");
        let orange = self.global_colour("Orange2048");
        out.push(Draw::Fill {
            rect: [x, y, PANEL_W, ROW_H],
            color: self.global_colour("Transparent2048"),
        });
        let scale = LABEL_SCALE * self.face_scale("NEOSANS_BOLD");
        out.push(Draw::Text {
            x: x + LABEL_AT[0],
            y: y + LABEL_AT[1],
            scale,
            color: blue,
            border: None,
            align: Align::Left,
            text: self.strings.get_or_id(label).to_string(),
            wrap_width: None,
        });
        let whole = rating as i32;
        out.push(Draw::Text {
            x: x + NUMBER_AT,
            y: y + LABEL_AT[1],
            scale,
            color: blue,
            border: None,
            align: Align::Centre,
            text: if whole == 0 || whole == 11 {
                "--".to_string()
            } else {
                whole.to_string()
            },
            wrap_width: None,
        });
        // The original draws all ten grey, then the ten filled ones over them.
        let cell_rect = |cell: usize| [x + CELLS_AT + CELL_PITCH * cell as f32, y, CELL_W, ROW_H];
        for cell in 0..CELLS {
            out.push(Draw::Fill {
                rect: cell_rect(cell),
                color: EMPTY_CELL,
            });
        }
        for cell in 0..CELLS {
            let fill = ((rating - cell as f32 - 0.5) * 2.0).clamp(0.0, 1.0);
            if fill > 0.0 {
                out.push(Draw::Fill {
                    rect: cell_rect(cell),
                    color: [orange[0], orange[1], orange[2], orange[3] * fill],
                });
            }
        }
    }
}
