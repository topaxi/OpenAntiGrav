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
//! [`Draw::Text`]: oag_ui::frontend::Draw::Text

use super::*;
use crate::loading::wrap;

/// Which atlas [`Renderer::push_text`]/[`Renderer::push_wrapped_text`] draw
/// a line of text from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GlyphSlot {
    /// [`Renderer::atlas`] - a plain [`Draw::Text`], or a [`Draw::FacedText`]
    /// whose `role` does not match [`Renderer::face_role`].
    Primary,
    /// [`Renderer::face_atlas`] - falls back to [`GlyphSlot::Primary`] when
    /// `None`, the same as every caller before [`GlyphSlot::Buttons`]
    /// existed. See [`Renderer::atlas_for`].
    Face,
    /// [`Renderer::buttons_atlas`] - **no fallback**. A [`Draw::FacedText`]
    /// asking for this with nothing loaded draws nothing at all, per
    /// [`Renderer::buttons_atlas`]'s own doc.
    Buttons,
}

impl Renderer {
    /// Sets which substitute glyph the title's button stand-ins draw as; an
    /// empty [`oag_ui::prompt::Substitution`] draws the disc's own.
    pub fn set_prompt_substitution(&mut self, substitution: oag_ui::prompt::Substitution) {
        self.prompt_substitution = substitution;
    }

    /// The atlas [`Self::push_text`] and [`Self::push_wrapped_text`] read
    /// glyphs from for `slot`. `None` only for [`GlyphSlot::Buttons`] with
    /// [`Self::buttons_atlas`] still unloaded - every other slot always
    /// answers `Some`, [`GlyphSlot::Face`] by falling back to
    /// [`Self::atlas`] exactly as it did when this was a plain `bool`.
    ///
    /// A short-lived borrow returned fresh on every call rather than bound
    /// once across a text-drawing loop: [`Self::quads`] needs `&mut self` in
    /// the same loop, and this way the two borrows never overlap. See
    /// `Self::atlas.cell(ch)`'s own call site below, which already worked
    /// this way before there was a second atlas to choose between.
    fn atlas_for(&self, slot: GlyphSlot) -> Option<&Atlas> {
        match slot {
            GlyphSlot::Primary => Some(&self.atlas),
            GlyphSlot::Face => Some(self.face_atlas.as_ref().unwrap_or(&self.atlas)),
            GlyphSlot::Buttons => self.buttons_atlas.as_ref(),
        }
    }

    /// One line of text, optionally clipped to a horizontal window (the
    /// marquee's).
    #[expect(
        clippy::too_many_arguments,
        reason = "a line of text is this many independent properties"
    )]
    pub(super) fn push_text(
        &mut self,
        slot: GlyphSlot,
        x: f32,
        y: f32,
        scale: f32,
        color: [f32; 4],
        border: [f32; 4],
        align: Align,
        text: &str,
        clip: Option<(f32, f32)>, // a value marquee's window; see `oag_ui_screens::marquee`
    ) {
        // [`GlyphSlot::Buttons`] with [`Self::buttons_atlas`] still `None` -
        // see [`GlyphSlot`]'s own doc: no fallback, so there is nothing
        // honest to draw. Checked before anything else measures or shapes
        // text against an atlas that is not the one asked for.
        let Some(atlas) = self.atlas_for(slot) else {
            return;
        };
        let swapped = self.prompt_substitution.apply(
            text,
            slot == GlyphSlot::Buttons
                && self
                    .buttons_role
                    .eq_ignore_ascii_case(oag_ui::language::roles::BUTTONS),
        );
        let text = swapped.as_ref();
        // `MODE_FACE_ATLAS` only when there is a real face atlas to sample -
        // never for the placeholder [`face::upload_face`] built with no role
        // loaded, which `Self::atlas_for` has already fallen back past.
        let mode = match slot {
            GlyphSlot::Primary => MODE_ATLAS,
            GlyphSlot::Face if self.face_atlas.is_some() => MODE_FACE_ATLAS,
            GlyphSlot::Face => MODE_ATLAS,
            GlyphSlot::Buttons => MODE_BUTTONS_ATLAS,
        };
        // `integer` resizes each texel to whole pixels, so the text is laid out
        // at that size and its origin put on a whole pixel; every other mode
        // leaves `scale` and the origin as given.
        let scale = self.hud.text_scale(atlas.texel_scale, scale);
        let width = font::measure(atlas, text) * scale;
        let texel = atlas.texel_scale;
        let mut pen = self.hud.snap(match align {
            Align::Left => x,
            Align::Centre => x - width / 2.0,
            Align::Right => x - width,
        });
        let y = self.hud.snap(y);

        for ch in text.chars() {
            // Re-derived rather than reusing `atlas` above: `self.quads.push`
            // below needs `&mut self` in this same loop, so the borrow has to
            // be this short-lived - see [`Self::atlas_for`]'s own doc.
            let Some(cell) = self.atlas_for(slot).and_then(|atlas| atlas.cell(ch)) else {
                continue;
            };
            // Size and advance come from the cell rather than a constant: the
            // disc's fonts are proportional, and the built-in set fills the
            // same fields in with its fixed 5x7 box. The quad itself is the
            // box grown by the face's authored `borderExtendPixels`.
            let Some(atlas) = self.atlas_for(slot) else {
                continue;
            };
            let (mut rect, mut uv) = atlas.glyph_quad(&cell, pen, y, scale);
            if let Some((left, right)) = clip {
                let (glyph_left, glyph_right) = (rect[0], rect[0] + rect[2]);
                if glyph_right <= left || glyph_left >= right {
                    pen += cell.advance * scale * texel;
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
                mode,
                rotation: 0.0,
                chamfer: [0.0, 0.0],
                tile: [0.0, 0.0],
            });
            pen += cell.advance * scale * texel;
        }
    }

    /// A [`Draw::Text`] or [`Draw::FacedText`] whose `wrap_width` is set:
    /// `text` split into lines by [`crate::loading::wrap`], each drawn
    /// exactly as one call to [`Self::push_text`] would, stacked downward
    /// from `y` by the chosen atlas's own line height at `scale` - the same
    /// quantity a real font's `.fnt` carries and menu rows already step by.
    /// `clip` is the same horizontal window [`Self::push_text`] takes, applied to every
    /// line: the track-description panel's width wipe cuts a paragraph mid-glyph.
    ///
    /// [`Draw::Text`]: oag_ui::frontend::Draw::Text
    /// [`Draw::FacedText`]: oag_ui::frontend::Draw::FacedText
    #[expect(
        clippy::too_many_arguments,
        reason = "a line of text is this many independent properties"
    )]
    pub(super) fn push_wrapped_text(
        &mut self,
        slot: GlyphSlot,
        x: f32,
        y: f32,
        scale: f32,
        color: [f32; 4],
        border: [f32; 4],
        align: Align,
        text: &str,
        width: f32,
        clip: Option<(f32, f32)>,
    ) {
        // [`GlyphSlot::Buttons`] with nothing loaded - see [`Self::push_text`]'s
        // own doc; no caller wraps buttons-atlas text today, but the guard
        // costs nothing to keep honest.
        let Some(atlas) = self.atlas_for(slot) else {
            return;
        };
        let scale = self.hud.text_scale(atlas.texel_scale, scale);
        let line_height = atlas.line_height * scale;
        for (index, line) in wrap(atlas, text, scale, width).into_iter().enumerate() {
            self.push_text(
                slot,
                x,
                y + index as f32 * line_height,
                scale,
                color,
                border,
                align,
                &line,
                clip,
            );
        }
    }
}
