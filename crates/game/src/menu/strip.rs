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
pub(super) fn draw(
    menu: &Menu,
    skin: &Skin,
    strip: Strip,
    measure: &dyn Fn(&str) -> f32,
) -> Vec<Draw> {
    let scale = skin.row_scale();
    let gap = skin.strip_gap();

    let mut out = Vec::new();
    let mut x = strip.x;
    for (index, entry) in menu.page().entries.iter().enumerate() {
        let label = entry.label();
        out.push(Draw::Text {
            x,
            y: strip.y,
            scale,
            // The same two states a row has, and the same reasoning: the
            // original brightens the selected entry rather than putting a bar
            // behind it. What "brightened" is on this title is unmeasured, so
            // `Skin::selected` supplies this build's white - see its docs. The
            // unselected colour is the widget's own and is not this build's
            // choice at all.
            color: if index == menu.selected() {
                skin.selected()
            } else {
                strip.color
            },
            border: None,
            align: Align::Left,
            text: label.to_string(),
        });
        x += measure(label) * scale + gap;
    }
    out
}
