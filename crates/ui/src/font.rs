//! The text atlas: the game's own font when the disc has one, ours when it does
//! not.
//!
//! # Two sources, one shape
//!
//! [`Atlas::from_font`] builds this from a decoded [`oag_texture::fnt::Font`] -
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

use oag_texture::fnt;

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
    /// Grid units per atlas texel: what a [`Cell`]'s pixel size means in the
    /// grid the layout is authored in. `1.0` everywhere except a source whose
    /// faces are drawn at a higher resolution than its grid - see
    /// [`Self::with_texel_scale`].
    pub texel_scale: f32,
    /// `borderExtendPixels`: atlas pixels outside a glyph's metric box that
    /// hold its baked halo. A quad draws the box grown by this on every side
    /// (see [`Self::glyph_quad`]); the pen advance is untouched. `0` for the
    /// built-in set and for a face whose language definition authors none.
    pub border_extend: u32,
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
            texel_scale: 1.0,
            border_extend: 0,
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
            texel_scale: 1.0,
            border_extend: 0,
            real: true,
        }
    }

    /// This atlas with its texels read as `scale` grid units each.
    ///
    /// Omega's faces are the same as HD's at twice the pixel size (every glyph
    /// width, height and advance of `helv`/`helvb`/`PS_BUTTONS` is 2.0x HD's
    /// at the median), while its `skin.xml` authors HD's numbers, so its text
    /// is laid out at half its texel size. `line_height` is stored in grid
    /// units, so it moves with the scale; [`measure`] and the renderer's glyph
    /// boxes read [`Self::texel_scale`]. `1.0` is an exact identity.
    #[must_use]
    pub fn with_texel_scale(mut self, scale: f32) -> Self {
        self.line_height *= scale;
        self.texel_scale *= scale;
        self
    }

    /// This atlas drawing each glyph `px` atlas pixels past its box, the
    /// `borderExtendPixels` its language definition authors. `0` is identity.
    #[must_use]
    pub fn with_border_extend(mut self, px: u32) -> Self {
        self.border_extend = px;
        self
    }

    /// The quad `cell` draws as with its pen at `(pen, y)`: `(rect, uv)`, the
    /// box grown by [`Self::border_extend`] on all four sides in both spaces.
    ///
    /// `uv` is clamped to the glyph rows of the atlas so a bottom-row glyph
    /// never samples the opaque solid patch appended below them. Where the
    /// clamp bites the rect shrinks by the same amount, so the texel size on
    /// screen stays constant.
    #[must_use]
    pub fn glyph_quad(&self, cell: &Cell, pen: f32, y: f32, scale: f32) -> ([f32; 4], [f32; 4]) {
        let unit = scale * self.texel_scale;
        let grow = self.border_extend as f32;
        let (mut left, mut top) = (cell.x as f32 - grow, cell.y as f32 - grow);
        let (mut right, mut bottom) = (
            (cell.x + cell.width) as f32 + grow,
            (cell.y + cell.height) as f32 + grow,
        );
        let (mut dl, mut dt, mut dr, mut db) = (grow, grow, grow, grow);
        let glyph_rows = self.glyph_rows() as f32;
        for (edge, lo, hi, d) in [
            (&mut left, 0.0, self.width as f32, &mut dl),
            (&mut right, 0.0, self.width as f32, &mut dr),
            (&mut top, 0.0, glyph_rows, &mut dt),
            (&mut bottom, 0.0, glyph_rows, &mut db),
        ] {
            let clamped = edge.clamp(lo, hi);
            *d -= (clamped - *edge).abs();
            *edge = clamped;
        }
        let rect = [
            pen - dl * unit,
            y - dt * unit,
            (cell.width as f32 + dl + dr) * unit,
            (cell.height as f32 + dt + db) * unit,
        ];
        (rect, [left, top, right - left, bottom - top])
    }

    /// Rows of the atlas that hold glyphs: everything above the solid patch.
    fn glyph_rows(&self) -> u32 {
        if self.real {
            self.height.saturating_sub(2)
        } else {
            self.height
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
        .map(|cell| cell.advance * atlas.texel_scale)
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
mod fold_tests;
