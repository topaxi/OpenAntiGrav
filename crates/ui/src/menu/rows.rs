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

use oag_gameplay::input::Button;

use crate::frontend::{Align, Draw};

use super::{DIMMED, Entry, Menu, Skin, WARNING};

/// The rows of `menu`'s current page, top to bottom.
///
/// Returns the body layer alone: the screen title is chrome and stays with the
/// caller, because a page change moves the two differently. See
/// [`super::Layers`].
pub(super) fn draw(
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
            other => other.value().map(|value| value.to_string()),
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
