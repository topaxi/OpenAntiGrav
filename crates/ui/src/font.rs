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
use oag_title::prompts::Prompts;

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

/// The largest texel scale (texels per grid unit) a texture pack's face is used at. **Chosen**:
/// the front end lays out in 480x272, and eight texels per unit covers a 3840-wide screen, so
/// a larger picture adds memory (a 16x face is 4096 square, and its button glyphs make it
/// several times that) and no pixels anyone sees.
pub const PACK_MAX_FACTOR: u32 = 8;

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
    /// How far the ink's left edge sits right of the pen, in the units of [`Self::advance`]
    /// (the font's signed left bearing). Layout only: the quad is already cut at the ink.
    pub bearing: f32,
    /// The glyph's authored ink width, in the units of [`Self::advance`]. Not [`Self::width`]:
    /// a texture pack or a reduction changes the box, and the layout must not follow.
    pub ink: f32,
    /// How many atlas pixels past this box the quad may draw: the face's
    /// authored `borderExtendPixels`, cut to half the gap to the nearest other
    /// box (see [`Atlas::with_border_extend`]). `0` until that is called.
    pub extend: u32,
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
    pub(crate) glyphs: BTreeMap<char, Cell>,
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
    /// Whether the font bakes an outline into its atlas (`fnt::Font::is_outlined`): the two HUD
    /// fonts. Its `luma` plane then names the body and the renderer draws the text in two passes.
    outlined: bool,
}

/// The built-in set, built once: what the engine's own screens (photo mode, the viewer, the disc
/// chooser) draw in, and so what their hints are laid out in.
#[must_use]
pub fn builtin() -> &'static Atlas {
    static BUILTIN: std::sync::OnceLock<Atlas> = std::sync::OnceLock::new();
    BUILTIN.get_or_init(Atlas::build)
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
            outlined: false,
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
                    bearing: f32::from(glyph.bearing),
                    ink: f32::from(glyph.width),
                    extend: 0,
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
            outlined: font.is_outlined(),
        }
    }

    /// [`Self::from_font`] drawn from the picture a texture pack replaces the face's atlas with
    /// (`oag_texture::replacer::font`): its alpha is the coverage and its red channel the
    /// body/outline mask, as the disc's palette is.
    ///
    /// The picture is the atlas at a whole multiple `f` of its size, the same on both axes
    /// (`docs/formats/ppsspp-texture-pack.md`, "Wired"). Every cell's box moves to the picture's
    /// pixels and [`Self::texel_scale`] reads each texel as `1/f` grid units, so a glyph keeps
    /// its size and its pen advance and only gains texels; the cells' halo (`with_border_extend`)
    /// is the caller's to give in the picture's pixels, `f` times the authored one.
    ///
    /// # Errors
    ///
    /// The picture is not a whole multiple of the atlas, its two axes differ (a face has one
    /// texel scale), or it is more than a texture holds (`MAX_ATLAS_SIDE`).
    pub fn from_replaced_font(
        font: &fnt::Font,
        picture: &oag_texture::replacer::Image,
    ) -> Result<Self, String> {
        let (width, source_height) = (u32::from(font.width), u32::from(font.height));
        let whole = |picture: u32, atlas: u32| {
            (atlas > 0 && picture.is_multiple_of(atlas)).then(|| picture / atlas)
        };
        let (Some(fx), Some(fy)) = (
            whole(picture.width, width),
            whole(picture.height, source_height),
        ) else {
            return Err(format!(
                "its {}x{} picture is not a whole multiple of the {width}x{source_height} atlas",
                picture.width, picture.height
            ));
        };
        if picture.width.max(picture.height + 2) > oag_texture::replacer::MAX_ATLAS_SIDE {
            return Err(format!(
                "its {}x{} picture is more than a texture of {} a side holds",
                picture.width,
                picture.height,
                oag_texture::replacer::MAX_ATLAS_SIDE
            ));
        }
        if fx != fy {
            return Err(format!(
                "its picture is {fx}x wide and {fy}x tall, and a face has one texel scale"
            ));
        }
        let mut atlas = Self::from_font(font);
        let pixels = (picture.width * picture.height) as usize;
        let rows = (picture.width * (picture.height + 2)) as usize;
        let mut coverage = vec![0u8; rows];
        let mut luma = vec![0u8; rows];
        let mut first_ink: Option<u8> = None;
        let mut outlined = false;
        for (at, texel) in picture
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .take(pixels)
            .enumerate()
        {
            coverage[at] = texel[3];
            // A transparent texel carries no colour, as the disc's transparent palette entry
            // carries none: a pack's PNG is white there, and an outlined face's body pass would
            // fill the whole glyph box with it.
            luma[at] = if texel[3] == 0 { 0 } else { texel[0] };
            // Greys are compared under fully opaque ink only: an upscaled picture's edge texels
            // round both channels (`pulse_text.fnt` in the Pulse pack has red 254 under alpha
            // 254), which is not a second grey.
            if texel[3] == u8::MAX {
                match first_ink {
                    None => first_ink = Some(texel[0]),
                    Some(first) => outlined |= first != texel[0],
                }
            }
        }
        atlas.width = picture.width;
        atlas.height = picture.height + 2;
        atlas.solid = solid_patch(&mut coverage, picture.width, 0, picture.height);
        solid_patch(&mut luma, picture.width, 0, picture.height);
        atlas.coverage = coverage;
        atlas.luma = luma;
        atlas.outlined = outlined;
        let factor = fx as f32;
        for cell in atlas.glyphs.values_mut() {
            cell.x *= fx;
            cell.y *= fx;
            cell.width *= fx;
            cell.height *= fx;
            cell.advance *= factor;
            cell.bearing *= factor;
            cell.ink *= factor;
        }
        atlas.texel_scale /= factor;
        Ok(atlas)
    }

    /// The face `name`'s atlas drawn from the active texture pack's picture of it, with the halo
    /// `border` the pack's factor gives (`border` times the factor), or `None` to keep the
    /// disc's own.
    ///
    /// The picture is used at the largest whole fraction of its factor that is at most
    /// [`PACK_MAX_FACTOR`] and leaves the atlas, with the substitute button glyphs `prompts`
    /// will add ([`Self::with_prompts`], which the caller still calls), within
    /// [`oag_texture::replacer::MAX_ATLAS_SIDE`] a side. The 16x Pulse pack's menu faces are
    /// 4096 square, and 64 button glyphs at that scale made them 18744 tall: more than a Steam
    /// Deck's textures hold.
    ///
    /// A face whose atlas the pack does not replace, whose picture is not a whole multiple of
    /// it, which no fraction fits, or whose `.fnt` keeps its atlas elsewhere (PS2) is the
    /// disc's; a refusal is a line in `report`, `what` labelling it ("HUD font", "menu font").
    /// The race HUD and the front end both build their faces through this.
    pub fn from_pack(
        archives: &mut oag_assets::Archives,
        name: &str,
        font: &fnt::Font,
        border: u32,
        what: &str,
        prompts: Option<&Prompts>,
        report: &mut Vec<String>,
    ) -> Option<(Self, u32)> {
        let picture = oag_texture::replacer::font(&archives.read_name(name).ok()?)?;
        let full = match Self::from_replaced_font(font, &picture) {
            Ok(atlas) => atlas,
            Err(why) => {
                report.push(format!(
                    "{what} {name}: texture pack picture not used: {why}"
                ));
                return None;
            }
        };
        let factor = picture.width / u32::from(font.width);
        let side = oag_texture::replacer::MAX_ATLAS_SIDE;
        for used in (1..=factor.min(PACK_MAX_FACTOR)).rev() {
            if !factor.is_multiple_of(used) {
                continue;
            }
            let atlas = full.reduced(factor / used);
            let grown = prompts.map_or_else(|| atlas.clone(), |p| atlas.clone().with_prompts(p));
            if grown.width.max(grown.height) > side {
                continue;
            }
            let kept = if used == factor {
                format!("{factor}x")
            } else {
                format!("{used}x of the pack's {factor}x")
            };
            report.push(format!(
                "{what} {name}: {}x{} from the texture pack ({kept})",
                atlas.width, atlas.height
            ));
            return Some((atlas, border * used));
        }
        report.push(format!(
            "{what} {name}: texture pack picture not used: no fraction of its {factor}x fits a \
             {side}-pixel texture with the button glyphs"
        ));
        None
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

    /// Whether these glyphs came off the disc rather than out of this file.
    #[must_use]
    pub fn is_real(&self) -> bool {
        self.real
    }

    /// Whether the font bakes an outline into its atlas, so its `luma` plane is a body mask.
    #[must_use]
    pub fn is_outlined(&self) -> bool {
        self.outlined
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
                .or_else(|| self.glyphs.get(&base_letter(ch).to_ascii_uppercase()))
                .copied();
        }

        // A glyph pushed onto the built-in set - see `Self::push_glyph`.
        if let Some(cell) = self.glyphs.get(&ch) {
            return Some(*cell);
        }
        let find = |c: char| GLYPHS.iter().position(|(g, _)| *g == c);
        let index = find(folded)
            .or_else(|| find(base_letter(folded)))
            .or_else(|| find(base_letter(folded).to_ascii_uppercase()))?;
        Some(Cell {
            x: index as u32 * CELL,
            y: 0,
            width: GLYPH_WIDTH,
            height: GLYPH_HEIGHT,
            advance: (GLYPH_WIDTH + 1) as f32,
            bearing: 0.0,
            ink: GLYPH_WIDTH as f32,
            extend: 0,
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
        // The ordinal indicators Portuguese writes after a number (`1º`, `2ª`):
        // a face with no superscript glyph draws the plain letter, `1o`.
        '\u{ba}' => 'o',
        '\u{aa}' => 'a',
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

/// The width a text widget reports to a layout: the ink extent from the first glyph's left edge
/// to the last glyph's right edge at `scale`, in screen units.
///
/// This is the original's `Font_MeasureString`
/// (`docs/ghidra/functions/ps3-hdfury-eu/navigation-legend.md`, "What measures a word"), not
/// [`measure`]'s advance sum: a glyph's pen starts at the previous pen plus its advance less its
/// bearing, and the string ends at the last glyph's ink. `snap` rounds each glyph's pen up to a
/// whole unit, which HD's measure does after any text has drawn and its draw never
/// does, so a layout snaps and a draw does not (Pure's draw snaps too, see [`pen_step`]). A face carries no kerning pairs here. Glyphs the
/// atlas lacks are left out, as [`measure`] leaves them.
#[must_use]
pub fn ink_width(atlas: &Atlas, text: &str, scale: f32, snap: bool) -> f32 {
    let cells: Vec<Cell> = text.chars().filter_map(|c| atlas.cell(c)).collect();
    cells_ink_width(&cells, scale * atlas.texel_scale, snap)
}

/// [`ink_width`] over glyphs already looked up, `scale` in screen units per texel.
#[must_use]
pub fn cells_ink_width(cells: &[Cell], scale: f32, snap: bool) -> f32 {
    let (Some(first), Some(last)) = (cells.first(), cells.last()) else {
        return 0.0;
    };
    let mut x = -first.bearing * scale;
    for cell in &cells[..cells.len() - 1] {
        x = pen_step(cell, scale, x, snap).1;
    }
    (last.bearing + last.ink).mul_add(scale, x)
}

/// One glyph's place on a line: `(ink_left, next)`, where `x` is the pen before it and `scale` is
/// the glyph's size in screen units per texel.
///
/// The ink's left edge sits its bearing right of the pen, and the next pen is one advance less
/// that bearing past it. `snap` rounds the ink's left edge up to a whole screen unit, as the
/// original's pens do on Pure's front end (`docs/ui/menus-original.md`, "Pure's text pens step
/// up to whole pixels") and its `Font_MeasureString` does on HD.
#[must_use]
pub fn pen_step(cell: &Cell, scale: f32, x: f32, snap: bool) -> (f32, f32) {
    let mut ink_left = cell.bearing.mul_add(scale, x);
    if snap {
        ink_left = ink_left.ceil();
    }
    (
        ink_left,
        (cell.advance - cell.bearing).mul_add(scale, ink_left),
    )
}

/// Where a line of text ends when it is drawn on the snapped pen ([`pen_step`]) from a whole-unit
/// origin: the single measure every layout of Pure's front end (wrap, alignment, hit rectangles)
/// reads, so that none of them disagrees with the draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnappedRun {
    /// The pen after the last glyph (its advance past the snapped ink edge). A centred string is
    /// not placed by it ([`snapped_centre_pen`]), and a right-aligned one by [`Self::ink_right`]
    /// (`docs/ui/menus-original.md`, "Pure's right-aligned text ends on its ink").
    pub end: f32,
    /// The right edge of the last glyph's quad: where the drawn text stops.
    pub ink_right: f32,
}

/// [`SnappedRun`] of `text` at `scale` screen units per texel of `atlas` (its own `texel_scale`
/// applies). Every glyph is snapped, the last included, from the origin 0; glyphs the atlas lacks
/// are left out, as the draw leaves them. Unlike [`ink_width`] it starts at the pen, not at the
/// first glyph's ink, and so is what a draw at `x` occupies from `x`.
#[must_use]
pub fn snapped_run(atlas: &Atlas, text: &str, scale: f32) -> SnappedRun {
    let scale = scale * atlas.texel_scale;
    let mut run = SnappedRun {
        end: 0.0,
        ink_right: 0.0,
    };
    for cell in text.chars().filter_map(|c| atlas.cell(c)) {
        let quad = snapped_quad(&cell, scale, run.end);
        run.ink_right = quad.left + quad.width;
        run.end = pen_step(&cell, scale, run.end, true).1;
    }
    run
}

/// The width `text` draws at `scale` from its origin: [`snapped_run`]'s ink edge on a snapped
/// pen, [`ink_width`] on a bearing pen, the fractional advance sum otherwise.
#[must_use]
pub fn run_width(atlas: &Atlas, text: &str, scale: f32, pens: Pens) -> f32 {
    match pens {
        Pens::Snapped => snapped_run(atlas, text, scale).ink_right,
        Pens::Bearing => ink_width(atlas, text, scale, false),
        Pens::Advance => measure(atlas, text) * scale,
    }
}

/// Breaks `text` into lines no wider than `max_width` in screen units.
///
/// Measured through the atlas that will draw it rather than by character count,
/// because the disc's fonts are proportional - a line of `MISSILES` and a line
/// of `illillil` are the same length in characters and nothing like it on
/// screen. A single word longer than `max_width` is left over-long rather than
/// broken mid-word: it cannot be made to fit, and hyphenating it would invent
/// text.
#[must_use]
pub fn wrap(atlas: &Atlas, text: &str, scale: f32, max_width: f32) -> Vec<String> {
    wrap_pens(atlas, text, scale, max_width, Pens::Advance)
}

/// [`wrap`] on the pen the draw lays out ([`run_width`] of `pens`): a front end whose pens step up
/// to whole units ([`pen_step`]) or by bearing is wrapped by the width it draws at, otherwise by
/// the fractional advance sum.
#[must_use]
pub fn wrap_pens(atlas: &Atlas, text: &str, scale: f32, max_width: f32, pens: Pens) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && run_width(atlas, &candidate, scale, pens) > max_width {
            out.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
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
        bearing: 0.0,
        ink: 0.0,
        extend: 0,
    }
}

mod border;
mod centre;
mod pens;
mod quad;
mod reduce;

pub use centre::snapped_centre_pen;
pub use pens::Pens;
pub use quad::{SNAPPED_UV_SHIFT, SnappedQuad, snapped_quad};

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

    fn face() -> oag_texture::fnt::Font {
        oag_texture::fnt::Font {
            line_height: 12,
            unknown: 0,
            width: 16,
            height: 8,
            palette: vec![[0, 0, 0, 0], [200, 200, 200, 255], [40, 40, 40, 255]],
            indices: vec![0; 16 * 8],
            glyphs: vec![oag_texture::fnt::Glyph {
                codepoint: u16::from(b'A'),
                width: 4,
                height: 4,
                u0: 2,
                u1: 6,
                v0: 1,
                v1: 5,
                advance: 5,
                bearing: 0,
            }],
        }
    }

    fn picture(
        width: u32,
        height: u32,
        grey: impl Fn(u32, u32) -> u8,
    ) -> oag_texture::replacer::Image {
        oag_texture::replacer::Image {
            width,
            height,
            rgba: (0..width * height)
                .flat_map(|at| {
                    let (x, y) = (at % width, at / width);
                    [
                        grey(x, y),
                        grey(x, y),
                        grey(x, y),
                        if x == 9 && y == 5 { 255 } else { 0 },
                    ]
                })
                .collect(),
        }
    }

    #[test]
    fn a_pack_picture_keeps_every_glyph_its_size_and_gains_texels() {
        let atlas = Atlas::from_replaced_font(&face(), &picture(64, 32, |_, _| 255)).unwrap();
        assert_eq!(
            (atlas.width, atlas.height),
            (64, 32 + 2),
            "the picture and the solid rows"
        );
        assert_eq!(atlas.texel_scale, 0.25);
        assert_eq!(atlas.line_height, 12.0, "grid units, unchanged");
        let cell = atlas.cell('A').unwrap();
        assert_eq!((cell.x, cell.y, cell.width, cell.height), (8, 4, 16, 16));
        assert_eq!(cell.advance, 20.0);
        assert_eq!(
            measure(&atlas, "A"),
            5.0,
            "the pen moves as far as the disc's face does"
        );
        assert_eq!(
            atlas.coverage[5 * 64 + 9],
            255,
            "coverage is the picture's alpha"
        );
        assert_eq!(atlas.coverage[5 * 64 + 10], 0);
        assert!(
            !atlas.is_outlined(),
            "a picture of one grey has no outline to mix in"
        );
    }

    #[test]
    fn a_picture_with_two_greys_under_ink_is_outlined() {
        let two = |x: u32, _| if x < 9 { 40 } else { 255 };
        let mut picture = picture(32, 16, two);
        picture.rgba[(5 * 32 + 8) * 4 + 3] = 255;
        let atlas = Atlas::from_replaced_font(&face(), &picture).unwrap();
        assert!(atlas.is_outlined());
        assert_eq!(atlas.luma[5 * 32 + 8], 40);
    }

    #[test]
    fn a_rounded_edge_grey_is_not_an_outline_and_transparent_texels_carry_no_body() {
        let mut picture = picture(32, 16, |_, _| 255);
        let edge = (5 * 32 + 8) * 4;
        picture.rgba[edge] = 254;
        picture.rgba[edge + 3] = 254;
        let atlas = Atlas::from_replaced_font(&face(), &picture).unwrap();
        assert!(
            !atlas.is_outlined(),
            "red 254 under alpha 254 is the upscaler's rounding, not a second grey"
        );
        assert_eq!(atlas.luma[5 * 32 + 8], 254);
        assert_eq!(atlas.luma[5 * 32 + 9], 255, "opaque ink keeps its grey");
        assert_eq!(
            atlas.luma[0], 0,
            "a white texel under zero alpha is no body"
        );
    }

    #[test]
    fn a_picture_that_is_not_a_whole_multiple_is_refused() {
        let face = face();
        for (width, height) in [(33, 16), (32, 17), (32, 8 * 3)] {
            assert!(
                Atlas::from_replaced_font(&face, &picture(width, height, |_, _| 255)).is_err(),
                "{width}x{height} is not one factor on both axes"
            );
        }
        // The size is read before any pixel is, so a picture too big for a texture is refused
        // without allocating it.
        let huge = oag_texture::replacer::Image {
            width: 16 * 1024,
            height: 8 * 1024,
            rgba: Vec::new(),
        };
        assert!(Atlas::from_replaced_font(&face, &huge).is_err());
    }

    #[test]
    fn a_space_is_a_real_glyph_so_it_takes_width() {
        let atlas = Atlas::build();
        assert!(atlas.cell(' ').is_some());
        assert_eq!(measure(&atlas, "A B"), 18.0);
    }

    /// The run of a string is the pen steps of [`pen_step`] and ends at the last glyph's quad ([`snapped_quad`]).
    #[test]
    fn a_snapped_run_ends_on_the_last_glyphs_snapped_ink() {
        let atlas = Atlas::build();
        let cell = atlas.cell('A').expect("a glyph");
        let run = snapped_run(&atlas, "AAA", 0.7);
        let (mut x, mut right) = (0.0, 0.0);
        for _ in 0..3 {
            let quad = snapped_quad(&cell, 0.7, x);
            right = quad.left + quad.width;
            x = pen_step(&cell, 0.7, x, true).1;
        }
        assert_eq!((run.end, run.ink_right), (x, right));
        assert_eq!(run_width(&atlas, "AAA", 0.7, Pens::Snapped), run.ink_right);
        assert_eq!(run_width(&atlas, "AAA", 0.7, Pens::Advance), 18.0 * 0.7);
        assert_eq!(snapped_run(&atlas, "", 0.7).ink_right, 0.0);
    }

    /// Pure's `Stats` glyphs: advance 9, left bearing 1, drawn at 0.8. The pen steps 8 whole
    /// pixels a glyph from a whole-pixel origin, where the unsnapped pen steps 7.2.
    #[test]
    fn a_snapped_pen_steps_up_to_whole_pixels_and_an_unsnapped_one_does_not() {
        let cell = Cell {
            advance: 9.0,
            bearing: 1.0,
            ..Atlas::build().cell('A').expect("a glyph")
        };
        let walk = |snap: bool| {
            let mut x = 249.0;
            (0..4)
                .map(|_| {
                    let (left, next) = pen_step(&cell, 0.8, x, snap);
                    x = next;
                    left
                })
                .collect::<Vec<f32>>()
        };
        assert_eq!(walk(true), [250.0, 258.0, 266.0, 274.0]);
        let loose = walk(false);
        assert!(
            loose[1] - loose[0] < 7.3 && loose[1] - loose[0] > 7.1,
            "{loose:?}"
        );
    }
}

#[cfg(test)]
mod fold_tests;
#[cfg(test)]
mod reduce_tests;
#[cfg(test)]
mod wrap_tests;

#[cfg(test)]
mod centre_tests;
