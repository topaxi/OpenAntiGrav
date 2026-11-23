//! Substitute button glyphs: PromptFont's, drawn where the disc's own
//! PlayStation glyph would be when the player is not holding a PlayStation pad.
//!
//! # How a string gets another controller's button
//!
//! The disc's strings embed a stand-in codepoint (`ε` in Pulse's "Press ε to
//! continue") and the face draws the PlayStation glyph for it. This module
//! adds, to every atlas that carries a stand-in, one extra cell per
//! (stand-in, substitute glyph) pair. Each extra cell is the substitute
//! scaled into **the disc glyph's own box and advance**, so swapping it in
//! cannot move a line, a centred prompt or a word-wrap. A cell lives under a
//! private-use codepoint ([`sub_char`]); [`Substitution::apply`] rewrites a
//! string's stand-ins to those codepoints just before it is drawn. Nothing
//! else about drawing text changes, and with no substitution active the text
//! is untouched, byte for byte.
//!
//! The art is PromptFont by Shinmera (SIL Open Font Licence, see
//! `licences/PromptFont-OFL.txt`), rasterised by `scripts/gen-prompt-glyphs.py`
//! into `assets/ui/prompts/`. Which glyph stands for which control on which
//! pad is the caller's table, not this module's.

use std::borrow::Cow;
use std::sync::OnceLock;

use oag_title::prompts::{Prompt, Prompts, Scope};

use crate::font::{Atlas, Cell};

/// Transparent pixels left between glyphs [`Atlas::push_glyph`] adds.
const PEN_GUTTER: u32 = 2;

/// Where [`Atlas::push_glyph`] puts the next glyph.
#[derive(Debug, Clone, Copy)]
struct Pen {
    x: u32,
    y: u32,
    row: u32,
}

/// The side of one cell in the embedded art, in pixels.
pub const ART_CELL: usize = 48;

const ART: &[u8] = include_bytes!("../../../assets/ui/prompts/promptfont-subset.a8");
const INDEX: &str = include_str!("../../../assets/ui/prompts/promptfont-subset.idx");

/// The code-names in the art, in cell order.
fn names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        INDEX
            .lines()
            .filter_map(|line| line.split_whitespace().nth(1))
            .collect()
    })
}

/// The art cell of the PromptFont glyph called `code_name`
/// (`"xbox-a"`, `"keyboard-enter"`...).
#[must_use]
pub fn art_index(code_name: &str) -> Option<usize> {
    names().iter().position(|name| *name == code_name)
}

/// The alpha of art cell `index`, [`ART_CELL`] squared bytes.
fn art(index: usize) -> &'static [u8] {
    &ART[index * ART_CELL * ART_CELL..(index + 1) * ART_CELL * ART_CELL]
}

/// The private-use codepoint the cell for (`stand_in`'th stand-in of a
/// title's table, art cell `art_index`) is stored under.
#[must_use]
pub fn sub_char(stand_in: usize, art_index: usize) -> char {
    char::from_u32(0xF_0000 + (stand_in * 64 + art_index) as u32).unwrap_or('\u{fffd}')
}

/// The ink's bounding box in an art cell: `(left, top, width, height)`.
fn ink_box(cell: &[u8]) -> (usize, usize, usize, usize) {
    let (mut left, mut top, mut right, mut bottom) = (ART_CELL, ART_CELL, 0, 0);
    for y in 0..ART_CELL {
        for x in 0..ART_CELL {
            if cell[y * ART_CELL + x] != 0 {
                left = left.min(x);
                right = right.max(x + 1);
                top = top.min(y);
                bottom = bottom.max(y + 1);
            }
        }
    }
    if right <= left {
        return (0, 0, ART_CELL, ART_CELL);
    }
    (left, top, right - left, bottom - top)
}

/// Art cell `index` scaled to fit a `width` x `height` box, centred, by area
/// averaging so a 12-pixel cross stays a cross.
fn fitted(index: usize, width: usize, height: usize) -> Vec<u8> {
    let cell = art(index);
    let (left, top, ink_w, ink_h) = ink_box(cell);
    let scale = (width as f32 / ink_w as f32).min(height as f32 / ink_h as f32);
    let out_w = ((ink_w as f32 * scale).round() as usize).clamp(1, width);
    let out_h = ((ink_h as f32 * scale).round() as usize).clamp(1, height);
    let (pad_x, pad_y) = ((width - out_w) / 2, (height - out_h) / 2);
    let mut out = vec![0u8; width * height];
    for oy in 0..out_h {
        for ox in 0..out_w {
            let (x0, x1) = (ox * ink_w / out_w, ((ox + 1) * ink_w).div_ceil(out_w));
            let (y0, y1) = (oy * ink_h / out_h, ((oy + 1) * ink_h).div_ceil(out_h));
            let (mut sum, mut count) = (0u32, 0u32);
            for y in y0..y1.max(y0 + 1) {
                for x in x0..x1.max(x0 + 1) {
                    sum += u32::from(
                        cell[(top + y.min(ink_h - 1)) * ART_CELL + left + x.min(ink_w - 1)],
                    );
                    count += 1;
                }
            }
            out[(pad_y + oy) * width + pad_x + ox] = (sum / count.max(1)) as u8;
        }
    }
    out
}

impl Atlas {
    /// Where [`Self::push_glyph`] starts: below everything already here.
    fn pen(&self) -> Pen {
        Pen {
            x: 0,
            y: self.height + PEN_GUTTER,
            row: 0,
        }
    }

    /// Appends a glyph of `alpha` (`width` x `height`, row-major) under `ch`.
    ///
    /// For glyphs this build adds beside the disc's own - see
    /// [`crate::prompt`]. Packed left to right in rows with a transparent
    /// gutter, so linear filtering cannot pull a neighbour in; the atlas grows
    /// downward, which leaves every existing cell where it was.
    fn push_glyph(
        &mut self,
        pen: &mut Pen,
        ch: char,
        (width, height): (u32, u32),
        (advance, bearing, ink): (f32, f32, f32),
        alpha: &[u8],
    ) {
        if pen.x + width > self.width {
            pen.y += pen.row + PEN_GUTTER;
            pen.x = 0;
            pen.row = 0;
        }
        let needed = pen.y + height + PEN_GUTTER;
        if needed > self.height {
            self.height = needed;
            self.coverage.resize((self.width * self.height) as usize, 0);
            self.luma.resize((self.width * self.height) as usize, 0);
        }
        for row in 0..height {
            for column in 0..width {
                let at = ((pen.y + row) * self.width + pen.x + column) as usize;
                self.coverage[at] = alpha[(row * width + column) as usize];
                // An outlined face takes opacity from this plane alone, so its mask must end
                // where the ink does; a one-pass face reads it as colour and stays solid body.
                self.luma[at] = if self.is_outlined() {
                    alpha[(row * width + column) as usize]
                } else {
                    255
                };
            }
        }
        self.glyphs.insert(
            ch,
            Cell {
                x: pen.x,
                y: pen.y,
                width,
                height,
                advance,
                bearing,
                ink,
                extend: 0,
            },
        );
        pen.x += width + PEN_GUTTER;
        pen.row = pen.row.max(height);
    }

    /// This atlas with, for every stand-in `prompts` lists that it has a
    /// glyph for, one extra cell per art glyph at the stand-in's own size.
    ///
    /// A stand-in the atlas lacks gets none, so a face that never carried the
    /// glyph is not given one.
    #[must_use]
    pub fn with_prompts(mut self, prompts: &Prompts) -> Self {
        let mut pen = self.pen();
        for (index, (stand_in, _)) in prompts.glyphs.iter().enumerate() {
            let Some(cell) = self.cell(*stand_in) else {
                continue;
            };
            for art_cell in 0..names().len().min(64) {
                let alpha = fitted(art_cell, cell.width as usize, cell.height as usize);
                self.push_glyph(
                    &mut pen,
                    sub_char(index, art_cell),
                    (cell.width, cell.height),
                    (cell.advance, cell.bearing, cell.ink),
                    &alpha,
                );
            }
        }
        self
    }
}

/// Which art glyph replaces each of a title's stand-ins right now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Substitution {
    /// `(stand-in, replacement)`, the replacement a [`sub_char`].
    pairs: Vec<(char, char)>,
    /// The control each replacement glyph depicts, by stand-in: what [`Self::button_drawn`]
    /// answers for a stand-in this substitution redraws.
    depicted: Vec<(char, Prompt)>,
    scope: Option<Scope>,
    /// The title's table, so the glyphs left as the disc draws them can be read as well.
    table: Option<Prompts>,
}

impl Substitution {
    /// No substitution: every string draws as the disc wrote it.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Nothing replaced, but `prompts` known: every glyph draws as the disc wrote it and
    /// [`Self::button_drawn`] reads it by the table.
    #[must_use]
    pub fn reading(prompts: &Prompts) -> Self {
        Self {
            scope: Some(prompts.scope),
            table: Some(*prompts),
            ..Self::default()
        }
    }

    /// Replaces stand-in number `index` of `prompts` with art cell
    /// `art_cell`, a glyph that depicts `depicts`. A prompt with no art is left as the disc
    /// draws it.
    #[must_use]
    pub fn with(
        mut self,
        prompts: &Prompts,
        index: usize,
        art_cell: Option<usize>,
        depicts: Prompt,
    ) -> Self {
        self.scope = Some(prompts.scope);
        self.table = Some(*prompts);
        if let (Some((stand_in, _)), Some(art_cell)) = (prompts.glyphs.get(index), art_cell) {
            self.pairs.push((*stand_in, sub_char(index, art_cell)));
            self.depicted.push((*stand_in, depicts));
        }
        self
    }

    /// The disc's own glyphs with the stand-ins for `a` and `b` exchanged: the circle's glyph
    /// where the triangle's was written, and the reverse. Nothing when the table lacks either
    /// ([`Prompts::UNREAD`], a title that has not read its glyphs).
    #[must_use]
    pub fn exchanging(prompts: &Prompts, a: Prompt, b: Prompt) -> Self {
        let all = |wanted: Prompt| {
            prompts
                .glyphs
                .iter()
                .filter(move |(_, prompt)| *prompt == wanted)
                .map(|&(stand_in, _)| stand_in)
                .collect::<Vec<_>>()
        };
        let (of_a, of_b) = (all(a), all(b));
        if of_a.is_empty() || of_b.is_empty() {
            return Self::none();
        }
        // The k-th stand-in of one control is drawn as the k-th of the other, in whichever face
        // it is: a title that has two styles of glyph (Pure's menus and its footers) lists them
        // in the same order, so a style keeps its own.
        let mut pairs = Vec::new();
        let mut depicted = Vec::new();
        for (from, to, shown) in [(&of_a, &of_b, b), (&of_b, &of_a, a)] {
            for (k, &stand_in) in from.iter().enumerate() {
                pairs.push((stand_in, to[k.min(to.len() - 1)]));
                depicted.push((stand_in, shown));
            }
        }
        Self {
            pairs,
            depicted,
            scope: Some(prompts.scope),
            table: Some(*prompts),
        }
    }

    /// The control the glyph drawn for `ch` depicts: the replacement's when this substitution
    /// redraws `ch`, else the disc's own. `None` for a letter, which includes a stand-in in a
    /// face the title's table does not hold glyphs in (`in_buttons_face` is whether the text is
    /// drawn in the title's buttons face, as for [`Self::apply`]).
    #[must_use]
    pub fn button_drawn(&self, ch: char, in_buttons_face: bool) -> Option<Prompt> {
        if self.scope == Some(Scope::ButtonsFace) && !in_buttons_face {
            return None;
        }
        if let Some(&(_, prompt)) = self.depicted.iter().find(|(stand_in, _)| *stand_in == ch) {
            return Some(prompt);
        }
        self.table?.prompt_of(ch)
    }

    /// Whether the title's glyphs are known, so [`Self::button_drawn`] can answer.
    #[must_use]
    pub fn reads_prompts(&self) -> bool {
        self.table.is_some_and(|table| !table.glyphs.is_empty())
    }

    /// Whether this changes nothing.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.pairs.is_empty()
    }

    /// `text` with its stand-ins swapped. `in_buttons_face` is whether the
    /// text is drawn in the title's buttons face, which a
    /// [`Scope::ButtonsFace`] table requires.
    #[must_use]
    pub fn apply<'a>(&self, text: &'a str, in_buttons_face: bool) -> Cow<'a, str> {
        if self.pairs.is_empty() || (self.scope == Some(Scope::ButtonsFace) && !in_buttons_face) {
            return Cow::Borrowed(text);
        }
        if !text
            .chars()
            .any(|c| self.pairs.iter().any(|(s, _)| *s == c))
        {
            return Cow::Borrowed(text);
        }
        Cow::Owned(
            text.chars()
                .map(|c| {
                    self.pairs
                        .iter()
                        .find(|(stand_in, _)| *stand_in == c)
                        .map_or(c, |&(_, replacement)| replacement)
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests;
