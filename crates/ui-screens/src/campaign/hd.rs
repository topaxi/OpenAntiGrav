//! Wipeout HD/Fury's own `Grid Selection`/`Cell Selection` - the same two
//! screen *names* Pulse authors, read off a completely different file
//! (`Data\Plugins\Frontend\Gui\CellMode_Definition.xml`, `oag_hd::campaign::SCREEN_ENTRY`)
//! at a completely different resolution (1920x1080, not the PSP's 480x272 -
//! see [`super::Layout::read_authored`]).
//!
//! # The two screens turned out to be the same shape after all
//!
//! Whether HD's `FlyerSelection` screen combines what Pulse splits into two
//! was open before this file - reading `CellMode_Definition.xml` whole
//! settled it: HD's own
//! `<Screen type="FlyerSelection" name="Grid Selection">` nests a second
//! `<Screen type="CellSelection" name="Cell Selection">` inside it, and
//! `oag_ui::screen::Screens::collect` already flattens a nested `<Screen>`
//! into its own entry regardless of depth - so [`super::Layout::read_authored`]
//! reaches both screens by the same two names Pulse uses, unmodified. What
//! differs is only the *content*:
//!
//! - **`Grid Selection` is not a hex grid of tiers at all.** HD pages one
//!   tier ("Event") at a time through a `Flyer Left Arrow`/`Flyer Right
//!   Arrow` pair beside a 3-D flyer model, not four-hexes-per-page paging.
//!   [`hd_grid_draw_list`] reuses [`super::GridSelection`]'s model
//!   unchanged (its `index`/`step`/`selected_is_locked` are exactly the
//!   "one at a time" shape this screen needs) but draws none of
//!   [`super::draw::grid_draw_list`]'s hex-tile widgets, since this screen
//!   authors none.
//! - **`Cell Selection` is the same 7x5 staggered hex grid Pulse's own
//!   `Cell Selection` is**, `Bg_x_y`/`Outline_x_y`/`Lock_x_y`/`Medal_x_y`
//!   widget names included - HD just layers a fourth `Bg_x_y` texture
//!   (`Hexagon_HD_OUTLINE.mip`) under Pulse's three, plus (on `DATA06`'s own
//!   copy - see "The winning archive" below) a fifth, `bBg_x_y`, gated the
//!   same occupied-slot way as the other four. [`hd_cell_draw_list`] reuses
//!   [`super::CellSelection`] unchanged, [`super::hex_rect`] unchanged (it
//!   already searches for "Medal_" or "Outline_", both of which HD
//!   authors), and most of [`super::draw`]'s own per-widget draw helpers.
//!
//! # The winning archive is `DATA06`, not `DATA02` - switched 2026-09-27
//!
//! `CellMode_Definition.xml` exists on two archives that disagree
//! (`oag_hd::campaign::SCREEN_ENTRY`'s own doc), and `oag_game::campaign::load_hd`
//! used to read it through [`oag_assets::Archives::read_name`]'s ordinary
//! precedence, which lands on `DATA02`'s copy - the same "chosen, not
//! measured" reasoning [`oag_hd::campaign::SELECTION_SCREEN_ARCHIVE`]'s own
//! doc already flagged as unsettled for this file. It reads `DATA06`
//! unconditionally now, for both `Grid Selection` and `Cell Selection`, not
//! just `Campaign Selection`/`Grid Selection Fury`. Two converging lines,
//! neither alone conclusive, together confidence 85:
//!
//! 1. **The general load-order rule.** `docs/formats/hd-frontend.md`'s
//!    "`DATA00`'s copy is the live one" section measures `TTY.log`'s own
//!    archive load order directly: `data01, data02, data03, data04, data05,
//!    data06, data00` - `DATA00` loads *last* and its own `skin.xml` is
//!    confirmed live by a second, independent signal (a `sys_fs_stat` probe
//!    for a filename only its copy names). That is a last-wins overlay, not
//!    merely a discovery order - and `CellMode_Definition.xml` has no
//!    `DATA00` copy at all, so among the two archives that do carry it,
//!    `DATA06` loads after `DATA02` and wins the same way.
//! 2. **A direct, per-screen confirmation for this exact file.**
//!    `oag_hd::campaign::SCREEN_ENTRY`'s own doc already established
//!    `Campaign Selection`/`Grid Selection Fury` as `DATA06`-only, measured
//!    by `TTY.log` naming a screen that exists in no other copy. That alone
//!    only proves `DATA06`'s copy loads *somewhere* - not that `Cell
//!    Selection` specifically reads from it rather than `DATA02`'s (the two
//!    archives could in principle be read per-screen). A live RPCS3 frame on
//!    `grid8_3_2` (`Elimination`, `TARGET 200 (NOVICE)` row,
//!    `data/scratch/lane-hd/rpcs3-grid0-3-2/02-square.png`) settles that:
//!    it shows real gold/silver/bronze medal icons where `DATA02`'s own
//!    `Cell Selection` authors a plain grey `Subtitle_Arrow_HD.gtf` bullet
//!    and only `DATA06`'s differently-shaped `Target0/1/2 Medal` widget
//!    authors a medal icon at all - first-party, per-screen evidence for
//!    the same conclusion the general rule already gives.
//!
//! **Not a merge of the two copies.** Rule 1 above is single-file overlay,
//! not "read every archive and take whichever widget a screen happens to
//! have" - so this reads `DATA06`'s `Cell Selection` *whole*, including its
//! own `bBg_x_y` background layer, its differently-positioned `GridController`,
//! `Event`/`Track`/`Speed Class`/`Weapons` emblem layout and its
//! `RightColumnText` panel, not a graft of `DATA06`'s `Target0/1/2 Medal`
//! onto `DATA02`'s otherwise-unchanged screen - that combination exists on
//! no disc. Every one of those differences draws automatically once the
//! screen source switches: `oag_ui::screen`'s own widget collector already
//! folds `OffsetX`/`OffsetY` generically (nothing here is hand-positioned),
//! and a `<Bracket>` - the only new element either copy's `Cell Selection`
//! introduces - has no parser arm at all (`oag_ui_screens::campaign::selection`'s
//! own doc already established this for `Campaign Selection`), so the new
//! cosmetic borders around the emblems and the target row draw nothing
//! rather than something wrong.
//!
//! **Left explicitly unswitched, on the coordinator's own instruction**: the
//! sixteen `grid_00.xml`..`grid_15.xml` files `oag_hd::campaign::DEFINITION_ENTRY`
//! and `entry_name` still read through the same precedence as before (which
//! still reaches `DATA02`'s flat-schema copy for `grid_00`..`07`), even
//! though the same rule 1 above would argue `DATA06`'s per-difficulty copy
//! is equally live. Switching the screen's own widgets and the campaign's
//! own numbers are different-sized changes - the numbers feed
//! [`oag_tables::race_campaign::Cell::evaluate_medal`], a medal-law question
//! this lane did not verify end to end. See
//! `data/scratch/drive-2026-09-27/hd-targets.md` for the full writeup.
//!
//! # What is genuinely new
//!
//! - **A three-wide difficulty toggle**, `DifficultyButton` - see
//!   [`super::CellSelection::difficulty`]/[`super::CellSelection::cycle_difficulty`].
//!   Feeds [`oag_tables::race_campaign::Cell::targets_for_difficulty`],
//!   which already exists for this - nothing new in `oag_tables`.
//! - **The detail column reads a track's display name through
//!   [`oag_ui::language::CircuitNames`]**, not `strings.get_or_id` directly -
//!   `crate::campaign::draw::track_line`'s own doc says Pulse needs no such
//!   fold "unlike Wipeout HD's"; this is the screen that fold was for.
//! - **No hex tiles to click on `Grid Selection`, so its own pointer
//!   targets are chosen, not measured** - see [`hd_grid_targets`]'s own doc.
//!   `Cell Selection`'s own hex targets are unchanged from Pulse's
//!   [`super::pointer::cell_targets`] and are reused directly, not
//!   duplicated here.
//!
//! # What this does not draw, and says so
//!
//! - **The 3-D flyer card** is not drawn by this crate - it is a mesh, and
//!   `oag_game::flyer` puts it between this list's backdrop and its widgets,
//!   so `Flyer Pad Lock` and the arrows sit over it. [`flyer`](super::flyer)
//!   reads the widget and names the archive entries; see
//!   `docs/ui/campaign-screens.md`'s "The flyer behind `Grid Selection`" for
//!   what is authored and what is chosen. **Fury's eight cards are not drawn
//!   at all**: they are authored at another scale and as a lit box, and the
//!   camera fitted to the base campaign's flat cards does not apply.
//! - **The per-grid logo is drawn, for another grid's name**: `flyerlogo`
//!   authors `01_uplift\Logo.gtf` on every grid, and the executable's own
//!   `Data\FE\Flyers\%s\Logo.gtf` makes that a default - the box under the
//!   card shows the logo of the grid it is talking about, see `UnlockBox`.
//!   The two "same literal on every grid" readings this page carried before
//!   2026-09-30 were wrong about that.
//! - **`Record` (`MSC_CAMREC`)** - `Cell_SavedRecord`'s own value, which
//!   this build keeps no saved record for, the same absence
//!   `crate::campaign::draw::cell_draw_list`'s own `Line5`/`Line8` leave.

use oag_tables::race_campaign::{Cell, Difficulty, Medal, Mode};

use oag_ui::frontend::{Draw, Placed};
use oag_ui::language::{CircuitNames, StringTable};
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::pointer::{Pointer, contains};
use oag_ui::screen::Image;

use super::draw::{
    cell_title, centred_selector_draw, fill_draw, hex_slot_xy, image_draw, laps_line, medal_line,
    text_draw,
};
use super::pointer::{Target, What};
use super::{CellSelection, Event, GridSelection, GridSummary, Layout, hex_rect};

#[cfg(test)]
mod cell_field_tests;
#[cfg(test)]
mod tests;

pub mod cell_brackets;
pub mod cell_emblems;
mod cell_text;
mod medal;
mod unlock;
use unlock::{POINTS_GROUP, UnlockKind, unlock_box};
// `hd_target_title`/`hd_tinted_medal_draw` are called below; `hd_medal_frame`/
// `hd_difficulty_id` are not called directly here (only from inside `medal`
// itself) but need to be in scope for `mod tests`' own `use super::*` to
// reach them, the same way every other private helper this file declares
// directly already is.
#[allow(unused_imports)]
use medal::{hd_difficulty_id, hd_medal_frame, hd_target_title, hd_tinted_medal_draw};

/// HD's own page title idstring - `<Text name="ScreenTitle"><Values idstring="FE_RC">`,
/// distinct from Pulse's `FE_RACE_CAM` (`crate::campaign::draw::grid_draw_list`'s
/// own title). Not reached through `Layout` at all: `ScreenTitle` sits
/// outside both named `<Screen>` elements in the file, a sibling
/// `oag_ui::screen::Screens::collect` never attributes to either screen's
/// own widget list - see the module doc's nesting note. Drawn the same way
/// Pulse's own title is, through `skin.title_font()`, not through this
/// file's widget tree.
const TITLE_ID: &str = "FE_RC";

/// `Grid Selection`'s draw list: HD's flyer-paging screen, not a hex grid of
/// tiers - see the module doc.
///
/// `footer_overlay` is [`hd_cell_draw_list`]'s own parameter, unchanged:
/// `Campaign Selection`'s own RPCS3 frames
/// (`data/scratch/lane-hd-sel/rpcs3-campaign-selection/01-right-tap.png`/
/// `01-l1-tap.png`) show `NAVIGATION`/`CONFIRM`/`BACK` under a screen the
/// shared front-end root also draws the footer for - the
/// `NavigationController` is not gated to one screen - so this one gets it
/// too. **Correction, 2026-09-25**: an earlier pass cited
/// `data/scratch/lane-hd/rpcs3-grid0-3-2/00-default.png` here as `Grid
/// Selection Fury` showing a third `CHANGE DIFFICULTY` prompt; every frame
/// in that directory is actually `Cell Selection` (`grid8`'s own hex grid,
/// confirmed by `01-down.png`/`02-square.png`/`02-triangle.png`'s clean,
/// legible captures of the same session), and `Grid Selection`/`Grid
/// Selection Fury` author no `DifficultyButton` widget anywhere in
/// `CellMode_Definition.xml` at all - see
/// `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: the footer's button
/// glyphs" section for the full correction. This screen draws
/// `Confirm`/`Back` only, and that is the disc's own answer, not a gap.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts crate::campaign::draw::grid_draw_list takes, plus the footer \
              overlay hd_cell_draw_list already takes"
)]
pub fn hd_grid_draw_list(
    model: &GridSelection,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    footer_overlay: &[Draw],
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
    let (title_x, title_y, title_scale) = skin.title_at();
    layers.chrome.push(Draw::title(
        skin.title_font(),
        title_x,
        title_y,
        title_scale,
        skin.title_color(frame.ink),
        strings.get_or_id(TITLE_ID).to_string(),
    ));
    layers.chrome.extend_from_slice(footer_overlay);

    let screen = &layout.screen;
    let mut out = Vec::new();
    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }
    let locked = model.selected_is_locked();
    let unlock = unlock_box(model);
    for image in &screen.images {
        // `Flyer Pad Lock` is HD's own single lock glyph for this screen -
        // one padlock over the flyer, not one `Lock_n_0` per tile, since
        // there are no tiles. Gated the same three-term way
        // `super::CellSelection::cell_shows_lock` is - see
        // `super::GridSelection::selected_is_locked`'s own doc for why this
        // is "assumed to transfer to HD, unmeasured" on the exact glyph, not
        // only the rule it reuses from Pulse.
        if image.name.as_deref() == Some("Flyer Pad Lock") && !locked {
            continue;
        }
        // The two bullet arrows beside the points figures, which a locked
        // tier does not show at all - see `POINTS_GROUP`.
        if locked && image.src.ends_with("Subtitle_Arrow_HD.gtf") {
            continue;
        }
        // `flyerlogo` authors `01_uplift\Logo.gtf` as a literal on every
        // grid; the executable's own `Data\FE\Flyers\%s\Logo.gtf` makes
        // that a default, and the `%s` is the grid the unlock box names -
        // see `UnlockBox`.
        if image.name.as_deref() == Some("flyerlogo") {
            let Some(named) = unlock.as_ref().and_then(|note| note.logo) else {
                continue;
            };
            let mut logo = image.clone();
            logo.src = super::flyer::logo_entry(named);
            let Some(placed) = sprites(&logo.src) else {
                continue;
            };
            out.push(image_draw(&logo, placed));
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    let Some(selected) = model.selected() else {
        layers.body = out;
        return layers;
    };
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        if locked && POINTS_GROUP.contains(&name) {
            continue;
        }
        let content = match name {
            "EventNum" | "GridNum" => Some(event_counter(model.index(), model.grids().len())),
            // **Two bare numbers, not the fractions this drew before
            // 2026-09-30.** The XML's own placeholders (`"00/16"`,
            // `"000/110"`) read as fractions, and this screen drew
            // `gold_medals/cell_count` and `points_earned/max_points`
            // from them. Every settled RPCS3 frame of an unlocked tier
            // reads `POINTS ACHIEVED` over a bare `0` and `TOTAL POINTS
            // AVAILABLE` over a bare `18` (HD `grid0`) or `21` (Fury
            // `grid8`) - the tier's `3 * cells`, `Grid_PointsPossible`.
            // The first is points earned on the label's own terms; the
            // widget is named `Medals Title`, and a fresh profile reads `0`
            // under either, so **which of the two it is stays unmeasured**.
            "Points" => Some(selected.points_earned.to_string()),
            "TotPoints" => Some(selected.max_points.to_string()),
            "Required" => unlock
                .as_ref()
                .filter(|note| note.kind == UnlockKind::ToUnlock)
                .and_then(|note| {
                    text.string
                        .as_deref()
                        .map(|s| s.replacen("%d", &note.points.to_string(), 1))
                }),
            "Required Previous" => unlock
                .as_ref()
                .filter(|note| note.kind == UnlockKind::NeededIn)
                .map(|note| note.needed_in(strings)),
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        out.push(text_draw(text, &content, layout));
    }
    layers.body = out;
    layers
}

/// `"Event {:02}/{:02}"` - `EventNum`'s own template (`"Event 01/08"`) and
/// `GridNum`'s identical copy on `Cell Selection`, 1-based. Confidence 95:
/// the literal template string on both widgets.
fn event_counter(index: usize, total: usize) -> String {
    format!("Event {:02}/{:02}", index + 1, total)
}

/// What `Cell Selection` draws that its screen file and the model do not
/// carry: the next grid and the circuits' emblems.
#[allow(missing_debug_implementations, reason = "holds a closure")]
pub struct CellArt<'a> {
    /// The next grid's `FlyerName`, whose logo the unlock box shows.
    pub next_flyer: Option<&'a str>,
    /// A circuit id's white emblem, as the `src` the sheet holds it under.
    pub track_emblem: &'a dyn Fn(&str) -> Option<String>,
}

/// `Cell Selection`'s draw list: the same 32-position staggered hex grid
/// Pulse's own `Cell Selection` is, plus HD's wider detail column and
/// difficulty toggle - see the module doc.
///
/// `footer_overlay` is [`crate::campaign::draw::cell_draw_list`]'s own
/// parameter, unchanged: HD's shared front-end root carries the identical
/// `NavigationController` shape Pulse's does (see
/// `oag_ui_screens::campaign::footer::NavigationLegend`'s own doc), and neither
/// title's `CellMode_Definition.xml` authors a per-screen gate on `Cell
/// Selection`, so this draws it on the same terms Pulse's own screen does.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same shape crate::campaign::draw::cell_draw_list takes, plus the two facts \
              (circuit_names, the enclosing grid's own summary) HD's detail column needs that \
              Pulse's never did"
)]
pub fn hd_cell_draw_list(
    model: &CellSelection,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    circuit_names: &CircuitNames,
    grid_index: usize,
    grid_count: usize,
    grid_summary: &GridSummary,
    art: &CellArt,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    footer_overlay: &[Draw],
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
    let (title_x, title_y, title_scale) = skin.title_at();
    layers.chrome.push(Draw::title(
        skin.title_font(),
        title_x,
        title_y,
        title_scale,
        skin.title_color(frame.ink),
        strings.get_or_id(TITLE_ID).to_string(),
    ));
    layers.chrome.extend_from_slice(footer_overlay);

    let screen = &layout.screen;
    let mut out = Vec::new();
    // The box under the hex field, as `Grid Selection` draws it for an
    // unlocked tier: how many points the tier still needs to open the next
    // grid, over the next grid's logo. Nothing once the tier's own figure
    // is met or when there is no next grid.
    out.extend(cell_brackets::draws(&screen.brackets, sprites));
    let remaining = grid_summary
        .required_points
        .saturating_sub(grid_summary.points_earned);
    let unlock_logo = art.next_flyer.filter(|_| remaining > 0);
    let occupied: Vec<(u32, u32)> = model.cells().iter().filter_map(Cell::grid_coords).collect();
    let selected_coords = model.selected().and_then(Cell::grid_coords);
    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        if let Some(name) = image.name.as_deref() {
            // `bBg_x_y` and `Bg_x_y` draw on all 32 slots, the empty ones too: the
            // original's field is a black hex honeycomb with the grid's own
            // hexes lit on it (RPCS3, `Cell Selection` of `09_blitzed`).
            if let Some((x, y)) = hex_slot_xy(name, "Outline_")
                .or_else(|| hex_slot_xy(name, "Lock_"))
                .or_else(|| hex_slot_xy(name, "Medal_"))
                && !occupied.contains(&(x, y))
            {
                continue;
            }
            if let Some((x, y)) = hex_slot_xy(name, "Medal_")
                && model.medal_at(x, y).is_none()
            {
                continue;
            }
            if let Some((x, y)) = hex_slot_xy(name, "Lock_")
                && !model.cell_shows_lock(x, y)
            {
                continue;
            }
            // `Target0/1/2 Medal` and their shared `Target Title Arrow`
            // bullet - `DATA06.PSARC`'s own replacement for `DATA02`'s
            // `Target0/1/2 Image` grey-arrow widgets (see the module doc's
            // "the winning archive" note). Deferred to the second image pass
            // below, gated on `targets_visible`, the same two-pass shape the
            // pre-switch code already used for `Target0/1/2 Image`.
            if matches!(name, "Target0 Medal" | "Target1 Medal" | "Target2 Medal")
                || name == "Target Title Arrow"
            {
                continue;
            }
            // `flyerlogo` authors `01_uplift\Logo.gtf`; the box names the next grid's
            // - the same rule `hd_grid_draw_list` draws it by.
            if name == "flyerlogo" {
                let Some(next) = unlock_logo else { continue };
                let mut logo = image.clone();
                logo.src = super::flyer::logo_entry(next);
                if let Some(placed) = sprites(&logo.src) {
                    out.push(image_draw(&logo, placed));
                }
                continue;
            }
            if name == "NextPoints Arrow" && unlock_logo.is_none() {
                continue;
            }
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        if image.name.as_deref() == Some("Selector")
            && let Some((sx, sy)) = selected_coords
            && let Some(hex) = hex_rect(screen, sx as usize, sy as usize, sprites)
        {
            out.push(centred_selector_draw(image, placed, hex));
            continue;
        }
        if let Some((x, y)) = image.name.as_deref().and_then(|n| hex_slot_xy(n, "Medal_")) {
            if let Some(medal) = model.medal_at(x, y) {
                // **Falls back to `Medium`, not `Hard`.** A medal with no
                // recorded difficulty was evaluated by the pre-per-difficulty
                // path (`Cell::evaluate_medal`, always against the medium
                // rung - `Cell::gold`/`silver`/`bronze` alias it), so
                // crediting it at Hard here would show a harder badge than
                // the disc's own `SaveData_MigrateCellMedalsToHardElite`
                // would for a `Race`/`Elimination` result: that migration
                // only touches `TimeTrial`/`SpeedLap`/`Zone`, the three
                // modes whose raw value does not depend on AI skill - an
                // AI-independent time can honestly be credited at Hard, a
                // placement against Hard's own tougher AI cannot. `Medium`
                // is the rung the value was actually judged against, not a
                // guess. See [`super::CellSelection::difficulty_at`]'s own
                // doc for when this triggers.
                let difficulty = model.difficulty_at(x, y).unwrap_or(Difficulty::Medium);
                out.push(hd_tinted_medal_draw(image, placed, medal, difficulty));
            }
            continue;
        }
        out.push(image_draw(image, placed));
    }
    let Some(cell) = model.selected() else {
        layers.body = out;
        return layers;
    };

    // **Widened to unconditional, 2026-09-27, measured against three live
    // RPCS3 frames - the `Weapons` row a few lines below was widened the
    // same way, on the same evidence.** The pre-switch gate here
    // (`TimeTrial | Zone | Elimination | SpeedLap`) excluded `Race`, and
    // `grid8_3_1`'s own frame
    // (`data/scratch/drive-2026-09-27/hd-targets/fury-race-3-1.png`)
    // contradicts that directly: a real `TARGET (NOVICE)` row with
    // `1ST`/`2ND`/`3RD` beside gold/silver/bronze icons, on a `Race` cell.
    // Combined with `grid8_3_2` (`Elimination`) and `grid8_4_2` (`Speed
    // Lap`) both also showing it, three of three sampled cells across three
    // different modes draw the row - no mode sampled this pass hides it, and
    // every cell on the disc authors a real `Gold`/`Silver`/`Bronze` triple
    // for [`hd_target_value`] to read regardless of mode (required fields,
    // `race_campaign::required_i64`), so there is no structural reason a
    // mode would have nothing to show. `Tournament`/`Head2Head`/`Mode::Other`
    // are not separately captured - extended by the same "nothing found
    // argues for hiding it" reasoning, not a fourth frame. Kept as a named
    // `bool` (unlike `Weapons`'s own single match arm) since this one gates
    // five separate sites below.
    out.extend(cell_emblems::draws(
        &screen.slots,
        cell,
        art.track_emblem,
        sprites,
    ));
    let targets_visible = true;
    let targets = cell.targets_for_difficulty(model.difficulty());

    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        let content = match name {
            "EventNum" | "GridNum" => Some(event_counter(grid_index, grid_count)),
            // `EPoints Title`'s own `<Values string="00/16 POINTS">` is a
            // dummy placeholder, not the runtime format - measured directly
            // against a live RPCS3 frame on this exact cell
            // (`data/scratch/lane-hd/rpcs3-grid0-3-2/02-square.png`,
            // `grid8_3_2` selected, a fresh zero-medal profile): the real
            // screen reads `"0/21 POINTS"`, not zero-padded, and `21` is
            // `grid_summary.max_points` (`3 * cell_count`, `Grid_PointsPossible`),
            // not `cell_count` itself - `gold_medals`/`cell_count` was this
            // widget's own earlier, wrong reading, confused with `Grid
            // Selection`'s different `Points`/`Medals Title` field (that one
            // genuinely is `gold_medals`/`cell_count`, unrelated to this
            // screen's own `POINTS` line). Confidence 90: one live capture on
            // the exact cell used here, not yet cross-checked against a
            // second cell's own numbers.
            "EPoints Title" => Some(format!(
                "{}/{} POINTS",
                grid_summary.points_earned, grid_summary.max_points
            )),
            "Event" => Some(cell_title(cell, strings)),
            "Track" => Some(hd_track_line(cell, circuit_names, strings)),
            "Speed Class" if cell.mode != Mode::Zone => Some(cell.class.clone()),
            "Speed Class" => None,
            "Weapons" => {
                let id = if cell.weapons { "FE_ON" } else { "FE_OFF" };
                Some(strings.get_or_id(id).to_string())
            }
            // `DATA02`'s own `Target0/1/2 Title` (idstring
            // `IG_HUD_GOLD`/`SILVER`/`BRONZE`) is gone on `DATA06`'s copy -
            // replaced by one shared `Target Title` header. See
            // [`hd_target_title`]'s own doc for what the header composes.
            "Target Title" if targets_visible => {
                Some(hd_target_title(cell, strings, model.difficulty()))
            }
            "Target0" if targets_visible => Some(hd_target_value(targets.gold, cell, strings)),
            "Target1" if targets_visible => Some(hd_target_value(targets.silver, cell, strings)),
            "Target2" if targets_visible => Some(hd_target_value(targets.bronze, cell, strings)),
            "Target Title" | "Target0" | "Target1" | "Target2" => None,
            // `RC_POINTSTOUNL`'s own template carries a raw `%d` this file
            // has no game-side `printf` to substitute - `strings.get_or_id`
            // resolves the idstring's text verbatim, `%d` included, unless
            // this fills it in. **Chosen, not measured**: which number the
            // template wants (the enclosing grid's own points still needed,
            // a different grid's, or something else) is not settled by
            // reading the file alone, so this substitutes the one concrete
            // figure this screen already carries -
            // `grid_summary.required_points` - rather than leave the raw
            // placeholder on screen. Also unmeasured: whether the widget is
            // gated on the grid actually being short of that many points,
            // which this build shows unconditionally.
            "NextPoints" => unlock_logo.and_then(|_| {
                text.string
                    .as_deref()
                    .map(|s| s.replacen("%d", &remaining.to_string(), 1))
            }),
            // Omega's own `Cell Selection` authors a `RecordsButton` (`FE_RECORDS`,
            // glyph `RecordsButtonIcon`) at exactly `DifficultyButton`'s `x=944
            // y=994`, so the two cannot both be visible. This build models the
            // difficulty toggle and has no leaderboard behind the other, the same
            // reason `endrace::hd` drops HD's `RecordsCycle` - **chosen, not
            // measured**: which of the two the original shows on this screen
            // was not watched (no PS4 emulator). HD authors no such widget.
            "RecordsButton" | "RecordsButtonIcon" => None,
            "RC Laps" => Some(laps_line(cell, strings)),
            // `Cell_SavedRecord` - no saved record kept by this build, the
            // same absence `crate::campaign::draw::cell_draw_list` leaves
            // `Line5`/`Line8` in. See the module doc.
            "Record" => Some(model.selected_record().map_or_else(
                || strings.get_or_id("MSC_NONE").to_string(),
                hd_format_centiseconds,
            )),
            "Points" => Some(format!(
                "{}/{}",
                model.selected_medal().map_or(0, |m| m.points()),
                oag_tables::race_campaign::Medal::Gold.points()
            )),
            "Best" => Some(medal_line(model.selected_medal(), None, strings)),
            // The square-button prompt - see [`hd_difficulty_button_line`]'s
            // own doc for the `RB_AI_DIF`/`RB_DIF` split and which modes get
            // computed text at all.
            "DifficultyButton" => {
                hd_difficulty_button_line(&cell.mode, model.difficulty(), strings)
                    .or_else(|| text.string.clone())
            }
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        out.push(cell_text::draw(text, &content, layout));
    }
    for image in &screen.images {
        let Some(name) = image.name.as_deref() else {
            continue;
        };
        let target_medal_tier = match name {
            "Target0 Medal" => Some(Medal::Gold),
            "Target1 Medal" => Some(Medal::Silver),
            "Target2 Medal" => Some(Medal::Bronze),
            _ => None,
        };
        let is_target_widget = target_medal_tier.is_some() || name == "Target Title Arrow";
        if is_target_widget && targets_visible {
            let Some(placed) = sprites(&image.src) else {
                continue;
            };
            match target_medal_tier {
                // `Target{n} Medal` authors its own `width`/`height`/`u`/`v`/
                // `TxtrWidth`/`TxtrHeight` crop of `Hexmedal_HD.gtf`
                // (measured directly off `DATA06`'s own XML - see the module
                // doc's medal-widget table), but only for the `v=0` (easy/
                // novice) block - it never varies by [`super::CellSelection::difficulty`],
                // because the widget is static XML and the difficulty rung
                // it shows is runtime state. [`hd_medal_frame`]'s own crop,
                // computed for whichever rung is currently browsed, replaces
                // it the same way [`hd_tinted_medal_draw`] already replaces
                // `Medal_{x}_{y}`'s own unauthored one - see that function's
                // doc for why this needs no tint either.
                Some(tier) => out.push(hd_tinted_medal_draw(
                    image,
                    placed,
                    tier,
                    model.difficulty(),
                )),
                // `Target Title Arrow` carries no tier of its own - draws at
                // its authored crop unchanged.
                None => out.push(image_draw(image, placed)),
            }
        }
    }
    layers.body = out;
    layers
}

/// `Target0`/`Target1`/`Target2`'s own value text - widened from the shared
/// [`super::draw::target_value`] (still correct for Pulse, untouched here) once a real
/// capture showed HD's own screen disagreeing with it on two counts a
/// generic `Time Trial`/`Speed Lap`-only split can't express:
///
/// - **A placement target reads as an ordinal, not a bare number.**
///   `grid8_3_1` (`Race`) and `grid8_3_2` (`Elimination`) both show `1ST`/
///   `2ND`/`3RD` beside the gold/silver/bronze icons
///   (`data/scratch/drive-2026-09-27/hd-targets/fury-race-3-1.png`,
///   `data/scratch/lane-hd/rpcs3-grid0-3-2/02-square.png`), never the raw
///   `1`/`2`/`3` [`Cell::targets_for_difficulty`] actually carries for those
///   cells (`grid_08.xml`'s own `EasyGold Target="1"` and siblings, on
///   every `Race`/`Elimination`/`NitroBattle`/`Detonator` cell in the
///   corpus alike - a placement rank, not a distinct value per tier, see
///   [`Cell::gold`]'s own doc). `Tournament`/`Head2Head` share the doc's
///   own "finishing position" reading with `Race` and get the same
///   treatment - chosen by that documented equivalence, not separately
///   captured. `Mode::Other` (`NitroBattle`/`Detonator`) is included too:
///   both author the identical `1`/`2`/`3` triple, so a bare-number
///   fallback would draw a raw placement rank as if it were a real
///   magnitude on those two as well - not captured, but the same evidence
///   that argues for `Race`/`Elimination` argues for them.
/// - **`Zone` is deliberately excluded from that widening.** Its own gold/
///   silver/bronze are real, distinct per-tier values (`grid_00.xml`'s
///   `EasyGold Target="10"`/`EasySilver Target="9"`/`EasyBronze
///   Target="8"`, three different numbers) - [`Cell::gold`]'s own doc reads
///   it as "a zone count for Zone", not a placement, so it keeps the plain
///   `value.to_string()` fallback [`super::draw::target_value`] already gives it. No
///   `Zone` capture exists to confirm the on-screen text directly.
/// - **`Time Trial`/`Speed Lap` keep a time format, but not [`super::draw::target_value`]'s
///   own `M:SS.CC`.** `grid8_4_2`'s own frame reads `"0.47.50"` - every
///   separator a period, no colon - see [`hd_format_centiseconds`]'s own
///   doc. [`super::draw::target_value`] is left untouched for Pulse, which this pass has
///   no capture to re-verify either way.
fn hd_target_value(value: i64, cell: &Cell, strings: &StringTable) -> String {
    match cell.mode {
        Mode::TimeTrial | Mode::SpeedLap => hd_format_centiseconds(value),
        Mode::Zone => value.to_string(),
        _ => hd_ordinal(value, strings),
    }
}

/// `1`/`2`/`3` to `"1ST"`/`"2ND"`/`"3RD"` through
/// `IG_HUD_1ST`/`IG_HUD_2ND`/`IG_HUD_3RD`
/// (`data/scratch/lane-hd-sel/english-entries-data06.xml`) -
/// [`hd_target_value`]'s own placement-mode arm. A value outside `1..=3`
/// falls back to the bare number: every placement target measured on this
/// disc is exactly one of the three (see [`hd_target_value`]'s own doc), so
/// this is a defensive fallback with nothing observed to exercise it, not a
/// second reading of what a fourth-or-lower target means.
fn hd_ordinal(value: i64, strings: &StringTable) -> String {
    let id = match value {
        1 => "IG_HUD_1ST",
        2 => "IG_HUD_2ND",
        3 => "IG_HUD_3RD",
        _ => return value.to_string(),
    };
    strings.get_or_id(id).to_string()
}

/// Centiseconds to `M.SS.CC` - **`Time Trial`/`Speed Lap`'s own separator on
/// Wipeout HD/Fury is a period throughout, not [`super::draw::format_centiseconds`]'s
/// `M:SS.CC` colon**, measured directly: `grid8_4_2`'s own live frame
/// (`data/scratch/drive-2026-09-27/hd-targets/fury-speedlap-4-2.png`) reads
/// `"0.47.50"`/`"0.49.50"`/`"0.51.50"` for its own gold/silver/bronze lap
/// targets (`EasyGold Target="4750"`/`EasySilver Target="4950"`/`EasyBronze
/// Target="5150"`, `grid_08.xml`) - every one all-period. Kept as its own
/// function rather than changing [`super::draw::format_centiseconds`]: that one is
/// shared with Pulse, which this pass has no capture to re-verify against,
/// and per-title RE evidence should not cross titles on an assumption.
/// Confidence 90 for HD: one direct capture, three values on it agreeing.
fn hd_format_centiseconds(value: i64) -> String {
    let value = value.max(0);
    let minutes = value / 6000;
    let seconds = (value / 100) % 60;
    let centis = value % 100;
    format!("{minutes}.{seconds:02}.{centis:02}")
}

/// `Track`'s own resolution - the circuit's display name through
/// [`CircuitNames`], falling back to the string table and then the raw id.
/// **Deliberately not `oag_raceplay::catalogue::label`**: that appends `FE_REVERSE`
/// where a reversed circuit shares its forward twin's name, and HD's own
/// screens never spell a direction in a name (`docs/formats/hd-frontend.md`),
/// so a reversed cell reads the bare circuit name here as on Track Select.
/// `Tournament` cells (`{n} Races`) are unaffected, the same reading
/// `crate::campaign::draw::track_line` already gives them.
fn hd_track_line(cell: &Cell, circuit_names: &CircuitNames, strings: &StringTable) -> String {
    if cell.mode == Mode::Tournament {
        return format!("{} Races", cell.tournament_tracks.len());
    }
    let Some(id) = cell.track.as_deref() else {
        return String::new();
    };
    circuit_names
        .get(id)
        .unwrap_or_else(|| strings.get_or_id(id))
        .to_string()
}

/// `DifficultyButton`'s own text on HD/Fury's `Cell Selection` - unlike
/// Pulse's identically-named widget ([`super::draw::difficulty_button_line`],
/// left untouched: this is a separate, HD-only function, not a shared one),
/// HD authors *two* distinct idstrings for it - `RB_AI_DIF` ("AI
/// DIFFICULTY") and `RB_DIF` ("DIFFICULTY"), both real `entries.xml` rows on
/// `DATA04.PSARC` (`/data/plugins/languages/american/entries.xml`), neither
/// referenced from `CellMode_Definition.xml` at all (checked directly on
/// `DATA06`'s copy: the widget authors one unconditional literal,
/// `string="Change Difficulty"` - the choice is the executable's, at draw
/// time, not an XML-authored per-mode swap). `docs/ui/campaign-screens.md`'s
/// "Which block is which difficulty" section (2026-09-28) reads the split
/// directly off three independent RPCS3 boots: `grid8_3_1` (`Race`, Talon's
/// Junction) shows `AI DIFFICULTY (<rung>)` on two boots, `grid8_3_2`
/// (`Elimination`, The Amphiseum) shows bare `DIFFICULTY (<rung>)` on one -
/// confidence 90 for those two modes specifically, per that section's own
/// rubric.
///
/// **The `RB_AI_DIF`/`RB_DIF` call site itself was not found in Ghidra this
/// pass** - see `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s
/// "The TOC-xref trap" section for why `get_xrefs_to` on either string
/// resolves to an unrelated function on this binary, and what was checked
/// instead. What *was* found and independently confirms the mode split's
/// shape: `CellSelection_UpdateDifficultyButton_q` (`0x0021db80`, the
/// widget's own square-press/update handler, found via a real `bl` xref
/// chasing `DifficultyRC`'s persist call) gates its whole cycle-the-rung
/// behaviour on `GameState+0xe0` (the selected cell's mode) being one of
/// `Race`/`TimeTrial`/`Zone`/`Elimination`/`Head2Head`/`SpeedLap` or the two
/// raw ordinals `0xd`/`0xe` (HD's `NitroBattle`/`Detonator` spellings,
/// [`Mode::Other`]) - and **not** `Tournament`. That match is reused
/// verbatim here for *whether* this function returns a computed line at all;
/// [`Mode::CustomGrid`]/[`Mode::AiRace`] are excluded the same way, on the
/// same "never authored by a shipped grid" grounds
/// [`Mode::ALL`](oag_tables::race_campaign::Mode::ALL)'s own doc already
/// gives them, not fresh Ghidra evidence.
///
/// The two-way split within that active set - `Race`/`Head2Head` get
/// `RB_AI_DIF`, everything else `RB_DIF` - extends the two measured modes by
/// `DATA04.PSARC`'s own `UPDATE_ANNOUNCEMENT` string (`entries.xml`):
/// `"Rather than having difficulty options for just Single Race and
/// Tournament events in campaign, it is now possible... to select Novice,
/// Skilled or Elite difficulty options for all events"` - i.e. `AI
/// DIFFICULTY` (opponent skill) is the original `Single Race`/`Tournament`-
/// only mechanic, and bare `DIFFICULTY` (a target threshold) is the later
/// addition covering every other mode. `Head2Head` is grouped with `Race`
/// by the same "single AI opponent, opponent skill matters" reading
/// `hd_target_value`'s own doc already uses to group the two for their
/// placement-target wording - **chosen, not measured**.
/// `TimeTrial`/`Zone`/`SpeedLap`/[`Mode::Other`] are grouped with the
/// confirmed-bare `Elimination` reading by the announcement's own
/// "target-threshold" description and by `0x0021db80`'s own gate treating
/// `0xd`/`0xe` no differently from `Elimination`'s `8` - confidence 60.
///
/// **`Tournament` is deliberately left out of the computed text**, even
/// though the announcement groups it with `Race` by name: `0x0021db80`'s
/// gate excludes ordinal `4` outright, so a `Tournament` cell's own
/// `DifficultyButton` is not confirmed to be interactive on this screen at
/// all (`Tournament` may route difficulty through a separate screen this
/// pass did not read) - showing computed `AI DIFFICULTY` text there would
/// contradict, not extend, the one piece of measured evidence this pass has
/// for that mode. `Cell Selection` keeps drawing the disc's own authored
/// `"Change Difficulty"` for it instead, the same honest fallback the
/// pre-existing catch-all already draws for every mode.
#[must_use]
fn hd_difficulty_button_line(
    mode: &Mode,
    difficulty: Difficulty,
    strings: &StringTable,
) -> Option<String> {
    let ai_difficulty = match mode {
        Mode::Race | Mode::Head2Head => true,
        Mode::TimeTrial | Mode::Zone | Mode::Elimination | Mode::SpeedLap | Mode::Other(_) => false,
        Mode::Tournament | Mode::CustomGrid | Mode::AiRace => return None,
    };
    let id = if ai_difficulty { "RB_AI_DIF" } else { "RB_DIF" };
    Some(format!(
        "{} ({})",
        strings.get_or_id(id),
        strings.get_or_id(hd_difficulty_id(difficulty))
    ))
}

/// `Grid Selection`'s pointer targets: the paging arrows, and a confirm
/// region over the flyer.
///
/// **The confirm rect is the drawn card's own screen rectangle when the
/// caller has one** (`card`, `[x, y, width, height]` in this screen's grid,
/// which `oag_game::flyer::Flyers::card_rect` projects from the card the
/// renderer actually draws), so a click lands on what the player sees.
///
/// **With no card drawn it is the page's whole content band**, between the
/// header rule and the footer rule ([`CONFIRM_BAND`]). Omega draws no card at
/// all, and the rect this used to fall back to - `Flyer Pad Lock`'s authored
/// `x="934" y="304" width="512" height="512"`, the padlock that sits centred
/// on a card - is then an invisible target: a player with only a mouse could
/// click anywhere on the page but one unmarked square and never leave it.
/// There is only ever one tier on screen, so a click on the page confirming
/// it is the two-tap idiom's second tap with nothing else to select.
/// **Chosen, not measured**, either way: the original's click region is not
/// read. The arrows come first in the list so they win where they overlap.
#[must_use]
pub fn hd_grid_targets(
    layout: &Layout,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    card: Option<[f32; 4]>,
) -> Vec<Target> {
    let screen = &layout.screen;
    let mut out = Vec::new();
    for image in &screen.images {
        let what = match image.name.as_deref() {
            Some("Flyer Left Arrow") => What::Previous,
            Some("Flyer Right Arrow") => What::Next,
            _ => continue,
        };
        if let Some(rect) = image_rect(image, sprites) {
            out.push(Target { what, rect });
        }
    }
    out.push(Target {
        what: What::Hex(0),
        rect: card.unwrap_or(CONFIRM_BAND),
    });
    out
}

/// Where a click confirms `Grid Selection` when no flyer card is drawn: the
/// page between its header and footer rules, in the 1920x1080 grid this
/// screen is authored in. Chosen, not measured.
pub const CONFIRM_BAND: [f32; 4] = [0.0, 100.0, 1920.0, 900.0];

/// Where an image is drawn - the same fallback [`super::pointer::grid_targets`]
/// applies, duplicated rather than exported since [`super::pointer`]'s own
/// copy is private.
fn image_rect(image: &Image, sprites: &dyn Fn(&str) -> Option<Placed>) -> Option<[f32; 4]> {
    let placed = sprites(&image.src);
    let width = image
        .width
        .or_else(|| placed.map(|placed| placed.width as f32))?;
    let height = image
        .height
        .or_else(|| placed.map(|placed| placed.height as f32))?;
    Some([image.x, image.y, width, height])
}

impl GridSelection {
    /// Consumes a tick of pointer input on HD's own `Grid Selection` -
    /// [`super::pointer::GridSelection::pointer`]'s own shape, against
    /// [`hd_grid_targets`] rather than Pulse's hex-tile targets, and with
    /// [`What::Hex`] read as "click the flyer to confirm" instead of "click
    /// a tile to select it", since there is only ever one tier on screen.
    pub fn hd_pointer(&mut self, pointer: &Pointer, targets: &[Target]) -> Vec<Event> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        if pointer.scroll != 0 {
            out.extend(self.step(pointer.scroll.signum()));
        }
        if pointer.clicked
            && let Some(at) = pointer.at
            && let Some(target) = targets.iter().find(|target| contains(target.rect, at))
        {
            match target.what {
                What::Previous => out.extend(self.step(-1)),
                What::Next => out.extend(self.step(1)),
                What::Hex(_) => out.push(Event::Confirmed),
            }
        }
        if pointer.back {
            out.push(Event::Back);
        }
        out
    }
}

/// `DifficultyButton`'s own rect, for a click. **Invented**: the widget is a
/// `Text`, which authors no width or height at all - `x="944" y="994"` is
/// its baseline alone. Sized generously enough to cover the label and its
/// `δ` icon (`x="900"`) rather than measured off a capture. Chosen, not
/// measured. `pub` so a caller can compute it in the same immutable-borrow
/// phase it computes [`super::pointer::cell_targets`] in - see [`CellSelection::hd_pointer`]'s
/// own doc for why that split matters here.
#[must_use]
pub fn difficulty_button_rect(screen: &oag_ui::screen::Screen) -> Option<[f32; 4]> {
    let text = screen
        .texts
        .iter()
        .find(|text| text.name.as_deref() == Some("DifficultyButton"))?;
    Some([text.x - 50.0, text.y - 10.0, 220.0, 40.0])
}

impl CellSelection {
    /// Consumes a tick of pointer input on HD's own `Cell Selection` -
    /// [`super::pointer::CellSelection::pointer`]'s own hex handling
    /// unchanged (`targets`, built the same way [`super::pointer::cell_targets`] already
    /// builds Pulse's), plus a click on `difficulty_rect`
    /// ([`difficulty_button_rect`]) cycling the difficulty rung the same way
    /// `Square` does on the pad. `Self::pointer` already swallows every
    /// click/back while `Cell Help` is open, so nothing extra is needed for
    /// that here.
    ///
    /// **Takes pre-built `targets`/`difficulty_rect` rather than a `Layout`
    /// and a sprite lookup**, unlike [`GridSelection::hd_pointer`] - the
    /// caller (`oag_game::main::session::pointer::campaign_pointer`) already
    /// has to borrow `campaign.cell_layout()`/`campaign.sprites`
    /// immutably to build them *before* taking `&mut campaign.screen`, the
    /// same two-phase split that function's own Pulse arm already uses; a
    /// `Layout`/sprite-lookup parameter here would ask for both borrows at
    /// once, which the borrow checker refuses across that split.
    pub fn hd_pointer(
        &mut self,
        pointer: &Pointer,
        targets: &[Target],
        difficulty_rect: Option<[f32; 4]>,
    ) -> Vec<Event> {
        if pointer.is_idle() {
            return Vec::new();
        }
        if !self.help_open()
            && pointer.clicked
            && let Some(at) = pointer.at
            && let Some(rect) = difficulty_rect
            && contains(rect, at)
        {
            self.cycle_difficulty();
            return vec![Event::DifficultyChanged];
        }
        self.pointer(pointer, targets)
    }
}
