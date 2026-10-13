//! The tables `EndRace Results` authors, and which of them a mode shows.
//!
//! `EndRace_Definition.xml` authors a grid - `zonetopline`, `tablebg1`-`tablebg8`,
//! `topbarcenter`, 27 `lap{r}.{c}` cells, `perfectlap1`-`5`, `boostimg` and
//! `tablehighlight` - and every mode's populate function fills it differently.
//! `EndRaceResults_ResetTable` hides the rows and the icons, and each populate shows
//! exactly what it uses: **bit `0x4` of a widget's flag word is the visible bit**,
//! which this crate had read the wrong way round until 2026-09-30 and so drew all
//! eight row backgrounds on every table. See
//! `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`'s "Bit `0x4` is the
//! visible bit".
//!
//! **The grid's geometry is the file's own** ([`Grid`]): the PSP authors a 20 px
//! pitch from `y = 92` in 480x272, the PS2 a 32.94 px pitch from 152 in 640x448
//! under a `-10` `Item`, and its `Results Split` two 25 px tables, one per person, whose
//! widgets carry a `_0`/`_1` suffix. Until 2026-10-13 the PSP's numbers were
//! constants here, so a PS2 table drew its row backs and its highlight in the wrong rows.
//!
//! [`table_layers`] is the one walk over the layout every table shares;
//! what differs per mode is a [`Shown`] per table and the text a cell reads.

use oag_ui::frontend::{Draw, Placed};
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::screen::{Fill, Text, argb_to_rgba};

use super::Layout;
use super::draw::{fill_draw, image_draw, text_draw};

/// How many rows the grid authors.
const ROWS: usize = 8;

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
    /// Further rows the highlight is drawn on as well, bit `r` for row `r`
    /// (1-based): a race several people flew marks each person's row, and the
    /// original's one highlight widget is drawn once per row. Zero everywhere else.
    pub also_highlight: u16,
}

/// Which table of a screen a widget belongs to: `None` for the one an ordinary
/// `EndRace Results` authors, `Some(n)` for `Results Split`'s `_n` copies.
pub(super) type Suffix = Option<u8>;

/// A widget name without its `_N` person suffix, and the suffix: `tablebg2_1` is
/// `("tablebg2", Some(1))`, `tablebg2` is `("tablebg2", None)`.
fn split_suffix(name: &str) -> (&str, Suffix) {
    match name.rsplit_once('_') {
        Some((stem, number)) => match number.parse() {
            Ok(number) => (stem, Some(number)),
            Err(_) => (name, None),
        },
        None => (name, None),
    }
}

/// A `tablebg{r}` wrapper fill: its table and 1-based row.
fn row_wrapper(name: &str) -> Option<(Suffix, usize)> {
    let (stem, suffix) = split_suffix(name);
    let row = stem.strip_prefix("tablebg")?.parse::<usize>().ok()?;
    (1..=ROWS).contains(&row).then_some((suffix, row))
}

/// One authored table's rows.
#[derive(Debug, Clone, PartialEq)]
struct Table {
    suffix: Suffix,
    /// Each row's fill top, row 1 first, in screen space.
    tops: Vec<f32>,
}

/// What an anonymous widget nested in a table is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    /// The rule above the first row: `zonetopline` and its anonymous right half.
    TopRule,
    /// Row `r`'s (1-based) tile and rules.
    Row(usize),
}

/// The geometry of every table a layout authors, read off its `tablebg` wrappers.
///
/// A wrapper is `<Image name="tablebg{r}" OffsetY="...">` with no `src`, so the screen
/// reader collects it as a fill and places it at its own `y` **without** the tag's
/// `OffsetY` (`Fill::own_offset`) - the container offset reaches the widgets nested
/// inside it and, for an image with a `src`, the image itself, but not a colour-only
/// wrapper's own rectangle. Left alone, all eight dark row backings pile up along the
/// top of the screen and no row has one; [`Self::top`] puts the offset back.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Grid {
    tables: Vec<Table>,
    /// Each wrapper's table index and row, in document order - the order the
    /// anonymous tile images follow.
    wrappers: Vec<(usize, usize)>,
}

impl Grid {
    pub(super) fn read(layout: &Layout) -> Self {
        let mut grid = Self {
            tables: Vec::new(),
            wrappers: Vec::new(),
        };
        for fill in &layout.screen.fills {
            let Some((suffix, row)) = fill.name.as_deref().and_then(row_wrapper) else {
                continue;
            };
            let index = grid
                .tables
                .iter()
                .position(|table| table.suffix == suffix)
                .unwrap_or_else(|| {
                    grid.tables.push(Table {
                        suffix,
                        tops: Vec::new(),
                    });
                    grid.tables.len() - 1
                });
            let table = &mut grid.tables[index];
            if table.tops.len() + 1 == row {
                table.tops.push(fill.y + fill.own_offset.1);
                grid.wrappers.push((index, row));
            }
        }
        grid
    }

    fn index(&self, suffix: Suffix) -> Option<usize> {
        self.tables.iter().position(|table| table.suffix == suffix)
    }

    /// The `y` a row's highlight sits at.
    ///
    /// The PSP executable steps the literal `0x5d + 0x14 * (row - 1)`
    /// (`EndRaceResults_PopulateLapTable` ends on `laps * 0x14 + 0x5d` for its totals
    /// row, the Tournament and Elimination tables step it per craft): one pixel below
    /// row 1's `tablebg` `OffsetY`, which is where every later row's own fill starts.
    /// So a row's highlight is its fill's top, and row 1's the same distance above
    /// row 2's. **Read off the PSP's file this is the literal; on the PS2's it is chosen**,
    /// not measured: its executable is not in a Ghidra project and the placement is
    /// unread. `None` for a table the grid does not have.
    #[must_use]
    pub(super) fn highlight_y(&self, suffix: Suffix, row: usize) -> Option<f32> {
        let tops = &self.tables[self.index(suffix)?].tops;
        let row = row.max(1);
        let pitch = match tops.as_slice() {
            [_, second, third, ..] => third - second,
            _ => return None,
        };
        Some(tops[1] + (row as f32 - 2.0) * pitch)
    }
}

/// The widget name a table cell carries, `lap{n}.{c}` or `lap{n}.{c}_{N}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LapCell {
    pub table: Suffix,
    /// `0` is the header row.
    pub row: usize,
    pub column: usize,
}

/// `"lap3.1"` -> row 3, column 1 of the one table; `"lap3.1_1"` the same cell of table 1.
#[must_use]
pub(super) fn lap_cell(name: &str) -> Option<LapCell> {
    let (stem, table) = split_suffix(name);
    let rest = stem.strip_prefix("lap")?;
    let (row, column) = rest.split_once('.')?;
    Some(LapCell {
        table,
        row: row.parse().ok()?,
        column: column.parse().ok()?,
    })
}

fn shown_for(shows: &[(Suffix, Shown)], suffix: Suffix) -> Option<&Shown> {
    shows
        .iter()
        .find(|(table, _)| *table == suffix)
        .map(|(_, shown)| shown)
}

/// One draw list: the backdrop, then the layout's own fills,
/// images and texts, with each table in `shows` gated by its [`Shown`] (a table the list
/// does not name is hidden whole) and every text's content asked of `cell`.
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
    shows: &[(Suffix, Shown)],
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
    let grid = Grid::read(layout);
    let part_shown = |table: usize, part: Part| {
        shown_for(shows, grid.tables[table].suffix).is_some_and(|shown| {
            shown.table
                && match part {
                    Part::TopRule => true,
                    Part::Row(row) => row <= shown.rows,
                }
        })
    };
    let mut out = Vec::new();
    // What the anonymous fills after a table's wrapper are nested in: the document
    // order is the nesting, which a `y` could not tell apart where two tables'
    // rows overlap (`Results Split` keeps the one-person table beside its two).
    let mut owner: Option<(usize, Part)> = None;
    let mut top_rules = 0;
    for authored in &screen.fills {
        let name = authored.name.as_deref();
        let (stem, suffix) = name.map_or(("", None), split_suffix);
        if stem == "tablehighlight" {
            owner = None;
            let Some((shown, row)) = grid
                .index(suffix)
                .and(shown_for(shows, suffix))
                .and_then(|shown| Some((shown, shown.highlight.filter(|_| shown.table)?)))
            else {
                continue;
            };
            let rows = std::iter::once(row).chain(
                (1..=ROWS)
                    .filter(|&other| other != row && (shown.also_highlight >> other) & 1 == 1),
            );
            for row in rows {
                let Some(y) = grid.highlight_y(suffix, row) else {
                    continue;
                };
                out.push(Draw::Fill {
                    rect: [
                        0.0,
                        y,
                        authored.width.unwrap_or(0.0),
                        authored.height.unwrap_or(0.0),
                    ],
                    color: argb_to_rgba(authored.color),
                });
            }
            continue;
        }
        if stem == "zonetopline" {
            owner = grid
                .tables
                .get(top_rules)
                .map(|_| (top_rules, Part::TopRule));
            top_rules += 1;
        } else if let Some((suffix, row)) = name.and_then(row_wrapper) {
            owner = grid.index(suffix).map(|table| (table, Part::Row(row)));
        } else if name.is_some() {
            owner = None;
        }
        if owner.is_some_and(|(table, part)| !part_shown(table, part)) {
            continue;
        }
        let placed = match name.and_then(row_wrapper) {
            Some(_) => Fill {
                y: authored.y + authored.own_offset.1,
                ..authored.clone()
            },
            None => authored.clone(),
        };
        out.push(fill_draw(&placed));
    }
    let mut tiles = 0;
    for image in &screen.images {
        let name = image.name.as_deref();
        let (stem, suffix) = name.map_or(("", None), split_suffix);
        let table_shown = |wanted: fn(&Shown) -> bool| {
            shown_for(shows, suffix).is_some_and(|shown| shown.table && wanted(shown))
        };
        let visible = match stem {
            _ if stem.starts_with("perfectlap") => false,
            "boostimg" => table_shown(|shown| shown.boost_icon),
            "topbarcenter" => table_shown(|shown| shown.header_bar),
            _ if name.is_none() && image.src.ends_with("hex_bg.mip") => {
                let owner = grid.wrappers.get(tiles).copied();
                tiles += 1;
                owner.is_none_or(|(table, row)| part_shown(table, Part::Row(row)))
            }
            _ => true,
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
        if let Some(cell) = lap_cell(name)
            && !shown_for(shows, cell.table).is_some_and(|shown| shown.table)
        {
            continue;
        }
        let Some(content) = cell(text) else { continue };
        out.push(text_draw(text, &content, layout));
    }
    layers.body = out;
    layers
}

#[cfg(test)]
mod tests;
