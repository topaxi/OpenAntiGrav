//! `Grid Selection`/`Cell Selection`'s own draw lists, split out of
//! `campaign.rs` once that file passed the 1,000-line rule
//! (`scripts/check-file-size.py`) - a move, with no behaviour change to the
//! functions carried across. The model types (`GridSelection`,
//! `CellSelection`, `Layout`, `GridSummary`) stay in the parent module;
//! this file only turns one into a [`crate::menu::Layers`].

use oag_tables::race_campaign::{Cell, Medal, Mode};

use crate::frontend::{Align, Draw, Placed};
use crate::language::StringTable;
use crate::menu::{Frame, Layers, Picture, Skin};
use crate::screen::{Fill, Image, Text, argb_to_rgba};

use super::{CellSelection, GRIDS_PER_PAGE, GridSelection, Layout, hex_rect};

#[cfg(test)]
mod tests;

/// `Grid Selection`'s draw list.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
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
                out.push(centred_selector_draw(image, placed, hex));
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
            "Medals" => Some(format!(
                "{:02}/{:02}",
                selected.gold_medals, selected.cell_count
            )),
            "Points" => Some(format!(
                "{:03}/{:03}",
                selected.points_earned, selected.max_points
            )),
            "Required" => Some(if selected.required_points == 0 {
                strings.get_or_id("FE_NA").to_string()
            } else {
                selected.required_points.to_string()
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
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
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
            out.push(centred_selector_draw(image, placed, hex));
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
            "Line4" | "Line5" | "Line8" => None,
            "Line6" => Some(format!(
                "{}/{}",
                model.selected_medal().map_or(0, Medal::points),
                Medal::Gold.points()
            )),
            "Line7" if !targets_visible => Some(medal_line(model.selected_medal(), strings)),
            "Line7" => None,
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
/// [`crate::menu::mode_label`]'s own `MSC_EVENT_*`-head-before-the-colon
/// reading, which [`oag_race::Mode`] does not have variants to receive
/// `Tournament`/`Head2Head` through - `oag_ui` cannot call
/// `oag_game::campaign::race_mode_for_cell` either way, the dependency rule
/// this crate is on the wrong side of (`crate::campaign`'s own module doc
/// links the two). `MSC_EVENT_TOURN`/`MSC_EVENT_HTH` exist on disc and are
/// read the identical way, unmeasured against a live frame but the same
/// mechanism as the five that are. `Custom Grid`/`AI Race`/`Other` fall back
/// to the raw spelling, per `docs/ui/campaign-screens.md`.
fn cell_title(cell: &Cell, strings: &StringTable) -> String {
    match cell.mode {
        Mode::Race => crate::menu::mode_label(oag_race::Mode::SingleRace, strings),
        Mode::TimeTrial => crate::menu::mode_label(oag_race::Mode::TimeTrial, strings),
        Mode::Zone => crate::menu::mode_label(oag_race::Mode::Zone, strings),
        Mode::Elimination => crate::menu::mode_label(oag_race::Mode::Eliminator, strings),
        Mode::SpeedLap => crate::menu::mode_label(oag_race::Mode::SpeedLap, strings),
        Mode::Tournament => mode_event_label("MSC_EVENT_TOURN", strings, cell.mode.as_str()),
        Mode::Head2Head => mode_event_label("MSC_EVENT_HTH", strings, cell.mode.as_str()),
        Mode::CustomGrid | Mode::AiRace | Mode::Other(_) => cell.mode.as_str().to_string(),
    }
}

/// The head-before-the-colon reading [`crate::menu::mode_label`] applies to
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
/// HD, Pulse's single copy of the table needs no [`crate::language::CircuitNames`]
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
/// `SILVER`/`BRONZE`, `MSC_NONE` for no saved medal.
/// `race-campaign.md` records this table but not the difficulty suffix
/// `CellSelection_PopulateDetail` appends (`Cell_SavedDifficulty`) - not
/// drawn here, since this build keeps no per-cell saved difficulty at all.
fn medal_line(medal: Option<Medal>, strings: &StringTable) -> String {
    let id = match medal {
        Some(Medal::Gold) => "IG_HUD_GOLD",
        Some(Medal::Silver) => "IG_HUD_SILVER",
        Some(Medal::Bronze) => "IG_HUD_BRONZE",
        None => "MSC_NONE",
    };
    strings.get_or_id(id).to_string()
}

fn laps_line(cell: &Cell, strings: &StringTable) -> String {
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
fn hex_slot_xy(name: &str, prefix: &str) -> Option<(u32, u32)> {
    let rest = name.strip_prefix(prefix)?;
    let mut parts = rest.split('_');
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    Some((x, y))
}

fn fill_draw(fill: &Fill) -> Draw {
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
fn image_draw(image: &Image, placed: Placed) -> Draw {
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
fn medal_argb(medal: Medal) -> u32 {
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
fn tinted_medal_draw(image: &Image, placed: Placed, argb: u32) -> Draw {
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
fn centred_selector_draw(image: &Image, placed: Placed, hex: [f32; 4]) -> Draw {
    let width = image.width.unwrap_or(placed.width as f32);
    let height = image.height.unwrap_or(placed.height as f32);
    let x = hex[0] + (hex[2] - width) * 0.5;
    let y = hex[1] + (hex[3] - height) * 0.5;
    sprite_draw(image, placed, x, y, image.color)
}

/// An image widget's draw, with its position and colour overridden - what
/// [`centred_selector_draw`]/[`tinted_medal_draw`] feed.
fn sprite_draw(image: &Image, placed: Placed, x: f32, y: f32, argb: u32) -> Draw {
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

fn text_draw(text: &Text, content: &str, layout: &Layout) -> Draw {
    Draw::Text {
        x: text.x,
        y: text.y,
        scale: text.scale * layout.face_scale(&text.font),
        color: argb_to_rgba(text.color),
        border: None,
        align: match text.align.to_ascii_lowercase().as_str() {
            "right" => Align::Right,
            "centre" | "center" => Align::Centre,
            _ => Align::Left,
        },
        text: content.to_string(),
        wrap_width: text.wrap_width,
    }
}
