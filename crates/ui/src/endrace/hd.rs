//! Wipeout HD/Fury's own `EndRace Results`/`EndRace Menu`/`EndRace Rewards` -
//! the same screen *names* Pulse authors, read off a completely different file
//! (`Data\Plugins\Frontend\Gui\EndRace_Definition.xml`,
//! [`oag_hd::endrace::SCREEN_ENTRY`]) at a completely different resolution
//! (1920x1080, not the PSP's 480x272) and a different widget vocabulary. See
//! `docs/formats/hd-endrace-screens.md` for the widget-by-widget read this
//! module draws off, and [`crate::campaign::hd`] for the sibling this mirrors
//! (Race Campaign's own HD extension, landed 2026-09-14).
//!
//! # What draws
//!
//! - **`EndRace Results`**: the title, a headline, and the whole field's
//!   finishing order - `Grid{col}.{row}`, four columns by ten rows, of which
//!   only two are captioned at all (`GridHead1`/`GridHead2`, `IG_HUD_POS`/
//!   `IG_HUD_TIME`; the other two are the literal idstring `X`, width `0` -
//!   inert on the disc itself, not a gap this build introduces). The
//!   player's own row gets `GridHighlight` repositioned onto it, the same
//!   "authored default, overridden at the right row" idiom
//!   [`super::results_draw_list`]'s own `tablehighlight` handling already
//!   uses for Pulse.
//! - **`EndRace Menu`**: the applicable `<Block>` options, at their own
//!   authored positions - which is also how the disc keeps different modes'
//!   option sets from overlapping: `race_again`/`return_to_grid`/
//!   `return_to_menu`/`view_again` sit at three stacked `y`s
//!   (`235`/`285`/`335`) and [`super::menu_options`] never asks for more than
//!   one per `y`, so drawing each option at its own Block's position
//!   reproduces the stacking with no synthetic pitch of this crate's own.
//!
//! # What does not, and why
//!
//! - **The forty `Grid{col}.{row}` cells author no real position at all** -
//!   every one reads `x="0" y="0"` in the file, a template the original
//!   fills in at layout time. This module computes its own row geometry
//!   instead, off the grid's own measured frame
//!   (`GridSideBarL`'s `y="44" height="347"`, within the `Item` that offsets
//!   the whole block by `375, 368`) divided by [`oag_gameplay::MAX_SHIPS`]
//!   rows - **chosen, not measured**: the disc authors ten row slots, not
//!   eight, and no capture pins the real row pitch. Column `x` is not
//!   chosen, though - it is read straight off `GridHead1`/`GridHead2`'s own
//!   resolved position at draw time, never hand-transcribed.
//! - **The ship-badge column (`Gridi.{row}`, `.tga`) never draws.** Every
//!   copy of the file names the same one or two teams' badges for every
//!   race regardless of who is actually on the grid (`Data\Ships\Feisar\fe\miniBW.tga`
//!   repeated for both padding rows in `DATA02`) - a placeholder the
//!   original fills at runtime, the same shape as `Line1`'s own "race
//!   complete!" literal below. Wiring it needs a per-race texture read this
//!   pass does not attempt; left undrawn rather than showing every racer as
//!   the same ship. See `docs/formats/hd-endrace-screens.md`'s own Open
//!   section.
//! - **`Gridp.{row}` (`ER_PERFECT`), `GridStrikeThrough`, `MedalBlock` (a 3-D
//!   `<ImageModel>` trophy, not a 2-D image `oag_ui::screen` collects),
//!   `Target Title`/`RecordNotifyBlock` and the loyalty block Results itself
//!   carries (`loyalty1.1`/`loyalty2`, `834 POINTS`/`3745` placeholders) -
//!   none of these draw this pass. Every one is a real widget with no
//!   settled law behind its trigger, the same "not this pass" this project
//!   already leaves Pulse's own trophy model in - see
//!   [`super::Rewards`]'s module doc for the precedent.
//! - **`EndRace Podium` is not read at all.** `Podium`'s own three
//!   `pod_head.{1,2,3}` widgets all carry the identical idstring
//!   `IG_HUD_1ST`, which reads as an authoring placeholder rather than
//!   something this build could draw correctly, and its badge panels are an
//!   achievement/online system with no analogue here.
//! - **`EndRace Rewards` draws ([`hd_rewards_draw_list`]) but is never
//!   entered by the live flow**: the original never enters it either - no
//!   redirect on any copy of any screen file names it, and the executable
//!   registers no screen class for it. See that function's own doc.
//! - **`Line1`'s own placeholder ("race complete!", a literal `string=`, no
//!   `idstring=` at all) is not drawn verbatim.** [`hd_headline_text`]
//!   substitutes an idstring instead, reusing
//!   [`super::draw::headline_text`]'s table for `TimeTrial`/`SpeedLap`/
//!   `NoPosition` - **measured**, not chosen: `ER_TT_COM`/`ER_SL_COM`/
//!   `ER_SHIP_DES` are confirmed present, verbatim, in `DATA02`'s own
//!   `Data\Plugins\Languages\English\entries.xml` (the same table
//!   `EndRace Menu`'s own Blocks already corroborate for their five shared
//!   idstrings). `Headline::Position` is the one variant that table is
//!   wrong for: HD's own ordinal idstrings are `ER_1PLACE`..`ER_8PLACE`, not
//!   Pulse's `ER_1STP`..`ER_8STP` - see [`hd_headline_text`]'s own doc.
//!   Still **chosen, not measured, that HD fills this exact placeholder with
//!   an idstring lookup at all** - no capture confirms the mechanism, only
//!   the vocabulary a mechanism would draw from.
//! - **Online-only widgets never draw**: `DelayPostMsg` (`ONL_MSG_DELAYPOST`)
//!   and the leaderboard cycle button (`RecordsCycleButton`/`RecordsCycle`,
//!   `ER_GLOB_REC`) would resolve to real text with nothing behind them in
//!   this build - drawing either would show a control that does nothing.

use crate::frontend::{Draw, Placed};
use crate::language::StringTable;
use crate::menu::{Frame, Layers, Picture, Skin};
use crate::screen::{Screen, Text, argb_to_rgba};

use super::draw::{
    fill_draw, format_ticks, headline_text, image_draw, medal_award_text, text_draw,
};
use super::pointer::Target;
use super::{EndRaceMenu, FieldResults, HdRewards, Headline, Layout};

#[cfg(test)]
mod tests;

/// The same brightened selection ink [`super::draw`]'s own `MENU_SELECTED`
/// is - duplicated rather than exported, the same "small helper, own module
/// tree" shape that module's own doc gives for `fill_draw`/`image_draw`.
const MENU_SELECTED: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// The grid's own frame, all three numbers read off `GridSideBarL` inside
/// the `<Item OffsetX="375" OffsetY="368">` that positions the whole block -
/// see the module doc.
const GRID_ROW_AREA_TOP: f32 = 368.0 + 44.0;
const GRID_ROW_AREA_HEIGHT: f32 = 347.0;

/// A named [`Text`] on `screen` - what both the header captions and the
/// menu's own `<Block>` rows are read back as, since [`crate::screen`]'s
/// `"block"` arm folds a `<Block>` into [`Screen::texts`] rather than a
/// vocabulary of its own. `None` when the name is not on this screen at all,
/// which every caller here treats as "draw nothing" rather than a panic -
/// the same tolerance a texture that will not resolve already gets.
fn find_text<'a>(screen: &'a Screen, name: &str) -> Option<&'a Text> {
    screen
        .texts
        .iter()
        .find(|text| text.name.as_deref() == Some(name))
}

/// The `Grid{col}.{row}` widget name a cell carries, e.g. `"Grid1.3"` ->
/// `(1, 3)`. Requires both halves to parse as plain digits, which is what
/// keeps this from matching `GridHead1`/`Gridp.0`/`Grid0.h` - none of those
/// have a numeric first segment.
fn grid_slot(name: &str) -> Option<(usize, usize)> {
    let rest = name.strip_prefix("Grid")?;
    let mut parts = rest.split('.');
    let col: usize = parts.next()?.parse().ok()?;
    let row: usize = parts.next()?.parse().ok()?;
    Some((col, row))
}

/// `Line1`'s own resolved text on HD - [`headline_text`] for every variant
/// but [`Headline::Position`], which HD's own English table names
/// differently from Pulse's: `ER_1PLACE`..`ER_8PLACE` (`"1ST PLACE"`), not
/// Pulse's `ER_1STP`..`ER_8STP` (`"1ST PLACE!"`, with the exclamation mark).
/// **Measured**, not assumed: read directly off `DATA02`'s own
/// `Data\Plugins\Languages\English\entries.xml` (confidence 95 - a direct
/// grep of the shipped table), which is also what confirms
/// [`Headline::TimeTrial`]/`SpeedLap`/`NoPosition`'s own idstrings
/// (`ER_TT_COM`/`ER_SL_COM`/`ER_SHIP_DES`) really are shared verbatim
/// between the two titles, not merely similar English - see the module doc.
fn hd_headline_text(headline: Headline, strings: &StringTable) -> Option<String> {
    if let Headline::Position(place) = headline {
        if !(1..=8).contains(&place) {
            return None;
        }
        return Some(strings.get_or_id(&format!("ER_{place}PLACE")).to_string());
    }
    headline_text(headline, strings)
}

/// A `Grid{col}.{row}` cell's own content - column `0` is place, column `1`
/// is the finish time, columns `2`/`3` are the disc's own inert `X`
/// placeholders (see the module doc) and never draw.
fn grid_cell_text(col: usize, row: usize, model: &FieldResults) -> Option<String> {
    let entry = model.rows.get(row)?;
    match col {
        0 => Some(entry.place.to_string()),
        1 => Some(
            entry
                .time_ticks
                .map_or_else(|| "-".to_string(), format_ticks),
        ),
        _ => None,
    }
}

/// A `Grid{col}.{row}` cell's own `x` - column 0 and 1's real, measured
/// position (`GridHead1`'s/`GridHead2`'s own `x`, read off the screen at
/// draw time rather than hand-transcribed), `None` for columns 2/3, which
/// never draw at all (see the module doc).
fn column_x(screen: &Screen, col: usize) -> Option<f32> {
    let name = match col {
        0 => "GridHead1",
        1 => "GridHead2",
        _ => return None,
    };
    find_text(screen, name).map(|text| text.x)
}

/// `EndRace Results`' draw list: the title, the headline, and the field's
/// own standings grid. See the module doc for what does and does not draw.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts crate::endrace::results_draw_list takes"
)]
pub fn hd_results_draw_list(
    model: &FieldResults,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut layers = Layers {
        backdrop: frame.backdrops(
            skin.space(),
            skin.background(),
            backdrop.map(Picture::draw),
            race_behind,
        ),
        ..Layers::default()
    };
    let screen = &layout.screen;
    let mut out = Vec::new();
    let player_row = model.rows.iter().position(|row| row.player);
    for fill in &screen.fills {
        if fill.name.as_deref() == Some("GridHighlight") {
            // Repositioned onto the player's own row - see the module doc,
            // and `super::results_draw_list`'s `tablehighlight` handling for
            // the identical idiom on Pulse. Drawn nowhere when this race
            // never placed the player at all (a field of one race that
            // never finished, say), which is honest rather than a guess.
            if let Some(row) = player_row {
                out.push(Draw::Fill {
                    rect: [
                        fill.x,
                        row_y(row),
                        fill.width.unwrap_or(0.0),
                        fill.height.unwrap_or(0.0),
                    ],
                    color: argb_to_rgba(fill.color),
                });
            }
            continue;
        }
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        // Every image whose texture this pass chose not to load (the
        // ship-badge column, the target/medal art, the record-notify mode
        // icons) is skipped here for free: `sprites` simply has nothing to
        // give back for it. See the module doc for which those are and why.
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        if let Some((col, row)) = grid_slot(name) {
            // Every `Grid{col}.{row}` cell is authored at `x="0" y="0"` (see
            // the module doc) - the position drawn here is computed, not the
            // widget's own parsed one, the same override
            // [`hd_results_draw_list`]'s own `GridHighlight` handling makes
            // for the same reason.
            let Some(content) = grid_cell_text(col, row, model) else {
                continue;
            };
            let Some(column_x) = column_x(screen, col) else {
                continue;
            };
            let mut positioned = text.clone();
            positioned.x = column_x;
            positioned.y = row_y(row);
            out.push(text_draw(&positioned, &content, layout));
            continue;
        }
        let content = match name {
            "Line1" => hd_headline_text(model.headline, strings),
            // Online-only - see the module doc.
            "DelayPostMsg" | "RecordsCycleButton" | "RecordsCycle" => None,
            // The Target/medal block and the loyalty block - real
            // literal/idstring text this build chose not to draw this
            // pass (see the module doc), which would otherwise leak
            // through the generic fallback arm below exactly the way
            // `ER_PERFECT` would if `Gridp.{row}` were not excluded too.
            "Target Title" | "Target0" | "Target1" | "Target2" | "loyalty1.1" | "loyalty1.2"
            | "loyalty2" => None,
            // `Gridp.{row}` (`ER_PERFECT`) - not modelled, see the module
            // doc. Everything else (`ResultsTitle`, `GridHead1`/`2`,
            // `ControlTextConfirmButton`/`ControlTextConfirm`) already
            // carries its own resolved text.
            _ if name.starts_with("Gridp.") => None,
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        out.push(text_draw(text, &content, layout));
    }
    layers.body = out;
    layers
}

/// A grid row's own `y`, top-anchored - see [`GRID_ROW_AREA_TOP`]'s own doc
/// for the geometry this divides. **Chosen, not measured**: the disc's own
/// row slots (ten) outnumber a race's own maximum field
/// ([`oag_gameplay::MAX_SHIPS`], eight), and nothing pins which of the two
/// the original actually steps by.
fn row_y(row: usize) -> f32 {
    let row_height = GRID_ROW_AREA_HEIGHT / oag_gameplay::MAX_SHIPS as f32;
    GRID_ROW_AREA_TOP + row as f32 * row_height
}

/// `EndRace Rewards`' draw list, off `DATA02`-`05`'s own copy of the screen
/// (`DATA06` authors none). **The original never enters this screen** -
/// `docs/formats/hd-endrace-screens.md`'s "`EndRace Rewards`: authored,
/// never entered" section has the evidence - so everything below that is
/// not a plain authored label is a rule this build chose, not one it
/// measured:
///
/// - The backdrop, the three divider fills and the `ER_REWARD` title draw
///   as authored. The confirm prompt would too, but the disc nests it in a
///   `<NavigationController>`, which [`crate::screen`] does not walk - the
///   same on `Results` and `Menu`.
/// - `BigPos` draws the player's own finishing place, and nothing without
///   one. **Chosen, not measured**: the widget's name and its placeholder
///   `"1"` are the only evidence it is a place at all - its authored centre
///   (`610, 370`) sits inside `MedalImg`'s 32x32 square, so it may as well
///   be a figure drawn on a medal icon.
/// - `MedalImg` and `LoyaltyImg` never draw. Both author a colour and a
///   32x32 size but no `src`, the shape [`crate::screen`]'s own
///   fallback-image doc names as a texture the original assigns at run
///   time; drawing the authored colour as a flat square would be a stand-in
///   for an icon nobody has identified.
/// - `RewardLine1` draws the `ER_GMA`/`ER_SMA`/`ER_BMA`/`ER_NMA` tier on a
///   campaign race, and nothing on any other race. **Chosen, not
///   measured**: the widget authors the bare label `ER_MEDAL_AWARD`
///   (`"MEDAL AWARDED:"`) and nothing names a tier beside it; the four tier
///   idstrings are in HD's own English table verbatim, the same vocabulary
///   Pulse's own `RewardLine1` resolves through.
/// - **The loyalty row draws nothing at all** - `LoyaltyImg`, `RewardLine2`,
///   `RewardLoyaltyPoints`, `RewardLoyaltyActive` (placeholders `"test"`/
///   `"points!"`/`"line 2"`). HD's own loyalty law is not recovered, and
///   Pulse's is the PSP's. `loyaltybar` is a `<Slider>`, which
///   [`crate::screen`] does not collect.
/// - `EndRaceCountDown` (authored empty) and every other named widget draw
///   nothing: the text loop below matches names explicitly and defaults to
///   drawing nothing, so a placeholder this function does not know about
///   cannot leak the way `Results`' Target/loyalty text once did.
///
/// No pointer targets: the screen has nothing to select, and a click
/// anywhere is its confirm - see [`super::pointer`]'s module doc.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts hd_results_draw_list takes"
)]
pub fn hd_rewards_draw_list(
    model: &HdRewards,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut layers = Layers {
        backdrop: frame.backdrops(
            skin.space(),
            skin.background(),
            backdrop.map(Picture::draw),
            race_behind,
        ),
        ..Layers::default()
    };
    let screen = &layout.screen;
    let mut out = Vec::new();
    for fill in &screen.fills {
        // Unnamed fills only: the backdrop and the dividers. See the doc
        // above for `MedalImg`/`LoyaltyImg`.
        if fill.name.is_none() {
            out.push(fill_draw(fill));
        }
    }
    for image in &screen.images {
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    for text in &screen.texts {
        let content = match text.name.as_deref() {
            None | Some("ControlTextConfirmButton" | "ControlTextConfirm") => text.string.clone(),
            Some("BigPos") => model.place.map(|place| place.to_string()),
            Some("RewardLine1") => model
                .campaign
                .then(|| medal_award_text(model.medal, strings)),
            Some(_) => None,
        };
        let Some(content) = content else { continue };
        out.push(text_draw(text, &content, layout));
    }
    layers.body = out;
    layers
}

/// Every `<Block>` name `EndRace Menu` authors an [`MenuOption`] for -
/// skipped in [`hd_menu_draw_list`]'s own generic text loop so that a block
/// this build has no [`MenuOption`] for (`quit_tournament`/`return_to_lobby`/
/// `view_MP_again` - Tournament and multiplayer, neither implemented here)
/// does not fall through to the default arm and draw unconditionally. The
/// five this build does implement are drawn explicitly, at their own
/// authored position, after this loop.
const ALL_BLOCK_NAMES: [&str; 8] = [
    "next_race",
    "race_again",
    "return_to_grid",
    "return_to_menu",
    "quit_tournament",
    "return_to_lobby",
    "view_again",
    "view_MP_again",
];

/// `EndRace Menu`'s draw list: the title and the applicable `<Block>`
/// options, at their own authored positions. See the module doc for why no
/// synthetic row pitch is needed here the way Pulse's own list-based menu
/// wants one.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts crate::endrace::endrace_menu_draw_list takes"
)]
pub fn hd_menu_draw_list(
    model: &EndRaceMenu,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut layers = Layers {
        backdrop: frame.backdrops(
            skin.space(),
            skin.background(),
            backdrop.map(Picture::draw),
            race_behind,
        ),
        ..Layers::default()
    };
    let screen = &layout.screen;
    let mut out = Vec::new();
    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        if ALL_BLOCK_NAMES.contains(&name) {
            continue;
        }
        let Some(content) = text.string.clone() else {
            continue;
        };
        out.push(text_draw(text, &content, layout));
    }
    for (index, option) in model.options().iter().enumerate() {
        let Some(block) = find_text(screen, option.hd_block_name()) else {
            continue;
        };
        let content = strings.get_or_id(option.idstring()).to_string();
        let mut draw = text_draw(block, &content, layout);
        if index == model.index()
            && let Draw::Text { color, .. } = &mut draw
        {
            *color = MENU_SELECTED;
        }
        out.push(draw);
    }
    layers.body = out;
    layers
}

/// A `<Block>` row's own click rect. **Width is measured** - every option
/// Block on this screen authors `width="520"`, read here as a shared
/// constant rather than parsed generically (nothing else needs a `Block`'s
/// width, so [`crate::screen`] does not carry one). **Height is chosen**: no
/// Block on this screen authors one at all, the same gap
/// [`crate::campaign::hd::difficulty_button_rect`] already names for HD's
/// campaign screen and fills the same way.
const BLOCK_WIDTH: f32 = 520.0;
const BLOCK_HEIGHT: f32 = 40.0;

/// `EndRace Menu`'s own row targets, one per applicable option, off each
/// option's own `<Block>` position - the pointer-support counterpart to
/// [`hd_menu_draw_list`]'s own position lookup, per this project's own rule
/// that a new screen ships with pointer support alongside the pad.
#[must_use]
pub fn hd_menu_targets(model: &EndRaceMenu, layout: &Layout) -> Vec<Target> {
    let screen = &layout.screen;
    model
        .options()
        .iter()
        .enumerate()
        .filter_map(|(index, option)| {
            let block = find_text(screen, option.hd_block_name())?;
            Some(Target {
                index,
                rect: [
                    block.x,
                    block.y - BLOCK_HEIGHT * 0.25,
                    BLOCK_WIDTH,
                    BLOCK_HEIGHT,
                ],
            })
        })
        .collect()
}
