//! A title's pause screen laid out as its disc authors it: the screen's own `Block` and `List`
//! widgets at their authored positions, the title and its arrow, and the shared chrome screen's
//! two rules and footer (`InGame Main Menu` on HD, `oag_title::PauseLayout`).
//!
//! The rows' behaviour is the menu's (`oag_ui::menu::Menu`, built by
//! `Definition::open_on_pause_screen`); this draws them. A widget finds its row by the disc
//! `idstring` through the title's table ([`oag_title::PauseMenu`]) and the row id the page gives
//! that action ([`oag_ui::menu::pause_row_id`]), so a widget whose row this build does not have
//! draws nothing.
//!
//! The box, its focus growth and its blinking marker are `Block_Render`/`Block_Update`'s
//! (`oag_ui::menu::block`); a list row is `List_Item`'s two boxes and step arrows with the same
//! numbers the front end's settings rows use ([`oag_title::ListBlocks`]).

use oag_title::{PauseAction, PauseMenu};
use oag_ui::frontend::{Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::menu::pointer::{Part, Region};
use oag_ui::menu::{Entry, Frame, Layers, Menu, Skin, block, pause_row_id};
use oag_ui::screen::{BlockWidget, ListWidget, Screens, Text, argb_to_rgba};

use crate::campaign::Layout;
use crate::campaign::draw::{fill_draw, image_draw, text_draw};
use crate::picker::FaceScales;

/// Where a block's label sits inside it, `(40, 3)`: `0x00189c38` puts the label child at
/// `X + 40` and, on an output of 720 lines or more, `Y + 3`
/// (`docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md`).
const LABEL_INSET: (f32, f32) = (40.0, 3.0);
/// Where a selectable block's marker arrow sits, `(8, 4)`, and its size, `32`: `0x0018c0e8`.
const ARROW_OFFSET: (f32, f32) = (8.0, 4.0);
const ARROW_SIZE: f32 = 32.0;
/// How far a list's value text sits in from its box's left edge (`crate::menu::rows`'s own
/// capture-measured inset for the same widget).
const VALUE_TEXT_INSET: f32 = 13.0;
/// The ink of a row that is drawn and cannot be chosen, on its label and its border: white at a
/// third. **Measured**, off the original's VIEW INVITES row over `HD_Grey` (100): border 153,
/// glyphs 151 to 165, so `0.33` to `0.4`.
const INERT: [f32; 4] = [1.0, 1.0, 1.0, 0.35];

/// What the fill of a row that cannot be chosen keeps of its alpha. **Chosen, not measured**:
/// the original's VIEW INVITES fill reads 73 where the other rows read 105 on the Fury palette
/// (an effective alpha of 0.44 against 0.68); this lands the Fury frame. On the HD palette it
/// reads lighter than its neighbours (111 against 100), which this build has no palette to show.
const INERT_FILL_ALPHA: f32 = 0.65;

/// One pause screen and the chrome behind it, read off the disc's definition file.
#[derive(Debug, Clone)]
pub struct PauseScreen {
    screen: Layout,
    chrome: Layout,
}

impl PauseScreen {
    /// The screen called `name` and the shared `chrome` screen out of `screens`, every
    /// `idstring` resolved through `strings`. `None` when either is not in the file.
    #[must_use]
    pub fn read(
        screens: &Screens,
        name: &str,
        chrome: &str,
        strings: &StringTable,
        faces: FaceScales,
        grid: [f32; 2],
    ) -> Option<Self> {
        let read =
            |screen: &str| Layout::read_authored(screens, screen, strings, faces, grid, grid);
        Some(Self {
            screen: read(name)?,
            chrome: read(chrome)?,
        })
    }

    /// The screen's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.screen.screen.name
    }

    /// The texture paths the screen and its chrome draw, for the caller to place on its sheet.
    pub fn sprite_srcs(&self) -> impl Iterator<Item = &str> {
        [&self.chrome, &self.screen]
            .into_iter()
            .flat_map(|layout| layout.screen.images.iter().map(|image| image.src.as_str()))
    }

    fn blocks(&self) -> &[BlockWidget] {
        &self.screen.screen.blocks
    }

    fn lists(&self) -> &[ListWidget] {
        &self.screen.screen.lists
    }

    fn label_of(&self, name: Option<&str>) -> Option<&Text> {
        let name = name?;
        self.screen
            .screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(name))
    }
}

/// One of the screen's rows with its menu row index.
struct Row<'a> {
    row: usize,
    kind: Kind<'a>,
}

enum Kind<'a> {
    Block(&'a BlockWidget, &'a Text),
    List(&'a ListWidget, &'a Text),
}

/// Every widget of `screen` that has a row on the page, in the screen's own order, with that
/// row's index. The disc's `idstring` is the key: widget, then the title's action for it, then
/// the page's row id for the action.
fn rows<'a>(screen: &'a PauseScreen, table: &PauseMenu, menu: &Menu) -> Vec<Row<'a>> {
    let Some(disc) = table.screen(screen.name()) else {
        return Vec::new();
    };
    let page = menu.page();
    let row_for = |text: &Text| -> Option<usize> {
        let id = text.idstring.as_deref()?;
        let action: PauseAction = disc.rows.iter().find(|row| row.disc_id == id)?.builds?;
        let wanted = pause_row_id(action);
        page.meta
            .iter()
            .position(|meta| meta.id.as_deref() == Some(wanted))
    };
    let mut out = Vec::new();
    for block in screen.blocks() {
        if let Some(text) = screen.label_of(block.name.as_deref())
            && let Some(row) = row_for(text)
        {
            out.push((
                block.y,
                Row {
                    row,
                    kind: Kind::Block(block, text),
                },
            ));
        }
    }
    for list in screen.lists() {
        if let Some(text) = screen.label_of(list.name.as_deref())
            && let Some(row) = row_for(text)
        {
            out.push((
                list.y,
                Row {
                    row,
                    kind: Kind::List(list, text),
                },
            ));
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out.into_iter().map(|(_, placed)| placed).collect()
}

/// Whether the marker arrow is lit on a page `ticks` after it arrived: eight ticks on, nine off.
fn arrow_lit(menu: &Menu) -> bool {
    menu.ticks_since_arrival()
        .is_none_or(|ticks| ticks % 17 < 8)
}

/// The pause screen's draw list: the chrome and title first, then the rows.
///
/// `sprites` places a texture by the path the widget names it by (the title arrow). `frame`
/// carries the block art; with none, the boxes draw nothing and the labels still do.
#[must_use]
pub fn draw_list(
    screen: &PauseScreen,
    table: &PauseMenu,
    menu: &Menu,
    skin: &Skin,
    frame: &Frame,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut layers = Layers::default();
    let mut chrome = Vec::new();
    for layout in [&screen.chrome, &screen.screen] {
        for fill in &layout.screen.fills {
            chrome.push(fill_draw(fill));
        }
        for image in &layout.screen.images {
            if let Some(placed) = sprites(&image.src) {
                chrome.push(image_draw(image, placed));
            }
        }
    }
    let (_, _, title_scale) = skin.title_at();
    for layout in [&screen.chrome, &screen.screen] {
        for text in &layout.screen.texts {
            if layout
                .screen
                .blocks
                .iter()
                .any(|b| b.name == text.name && b.name.is_some())
                || layout
                    .screen
                    .lists
                    .iter()
                    .any(|l| l.name == text.name && l.name.is_some())
            {
                continue;
            }
            let Some(content) = text.string.as_deref() else {
                continue;
            };
            if text.font.eq_ignore_ascii_case("title") {
                chrome.push(Draw::title(
                    skin.title_font(),
                    text.x,
                    text.y,
                    title_scale * text.scale,
                    argb_to_rgba(text.color),
                    content.to_string(),
                ));
            } else {
                chrome.push(text_draw(text, content, layout));
            }
        }
    }
    layers.chrome = chrome;

    let mut body = Vec::new();
    let lit = arrow_lit(menu);
    let numbers = skin.blocks();
    for placed in rows(screen, table, menu) {
        let entry = &menu.page().entries[placed.row];
        let focus = menu.focus_of(placed.row);
        let selected = placed.row == menu.selected();
        match placed.kind {
            Kind::Block(widget, text) => block_draw(
                &screen.screen,
                widget,
                text,
                entry.label(),
                (selected, focus, lit, menu.is_disabled(entry)),
                frame,
                &mut body,
            ),
            Kind::List(widget, text) => {
                if let Some(numbers) = numbers {
                    list_draw(
                        &screen.screen,
                        widget,
                        text,
                        entry,
                        menu,
                        (selected, focus, lit),
                        &numbers,
                        frame,
                        &mut body,
                    );
                }
            }
        }
    }
    layers.body = body;
    layers
}

/// One `<Block>` as `Block_Render` draws it: the box (the authored width plus the focus growth
/// on a selectable one), the marker arrow while it blinks lit on the focused one, then the label.
fn block_draw(
    layout: &Layout,
    widget: &BlockWidget,
    text: &Text,
    label: &str,
    (selected, focus, lit, inert): (bool, f32, bool, bool),
    frame: &Frame,
    out: &mut Vec<Draw>,
) {
    let grows = if widget.selectable && !inert {
        focus
    } else {
        0.0
    };
    let width = widget.width + grows * block::FOCUS_GROWTH;
    if let Some(art) = frame.blocks {
        let color = if selected && widget.selectable {
            widget.active_color
        } else {
            widget.color
        };
        let shape = block::Block {
            x: widget.x,
            y: widget.y,
            width,
            height: widget.height,
            color: argb_to_rgba(color),
            landing: widget.landing,
        };
        if inert {
            let mut faded = shape;
            faded.color[3] *= INERT_FILL_ALPHA;
            block::draw_fill(&faded, &art, (1.0, 1.0), out);
            block::draw_border(
                &block::Block {
                    color: INERT,
                    ..shape
                },
                &art,
                (1.0, 1.0),
                out,
            );
        } else {
            block::draw(&shape, &art, (1.0, 1.0), out);
        }
        if selected
            && widget.selectable
            && lit
            && let (Some(arrow), Some(tint)) = (art.arrow, widget.arrow_color)
        {
            out.push(Draw::Sprite {
                rect: [
                    widget.x + ARROW_OFFSET.0,
                    widget.y + ARROW_OFFSET.1,
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
    let mut positioned = text.clone();
    positioned.x = widget.x + LABEL_INSET.0;
    positioned.y = widget.y + LABEL_INSET.1;
    positioned.scale = widget.text_scale;
    positioned.color = widget.text_color.unwrap_or(text.color);
    let draw = text_draw(&positioned, label, layout);
    out.push(if inert { dimmed(draw) } else { draw });
}

fn dimmed(mut draw: Draw) -> Draw {
    if let Draw::Text { color, .. } | Draw::FacedText { color, .. } = &mut draw {
        *color = INERT;
    }
    draw
}

/// One `<List>` as `List_Item` draws it: a label box, a value box that grows from 280 to 340
/// wide as the row takes focus, and the step arrows at the label box's right end.
#[allow(clippy::too_many_arguments)]
fn list_draw(
    layout: &Layout,
    widget: &ListWidget,
    text: &Text,
    entry: &Entry,
    menu: &Menu,
    (selected, focus, _lit): (bool, f32, bool),
    numbers: &oag_title::MenuBlocks,
    frame: &Frame,
    out: &mut Vec<Draw>,
) {
    let list = numbers.list;
    let [label, value] = list_rects(widget, &list, focus);
    let value_text = menu.shown(entry).map(|shown| shown.to_string());
    let fill = |active: bool| {
        let own = if active {
            widget.active_color
        } else {
            widget.inactive_color
        };
        own.map(argb_to_rgba).or(if active {
            frame.tab_selected
        } else {
            frame.ink
        })
    };
    if let Some(art) = frame.blocks {
        if let Some(color) = fill(selected) {
            for (rect, landing) in [(label, false), (value, true)] {
                block::draw(
                    &block::Block {
                        x: rect[0],
                        y: rect[1],
                        width: rect[2],
                        height: rect[3],
                        color,
                        landing,
                    },
                    &art,
                    (1.0, 1.0),
                    out,
                );
            }
        }
        if let Some(arrow) = art.arrow {
            let tint = widget.arrow_color.map_or([1.0; 4], argb_to_rgba);
            let uv = [
                arrow.x as f32,
                arrow.y as f32,
                arrow.width as f32,
                arrow.height as f32,
            ];
            let [left, right] = oag_ui::menu::block::list_arrow_rects(&list, label, (1.0, 1.0));
            out.push(Draw::Sprite {
                rect: left,
                uv: [uv[0] + uv[2], uv[1], -uv[2], uv[3]],
                color: tint,
            });
            out.push(Draw::Sprite {
                rect: right,
                uv,
                color: tint,
            });
        }
    }
    let mut label_text = text.clone();
    // The disc authors a list's label in `font="small"`; the original's frame draws it at the
    // block labels' size and face (measured, RPCS3), so it takes theirs.
    label_text.font = "default".to_string();
    label_text.x = label[0] + LABEL_INSET.0;
    label_text.y = label[1] + LABEL_INSET.1;
    label_text.scale = 0.8;
    out.push(text_draw(&label_text, entry.label(), layout));
    if let Some(content) = value_text {
        let mut value_text = label_text;
        value_text.x = value[0] + VALUE_TEXT_INSET;
        out.push(text_draw(&value_text, &content, layout));
    }
}

/// A list row's label box and value box, `[x, y, w, h]` each, the value box at `focus`'s growth.
fn list_rects(widget: &ListWidget, list: &oag_title::ListBlocks, focus: f32) -> [[f32; 4]; 2] {
    let value_x = widget.x + list.label_width + list.gap;
    let value_width = list.value_width + (list.value_focus_width - list.value_width) * focus;
    [
        [widget.x, widget.y, list.label_width, list.height],
        [value_x, widget.y, value_width, list.height],
    ]
}

/// The rects a pointer can land on, for the rows the screen draws: a block's box at its resting
/// width, a list's two boxes and its two step arrows. The same positions [`draw_list`] paints.
#[must_use]
pub fn regions(screen: &PauseScreen, table: &PauseMenu, menu: &Menu, skin: &Skin) -> Vec<Region> {
    let numbers = skin.blocks();
    let mut out = Vec::new();
    for placed in rows(screen, table, menu) {
        let focus = menu.focus_of(placed.row);
        let entry = &menu.page().entries[placed.row];
        if matches!(entry, Entry::Inert { .. }) {
            continue;
        }
        match placed.kind {
            Kind::Block(widget, _) => out.push(Region {
                row: placed.row,
                part: Part::Row,
                rect: [
                    widget.x,
                    widget.y,
                    widget.width + focus * block::FOCUS_GROWTH,
                    widget.height,
                ],
            }),
            Kind::List(widget, _) => {
                let Some(numbers) = numbers else {
                    continue;
                };
                let [label, value] = list_rects(widget, &numbers.list, focus);
                out.push(Region {
                    row: placed.row,
                    part: Part::Row,
                    rect: label,
                });
                out.push(Region {
                    row: placed.row,
                    part: Part::Row,
                    rect: value,
                });
                let [left, right] =
                    oag_ui::menu::block::list_arrow_rects(&numbers.list, label, (1.0, 1.0));
                out.push(Region {
                    row: placed.row,
                    part: Part::StepBack,
                    rect: left,
                });
                out.push(Region {
                    row: placed.row,
                    part: Part::StepForward,
                    rect: right,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
