//! The campaign editor's screen: `Cell Selection`'s own hex grid with every slot
//! drawn, the placed events filled and the empty ones dimmed, and the selected
//! event's fields listed down the details panel.
//!
//! The model is `oag_game::campaign::editor::Editor`; this crate cannot see it, so the
//! session hands over a [`View`] of plain strings and slots. Every hex, the selector
//! and the panel's text sit where the disc's own layout puts them: the rows reuse the
//! `Line1`..`Line3` widgets' positions and faces.

use oag_ui::frontend::{Draw, Placed};
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::pointer::{Pointer, contains, hex_contains};
use oag_ui::screen::argb_to_rgba;

use super::draw::{hex_slot_xy, image_draw, push_selector, sprite_draw, text_draw};
use super::pointer::Extent;
use super::pulse::SelectorPulse;
use super::{Layout, hex_rect};

/// One line of the panel: a field and what it is set to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The field's name.
    pub label: String,
    /// What it is set to.
    pub value: String,
}

/// What the screen shows.
#[derive(Debug, Clone)]
pub struct View<'a> {
    /// The screen's title, over the grid.
    pub title: &'a str,
    /// The campaign's name, the details panel's heading.
    pub name: &'a str,
    /// The line under it: what the cursor is on, or how many events are placed.
    pub heading: &'a str,
    /// The selected slot.
    pub cursor: (u32, u32),
    /// The slots holding an event.
    pub placed: &'a [(u32, u32)],
    /// The selected event's fields; empty when the cursor is on an empty slot.
    pub fields: &'a [Row],
    /// The rows below them that act on the whole campaign (its name, saving it).
    pub actions: &'a [Row],
    /// The field the panel's cursor is on, when the panel is open.
    pub selected_row: Option<usize>,
    /// A line under the rows: what the last save did, or how to save.
    pub status: &'a str,
    /// The footer's two prompts beside the disc's own `Confirm` and `Back`: what the
    /// triangle does, and what the square does.
    pub legend: [&'a str; 2],
    /// The selector's pulse clock.
    pub pulse: &'a SelectorPulse,
}

/// What a click or tap on the screen landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    /// A hex slot.
    Hex((u32, u32)),
    /// A row of the panel: a field, then an action, counted together.
    Row(usize),
}

/// What the pointer did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The pointer moved onto a hex.
    Hover((u32, u32)),
    /// A second click on the selected hex.
    Confirm,
    /// A click on a hex other than the selected one.
    Select((u32, u32)),
    /// The pointer moved onto a field.
    Field(usize),
    /// A click on a panel row (a field or an action, counted together).
    Row(usize),
    /// The secondary button.
    Back,
}

/// A target on the screen and its rect, in the screen's grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    /// What it is.
    pub what: What,
    /// `[x, y, width, height]`.
    pub rect: [f32; 4],
}

/// Where the panel's rows stand: the `Line1 Title`/`Line1` widgets' positions and
/// face, stepped down by the pitch between `Line1` and `Line2`.
struct Rows {
    label_x: f32,
    value_x: f32,
    y: f32,
    pitch: f32,
}

fn rows_of(layout: &Layout) -> Rows {
    let text = |name: &str| {
        layout
            .screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(name))
    };
    let first = text("Line1 Title");
    let second = text("Line2 Title");
    let value = text("Line1");
    let y = first.map_or(120.0, |t| t.y);
    Rows {
        label_x: first.map_or(260.0, |t| t.x),
        value_x: value.map_or(380.0, |t| t.x),
        y,
        pitch: second.map_or(18.0, |t| t.y - y).abs().max(10.0),
    }
}

/// How many rows the fields keep free, so the actions stand in the same place on an
/// empty slot as on a placed one.
pub const FIELD_ROWS: usize = 5;

/// Where logical row `index` (fields, then actions) stands, counted in rows.
fn place(view: &View, index: usize) -> usize {
    if index < view.fields.len() {
        index
    } else {
        FIELD_ROWS + index - view.fields.len()
    }
}

/// The rect of panel row `index`.
/// The PSP screen's width, the grid the disc's layout is authored in.
const PSP_WIDTH: f32 = 480.0;

fn row_rect(rows: &Rows, index: usize, layout: &Layout) -> [f32; 4] {
    let width = (PSP_WIDTH * layout.scale[0] - rows.label_x - 8.0).max(100.0);
    [
        rows.label_x - 2.0,
        rows.y + rows.pitch * index as f32 - 2.0,
        width,
        rows.pitch,
    ]
}

/// The draw list.
#[must_use]
#[allow(clippy::too_many_arguments, reason = "the same facts `cell_draw_list` takes")]
pub fn draw_list(
    view: &View,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
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
        view.title.to_string(),
    ));
    layers.chrome.extend_from_slice(footer_overlay);

    let screen = &layout.screen;
    let mut out = Vec::new();
    for fill in &screen.fills {
        let keep = fill.name.as_deref().is_none_or(|n| !n.starts_with("linebg"));
        if keep {
            out.push(fill.draw());
        }
    }
    let slots: Vec<(u32, u32)> = screen
        .images
        .iter()
        .filter_map(|image| hex_slot_xy(image.name.as_deref()?, "Outline_"))
        .collect();
    // The filled hex the layout keeps for its `Target` swatches, drawn under a placed event's
    // outline at that outline's own size.
    let fill_sprite = screen
        .images
        .iter()
        .find(|image| image.name.as_deref() == Some("Target0 Image"))
        .and_then(|image| sprites(&image.src))
        .and_then(|placed| {
            let outline = screen
                .images
                .iter()
                .find(|image| image.name.as_deref().is_some_and(|n| n.starts_with("Outline_")))?;
            let size = hex_rect(screen, 0, 0, sprites).or_else(|| {
                let (x, y) = hex_slot_xy(outline.name.as_deref()?, "Outline_")?;
                hex_rect(screen, x as usize, y as usize, sprites)
            })?;
            Some((placed, size[2], size[3]))
        });
    for image in &screen.images {
        let name = image.name.as_deref().unwrap_or("");
        if hex_slot_xy(name, "Medal_").is_some()
            || hex_slot_xy(name, "Lock_").is_some()
            || name.starts_with("Target")
        {
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        if let Some(at) = hex_slot_xy(name, "Outline_") {
            if view.placed.contains(&at) {
                if let Some(fill) = fill_sprite {
                    out.push(Draw::Sprite {
                        rect: [image.x, image.y, fill.1, fill.2],
                        uv: [fill.0.x as f32, fill.0.y as f32, fill.0.width as f32, fill.0.height as f32],
                        color: argb_to_rgba(PLACED_HEX),
                    });
                }
                out.push(image_draw(image, placed));
            } else {
                out.push(sprite_draw(image, placed, image.x, image.y, EMPTY_HEX));
            }
            continue;
        }
        if name == "Selector" {
            if let Some(hex) = slots
                .contains(&view.cursor)
                .then(|| hex_rect(screen, view.cursor.0 as usize, view.cursor.1 as usize, sprites))
                .flatten()
            {
                push_selector(&mut out, image, placed, hex, view.pulse, skin.selector_origin());
            }
            continue;
        }
        out.push(image_draw(image, placed));
    }

    let rows = rows_of(layout);
    let title_text = |name: &str| {
        screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(name))
    };
    if let Some(text) = title_text("Title") {
        out.push(text_draw(text, view.name, layout));
    }
    if let Some(text) = title_text("Track Line") {
        out.push(text_draw(text, view.heading, layout));
    }
    if let (Some(label), Some(value)) = (title_text("Line1 Title"), title_text("Line1")) {
        for (index, row) in view.fields.iter().chain(view.actions).enumerate() {
            let dy = rows.pitch * place(view, index) as f32;
            let mut label = label.clone();
            let mut value = value.clone();
            label.y += dy;
            value.y += dy;
            value.x = rows.value_x;
            if view.selected_row == Some(index) {
                label.color = SELECTED;
                value.color = SELECTED;
            }
            label.scale *= LABEL_SCALE;
            value.scale *= VALUE_SCALE;
            out.push(text_draw(&label, &row.label, layout));
            out.push(text_draw(&value, &clip(&row.value), layout));
        }
        if !view.status.is_empty() {
            let mut status = label.clone();
            status.y = rows.y + rows.pitch * (FIELD_ROWS + view.actions.len()) as f32;
            status.scale *= LABEL_SCALE;
            out.push(text_draw(&status, view.status, layout));
        }
    }
    for text in &screen.texts {
        let content = match (text.name.as_deref(), text.idstring.as_deref(), text.string.as_deref()) {
            (None, None, Some(glyph @ ("\u{3b2}" | "\u{3b4}"))) => glyph,
            (None, Some("FE_HELP"), _) => view.legend[0],
            (Some("DifficultyButton"), _, _) => view.legend[1],
            _ => continue,
        };
        out.push(text_draw(text, content, layout));
    }
    layers.body = out;
    layers
}

/// The fill under a placed event's outline.
const PLACED_HEX: u32 = 0x9033_a6b9;
/// A dimmed hex, an unplaced slot.
const EMPTY_HEX: u32 = 0x4033_a6b9;
/// The panel row the cursor is on.
const SELECTED: u32 = 0xffff_ffff;
/// The panel's labels and values are drawn smaller than `Line1`'s own face, so a
/// circuit's name and `SPEED CLASS` fit the panel.
const LABEL_SCALE: f32 = 0.85;
const VALUE_SCALE: f32 = 0.72;
/// The longest value drawn; a longer one ends in `..`.
const VALUE_CHARS: usize = 21;

fn clip(value: &str) -> String {
    if value.chars().count() <= VALUE_CHARS {
        return value.to_string();
    }
    let kept: String = value.chars().take(VALUE_CHARS - 2).collect();
    format!("{kept}..")
}

/// Every hex the layout draws, and the panel's rows and save line.
#[must_use]
pub fn targets(
    view: &View,
    layout: &Layout,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    extent: Extent,
) -> Vec<Target> {
    let screen = &layout.screen;
    let mut out = Vec::new();
    let rows = rows_of(layout);
    for index in 0..view.fields.len() + view.actions.len() {
        out.push(Target {
            what: What::Row(index),
            rect: row_rect(&rows, place(view, index), layout),
        });
    }
    for at in layout.hex_slots() {
        if let Some(rect) = super::pointer::hex_hit_rect(screen, at.0 as usize, at.1 as usize, sprites, extent) {
            out.push(Target {
                what: What::Hex(at),
                rect,
            });
        }
    }
    out
}

/// The first target under `at`.
#[must_use]
pub fn hit(targets: &[Target], at: (f32, f32)) -> Option<Target> {
    targets.iter().copied().find(|target| match target.what {
        What::Hex(_) => hex_contains(target.rect, at),
        What::Row(_) => contains(target.rect, at),
    })
}

/// What `pointer` did this tick against `targets`, with `view` the state before it:
/// hover moves the cursor or the panel's row, a click confirms a selected hex, selects
/// another, steps a row's value or saves.
#[must_use]
pub fn pointer(view: &View, pointer: &Pointer, targets: &[Target]) -> Vec<Event> {
    let mut out = Vec::new();
    if pointer.is_idle() {
        return out;
    }
    let under = pointer.at.and_then(|at| hit(targets, at));
    if pointer.moved
        && let Some(target) = under
    {
        match target.what {
            What::Hex(at) if at != view.cursor => out.push(Event::Hover(at)),
            What::Row(index)
                if index < view.fields.len() && view.selected_row != Some(index) =>
            {
                out.push(Event::Field(index));
            }
            _ => {}
        }
    }
    if pointer.clicked
        && let Some(target) = under
    {
        out.push(match target.what {
            What::Hex(at) if at == view.cursor => Event::Confirm,
            What::Hex(at) => Event::Select(at),
            What::Row(index) => Event::Row(index),
        });
    }
    if pointer.back {
        out.push(Event::Back);
    }
    out
}
