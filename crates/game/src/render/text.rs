//! Turning a [`Draw::Text`] into quads, including the word-wrap
//! `widthlimited="true"` needs.
//!
//! Split out of [`super`] rather than designed apart from it, under the
//! 1,000-line/1,055-line-baseline rule in `scripts/check-file-size.py`:
//! `render.rs` had two spare lines and this needed more than that. `use
//! super::*` reaches `Quad`, `MODE_ATLAS` and the rest unchanged, the same
//! seam `frontend/draw.rs` already uses for `Frontend`.
//!
//! **The wrapping itself is [`crate::loading::wrap`], not a new algorithm.**
//! `loading.rs` already measures a loading tip through the atlas and breaks it
//! at word boundaries for exactly this reason; a second copy here would be the
//! hand-transcribed-table mistake `CLAUDE.md` warns about, just for code
//! instead of data.
//!
//! [`Draw::Text`]: crate::frontend::Draw::Text

use super::*;
use crate::loading::wrap;

impl Renderer {
    /// One line of text, optionally clipped to a horizontal window (the
    /// marquee's).
    #[expect(
        clippy::too_many_arguments,
        reason = "a line of text is this many independent properties"
    )]
    pub(super) fn push_text(
        &mut self,
        x: f32,
        y: f32,
        scale: f32,
        color: [f32; 4],
        border: [f32; 4],
        align: Align,
        text: &str,
        clip: Option<(f32, f32)>, // a value marquee's window; see `crate::marquee`
    ) {
        let width = font::measure(&self.atlas, text) * scale;
        let mut pen = match align {
            Align::Left => x,
            Align::Centre => x - width / 2.0,
            Align::Right => x - width,
        };

        for ch in text.chars() {
            let Some(cell) = self.atlas.cell(ch) else {
                continue;
            };
            // Size and advance come from the cell rather than a constant: the
            // disc's fonts are proportional, and the built-in set fills the
            // same fields in with its fixed 5x7 box.
            let (w, h) = (cell.width as f32 * scale, cell.height as f32 * scale);
            let mut rect = [pen, y, w, h];
            let mut uv = [
                cell.x as f32,
                cell.y as f32,
                cell.width as f32,
                cell.height as f32,
            ];
            if let Some((left, right)) = clip {
                let (glyph_left, glyph_right) = (rect[0], rect[0] + rect[2]);
                if glyph_right <= left || glyph_left >= right {
                    pen += cell.advance * scale;
                    continue;
                }
                // `rect` and `uv` trimmed together, so a half-visible glyph
                // samples half its own ink rather than stretching the rest.
                let (cl, cr) = (glyph_left.max(left), glyph_right.min(right));
                let (from, to) = ((cl - glyph_left) / rect[2], (cr - glyph_left) / rect[2]);
                uv[0] += from * uv[2];
                uv[2] *= to - from;
                rect[0] = cl;
                rect[2] = cr - cl;
            }
            self.quads.push(Quad {
                rect,
                uv,
                color,
                border,
                mode: MODE_ATLAS,
                rotation: 0.0,
            });
            pen += cell.advance * scale;
        }
    }

    /// A [`Draw::Text`] whose `wrap_width` is set: `text` split into lines by
    /// [`crate::loading::wrap`], each drawn exactly as one call to
    /// [`Self::push_text`] would, stacked downward from `y` by the atlas's own
    /// line height at `scale` - the same quantity a real font's `.fnt` carries
    /// and menu rows already step by. No `clip`: a wrapped block is a
    /// multi-line paragraph, not a scrolling single line, so nothing has
    /// needed the two together yet.
    ///
    /// [`Draw::Text`]: crate::frontend::Draw::Text
    #[expect(
        clippy::too_many_arguments,
        reason = "a line of text is this many independent properties"
    )]
    pub(super) fn push_wrapped_text(
        &mut self,
        x: f32,
        y: f32,
        scale: f32,
        color: [f32; 4],
        border: [f32; 4],
        align: Align,
        text: &str,
        width: f32,
    ) {
        let line_height = self.atlas.line_height * scale;
        for (index, line) in wrap(&self.atlas, text, scale, width)
            .into_iter()
            .enumerate()
        {
            self.push_text(
                x,
                y + index as f32 * line_height,
                scale,
                color,
                border,
                align,
                &line,
                None,
            );
        }
    }
}
