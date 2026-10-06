//! `Grid Selection`/`Cell Selection`'s own draw lists, split out of
//! `campaign.rs` once that file passed the 1,000-line rule
//! (`scripts/check-file-size.py`) - a move, with no behaviour change to the
//! functions carried across. The model types (`GridSelection`,
//! `CellSelection`, `Layout`, `GridSummary`) stay in the parent module;
//! this file only turns one into a [`oag_ui::menu::Layers`].

use oag_tables::race_campaign::{Cell, Difficulty, Medal, Mode};

use oag_ui::frontend::{Align, Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::screen::{Fill, Image, Text, argb_to_rgba};

use super::{CellSelection, GRIDS_PER_PAGE, GridSelection, Layout, hex_rect};

#[cfg(test)]
mod tests;

/// `Grid Selection`'s draw list.
///
/// `footer_overlay` is already-built `Draw`s to draw over everything else -
/// the scrolling tip ticker, off [`super::footer::ticker_draw`], since that widget
/// is authored once on the front-end root rather than per screen and this
/// function has no reason to know `Skin.xml` exists. Empty on any caller
/// that has not built one (every `--menu-page grid-select` capture today -
/// see `oag_game::campaign::Campaign::ticker`'s own doc).
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes, plus the footer overlay"
)]
pub fn grid_draw_list(
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
    let title = strings.get_or_id("FE_RACE_CAM").to_string();
    layers.chrome.push(Draw::title(
        skin.title_font(),
        title_x,
        title_y,
        title_scale,
        skin.title_color(frame.ink),
        title,
    ));
    layers.chrome.extend_from_slice(footer_overlay);

    let screen = &layout.screen;
    let mut out = Vec::new();
    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }
    let page = model.page();
    let slot = model.slot();
    for image in &screen.images {
        if let Some(name) = image.name.as_deref()
            && let Some(slot_index) = hex_slot_of(name, "Medal_", 1)
                .or_else(|| hex_slot_of(name, "Outline_", 1))
                .or_else(|| hex_slot_of(name, "Lock_", 1))
        {
            if page * GRIDS_PER_PAGE + slot_index >= model.grids().len() {
                continue;
            }
            if name.starts_with("Medal_")
                && !model
                    .grids()
                    .get(page * GRIDS_PER_PAGE + slot_index)
                    .is_some_and(|grid| grid.points_earned > 0)
            {
                // The medal-colour swatch draws only once the tier has
                // earned any points of its own - `GridSelection_PopulateTiles`'s
                // own `if pointsEarned != 0` branch. Every tier's base hex
                // (`Outline_x_y`) still draws regardless - see the module doc.
                continue;
            }
            if name.starts_with("Lock_")
                && !model.tier_shows_lock(page * GRIDS_PER_PAGE + slot_index)
            {
                continue;
            }
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        if image.name.as_deref() == Some("Selector") {
            if let Some(hex) = hex_rect(screen, slot, 0, sprites) {
                out.push(centred_selector_tinted_draw(
                    image,
                    placed,
                    hex,
                    SELECTOR_TINT,
                ));
            }
            continue;
        }
        if image
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("Medal_"))
        {
            out.push(tinted_medal_draw(image, placed, medal_tint(0)));
            continue;
        }
        if image
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("Outline_"))
        {
            out.push(sprite_draw(
                image,
                placed,
                image.x,
                image.y,
                TIER_OUTLINE_TINT,
            ));
            continue;
        }
        if image.name.as_deref() == Some("up arrow") {
            let argb = if page == 0 {
                ARROW_DISABLED_TINT
            } else {
                0xffff_ffff
            };
            out.push(sprite_draw(image, placed, image.x, image.y, argb));
            continue;
        }
        if image.name.as_deref() == Some("down arrow") {
            let last_page = model.grids().len().saturating_sub(1) / GRIDS_PER_PAGE;
            let argb = if page >= last_page {
                ARROW_DISABLED_TINT
            } else {
                0xffff_ffff
            };
            out.push(sprite_draw(image, placed, image.x, image.y, argb));
            continue;
        }
        out.push(image_draw(image, placed));
    }
    let Some(selected) = model.selected() else {
        layers.body = out;
        return layers;
    };
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        let content = match name {
            "honey" => Some(model.counter()),
            "Title" => Some(grid_title(&selected.name, strings)),
            "Medals" => Some(format!("{}/{}", selected.gold_medals, selected.cell_count)),
            "Points" => Some(format!(
                "{:03}/{:03}",
                selected.points_earned, selected.max_points
            )),
            "Required" => Some(if selected.required_points == 0 {
                strings.get_or_id("FE_NA").to_string()
            } else {
                format!("{:03}", selected.required_points)
            }),
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        out.push(text_draw(text, &content, layout));
    }
    layers.body = out;
    layers
}

/// `Cell Selection`'s draw list.
///
/// `footer_overlay` is [`grid_draw_list`]'s own new parameter - here it
/// carries both the ticker and the `Confirm`/`Back` half of the button
/// legend ([`super::footer::NavigationLegend::draw`]), since this is the one
/// screen `docs/ui/campaign-screens.md`'s 2026-09-14 PPSSPP pass measured
/// showing both.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes, plus the footer overlay"
)]
pub fn cell_draw_list(
    model: &CellSelection,
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
    let title = strings.get_or_id("FE_RACE_CAM").to_string();
    layers.chrome.push(Draw::title(
        skin.title_font(),
        title_x,
        title_y,
        title_scale,
        skin.title_color(frame.ink),
        title,
    ));
    layers.chrome.extend_from_slice(footer_overlay);

    let screen = &layout.screen;
    let mut out = Vec::new();
    let occupied: Vec<(u32, u32)> = model.cells().iter().filter_map(Cell::grid_coords).collect();
    let selected_coords = model.selected().and_then(Cell::grid_coords);
    for fill in &screen.fills {
        if !line_bg_visible(fill, model) {
            continue;
        }
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        if let Some(name) = image.name.as_deref() {
            if let Some((x, y)) = hex_slot_xy(name, "Medal_")
                .or_else(|| hex_slot_xy(name, "Outline_"))
                .or_else(|| hex_slot_xy(name, "Lock_"))
                && !occupied.contains(&(x, y))
            {
                continue;
            }
            if let Some((x, y)) = hex_slot_xy(name, "Medal_")
                && model.medal_at(x, y).is_none()
            {
                // The medal-colour swatch draws only over a cell that has
                // one - `CellSelection_PopulateGrid`'s own `else` branch.
                // The base hex (`Outline_x_y`) still draws for every
                // occupied slot - see the module doc.
                continue;
            }
            if let Some((x, y)) = hex_slot_xy(name, "Lock_")
                && !model.cell_shows_lock(x, y)
            {
                continue;
            }
            // Drawn below, gated on `targets_visible` - not here, where
            // nothing yet knows the selected cell's own mode.
            if matches!(name, "Target0 Image" | "Target1 Image" | "Target2 Image") {
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
            out.push(centred_selector_tinted_draw(
                image,
                placed,
                hex,
                SELECTOR_TINT,
            ));
            continue;
        }
        if let Some((x, y)) = image.name.as_deref().and_then(|n| hex_slot_xy(n, "Medal_")) {
            out.push(tinted_medal_draw(
                image,
                placed,
                model.medal_at(x, y).map_or(0xffff_ffff, medal_argb),
            ));
            continue;
        }
        out.push(image_draw(image, placed));
    }
    let Some(cell) = model.selected() else {
        layers.body = out;
        return layers;
    };

    let counting = matches!(cell.mode, Mode::Zone | Mode::Elimination);
    let targets_visible = matches!(
        cell.mode,
        Mode::TimeTrial | Mode::Zone | Mode::Elimination | Mode::SpeedLap
    );
    let weapons_visible = matches!(cell.mode, Mode::Race | Mode::Head2Head | Mode::Tournament);

    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        let content = match name {
            "Title" => Some(cell_title(cell, strings)),
            "Track Line" => Some(track_line(cell, strings)),
            "Line1 Title" if cell.mode != Mode::Zone => {
                Some(strings.get_or_id("RC_SC").to_string())
            }
            "Line2 Title" => Some(strings.get_or_id("RC_LAPS").to_string()),
            "Line3 Title" if weapons_visible => Some(strings.get_or_id("RB_WEAP").to_string()),
            "Line6 Title" => Some(strings.get_or_id("ER_POINTS").to_string()),
            // `Best` shares its row with `Target0..2` rather than sitting
            // alongside them - see `docs/ui/campaign-screens.md`'s "Best/
            // Target looks mutually exclusive" finding.
            "Line7 Title" if !targets_visible => Some(strings.get_or_id("IG_HUD_BEST").to_string()),
            name if name.starts_with("Line") && name.ends_with("Title") => {
                // `Line4`/`Line5`/`Line8 Title` and the two above's own
                // negative case - no traced idstring, or hidden this cell.
                // See the module doc.
                None
            }
            "Line1" if cell.mode != Mode::Zone => Some(cell.class.clone()),
            "Line1" => None,
            "Line2" => Some(laps_line(cell, strings)),
            "Line3" if weapons_visible => {
                let id = if cell.weapons { "FE_ON" } else { "FE_OFF" };
                Some(strings.get_or_id(id).to_string())
            }
            "Line3" => None,
            // `Line5`: `Cell_SavedRecord` - see `CellSelection::selected_record`'s
            // own doc for which modes actually carry one today. `Line8`
            // authors the identical `OffsetX="260" OffsetY="180"` Target0
            // sits at (`CellMode_Definition.xml`), so drawing it whenever
            // `targets_visible` would overlap that row outright; nothing
            // traces what the original shows there instead, so it stays
            // blank rather than guessed. `Line4` never appears in
            // `CellSelection_PopulateDetail`'s own table at all.
            "Line5" => model
                .selected_record()
                .map(|centis| target_value(centis, &cell.mode)),
            "Line4" | "Line8" => None,
            "Line6" => Some(format!(
                "{}/{}",
                model.selected_medal().map_or(0, Medal::points),
                Medal::Gold.points()
            )),
            "Line7" if !targets_visible => Some(medal_line(
                model.selected_medal(),
                model.selected_difficulty(),
                strings,
            )),
            "Line7" => None,
            // The square-button prompt, `CellSelection_Update`'s own
            // `sprintf("%s (%s)", RB_AI_DIF, rung)` rebuilt every frame -
            // see `difficulty_button_line`'s own doc.
            "DifficultyButton" => Some(difficulty_button_line(model.difficulty(), strings)),
            "Target0 Title" | "Target1 Title" | "Target2 Title" if targets_visible => {
                Some(strings.get_or_id("IG_HUD_TARGET").to_string())
            }
            "Target0" if targets_visible => Some(target_value(cell.gold, &cell.mode)),
            "Target1" if targets_visible => Some(target_value(cell.silver, &cell.mode)),
            "Target2" if targets_visible => Some(target_value(cell.bronze, &cell.mode)),
            "Target0 Title" | "Target1 Title" | "Target2 Title" | "Target0" | "Target1"
            | "Target2" => None,
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        out.push(text_draw(text, &content, layout));
    }
    // The three medal-tier swatches: shown iff their own text row is.
    for image in &screen.images {
        let Some(name) = image.name.as_deref() else {
            continue;
        };
        if matches!(name, "Target0 Image" | "Target1 Image" | "Target2 Image") && targets_visible {
            let Some(placed) = sprites(&image.src) else {
                continue;
            };
            out.push(image_draw(image, placed));
        }
    }
    let _ = counting; // direction is `Cell::evaluate_medal`'s own concern, not this draw's
    layers.body = out;
    layers
}

/// Whether a `line bg{n}l`/`line bg{n}r` divider draws - the frames show one
/// stripe per *visible* detail row (`cell-selection-grid0-default-cell.png`:
/// 3 on a `Race` cell; `cell-selection-grid0-cell2-t0.png`: 2 on `Time
/// Trial`), not four unconditionally. `line bg1`/`2`/`3` pair with `Line1`/
/// `Line2`/`Line3`'s own visibility; `line bg4` never draws, since nothing
/// ever populates `Line4` (see the module doc) to sit above it. `Grid
/// Selection`'s own three stripes have no such gate - `Medals`/`Points`/
/// `Required` are always populated - so this only ever filters something out
/// on `Cell Selection`.
fn line_bg_visible(fill: &Fill, model: &CellSelection) -> bool {
    let Some(name) = &fill.name else { return true };
    let Some(cell) = model.selected() else {
        return true;
    };
    if name.starts_with("linebg1") {
        return cell.mode != Mode::Zone;
    }
    if name.starts_with("linebg2") {
        return true;
    }
    if name.starts_with("linebg3") {
        return matches!(cell.mode, Mode::Race | Mode::Head2Head | Mode::Tournament);
    }
    if name.starts_with("linebg4") {
        return false;
    }
    true
}

/// `Grid Selection`'s `Title`: the grid's own name, capitalised and looked
/// up as an idstring - `"grid0"` -> `"Grid0"` -> `RC_`-adjacent entry
/// `"Grid0"`, which the disc's own English table answers `"Grid 1"` (1-based,
/// the font upper-casing it to `"GRID 1"` on screen). Measured,
/// `docs/ui/campaign-screens.md`'s "Measured against PPSSPP": three separate
/// grids (`grid0`, `grid4`, `grid8`) read `"GRID 1"`/`"GRID 5"`/`"GRID 9"`
/// digit for digit against this lookup, and `grid12`..`grid15` are **not**
/// `"Grid 13"`..`"Grid 16"` - the table answers `"Phantom Grid 1"`..
/// `"Phantom Grid 4"` for those four instead, which is a real per-grid
/// idstring rather than a formula this build could derive. Falls back to
/// the raw name on a source whose table has no such entry, the same visible-
/// absence rule every other `get_or_id` call on this screen already follows.
fn grid_title(grid_name: &str, strings: &StringTable) -> String {
    let mut chars = grid_name.chars();
    let id = match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => return grid_name.to_string(),
    };
    strings
        .get(&id)
        .map_or_else(|| grid_name.to_string(), str::to_string)
}

/// `Cell Selection`'s `Title`: the localised mode name, not `cell.mode`'s
/// raw enum spelling. Measured for the five modes this engine implements
/// (`docs/ui/campaign-screens.md`'s "Measured against PPSSPP": a `Race` cell
/// reads `"SINGLE RACE"`, a `Time Trial` cell `"TIME TRIAL"`) by mirroring
/// [`oag_ui::menu::mode_label`]'s own `MSC_EVENT_*`-head-before-the-colon
/// reading, which [`oag_race::Mode`] does not have variants to receive
/// `Tournament`/`Head2Head` through - `oag_ui` cannot call
/// `oag_game::campaign::race_mode_for_cell` either way, the dependency rule
/// this crate is on the wrong side of (`crate::campaign`'s own module doc
/// links the two). `MSC_EVENT_TOURN`/`MSC_EVENT_HTH` exist on disc and are
/// read the identical way, unmeasured against a live frame but the same
/// mechanism as the five that are. `Custom Grid`/`AI Race`/`Other` fall back
/// to the raw spelling, per `docs/ui/campaign-screens.md`.
pub(super) fn cell_title(cell: &Cell, strings: &StringTable) -> String {
    match cell.mode {
        Mode::Race => oag_ui::menu::mode_label(oag_race::Mode::SingleRace, strings),
        Mode::TimeTrial => oag_ui::menu::mode_label(oag_race::Mode::TimeTrial, strings),
        Mode::Zone => oag_ui::menu::mode_label(oag_race::Mode::Zone, strings),
        Mode::Elimination => oag_ui::menu::mode_label(oag_race::Mode::Eliminator, strings),
        Mode::SpeedLap => oag_ui::menu::mode_label(oag_race::Mode::SpeedLap, strings),
        Mode::Tournament => mode_event_label("MSC_EVENT_TOURN", strings, cell.mode.as_str()),
        Mode::Head2Head => mode_event_label("MSC_EVENT_HTH", strings, cell.mode.as_str()),
        Mode::CustomGrid | Mode::AiRace | Mode::Other(_) => cell.mode.as_str().to_string(),
    }
}

/// The head-before-the-colon reading [`oag_ui::menu::mode_label`] applies to
/// an `MSC_EVENT_*` id, duplicated here rather than shared: `mode_label`
/// takes an [`oag_race::Mode`], which has no variant for `Tournament`/
/// `Head2Head` to reach it through - see [`cell_title`]'s own doc.
fn mode_event_label(id: &str, strings: &StringTable, fallback: &str) -> String {
    const MAX: usize = 24;
    strings
        .get(id)
        .and_then(|text| text.split(':').next())
        .map(str::trim)
        .filter(|head| !head.is_empty() && head.chars().count() <= MAX)
        .unwrap_or(fallback)
        .to_string()
}

/// `Track Line`: the circuit's own display name, resolved the same way
/// every other fixed label on this screen is - `strings.get_or_id` - rather
/// than the raw `NN_Track` id `cell.track` carries. Measured,
/// `docs/ui/campaign-screens.md`'s "Measured against PPSSPP": `16_Track`
/// resolves to `"Talon's Junction White"` off the disc's own English table,
/// case for case with `PI_Track`'s own `name=` attribute - unlike Wipeout
/// HD, Pulse's single copy of the table needs no [`oag_ui::language::CircuitNames`]
/// fold to reach it.
fn track_line(cell: &Cell, strings: &StringTable) -> String {
    if cell.mode == Mode::Tournament {
        format!("{} Races", cell.tournament_tracks.len())
    } else {
        cell.track
            .as_deref()
            .map_or_else(String::new, |id| strings.get_or_id(id).to_string())
    }
}

/// `Line7`'s own resolution: `Cell_SavedMedal` maps onto `IG_HUD_GOLD`/
/// `SILVER`/`BRONZE`, `MSC_NONE` for no saved medal - suffixed with
/// `Cell_SavedDifficulty`'s own rung when `medal` is `Some` and
/// `difficulty` answers one, `"Gold (Medium)"` rather than a bare medal
/// word. `CellSelection_PopulateDetail`'s own `"%s (%s)"` format, decompiled
/// in full 2026-09-28 - see
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "The
/// `DifficultyRC` persisted rung" section. `difficulty` is `None` on HD's
/// own call site (`hd.rs`'s `"Best"` widget): HD draws its own per-cell
/// rung through a hex medal icon (`hd_medal_frame`), not this text suffix,
/// which is unmeasured there - not invented onto a screen this build has
/// not seen show it.
pub(super) fn medal_line(
    medal: Option<Medal>,
    difficulty: Option<Difficulty>,
    strings: &StringTable,
) -> String {
    let id = match medal {
        Some(Medal::Gold) => "IG_HUD_GOLD",
        Some(Medal::Silver) => "IG_HUD_SILVER",
        Some(Medal::Bronze) => "IG_HUD_BRONZE",
        None => "MSC_NONE",
    };
    if let (Some(_), Some(difficulty)) = (medal, difficulty) {
        return format!(
            "{} ({})",
            strings.get_or_id(id),
            strings.get_or_id(difficulty_rung_id(difficulty))
        );
    }
    strings.get_or_id(id).to_string()
}

/// The bare idstring `Cell_SavedDifficulty`/`CellSelection_Update` both key
/// on for a rung's own display name - `"Easy"`/`"Medium"`/`"Hard"`, no
/// `MSC_`/`FE_` prefix, read directly off the executable's own string table
/// at `0x08a826e4` (confidence 85). Not [`Difficulty`]'s own Rust
/// `Debug`/variant spelling reused as an idstring by convention - a
/// coincidence of English, confirmed by `read_memory` on the literal bytes,
/// not assumed from the enum's own name.
fn difficulty_rung_id(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "Easy",
        Difficulty::Medium => "Medium",
        Difficulty::Hard => "Hard",
    }
}

/// `DifficultyButton`'s own text, Pulse's `Cell Selection`: the disc
/// authors the static `string="Change Difficulty"`, but `CellSelection_Update`
/// (`0x088d6430`) overwrites it every frame with `sprintf("%s (%s)",
/// resolve("RB_AI_DIF"), resolve(rung))` off the screen's own browsed
/// rung (`CellSelection::difficulty`, cycled on `Square`) - `"AI difficulty
/// (Medium)"`, matching the live PPSSPP capture
/// (`docs/ui/campaign-screens.md`'s "AI difficulty (square) cycles"
/// finding: `"AI difficulty (Medium)"` -> `"(Hard)"` on a `Square` press,
/// nothing else in the panel). `RB_AI_DIF` is the same idstring `Single
/// Player`'s own `Difficulty` row authors (`docs/formats/race-setup.md`),
/// resolved here rather than hardcoded so a non-English table changes this
/// widget's word too. See
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "The
/// `DifficultyRC` persisted rung" section. Confidence 85.
pub(super) fn difficulty_button_line(difficulty: Difficulty, strings: &StringTable) -> String {
    format!(
        "{} ({})",
        strings.get_or_id("RB_AI_DIF"),
        strings.get_or_id(difficulty_rung_id(difficulty))
    )
}

pub(super) fn laps_line(cell: &Cell, strings: &StringTable) -> String {
    match cell.laps {
        Some(laps) if laps >= 1 && cell.mode != Mode::Zone => laps.to_string(),
        _ => strings.get_or_id("RC_INF").to_string(),
    }
}

/// `Target0..2`'s own value: a time for `Time Trial`/`Speed Lap`, a plain
/// number otherwise (`Zone`'s zone count, `Elimination`'s kill count).
pub(super) fn target_value(value: i64, mode: &Mode) -> String {
    if matches!(mode, Mode::TimeTrial | Mode::SpeedLap) {
        format_centiseconds(value)
    } else {
        value.to_string()
    }
}

/// Centiseconds to `M:SS.CC`, the unit `PI_Cell_ParseElement` stores a
/// `Gold`/`Silver`/`Bronze Target` in for a timed mode - see
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`.
pub(super) fn format_centiseconds(value: i64) -> String {
    let value = value.max(0);
    let minutes = value / 6000;
    let seconds = (value / 100) % 60;
    let centis = value % 100;
    format!("{minutes}:{seconds:02}.{centis:02}")
}

/// The hex slot index (`0..GRIDS_PER_PAGE`) a `Grid Selection` widget name
/// carries, e.g. `"Medal_2_0"` -> `2`. `coord` is which underscore-separated
/// number to read (both screens spell the coordinate the same way).
fn hex_slot_of(name: &str, prefix: &str, coord: usize) -> Option<usize> {
    let rest = name.strip_prefix(prefix)?;
    rest.split('_').nth(coord.saturating_sub(1))?.parse().ok()
}

/// The `(x, y)` a `Cell Selection` widget name carries, e.g.
/// `"Outline_3_2"` -> `(3, 2)`.
pub(super) fn hex_slot_xy(name: &str, prefix: &str) -> Option<(u32, u32)> {
    let rest = name.strip_prefix(prefix)?;
    let mut parts = rest.split('_');
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    Some((x, y))
}

pub(super) fn fill_draw(fill: &Fill) -> Draw {
    let rect = [
        fill.x,
        fill.y,
        fill.width.unwrap_or(0.0),
        fill.height.unwrap_or(0.0),
    ];
    match fill.gradient {
        Some([c1, c2, c3, c4]) => Draw::GradientFill {
            rect,
            left: mean(argb_to_rgba(c1), argb_to_rgba(c2)),
            right: mean(argb_to_rgba(c3), argb_to_rgba(c4)),
        },
        None => Draw::Fill {
            rect,
            color: argb_to_rgba(fill.color),
        },
    }
}

fn mean(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
        (a[3] + b[3]) * 0.5,
    ]
}

/// An ordinary image widget, at its own authored position.
pub(super) fn image_draw(image: &Image, placed: Placed) -> Draw {
    sprite_draw(image, placed, image.x, image.y, image.color)
}

/// `gold`/`silver`/`bronze[medal]` - `CellSelection_PopulateGrid`'s own
/// `draw_medal_colour`. **Chosen, not measured**: the medal-colour swatch's
/// real ARGB was not traced (no PPSSPP capture exercises it - every frame
/// this build has is a zero-medal profile). Reused here from the panel's
/// own `Target0..2 Image` swatches - `docs/ui/campaign-screens.md`'s
/// measured table - since they are the same three-tier colour concept
/// authored on the same screen, not an independently measured value for the
/// hex fill itself.
pub fn medal_argb(medal: Medal) -> u32 {
    match medal {
        Medal::Gold => 0xfffa_eb38,
        Medal::Silver => 0xffda_e3e4,
        Medal::Bronze => 0xffdf_942f,
    }
}

/// `Grid Selection`'s own tint for a tier that has scored anything at all -
/// `medal_tier_colour(pointsEarned, ...)`'s exact mapping was not traced, so
/// this always answers gold's own swatch. **Chosen, not measured** twice
/// over: whether to tint at all is [`GridSelection`]'s own `points_earned >
/// 0` gate (measured, `GridSelection_PopulateTiles`), but which of the
/// three colours is not.
fn medal_tint(_points_earned: u32) -> u32 {
    medal_argb(Medal::Gold)
}

/// A `Medal_x_y` swatch, tinted `argb` in place of the image's own authored
/// colour (always `0xffffffff` on both screens - neither XML tints per
/// medal itself, since the tint is a runtime choice).
pub(super) fn tinted_medal_draw(image: &Image, placed: Placed, argb: u32) -> Draw {
    sprite_draw(image, placed, image.x, image.y, argb)
}

/// `Selector`'s own draw, centred on `hex` - the selected hex's own rect
/// from [`hex_rect`]. **Chosen, not measured**: `CellMode_Definition.xml`
/// positions `Selector` at a fixed default (`x="30" y="95"`, overridden here
/// regardless) and authors no offset between its own 42x43 sprite and
/// whatever size a hex actually draws at (32x32, measured off
/// `hex_filled.mip`) - top-left aligning the two, as this build previously
/// did, draws the cursor visibly down-and-right of the hex it marks rather
/// than around it. See `docs/ui/campaign-screens.md`.
pub(super) fn centred_selector_draw(image: &Image, placed: Placed, hex: [f32; 4]) -> Draw {
    let width = image.width.unwrap_or(placed.width as f32);
    let height = image.height.unwrap_or(placed.height as f32);
    let x = hex[0] + (hex[2] - width) * 0.5;
    let y = hex[1] + (hex[3] - height) * 0.5;
    sprite_draw(image, placed, x, y, image.color)
}

/// [`centred_selector_draw`], but with the tint overridden rather than taken
/// from `image.color` - `Pulse`'s own `grid_draw_list`/`cell_draw_list` want
/// [`SELECTOR_TINT`] here, but [`super::hd::hd_grid_draw_list`] still calls
/// [`centred_selector_draw`] itself unchanged, so that function's own
/// signature (and HD's own, unmeasured, selected-tile look) is left alone
/// rather than threading a tint parameter through a call this crate does not
/// own the other end of.
pub(super) fn centred_selector_tinted_draw(
    image: &Image,
    placed: Placed,
    hex: [f32; 4],
    argb: u32,
) -> Draw {
    let width = image.width.unwrap_or(placed.width as f32);
    let height = image.height.unwrap_or(placed.height as f32);
    let x = hex[0] + (hex[2] - width) * 0.5;
    let y = hex[1] + (hex[3] - height) * 0.5;
    sprite_draw(image, placed, x, y, argb)
}

/// `Selector`'s own runtime colour, one of two endpoints a shared,
/// non-PI001 widget routine (`FUN_088a5700`, xref'd off the `Selector`/
/// `SelectorGlow` widget-name strings at `0x08a7f130`/`0x08a7f178`) pulses
/// the selected tile's outline between on a continuous ~1s triangle wave:
/// `lerp(t, 0xff33a6b9, 0xffffffff)` one half of the cycle and the reverse
/// the other half - `CellMode_Definition.xml` itself only authors `Selector`
/// at a fixed `i="0xffffffff"` (multiply-neutral), so the pulse is entirely
/// this routine's doing. Measured (the colour constant is a literal
/// decompiled off `0x088a5700`, confidence 85), but the animation itself is
/// **not implemented** - `grid_draw_list`/`cell_draw_list` build a draw list
/// with no clock, so this always draws the cyan endpoint rather than
/// oscillating. Picked over the white endpoint because it is what
/// `data/reference/psp-campaign-screens/grid-selection-page1-grid0-unlocked.png`
/// (this build's own comparison frame) happens to show; a capture taken at a
/// different phase would show closer to white instead. See
/// `docs/ui/campaign-screens.md`.
const SELECTOR_TINT: u32 = 0xff33_a6b9;

/// `Outline_x_y`'s own runtime tint on `Grid Selection` - **not** on `Cell
/// Selection`, whose own `CellSelection_PopulateGrid` never calls
/// `GridController_SetTileColor` on layer 1 at all, leaving that screen's
/// hex outlines at the XML's own multiply-neutral default. `grid_draw_list`
/// only, then: `GridSelection_PopulateTiles` (`0x088de6f4`) sets every
/// tier's own base hex - locked or not, selected or not, the tint is
/// unconditional - to `param_2 * 0.5 * 255` alpha over literal RGB
/// `0x34acc2`; steady state (`param_2` settled at `1.0` once its own
/// entrance fade finishes) gives `(int)(0.5 * 255.0)` truncated to `127`.
/// Measured, confidence 85 (decompile literal, not runtime-verified).
const TIER_OUTLINE_TINT: u32 = 0x7f34_acc2;

/// `up arrow`/`down arrow`'s own disabled tint - `FUN_088de9bc`, called at
/// the end of every `GridSelection_Update`: dark grey `0xff505050` opaque
/// when the current page is already the first (`up arrow`) or the last
/// (`down arrow`), full white otherwise. Measured, confidence 85.
const ARROW_DISABLED_TINT: u32 = 0xff50_5050;

/// An image widget's draw, with its position and colour overridden - what
/// [`centred_selector_draw`]/[`tinted_medal_draw`] feed.
pub(crate) fn sprite_draw(image: &Image, placed: Placed, x: f32, y: f32, argb: u32) -> Draw {
    let width = image.width.unwrap_or(placed.width as f32);
    let height = image.height.unwrap_or(placed.height as f32);
    let sampled = [
        image.texture_width.unwrap_or(placed.width as f32),
        image.texture_height.unwrap_or(placed.height as f32),
    ];
    let color = argb_to_rgba(argb);
    if sampled[0] > placed.width as f32 + 0.5 || sampled[1] > placed.height as f32 + 0.5 {
        return Draw::TiledSprite {
            rect: [x, y, width, height],
            uv: [
                placed.x as f32,
                placed.y as f32,
                placed.width as f32,
                placed.height as f32,
            ],
            repeat: [
                sampled[0] / placed.width.max(1) as f32,
                sampled[1] / placed.height.max(1) as f32,
            ],
            color,
        };
    }
    Draw::Sprite {
        rect: [x, y, width, height],
        uv: [
            placed.x as f32 + image.u.unwrap_or(0.0),
            placed.y as f32 + image.v.unwrap_or(0.0),
            sampled[0],
            sampled[1],
        ],
        color,
    }
}

/// `Cell Help`'s own overlay, drawn as a **static** panel - every widget on
/// that screen at its own authored position, with no animation at all. The
/// disc's own copy scrolls a `Viewport`/`Animation` timeline
/// (`LimitVerticalScroll="10"`) that would let `Main Help`/`Speed Class
/// Help`/`Event Help` slide up past each other; `oag_ui::screen::Screens::
/// collect_widgets` already discards a timeline like that everywhere else in
/// this crate and keeps the widget, so drawing the three stacked at their
/// own `y` (all near the panel's own top, since nothing here shifts them
/// down to make room) is the honest next step rather than a scripted one -
/// see `docs/ui/campaign-screens.md`'s own `[Open]` entry. `Speed Class
/// Help`/`Event Help` are the disc's own literal authored text
/// (`MSC_LOAD_VENOM`/`MSC_EVENT_SR`) regardless of the cell actually
/// selected - no per-cell substitution for either idstring is traced (see
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`), so this build
/// does not invent one.
#[must_use]
pub fn cell_help_draw(layout: &Layout, sprites: &dyn Fn(&str) -> Option<Placed>) -> Vec<Draw> {
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
        let Some(content) = text.string.as_deref() else {
            continue;
        };
        out.push(text_draw(text, content, layout));
    }
    out
}

/// **Routes through the `Default`-role atlas when `text.font` names it** -
/// `super::footer::face_role`, the same check `NavigationLegend::draw`
/// already makes for the `Confirm`/`Back` prompts, now meaningful here too:
/// lane 7's own `Default`-role atlas (`crates/game/src/boot/fonts.rs`'s
/// `face_atlas_slot`) is a real, second, mixed-case-capable face, not the
/// upper-case-only menu atlas every campaign-screen label drew through
/// before it landed. `Speed Class`/`Line1`..`8`/`Title`/`Track Line` all
/// author `font="default"` and reach this function, so they pick it up for
/// free. **Not [`Draw::in_role`]**: that helper always hardcodes
/// `wrap_width: None`, which would silently drop the wrap width a
/// `widthlimited="true"` text (`Required`/`Required Previous` on HD) still
/// needs - this preserves `text.wrap_width` on either face instead.
pub(super) fn text_draw(text: &Text, content: &str, layout: &Layout) -> Draw {
    let x = text.x;
    let y = text.y;
    let scale = text.scale * layout.face_scale(&text.font);
    let color = argb_to_rgba(text.color);
    let align = match text.align.to_ascii_lowercase().as_str() {
        "right" => Align::Right,
        "centre" | "center" => Align::Centre,
        _ => Align::Left,
    };
    let content = content.to_string();
    let wrap_width = text.wrap_width;
    match super::footer::face_role(&text.font) {
        Some(role) => Draw::FacedText {
            role,
            x,
            y,
            scale,
            color,
            border: None,
            align,
            text: content,
            wrap_width,
        },
        None => Draw::Text {
            x,
            y,
            scale,
            color,
            border: None,
            align,
            text: content,
            wrap_width,
        },
    }
}
