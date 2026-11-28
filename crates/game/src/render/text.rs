//! Turning a [`Draw::Text`] into quads, including the word-wrap
//! `widthlimited="true"` needs.
//!
//! Split out of [`super`] rather than designed apart from it, under the
//! 1,000-line/1,055-line-baseline rule in `scripts/check-file-size.py`:
//! `render.rs` had two spare lines and this needed more than that. `use
//! super::*` reaches `Quad`, `MODE_ATLAS` and the rest unchanged, the same
//! seam `frontend/draw.rs` already uses for `Frontend`.
//!
//! **The wrapping itself is [`oag_ui::font::wrap`], not a new algorithm.**
//! `loading.rs` already measures a loading tip through the atlas and breaks it
//! at word boundaries for exactly this reason; a second copy here would be the
//! hand-transcribed-table mistake `CLAUDE.md` warns about, just for code
//! instead of data.
//!
//! [`Draw::Text`]: oag_ui::frontend::Draw::Text

use super::*;
use oag_ui::font::wrap_pens;

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

    /// The slot a [`Draw::FacedText`] in `role` is drawn from, `None` when it draws nothing: a
    /// `Buttons` draw with the third slot holding another role's face (Pulse's `Title`), where
    /// sampling that face would print the button codepoints as letters. A `Buttons` role always
    /// reads [`Self::buttons_atlas`] and never falls back; every other role falls back to the
    /// primary atlas whether or not it matches [`Self::face_role`].
    ///
    /// [`Draw::FacedText`]: oag_ui::frontend::Draw::FacedText
    pub(super) fn slot_for(&self, role: &str) -> Option<GlyphSlot> {
        let buttons = oag_ui::language::roles::BUTTONS;
        let is_buttons = role.eq_ignore_ascii_case(buttons);
        if is_buttons && !self.buttons_role.eq_ignore_ascii_case(buttons) {
            return None;
        }
        Some(
            if is_buttons
                || (self.buttons_atlas.is_some() && role.eq_ignore_ascii_case(self.buttons_role))
            {
                GlyphSlot::Buttons
            } else if self.face_atlas.is_some()
                && self
                    .face_role
                    .is_some_and(|loaded| loaded.eq_ignore_ascii_case(role))
            {
                GlyphSlot::Face
            } else {
                GlyphSlot::Primary
            },
        )
    }

    /// The hints the last cleared pass and the passes drawn over it put on screen, in the grid
    /// the draw lists are in (before [`Self::inset`] moves them). The pointer reads these a tick
    /// later, so a click lands on what the player saw. See [`oag_ui::hints`].
    #[must_use]
    pub fn hints(&self) -> &[oag_ui::hints::Hint] {
        &self.hints
    }

    /// Reads `list`'s hints into [`Self::hints`]: cleared by a pass that clears the target,
    /// added to by one that draws over it. `clip` is the index of the clipped (scrolling) line.
    pub(super) fn collect_hints(
        &mut self,
        load: &wgpu::LoadOp<wgpu::Color>,
        list: &[Draw],
        clip: Option<usize>,
    ) {
        if matches!(load, wgpu::LoadOp::Clear(_)) {
            self.hints.clear();
        }
        if !self.prompt_substitution.reads_prompts() {
            return;
        }
        let found = oag_ui::hints::scan(list, clip, &self.prompt_substitution, &|role| {
            let slot = match role {
                Some(role) => self.slot_for(role)?,
                None => GlyphSlot::Primary,
            };
            Some(oag_ui::hints::Face {
                atlas: self.atlas_for(slot)?,
                buttons: slot == GlyphSlot::Buttons
                    && self
                        .buttons_role
                        .eq_ignore_ascii_case(oag_ui::language::roles::BUTTONS),
                pens: self.hud.pens(),
            })
        });
        self.hints.extend(found);
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
        let snapped_run = self
            .hud
            .snapped_pens
            .then(|| font::snapped_run(atlas, text, scale));
        let bearing_pens = self.hud.pens() == font::Pens::Bearing;
        let width: f32 = if let Some(run) = snapped_run {
            run.end
        } else if bearing_pens {
            font::ink_width(atlas, text, scale, true)
        } else if self.hud.whole_advances {
            text.chars()
                .filter_map(|c| atlas.cell(c))
                .map(|cell| self.hud.advance(cell.advance * scale * atlas.texel_scale))
                .sum()
        } else {
            font::measure(atlas, text) * scale
        };
        let texel = atlas.texel_scale;
        // **A right-aligned HUD string (and, for `HudArt::ink_aligned_text`, a centred one) ends on
        // its last glyph's ink, not on its advance.** Read off
        // the GE lists of the Pulse grid: `0 kmh`, `100%` and `Record` sit 2, 1 and 1 px further
        // right than the advance-aligned pens, the last glyph's trailing bearing (its advance less
        // its cell width) in each, where the record time's last glyph has none and sits where it
        // always did. `docs/ui/hud.md`, "Right-aligned HUD text ends on the ink".
        let ink_end = match align {
            Align::Right => self.hud.whole_advances || self.hud.ink_aligned,
            Align::Centre => self.hud.ink_aligned,
            Align::Left => false,
        };
        let trailing = if ink_end {
            text.chars()
                .next_back()
                .and_then(|c| atlas.cell(c))
                .map_or(0.0, |cell| {
                    let advance = self.hud.advance(cell.advance * scale * texel);
                    (advance - cell.width as f32 * scale * texel).max(0.0)
                })
        } else {
            0.0
        };
        let mut pen = self.hud.snap(match align {
            Align::Left => x,
            Align::Centre => x - (width - trailing) / 2.0,
            // Pure's right-aligned text ends its last glyph's quad on `x`, not its pen
            // (`docs/ui/menus-original.md`, "Pure's right-aligned text ends on its ink").
            Align::Right if snapped_run.is_some() => x - snapped_run.map_or(0.0, |r| r.ink_right),
            Align::Right => x - width + trailing,
        });
        if self.hud.snapped_pens && align == Align::Centre {
            // Pure's centred text starts at `x` less half its own measure, left unrounded for the
            // first glyph's step to round (`oag_ui::font::snapped_centre_pen`).
            pen = self
                .hud
                .snap(font::snapped_centre_pen(atlas, text, scale, x));
        } else if self.hud.snapped_pens {
            // The glyph quads start at whole pixels from a whole pen, so a line measures the same
            // wherever it is placed (`oag_ui::font::snapped_run` counts from 0).
            pen = pen.ceil();
        }
        let y = self.hud.snap(y);
        // A bearing pen counts from the text's own origin, `origin`: its first glyph's ink starts
        // there (`font::ink_width` measures from it) and the ink edges step up to whole units from
        // there, so a string at a fractional position (a centred line, a scrolling ticker) moves as
        // one piece. At the whole positions a disc widget authors this is the absolute step.
        let mut origin = 0.0;
        if bearing_pens {
            let first = text
                .chars()
                .find_map(|c| self.atlas_for(slot).and_then(|atlas| atlas.cell(c)));
            origin = pen;
            pen = -first.map_or(0.0, |cell| cell.bearing * scale * texel);
        }

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
            let half_unit = self.hud.half_unit_glyphs && !self.hud.whole_advances;
            let bare = oag_ui::font::Cell { extend: 0, ..cell };
            // The original's pen steps up to a whole unit before each glyph is drawn
            // ([`oag_ui::font::pen_step`]): the quad sits at the snapped ink edge, not the pen.
            let (ink_left, next) = if self.hud.snapped_pens {
                font::pen_step(&cell, scale * texel, pen, true)
            } else if bearing_pens {
                let (ink_left, next) = font::pen_step(&cell, scale * texel, pen, true);
                (origin + ink_left, next)
            } else {
                (pen, pen + self.hud.advance(cell.advance * scale * texel))
            };
            let snapped = self.hud.snapped_pens;
            let (mut rect, mut uv) = atlas.glyph_quad(
                if half_unit || snapped { &bare } else { &cell },
                ink_left,
                y,
                scale,
            );
            if snapped {
                // A PSP sprite: whole pixels from the pen, the texture started half a texel early.
                let quad = font::snapped_quad(&cell, scale * texel, pen);
                (rect[0], rect[2], rect[3]) = (quad.left, quad.width, quad.height);
                // Both axes start half a texel early and span the original's `texels + 1`; a glyph
                // at the atlas edge has no texel before it and keeps its far edge.
                for (at, span) in [(0, quad.u_span), (1, quad.v_span)] {
                    let start = uv[at] - font::SNAPPED_UV_SHIFT;
                    uv[at] = start.max(0.0);
                    uv[at + 2] = start + span - uv[at];
                }
            }
            if self.hud.whole_advances || half_unit {
                // The PSP's glyph rect is the box grown half a pixel each side with the uv grown
                // half a texel to match (`225.5..249.5` for the box `226..249` in a GE list of
                // the Pulse grid, every glyph of every string), in its pixel-centre convention.
                for quad in [&mut rect, &mut uv] {
                    quad[0] -= 0.5;
                    quad[1] -= 0.5;
                    quad[2] += 1.0;
                    quad[3] += 1.0;
                }
            }
            if let Some((left, right)) = clip {
                let (glyph_left, glyph_right) = (rect[0], rect[0] + rect[2]);
                if glyph_right <= left || glyph_left >= right {
                    pen = next;
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
                warp: [0.0; 4],
            });
            pen = next;
        }
    }

    /// A [`Draw::Text`] or [`Draw::FacedText`] whose `wrap_width` is set:
    /// `text` split into lines by [`oag_ui::font::wrap`], each drawn
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
        for (index, line) in wrap_pens(atlas, text, scale, width, self.hud.pens())
            .into_iter()
            .enumerate()
        {
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

/// Takes `quads` to `quad * factor + offset`: the transform of a
/// [`oag_ui::frontend::Draw::Magnified`] text, laid out at rest.
pub(super) fn magnify_quads(quads: &mut [Quad], factor: f32, offset: (f32, f32)) {
    for quad in quads {
        let [x, y, w, h] = quad.rect;
        quad.rect = [
            x * factor + offset.0,
            y * factor + offset.1,
            w * factor,
            h * factor,
        ];
    }
}

/// Scales `quads` horizontally by `x_scale` about `anchor`: the second axis of a
/// [`oag_ui::frontend::Draw::Stretched`] text, whose own layout used the vertical scale.
pub(super) fn stretch_quads(quads: &mut [Quad], anchor: f32, x_scale: f32) {
    for quad in quads {
        quad.rect[0] = anchor + (quad.rect[0] - anchor) * x_scale;
        quad.rect[2] *= x_scale;
    }
}
