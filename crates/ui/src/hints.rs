//! The key and button hints a screen draws, as click and touch targets.
//!
//! A hint is a button glyph followed by its word: `(O) Help`, `(X) Confirm`. The disc draws the
//! glyph as a stand-in codepoint inside a string ([`oag_title::prompts::Prompts`]), and the
//! prompt system redraws it for the pad in use ([`crate::prompt::Substitution`]). So a hint is
//! found where it is drawn: [`scan`] reads the [`Draw`] list a screen hands the renderer and
//! answers, for each hint, the rectangle its glyph and word occupy and the control the glyph
//! **as drawn** depicts. Whatever the footer shows is what a click on it presses, with no second
//! list to keep in step: a hint a page does not draw (a `Back` on a root page) is not a target, and
//! a title whose Back is the other face button (`docs/ui/back-and-help.md`) draws the exchanged
//! glyphs and so taps the exchanged button.
//!
//! What counts as a hint:
//!
//! - a face or shoulder button glyph that opens a string (after spaces), with the rest of the
//!   string as its word (`"γ Back"`); a string may carry several such pairs;
//! - a glyph that stands alone in its string, with the nearest word to its right on the same row
//!   (Pulse's `NavigationController` draws the glyph and the word as two widgets);
//! - not a glyph inside a sentence (`"Press ε to continue"`), not a d-pad glyph (it is four
//!   controls, not one), not wrapped text and not the clipped ticker line.
//!
//! **Chosen, not measured:** the row tolerance and the largest gap between a glyph and its word,
//! as multiples of the faces' line height.

use oag_core::buttons::Button;
use oag_title::prompts::Prompt;

use crate::font::Atlas;
use crate::frontend::{Align, Draw};
use crate::pointer::contains;
use crate::prompt::Substitution;

/// The row tolerance when pairing a stand-alone glyph with its word, in line heights.
const ROW_TOLERANCE: f32 = 0.5;

/// The widest gap between a stand-alone glyph and its word, in line heights.
const WORD_GAP: f32 = 3.0;

/// What pressing a hint does: the press it stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// A pad button, which the keyboard's latch delivers to the screen as the pad would.
    Button(Button),
    /// The Escape key, which no pad button stands for (the disc chooser's `ESCAPE QUIT`).
    Escape,
}

/// One hint on screen: the press it stands for and where it is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hint {
    /// The press a click on it makes: for a glyph, the control it depicts as drawn.
    pub press: Press,
    /// `[x, y, width, height]`, in the grid the draws are in.
    pub rect: [f32; 4],
}

/// The pad button a prompt glyph depicts.
#[must_use]
pub fn button_of(prompt: Prompt) -> Button {
    match prompt {
        Prompt::Cross => Button::Cross,
        Prompt::Circle => Button::Circle,
        Prompt::Square => Button::Square,
        Prompt::Triangle => Button::Triangle,
        Prompt::L => Button::L,
        Prompt::R => Button::R,
        Prompt::Up => Button::Up,
        Prompt::Down => Button::Down,
        Prompt::Left => Button::Left,
        Prompt::Right => Button::Right,
    }
}

/// The face a draw is rendered in, as far as laying a string out needs.
#[derive(Debug, Clone, Copy)]
pub struct Face<'a> {
    /// The atlas the string's glyphs come from.
    pub atlas: &'a Atlas,
    /// Whether it is the title's buttons face, the only face a `Scope::ButtonsFace` table
    /// holds glyphs in.
    pub buttons: bool,
    /// How the renderer lays this face's pens out ([`crate::font::Pens`]): a rectangle covers the
    /// ink as drawn, not the fractional advances.
    pub pens: crate::font::Pens,
}

/// One drawn line, laid out.
struct Run<'a> {
    index: usize,
    text: &'a str,
    left: f32,
    top: f32,
    height: f32,
    scale: f32,
    face: Face<'a>,
}

/// A glyph with the word that follows it in its own string, if any.
struct Segment {
    prompt: Prompt,
    left: f32,
    glyph_right: f32,
    word_right: f32,
}

/// Whether a hint can be one control: the face and shoulder buttons.
fn is_button(prompt: Prompt) -> bool {
    !matches!(
        prompt,
        Prompt::Up | Prompt::Down | Prompt::Left | Prompt::Right
    )
}

/// Every hint `list` draws.
///
/// `clip` is the index of the draw the renderer clips to a window (the ticker, a value marquee):
/// scrolling text is never a hint. `faces` resolves a draw's role to the face it is rendered in,
/// `None` for one the renderer draws nothing for.
#[must_use]
pub fn scan<'a>(
    list: &'a [Draw],
    clip: Option<usize>,
    substitution: &Substitution,
    faces: &dyn Fn(Option<&str>) -> Option<Face<'a>>,
) -> Vec<Hint> {
    let mut runs = Vec::new();
    for (index, draw) in list.iter().enumerate() {
        let (role, x, y, scale, align, text, wrap_width) = match draw {
            Draw::Text {
                x,
                y,
                scale,
                align,
                text,
                wrap_width,
                ..
            } => (None, *x, *y, *scale, *align, text, wrap_width),
            Draw::FacedText {
                role,
                x,
                y,
                scale,
                align,
                text,
                wrap_width,
                ..
            } => (Some(*role), *x, *y, *scale, *align, text, wrap_width),
            _ => continue,
        };
        if wrap_width.is_some() || clip == Some(index) {
            continue;
        }
        let Some(face) = faces(role) else { continue };
        let (width, ink_right) = placed(face, text, scale);
        let left = match align {
            Align::Left => x,
            Align::Centre if face.pens == crate::font::Pens::Snapped => {
                crate::font::snapped_centre_pen(face.atlas, text, scale, x)
            }
            Align::Centre => x - width / 2.0,
            // A snapped right-aligned line ends its last quad on `x`, as the draw does.
            Align::Right if face.pens == crate::font::Pens::Snapped => x - ink_right,
            Align::Right => x - width,
        };
        // A snapped line starts on a whole pixel, as the draw's does; a centred one stays
        // unrounded for the first glyph's step to round, as `Renderer::push_text` leaves it.
        let left = if face.pens == crate::font::Pens::Snapped && align != Align::Centre {
            left.ceil()
        } else {
            left
        };
        runs.push(Run {
            index,
            text,
            left,
            top: y,
            height: face.atlas.line_height * face.atlas.texel_scale * scale,
            scale,
            face,
        });
    }
    let mut hints: Vec<(usize, Hint)> = Vec::new();
    let mut alone = Vec::new();
    let mut words = Vec::new();
    for run in &runs {
        let segments = segments(run, substitution);
        if segments.is_empty() {
            if !run.text.trim().is_empty() {
                words.push(run);
            }
            continue;
        }
        for segment in segments.into_iter().filter(|s| is_button(s.prompt)) {
            if segment.word_right > segment.glyph_right {
                let rect = [
                    segment.left,
                    run.top,
                    segment.word_right - segment.left,
                    run.height,
                ];
                hints.push((
                    run.index,
                    Hint {
                        press: Press::Button(button_of(segment.prompt)),
                        rect,
                    },
                ));
            } else {
                alone.push((run, segment));
            }
        }
    }
    for (glyph, segment) in alone {
        let near = words
            .iter()
            .filter(|word| {
                let line = glyph.height.max(word.height);
                (word.top - glyph.top).abs() <= ROW_TOLERANCE * line
                    && word.left >= segment.glyph_right - 1.0
                    && word.left - segment.glyph_right <= WORD_GAP * line
            })
            .min_by(|a, b| a.left.total_cmp(&b.left));
        let (right, top, bottom) = match near {
            Some(word) => (
                word.left + placed(word.face, word.text.trim_end(), word.scale).1,
                glyph.top.min(word.top),
                (glyph.top + glyph.height).max(word.top + word.height),
            ),
            None => (segment.glyph_right, glyph.top, glyph.top + glyph.height),
        };
        let rect = [segment.left, top, right - segment.left, bottom - top];
        hints.push((
            glyph.index,
            Hint {
                press: Press::Button(button_of(segment.prompt)),
                rect,
            },
        ));
    }
    hints.sort_by_key(|(index, _)| *index);
    hints.into_iter().map(|(_, hint)| hint).collect()
}

/// The width of `text` in grid units at scale 1.
fn advance(atlas: &Atlas, text: &str) -> f32 {
    crate::font::measure(atlas, text)
}

/// `(pen end, ink right)` of `text` drawn at `scale` from its origin: the fractional advance sum
/// for both, [`crate::font::snapped_run`] on a snapped face, and on a bearing face the ink width
/// (the text's first ink is its origin) for both. A string is aligned by the first and a
/// rectangle over it stops at the second.
fn placed(face: Face, text: &str, scale: f32) -> (f32, f32) {
    match face.pens {
        crate::font::Pens::Snapped => {
            let run = crate::font::snapped_run(face.atlas, text, scale);
            (run.end, run.ink_right)
        }
        crate::font::Pens::Bearing => {
            let width = crate::font::ink_width(face.atlas, text, scale, true);
            (width, width)
        }
        crate::font::Pens::Advance => {
            let width = advance(face.atlas, text) * scale;
            (width, width)
        }
    }
}

/// `(ink left, ink right)` of each space-separated piece of `line` drawn on the bearing pen at
/// `scale` from the line's first ink, so a rectangle over a piece covers the ink `push_text` draws.
fn piece_ink(atlas: &Atlas, line: &str, scale: f32) -> Vec<(f32, f32)> {
    let scale = scale * atlas.texel_scale;
    let first = line.chars().find_map(|ch| atlas.cell(ch));
    let mut pen = -first.map_or(0.0, |cell| cell.bearing * scale);
    let mut spans: Vec<(f32, f32)> = vec![(f32::MAX, f32::MIN)];
    for ch in line.chars() {
        let Some(cell) = atlas.cell(ch) else { continue };
        let (ink_left, next) = crate::font::pen_step(&cell, scale, pen, true);
        if ch == ' ' {
            spans.push((f32::MAX, f32::MIN));
        } else if let Some(span) = spans.last_mut() {
            span.0 = span.0.min(ink_left);
            span.1 = span.1.max(cell.ink.mul_add(scale, ink_left));
        }
        pen = next;
    }
    spans
        .into_iter()
        .map(|(left, right)| {
            if left > right {
                (0.0, 0.0)
            } else {
                (left, right)
            }
        })
        .collect()
}

/// The glyph-and-word pairs a string is made of, when it opens with a glyph and holds nothing but
/// such pairs. A string that has text before its first glyph is a sentence and has none.
fn segments(run: &Run, substitution: &Substitution) -> Vec<Segment> {
    let atlas = run.face.atlas;
    let mut out: Vec<Segment> = Vec::new();
    let scale = atlas.texel_scale * run.scale;
    let mut pen = run.left;
    // A bearing face counts its pen from `run.left` (`origin`), the text's first ink, as
    // `Renderer::push_text` draws it; the other faces keep an absolute pen.
    let mut origin = 0.0;
    if run.face.pens == crate::font::Pens::Bearing {
        let first = run.text.chars().find_map(|ch| atlas.cell(ch));
        origin = run.left;
        pen = -first.map_or(0.0, |cell| cell.bearing * scale);
    }
    for ch in run.text.chars() {
        // The glyph's own extent: its quad on a snapped face (whole pixels from the pen), its ink
        // on a bearing face, the advance otherwise.
        let (next, left, right) =
            atlas
                .cell(ch)
                .map_or((pen, origin + pen, origin + pen), |cell| {
                    match run.face.pens {
                        crate::font::Pens::Snapped => {
                            let quad = crate::font::snapped_quad(&cell, scale, pen);
                            let next = crate::font::pen_step(&cell, scale, pen, true).1;
                            (next, quad.left, quad.left + quad.width)
                        }
                        crate::font::Pens::Bearing => {
                            let (ink_left, next) = crate::font::pen_step(&cell, scale, pen, true);
                            (next, origin + ink_left, origin + next)
                        }
                        crate::font::Pens::Advance => {
                            let next = pen + cell.advance * scale;
                            (next, pen, next)
                        }
                    }
                });
        match substitution.button_drawn(ch, run.face.buttons) {
            Some(prompt) => out.push(Segment {
                prompt,
                left,
                glyph_right: right,
                word_right: right,
            }),
            None if ch.is_whitespace() => {}
            None => match out.last_mut() {
                Some(segment) => segment.word_right = right,
                None => return Vec::new(),
            },
        }
        pen = next;
    }
    out
}

/// One entry of a legend line: words, and the press a click on them makes when they name a key
/// or a button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The press a click on the entry makes; `None` for words that name no single control (a
    /// stick, two buttons, a modifier held while another is used).
    pub press: Option<Press>,
    /// The separator drawn before the entry.
    pub lead: &'static str,
    /// The words, key name included: `CROSS (ENTER) TAKE PHOTO`.
    pub text: String,
}

impl Entry {
    /// An entry with no separator before it.
    #[must_use]
    pub fn new(press: Option<Press>, text: impl Into<String>) -> Self {
        Self {
            press,
            lead: "",
            text: text.into(),
        }
    }

    /// An entry for pad button `button`.
    #[must_use]
    pub fn button(button: Button, text: impl Into<String>) -> Self {
        Self::new(Some(Press::Button(button)), text)
    }

    /// This entry after `lead`.
    #[must_use]
    pub const fn after(mut self, lead: &'static str) -> Self {
        self.lead = lead;
        self
    }
}

/// The text a legend line draws: its entries, each after its separator.
#[must_use]
pub fn line_text(entries: &[Entry]) -> String {
    entries
        .iter()
        .flat_map(|entry| [entry.lead, entry.text.as_str()])
        .collect()
}

/// Where one legend line is drawn: a single `Draw::Text` of [`line_text`].
#[derive(Debug, Clone, Copy)]
pub struct Line<'a> {
    pub entries: &'a [Entry],
    /// The line's anchor (`Draw::Text::x`), left, middle or right by `align`.
    pub x: f32,
    /// The line's top edge.
    pub y: f32,
    pub align: Align,
    /// How tall a hint on the line is.
    pub height: f32,
}

/// The hints of legend `lines` drawn at `scale` in `atlas`: one rectangle for each entry that
/// names a press, over the words as the face lays them out.
#[must_use]
pub fn legend(atlas: &Atlas, scale: f32, lines: &[Line]) -> Vec<Hint> {
    let mut out = Vec::new();
    for line in lines {
        let width = advance(atlas, &line_text(line.entries)) * scale;
        let left = match line.align {
            Align::Left => line.x,
            Align::Centre => line.x - width / 2.0,
            Align::Right => line.x - width,
        };
        let mut before = String::new();
        for entry in line.entries {
            before.push_str(entry.lead);
            let start = advance(atlas, &before) * scale;
            before.push_str(&entry.text);
            if let Some(press) = entry.press {
                out.push(Hint {
                    press,
                    rect: [
                        left + start,
                        line.y,
                        advance(atlas, &entry.text) * scale,
                        line.height,
                    ],
                });
            }
        }
    }
    out
}

/// Where a wrapped legend is drawn: `Draw::Text { x, y, scale, wrap_width: Some(width), .. }`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wrapped {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub width: f32,
}

/// The hints of a legend drawn as one wrapped `Draw::Text` of [`line_text`]: the renderer breaks
/// it at word boundaries ([`crate::font::wrap`]), so an entry's words are found on the lines they
/// fall on, and an entry broken across two lines is a rectangle on each.
#[must_use]
pub fn legend_wrapped(
    atlas: &Atlas,
    at: Wrapped,
    entries: &[Entry],
    pens: crate::font::Pens,
) -> Vec<Hint> {
    let text = line_text(entries);
    // The entry each word belongs to, in the order `wrap` reads them.
    let owner: Vec<usize> = entries
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| {
            std::iter::repeat_n(index, entry.text.split_whitespace().count())
        })
        .collect();
    let line_height = atlas.line_height * at.scale;
    let mut out = Vec::new();
    let mut word = 0;
    let face = Face {
        atlas,
        buttons: false,
        pens,
    };
    for (row, line) in crate::font::wrap_pens(atlas, &text, at.scale, at.width, pens)
        .iter()
        .enumerate()
    {
        let mut from = None;
        let mut last = (0.0, 0.0);
        let flush = |entry: usize, left: f32, right: f32, out: &mut Vec<Hint>| {
            if let Some(press) = entries[entry].press {
                out.push(Hint {
                    press,
                    rect: [
                        at.x + left,
                        at.y + row as f32 * line_height,
                        right - left,
                        line_height,
                    ],
                });
            }
        };
        let walked = (pens == crate::font::Pens::Bearing).then(|| piece_ink(atlas, line, at.scale));
        let mut start = 0;
        for (piece_at, piece) in line.split(' ').enumerate() {
            // One pen walks the whole line, as the draw's does: a word is where the text before
            // it puts the pen, not where it would stand measured alone.
            let end = start + piece.len();
            let pen = placed(face, &line[..start], at.scale).0;
            let left = if pens == crate::font::Pens::Snapped {
                pen.ceil()
            } else {
                pen
            };
            let right = placed(face, &line[..end], at.scale).1;
            let (left, right) = walked
                .as_ref()
                .and_then(|spans| spans.get(piece_at).copied())
                .unwrap_or((left, right));
            start = end + 1;
            let entry = owner[word];
            match from {
                Some((current, _)) if current != entry => {
                    flush(current, last.0, last.1, &mut out);
                    from = Some((entry, left));
                    last = (left, right);
                }
                None => {
                    from = Some((entry, left));
                    last = (left, right);
                }
                Some(_) => last.1 = right,
            }
            word += 1;
        }
        if let Some((current, _)) = from {
            flush(current, last.0, last.1, &mut out);
        }
    }
    out
}

/// The press of the first hint under `at`.
#[must_use]
pub fn hit(hints: &[Hint], at: (f32, f32)) -> Option<Press> {
    hints
        .iter()
        .find(|hint| contains(hint.rect, at))
        .map(|hint| hint.press)
}

#[cfg(test)]
mod tests;
