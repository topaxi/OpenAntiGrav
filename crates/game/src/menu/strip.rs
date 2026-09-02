//! A page drawn as the original's `<HorizMenu>`: entries left to right.
//!
//! Wipeout HD's main menu is horizontal where both PSP titles' are a column, and
//! that is authored rather than styled - `MainMenu_Definition.xml` carries one
//! `<HorizMenu name="Mode">` and no `<Menu>` at all. This module is what draws
//! it; [`super::rows`] is the column, and [`super::draw_list`] picks.
//!
//! # What is the disc's here, and what is ours
//!
//! The disc's: that the menu is horizontal at all, where it starts, and what
//! colour an entry is - [`oag_title::MenuStrip`], read off five archives' copies
//! of the same file.
//!
//! Ours, and each marked where it is made: the gap between two entries
//! ([`super::Skin::strip_gap`]), the selected colour on a title with no capture
//! ([`super::Skin::selected`]), and [`suits`]'s rule about which pages get drawn
//! this way.
//!
//! # Every entry is drawn, and there is no carousel
//!
//! The widget states one position and lists its entries; nothing in it says the
//! selected entry is centred, or that the strip scrolls, or that entries off the
//! end are hidden. So all of them are drawn from the anchor, which is the only
//! reading the data supports. A capture of the original would settle whether it
//! centres the selection - none exists, and a carousel built on the guess would
//! be exactly the plausible-looking stand-in `CLAUDE.md` forbids.

use crate::frontend::{Align, Draw};

use super::{Entry, Menu, Page, Skin, skin::Strip};

/// Whether this page can be drawn as a strip.
///
/// **Ours.** The rule is that every entry is *navigation* - a submenu, an
/// action, or the way back - because a strip has nowhere to put a right-hand
/// column: [`super::rows`] anchors a value at [`super::Skin::value_right`], and
/// entries running left to right have no such edge to share.
///
/// It is shaped by what HD does rather than stated by it. Its own `<HorizMenu>`
/// screens - `Main Menu`, `Additional`, `Extras`, `Controls Menu` - are all
/// pages that only choose where to go next, while `Settings` and the rest of the
/// pages that carry values are `<Item>` lists. That is a consistent split and it
/// is not a declaration, so the rule is written here as this build's, not filed
/// as a recovered one.
///
/// **On `assets/ui/menu.toml` as it stands this fires on the root page alone.**
/// `options` looks like `Additional` and would be a strip on HD's own reading,
/// but this build hangs a `choice` off it, so it keeps its value column. Counted
/// rather than assumed: of the seven pages, `main` is the only one with no
/// `choice`, `toggle` or `binding` on it.
///
/// Matched on the variant rather than on [`Entry::value`], deliberately: a
/// `choice` whose list the source turned out not to carry has no value to show
/// and is still a choice, and a page of those must not silently become a strip.
pub(super) fn suits(page: &Page) -> bool {
    !page.entries.is_empty()
        && page.entries.iter().all(|entry| {
            matches!(
                entry,
                Entry::Submenu { .. } | Entry::Run { .. } | Entry::Back { .. }
            )
        })
}

/// The entries of `menu`'s current page, left to right along the strip.
///
/// Returns the body layer alone, as [`super::rows::draw`] does: the screen title
/// is chrome, and a page change moves the two differently.
///
/// `measure` is the width of a string in the face the entries will be drawn in,
/// at scale 1. Passed in for the reason `bindings` is: the atlas belongs to the
/// renderer and the layout belongs here, so neither has to hold the other. A
/// column never needed it - rows start at one x - and a strip cannot be laid out
/// without it.
///
/// `frame` supplies the two colours a capture settled: [`super::Frame::ink`]
/// (`HD_Grey`) for an unselected tab and [`super::Frame::tab_selected`]
/// (`HD_Blue`) for the selected one. A colour that is `None` - a title with no
/// frame at all, or a served archive whose globals do not carry the name -
/// skips its own fill rather than inventing one, the same rule
/// [`super::read_frame`] already applies to a mark whose texture did not
/// decode.
pub(super) fn draw(
    menu: &Menu,
    skin: &Skin,
    strip: Strip,
    measure: &dyn Fn(&str) -> f32,
    frame: &super::Frame,
) -> Vec<Draw> {
    let scale = skin.row_scale();
    let gap = skin.strip_gap();
    let (tab_left_pad, tab_top_pad) = skin.tab_pad();
    let tab_height = skin.tab_height();
    let (chamfer_width, chamfer_height) = skin.tab_chamfer();
    let (underline_width, underline_height, underline_offset_y) = skin.underline();

    let mut out = Vec::new();
    let mut x = strip.x;
    for (index, entry) in menu.page().entries.iter().enumerate() {
        let label = entry.label();
        let selected = index == menu.selected();
        let width = measure(label) * scale;

        // The tab behind the label, in two bands rather than one rectangle:
        // the real tab's top-right corner is chamfered, and a diagonal cut
        // needs a primitive this build's `Draw`/`Quad` pipeline does not
        // have. A staircase of one step - the full tab below the chamfer's
        // height, a second band above it short by the chamfer's width - draws
        // the true measured size and position of the cut with the primitive
        // that already exists. See `Skin`'s `TAB_LEFT_PAD` doc for the raw
        // measurement and the render comparison that checked this was worth
        // shipping.
        let fill = if selected {
            frame.tab_selected
        } else {
            frame.ink
        };
        if let Some(color) = fill {
            let tab_width = tab_left_pad + width;
            out.push(Draw::Fill {
                rect: [
                    x - tab_left_pad,
                    strip.y - tab_top_pad + chamfer_height,
                    tab_width,
                    tab_height - chamfer_height,
                ],
                color,
            });
            // A label short enough that its tab is narrower than the chamfer
            // itself would otherwise flip this band's width negative - the
            // chamfer only ever eats up to the whole top band, never past it.
            out.push(Draw::Fill {
                rect: [
                    x - tab_left_pad,
                    strip.y - tab_top_pad,
                    (tab_width - chamfer_width).max(0.0),
                    chamfer_height,
                ],
                color,
            });
        }

        out.push(Draw::Text {
            x,
            y: strip.y,
            scale,
            // Every entry is this same colour, selected or not - a 2026-09-01
            // capture found no text brightening at all, only the tab fill
            // above changing. See `Strip::color`'s own doc for the reading
            // this replaced.
            color: skin.normal(),
            border: None,
            align: Align::Left,
            text: label.to_string(),
            wrap_width: None,
        });

        // The underline mark, selected entry only, and only alongside its
        // own tab fill: the mark reads as underlining the tab, so on a title
        // with no frame colours to fill the tab with, drawing the mark alone
        // would be a floating dash under nothing rather than the absence
        // `super::read_frame` already chose. Its own colour is measured
        // rather than assumed: the one capture with a visible mark shows it
        // in the label's own white, not the tab's `HD_Blue`.
        if selected && fill.is_some() {
            out.push(Draw::Fill {
                rect: [
                    x,
                    strip.y + underline_offset_y,
                    underline_width,
                    underline_height,
                ],
                color: skin.normal(),
            });
        }

        x += width + gap;
    }
    out
}
