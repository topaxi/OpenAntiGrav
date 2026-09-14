//! The Race Campaign's two screens: **Grid Selection** and **Cell
//! Selection**.
//!
//! Wipeout Pulse authors both in `Data\Plugins\PI001\GUI\CellMode_Definition.xml`,
//! as `<Screen type="GridSelection" name="Grid Selection">` and
//! `<Screen type="CellSelection" name="Cell Selection">`, and fills them at
//! runtime from the campaign's own 236 `PI_Cell` records
//! (`oag_tables::race_campaign`). See `docs/formats/race-setup.md`'s "The
//! Race Campaign" section, `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`
//! for the decompiled law, and `docs/ui/campaign-screens.md` for what this
//! build measures and draws.
//!
//! **The same rule `oag_ui::picker` follows: the tree is ours, the
//! presentation is the disc's.** Every position, colour and texture sub-rect
//! below comes off `CellMode_Definition.xml` through [`Layout::read`], the
//! same way [`crate::picker::Layout::read`] reads `Track Creation`/`Team
//! Selection`. Nothing here invents a layout number.
//!
//! # This build has no player-progress source, and draws that honestly
//!
//! `Grid_PointsEarned`/`Grid_CountMedalsAtLeast`/`Cell_SavedMedal` all read a
//! saved profile record this project does not keep - see
//! `crates/game/src/records.rs`'s own doc, "nothing selects which `PI_Cell` a
//! launched race corresponds to" - so every grid and cell this draws is in
//! its **fresh-profile** state: zero medals, zero points earned, no saved
//! record. That is not a stand-in; it is what those fields are worth with no
//! progress to report, the same reading `docs/formats/race-setup.md` gives
//! `Team Selection`'s `Loyalty` bar (drawn as absent, never as a zero-filled
//! bar that would read as a real zero).
//!
//! # Stub: draws both screens, stops at confirm
//!
//! Per the gameplay handover's own instruction ("do not implement the
//! cell-selection wiring without first tracing the launch path" -
//! `handover/gameplay/the-race-campaign-is-authored-shape-not-content.md`),
//! [`CellSelection::update`]'s [`Event::Confirmed`] is not wired to
//! `Team Selection`/`Launch Game` here - see `crate::main::session::campaign`
//! in `oag-game` for the composition-root side of the stub.
//!
//! # Three widgets this build deliberately does not draw
//!
//! - **`Lock_x_y` / `Lock_n_0`.** `Locked` on a `PI_Cell`/`PI_Grid` has no
//!   traced consumer (`race-campaign.md`'s "what is not determined",
//!   confidence 50) - drawing a lock glyph from it would be a guess dressed
//!   as a measurement. Every tier and every cell draws open.
//! - **`Line{n} Title`, every `n`.** `CellSelection_PopulateDetail`'s own
//!   table names `Line1`..`Line8` as *values* but never their `Title`
//!   companions, unlike `Target0..2 Title` (`IG_HUD_TARGET`, a real
//!   idstring) or `Medals Title`/`Points Title`/`Required Title` (`RC_GM`/
//!   `RC_TP`/`RC_PN`). Their own authored strings are template junk
//!   (`"l1 title"`, `"--7"`) rather than an idstring, so there is nothing to
//!   resolve and nothing safe to invent; skipped uniformly rather than
//!   drawing the junk or guessing a label.
//! - **`Line4`, `Line5` or `Line8`.** `Line4` never appears in
//!   `PopulateDetail`'s own table at all. `Line5`/`Line8` share one offset
//!   (`Item OffsetX="260" OffsetY="180"`, the same swap idiom
//!   `docs/formats/race-setup.md` documents for `Single Player`'s `Zone`/
//!   `DifficultyNaText`) and both are `Cell_SavedRecord` - the saved best
//!   this build has no record for. All three are left blank.

use oag_gameplay::input::{Button, Input};
use oag_tables::race_campaign::{Cell, Grid, Mode};

use crate::frontend::{Align, Draw, Placed};
use crate::language::StringTable;
use crate::menu::{Frame, Layers, Picture, Skin};
use crate::screen::{Fill, Image, Screen, Screens, Text, argb_to_rgba};

#[cfg(test)]
mod tests;

/// The grid `CellMode_Definition.xml` is authored in on the PSP - see
/// `crate::picker::PSP_GRID`, which this mirrors for the same PS2-scaling
/// reason.
const PSP_GRID: [f32; 2] = [480.0, 272.0];

/// How many grid tiers `Grid Selection`'s own `GridController` shows at
/// once - `MaxX="4" MaxY="1"`, confirmed by `GridSelection_Update`'s own
/// `honey` formula (`index*4+1, index*4+4, max*4+4` against 16 grids: `max`
/// only closes at 3 if it counts *pages* of four, not grids or tiers
/// directly). See `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`.
const GRIDS_PER_PAGE: usize = 4;

/// One selectable thing an [`Event`] can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The selection moved - a new tier on `Grid Selection`, a new cell on
    /// `Cell Selection`.
    Moved,
    /// The player confirmed. **Not wired past this crate** - see the module
    /// doc's stub section.
    Confirmed,
    /// `Cell Selection` only: `Cell Help` opened or closed (`triangle`).
    Help,
    /// The player backed out.
    Back,
}

/// A grid tier, reduced to what `GridSelection_Update` binds - see the
/// module doc for why every earned figure here is zero rather than read off
/// a save this project does not keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridSummary {
    /// The grid's own name, e.g. `"grid0"` - what `GridSelection_Update`
    /// binds `Title` to directly, with no further formatting. Reads as
    /// literally `"grid0"` on screen, which looks like an internal id
    /// because it is one; no friendlier label is authored anywhere this
    /// build has read. See `docs/ui/campaign-screens.md`.
    pub name: String,
    /// `Grid_CellCount`: `grid.cells.len()`.
    pub cell_count: u32,
    /// `Grid_PointsPossible`: `3 * cell_count` - [`Grid::max_points`].
    pub max_points: u32,
    /// `grid->RequiredPoints`. `0` renders as `FE_NA` (`grid15`'s own value).
    pub required_points: u32,
}

impl GridSummary {
    #[must_use]
    pub fn from_grid(grid: &Grid) -> Self {
        Self {
            name: grid.name.clone(),
            cell_count: u32::try_from(grid.cells.len()).unwrap_or(u32::MAX),
            max_points: grid.max_points(),
            required_points: grid.required_points,
        }
    }
}

/// `Grid Selection`: sixteen tiers, paged four at a time.
#[derive(Debug, Clone)]
pub struct GridSelection {
    grids: Vec<GridSummary>,
    index: usize,
}

impl GridSelection {
    #[must_use]
    pub fn new(grids: Vec<GridSummary>) -> Self {
        Self { grids, index: 0 }
    }

    #[must_use]
    pub fn grids(&self) -> &[GridSummary] {
        &self.grids
    }

    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Moves the selection directly to `index`, clamped to the last grid -
    /// how `Cell Selection`'s own `Back` lands `Grid Selection` on the tier
    /// it was opened from, rather than resetting to the first.
    pub fn set_index(&mut self, index: usize) {
        self.index = index.min(self.grids.len().saturating_sub(1));
    }

    #[must_use]
    pub fn selected(&self) -> Option<&GridSummary> {
        self.grids.get(self.index)
    }

    /// Which page of [`GRIDS_PER_PAGE`] is on screen.
    #[must_use]
    pub fn page(&self) -> usize {
        self.index / GRIDS_PER_PAGE
    }

    /// The selected tier's own hex slot within its page, `0..GRIDS_PER_PAGE`.
    #[must_use]
    pub fn slot(&self) -> usize {
        self.index % GRIDS_PER_PAGE
    }

    /// The `honey` counter's own formula: `"{first}-{last} / {total}"`, where
    /// `first`/`last` are the current page's 1-based bounds. See
    /// `GridSelection_Update`'s `"%d-%d / %d"` binding.
    #[must_use]
    pub fn counter(&self) -> String {
        let page = self.page();
        let first = page * GRIDS_PER_PAGE + 1;
        let last = ((page + 1) * GRIDS_PER_PAGE).min(self.grids.len().max(1));
        format!("{first}-{last} / {}", self.grids.len())
    }

    /// Up/down wrap over every grid, one at a time - the same `Picker::update`
    /// idiom `oag_ui::picker` already uses for a wrapping list. Left/right are
    /// inert: nothing in `CellMode_Definition.xml` authors a left/right arrow
    /// on this screen, unlike `Track Creation`'s livery row.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if self.grids.is_empty() {
            return out;
        }
        if input.take(Button::Down) {
            self.index = (self.index + 1) % self.grids.len();
            out.push(Event::Moved);
        }
        if input.take(Button::Up) {
            self.index = (self.index + self.grids.len() - 1) % self.grids.len();
            out.push(Event::Moved);
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            out.push(Event::Confirmed);
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        out
    }
}

/// `Cell Selection`: the 32-position staggered hex grid, filled to whichever
/// of the 8-16 cells the selected [`Grid`] actually carries.
#[derive(Debug, Clone)]
pub struct CellSelection {
    cells: Vec<Cell>,
    index: usize,
    help_open: bool,
}

impl CellSelection {
    /// `cells` is one [`Grid`]'s own list, in document order. `index` starts
    /// on the first cell the grid names - not necessarily hex position
    /// `(0, 0)`, since not every grid fills that slot.
    #[must_use]
    pub fn new(cells: Vec<Cell>) -> Self {
        Self {
            cells,
            index: 0,
            help_open: false,
        }
    }

    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    #[must_use]
    pub fn selected(&self) -> Option<&Cell> {
        self.cells.get(self.index)
    }

    #[must_use]
    pub fn help_open(&self) -> bool {
        self.help_open
    }

    /// Moves to the cell nearest `(dx, dy)` away from the current one, in the
    /// hex grid's own coordinate space (`Cell::grid_coords`, not screen
    /// pixels). **Chosen, not measured**: the original's own directional
    /// adjacency on a staggered hex grid is unread - see
    /// `docs/ui/campaign-screens.md`. This picks whichever other cell's own
    /// `(x, y)` has the most positive dot product with `(dx, dy)` and, among
    /// ties, the smallest perpendicular offset - which reduces to "the
    /// nearest neighbour in roughly that direction" without needing the
    /// staggered geometry spelled out twice.
    fn step(&mut self, dx: f32, dy: f32) -> bool {
        let Some(current) = self.selected().and_then(Cell::grid_coords) else {
            return false;
        };
        let (cx, cy) = (
            f32::from(u16::try_from(current.0).unwrap_or(u16::MAX)),
            f32::from(u16::try_from(current.1).unwrap_or(u16::MAX)),
        );
        let mut best: Option<(usize, f32, f32)> = None;
        for (index, cell) in self.cells.iter().enumerate() {
            if index == self.index {
                continue;
            }
            let Some((x, y)) = cell.grid_coords() else {
                continue;
            };
            let (ex, ey) = (
                f32::from(u16::try_from(x).unwrap_or(u16::MAX)) - cx,
                f32::from(u16::try_from(y).unwrap_or(u16::MAX)) - cy,
            );
            let along = ex * dx + ey * dy;
            if along <= 0.0 {
                continue;
            }
            let perpendicular = (ex * dy - ey * dx).abs();
            let better = match &best {
                None => true,
                Some((_, best_perp, best_along)) => {
                    perpendicular < *best_perp
                        || ((perpendicular - best_perp).abs() < f32::EPSILON && along < *best_along)
                }
            };
            if better {
                best = Some((index, perpendicular, along));
            }
        }
        if let Some((index, _, _)) = best {
            self.index = index;
            true
        } else {
            false
        }
    }

    /// The four directions and the three buttons this screen answers to.
    /// Movement is inert while [`Self::help_open`] - the disc's own `Watch`
    /// element redirects every directional press straight back to `Cell
    /// Help` while it is the present screen, which this reproduces as "do
    /// nothing" rather than modelling a redirect loop.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if input.take(Button::Triangle) {
            self.help_open = !self.help_open;
            out.push(Event::Help);
            return out;
        }
        if self.help_open {
            if input.take(Button::Cross) || input.take(Button::Circle) {
                self.help_open = false;
                out.push(Event::Help);
            }
            input.take(Button::Up);
            input.take(Button::Down);
            input.take(Button::Left);
            input.take(Button::Right);
            return out;
        }
        if input.take(Button::Down) && self.step(0.0, 1.0) {
            out.push(Event::Moved);
        }
        if input.take(Button::Up) && self.step(0.0, -1.0) {
            out.push(Event::Moved);
        }
        if input.take(Button::Right) && self.step(1.0, 0.0) {
            out.push(Event::Moved);
        }
        if input.take(Button::Left) && self.step(-1.0, 0.0) {
            out.push(Event::Moved);
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            out.push(Event::Confirmed);
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        out
    }
}

/// The disc's own layout for one campaign screen, with every fixed string
/// already resolved.
#[derive(Debug, Clone)]
pub struct Layout {
    pub screen: Screen,
    /// How much larger this screen's grid is than the PSP's - see
    /// `crate::picker::Layout::scale`.
    pub scale: [f32; 2],
    /// The `default`/`small` faces' own scale against the loaded `menu`
    /// face - see `crate::picker::FaceScales`, which this reuses rather than
    /// duplicating: both screens are authored in the same three faces
    /// `Selection_Definition.xml` is.
    pub faces: crate::picker::FaceScales,
}

impl Layout {
    /// Reads `Grid Selection` or `Cell Selection` off the parsed
    /// `CellMode_Definition.xml`, resolving every `idstring` through
    /// `strings` - the same second pass `crate::picker::Layout::read` makes,
    /// so a fixed label like `Medals Title` (`RC_GM`) or `Target0 Title`
    /// (`IG_HUD_TARGET`) already carries its text by the time a draw
    /// function's generic `text.string.clone()` fallback reaches it.
    /// `None` when the screen is not in `screens` at all.
    #[must_use]
    pub fn read(
        screens: &Screens,
        name: &str,
        strings: &StringTable,
        faces: crate::picker::FaceScales,
        grid: [f32; 2],
    ) -> Option<Self> {
        let mut screen = screens.by_name(name)?.clone();
        for text in &mut screen.texts {
            if let Some(id) = text.idstring.as_deref()
                && let Some(resolved) = strings.get(id)
            {
                text.string = Some(resolved.to_string());
            }
        }
        let scale = [grid[0] / PSP_GRID[0], grid[1] / PSP_GRID[1]];
        Some(Self {
            screen,
            scale,
            faces,
        })
    }

    fn face_scale(&self, font: &str) -> f32 {
        match font.to_ascii_lowercase().as_str() {
            "default" => self.faces.default,
            "small" => self.faces.small,
            _ => 1.0,
        }
    }
}

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
            && let Some(slot_index) =
                hex_slot_of(name, "Medal_", 1).or_else(|| hex_slot_of(name, "Outline_", 1))
        {
            if name.starts_with("Lock_") {
                continue;
            }
            if page * GRIDS_PER_PAGE + slot_index >= model.grids().len() {
                continue;
            }
        }
        if image
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("Lock_"))
        {
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        if image.name.as_deref() == Some("Selector") {
            if let Some((x, y)) = hex_position(screen, "Medal_", slot, 0) {
                out.push(sprite_draw(image, placed, x, y));
            }
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
            "Title" => Some(selected.name.clone()),
            "Medals" => Some(format!("00/{:02}", selected.cell_count)),
            "Points" => Some(format!("000/{:03}", selected.max_points)),
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
    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }
    let occupied: Vec<(u32, u32)> = model.cells().iter().filter_map(Cell::grid_coords).collect();
    let selected_coords = model.selected().and_then(Cell::grid_coords);
    for image in &screen.images {
        if let Some(name) = image.name.as_deref() {
            if name.starts_with("Lock_") {
                continue;
            }
            if let Some((x, y)) =
                hex_slot_xy(name, "Medal_").or_else(|| hex_slot_xy(name, "Outline_"))
                && !occupied.contains(&(x, y))
            {
                continue;
            }
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        if image.name.as_deref() == Some("Selector")
            && let Some((sx, sy)) = selected_coords
            && let Some((x, y)) = hex_position(screen, "Medal_", sx as usize, sy as usize)
        {
            out.push(sprite_draw(image, placed, x, y));
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
        if name.starts_with("Line") && name.ends_with("Title") {
            // No traced consumer - see the module doc.
            continue;
        }
        let content = match name {
            "Title" => Some(cell.mode.as_str().to_string()),
            "Track Line" => Some(track_line(cell)),
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
                "0/{}",
                oag_tables::race_campaign::Medal::Gold.points()
            )),
            "Line7" => Some(strings.get_or_id("MSC_NONE").to_string()),
            "Target0 Title" | "Target1 Title" | "Target2 Title" if targets_visible => {
                Some(strings.get_or_id("IG_HUD_TARGET").to_string())
            }
            "Target0" if targets_visible => Some(target_value(cell.gold, cell.mode)),
            "Target1" if targets_visible => Some(target_value(cell.silver, cell.mode)),
            "Target2" if targets_visible => Some(target_value(cell.bronze, cell.mode)),
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

fn track_line(cell: &Cell) -> String {
    if cell.mode == Mode::Tournament {
        format!("{} Races", cell.tournament_tracks.len())
    } else {
        cell.track.clone().unwrap_or_default()
    }
}

fn laps_line(cell: &Cell, strings: &StringTable) -> String {
    match cell.laps {
        Some(laps) if laps >= 1 && cell.mode != Mode::Zone => laps.to_string(),
        _ => strings.get_or_id("RC_INF").to_string(),
    }
}

/// `Target0..2`'s own value: a time for `Time Trial`/`Speed Lap`, a plain
/// number otherwise (`Zone`'s zone count, `Elimination`'s kill count).
fn target_value(value: i64, mode: Mode) -> String {
    if matches!(mode, Mode::TimeTrial | Mode::SpeedLap) {
        format_centiseconds(value)
    } else {
        value.to_string()
    }
}

/// Centiseconds to `M:SS.CC`, the unit `PI_Cell_ParseElement` stores a
/// `Gold`/`Silver`/`Bronze Target` in for a timed mode - see
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`.
fn format_centiseconds(value: i64) -> String {
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

/// The resolved screen position of `{prefix}{x}_{y}` - the position the
/// cursor moves to, since `Selector` is one movable widget rather than one
/// per cell.
fn hex_position(screen: &Screen, prefix: &str, x: usize, y: usize) -> Option<(f32, f32)> {
    let name = format!("{prefix}{x}_{y}");
    screen
        .images
        .iter()
        .find(|image| image.name.as_deref() == Some(name.as_str()))
        .map(|image| (image.x, image.y))
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
    sprite_draw(image, placed, image.x, image.y)
}

/// An image widget's draw, with its position overridden - what
/// [`hex_position`] feeds `Selector`.
fn sprite_draw(image: &Image, placed: Placed, x: f32, y: f32) -> Draw {
    let width = image.width.unwrap_or(placed.width as f32);
    let height = image.height.unwrap_or(placed.height as f32);
    let sampled = [
        image.texture_width.unwrap_or(placed.width as f32),
        image.texture_height.unwrap_or(placed.height as f32),
    ];
    let color = argb_to_rgba(image.color);
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
