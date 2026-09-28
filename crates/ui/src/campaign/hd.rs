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
//! introduces - has no parser arm at all (`oag_ui::campaign::selection`'s
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
//!   [`crate::language::CircuitNames`]**, not `strings.get_or_id` directly -
//!   `crate::campaign::draw::track_line`'s own doc says Pulse needs no such
//!   fold "unlike Wipeout HD's"; this is the screen that fold was for.
//! - **No hex tiles to click on `Grid Selection`, so its own pointer
//!   targets are invented outright** - see [`hd_grid_targets`]'s own doc.
//!   `Cell Selection`'s own hex targets are unchanged from Pulse's
//!   [`super::pointer::cell_targets`] and are reused directly, not
//!   duplicated here.
//!
//! # What this does not draw, and says so
//!
//! - **The 3-D flyer model itself** (`Data\FE\Flyers\00_flyer.vex`, the
//!   `<Flyer name="FlyerModel">` widget). This crate draws a flat
//!   [`crate::frontend::Draw`] list, not a mesh scene - wiring a `.vex`
//!   flyer through `oag_render` is out of this pass's scope, and nothing
//!   here pretends otherwise: the screen behind the hex grid and the
//!   difficulty column is left at the frame's own backdrop.
//! - **The per-grid flyer logo** (`flyerlogo`, `Data\FE\Flyers\01_uplift\Logo.gtf`
//!   on both screens) - the path is the same literal string on every grid in
//!   the file, with no per-grid attribute selecting a different one, so
//!   drawing it would show grid 9's own logo under grid 1's name. Left
//!   undrawn rather than guessed at; see `docs/ui/campaign-screens.md`'s HD
//!   section.
//! - **`Record` (`MSC_CAMREC`)** - `Cell_SavedRecord`'s own value, which
//!   this build keeps no saved record for, the same absence
//!   `crate::campaign::draw::cell_draw_list`'s own `Line5`/`Line8` leave.

use oag_tables::race_campaign::{Cell, Difficulty, Medal, Mode};

use crate::frontend::{Draw, Placed};
use crate::language::{CircuitNames, StringTable};
use crate::menu::{Frame, Layers, Picture, Skin};
use crate::pointer::{Pointer, contains};
use crate::screen::Image;

use super::draw::{
    cell_title, centred_selector_draw, fill_draw, hex_slot_xy, image_draw, laps_line, medal_line,
    sprite_draw, text_draw,
};
use super::pointer::{Target, What};
use super::{CellSelection, Event, GridSelection, GridSummary, Layout, hex_rect};

#[cfg(test)]
mod tests;

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
        // `flyerlogo` names the same literal per-grid path
        // (`Data\FE\Flyers\01_uplift\Logo.gtf`) on every grid this screen
        // shows - see the module doc's "what this does not draw" section.
        if image.name.as_deref() == Some("flyerlogo") {
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
    let previous_cleared = model
        .grids()
        .get(model.index().wrapping_sub(1))
        .is_none_or(|previous| previous.points_earned >= previous.required_points);
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        let content = match name {
            "EventNum" | "GridNum" => Some(event_counter(model.index(), model.grids().len())),
            // `Medals Title`'s own child - named "Points" on this screen,
            // despite carrying the same gold-medal fraction Pulse's own
            // `Medals` field does (`Grid_CountMedalsAtLeast(grid, 0) /
            // Grid_CellCount`). Not `selected.points_earned`: that is
            // `TotPoints`'s own field below, under `Points Title`
            // (`RC_TOTPOINTSAV`) rather than `Medals Title`
            // (`RC_POINTSACH`).
            "Points" => Some(format!(
                "{:02}/{:02}",
                selected.gold_medals, selected.cell_count
            )),
            "TotPoints" => Some(format!(
                "{:03}/{:03}",
                selected.points_earned, selected.max_points
            )),
            // Two texts for two distinct reasons a tier is still locked -
            // **chosen, not measured**: no capture or decompile pins which
            // predicate each one is actually gated on, so this reuses
            // `GridSelection::selected_is_locked`'s own two terms (this
            // tier's own points against its own requirement, and the
            // previous tier's) to pick between them rather than showing
            // both or neither. See the module doc.
            //
            // `Required`'s own idstring (`RC_POINTSTOUNL`) is the same raw
            // `%d`-carrying template `hd_cell_draw_list`'s `NextPoints`
            // substitutes - see that arm's own doc for why
            // `required_points` is the number filled in.
            "Required" if locked && previous_cleared => text
                .string
                .as_deref()
                .map(|s| s.replacen("%d", &selected.required_points.to_string(), 1)),
            "Required Previous" if locked && !previous_cleared => text.string.clone(),
            "Required" | "Required Previous" => None,
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

/// `Cell Selection`'s draw list: the same 32-position staggered hex grid
/// Pulse's own `Cell Selection` is, plus HD's wider detail column and
/// difficulty toggle - see the module doc.
///
/// `footer_overlay` is [`crate::campaign::draw::cell_draw_list`]'s own
/// parameter, unchanged: HD's shared front-end root carries the identical
/// `NavigationController` shape Pulse's does (see
/// `oag_ui::campaign::footer::NavigationLegend`'s own doc), and neither
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
    let occupied: Vec<(u32, u32)> = model.cells().iter().filter_map(Cell::grid_coords).collect();
    let selected_coords = model.selected().and_then(Cell::grid_coords);
    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        if let Some(name) = image.name.as_deref() {
            if let Some((x, y)) = hex_slot_xy(name, "bBg_")
                .or_else(|| hex_slot_xy(name, "Bg_"))
                .or_else(|| hex_slot_xy(name, "Outline_"))
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
            // Same per-grid-identical path as `hd_grid_draw_list`'s own
            // `flyerlogo` skip - see the module doc.
            if name == "flyerlogo" {
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
            "NextPoints" => text
                .string
                .as_deref()
                .map(|s| s.replacen("%d", &grid_summary.required_points.to_string(), 1)),
            "RC Laps" => Some(laps_line(cell, strings)),
            // `Cell_SavedRecord` - no saved record kept by this build, the
            // same absence `crate::campaign::draw::cell_draw_list` leaves
            // `Line5`/`Line8` in. See the module doc.
            "Record" => None,
            "Points" => Some(format!(
                "{}/{}",
                model.selected_medal().map_or(0, |m| m.points()),
                oag_tables::race_campaign::Medal::Gold.points()
            )),
            "Best" => Some(medal_line(model.selected_medal(), strings)),
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        out.push(text_draw(text, &content, layout));
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

/// Where one medal tier's own static icon sits in `Hexmedal_HD.mip`/`.gtf` -
/// **measured off the disc's own authored crop, not eyeballed**:
/// `DATA06.PSARC`'s own copy of `CellMode_Definition.xml` (`data/fe/frontend/gui/cellmode_definition.xml`)
/// draws the same shared atlas a third way, on `Cell Selection`'s
/// `Target0/1/2 Medal` widgets, and *those* author the crop this build never
/// had for `Medal_{x}_{y}`: `width="60" height="60" u="0" v="0"` for
/// `Target0 Medal`, `v="61"` for `Target1`, `v="122"` for `Target2`,
/// `TxtrWidth`/`TxtrHeight` both `60` on all three. `Target0`/`Target1`/
/// `Target2` are already gold/silver/bronze elsewhere on this same screen
/// (`hd_cell_draw_list`'s own `"Target0" => hd_target_value(targets.gold, ..)`
/// arm and siblings, and `docs/ui/campaign-screens.md`'s note that this
/// widget family replaces the disc's own `IG_HUD_GOLD`/`SILVER`/`BRONZE`
/// labels), so the atlas reads **gold at the top, descending** - confidence
/// 90: a real widget's own authored attributes, for the identical texture,
/// on the same title's own screen family, not a decompile or a capture.
///
/// **`Target0/1/2 Medal` stopped being evidence-only and started being
/// drawn, 2026-09-27**, when `hd_cell_draw_list`'s own screen source switched
/// from `DATA02` to `DATA06` (see the module doc's "the winning archive"
/// section) - before that switch this doc's own numbers were read off
/// `DATA06` but the running screen still drew `DATA02`'s plain grey
/// `Target0/1/2 Image` arrow instead. `image_draw` draws the widget's own
/// authored crop directly now (`hd_medal_frame`'s numbers below still cover
/// `Medal_{x}_{y}`, the unauthored hex-grid badge, which is a different
/// widget on a different atlas placement rule).
///
/// **Cross-checked against the raw texel data independently**, and the two
/// disagree on which end is "top" until the discrepancy itself is
/// explained: decoding `data/fe/images/hexmedal_hd.gtf` directly
/// (`oag_texture::gtf::Gtf::parse`, `crates/texture/examples/scratch_hexmedal.rs`,
/// not committed) shows a 1024x256 DXT5 atlas whose *raster* row order is
/// the authored one upside down - the band this decode puts at
/// `y=196..256` (reaching the texture's own bottom edge exactly) is the one
/// whose mean RGB reads warm yellow-gold, matching `Target0`'s own gold at
/// `v=0`. That is consistent with a Y-flip between this decoder's raster
/// order and the GPU's own `V` convention (unmeasured *which* of the two is
/// "backwards" - nothing here decodes a second, independently-written GTF
/// reader to settle it) rather than two different textures: the row order,
/// the row height (60, matching `TxtrHeight`) and the per-row colour (one
/// warm, one neutral grey, one warm) all agree once the flip is accounted
/// for. The authored numbers below are what a caller already trusts
/// elsewhere in this same file (`Target0 Medal`'s own widget, wired as
/// ordinary XML), so they are used verbatim rather than the raster reading.
///
/// **Which column is "the" icon is not independently measured, and says
/// so** - `Target0 Medal`'s own `u="0"` picks the same first frame this
/// function already uses, but nothing pins whether the real screen holds
/// still on it or animates a spin through the rest of the row (the atlas
/// is far wider than one 60px frame). No earned-medal capture exists to
/// check either way - the same gap `medal_argb`'s own doc already records.
/// `u=0` is used because it is what the disc's own comparable widget uses,
/// not a guess.
///
/// **`difficulty` (`0` easy/novice .. `2` hard/elite, [`super::CellSelection::difficulty`]'s
/// own convention) selects one of three vertical blocks, not just a row.**
/// Measured 2026-09-28, against `DATA04.PSARC`'s own `1024x768` copy of this
/// texture (see [`oag_hd::campaign::PER_DIFFICULTY_MEDAL_ARCHIVE`]'s doc for
/// why that is the copy loaded): three `1024x256`-shaped bands, each holding
/// its own gold/silver/bronze row at the *same* `61`-pitch spacing the
/// authored `v=0/61/122` numbers above already give, separated by an
/// **unauthored** measured pitch of `183` (`60 + 1` gap, three rows, `3*61
/// = 183`) - read directly off the decoded atlas's own content bands
/// (alpha-per-row), not an authored crop; no widget on either archive's
/// `CellMode_Definition.xml` sources a `v` past `122`, so there is nothing
/// to cross-check this pitch against the way [`super::hd`]'s own doc
/// cross-checks the `61` spacing. Confidence 60 on the pitch number itself,
/// still measured rather than guessed.
///
/// **Block-to-difficulty mapping is chosen from convergent evidence, not an
/// authored crop or a live capture.** The three blocks hold visibly
/// different icon *shapes*, not only colour - a plain, unadorned hex
/// (bottom of the raster, nearest `v=0`), a hook/"cane"-shaped emblem
/// (middle), and a swirl/spiral emblem (top of the raster, farthest from
/// `v=0`), after the same raster/`V`-flip this file's own crop-reconciliation
/// paragraph above already establishes. `DATA02`'s shorter, flat-schema
/// copy (the one every pre-Fury cell effectively used before per-difficulty
/// targets existed) carries *only* the swirl shape, and
/// `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s
/// `SaveData_MigrateCellMedalsToHardElite` credits exactly that
/// pre-existing, single-tier medal at `HARD`/`ELITE` when migrating an old
/// save: the swirl icon and the hardest rung are the two things that
/// pre-existed the difficulty split, and the migration equates them. That
/// makes `easy/novice = plain hex = v-block 0`, `medium/skilled =
/// cane = v-block 1`, `hard/elite = swirl = v-block 2` the reading two
/// independent facts agree on, not a coin flip, but neither fact is a
/// pixel on a live screen, so this stays a **chosen** mapping (no
/// confidence score) until an RPCS3 capture toggling `DifficultyButton`
/// confirms which shape actually draws at which rung.
fn hd_medal_frame(medal: Medal, difficulty: Difficulty) -> [f32; 4] {
    const FRAME: f32 = 60.0;
    const BLOCK_PITCH: f32 = 183.0;
    let tier_v = match medal {
        Medal::Gold => 0.0,
        Medal::Silver => 61.0,
        Medal::Bronze => 122.0,
    };
    let block = match difficulty {
        Difficulty::Easy => 0.0,
        Difficulty::Medium => 1.0,
        Difficulty::Hard => 2.0,
    };
    [0.0, block * BLOCK_PITCH + tier_v, FRAME, FRAME]
}

/// `Medal_{x}_{y}`'s own draw: [`hd_medal_frame`]'s crop, drawn at its own
/// native 60x60 size, **at the widget's own authored `image.x`/`image.y`** -
/// unchanged from the pre-fix code, which already drew there (the size and
/// crop were the only things wrong - see the module's own bug writeup in
/// `docs/ui/campaign-screens.md`). **Deliberately not `hex_rect`-centred**,
/// an earlier draft of this fix tried: `CellMode_Definition.xml`'s own
/// `<Item>` grouping shows every `Medal_{x}_{y}` sits in its own item,
/// offset a constant `(+7, +2)` from `Bg_{x}_{y}`/`Outline_{x}_{y}`'s own
/// item at the same grid slot - checked across all seven columns
/// (`(7,2)`/`(67,36)`/`(127,2)`/`(187,36)`/`(247,2)`/`(307,36)`/`(367,2)`
/// against `Bg`'s own `(0,0)`/`(60,34)`/`(120,0)`/`(180,34)`/`(240,0)`/
/// `(300,34)`/`(360,0)`, the same `+7,+2` every time). That is the disc's
/// own registration between the medal layer and the hex layer, authored
/// once and not a per-column tune - a runtime centring formula would only
/// coincidentally reproduce it, and does not: it was tried, produced a
/// visibly-offset badge on every column, and was reverted in favour of
/// this, the simpler and disc-measured choice. **No tint**: unlike Pulse's
/// `hex_filled.mip` (a plain white hex [`super::draw::tinted_medal_draw`]
/// still colours with [`super::draw::medal_argb`] - that path is
/// untouched, and still correct for Pulse), HD's own atlas frame already
/// carries the tier's colour baked into its texels, so multiplying a flat
/// swatch over it a second time was the other half of the bug this
/// replaces.
fn hd_tinted_medal_draw(
    image: &Image,
    placed: Placed,
    medal: Medal,
    difficulty: Difficulty,
) -> Draw {
    let [u, v, frame_width, frame_height] = hd_medal_frame(medal, difficulty);
    let cropped = Image {
        width: Some(frame_width),
        height: Some(frame_height),
        u: Some(u),
        v: Some(v),
        texture_width: Some(frame_width),
        texture_height: Some(frame_height),
        ..image.clone()
    };
    sprite_draw(&cropped, placed, image.x, image.y, 0xffff_ffff)
}

/// `Target Title`'s own text - `DATA06`'s single shared header replacing
/// `DATA02`'s per-target `IG_HUD_GOLD`/`SILVER`/`BRONZE` labels (see
/// [`hd_medal_frame`]'s own doc). **Composed, not read verbatim from the
/// widget's own `idstring="IG_HUD_TARGET"`** - three live RPCS3 frames on
/// this exact widget (`data/scratch/drive-2026-09-27/hd-targets/`,
/// `fury-race-3-1.png`/`fury-speedlap-4-2.png`, plus
/// `data/scratch/lane-hd/rpcs3-grid0-3-2/02-square.png` for `Elimination`)
/// show text the bare idstring alone cannot produce:
///
/// | Cell | Mode | Frame reads |
/// | --- | --- | --- |
/// | `grid8_3_1` | `Race` | `"TARGET (NOVICE)"` |
/// | `grid8_4_2` | `Speed Lap` | `"TARGET LAP TIME (NOVICE)"` |
/// | `grid8_3_2` | `Elimination` | `"TARGET 200 (NOVICE)"` |
///
/// Three parts, each independently evidenced:
/// - **The base label is mode-dependent.** `Race`'s own is `IG_HUD_TARGET`
///   (`"TARGET"`) unchanged; `Speed Lap`'s is a different idstring entirely,
///   `FE_TLTIME` (`"TARGET LAP TIME"`,
///   `data/scratch/lane-hd-sel/english-entries-data06.xml`). `Time
///   Trial`'s own case is inferred, not captured: `FE_TARGTIME` (`"TARGET
///   TIME"`) is the only other `FE_TARG*`/`FE_TL*` idstring on the disc,
///   named for exactly the mode this function has no frame for -
///   confidence 60. Every other mode falls back to the widget's own
///   `IG_HUD_TARGET`, measured for `Race`, chosen by elimination for
///   `Zone`/`Tournament`/`Head2Head`/`Mode::Other`.
/// - **A number is inserted for `Elimination` and `NitroBattle` alike -
///   `"the elimination family"`.** `grid8_3_2`'s own `NitroElimNovice="200"`
///   (`grid_08.xml`) matches the captured `200` exactly; `grid8_3_1`/`grid8_4_2`
///   (`Race`/`Speed Lap`) both author the same field too, always dummy `1`s,
///   and neither frame shows a number - confirming the insert is mode-gated,
///   not "whenever the field parses". `NitroBattle` (`grid8_2_1`) has no
///   frame of its own, but is included on stronger evidence than a guess:
///   `campaign_grids_ground_truth.rs`'s own
///   `eliminationfamily_cells_carry_a_real_nitro_triple_and_a_dummy_flat_one`
///   ground-truths, against the real disc, that `Elimination` and
///   `Mode::Other("NitroBattle")` are exactly the two modes whose
///   `nitro_elimination_targets` is real (non-`(1, 1, 1)`) while
///   `difficulty_targets` is the dummy flat `1`/`2`/`3` - the same pairing
///   this function's own condition below tests for. `Detonator`
///   (`Mode::Other("Detonator")`) is the disc's own mirror image of that
///   (real `difficulty_targets`, dummy nitro triple) and is correctly left
///   out by the same test.
/// - **The `(DIFFICULTY)` suffix is unconditional across all three
///   captures.** Gated here on the cell carrying a difficulty rung at all
///   (`difficulty_targets`/`nitro_elimination_targets` `Some`) since every
///   captured cell is Fury's own (`grid8`, always per-difficulty) - a
///   base-`Wipeout HD` cell reached through this project's own `DATA02`
///   grid precedence parses neither field, so this degrades to no suffix
///   there rather than a guessed difficulty name. Not independently
///   captured either way for a base-HD cell.
fn hd_target_title(cell: &Cell, strings: &StringTable, difficulty: Difficulty) -> String {
    let base = match cell.mode {
        Mode::TimeTrial => strings.get_or_id("FE_TARGTIME"),
        Mode::SpeedLap => strings.get_or_id("FE_TLTIME"),
        _ => strings.get_or_id("IG_HUD_TARGET"),
    };
    let mut title = base.to_string();
    let is_elimination_family = matches!(cell.mode, Mode::Elimination)
        || matches!(&cell.mode, Mode::Other(name) if name == "NitroBattle");
    if is_elimination_family
        && let Some(target) = cell.nitro_elimination_target_for_difficulty(difficulty)
    {
        title.push(' ');
        title.push_str(&target.to_string());
    }
    if cell.difficulty_targets.is_some() || cell.nitro_elimination_targets.is_some() {
        title.push_str(" (");
        title.push_str(strings.get_or_id(hd_difficulty_id(difficulty)));
        title.push(')');
    }
    title
}

/// `Easy`/`Medium`/`Hard`'s own idstring names, `0` through `2` - the same
/// three ids this disc's own difficulty rungs resolve
/// (`data/scratch/lane-hd-sel/english-entries-data06.xml`: `<entry
/// id="Easy" string="NOVICE">`, `id="Medium" string="SKILLED">`, `id="Hard"
/// string="ELITE">`), this title's own words for the same rung
/// (`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s own
/// `Novice`/`Skilled`/`Elite` measurement, the same one
/// [`Cell::nitro_elimination_target_for_difficulty`]'s doc already cites).
/// Confidence 85: the id spelling is a direct disc read; matching it to
/// [`Difficulty`]'s own rung order reuses the same one
/// [`Cell::targets_for_difficulty`] already assumes, not independently
/// re-verified here.
fn hd_difficulty_id(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "Easy",
        Difficulty::Medium => "Medium",
        Difficulty::Hard => "Hard",
    }
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
/// [`CircuitNames`], falling back to the string table and then the raw id,
/// the same three-step fallback `oag_game::catalogue::label` gives the RACE
/// page. **Simplified from that function**: it also appends `FE_REVERSE`
/// where a reversed circuit shares its forward twin's own name, which needs
/// the full circuit catalogue (`every: &[Track]`) this module has no access
/// to at draw time - not applied here, so a reversed HD circuit whose name
/// collides with its forward twin draws identically to it. `Tournament`
/// cells (`{n} Races`) are unaffected, the same reading
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

/// `Grid Selection`'s pointer targets: the paging arrows, and a confirm
/// region over the flyer.
///
/// **The confirm rect is invented, and grounded in a real one rather than
/// picked freely.** This screen authors no hex tile or button widget a
/// player could click to enter the tier the way Pulse's own hex tiles are -
/// the flyer itself is what a player would reach for, and nothing gives its
/// on-screen size (it is a 3-D model, not a 2-D widget with a rect). Rather
/// than invent a number outright, this reuses `Flyer Pad Lock`'s own
/// authored rect (`x="934" y="304" width="512" height="512"`) - the padlock
/// overlay that already sits centred on the flyer when one is locked, so its
/// rect is a real, disc-authored bound on roughly where the flyer itself is,
/// not an arbitrary box. **Chosen, not measured.**
#[must_use]
pub fn hd_grid_targets(layout: &Layout, sprites: &dyn Fn(&str) -> Option<Placed>) -> Vec<Target> {
    let screen = &layout.screen;
    let mut out = Vec::new();
    for image in &screen.images {
        let what = match image.name.as_deref() {
            Some("Flyer Left Arrow") => What::Previous,
            Some("Flyer Right Arrow") => What::Next,
            Some("Flyer Pad Lock") => What::Hex(0),
            _ => continue,
        };
        if let Some(rect) = image_rect(image, sprites) {
            out.push(Target { what, rect });
        }
    }
    out
}

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
pub fn difficulty_button_rect(screen: &crate::screen::Screen) -> Option<[f32; 4]> {
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
