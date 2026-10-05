//! The one table `EndRace Results` authors, and which of it a mode shows.
//!
//! `EndRace_Definition.xml` authors a single grid - `zonetopline`,
//! `tablebg1`-`tablebg8`, `topbarcenter`, 27 `lap{r}.{c}` cells, `perfectlap1`-`5`,
//! `boostimg` and `tablehighlight` - and every mode's populate function fills
//! it differently. `EndRaceResults_ResetTable` hides the rows and the icons, and
//! each populate shows exactly what it uses: **bit `0x4` of a widget's flag
//! word is the visible bit**, which this crate had read the wrong way round
//! until 2026-09-30 and so drew all eight row backgrounds on every table. See
//! `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`'s "Bit `0x4` is
//! the visible bit".
//!
//! [`table_layers`] is the one walk over the layout every table shares;
//! what differs per mode is a [`Shown`] and the text a cell reads.

use oag_ui::frontend::{Draw, Placed};
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::screen::{Fill, Text, argb_to_rgba};

use super::Layout;
use super::draw::{fill_draw, image_draw, text_draw};

/// `tablebg1`'s own `OffsetY`, the top edge of the first row.
const FIRST_ROW_Y: f32 = 92.0;

/// The distance between two rows' `OffsetY`s.
const ROW_PITCH: f32 = 20.0;

/// How many rows the grid authors.
const ROWS: usize = 8;

/// The `y` a row's highlight sits at: `0x5d + 0x14 * (row - 1)`, the literal every
/// populate function steps - `EndRaceResults_PopulateLapTable` ends on `laps * 0x14 +
/// 0x5d` for its totals row, the Tournament and Elimination tables step it per craft.
/// One pixel below the row's own `tablebg` `OffsetY`.
#[must_use]
pub(super) fn highlight_y(row: usize) -> f32 {
    93.0 + row.saturating_sub(1) as f32 * ROW_PITCH
}

/// Which parts of the authored table one mode's populate function leaves shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Shown {
    /// Whether the `table` group is shown at all. `false` hides everything in it,
    /// which is what an ordinary lap table with no completed lap does.
    pub table: bool,
    /// `tablebg1..=rows`, and the tile and rules nested inside each.
    pub rows: usize,
    /// `topbarcenter`, the header bar. Zone has no header row and hides it.
    pub header_bar: bool,
    /// `boostimg`, the third column's header icon - the ordinary lap table alone.
    pub boost_icon: bool,
    /// The row `tablehighlight` sits on, or `None` to hide it.
    pub highlight: Option<usize>,
}

/// The row an unnamed widget at `y` is nested in, when it is one of the grid's row
/// backgrounds.
///
/// Each `tablebg{i}` is an `<Image OffsetY=...>` holding a dark fill, a `hex_bg.mip`
/// tile and two fading rules, and the format collects the nested three as anonymous
/// widgets at absolute positions - so their row is recoverable only from `y`. Row 1
/// owns `[92, 113)`, which takes in the rules it draws at `112`; row `i > 1` owns
/// `[93 + 20 (i - 1), 113 + 20 (i - 1))`. `docs` and a ground-truth test count four
/// widgets per row on the real file, so the bands are not a guess about it.
#[must_use]
pub(super) fn row_of(y: f32) -> Option<usize> {
    let end = FIRST_ROW_Y + ROWS as f32 * ROW_PITCH + 1.0;
    if !(FIRST_ROW_Y..end).contains(&y) {
        return None;
    }
    if y < FIRST_ROW_Y + ROW_PITCH + 1.0 {
        return Some(1);
    }
    let row = ((y - (FIRST_ROW_Y + 1.0)) / ROW_PITCH).floor() as usize + 1;
    Some(row.min(ROWS))
}

/// A `tablebg{N}` wrapper fill at the row's own position.
///
/// The wrapper is an `<Image name="tablebg{N}" OffsetY="...">` with no `src`, so the
/// screen reader collects it as a fill and places it at its own `y` **without** the
/// tag's `OffsetY` - the container offset reaches the widgets nested inside and, for
/// an image with a `src`, the image itself, but not a colour-only wrapper's own
/// rectangle. Left alone, all eight dark row backings pile up along the top of the
/// screen (`y` 0 and 1) and no row has one. The reader is shared by every screen and
/// nothing here measured how another one relies on that, so this puts the offset
/// back for the eight widgets it is known to matter for rather than changing it.
fn with_row_offset(fill: &Fill) -> Fill {
    let Some(row) = fill
        .name
        .as_deref()
        .and_then(|name| name.strip_prefix("tablebg"))
        .and_then(|number| number.parse::<usize>().ok())
        .filter(|row| (1..=ROWS).contains(row))
    else {
        return fill.clone();
    };
    Fill {
        y: fill.y + FIRST_ROW_Y + (row - 1) as f32 * ROW_PITCH,
        ..fill.clone()
    }
}

/// Whether a widget belongs to the `table` group: a named piece of it, or a row
/// background nested in one.
fn in_table(name: Option<&str>, y: f32) -> bool {
    matches!(
        name,
        Some("zonetopline" | "topbarcenter" | "boostimg" | "tablehighlight")
    ) || row_of(y).is_some()
        || is_top_rule(y)
}

/// The anonymous right half of `zonetopline`, nested in it at the same `y` (91) as the
/// named left half - a rule one pixel above the first row.
fn is_top_rule(y: f32) -> bool {
    (FIRST_ROW_Y - 1.0..FIRST_ROW_Y).contains(&y)
}

/// One `EndRace Results` draw list: the backdrop, then the layout's own fills,
/// images and texts, with the table gated by `shown` and every text's content asked
/// of `cell`.
///
/// `cell` answers for every `Text` widget but the `MSC_PL` overlay nested in each
/// `perfectlap{n}` (the icon it sits on is never shown - this build keeps no
/// per-lap perfect flag - so neither is its label). A `None` draws nothing.
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes, and the two the table adds"
)]
pub(super) fn table_layers(
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    shown: Shown,
    cell: &dyn Fn(&Text) -> Option<String>,
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
    let screen = &layout.screen;
    let mut out = Vec::new();
    let row_shown = |y: f32| row_of(y).is_none_or(|row| row <= shown.rows);
    for authored in &screen.fills {
        let name = authored.name.as_deref();
        let fill = &with_row_offset(authored);
        if name == Some("tablehighlight") {
            let Some(row) = shown.highlight.filter(|_| shown.table) else {
                continue;
            };
            out.push(Draw::Fill {
                rect: [
                    0.0,
                    highlight_y(row),
                    fill.width.unwrap_or(0.0),
                    fill.height.unwrap_or(0.0),
                ],
                color: argb_to_rgba(fill.color),
            });
            continue;
        }
        if in_table(name, fill.y) && !(shown.table && row_shown(fill.y)) {
            continue;
        }
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        let name = image.name.as_deref();
        let visible = match name {
            Some(name) if name.starts_with("perfectlap") => false,
            Some("boostimg") => shown.table && shown.boost_icon,
            Some("topbarcenter") => shown.table && shown.header_bar,
            _ => !in_table(name, image.y) || (shown.table && row_shown(image.y)),
        };
        if !visible {
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    for text in &screen.texts {
        if text.idstring.as_deref() == Some("MSC_PL") {
            continue;
        }
        let name = text.name.as_deref().unwrap_or("");
        if name.starts_with("lap") && !shown.table {
            continue;
        }
        let Some(content) = cell(text) else { continue };
        out.push(text_draw(text, &content, layout));
    }
    layers.body = out;
    layers
}

/// The `lap{n}.{c}` widget name a table cell carries, e.g. `"lap3.1"` ->
/// `(3, 1)`. `n = 0` is the header row.
#[must_use]
pub(super) fn lap_slot(name: &str) -> Option<(usize, usize)> {
    let rest = name.strip_prefix("lap")?;
    let mut parts = rest.split('.');
    let n = parts.next()?.parse().ok()?;
    let c = parts.next()?.parse().ok()?;
    Some((n, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bands take every widget a real `tablebg{i}` nests to its own row: the fill and
    /// tile at the row's `OffsetY` (plus one below the first), and the two rules at
    /// `OffsetY + 20`.
    #[test]
    fn a_row_owns_its_fill_its_tile_and_its_rules() {
        assert_eq!(row_of(91.0), None, "`zonetopline` sits above the first row");
        for row in 1..=ROWS {
            let offset = FIRST_ROW_Y + (row - 1) as f32 * ROW_PITCH;
            let fill = if row == 1 { offset } else { offset + 1.0 };
            assert_eq!(row_of(fill), Some(row), "row {row}'s fill");
            assert_eq!(row_of(offset + 20.0), Some(row), "row {row}'s rules");
        }
        assert_eq!(row_of(253.0), None);
    }

    /// The highlight steps the literal the executable does.
    #[test]
    fn the_highlight_sits_one_pixel_under_its_row() {
        assert!((highlight_y(1) - 93.0).abs() < f32::EPSILON);
        assert!((highlight_y(4) - 153.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lap_slot_reads_the_cell_names() {
        assert_eq!(lap_slot("lap3.1"), Some((3, 1)));
        assert_eq!(lap_slot("lap0.2"), Some((0, 2)));
        assert_eq!(lap_slot("Line1"), None);
    }
}
