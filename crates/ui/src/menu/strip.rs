//! A page drawn as the original's `<HorizMenu>`: entries left to right.
//!
//! Wipeout HD's main menu is horizontal where both PSP titles' are a column, and
//! that is authored rather than styled - `MainMenu_Definition.xml` carries one
//! `<HorizMenu name="Mode">` and no `<Menu>` at all. This module is what draws
//! it; [`super::rows`] is the column, and [`super::draw_list`] picks.
//!
//! # What is the disc's here, what is the executable's, and what is ours
//!
//! The disc's: that the menu is horizontal at all, where it starts, and what
//! colour an entry is - [`oag_title::MenuStrip`], read off five archives' copies
//! of the same file - and the two textures the tab and its underline are cut
//! from, decoded by the caller into [`super::Frame::blocks`].
//!
//! The executable's, since 2026-09-14: everything about the tab itself. Its
//! width is `HorizMenu_Item.cpp`'s `ItemWidth` default of 298, the selected
//! one is 70 wider and eases there at a sixth of the remaining distance a
//! tick, the pitch is the width plus 10, the label is inset 10, the box
//! behind it is a `Block` - see [`super::block`] and
//! [`oag_title::MenuBlocks`] for every number's address. **This replaced a set
//! of measured figures** (`TAB_*` in `skin.rs`, which [`draw_measured`] still
//! reads for a title that has a strip and no blocks): those were read off
//! captures at ruler accuracy and came out within a unit of these, which is
//! what says both are right.
//!
//! Ours, and marked where made: [`suits`]'s rule about which pages get drawn
//! this way, the label's vertical placement inside its tab
//! ([`super::Skin::tab_pad`], a measured figure this build's font atlas still
//! needs), and - on a title with no blocks - the measured tab.
//!
//! # What the anchor means: the tab's corner, not the label's pen
//!
//! [`oag_title::MenuStrip::x`] and [`oag_title::MenuStrip::y`] are the
//! `<HorizMenu>`'s own `x="160" y="125"`, and they are the **top-left corner of
//! an entry's block**. `HorizMenu_AddEntryBlock` puts the block there and
//! `HorizMenu_LayoutBlocks` puts the label ten units in from it - which is
//! what a 2026-09-05 capture had measured before the code was read: the tab's
//! corner within a unit of the anchor on both axes, this module having read
//! the anchor as the label's pen until then.
//!
//! # Every entry is drawn, and there is no carousel
//!
//! `HorizMenu_LayoutBlocks` walks every entry from the anchor and moves
//! nothing to centre the selection; a 2026-09-05 capture of `RECORDS`, the
//! last entry, sitting at its natural position at the strip's right end had
//! already said so.

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
/// renderer and the layout belongs here, so neither has to hold the other. Only
/// [`draw_measured`] needs it - the executable's tab has a width of its own -
/// but a title with a strip and no blocks still has to size a tab somehow.
///
/// `frame` supplies the two colours a capture settled: [`super::Frame::ink`]
/// (`HD_Grey`) for an unselected tab and [`super::Frame::tab_selected`]
/// (`HD_Blue`) for the selected one, and the block art. A colour that is
/// `None` - a title with no frame at all, or a served archive whose globals do
/// not carry the name - skips its own fill rather than inventing one, the same
/// rule [`super::read_frame`] already applies to a mark whose texture did not
/// decode.
pub(super) fn draw(
    menu: &Menu,
    skin: &Skin,
    strip: Strip,
    measure: &dyn Fn(&str) -> f32,
    frame: &super::Frame,
) -> Vec<Draw> {
    match (skin.blocks(), frame.blocks) {
        (Some(blocks), Some(art)) => draw_blocks(menu, skin, strip, frame, blocks, art),
        _ => draw_measured(menu, skin, strip, measure, frame),
    }
}

/// The strip as `HorizMenu_LayoutBlocks` lays it out: one `Block` per entry
/// at the executable's own widths, the selected one eased wider, the label
/// ten units in, and the underline `Image` sliding in and blinking on the
/// page's own arrival counter.
fn draw_blocks(
    menu: &Menu,
    skin: &Skin,
    strip: Strip,
    frame: &super::Frame,
    blocks: oag_title::MenuBlocks,
    art: super::block::BlockArt,
) -> Vec<Draw> {
    let scale = skin.row_scale();
    let (sx, sy) = skin.theirs_scale();
    let numbers = blocks.strip;
    let (_, tab_top_pad) = skin.tab_pad();
    let gap = numbers.gap * sx;
    let height = numbers.height * sy;

    let mut out = Vec::new();
    let mut left = strip.x;
    for (index, entry) in menu.page().entries.iter().enumerate() {
        let selected = index == menu.selected();
        // `ItemWidth`, plus the selected bonus by however far this entry's
        // block has grown toward it - `1.0` at rest on the selected entry,
        // `0.0` on the rest, in between during a cursor move.
        let width = (numbers.item_width + numbers.focus_extra * menu.focus_of(index)) * sx;

        // The block's colour switches on selection outright; only the width
        // eases. `HD_Blue` for the selected entry, `HD_Grey` for the rest,
        // each resolved off the served archive - and a title whose globals
        // carry neither draws no block at all.
        let fill = if selected {
            frame.tab_selected
        } else {
            frame.ink
        };
        if let Some(color) = fill {
            super::block::draw(
                &super::block::Block {
                    x: left,
                    y: strip.y,
                    width,
                    height,
                    color,
                    landing: true,
                },
                &art,
                (sx, sy),
                &mut out,
            );
        }

        out.push(Draw::Text {
            x: left + gap,
            y: strip.y + tab_top_pad,
            scale,
            // Every entry is this same colour, selected or not: `HD_White`
            // in `HorizMenu_LayoutBlocks`, which is `TextColor` here.
            color: skin.normal(),
            border: None,
            align: Align::Left,
            text: entry.label().to_string(),
            wrap_width: None,
        });

        // The underline, selected entry only, and only alongside its own
        // tab: on a title with no fill the mark would float under nothing.
        if selected
            && fill.is_some()
            && let Some(cursor) = art.cursor
        {
            underline(&numbers, cursor, menu, skin, left, strip.y, &mut out);
        }

        left += width + gap;
    }
    out
}

/// The selected entry's underline: `cursor.gtf` at `(x + 10 + slide, y + 35)`.
///
/// On the tick a page arrives `slide` is a whole `ItemWidth` - the mark
/// starts off the tab's right end - and it is multiplied by five sixths every
/// tick after, so the mark sweeps in and settles over about half a second.
/// The same counter blinks it: eight ticks on, nine off. Both from
/// `HorizMenu_LayoutBlocks`; a settled menu draws it at rest and visible.
///
/// The image goes at `y + 35`, and its bar is at row 7 of the sheet's copy
/// of the texture - file row 1, a `.gtf`'s rows running bottom-up - so the
/// bar's top lands at `y + 42`, which is the capture's `y + 42.6`. The
/// residual this module carried against the measured figure until the
/// sheet's own flip was accounted for is gone; the measured
/// [`super::Skin::underline`] offset is no longer read here.
fn underline(
    numbers: &oag_title::StripBlocks,
    cursor: crate::frontend::Placed,
    menu: &Menu,
    skin: &Skin,
    x: f32,
    y: f32,
    out: &mut Vec<Draw>,
) {
    let (sx, sy) = skin.theirs_scale();
    let (slide, visible) = match menu.ticks_since_arrival() {
        Some(ticks) => {
            let period = numbers.underline_on_ticks + numbers.underline_off_ticks;
            let decay = 5.0_f32 / 6.0;
            (
                numbers.item_width * decay.powi(i32::try_from(ticks).unwrap_or(i32::MAX)),
                ticks % period < numbers.underline_on_ticks,
            )
        }
        None => (0.0, true),
    };
    if !visible {
        return;
    }
    out.push(Draw::Sprite {
        rect: [
            x + (numbers.underline_offset.0 + slide) * sx,
            y + numbers.underline_offset.1 * sy,
            numbers.underline_size.0 * sx,
            numbers.underline_size.1 * sy,
        ],
        uv: [
            cursor.x as f32,
            cursor.y as f32,
            cursor.width as f32,
            cursor.height as f32,
        ],
        color: skin.normal(),
    });
}

/// The strip drawn from measured figures, for a title with a strip and no
/// block art - the way every tab was drawn until 2026-09-14.
///
/// The tab is sized from its label plus a pad, the corner is the measured
/// cut-and-landing (`skin.rs`'s `TAB_*` group), and the underline is a plain
/// fill. Kept rather than deleted because the frame's textures can fail to
/// decode, and a strip with no tabs at all is a worse fallback than one
/// within a unit of the capture.
fn draw_measured(
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
    let (chamfer_cut, chamfer_landing, chamfer_height) = skin.tab_chamfer();
    let (underline_width, underline_height, underline_offset_y) = skin.underline();

    let mut out = Vec::new();
    // The **tab's own left edge**, not the label's pen position - see this
    // module's own "what the anchor means" note. The pen is derived from it.
    let mut left = strip.x;
    for (index, entry) in menu.page().entries.iter().enumerate() {
        let label = entry.label();
        let selected = index == menu.selected();
        let width = measure(label) * scale;
        let pen_x = left + tab_left_pad;
        let pen_y = strip.y + tab_top_pad;

        // The tab behind the label, in two pieces: the real tab's top-right
        // corner is a 45-degree cut followed by a flat landing before the
        // vertical right edge - a pentagon, not a rectangle and not a single
        // diagonal to the corner. The full-width part below the cut is an
        // ordinary rectangle; the band above it is one `ChamferedFill`,
        // narrowed by the landing so its right edge is where the diagonal
        // lands and chamfered by the cut so its top edge is where the diagonal
        // starts. The landing itself is then just the gap between the band's
        // right edge and the tab's, and nothing draws it. See `Skin`'s
        // `TAB_CHAMFER_CUT` doc for the measurement.
        let fill = if selected {
            frame.tab_selected
        } else {
            frame.ink
        };
        if let Some(color) = fill {
            let tab_width = tab_left_pad + width;
            out.push(Draw::Fill {
                rect: [
                    left,
                    strip.y + chamfer_height,
                    tab_width,
                    tab_height - chamfer_height,
                ],
                color,
            });
            // A label short enough that its tab is narrower than the landing
            // would otherwise flip the band's width negative. `ChamferedFill`
            // clamps its own cut to the band's width, so the two together
            // shrink the band to nothing rather than inverting it.
            out.push(Draw::ChamferedFill {
                rect: [
                    left,
                    strip.y,
                    (tab_width - chamfer_landing).max(0.0),
                    chamfer_height,
                ],
                chamfer: [0.0, chamfer_cut],
                color,
            });
        }

        out.push(Draw::Text {
            x: pen_x,
            y: pen_y,
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
                    pen_x,
                    pen_y + underline_offset_y,
                    underline_width,
                    underline_height,
                ],
                color: skin.normal(),
            });
        }

        // The pen advances by the same step it always did, so the gap between
        // two tabs is unchanged: `left` and `pen_x` differ by a constant.
        left += width + gap;
    }
    out
}

/// The rects a pointer can land on: one tab per entry, left to right, at
/// the width [`draw`] just gave it. See [`super::pointer`].
///
/// A strip is hit-tested on **x** where a column is hit-tested on y, and
/// every entry is navigation, so each tab is one [`super::pointer::Part::Row`]
/// region and nothing else. The widths are the same two rules the two
/// drawings use (the executable's `ItemWidth` plus the eased focus bonus
/// on a title with blocks, the label's measure plus the pad on one without)
/// and the tab advances by the same `width + gap` step, so the region and
/// the tab it stands for are one arithmetic.
pub(super) fn regions(
    menu: &Menu,
    skin: &Skin,
    strip: Strip,
    measure: &dyn Fn(&str) -> f32,
    frame: &super::Frame,
) -> Vec<super::pointer::Region> {
    use super::pointer::{Part, Region};
    let scale = skin.row_scale();
    let mut out = Vec::new();
    match (skin.blocks(), frame.blocks) {
        (Some(blocks), Some(_)) => {
            let (sx, sy) = skin.theirs_scale();
            let numbers = blocks.strip;
            let gap = numbers.gap * sx;
            let height = numbers.height * sy;
            let mut left = strip.x;
            for index in 0..menu.page().entries.len() {
                let width = (numbers.item_width + numbers.focus_extra * menu.focus_of(index)) * sx;
                out.push(Region {
                    row: index,
                    part: Part::Row,
                    rect: [left, strip.y, width, height],
                });
                left += width + gap;
            }
        }
        _ => {
            let gap = skin.strip_gap();
            let (tab_left_pad, _) = skin.tab_pad();
            let tab_height = skin.tab_height();
            let mut left = strip.x;
            for entry in &menu.page().entries {
                let width = measure(entry.label()) * scale;
                out.push(Region {
                    row: out.len(),
                    part: Part::Row,
                    rect: [left, strip.y, tab_left_pad + width, tab_height],
                });
                left += width + gap;
            }
        }
    }
    out
}
