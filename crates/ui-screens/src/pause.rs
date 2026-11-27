//! A title's pause screen laid out as its disc authors it: the screen's own `Block` and `List`
//! widgets at their authored positions, the title and its arrow, and the shared chrome screen's
//! two rules and footer (`InGame Main Menu` on HD, `oag_title::PauseLayout`). A screen that
//! authors one `<Menu>` instead (Pure's band of centred rows) draws every row of the page down
//! that widget, and a screen of `<TouchButton>` tiles (Wipeout 2048's) draws one box and icon per
//! tile, the pad's cursor filling the tile it is on.
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

use oag_title::{PauseAction, PauseMenu, PauseTouch};
use oag_ui::frontend::{Draw, Placed, label_under_tile};
use oag_ui::language::StringTable;
use oag_ui::menu::nav::Dir;
use oag_ui::menu::pointer::{Part, Region};
use oag_ui::menu::{Entry, Frame, Layers, Menu, Skin, block, pause_row_id};
use oag_ui::screen::{
    BlockWidget, ListWidget, Screens, Text, TouchButton, TouchList, argb_to_rgba, parse_argb,
};
use oag_ui::state_machine::SEPARATOR;

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
    /// The screens drawn behind this one, outermost first: the shared chrome screen, or for a
    /// screen of tiles the screens it is nested in.
    chrome: Vec<Layout>,
    /// The tile and cursor fills of a screen of `<TouchButton>` tiles.
    tiles: Option<TileInk>,
}

/// What a tile is filled with, as `ARGB`: at rest, and with the pad's cursor on it.
#[derive(Debug, Clone, Copy)]
struct TileInk {
    rest: u32,
    cursor: u32,
    /// A list entry that cannot be chosen.
    inactive: u32,
}

impl PauseScreen {
    /// The screen called `name` and the shared `chrome` screen, when the file has one, out of
    /// `screens`, every `idstring` resolved through `strings`. `None` when either is not in
    /// the file.
    #[must_use]
    pub fn read(
        screens: &Screens,
        name: &str,
        chrome: Option<&str>,
        strings: &StringTable,
        faces: FaceScales,
        grid: [f32; 2],
    ) -> Option<Self> {
        let read =
            |screen: &str| Layout::read_authored(screens, screen, strings, faces, grid, grid);
        Some(Self {
            screen: read(name)?,
            chrome: match chrome {
                Some(chrome) => vec![read(chrome)?],
                None => Vec::new(),
            },
            tiles: None,
        })
    }

    /// A screen of `<TouchButton>` tiles: the screen called `name` and, as its chrome, the screens
    /// it is nested in (2048's `PauseMenuSpeed` stands in `PauseMenuSpeedTop`, which authors the
    /// panel and the heading; a confirm screen stands in the page it asks on). Every tile's
    /// `idstring` is resolved through `strings` to the words under it. `None` when the file lacks
    /// the screen or a colour global.
    #[must_use]
    pub fn read_touch(
        screens: &Screens,
        name: &str,
        touch: PauseTouch,
        strings: &StringTable,
        faces: FaceScales,
        grid: [f32; 2],
    ) -> Option<Self> {
        let colour = |global: &str| {
            screens
                .resolve(&format!("FEGlobals->{global}"))
                .and_then(parse_argb)
        };
        let read =
            |screen: &str| Layout::read_authored(screens, screen, strings, faces, grid, grid);
        let path = &screens.by_name(name)?.path;
        let mut names: Vec<&str> = path.split(SEPARATOR).collect();
        names.pop();
        let mut chrome: Vec<Layout> = names.into_iter().filter_map(read).collect();
        let mut screen = read(name)?;
        for layout in chrome.iter_mut().chain(std::iter::once(&mut screen)) {
            for button in &mut layout.screen.touch_buttons {
                if let Some(words) = button.idstring.as_deref().and_then(|id| strings.get(id)) {
                    button.string = Some(words.to_string());
                }
            }
            // A list entry's words go in its `value`, the literal the entry carries beside its
            // label, as a tile's go in `string`.
            for entry in layout
                .screen
                .touch_lists
                .iter_mut()
                .flat_map(|list| &mut list.entries)
            {
                if let Some(words) = entry.idstring.as_deref().and_then(|id| strings.get(id)) {
                    entry.value = Some(words.to_string());
                }
            }
        }
        Some(Self {
            screen,
            chrome,
            tiles: Some(TileInk {
                rest: colour(touch.tile)?,
                cursor: colour(touch.cursor)?,
                inactive: colour(touch.inactive)?,
            }),
        })
    }

    /// Whether the screen is a page of `<TouchButton>` tiles.
    #[must_use]
    pub fn is_tiles(&self) -> bool {
        self.tiles.is_some()
    }

    /// The screen's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.screen.screen.name
    }

    /// The texture paths the screen and its chrome draw, for the caller to place on its sheet.
    pub fn sprite_srcs(&self) -> impl Iterator<Item = &str> {
        self.layouts().flat_map(|layout| {
            let icons = layout
                .screen
                .touch_buttons
                .iter()
                .filter_map(|button| button.src.as_deref())
                .chain(
                    layout
                        .screen
                        .touch_lists
                        .iter()
                        .flat_map(|list| &list.entries)
                        .filter_map(|entry| entry.src.as_deref()),
                );
            layout
                .screen
                .images
                .iter()
                .map(|image| image.src.as_str())
                .chain(icons)
        })
    }

    /// The chrome screen, when there is one, then the screen itself: the order they draw in.
    fn layouts(&self) -> impl Iterator<Item = &Layout> {
        self.chrome.iter().chain(std::iter::once(&self.screen))
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

/// The texts of `layout` that are not a widget's label, as the chrome draws them.
fn text_draws(layout: &Layout, skin: &Skin, out: &mut Vec<Draw>) {
    let (_, _, title_scale) = skin.title_at();
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
            out.push(Draw::title(
                skin.title_font(),
                text.x,
                text.y,
                title_scale * text.scale,
                argb_to_rgba(text.color),
                content.to_string(),
            ));
        } else {
            out.push(text_draw(text, content, layout));
        }
    }
}

/// The key a `<TouchButton>` is found by in the title's table: its `idstring`, else its `name`,
/// else (a confirm screen's tick and cross, which author neither) its `redirect`.
fn tile_key(button: &TouchButton) -> Option<&str> {
    button
        .idstring
        .as_deref()
        .or(button.name.as_deref())
        .or(button.redirect.as_deref())
}

/// A screen of tiles as a draw list: each screen it is nested in with the tiles of that screen
/// at rest (the page a confirm screen asks on stays drawn under its dimming), then its own panel,
/// words and icon, then its tiles with the pad's cursor on the focused one.
fn touch_draw_list(
    screen: &PauseScreen,
    table: &PauseMenu,
    menu: &Menu,
    skin: &Skin,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    pad_cursor: bool,
) -> Layers {
    let mut chrome = Vec::new();
    for layout in screen.layouts() {
        for fill in &layout.screen.fills {
            chrome.push(fill_draw(fill));
        }
        for image in &layout.screen.images {
            if let Some(placed) = sprites(&image.src) {
                chrome.push(image_draw(image, placed));
            }
        }
        text_draws(layout, skin, &mut chrome);
        if std::ptr::eq(layout, &screen.screen) {
            continue;
        }
        let Some(behind) = table.screen(&layout.screen.name) else {
            continue;
        };
        for button in &layout.screen.touch_buttons {
            if tile_key(button)
                .and_then(|key| behind.rows.iter().find(|row| row.disc_id == key))
                .is_some_and(|row| row.builds.is_some())
            {
                tile_draw(screen, button, false, sprites, &mut chrome);
            }
        }
    }
    let mut body = Vec::new();
    for placed in rows(screen, table, menu) {
        let entry = &menu.page().entries[placed.row];
        match placed.kind {
            // A tile for the page being drawn is filled in the cursor's colour, as the disc
            // authors it (`Colour="FEGlobals->Orange2048"`).
            Kind::Tile(button) => tile_draw(
                screen,
                button,
                matches!(entry, Entry::Inert { .. })
                    || (pad_cursor && placed.row == menu.selected()),
                sprites,
                &mut body,
            ),
            Kind::Strip(list) => strip_draw(screen, list, entry, sprites, &mut body),
            _ => {}
        }
    }
    Layers {
        chrome,
        body,
        ..Layers::default()
    }
}

/// One of the screen's rows with its menu row index.
struct Row<'a> {
    row: usize,
    kind: Kind<'a>,
}

enum Kind<'a> {
    Block(&'a BlockWidget, &'a Text),
    List(&'a ListWidget),
    /// Row `n` of the screen's `<Menu>`, counting from its first.
    Menu(usize),
    Tile(&'a TouchButton),
    /// A `<Touchlist>`: one row whose values are the entries.
    Strip(&'a TouchList),
}

/// Every widget of `screen` that has a row on the page, in the screen's own order, with that
/// row's index. The disc's `idstring` is the key: widget, then the title's action for it, then
/// the page's row id for the action.
fn rows<'a>(screen: &'a PauseScreen, table: &PauseMenu, menu: &Menu) -> Vec<Row<'a>> {
    let Some(disc) = table.screen(screen.name()) else {
        return Vec::new();
    };
    let page = menu.page();
    if screen.screen.screen.menu.is_some() && table.layout.is_some_and(|l| l.menu_rows.is_some()) {
        return (0..page.entries.len())
            .map(|row| Row {
                row,
                kind: Kind::Menu(row),
            })
            .collect();
    }
    let row_of = |id: &str| -> Option<usize> {
        let action: PauseAction = disc.rows.iter().find(|row| row.disc_id == id)?.builds?;
        let wanted = pause_row_id(action);
        page.meta
            .iter()
            .position(|meta| meta.id.as_deref() == Some(wanted))
    };
    if screen.tiles.is_some() {
        let tiles = screen
            .screen
            .screen
            .touch_buttons
            .iter()
            .filter_map(|button| {
                let key = tile_key(button)?;
                Some(Row {
                    row: row_of(key)?,
                    kind: Kind::Tile(button),
                })
            });
        let strips = screen.screen.screen.touch_lists.iter().filter_map(|list| {
            Some(Row {
                row: row_of(list.name.as_deref()?)?,
                kind: Kind::Strip(list),
            })
        });
        return tiles.chain(strips).collect();
    }
    let row_for = |text: &Text| -> Option<usize> { row_of(text.idstring.as_deref()?) };
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
        if let Some(row) = row_for(&list.label) {
            out.push((
                list.y,
                Row {
                    row,
                    kind: Kind::List(list),
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
    draw_list_for(screen, table, menu, skin, frame, sprites, true)
}

/// [`draw_list`] with the pad's cursor shown or not: a screen of tiles fills the focused tile
/// only while the pad is the device in use, as the front end's touch grids do, so a pointer
/// that is moving over the tiles leaves them all at rest.
#[must_use]
pub fn draw_list_for(
    screen: &PauseScreen,
    table: &PauseMenu,
    menu: &Menu,
    skin: &Skin,
    frame: &Frame,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    pad_cursor: bool,
) -> Layers {
    if screen.tiles.is_some() {
        return touch_draw_list(screen, table, menu, skin, sprites, pad_cursor);
    }
    let mut layers = Layers::default();
    let mut chrome = Vec::new();
    for layout in screen.layouts() {
        for fill in &layout.screen.fills {
            chrome.push(fill_draw(fill));
        }
        for image in &layout.screen.images {
            if let Some(placed) = sprites(&image.src) {
                chrome.push(image_draw(image, placed));
            }
        }
    }
    for layout in screen.layouts() {
        text_draws(layout, skin, &mut chrome);
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
            Kind::Menu(n) => {
                if let Some(draw) = menu_row_draw(screen, table, skin, n, entry, focus) {
                    body.push(draw);
                }
            }
            Kind::Tile(_) | Kind::Strip(_) => {}
            Kind::List(widget) => {
                if let Some(numbers) = numbers {
                    list_draw(
                        &screen.screen,
                        widget,
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

/// One `<TouchButton>` as the touch front end draws a tile: its box filled with the tile colour
/// (the cursor's while the pad is on it), the icon at the texture's own size centred in it, and
/// for a tile that authors an `idstring` its words under the box in the tile colour.
fn tile_draw(
    screen: &PauseScreen,
    button: &TouchButton,
    selected: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    out: &mut Vec<Draw>,
) {
    let Some(ink) = screen.tiles else {
        return;
    };
    let rect = [button.x, button.y, button.width, button.height];
    out.push(Draw::Fill {
        rect,
        color: argb_to_rgba(if selected { ink.cursor } else { ink.rest }),
    });
    if let Some(placed) = button.src.as_deref().and_then(sprites) {
        let (w, h) = (placed.width as f32, placed.height as f32);
        out.push(Draw::Sprite {
            rect: [
                rect[0] + (rect[2] - w) * 0.5,
                rect[1] + (rect[3] - h) * 0.5,
                w,
                h,
            ],
            uv: placed.sheet_rect([0.0, 0.0, w, h]),
            color: argb_to_rgba(button.color),
        });
    }
    if button.idstring.is_some()
        && let Some(words) = &button.string
    {
        out.push(label_under_tile(
            rect,
            words.clone(),
            argb_to_rgba(ink.rest),
            button.string_width_limit,
            button.small_text,
            None,
        ));
    }
}

/// A `<Touchlist>` as a strip of tiles: its entries side by side across its authored rect, each
/// with its icon and its words, the chosen one filled in the cursor's colour and an entry the row
/// has no value for (a friend's ghost, the world's: they need the online service) in the
/// inactive colour. **Chosen**: the widget authors one rect for the whole strip and no entry's
/// own, nor a disabled look; the same spacing the front end's touch lists use.
fn strip_draw(
    screen: &PauseScreen,
    list: &TouchList,
    entry: &Entry,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    out: &mut Vec<Draw>,
) {
    let Some(ink) = screen.tiles else {
        return;
    };
    let count = list.entries.len().max(1) as f32;
    let chosen = match entry {
        Entry::Choice {
            current, values, ..
        } if *current < values.len() => Some(*current),
        _ => None,
    };
    let selectable = match entry {
        Entry::Choice { values, .. } => values.len(),
        _ => 0,
    };
    for (i, item) in list.entries.iter().enumerate() {
        let cell = list.width / count;
        let rect = [list.x + cell * i as f32, list.y, cell, list.height];
        let ink = if Some(i) == chosen {
            ink.cursor
        } else if i < selectable {
            ink.rest
        } else {
            ink.inactive
        };
        out.push(Draw::Fill {
            rect,
            color: argb_to_rgba(ink),
        });
        if let Some(placed) = item.src.as_deref().and_then(sprites) {
            let (w, h) = (placed.width as f32, placed.height as f32);
            out.push(Draw::Sprite {
                rect: [
                    rect[0] + (rect[2] - w) * 0.5,
                    rect[1] + (rect[3] - h) * 0.5,
                    w,
                    h,
                ],
                uv: placed.sheet_rect([0.0, 0.0, w, h]),
                color: [1.0; 4],
            });
        }
        if let Some(words) = &item.value {
            out.push(label_under_tile(
                rect,
                words.clone(),
                argb_to_rgba(screen.tiles.map_or(0, |ink| ink.rest)),
                Some(cell),
                true,
                None,
            ));
        }
    }
}

/// The line `n` of a `<Menu>` screen's rows: centred on the widget's `x`, the first row's middle
/// at its `y`, each `pitch * scale` below the one before, white at the widget's colour for the
/// row with the cursor and dimmed on the others as the cursor eases on and off them.
fn menu_row_draw(
    screen: &PauseScreen,
    table: &PauseMenu,
    skin: &Skin,
    n: usize,
    entry: &Entry,
    focus: f32,
) -> Option<Draw> {
    let widget = screen.screen.screen.menu.as_ref()?;
    let rows = table.layout?.menu_rows?;
    let (_, y, _) = menu_row_box(widget, &rows, n);
    let top = if widget.middle {
        y - 0.5 * skin.line_height() * widget.scale
    } else {
        y
    };
    let alpha = rows.unfocused_alpha + (1.0 - rows.unfocused_alpha) * focus;
    let mut color = argb_to_rgba(widget.color);
    color[3] *= alpha;
    let text = Text {
        font: widget.font.clone(),
        x: widget.x,
        y: top,
        scale: widget.scale,
        color: widget.color,
        align: widget.align.clone(),
        ..Text::default()
    };
    let mut draw = text_draw(&text, entry.label(), &screen.screen);
    if let Draw::Text { color: ink, .. } | Draw::FacedText { color: ink, .. } = &mut draw {
        *ink = color;
    }
    Some(draw)
}

/// Where row `n` of a `<Menu>` stands: the widget's `x`, the row's middle `y` and its pitch.
fn menu_row_box(
    widget: &oag_ui::screen::Menu,
    rows: &oag_title::PauseMenuRows,
    n: usize,
) -> (f32, f32, f32) {
    let pitch = rows.pitch * widget.scale;
    (widget.x, widget.y + n as f32 * pitch, pitch)
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
                uv: arrow.sheet_rect([0.0, 0.0, arrow.width as f32, arrow.height as f32]),
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
            let uv = arrow.sheet_rect([0.0, 0.0, arrow.width as f32, arrow.height as f32]);
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
    let mut label_text = widget.label.clone();
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

/// The row a pad's `direction` moves the cursor to on a screen of tiles: the nearest tile whose
/// centre lies that way, favouring the one in line. The game asks it for LEFT and RIGHT. `None` at an edge (the cursor stays) and on
/// a screen that is not tiles. **Chosen**: the original's tiles answer touch only.
#[must_use]
pub fn tile_neighbour(
    screen: &PauseScreen,
    table: &PauseMenu,
    menu: &Menu,
    skin: &Skin,
    direction: Dir,
) -> Option<usize> {
    screen.tiles?;
    // LEFT and RIGHT on a list's row step its value, which the menu does.
    if matches!(
        menu.page().entries.get(menu.selected()),
        Some(Entry::Choice { .. })
    ) {
        return None;
    }
    let tiles = regions(screen, table, menu, skin);
    let centre = |rect: [f32; 4]| (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5);
    let (fx, fy) = centre(tiles.iter().find(|tile| tile.row == menu.selected())?.rect);
    tiles
        .iter()
        .filter(|tile| tile.row != menu.selected())
        .filter_map(|tile| {
            let (x, y) = centre(tile.rect);
            let (along, across) = match direction {
                Dir::Right => (x - fx, y - fy),
                Dir::Left => (fx - x, y - fy),
                Dir::Down => (y - fy, x - fx),
                Dir::Up => (fy - y, x - fx),
            };
            (along > 1.0).then_some((tile.row, along + 3.0 * across.abs()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(row, _)| row)
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
            Kind::Tile(button) => out.push(Region {
                row: placed.row,
                part: Part::Row,
                rect: [button.x, button.y, button.width, button.height],
            }),
            Kind::Strip(list) => {
                let count = list.entries.len().max(1) as f32;
                let selectable = match entry {
                    Entry::Choice { values, .. } => values.len(),
                    _ => 0,
                };
                for at in 0..selectable.min(list.entries.len()) {
                    let cell = list.width / count;
                    out.push(Region {
                        row: placed.row,
                        part: Part::Value(at),
                        rect: [list.x + cell * at as f32, list.y, cell, list.height],
                    });
                }
            }
            Kind::Menu(n) => {
                let (Some(widget), Some(rows)) = (
                    screen.screen.screen.menu.as_ref(),
                    table.layout.and_then(|layout| layout.menu_rows),
                ) else {
                    continue;
                };
                let (_, y, pitch) = menu_row_box(widget, &rows, n);
                let width = table.layout.map_or(0.0, |layout| layout.grid[0] as f32);
                out.push(Region {
                    row: placed.row,
                    part: Part::Row,
                    rect: [0.0, y - 0.5 * pitch, width, pitch],
                });
            }
            Kind::List(widget) => {
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
