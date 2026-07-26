//! A small bitmap font, written for this project.
//!
//! # Why not the game's own font
//!
//! `FE.wad` holds five `.fnt` files and their **metrics decode cleanly**: a
//! codepoint table, one 18-byte record per glyph giving its box in a texture
//! atlas, and a 4bpp palette-indexed atlas whose declared size matches to the
//! byte. All five files validate against each other. See
//! `docs/formats/fnt.md`.
//!
//! What is *not* resolved is the atlas's **pixel layout**. Read literally it is
//! noise, and none of the PSP swizzle variants tried recovers glyph shapes. That
//! is the same wall `.mip` textures hit, recorded in `docs/formats/psp-texture.md`,
//! and it is one problem, not two: resolving it fixes both.
//!
//! So the menu draws with the glyphs below until then. They are 5x7, uppercase
//! only, and mine. Lowercase is folded to uppercase and characters outside the
//! set are skipped, which means accented names render unaccented: the disc's
//! `Français` draws as `FRANCAIS`. That is a visible approximation and it is
//! meant to look like one.

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
];

/// Where a glyph sits in the atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Left edge in atlas pixels.
    pub x: u32,
    /// Top edge in atlas pixels.
    pub y: u32,
}

/// The glyph atlas: one 8-bit coverage byte per pixel.
#[derive(Debug, Clone)]
pub struct Atlas {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Coverage, row-major from the top left.
    pub coverage: Vec<u8>,
    /// The fully opaque texel used to draw solid rectangles.
    pub solid: Cell,
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

        let solid = Cell {
            x: GLYPHS.len() as u32 * CELL,
            y: 0,
        };
        coverage[(solid.y * width + solid.x) as usize] = 0xff;

        Self {
            width,
            height,
            coverage,
            solid,
        }
    }

    /// The cell for `ch`, folding case and skipping anything unknown.
    #[must_use]
    pub fn cell(&self, ch: char) -> Option<Cell> {
        let upper = ch.to_ascii_uppercase();
        let index = GLYPHS.iter().position(|(c, _)| *c == upper)?;
        Some(Cell {
            x: index as u32 * CELL,
            y: 0,
        })
    }
}

/// Width of `text` in pixels at scale 1, counting only glyphs that exist.
#[must_use]
pub fn measure(atlas: &Atlas, text: &str) -> f32 {
    let count = text.chars().filter(|&c| atlas.cell(c).is_some()).count();
    (count as u32 * (GLYPH_WIDTH + 1)) as f32
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

    #[test]
    fn unknown_characters_are_skipped_not_substituted() {
        let atlas = Atlas::build();
        assert_eq!(atlas.cell('\u{e7}'), None, "no cedilla in this font");
        // "Français" loses one glyph rather than gaining a placeholder.
        assert_eq!(
            measure(&atlas, "Francais") - measure(&atlas, "Français"),
            6.0
        );
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
