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
//!   Each option draws as the Block it is - box, focus colour, blinking
//!   arrow - per [`hd_menu_draw_list`], and steps in screen order
//!   ([`hd_screen_order`]).
//!
//! # What does not, and why
//!
//! - **The forty `Grid{col}.{row}` cells author no real position at all** -
//!   every one reads `x="0" y="0"` in the file, a template the original
//!   fills in at layout time. **On a race the layout is the executable's**
//!   ([`RaceGrid`]: rows at `96 + 45 r`, columns at `40`/`200`/`545`, the
//!   frame stretched to a `487` bottom bar and the footer block hidden -
//!   `docs/ghidra/functions/ps3-hdfury-eu/endrace-results-grid.md`). On Time
//!   Trial / Speed Lap, whose fillers are unread, the rows keep this
//!   module's older geometry: the file's frame divided by
//!   [`oag_gameplay::MAX_SHIPS`] rows, **chosen, not measured**, with
//!   column `x` read off `GridHead1`/`GridHead2`.
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
//!   `Target Title`/`RecordNotifyBlock` - none of these draw this pass. Every one is a real widget with no
//!   settled law behind its trigger, the same "not this pass" this project
//!   already leaves Pulse's own trophy model in - see
//!   [`super::Rewards`]'s module doc for the precedent.
//! - **`EndRace Podium` draws ([`hd_podium_draw_list`]) but is never entered by
//!   the live flow**: its entry is untraced. See [`podium`]'s module doc.
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

use oag_ui::frontend::{Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::screen::{BlockWidget, Screen, Text, argb_to_rgba};

use super::draw::{
    fill_draw, format_ticks, headline_text, image_draw, medal_award_text, text_draw,
};
use super::pointer::Target;
use super::{EndRaceMenu, FieldResults, HdRewards, Headline, Layout};

mod podium;
pub use podium::{HdPodium, PodiumSlot, hd_podium_draw_list};

#[cfg(test)]
mod tests;

/// The grid's own frame, all three numbers read off `GridSideBarL` inside
/// the `<Item OffsetX="375" OffsetY="368">` that positions the whole block -
/// see the module doc.
const GRID_ROW_AREA_TOP: f32 = 368.0 + 44.0;
const GRID_ROW_AREA_HEIGHT: f32 = 347.0;

/// A named [`Text`] on `screen` - what both the header captions and the
/// menu's own `<Block>` rows are read back as, since [`oag_ui::screen`]'s
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
    let race = RaceGrid::of(model.headline, screen);
    for fill in &screen.fills {
        // `0x00223f50`, the grid reset every mode's layout runs first,
        // hides `GridStrikeThrough` and nothing this build drives shows it
        // again - a struck-out row this build does not model.
        if fill.name.as_deref() == Some("GridStrikeThrough") {
            continue;
        }
        if let Some(race) = &race {
            if let Some(draw) = race.fill(fill, player_row) {
                out.extend(draw);
                continue;
            }
        } else if fill.name.as_deref() == Some("GridHighlight") {
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
        if let Some((col, row)) = grid_slot(name)
            && let Some(race) = &race
        {
            let Some(content) = race_cell_text(col, row, model) else {
                continue;
            };
            let mut positioned = text.clone();
            positioned.x = race.origin.0 + RACE_COLUMN_X[col];
            positioned.y = race.origin.1 + RACE_ROW_TOP + row as f32 * RACE_ROW_PITCH;
            if player_row == Some(row) {
                positioned.color = 0xffff_ffff;
            }
            out.push(text_draw(&positioned, &content, layout));
            continue;
        }
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
        // The column headers are `<Block>`s: drawn as their box with their
        // label inside it, which is what makes the white `POS`/`TIME`
        // readable at all. `GridHead3`/`GridHead4` are `width="0"` with the
        // placeholder label `X`, and `0x0022c068` hides both on a race.
        if matches!(name, "GridHead1" | "GridHead2") {
            if let Some(block) = find_block(screen, name)
                && let Some(content) = text.string.as_deref()
            {
                block_draw(
                    block,
                    Some((text, content)),
                    None,
                    0.0,
                    frame,
                    layout,
                    &mut out,
                );
            }
            continue;
        }
        if matches!(name, "GridHead3" | "GridHead4") {
            continue;
        }
        // The loyalty group's two `<Block>`s (`ER_LOY`, `IG_HUD_TOTAL`) author no
        // `name`, so they cannot be looked up the way the grid headers are: the
        // block sharing this text's position is its box, in `HD_Blue` behind the
        // white label. Drawn as the text alone they were white on the light panel.
        if name.is_empty()
            && let Some(block) = unnamed_block_at(screen, text)
        {
            if let Some(content) = text.string.as_deref() {
                block_draw(
                    block,
                    Some((text, content)),
                    None,
                    0.0,
                    frame,
                    layout,
                    &mut out,
                );
            }
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
            "Target Title" | "Target0" | "Target1" | "Target2" => None,
            // The loyalty block's final state. `loyalty1.2` is the ticker's
            // reason text and ends on `""`, so it draws nothing; with no
            // `loyalty` on the model the disc's placeholders stay hidden.
            "loyalty1.1" => model
                .loyalty
                .map(|loyalty| format!("{} {}", loyalty.award, strings.get_or_id("ER_POINTS"))),
            "loyalty2" => model.loyalty.map(|loyalty| loyalty.total.to_string()),
            "loyalty1.2" => None,
            // The `NavigationController`'s own icon glyph - `font="buttons"`
            // (`ps_buttons.fnt`) - still draws nothing **here**, unchanged
            // by this widget's own local `Layout` never learning a
            // `Buttons`-role atlas the way `oag_ui_screens::campaign::footer`'s
            // shared-root path now does (2026-09-25,
            // `crate::render::Renderer::set_buttons_atlas`): this screen's
            // `text_draw` (this function's own, not `campaign::draw`'s) has
            // no `face_role` check at all, so nothing here would route a
            // `font="buttons"` widget to that atlas even once loaded - a
            // real gap, left open rather than fixed in the same pass that
            // found it (out of this screen's own lane). Left `None` in the
            // meantime for the reason it always was: drawing the resolved
            // idstring's literal codepoint (`"ε"`) through whichever body
            // face this screen's `Layout` was built with would show a
            // Greek letter, not the disc's own cross-button glyph, and per
            // `CLAUDE.md`'s "never invent what the assets already author" a
            // wrong glyph is worse than none. `ControlTextConfirm` (the
            // word, `font="default"` - a loaded face) still draws below.
            "ControlTextConfirmButton" => None,
            // `Gridp.{row}` (`ER_PERFECT`) - not modelled, see the module
            // doc. Everything else (`ResultsTitle`, `GridHead1`/`2`,
            // `ControlTextConfirm`) already carries its own resolved text.
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

/// The executable's race-family grid (`0x0022c068`'s mode-3 branch, the
/// frame, and `0x0022b688`, the rows), all relative to the grid's own
/// `<Item OffsetX="375" OffsetY="368">`. The file's own frame numbers
/// (`GridSideBarL`'s `height="347"`, `GridBottomBar`'s `y="391"`, a visible
/// `GridBottomBlock`) are the Time Trial/Speed Lap layout - `0x0022c068`'s
/// mode-5/10 branch writes exactly those back - and a race rewrites them:
///
/// - side bars at `y = 44` (`0x008b08c0`), `443` tall (`0x008b0874`);
/// - bottom bar at `y = 487` (`0x008b087c`), which is `44 + 443`;
/// - `GridTopBar`, `GridBottomBlock` and `RecordNotifyBlock` hidden (and
///   `GridStrikeThrough`, which `0x00223f50`'s reset hides on every mode);
/// - row `r`'s text at `y = 96 + 45 r` (`0x008b07b4`, `0x008b07b0`),
///   `GridHighlight` at `y = 94 + 45 r` (the `0x5e` start and `0x2d` step
///   `0x0022b688` passes `0x002278c0`), the player's row inked white.
///
/// Eight rows end at `96 + 7 * 45 = 411`, inside the `487` bar; the file's
/// own `357` footer block is what the old chosen pitch ran row eight into.
struct RaceGrid {
    origin: (f32, f32),
}

/// `0x0022b688`'s three row columns: `Grid0.r` at `x = 40` (`0x42200000`),
/// `Grid1.r` at `200` (`0x43480000`), `Grid2.r` at `545` (`0x008b07d4`).
/// Header labels sit at a Block's `X + 40` ([`LABEL_INSET`]), so `GridHead1`
/// (`x = 0`) and `GridHead2` (`x = 505`) caption columns 0 and 2 exactly -
/// the cross-check that these are the columns the captions mean.
const RACE_COLUMN_X: [f32; 4] = [40.0, 200.0, 545.0, 0.0];
const RACE_ROW_TOP: f32 = 96.0;
const RACE_ROW_PITCH: f32 = 45.0;
const RACE_HIGHLIGHT_TOP: f32 = 94.0;
const RACE_FRAME_TOP: f32 = 44.0;
const RACE_FRAME_HEIGHT: f32 = 443.0;
const RACE_BOTTOM_BAR_Y: f32 = 487.0;

impl RaceGrid {
    /// The race layout, for a race-family headline on a screen that has the
    /// grid; `None` for Time Trial/Speed Lap (whose own rows `0x002239a8`/
    /// `0x00227b30` lay out and this pass did not read) and for a screen
    /// with no `GridHead1`. The grid's origin is `GridHead1`'s own
    /// position less its `y = 24` - `0x0022c068` moves it to `x = 0` and
    /// leaves the file's `y`, so the block itself is the anchor.
    fn of(headline: Headline, screen: &Screen) -> Option<Self> {
        if !matches!(headline, Headline::Position(_) | Headline::NoPosition) {
            return None;
        }
        let head = find_block(screen, "GridHead1")?;
        Some(Self {
            origin: (head.x, head.y - GRID_HEAD_Y),
        })
    }

    /// A frame fill as the race layout draws it: `Some(vec![])` for one it
    /// hides, `Some` of the moved fill for one it moves, `None` for a fill
    /// it leaves as authored.
    fn fill(&self, fill: &oag_ui::screen::Fill, player_row: Option<usize>) -> Option<Vec<Draw>> {
        let moved = |y: f32, height: Option<f32>| {
            let mut moved = fill.clone();
            moved.y = self.origin.1 + y;
            if let Some(height) = height {
                moved.height = Some(height);
            }
            vec![fill_draw(&moved)]
        };
        match fill.name.as_deref()? {
            "GridTopBar" | "GridBottomBlock" => Some(Vec::new()),
            "GridSideBarL" | "GridSideBarR" => Some(moved(RACE_FRAME_TOP, Some(RACE_FRAME_HEIGHT))),
            "GridBottomBar" => Some(moved(RACE_BOTTOM_BAR_Y, None)),
            "GridHighlight" => Some(player_row.map_or_else(Vec::new, |row| {
                moved(RACE_HIGHLIGHT_TOP + row as f32 * RACE_ROW_PITCH, None)
            })),
            _ => None,
        }
    }
}

/// `GridHead1`'s own `y` inside the grid's `Item`, as the file authors it.
const GRID_HEAD_Y: f32 = 24.0;

/// A race-layout cell's content: column 0 the place (`0x0022b688` formats
/// the entry's `+0x168` position), column 2 the finish time. **Column 1
/// draws nothing**: `0x0022b688` fills it from a string at the entry's own
/// `+0x50`, a field this pass did not identify - most likely a pilot or team
/// name, but a guess is not drawn. Column 3 is never written on a race.
fn race_cell_text(col: usize, row: usize, model: &FieldResults) -> Option<String> {
    let entry = model.rows.get(row)?;
    match col {
        0 => Some(entry.place.to_string()),
        2 => Some(
            entry
                .time_ticks
                .map_or_else(|| "-".to_string(), format_ticks),
        ),
        _ => None,
    }
}

/// `EndRace Rewards`' draw list, off `DATA02`-`05`'s own copy of the screen
/// (`DATA06` authors none). **The original never enters this screen** -
/// `docs/formats/hd-endrace-screens.md`'s "`EndRace Rewards`: authored,
/// never entered" section has the evidence - so everything below that is
/// not a plain authored label is a rule this build chose, not one it
/// measured:
///
/// - The backdrop, the three divider fills and the `ER_REWARD` title draw
///   as authored, and now so does the confirm prompt's own word
///   (`ControlTextConfirm`, `"CONFIRM"`): the disc nests it in a
///   `<NavigationController>`, which [`oag_ui::screen`]'s
///   `collect_widgets` walks straight through as of the change that added
///   this sentence - the same on `Results` and `Menu`. Its own icon glyph
///   (`ControlTextConfirmButton`, `font="buttons"`) still draws nothing -
///   see this function's own text-loop match arm for why.
/// - `BigPos` draws the player's own finishing place, and nothing without
///   one. **Chosen, not measured**: the widget's name and its placeholder
///   `"1"` are the only evidence it is a place at all - its authored centre
///   (`610, 370`) sits inside `MedalImg`'s 32x32 square, so it may as well
///   be a figure drawn on a medal icon.
/// - `MedalImg` and `LoyaltyImg` never draw. Both author a colour and a
///   32x32 size but no `src`, the shape [`oag_ui::screen`]'s own
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
///   `"points!"`/`"line 2"`). HD never enters this screen, and it shows its
///   loyalty on `Results` instead (see [`hd_results_draw_list`]). `loyaltybar`
///   is a `<Slider>`, which [`oag_ui::screen`] does not collect.
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
            // The icon glyph draws nothing - see `hd_results_draw_list`'s
            // identical exclusion and its own doc for why.
            Some("ControlTextConfirmButton") => None,
            None | Some("ControlTextConfirm") => text.string.clone(),
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
/// options, each drawn as the box `Block_Render` draws with its label and,
/// on the focused one, its marker arrow - see [`block_draw`]. Positions are
/// each Block's own authored ones, so no synthetic row pitch is needed here
/// the way Pulse's own list-based menu wants one (see the module doc).
///
/// **The cursor is the block's fill**, `Block_Update`'s own focus rule: the
/// focused option switches from its authored `Color` (`0xff646464`) to its
/// `ActiveColor` - which this screen never authors, so it is
/// `Block_Construct`'s compiled-in `0xff8ac0ca` - and eases 60 units wider,
/// while a 32x32 arrow blinks at its left edge. The label stays its
/// authored `TextColor` throughout; nothing brightens it.
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
        // The icon glyph draws nothing - see `hd_results_draw_list`'s
        // identical exclusion and its own doc for why.
        if ALL_BLOCK_NAMES.contains(&name) || name == "ControlTextConfirmButton" {
            continue;
        }
        let Some(content) = text.string.clone() else {
            continue;
        };
        out.push(text_draw(text, &content, layout));
    }
    for (index, option) in model.options().iter().enumerate() {
        let name = option.hd_block_name();
        let Some(block) = find_block(screen, name) else {
            continue;
        };
        let content = strings.get_or_id(option.idstring()).to_string();
        let focus = (index == model.index()).then(|| model.focus(index));
        let label = find_text(screen, name).map(|text| (text, content.as_str()));
        block_draw(
            block,
            label,
            focus,
            model.focus(index).fraction(),
            frame,
            layout,
            &mut out,
        );
    }
    layers.body = out;
    layers
}

/// `model` with its options in the order their `<Block>`s stand on
/// `layout`'s screen, top to bottom, and the same option still focused.
///
/// The shared option list ([`super::MenuOption`], built by the binary's
/// `menu_options`) is Pulse's `<Menu>` order - `RETURN TO GRID` first - and
/// HD draws each option at its own Block's `y` instead
/// (`race_again` 235, `return_to_grid` 285, `view_again` 335). Stepping the
/// list's order on HD made Up move the cursor down the screen. A Block the
/// screen does not author sorts last, where it draws nothing anyway.
#[must_use]
pub fn hd_screen_order(model: &EndRaceMenu, layout: &Layout) -> EndRaceMenu {
    let focused = model.selected();
    let mut options = model.options().to_vec();
    options.sort_by(|a, b| {
        let y = |option: &super::MenuOption| {
            find_block(&layout.screen, option.hd_block_name()).map_or(f32::INFINITY, |b| b.y)
        };
        y(a).total_cmp(&y(b))
    });
    let mut out = EndRaceMenu::new(options, model.new_best_lap_ticks);
    if let Some(focused) = focused
        && let Some(index) = out.options().iter().position(|option| *option == focused)
    {
        out.select(index);
    }
    out
}

/// A named [`BlockWidget`] on `screen` - the box half of a `<Block>`, see
/// [`oag_ui::screen::BlockWidget`].
fn find_block<'a>(screen: &'a Screen, name: &str) -> Option<&'a BlockWidget> {
    screen
        .blocks
        .iter()
        .find(|block| block.name.as_deref() == Some(name))
}

/// The `<Block>` with no `name` that sits where `text` does - the half of an
/// unnamed block [`oag_ui::screen`] folded into [`Screen::texts`].
fn unnamed_block_at<'a>(screen: &'a Screen, text: &Text) -> Option<&'a BlockWidget> {
    screen.blocks.iter().find(|block| {
        block.name.is_none() && (block.x - text.x).abs() < 0.5 && (block.y - text.y).abs() < 0.5
    })
}

/// Where a Block's label sits inside it, `(40, 3)`: `0x00189c38` puts the
/// label child at `X + 40` (TOC `0x008ac55c`) and, on an output of 720
/// lines or more, `Y + 3`. The 720-line test is the PS3's own video mode,
/// not this build's window.
const LABEL_INSET: (f32, f32) = (40.0, 3.0);
/// Where a selectable Block's marker arrow sits, `(8, 4)`, and its size,
/// `32`: `0x0018c0e8` places a 32x32 `HD_options_arrow.gtf` image at
/// `X + 8` (`0x008ac554`), `Y + 4` (`0x008ac588`).
const ARROW_OFFSET: (f32, f32) = (8.0, 4.0);
const ARROW_SIZE: f32 = 32.0;

/// One `<Block>` as `Block_Render` draws it: the box, then the arrow on a
/// focused selectable block while its blink is lit, then the label.
///
/// `focus` is `Some` only on the focused block; `fraction` is the block's
/// own eased focus, which keeps widening/narrowing after focus moves.
/// `frame.blocks` is the decoded nine-patch; with none, the box draws
/// nothing and the label still does, the same rule every other HD menu
/// applies to a nine-patch that did not decode.
fn block_draw(
    block: &BlockWidget,
    label: Option<(&Text, &str)>,
    focus: Option<oag_ui::menu::block::Focus>,
    fraction: f32,
    frame: &Frame,
    layout: &Layout,
    out: &mut Vec<Draw>,
) {
    let grows = if block.selectable { fraction } else { 0.0 };
    let width = block.width + grows * oag_ui::menu::block::FOCUS_GROWTH;
    if let Some(art) = frame.blocks {
        let color = if focus.is_some() && block.selectable {
            block.active_color
        } else {
            block.color
        };
        oag_ui::menu::block::draw(
            &oag_ui::menu::block::Block {
                x: block.x,
                y: block.y,
                width,
                height: block.height,
                color: argb_to_rgba(color),
                landing: block.landing,
            },
            &art,
            (1.0, 1.0),
            out,
        );
        if block.selectable
            && let (Some(focus), Some(arrow), Some(tint)) = (focus, art.arrow, block.arrow_color)
            && focus.arrow_lit()
        {
            out.push(Draw::Sprite {
                rect: [
                    block.x + ARROW_OFFSET.0,
                    block.y + ARROW_OFFSET.1,
                    ARROW_SIZE,
                    ARROW_SIZE,
                ],
                uv: [
                    arrow.x as f32,
                    arrow.y as f32,
                    arrow.width as f32,
                    arrow.height as f32,
                ],
                color: argb_to_rgba(tint),
            });
        }
    }
    if let Some((text, content)) = label {
        let mut positioned = text.clone();
        positioned.x = block.x + LABEL_INSET.0;
        positioned.y = block.y + LABEL_INSET.1;
        positioned.scale = block.text_scale;
        out.push(text_draw(&positioned, content, layout));
    }
}

/// `EndRace Menu`'s own row targets, one per applicable option: each
/// option's own `<Block>` box as [`hd_menu_draw_list`] draws it, at its
/// resting width - the pointer-support counterpart to that draw, per this
/// project's own rule that a new screen ships with pointer support
/// alongside the pad. Width and height are the Block's own (`width="520"`
/// authored, `40` `Block_Construct`'s default), not constants of this
/// crate's.
#[must_use]
pub fn hd_menu_targets(model: &EndRaceMenu, layout: &Layout) -> Vec<Target> {
    let screen = &layout.screen;
    model
        .options()
        .iter()
        .enumerate()
        .filter_map(|(index, option)| {
            let block = find_block(screen, option.hd_block_name())?;
            Some(Target {
                index,
                rect: [block.x, block.y, block.width, block.height],
            })
        })
        .collect()
}
