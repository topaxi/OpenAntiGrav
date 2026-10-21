//! Moving a drawn HUD into one view of a split screen.
//!
//! A title authors its HUD for the whole screen, in its own grid. A split-screen
//! race draws one HUD per view, each from that view's craft, so the draw list
//! [`super::draw_list`] builds is moved and scaled into the view's share of the
//! screen before it is painted: [`Placement`] is the move and [`Frame::placed`]
//! applies it to every draw. One list for the whole screen is then painted in a
//! single pass, which is also what keeps the HUD renderer's uniform and quad
//! buffers from being overwritten between views.
//!
//! **Which layout goes into a view is chosen, not measured.** The title's
//! whole-screen layout is scaled uniformly to fit the view (letterboxed, the
//! aspect kept). Where a title authors a split-screen layout of its own, that
//! one is the better source and takes this same placement at the identity.

use oag_ui::frontend::Draw;

use super::draw::{Frame, Span};

/// Where a view's HUD goes, in the grid the layout is authored in.
///
/// A point `g` of the layout lands at `scale * g + offset`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Uniform scale applied to every position, size and text scale.
    pub scale: f32,
    /// Added after the scale, in grid units.
    pub offset: [f32; 2],
    /// The view's share of the grid, `[x, y, width, height]`. A draw that falls
    /// wholly outside it is dropped, so what a layout puts off-screen (or beside
    /// the fitted HUD, in a wide view) cannot paint over a neighbouring view.
    pub clip: [f32; 4],
}

impl Placement {
    /// The whole screen: nothing moves and nothing is dropped.
    #[must_use]
    pub fn identity(screen: (f32, f32)) -> Self {
        Self {
            scale: 1.0,
            offset: [0.0, 0.0],
            clip: [0.0, 0.0, screen.0, screen.1],
        }
    }
}

fn moved(rect: [f32; 4], p: &Placement) -> [f32; 4] {
    [
        rect[0] * p.scale + p.offset[0],
        rect[1] * p.scale + p.offset[1],
        rect[2] * p.scale,
        rect[3] * p.scale,
    ]
}

fn overlaps(rect: [f32; 4], clip: [f32; 4]) -> bool {
    rect[0] < clip[0] + clip[2]
        && rect[0] + rect[2] > clip[0]
        && rect[1] < clip[1] + clip[3]
        && rect[1] + rect[3] > clip[1]
}

fn inside(x: f32, y: f32, clip: [f32; 4]) -> bool {
    x >= clip[0] && x < clip[0] + clip[2] && y >= clip[1] && y < clip[1] + clip[3]
}

/// `draw` moved by `p`, or `None` when it falls outside `p.clip`.
///
/// Only the draws a HUD makes are placed; a video or a backdrop is not a HUD
/// element and is dropped.
#[must_use]
pub fn place(draw: &Draw, p: &Placement) -> Option<Draw> {
    let mut draw = draw.clone();
    {
        match &mut draw {
            Draw::Fill { rect, .. }
            | Draw::GradientFill { rect, .. }
            | Draw::GradientSprite { rect, .. }
            | Draw::Sprite { rect, .. }
            | Draw::RotatedSprite { rect, .. }
            | Draw::TiledSprite { rect, .. }
            | Draw::BlendedSprite { rect, .. } => {
                *rect = moved(*rect, p);
                overlaps(*rect, p.clip).then_some(())?;
            }
            Draw::ChamferedFill { rect, chamfer, .. } => {
                *rect = moved(*rect, p);
                *chamfer = [chamfer[0] * p.scale, chamfer[1] * p.scale];
                overlaps(*rect, p.clip).then_some(())?;
            }
            Draw::Text {
                x,
                y,
                scale,
                wrap_width,
                ..
            }
            | Draw::FacedText {
                x,
                y,
                scale,
                wrap_width,
                ..
            } => {
                *x = *x * p.scale + p.offset[0];
                *y = *y * p.scale + p.offset[1];
                *scale *= p.scale;
                *wrap_width = wrap_width.map(|width| width * p.scale);
                inside(*x, *y, p.clip).then_some(())?;
            }
            Draw::Video { .. } | Draw::FuryBackdrop(_) | Draw::SceneBackdrop(_) => return None,
        }
    }
    Some(draw)
}

impl Frame {
    /// Every draw moved by `p`, those outside its clip dropped.
    #[must_use]
    pub fn placed(&self, p: &Placement) -> Frame {
        let place_all = |list: &[Draw]| list.iter().filter_map(|draw| place(draw, p)).collect();
        Frame {
            sprites: place_all(&self.sprites),
            hud_text: place_all(&self.hud_text),
            small_text: place_all(&self.small_text),
            default_text: place_all(&self.default_text),
            skin_text: place_all(&self.skin_text),
            spans: vec![Span {
                clip: p.clip,
                start: [0; 5],
            }],
            glow: self
                .glow
                .iter()
                .map(|stamp| super::boost_smear::GlowStamp {
                    rect: moved(stamp.rect, p),
                    ..*stamp
                })
                .collect(),
        }
    }

    /// Appends `other`'s draws after this frame's, list by list, and its spans
    /// with them.
    pub fn extend(&mut self, other: Frame) {
        let at = [
            self.sprites.len(),
            self.hud_text.len(),
            self.small_text.len(),
            self.default_text.len(),
            self.skin_text.len(),
        ];
        self.spans.extend(other.spans.into_iter().map(|span| Span {
            clip: span.clip,
            start: [0, 1, 2, 3, 4].map(|i| span.start[i] + at[i]),
        }));
        self.sprites.extend(other.sprites);
        self.hud_text.extend(other.hud_text);
        self.small_text.extend(other.small_text);
        self.default_text.extend(other.default_text);
        self.skin_text.extend(other.skin_text);
        self.glow.extend(other.glow);
    }
}

#[cfg(test)]
mod tests;
