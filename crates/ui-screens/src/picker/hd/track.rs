//! Wipeout HD/Fury's own `Track Creation` - the circuit screen the race box's
//! `Single Player` page redirects to, read off a different file in a
//! different dialect from Pulse's.
//!
//! # Where it is, and that it is live
//!
//! `Data\Plugins\Frontend\Gui\Track_Selection_Definition.xml`, which only
//! `DATA06` carries. `DATA00`'s `skin.xml` includes it by `SrcRel`. The five
//! archives that carry `racebox_definition.xml` do not author the screen:
//! that file only names `Track Creation` as the `goto` of `Single Player`'s
//! default redirect. The campaign path never shows it - `Cell Selection`
//! redirects straight to `Team Selection`. See `docs/ui/campaign-screens.md`.
//!
//! # The nested screen
//!
//! As the ship screen: every widget sits on `TrackSelectionTopLevel` and the
//! `Track Creation` screen nested in it holds the title, the RECORDS table and
//! the redirects. [`read`] merges the parent's widgets under the child's name.
//! The sibling `Tournament C` screen is a different screen and is not read.
//!
//! # What this draws, and what it does not
//!
//! Drawn, off the disc: the title, the three `MiniText` headings, the emblem
//! frame, the circuit's own `Emblem` (`<environment>\FE\TrackSelectEmblem_Fury.gtf`,
//! the widget authors no `src`), the circuit name, the length and distance
//! rows, and the RECORDS table's header and empty cells.
//!
//! Also drawn: the `TrackHexSelection` grid, a circuit per hexagon (see
//! [`draw_grid`]).
//!
//! The `TrackModel` circuit model is a 3-D pass over this screen's draws:
//! [`TrackScreen::model`] carries its authored pose, `oag_game::preview::track_model`
//! places and shades it, and the scene each circuit draws is
//! `oag_title::FrontEnd::circuit_models`.
//!
//! Not drawn, and said so: the `FlyByMovie` (`preview.bik`, Bink), the page
//! squares, the `Padlock` (HD's circuit gate is open) and the small direction
//! glyphs on the forward hexagon.

use oag_ui::frontend::{Align, Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::menu::Frame;
use oag_ui::screen::{Screens, argb_to_rgba};

use super::hex::{self, ArtSize, Face};
use super::{TeamScreen, draw_labels, find_screen, find_text, sprite, text_draw, walk};
use crate::picker::{Details, Entry, FaceScales, Layout, Picker};

/// The screen every widget is authored on.
pub const TOP_LEVEL: &str = "TrackSelectionTopLevel";
/// The screen nested inside [`TOP_LEVEL`] that `Single Player` redirects to.
pub const SCREEN: &str = "Track Creation";

/// The three ids the RECORDS table's row labels are looked up under. The
/// widgets author `string="16"` (a width, not text) and the original fills
/// them in code; the ids are **chosen** - they are the string table's own
/// entries whose text is the `PERSONAL`, `FRIENDS` and `GLOBAL` an RPCS3
/// frame of the screen shows, in that order.
pub const RECORD_ROW_IDS: [&str; 3] = ["FE_PERSONAL", "FE_FRIENDS", "FE_GLOBAL"];

/// A circuit's icon, in units across, in its hexagon: the selected Vineta K
/// cell's sails span 55 px of a 2000 px frame, 58 units at 0.9424 px per
/// unit, and the icon texture is drawn at the size that puts its own sails
/// there (**chosen** from one frame, the texture's fill not measured).
const ICON_SIZE: f32 = 62.0;
/// The cursor ring's tint, off the frame's pale ring (221/155/162).
const RING: [f32; 4] = [0.87, 0.61, 0.64, 1.0];

/// What a widget the generic parser does not reach says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackScreen {
    /// The `MiniText` headings and the bracket rects, as [`TeamScreen`]
    /// collects them - the rest of that struct is the ship screen's.
    pub common: TeamScreen,
    /// The `Emblem` image's rect. It authors no `src`.
    pub emblem: Option<[f32; 4]>,
    /// `TrackModel`'s own pose, which `oag_game::preview::track_model` draws
    /// the circuit's scene at.
    pub model: Option<TrackModel>,
    /// The `TrackHexSelection` widget: circuits on hexagons, see [`super::hex`].
    pub hex_grid: Option<hex::HexGrid>,
    /// The RECORDS table's three row labels, resolved from
    /// [`RECORD_ROW_IDS`].
    pub row_labels: [String; 3],
}

/// The `<Model name="TrackModel">` widget's values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackModel {
    pub origin: [f32; 2],
    /// The widget's `x` and `y`, which place the model beside the axis:
    /// `0 0` on HD, `0.525 0.125` on Omega.
    pub offset: [f32; 2],
    pub z: f32,
    /// `orthoScaleX/Y/Z` (or `orthoScale` for all three), a scale on the
    /// placement (`x`, `y` and `z` stand `1 / scale` times as far, see
    /// `oag_game::flyer::campaign_pose`): `1` where the widget authors none
    /// (HD), `0.012` on Omega.
    pub ortho_scale: [f32; 3],
}

/// `<environment>\FE\TrackSelectEmblem_Fury.gtf` - what the `Emblem` widget
/// shows for the circuit whose environment folder is `location`.
///
/// **Measured to exist, chosen to be the one the widget shows**: every base
/// environment on `DATA00` carries a `fe\trackselectemblem_fury.gtf`, and
/// the frame shows a per-circuit emblem in the `Emblem` rect; `DATA02` holds
/// an older `trackselectemblem.gtf` in `hd_textures` that no `FE` folder
/// names. `pub` so the boot can put every circuit's on the sheet.
#[must_use]
pub fn emblem_src(location: &str) -> String {
    format!(
        r"{}\FE\TrackSelectEmblem_Fury.gtf",
        location.trim_end_matches('\\')
    )
}

/// `<environment>\FE\TrackSelectEmblem_BW.gtf` - the white icon a circuit's
/// hexagon carries in the `TrackHexSelection` grid. It is the file `Cell
/// Selection`'s `Track Emblem` draws ([`crate::campaign::hd::cell_emblems`]),
/// and the frame's hexagons show the same pictures as the circuit's own
/// emblem in white, so the path is that module's. **Chosen, not measured**:
/// the widget names no art; the match is by picture on one frame.
#[must_use]
pub fn grid_icon_src(location: &str) -> String {
    crate::campaign::hd::cell_emblems::track_emblem_src(location)
}

/// How a circuit's hexagon is arranged in `picker`: `(circuits per row,
/// row count)`. The entries are every forward circuit then every reverse one,
/// so a picker without that shape (no reverse twin for some circuit) is one
/// row of everything.
fn shape(picker: &Picker) -> (usize, usize) {
    let width = picker.row_width();
    if width == 0 {
        (picker.entries().len(), 1)
    } else {
        (width, picker.entries().len().div_ceil(width))
    }
}

/// The grid: one column per circuit, the centre column the selected one,
/// both rows red there and the cursor ring on the chosen direction's row.
/// Circuits wrap round the list. **Chosen, not measured**: the widget
/// authors no colour, so the hexagon fills are `Team Selection`'s authored
/// `HexCol` / `SelectedColumnCol` values (the frame's red and dim grey read
/// the same on both screens) and the ring is the frame's pale pink; the
/// small direction glyphs under the forward hexagon's emblem are not drawn
/// (their art is unlocated). Nothing locks: HD's circuit gate is open
/// (Pulse's law, unmeasured on HD), so no cell shows a padlock.
fn draw_grid(
    grid: &hex::HexGrid,
    picker: &Picker,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    out: &mut Vec<Draw>,
) {
    let (width, rows) = shape(picker);
    let face = |cell: &hex::Cell| match picker.entries().get(cell.row * width + cell.entry) {
        Some(Entry {
            details: Details::Track {
                icon: Some(src), ..
            },
            ..
        }) => Face::Art {
            src: src.clone(),
            size: ArtSize::Fit(ICON_SIZE),
        },
        _ => Face::Empty,
    };
    let mut grid = *grid;
    grid.rows = grid.rows.min(rows);
    hex::draw_cells(
        &grid,
        (picker.index() % width.max(1), width),
        picker.pan(),
        Some(picker.index() / width.max(1)),
        RING,
        &face,
        sprites,
        out,
    );
}

/// The `CIRCUIT LENGTH` and `RACE DISTANCE` values for a lap of `metres`
/// run `laps` times, as the frame prints them: kilometres to one decimal
/// place, `4.4KM` and `13.2KM` on Vineta K. **Chosen**: one frame, whose two
/// numbers are consistent with three laps of the length the circuit's own
/// spline measures, so the unit and precision are read and the lap count is
/// the race's own setting. A race with no lap target has no distance, and
/// its row is the empty string: the screen authors an `Infinity` glyph
/// (`Infinity_symbol.gtf`) for exactly that and [`body`] draws it there.
#[must_use]
pub fn length_rows(metres: f32, laps: Option<u32>) -> [String; 2] {
    [
        format!("{:.1}KM", metres / 1000.0),
        laps.map_or_else(String::new, |laps| {
            format!("{:.1}KM", metres * laps as f32 / 1000.0)
        }),
    ]
}

/// Reads HD's `Track Creation` off its own definition file. `xml` is the
/// expanded text and `screens` the same file parsed with the front end's
/// globals. `None` when either screen is missing.
#[must_use]
pub fn read(
    xml: &str,
    screens: &Screens,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
) -> Option<Layout> {
    let parent = screens.by_name(TOP_LEVEL)?;
    let child = screens.by_name(SCREEN)?;
    let mut screen = parent.clone();
    screen.name = child.name.clone();
    screen.kind.clone_from(&child.kind);
    screen.path.clone_from(&child.path);
    screen.texts.extend(child.texts.iter().cloned());
    screen.fills.extend(child.fills.iter().cloned());
    screen.images.extend(child.images.iter().cloned());
    screen.blocks.extend(child.blocks.iter().cloned());
    screen.redirects.clone_from(&child.redirects);
    for text in &mut screen.texts {
        if let Some(id) = text.idstring.as_deref()
            && let Some(resolved) = strings.get(id)
        {
            text.string = Some(resolved.to_string());
        }
    }
    let title = child
        .texts
        .iter()
        .find(|text| text.font.eq_ignore_ascii_case("Title"))
        .and_then(|text| text.idstring.as_deref())
        .map_or_else(
            || SCREEN.to_string(),
            |id| strings.get_or_id(id).to_string(),
        );

    let root = oag_ui::screen::parse(xml);
    let top = find_screen(&root, TOP_LEVEL)?;
    let mut extra = TrackScreen::default();
    walk(top, (0.0, 0.0), screens, strings, &mut extra.common);
    if let Some(nested) = find_screen(top, SCREEN) {
        walk(nested, (0.0, 0.0), screens, strings, &mut extra.common);
    }
    walk_track(top, (0.0, 0.0), screens, &mut extra);
    extra.row_labels = RECORD_ROW_IDS.map(|id| strings.get_or_id(id).to_string());

    let panel = extra.common.brackets.first().map_or([0.0; 4], |b| b.rect);
    let preview = extra
        .common
        .brackets
        .iter()
        .find(|b| b.middle)
        .map_or(panel, |b| b.rect);
    Some(Layout {
        title,
        screen,
        preview,
        panel,
        faces,
        scale: [
            grid[0] / crate::picker::PSP_GRID[0],
            grid[1] / crate::picker::PSP_GRID[1],
        ],
        hd: None,
        hd_track: Some(Box::new(extra)),
    })
}

fn walk_track(
    node: &oag_ui::screen::Node,
    offset: (f32, f32),
    screens: &Screens,
    out: &mut TrackScreen,
) {
    let number = |node: &oag_ui::screen::Node, attr: &str| screens.number(node.value(attr));
    for child in &node.children {
        let name = child.name.to_ascii_lowercase();
        if name == "values" {
            continue;
        }
        let inner = (
            offset.0 + number(child, "OffsetX").unwrap_or(0.0),
            offset.1 + number(child, "OffsetY").unwrap_or(0.0),
        );
        let x = inner.0 + number(child, "x").unwrap_or(0.0);
        let y = inner.1 + number(child, "y").unwrap_or(0.0);
        match name.as_str() {
            "image" if child.attr("name") == Some("Emblem") => {
                out.emblem = Some([
                    x,
                    y,
                    number(child, "width").unwrap_or(0.0),
                    number(child, "height").unwrap_or(0.0),
                ]);
            }
            "model" if child.attr("name") == Some("TrackModel") => {
                out.model = Some(TrackModel {
                    origin: [
                        number(child, "OriginX").unwrap_or(0.0),
                        number(child, "OriginY").unwrap_or(0.0),
                    ],
                    offset: [
                        number(child, "x").unwrap_or(0.0),
                        number(child, "y").unwrap_or(0.0),
                    ],
                    z: number(child, "z").unwrap_or(0.0),
                    ortho_scale: ["X", "Y", "Z"].map(|axis| {
                        number(child, &format!("orthoScale{axis}"))
                            .or_else(|| number(child, "orthoScale"))
                            .filter(|scale| *scale > 0.0)
                            .unwrap_or(1.0)
                    }),
                });
            }
            "trackhexselection" => {
                out.hex_grid = Some(hex::HexGrid::unauthored(
                    [inner.0, inner.1],
                    number(child, "columns").map_or(0, |v| v as usize),
                    number(child, "rows").map_or(0, |v| v as usize),
                ));
            }
            _ => {}
        }
        if name != "screen" || child.attr("name") == Some(SCREEN) {
            walk_track(child, inner, screens, out);
        }
    }
}

/// The RECORDS table's cell text: the row label in column 1 and `---` in
/// the rest, which is what an RPCS3 frame of a fresh profile shows in all
/// twelve. This build keeps no per-circuit lap record in the shape the table
/// wants (name, team, time), so no cell is ever anything else: **open**.
fn record_cell(row: usize, column: usize, labels: &[String; 3]) -> String {
    if column == 1 {
        labels[row - 1].clone()
    } else {
        "---".to_string()
    }
}

fn info_text(text: &oag_ui::screen::Text, content: &str) -> Draw {
    Draw::Text {
        x: text.x,
        y: text.y,
        scale: text.scale,
        color: argb_to_rgba(text.color),
        border: None,
        align: Align::Left,
        text: content.to_string(),
        wrap_width: None,
    }
}

fn entry_info(picker: &Picker) -> Option<&[String; 3]> {
    match picker.selected().map(|entry| &entry.details) {
        Some(Details::Track { info, .. }) => Some(info),
        _ => None,
    }
}

/// The body of the screen.
pub(in crate::picker) fn body(
    picker: &Picker,
    layout: &Layout,
    extra: &TrackScreen,
    frame: &Frame,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Vec<Draw> {
    let mut out = Vec::new();
    let screen = &layout.screen;
    let reversed = matches!(
        picker.selected().map(|entry| &entry.details),
        Some(Details::Track { reversed: true, .. })
    );
    for fill in &screen.fills {
        out.push(crate::picker::fill_draw(fill));
    }
    for image in &screen.images {
        let name = image.name.as_deref().unwrap_or("");
        if name == "Infinity" {
            let unlimited = matches!(&entry_info(picker), Some(info) if info[1].is_empty());
            if unlimited && let Some(placed) = sprites(&image.src) {
                out.push(sprite(image, placed, None));
            }
            continue;
        }
        // The reverse glyph and its `REVERSE` text are authored for a
        // reversed circuit; they show only then (measured 2026-09-29: `Down`
        // drew them, `Up` removed them).
        if name.starts_with("ReverseIcon") {
            if reversed && let Some(placed) = sprites(&image.src) {
                out.push(sprite(image, placed, None));
            }
            continue;
        }
        if matches!(name, "Padlock" | "Emblem") || name.starts_with("furyship") {
            continue;
        }
        if let Some(placed) = sprites(&image.src) {
            out.push(sprite(image, placed, None));
        }
    }
    draw_labels(&extra.common.labels, &mut out);
    if let Some(grid) = &extra.hex_grid {
        draw_grid(grid, picker, sprites, &mut out);
    }
    let Some(entry) = picker.selected() else {
        return out;
    };
    if let Some(rect) = extra.emblem
        && let Details::Track {
            emblem: Some(src), ..
        } = &entry.details
        && let Some(placed) = sprites(src)
    {
        out.push(Draw::Sprite {
            rect,
            uv: [
                placed.x as f32,
                placed.y as f32,
                placed.width as f32,
                placed.height as f32,
            ],
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    if let Some(name) = find_text(screen, "TrackName") {
        out.push(text_draw(name, &entry.label, layout));
    }
    let Details::Track { info, .. } = &entry.details else {
        return out;
    };
    // The two info rows author `scale="0.45"` on the default font and the
    // frame shows them the size of the `MiniText` headings, which is 0.45 of
    // the menu face: so no face ratio on top, as `text_draw` would apply.
    for (index, label) in ["Info1", "Info2"].iter().enumerate() {
        if let Some(text) = find_text(screen, label) {
            out.push(info_text(text, info.get(index).map_or("", String::as_str)));
        }
        if let Some(title) = find_text(screen, &format!("{label} Title")) {
            out.push(info_text(title, title.string.as_deref().unwrap_or("")));
        }
    }
    // The `RECORDS` heading beside its arrow: the one loose text that
    // repeats a block's id, told apart by standing where no block does.
    for text in &screen.texts {
        let at_a_block = screen
            .blocks
            .iter()
            .any(|block| (text.x - block.x).abs() < 0.5 && (text.y - block.y).abs() < 0.5);
        let heading = text.idstring.as_deref() == Some("FE_RECORDS") && !at_a_block;
        if heading || (reversed && text.idstring.as_deref() == Some("FE_REVERSE")) {
            out.push(text_draw(
                text,
                text.string.as_deref().unwrap_or(""),
                layout,
            ));
        }
    }
    records(screen, layout, frame, &extra.row_labels, &mut out);
    out
}

fn records(
    screen: &oag_ui::screen::Screen,
    layout: &Layout,
    frame: &Frame,
    row_labels: &[String; 3],
    out: &mut Vec<Draw>,
) {
    for block in &screen.blocks {
        if let Some(art) = frame.blocks {
            let whole = oag_ui::menu::block::Block {
                x: block.x,
                y: block.y,
                width: block.width,
                height: block.height,
                color: argb_to_rgba(block.color),
                landing: block.landing,
            };
            oag_ui::menu::block::draw(&whole, &art, (1.0, 1.0), out);
        }
        let name = block.name.as_deref().unwrap_or("");
        let label = match name
            .strip_prefix("Record.")
            .and_then(|rest| rest.split_once('.'))
            .and_then(|(r, c)| Some((r.parse::<usize>().ok()?, c.parse::<usize>().ok()?)))
        {
            Some((row @ 1..=3, column @ 1..=4)) => Some(record_cell(row, column, row_labels)),
            _ => screen
                .texts
                .iter()
                .find(|text| {
                    text.idstring.is_some()
                        && (text.x - block.x).abs() < 0.5
                        && (text.y - block.y).abs() < 0.5
                })
                .and_then(|text| text.string.clone()),
        };
        if let Some(label) = label {
            out.push(Draw::Text {
                x: block.x + 40.0,
                y: block.y + 3.0,
                scale: block.text_scale * layout.faces.default,
                color: [1.0, 1.0, 1.0, 1.0],
                border: None,
                align: Align::Left,
                text: label,
                wrap_width: None,
            });
        }
    }
}

/// Pointer targets, **chosen, not measured**: the `CHOOSE CIRCUIT` frame's
/// left and right halves step the circuit back and forward as left/right do
/// on the pad there, the hex frame's top and bottom halves switch the
/// direction row as up/down do (only where there are two rows), and the
/// circuit model's frame confirms.
pub(in crate::picker) fn targets(
    picker: &Picker,
    layout: &Layout,
    extra: &TrackScreen,
) -> Vec<crate::picker::pointer::Target> {
    use crate::picker::pointer::{Target, What};
    let mut out = Vec::new();
    let mut brackets = extra.common.brackets.iter().filter(|b| !b.middle);
    if let Some(choose) = brackets.next() {
        let [x, y, w, h] = choose.rect;
        out.push(Target {
            what: What::Previous,
            rect: [x, y, w / 2.0, h],
        });
        out.push(Target {
            what: What::Next,
            rect: [x + w / 2.0, y, w / 2.0, h],
        });
    }
    if let Some(grid) = &extra.hex_grid {
        let (width, rows) = shape(picker);
        let mut grid = *grid;
        grid.rows = grid.rows.min(rows);
        for cell in grid.cells(picker.index() % width.max(1), width, picker.pan()) {
            out.push(Target {
                what: What::Cell {
                    entry: cell.row * width + cell.entry,
                    variant: 0,
                },
                rect: cell.rect,
            });
        }
    } else if picker.has_rows()
        && let Some(hexes) = brackets.next()
    {
        let [x, y, w, h] = hexes.rect;
        out.push(Target {
            what: What::PreviousVariant,
            rect: [x, y, w, h / 2.0],
        });
        out.push(Target {
            what: What::NextVariant,
            rect: [x, y + h / 2.0, w, h / 2.0],
        });
    }
    out.push(Target {
        what: What::Confirm,
        rect: layout.preview,
    });
    out
}

#[cfg(test)]
mod tests;
