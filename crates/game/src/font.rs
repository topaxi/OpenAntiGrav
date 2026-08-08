//! The text atlas: the game's own font when the disc has one, ours when it does
//! not.
//!
//! # Two sources, one shape
//!
//! [`Atlas::from_font`] builds this from a decoded [`oag_formats::fnt::Font`] -
//! the disc's real glyphs, real boxes and real advances. [`Atlas::build`] is the
//! fallback: 5x7, uppercase only, written for this project, and meant to look
//! like the approximation it is.
//!
//! Both produce **two** 8-bit planes - a coverage plane and a body/outline mask -
//! with a fully opaque patch for solid fills, and a per-character [`Cell`]
//! carrying its own size and advance, so the renderer draws either without knowing
//! which it has.
//!
//! # Two planes, because the HUD fonts need both
//!
//! An earlier revision of this comment said the atlas was coverage-only on
//! purpose: "its palette is a 16-level alpha ramp over a single RGB - white in the
//! three menu fonts, black in the two HUD ones - so the colour carries no
//! information the vertex colour does not already supply". **That was measured and
//! it is wrong for the two HUD fonts.** Distinct palette RGB values, per font:
//!
//! | Font (PSP) | Distinct RGB | Digit ink at alpha > 128 |
//! | --- | ---: | ---: |
//! | `pulse_text.fnt`, `Pulse_20.fnt` (menus) | **1** - pure white | 25 % |
//! | `PulseHud.fnt`, `small.fnt` (HUD) | **6** - 0, 1, 5, 77, 209, 255 | 64 % |
//!
//! **That table is the PSP's and does not carry over.** On the PS2 four of the
//! five bake an outline in, not two: `Pulse_14` has 10 distinct greys and
//! `Pulse_20` has 9, where their PSP counterparts have one white each. Only
//! `pulse_text` is a single pure white on both discs, and it is the one the
//! menus draw with. See `docs/formats/fnt.md`.
//!
//! The HUD fonts are **pre-outlined**: alpha is the silhouette of glyph *plus*
//! outline, and the grey level says which part is which - white body, black
//! outline. Reading alpha alone throws the distinction away and draws the whole
//! silhouette in one colour, which turns a white digit with a black edge into a
//! solid white box. That is what it did, and at 25 px the lap time was
//! unreadable.
//!
//! So [`Atlas`] carries [`Atlas::luma`] beside [`Atlas::coverage`], and the
//! renderer composites `mix(border, colour, luma)` at `coverage`. The XML supplies
//! both colours - `Color` and `BorderColor`, the latter on 54 of the HUD's
//! widgets - which is the corroboration that this is the intended model rather
//! than a plausible one.
//!
//! **The menu fonts are unaffected by construction**: their luma is a constant 255,
//! so `mix(border, colour, 1)` is `colour` whatever the border is. Same for the
//! built-in glyphs. See `docs/formats/fnt.md` and `docs/ui/hud.md`.
//!
//! # Folding, and the letter that disappeared
//!
//! Lowercase folds to uppercase, and an accented letter with no glyph of its own
//! folds again to its base letter. The order matters and the second step is the
//! one that was missing: `to_ascii_uppercase` leaves `ç` untouched, the lookup
//! then missed, and the character was **skipped entirely**, so the disc's
//! `Français` came out as `FRANAIS`. Losing a letter changes a word; losing an
//! accent only misspells it.
//!
//! The set now carries the Latin-1 accented capitals these five languages need,
//! with the base letter compressed into six rows so the diacritic has one. They
//! read slightly squat next to their neighbours, which is what a 5x7 cell costs.

use std::collections::BTreeMap;

use oag_formats::fnt;

/// Glyph width in pixels.
pub const GLYPH_WIDTH: u32 = 5;
/// Glyph height in pixels.
pub const GLYPH_HEIGHT: u32 = 7;
/// Cell size in the atlas, leaving a transparent guard band so linear filtering
/// cannot bleed one glyph into the next.
pub const CELL: u32 = 8;

/// The glyph set, as art. One string per row, `#` for ink.
///
/// Written out rather than hex-encoded so a reviewer can see what each glyph
/// looks like and fix one without a bitmap editor.
const GLYPHS: &[(char, [&str; 7])] = &[
    (
        ' ',
        [
            "     ", "     ", "     ", "     ", "     ", "     ", "     ",
        ],
    ),
    (
        'A',
        [
            "  #  ", " # # ", "#   #", "#####", "#   #", "#   #", "#   #",
        ],
    ),
    (
        'B',
        [
            "#### ", "#   #", "#   #", "#### ", "#   #", "#   #", "#### ",
        ],
    ),
    (
        'C',
        [
            " ### ", "#   #", "#    ", "#    ", "#    ", "#   #", " ### ",
        ],
    ),
    (
        'D',
        [
            "#### ", "#   #", "#   #", "#   #", "#   #", "#   #", "#### ",
        ],
    ),
    (
        'E',
        [
            "#####", "#    ", "#    ", "#### ", "#    ", "#    ", "#####",
        ],
    ),
    (
        'F',
        [
            "#####", "#    ", "#    ", "#### ", "#    ", "#    ", "#    ",
        ],
    ),
    (
        'G',
        [
            " ### ", "#   #", "#    ", "#  ##", "#   #", "#   #", " ### ",
        ],
    ),
    (
        'H',
        [
            "#   #", "#   #", "#   #", "#####", "#   #", "#   #", "#   #",
        ],
    ),
    (
        'I',
        [
            "#####", "  #  ", "  #  ", "  #  ", "  #  ", "  #  ", "#####",
        ],
    ),
    (
        'J',
        [
            "#####", "   # ", "   # ", "   # ", "   # ", "#  # ", " ##  ",
        ],
    ),
    (
        'K',
        [
            "#   #", "#  # ", "# #  ", "##   ", "# #  ", "#  # ", "#   #",
        ],
    ),
    (
        'L',
        [
            "#    ", "#    ", "#    ", "#    ", "#    ", "#    ", "#####",
        ],
    ),
    (
        'M',
        [
            "#   #", "## ##", "# # #", "#   #", "#   #", "#   #", "#   #",
        ],
    ),
    (
        'N',
        [
            "#   #", "##  #", "# # #", "#  ##", "#   #", "#   #", "#   #",
        ],
    ),
    (
        'O',
        [
            " ### ", "#   #", "#   #", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        'P',
        [
            "#### ", "#   #", "#   #", "#### ", "#    ", "#    ", "#    ",
        ],
    ),
    (
        'Q',
        [
            " ### ", "#   #", "#   #", "#   #", "# # #", "#  # ", " ## #",
        ],
    ),
    (
        'R',
        [
            "#### ", "#   #", "#   #", "#### ", "# #  ", "#  # ", "#   #",
        ],
    ),
    (
        'S',
        [
            " ####", "#    ", "#    ", " ### ", "    #", "    #", "#### ",
        ],
    ),
    (
        'T',
        [
            "#####", "  #  ", "  #  ", "  #  ", "  #  ", "  #  ", "  #  ",
        ],
    ),
    (
        'U',
        [
            "#   #", "#   #", "#   #", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        'V',
        [
            "#   #", "#   #", "#   #", "#   #", "#   #", " # # ", "  #  ",
        ],
    ),
    (
        'W',
        [
            "#   #", "#   #", "#   #", "#   #", "# # #", "## ##", "#   #",
        ],
    ),
    (
        'X',
        [
            "#   #", "#   #", " # # ", "  #  ", " # # ", "#   #", "#   #",
        ],
    ),
    (
        'Y',
        [
            "#   #", "#   #", " # # ", "  #  ", "  #  ", "  #  ", "  #  ",
        ],
    ),
    (
        'Z',
        [
            "#####", "    #", "   # ", "  #  ", " #   ", "#    ", "#####",
        ],
    ),
    (
        '0',
        [
            " ### ", "#   #", "#  ##", "# # #", "##  #", "#   #", " ### ",
        ],
    ),
    (
        '1',
        [
            "  #  ", " ##  ", "  #  ", "  #  ", "  #  ", "  #  ", "#####",
        ],
    ),
    (
        '2',
        [
            " ### ", "#   #", "    #", "   # ", "  #  ", " #   ", "#####",
        ],
    ),
    (
        '3',
        [
            "#####", "   # ", "  #  ", "   # ", "    #", "#   #", " ### ",
        ],
    ),
    (
        '4',
        [
            "   # ", "  ## ", " # # ", "#  # ", "#####", "   # ", "   # ",
        ],
    ),
    (
        '5',
        [
            "#####", "#    ", "#### ", "    #", "    #", "#   #", " ### ",
        ],
    ),
    (
        '6',
        [
            "  ## ", " #   ", "#    ", "#### ", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '7',
        [
            "#####", "    #", "   # ", "  #  ", " #   ", " #   ", " #   ",
        ],
    ),
    (
        '8',
        [
            " ### ", "#   #", "#   #", " ### ", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '9',
        [
            " ### ", "#   #", "#   #", " ####", "    #", "   # ", " ##  ",
        ],
    ),
    (
        '.',
        [
            "     ", "     ", "     ", "     ", "     ", " ##  ", " ##  ",
        ],
    ),
    (
        ',',
        [
            "     ", "     ", "     ", "     ", " ##  ", " ##  ", " #   ",
        ],
    ),
    (
        '-',
        [
            "     ", "     ", "     ", "#####", "     ", "     ", "     ",
        ],
    ),
    (
        ':',
        [
            "     ", " ##  ", " ##  ", "     ", " ##  ", " ##  ", "     ",
        ],
    ),
    (
        '!',
        [
            "  #  ", "  #  ", "  #  ", "  #  ", "  #  ", "     ", "  #  ",
        ],
    ),
    (
        '?',
        [
            " ### ", "#   #", "    #", "   # ", "  #  ", "     ", "  #  ",
        ],
    ),
    (
        '<',
        [
            "     ", "    #", "   # ", "  #  ", "   # ", "    #", "     ",
        ],
    ),
    (
        '>',
        [
            "     ", "#    ", " #   ", "  #  ", " #   ", "#    ", "     ",
        ],
    ),
    (
        '/',
        [
            "    #", "    #", "   # ", "  #  ", " #   ", "#    ", "#    ",
        ],
    ),
    (
        '(',
        [
            "  ## ", " #   ", " #   ", " #   ", " #   ", " #   ", "  ## ",
        ],
    ),
    (
        ')',
        [
            " ##  ", "   # ", "   # ", "   # ", "   # ", "   # ", " ##  ",
        ],
    ),
    (
        '\'',
        [
            "  #  ", "  #  ", "     ", "     ", "     ", "     ", "     ",
        ],
    ),
    (
        '+',
        [
            "     ", "  #  ", "  #  ", "#####", "  #  ", "  #  ", "     ",
        ],
    ),
    (
        '=',
        [
            "     ", "     ", "#####", "     ", "#####", "     ", "     ",
        ],
    ),
    (
        '*',
        [
            "     ", "#   #", " # # ", "#####", " # # ", "#   #", "     ",
        ],
    ),
    (
        '%',
        [
            "#   #", "   # ", "  #  ", "  #  ", " #   ", "#    ", "#   #",
        ],
    ),
    // Accented letters, for the language names the disc offers. A 5x7 cell has
    // no room above a full-height capital, so the base letter is compressed into
    // six rows and the diacritic takes the row it frees. That is how bitmap
    // fonts of this size have always done it, and it is why these read as
    // slightly squat next to their unaccented neighbours.
    (
        '\u{c7}', // C-cedilla
        [
            " ### ", "#   #", "#    ", "#    ", "#   #", " ### ", "  #  ",
        ],
    ),
    (
        '\u{d1}', // N-tilde
        [
            " # # ", "#   #", "##  #", "# # #", "#  ##", "#   #", "#   #",
        ],
    ),
    (
        '\u{c1}', // A-acute
        [
            "   # ", "  #  ", " # # ", "#####", "#   #", "#   #", "#   #",
        ],
    ),
    (
        '\u{c0}', // A-grave
        [
            " #   ", "  #  ", " # # ", "#####", "#   #", "#   #", "#   #",
        ],
    ),
    (
        '\u{c2}', // A-circumflex
        [
            "  #  ", " # # ", " # # ", "#####", "#   #", "#   #", "#   #",
        ],
    ),
    (
        '\u{c4}', // A-diaeresis
        [
            " # # ", "     ", " # # ", "#####", "#   #", "#   #", "#   #",
        ],
    ),
    (
        '\u{c9}', // E-acute
        [
            "   # ", "#####", "#    ", "#### ", "#    ", "#    ", "#####",
        ],
    ),
    (
        '\u{c8}', // E-grave
        [
            " #   ", "#####", "#    ", "#### ", "#    ", "#    ", "#####",
        ],
    ),
    (
        '\u{ca}', // E-circumflex
        [
            "  #  ", " # # ", "#####", "#### ", "#    ", "#    ", "#####",
        ],
    ),
    (
        '\u{cb}', // E-diaeresis
        [
            " # # ", "#####", "#    ", "#### ", "#    ", "#    ", "#####",
        ],
    ),
    (
        '\u{cd}', // I-acute
        [
            "   # ", " ### ", "  #  ", "  #  ", "  #  ", "  #  ", " ### ",
        ],
    ),
    (
        '\u{cc}', // I-grave
        [
            " #   ", " ### ", "  #  ", "  #  ", "  #  ", "  #  ", " ### ",
        ],
    ),
    (
        '\u{ce}', // I-circumflex
        [
            "  #  ", " # # ", " ### ", "  #  ", "  #  ", "  #  ", " ### ",
        ],
    ),
    (
        '\u{cf}', // I-diaeresis
        [
            " # # ", " ### ", "  #  ", "  #  ", "  #  ", "  #  ", " ### ",
        ],
    ),
    (
        '\u{d3}', // O-acute
        [
            "   # ", " ### ", "#   #", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '\u{d2}', // O-grave
        [
            " #   ", " ### ", "#   #", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '\u{d4}', // O-circumflex
        [
            "  #  ", " # # ", " ### ", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '\u{d6}', // O-diaeresis
        [
            " # # ", " ### ", "#   #", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '\u{da}', // U-acute
        [
            "   # ", "#   #", "#   #", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '\u{d9}', // U-grave
        [
            " #   ", "#   #", "#   #", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '\u{db}', // U-circumflex
        [
            "  #  ", " # # ", "#   #", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '\u{dc}', // U-diaeresis
        [
            " # # ", "     ", "#   #", "#   #", "#   #", "#   #", " ### ",
        ],
    ),
    (
        '\u{df}', // sharp s, which German uppercases to SS but the disc writes as one
        [
            " ##  ", "#  # ", "#  # ", " ##  ", "#   #", "#   #", "###  ",
        ],
    ),
];

/// Where a glyph sits in the atlas, and how much room it takes.
///
/// The size and advance travel with the cell because a real font's glyphs are
/// not all one size; the built-in set fills them in with its fixed 5x7 cell so
/// the renderer has one path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// Left edge in atlas pixels.
    pub x: u32,
    /// Top edge in atlas pixels.
    pub y: u32,
    /// Width in atlas pixels.
    pub width: u32,
    /// Height in atlas pixels.
    pub height: u32,
    /// How far the pen moves after drawing, in pixels.
    ///
    /// A real font's boxes share `v0` across an atlas row and are cut to the
    /// row's height, so the glyph's position within the line is already baked
    /// into the box and a quad drawn at the line's top edge lands correctly.
    /// There is no separate bearing to apply.
    pub advance: f32,
}

/// The glyph atlas: a coverage byte and a body/outline byte per pixel.
#[derive(Debug, Clone)]
pub struct Atlas {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Coverage, row-major from the top left.
    ///
    /// The glyph's silhouette, outline included. This is the opacity.
    pub coverage: Vec<u8>,
    /// The body/outline mask, row-major, same length as [`Self::coverage`].
    ///
    /// `255` is glyph body, `0` is outline, and the renderer mixes the text colour
    /// toward the border colour by it. A constant `255` - which is what the menu
    /// fonts and the built-in set produce - means "all body", so the border colour
    /// never shows and the result is the plain coloured glyph.
    pub luma: Vec<u8>,
    /// The fully opaque texel used to draw solid rectangles.
    pub solid: Cell,
    /// Distance between baselines in pixels, for callers laying out rows.
    pub line_height: f32,
    /// Real glyphs keyed by codepoint, empty for the built-in set.
    ///
    /// A `BTreeMap` rather than a hash map so the iteration order is stable;
    /// nothing here feeds the simulation, but determinism by default is cheaper
    /// than remembering where the exception was.
    glyphs: BTreeMap<char, Cell>,
    /// Whether this came off the disc.
    real: bool,
}

impl Atlas {
    /// Builds the atlas.
    #[must_use]
    pub fn build() -> Self {
        // One cell per glyph in a single row, then one more cell whose top-left
        // texel is opaque. Sharing one texture between text and solid fills
        // means the renderer needs exactly one pipeline.
        let cells = GLYPHS.len() as u32 + 1;
        let width = cells * CELL;
        let height = CELL;
        let mut coverage = vec![0u8; (width * height) as usize];
        // All body, no outline: the built-in glyphs are hard-edged pixel art and
        // have no outline to distinguish, so the mask is constant and the mix
        // collapses to the text colour.
        let luma = vec![0xffu8; (width * height) as usize];

        for (index, (_, rows)) in GLYPHS.iter().enumerate() {
            let left = index as u32 * CELL;
            for (row, art) in rows.iter().enumerate() {
                for (column, ink) in art.bytes().enumerate() {
                    if ink == b'#' {
                        let x = left + column as u32;
                        let y = row as u32;
                        coverage[(y * width + x) as usize] = 0xff;
                    }
                }
            }
        }

        let solid = solid_patch(&mut coverage, width, GLYPHS.len() as u32 * CELL, 0);

        Self {
            width,
            height,
            coverage,
            luma,
            solid,
            line_height: CELL as f32,
            glyphs: BTreeMap::new(),
            real: false,
        }
    }

    /// Builds the atlas from one of the disc's own fonts.
    ///
    /// The coverage plane is the atlas's palette **alpha**, one byte per pixel,
    /// with one extra row appended to hold the opaque patch solid fills sample.
    /// The `.fnt` block is exactly its own pixels with no slack, so there is
    /// nowhere in it to borrow a texel from.
    #[must_use]
    pub fn from_font(font: &fnt::Font) -> Self {
        let width = u32::from(font.width);
        let source_height = u32::from(font.height);
        // One extra row for the solid patch, and the patch is 2x2 so linear
        // filtering cannot pull a transparent neighbour into a solid fill.
        let height = source_height + 2;
        let mut coverage = vec![0u8; (width * height) as usize];
        // Body where the palette is light, outline where it is dark. The two HUD
        // fonts carry six distinct greys here; the three menu fonts carry only
        // white, so this plane is constant for them and costs nothing.
        let mut luma = vec![0u8; (width * height) as usize];

        for y in 0..source_height {
            for x in 0..width {
                let at = (y * width + x) as usize;
                coverage[at] = font.alpha_at(x as usize, y as usize);
                luma[at] = font.luma_at(x as usize, y as usize);
            }
        }
        let solid = solid_patch(&mut coverage, width, 0, source_height);
        // The solid patch is a fill, not a glyph: it must be all body, or a fill
        // drawn next to outlined text would take the border colour.
        solid_patch(&mut luma, width, 0, source_height);

        let mut glyphs = BTreeMap::new();
        for glyph in &font.glyphs {
            let Some(ch) = char::from_u32(u32::from(glyph.codepoint)) else {
                continue;
            };
            glyphs.insert(
                ch,
                Cell {
                    x: u32::from(glyph.u0),
                    y: u32::from(glyph.v0),
                    width: u32::from(glyph.width),
                    height: u32::from(glyph.height),
                    advance: f32::from(glyph.advance),
                },
            );
        }

        Self {
            width,
            height,
            coverage,
            luma,
            solid,
            #[expect(
                clippy::cast_precision_loss,
                reason = "line heights are 10 to 25 pixels"
            )]
            line_height: font.line_height as f32,
            glyphs,
            real: true,
        }
    }

    /// Whether these glyphs came off the disc rather than out of this file.
    #[must_use]
    pub fn is_real(&self) -> bool {
        self.real
    }

    /// The cell for `ch`, folding case and then accents.
    ///
    /// Two folds, in order. Case first, for the whole of Latin-1 rather than
    /// just ASCII: `to_ascii_uppercase` leaves `ç` alone, and the lookup then
    /// misses and the character vanishes. That is how `Français` came out as
    /// `FRANAIS`.
    ///
    /// Then, if the accented glyph is missing, the **base letter** stands in, so
    /// an unknown accent costs its diacritic rather than the whole letter.
    /// Dropping a letter changes a word; dropping an accent only spells it
    /// badly.
    #[must_use]
    pub fn cell(&self, ch: char) -> Option<Cell> {
        // `to_uppercase` can yield more than one char (ß becomes SS); take the
        // single-char case and leave the rest to the accent fold.
        let mut upper = ch.to_uppercase();
        let folded = match (upper.next(), upper.next()) {
            (Some(c), None) => c,
            _ => ch,
        };

        if self.real {
            // Try the character as written first: a real font carries lower
            // case and accents of its own, so folding before looking would
            // throw away glyphs the disc actually has.
            return self
                .glyphs
                .get(&ch)
                .or_else(|| self.glyphs.get(&folded))
                .or_else(|| self.glyphs.get(&base_letter(ch)))
                .or_else(|| self.glyphs.get(&base_letter(folded)))
                .copied();
        }

        let find = |c: char| GLYPHS.iter().position(|(g, _)| *g == c);
        let index = find(folded).or_else(|| find(base_letter(folded)))?;
        Some(Cell {
            x: index as u32 * CELL,
            y: 0,
            width: GLYPH_WIDTH,
            height: GLYPH_HEIGHT,
            advance: (GLYPH_WIDTH + 1) as f32,
        })
    }
}

/// The unaccented letter a Latin-1 character reduces to.
///
/// A last resort for a character the glyph set does not carry, so that a missing
/// accent does not take its letter with it.
#[must_use]
pub fn base_letter(ch: char) -> char {
    match ch {
        '\u{c0}'..='\u{c5}' => 'A',
        '\u{c7}' => 'C',
        '\u{c8}'..='\u{cb}' => 'E',
        '\u{cc}'..='\u{cf}' => 'I',
        '\u{d1}' => 'N',
        '\u{d2}'..='\u{d6}' | '\u{d8}' => 'O',
        '\u{d9}'..='\u{dc}' => 'U',
        '\u{dd}' => 'Y',
        '\u{df}' => 'S',
        other => other,
    }
}

/// Width of `text` in pixels at scale 1, counting only glyphs that exist.
#[must_use]
pub fn measure(atlas: &Atlas, text: &str) -> f32 {
    text.chars()
        .filter_map(|c| atlas.cell(c))
        .map(|cell| cell.advance)
        .sum()
}

/// Writes the opaque patch solid fills sample, and returns its cell.
///
/// Two by two rather than a single texel so that linear filtering, which a real
/// antialiased font wants, cannot pull a transparent neighbour into a fill.
fn solid_patch(coverage: &mut [u8], width: u32, x: u32, y: u32) -> Cell {
    for dy in 0..2 {
        for dx in 0..2 {
            let at = ((y + dy) * width + x + dx) as usize;
            if let Some(texel) = coverage.get_mut(at) {
                *texel = 0xff;
            }
        }
    }
    Cell {
        x,
        y,
        width: 2,
        height: 2,
        advance: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_is_five_by_seven() {
        for (ch, rows) in GLYPHS {
            for (row, art) in rows.iter().enumerate() {
                assert_eq!(
                    art.len(),
                    GLYPH_WIDTH as usize,
                    "glyph {ch:?} row {row} is {:?}",
                    art
                );
                assert!(
                    art.bytes().all(|b| b == b' ' || b == b'#'),
                    "glyph {ch:?} row {row} has something other than space and hash"
                );
            }
        }
    }

    #[test]
    fn no_glyph_is_defined_twice() {
        let mut seen: Vec<char> = GLYPHS.iter().map(|(c, _)| *c).collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), before);
    }

    #[test]
    fn the_atlas_has_a_cell_for_every_glyph_and_one_solid_texel() {
        let atlas = Atlas::build();
        assert_eq!(atlas.width, (GLYPHS.len() as u32 + 1) * CELL);
        for (ch, _) in GLYPHS {
            assert!(atlas.cell(*ch).is_some(), "no cell for {ch:?}");
        }
        let at = (atlas.solid.y * atlas.width + atlas.solid.x) as usize;
        assert_eq!(atlas.coverage[at], 0xff);
    }

    #[test]
    fn lowercase_folds_to_uppercase() {
        let atlas = Atlas::build();
        assert_eq!(atlas.cell('a'), atlas.cell('A'));
    }

    /// This test used to assert the opposite, and the opposite was the bug: a
    /// cedilla with no glyph took its `c` with it, so `Français` measured one
    /// glyph short and drew as `FRANAIS`.
    #[test]
    fn a_missing_glyph_never_costs_a_whole_letter() {
        let atlas = Atlas::build();
        assert!(atlas.cell('\u{e7}').is_some(), "c-cedilla must resolve");
        assert_eq!(
            measure(&atlas, "Francais"),
            measure(&atlas, "Français"),
            "the accented and unaccented spellings occupy the same width"
        );
    }

    /// Characters genuinely outside a Latin alphabet are still skipped. There is
    /// no sensible base letter for them and inventing a placeholder box would be
    /// worse than a gap.
    #[test]
    fn characters_outside_latin_are_still_skipped() {
        let atlas = Atlas::build();
        for ch in ['\u{3042}', '\u{4e2d}', '\u{444}'] {
            assert_eq!(atlas.cell(ch), None, "{ch:?} has no base letter");
        }
    }

    #[test]
    fn measuring_counts_advance_not_ink() {
        let atlas = Atlas::build();
        assert_eq!(measure(&atlas, ""), 0.0);
        assert_eq!(measure(&atlas, "AB"), 12.0);
    }

    #[test]
    fn a_space_is_a_real_glyph_so_it_takes_width() {
        let atlas = Atlas::build();
        assert!(atlas.cell(' ').is_some());
        assert_eq!(measure(&atlas, "A B"), 18.0);
    }
}

#[cfg(test)]
mod fold_tests {
    use super::*;

    /// The bug the user saw: a letter, not just its accent, went missing.
    #[test]
    fn accented_letters_are_never_dropped() {
        let atlas = Atlas::build();
        for name in ["Français", "Español", "Português", "Türkçe", "Íslenska"] {
            for ch in name.chars() {
                assert!(
                    atlas.cell(ch).is_some(),
                    "{name:?}: {ch:?} has no cell, so it would vanish from the screen"
                );
            }
        }
    }

    #[test]
    fn the_language_names_measure_their_full_length() {
        let atlas = Atlas::build();
        for name in ["Français", "Español", "Italiano", "Deutsch", "English"] {
            let expected = name.chars().count() as u32 * (GLYPH_WIDTH + 1);
            assert_eq!(
                measure(&atlas, name),
                expected as f32,
                "{name:?} must measure every character, or it draws off-centre"
            );
        }
    }

    #[test]
    fn case_folds_beyond_ascii() {
        let atlas = Atlas::build();
        // The accented pair share one glyph, and both resolve.
        assert_eq!(atlas.cell('ç'), atlas.cell('Ç'));
        assert_eq!(atlas.cell('ñ'), atlas.cell('Ñ'));
        assert_eq!(atlas.cell('é'), atlas.cell('É'));
    }

    /// An accent we do not draw must cost the accent, not the letter.
    #[test]
    fn an_unknown_accent_falls_back_to_its_base_letter() {
        let atlas = Atlas::build();
        // A-ring is not in the set; it must land on 'A' rather than vanish.
        assert_eq!(atlas.cell('Å'), atlas.cell('A'));
        assert_eq!(atlas.cell('å'), atlas.cell('A'));
        assert_eq!(atlas.cell('Ø'), atlas.cell('O'));
        assert_eq!(base_letter('Ç'), 'C');
        assert_eq!(base_letter('Z'), 'Z');
    }

    /// Every accented glyph must still leave its base letter recognisable.
    #[test]
    fn accented_glyphs_keep_ink_below_their_diacritic() {
        for (ch, rows) in GLYPHS {
            if (*ch as u32) < 0xc0 {
                continue;
            }
            let ink: usize = rows[2..]
                .iter()
                .map(|r| r.bytes().filter(|&b| b == b'#').count())
                .sum();
            assert!(
                ink >= 6,
                "glyph {ch:?} has only {ink} ink below its diacritic, which will not read as a letter"
            );
        }
    }

    /// Builds a `.fnt` by hand so the real-font path is tested without game
    /// data, the same way `oag-formats` tests its own decoder.
    fn synthetic_font(glyphs: &[(u16, u8, u8, u16, u16)]) -> Vec<u8> {
        const HEADER: usize = 0x30;
        const RECORD: usize = 18;
        let count = glyphs.len();
        let codepoints_at = HEADER;
        let offsets_at = codepoints_at + count * 2;
        let records_at = offsets_at + count * 4;
        let atlas_at = records_at + count * RECORD;
        let (width, height): (u16, u16) = (64, 16);

        let mut out = vec![0u8; HEADER];
        out[0] = 1;
        out[1..4].copy_from_slice(b"FNT");
        out[4..8].copy_from_slice(&(count as u32).to_le_bytes());
        out[8..12].copy_from_slice(&(codepoints_at as u32).to_le_bytes());
        out[12..16].copy_from_slice(&(offsets_at as u32).to_le_bytes());
        out[16..20].copy_from_slice(&13u32.to_le_bytes());
        out[24..28].copy_from_slice(&(atlas_at as u32).to_le_bytes());

        for g in glyphs {
            out.extend_from_slice(&g.0.to_le_bytes());
        }
        for i in 0..count {
            out.extend_from_slice(&((records_at + i * RECORD) as u32).to_le_bytes());
        }
        for &(codepoint, w, h, u0, v0) in glyphs {
            out.extend_from_slice(&codepoint.to_le_bytes());
            out.push(w);
            out.push(h);
            out.extend_from_slice(&u0.to_le_bytes());
            out.extend_from_slice(&(u0 + u16::from(w)).to_le_bytes());
            out.extend_from_slice(&v0.to_le_bytes());
            out.extend_from_slice(&(v0 + u16::from(h)).to_le_bytes());
            out.push(w + 1);
            out.extend_from_slice(&[0xff; 5]);
        }

        let texels = usize::from(width) * usize::from(height) / 2;
        let mut atlas = vec![0u8; 0x40];
        atlas[0..2].copy_from_slice(&width.to_le_bytes());
        atlas[2..4].copy_from_slice(&height.to_le_bytes());
        atlas[4] = 4;
        atlas[5] = 1;
        atlas[6] = 0; // stored linear, so the fixture needs no swizzler
        atlas[8..12].copy_from_slice(&64u32.to_le_bytes());
        atlas[12..16].copy_from_slice(&(texels as u32).to_le_bytes());
        for i in 0..16u8 {
            atlas.extend_from_slice(&[255, 255, 255, i * 17]);
        }
        // Index 15 everywhere: fully opaque, so coverage is easy to assert on.
        atlas.extend(std::iter::repeat_n(0xffu8, texels));
        out.extend_from_slice(&atlas);
        out
    }

    #[test]
    fn a_real_font_supplies_its_own_boxes_and_advances() {
        let blob = synthetic_font(&[(b'A' as u16, 6, 9, 0, 0), (b'i' as u16, 2, 9, 8, 0)]);
        let font = fnt::Font::parse(&blob).expect("parse");
        let atlas = Atlas::from_font(&font);
        assert!(atlas.is_real());
        assert!((atlas.line_height - 13.0).abs() < f32::EPSILON);

        let wide = atlas.cell('A').expect("A");
        let narrow = atlas.cell('i').expect("i");
        assert_eq!((wide.width, wide.height), (6, 9));
        assert_eq!((narrow.x, narrow.width), (8, 2));
        // Proportional, unlike the built-in set where every advance is 6.
        assert!(narrow.advance < wide.advance);
        assert!((measure(&atlas, "Ai") - (wide.advance + narrow.advance)).abs() < f32::EPSILON);
    }

    #[test]
    fn a_real_atlas_keeps_room_for_the_solid_patch() {
        let blob = synthetic_font(&[(b'A' as u16, 6, 9, 0, 0)]);
        let font = fnt::Font::parse(&blob).expect("parse");
        let atlas = Atlas::from_font(&font);
        // The `.fnt` block is exactly its own pixels, so the patch needs rows
        // that the font itself does not provide.
        assert_eq!(atlas.height, u32::from(font.height) + 2);
        for dy in 0..2 {
            for dx in 0..2 {
                let at =
                    ((atlas.solid.y + dy) * atlas.width + atlas.solid.x) as usize + dx as usize;
                assert_eq!(atlas.coverage[at], 0xff, "solid patch at +{dx},+{dy}");
            }
        }
    }

    #[test]
    fn a_real_font_prefers_its_own_lower_case_over_folding() {
        // The built-in set has no lower case and folds; a real font does, and
        // folding first would throw away glyphs the disc actually carries.
        let blob = synthetic_font(&[(b'A' as u16, 6, 9, 0, 0), (b'a' as u16, 5, 9, 16, 0)]);
        let font = fnt::Font::parse(&blob).expect("parse");
        let atlas = Atlas::from_font(&font);
        assert_eq!(atlas.cell('a').expect("a").x, 16);
        assert_eq!(atlas.cell('A').expect("A").x, 0);
    }

    #[test]
    fn a_real_font_still_folds_an_accent_it_does_not_carry() {
        // The fold that stopped `Français` becoming `FRANAIS` has to survive
        // the switch to real glyphs, because a real font can be missing a
        // character too.
        let blob = synthetic_font(&[(b'C' as u16, 6, 9, 0, 0)]);
        let font = fnt::Font::parse(&blob).expect("parse");
        let atlas = Atlas::from_font(&font);
        assert!(atlas.cell('\u{e7}').is_some(), "c-cedilla lost its letter");
        assert_eq!(atlas.cell('\u{e7}'), atlas.cell('C'));
        assert!(
            atlas.cell('\u{4e2d}').is_none(),
            "unrelated scripts still skip"
        );
    }

    #[test]
    fn the_fallback_is_still_the_fallback() {
        let atlas = Atlas::build();
        assert!(!atlas.is_real());
        let cell = atlas.cell('A').expect("A");
        assert_eq!((cell.width, cell.height), (GLYPH_WIDTH, GLYPH_HEIGHT));
    }
}
