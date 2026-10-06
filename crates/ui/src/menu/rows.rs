//! A page drawn as a column of rows: the vertical `<Menu>` idiom.
//!
//! **A move out of [`super::draw_list`], not a new arrangement.** Every number,
//! colour and comment here is the one that was in that function; what changed is
//! that it now sits beside [`super::strip`], because a title whose main menu is a
//! `<HorizMenu>` draws the same page a different way and one function cannot be
//! both. [`super::draw_list`] picks between them and does nothing else.
//!
//! This is the idiom both PSP titles use everywhere, and the one Wipeout HD uses
//! for every page that has a value column - so it is still what almost every
//! page in `assets/ui/menu.toml` is drawn as. See [`super::strip::suits`].
//!
//! # Two idioms in one column
//!
//! A PSP title's row is bare text, selected by brightening - measured on
//! Pulse, and [`draw_text_rows`] draws it. Wipeout HD's settings row is a
//! `<List>`: a label box, a value box that grows when the row is focused, and
//! a pair of step arrows, every one of them the executable's rather than the
//! XML's - `List_Item.cpp`, read on 2026-09-14 and drawn by
//! [`draw_list_rows`]. Which one a title gets is decided by whether its skin
//! carries blocks ([`super::Skin::blocks`]) and the frame decoded their art,
//! never by the title's name, so a PSP title's rows are exactly what they
//! measured and an HD title's are exactly what its code draws.

use oag_core::buttons::Button;

use crate::frontend::{Align, Draw};

use super::{DIMMED, Entry, Menu, Skin, WARNING};

/// How far a value's first glyph sits in from its block's left edge, in the
/// title's own units. **Measured**, off `hd-settings-screenshot-2/01.png`:
/// `13.0` on both rows read, at a `0.639` ruler. `List_Item.cpp`'s own text
/// placement was not read, so this is the capture's number rather than the
/// executable's - the same status the strip's `TAB_TOP_PAD` has.
const VALUE_TEXT_INSET: f32 = 13.0;

/// The rows of `menu`'s current page, top to bottom.
///
/// Returns the body layer alone: the screen title is chrome and stays with the
/// caller, because a page change moves the two differently. See
/// [`super::Layers`].
///
/// `frame` carries the block art and the two fill colours an HD row box
/// needs; a PSP title's frame has none of them and reaches
/// [`draw_text_rows`] the way it always did.
pub(super) fn draw(
    menu: &Menu,
    skin: &Skin,
    bindings: &dyn Fn(Button) -> Vec<&'static str>,
    frame: &super::Frame,
) -> Vec<Draw> {
    match (skin.blocks(), skin.list(), frame.blocks) {
        (Some(blocks), Some(list), Some(art)) => {
            draw_list_rows(menu, skin, bindings, frame, blocks, list, art)
        }
        _ => draw_text_rows(menu, skin, bindings),
    }
}

/// The rows as `List_CreateWidgets` builds them and `List_Update` keeps
/// them: a 520-wide label block, a value block ten units right of it that
/// eases from 280 to 340 wide as the row takes focus, both `HD_Blue` on the
/// focused row and `HD_Grey` on the rest, and the step arrows at the label
/// block's right end.
///
/// The column's anchor and pitch are the screens' own (`<Item OffsetX="160"
/// OffsetY="170">`, rows fifty apart - [`super::Skin::list`]), the widths
/// and heights the executable's ([`oag_title::ListBlocks`]).
///
/// Three things are this build's, each marked below: which rows carry a
/// value block, the text's inset inside each block, and how a value with no
/// block behind it - a binding's key names, which HD has no row for - is
/// still shown.
fn draw_list_rows(
    menu: &Menu,
    skin: &Skin,
    bindings: &dyn Fn(Button) -> Vec<&'static str>,
    frame: &super::Frame,
    blocks: oag_title::MenuBlocks,
    list: super::skin::List,
    art: super::block::BlockArt,
) -> Vec<Draw> {
    let page = menu.page();
    let (sx, sy) = skin.theirs_scale();
    let numbers = blocks.list;
    let row_scale = skin.row_scale() * list.text_scale;
    let (_, tab_top_pad) = skin.tab_pad();
    let label_width = numbers.label_width * sx;
    let value_x = list.x + label_width + numbers.gap * sx;
    let visible = menu.visible_rows();
    let first = menu.scroll();

    let mut out = Vec::new();
    let mut noted: Option<String> = None;
    let mut shown = 0usize;
    for (row, entry) in page.entries.iter().enumerate().skip(first).take(visible) {
        let geometry = list_row(&list, &numbers, (sx, sy), first, row, menu.focus_of(row));
        let y = geometry.y;
        shown += 1;
        let selected = row == menu.selected();
        let inert = matches!(entry, Entry::Binding { .. }) || menu.is_disabled(entry);

        // `HD_Blue` while the row has focus, `HD_Grey` otherwise - both
        // switch outright; the width is what eases. A served archive whose
        // globals carry neither draws no boxes, and the row is its text.
        let fill = if selected {
            frame.tab_selected
        } else {
            frame.ink
        };

        if let Some(color) = fill {
            super::block::draw(
                &super::block::Block {
                    x: geometry.label[0],
                    y,
                    width: geometry.label[2],
                    height: geometry.label[3],
                    color,
                    landing: false,
                },
                &art,
                (sx, sy),
                &mut out,
            );
        }

        let note = menu
            .warning(entry)
            .map(|warning| &warning.message)
            .or_else(|| menu.restart_note(entry).map(|restart| &restart.message));
        if let Some(message) = note {
            noted.get_or_insert(message.clone());
            out.push(Draw::Text {
                x: list.x - 18.0 * sx,
                y: y + tab_top_pad,
                scale: row_scale,
                color: WARNING,
                border: None,
                align: Align::Left,
                text: "!".to_string(),
                wrap_width: None,
            });
        }

        // The label sits past the marker image's slot: `List_CreateWidgets`
        // puts a 32-wide marker at `x + 8`, and the capture puts the first
        // glyph at `x + 40.7`. The vertical placement is the strip's
        // measured pad, for the same atlas reason `strip.rs` gives.
        out.push(Draw::Text {
            x: list.x + (numbers.marker_offset.0 + numbers.arrow_size) * sx,
            y: y + tab_top_pad,
            scale: row_scale,
            color: if inert { DIMMED } else { skin.normal() },
            border: None,
            align: Align::Left,
            text: entry.label().to_string(),
            wrap_width: None,
        });

        // The value block and the arrows, on the rows that have a value.
        // **Ours**: HD's `<List>` always has both, but this build's tree
        // has submenu and action rows in the same column, and a value box
        // with nothing in it would be a box for its own sake.
        let value = match entry {
            Entry::Binding { button, .. } => {
                let keys = bindings(*button);
                if keys.is_empty() {
                    Some("UNBOUND".to_string())
                } else {
                    Some(keys.join(" / "))
                }
            }
            other => menu.shown(other).map(|value| value.to_string()),
        };
        if let Some(text) = value {
            if let Some(color) = fill {
                super::block::draw(
                    &super::block::Block {
                        x: geometry.value[0],
                        y,
                        width: geometry.value[2],
                        height: geometry.value[3],
                        color,
                        landing: true,
                    },
                    &art,
                    (sx, sy),
                    &mut out,
                );
            }
            // The value's inset is the capture's - see `VALUE_TEXT_INSET`.
            out.push(Draw::Text {
                x: value_x + VALUE_TEXT_INSET * sx,
                y: y + tab_top_pad,
                scale: row_scale,
                color: if inert { DIMMED } else { skin.normal() },
                border: None,
                align: Align::Left,
                text,
                wrap_width: None,
            });

            // The arrows, for a row that steps. This build's choices wrap,
            // so a step is always possible both ways and both arrows are
            // lit; the original dims the one at a list's end, and dims
            // both on a disabled row, which is the case kept here.
            if entry.is_adjustable()
                && let Some(arrow) = art.arrow
            {
                let color = if inert {
                    super::skin::argb(numbers.arrow_inert)
                } else {
                    skin.normal()
                };
                let [left_rect, right_rect] =
                    arrow_rects(&numbers, list.x, label_width, y, (sx, sy));
                let uv = [
                    arrow.x as f32,
                    arrow.y as f32,
                    arrow.width as f32,
                    arrow.height as f32,
                ];
                // The left arrow is the right one drawn with a negative
                // width from its anchor, so it extends leftward, mirrored.
                out.push(Draw::Sprite {
                    rect: left_rect,
                    uv: [uv[0] + uv[2], uv[1], -uv[2], uv[3]],
                    color,
                });
                out.push(Draw::Sprite {
                    rect: right_rect,
                    uv,
                    color,
                });
            }
        }
    }

    if let Some(message) = noted {
        out.push(Draw::Text {
            x: list.x - 18.0 * sx,
            y: list.y + shown as f32 * list.pitch + skin.message_gap(),
            scale: row_scale * skin.message_scale(),
            color: WARNING,
            border: None,
            align: Align::Left,
            text: format!("! {message}"),
            wrap_width: None,
        });
    }

    out
}

/// The rows as bare text, selected by brightening: both PSP titles'
/// measured idiom, and every title's before 2026-09-14.
fn draw_text_rows(
    menu: &Menu,
    skin: &Skin,
    bindings: &dyn Fn(Button) -> Vec<&'static str>,
) -> Vec<Draw> {
    let page = menu.page();
    let margin_x = skin.menu_x();
    let row_height = skin.row_pitch();
    let row_scale = skin.row_scale();
    let first_row_y = skin.first_row_y();
    // The cached figure `Menu::scroll`'s own window math already used to
    // place this window, not a fresh recompute off `skin` alone: the two
    // disagreeing is how a row `Menu::scroll` counted as on screen could
    // still go undrawn. Whoever is driving the menu is responsible for
    // keeping it fresh - see `menu::visible_rows`'s own doc for who that is
    // and why it takes `reserve_note`.
    let visible = menu.visible_rows();

    let mut out = Vec::new();

    // The first noted row's message, shown once under the rows however many
    // rows are marked: two lines of small text competing for the same corner
    // would be less readable than one, and the markers already say which rows.
    // A restart note and a warning share the slot and the first row in page
    // order wins, because they are the same kind of thing to a player - "this
    // row is not doing what it says" - and ranking them would mean deciding
    // which of two true sentences to hide.
    //
    // Only the rows on screen are considered, markers and message alike: the
    // message sits under the rows and the markers say which of them, so a note
    // belonging to a row that has scrolled away would be a sentence about
    // nothing the player can see.
    let mut noted: Option<String> = None;
    let first = menu.scroll();
    let mut shown = 0usize;
    for (row, entry) in page.entries.iter().enumerate().skip(first).take(visible) {
        let y = first_row_y + (row - first) as f32 * row_height;
        shown += 1;
        let selected = row == menu.selected();

        // **No fill behind the selected row.** The original marks selection by
        // brightening the row's own text toward white - a capture of its main
        // menu shows no bar, and none of the pink `MenuHighLightArrowColor` the
        // palette declares appears anywhere on that screen. This build used to
        // draw a translucent bar here; it was invented, and it is gone.

        // A binding cannot be changed yet and a disabled row cannot be changed
        // now; both read as "this does nothing if you press it", which is what
        // the dim colour says. A disabled row can still be selected and read
        // rather than being unreachable.
        let inert = matches!(entry, Entry::Binding { .. }) || menu.is_disabled(entry);
        // Marked in the margin rather than by recolouring the row: the colour
        // already means selected, normal or inert, and a fourth meaning on the
        // same channel would collide with those three. See `WARNING`.
        let note = menu
            .warning(entry)
            .map(|warning| &warning.message)
            .or_else(|| menu.restart_note(entry).map(|restart| &restart.message));
        if let Some(message) = note {
            noted.get_or_insert(message.clone());
            out.push(Draw::Text {
                x: margin_x - 18.0,
                y,
                scale: row_scale,
                color: WARNING,
                border: None,
                align: Align::Left,
                text: "!".to_string(),
                wrap_width: None,
            });
        }
        out.push(Draw::Text {
            x: margin_x,
            y,
            scale: row_scale,
            color: if inert {
                DIMMED
            } else if selected {
                skin.selected()
            } else {
                skin.normal()
            },
            border: None,
            align: Align::Left,
            text: entry.label().to_string(),
            wrap_width: None,
        });

        // The right-hand column: a setting's value, or what a button is bound
        // to. `Align::Right` anchors at `x - width`, so both kinds land on the
        // same edge whatever they say.
        let value = match entry {
            Entry::Binding { button, .. } => {
                let keys = bindings(*button);
                if keys.is_empty() {
                    Some("UNBOUND".to_string())
                } else {
                    Some(keys.join(" / "))
                }
            }
            other => menu.shown(other).map(|value| value.to_string()),
        };
        if let Some(text) = value {
            out.push(Draw::Text {
                x: skin.value_right(),
                y,
                scale: row_scale,
                color: if selected && entry.is_adjustable() && !inert {
                    skin.selected()
                } else {
                    DIMMED
                },
                border: None,
                align: Align::Right,
                text,
                wrap_width: None,
            });
        }
    }

    // The selected row's own subtitle, only where a title's skin measured
    // where one goes - `Skin::help_text` is `None` on every title but Pulse.
    // Only drawn when the selected row is actually on screen: it always is
    // in practice, since `Menu::scroll` keeps the selection inside the
    // window, but this is the same defensive bound the rows above and the
    // note below already apply to their own row.
    let selected_row = menu.selected();
    if let (Some(subtitle), Some(Some(text))) = (skin.help_text(), page.subtitles.get(selected_row))
        && (first..first + visible).contains(&selected_row)
    {
        // `Default`-role, not the row face this column otherwise draws in -
        // see `oag_title::HelpText::scale`'s own doc for why the atlas
        // changed under this widget rather than its scale.
        out.push(Draw::in_role(
            Some(crate::language::roles::DEFAULT),
            margin_x,
            first_row_y + (selected_row - first) as f32 * row_height + subtitle.offset_y,
            subtitle.scale,
            subtitle.color,
            Align::Left,
            text.clone(),
        ));
    }

    // Under the last row *drawn* rather than at a fixed height, so it sits with
    // the page it belongs to instead of floating away from a short one - and so
    // a scrolled page puts it under the window rather than off the bottom.
    if let Some(message) = noted {
        out.push(Draw::Text {
            x: margin_x - 18.0,
            y: first_row_y + shown as f32 * row_height + skin.message_gap(),
            scale: row_scale * skin.message_scale(),
            color: WARNING,
            border: None,
            align: Align::Left,
            text: format!("! {message}"),
            wrap_width: None,
        });
    }

    out
}

/// The two blocks of one HD list row, at the row's own focus: `[x, y,
/// width, height]` each, in the grid being drawn in.
///
/// **The one place the row's geometry is written**, read by
/// [`draw_list_rows`] for the boxes it paints and by [`regions`] for the
/// rects a pointer is tested against - so the two cannot disagree about
/// where a row is, which is the drift `rows.rs` already guards against
/// between `Menu::scroll` and the skin.
struct ListRow {
    /// The row's top, the label block's and the value block's alike.
    y: f32,
    label: [f32; 4],
    /// The value block, grown by `focus` toward its focused width.
    value: [f32; 4],
}

fn list_row(
    list: &super::skin::List,
    numbers: &oag_title::ListBlocks,
    (sx, sy): (f32, f32),
    first: usize,
    row: usize,
    focus: f32,
) -> ListRow {
    let y = list.y + (row - first) as f32 * list.pitch;
    let height = numbers.height * sy;
    let label_width = numbers.label_width * sx;
    let value_x = list.x + label_width + numbers.gap * sx;
    let grown = numbers.value_width + (numbers.value_focus_width - numbers.value_width) * focus;
    ListRow {
        y,
        label: [list.x, y, label_width, height],
        value: [value_x, y, grown * sx, height],
    }
}

/// Where a list row's two step arrows are drawn: the left one first. Both
/// `[x, y, width, height]`, in the grid being drawn in.
///
/// The left arrow is anchored by its right edge - it is the right one
/// drawn mirrored from the same anchor - so its rect starts a full arrow
/// width before `arrow_left_offset`.
fn arrow_rects(
    numbers: &oag_title::ListBlocks,
    list_x: f32,
    label_width: f32,
    y: f32,
    (sx, sy): (f32, f32),
) -> [[f32; 4]; 2] {
    let size = numbers.arrow_size * sx;
    let label_right = list_x + label_width;
    let top = y + numbers.marker_offset.1 * sy;
    let height = numbers.arrow_size * sy;
    [
        [
            label_right + numbers.arrow_left_offset * sx - size,
            top,
            size,
            height,
        ],
        [
            label_right + numbers.arrow_right_offset * sx,
            top,
            size,
            height,
        ],
    ]
}

/// The rects a pointer can land on, for the rows on screen - the same
/// rows, at the same places, [`draw`] just painted them. See
/// [`super::pointer`].
///
/// A text row (both PSP titles) is one band the width of the column, from
/// the margin's marker slot to the value's right edge: there are no arrows
/// to aim at, so the whole row is the target and a click activates it,
/// which on an adjustable row steps it forward the way cross does. An HD
/// list row is its label block and its value block, both activating, plus
/// the two step arrows on a row that has them - the natural target for a
/// `choice` that the original draws them for. The value block is tested
/// at the width it has *right now*, mid-ease included, because that is the
/// box the player sees.
pub(super) fn regions(
    menu: &Menu,
    skin: &Skin,
    frame: &super::Frame,
) -> Vec<super::pointer::Region> {
    use super::pointer::{Part, Region};
    let page = menu.page();
    let visible = menu.visible_rows();
    let first = menu.scroll();
    let mut out = Vec::new();
    match (skin.blocks(), skin.list(), frame.blocks) {
        (Some(blocks), Some(list), Some(art)) => {
            let (sx, sy) = skin.theirs_scale();
            let numbers = blocks.list;
            for (row, entry) in page.entries.iter().enumerate().skip(first).take(visible) {
                let geometry = list_row(&list, &numbers, (sx, sy), first, row, menu.focus_of(row));
                out.push(Region {
                    row,
                    part: Part::Row,
                    rect: geometry.label,
                });
                // A value block only where a value is drawn - the same rows
                // `draw_list_rows` gives one, which is every row with
                // something to show in it.
                let has_value = matches!(entry, Entry::Binding { .. }) || entry.value().is_some();
                if has_value {
                    out.push(Region {
                        row,
                        part: Part::Row,
                        rect: geometry.value,
                    });
                }
                if has_value && entry.is_adjustable() && art.arrow.is_some() {
                    let [left, right] =
                        arrow_rects(&numbers, list.x, geometry.label[2], geometry.y, (sx, sy));
                    out.push(Region {
                        row,
                        part: Part::StepBack,
                        rect: left,
                    });
                    out.push(Region {
                        row,
                        part: Part::StepForward,
                        rect: right,
                    });
                }
            }
        }
        _ => {
            let margin_x = skin.menu_x();
            let row_height = skin.row_pitch();
            let first_row_y = skin.first_row_y();
            // From the marker's slot (`margin_x - 18.0`, where a `!` goes) to
            // the value column's right edge - the same two x's the text is
            // drawn between.
            let left = margin_x - 18.0;
            let width = (skin.value_right() - left).max(0.0);
            for row in (first..page.entries.len()).take(visible) {
                let y = first_row_y + (row - first) as f32 * row_height;
                out.push(Region {
                    row,
                    part: Part::Row,
                    rect: [left, y, width, row_height],
                });
            }
        }
    }
    out
}
