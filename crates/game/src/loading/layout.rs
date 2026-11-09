//! A loading screen drawn from its layout file.
//!
//! The file is `screen = "loading"` of the screen-layout format ([`oag_hud::loading_layout`],
//! `docs/formats/screen-layout.md`): a `grid` of the source's own pixels and the primitives
//! placed in it, in draw order. This module only turns each primitive into the [`Draw`] the
//! renderer takes, which is in the PSP's 480x272 screen space, so a grid is mapped into it by
//! a fixed divide and the player's Safe Area is applied about the grid's centre, as the
//! original applies it to this screen.
//!
//! **Nothing here knows a number of the layout.** Where a rule, a label or the bar sits, how
//! big a corner bracket is and which face a line is in are all the file's; what this module
//! owns is what a source name means (`caption`, `feature.text`, `counts`...) and which colour a
//! role is. An element whose asset, source or string the title does not resolve is skipped,
//! never replaced by a stand-in.

use oag_hud::loading_layout::{Align as At, Draw as Kind, Element, Layout};
use oag_ui::frontend::{Align, Draw};

use super::wording::{counts, percentage, step_line};
use super::{Phase, Progress, Screen, counted, elide, estimated_fraction, fraction, wrap};
use oag_display::space::Space;
use oag_ui::font::Atlas;

/// Where the grid sits in the screen's space, with the Safe Area applied.
///
/// The renderer is told the layout's own grid ([`Screen::space`]), so a draw's coordinates are
/// the file's: a grid pixel is a unit, and nothing here converts between grids.
struct Frame {
    /// The grid's own size.
    grid: [f32; 2],
    /// The Safe Area's scale: `1.0` draws the whole grid.
    safe: f32,
    /// The bracketed panels' rectangles `[x0, y0, x1, y1]`, in the grid.
    panels: Vec<[f32; 4]>,
}

impl Frame {
    fn new(layout: &Layout, safe: f32) -> Self {
        Self {
            grid: layout.grid,
            safe,
            panels: layout
                .elements
                .iter()
                .filter(|element| element.draw == Kind::Corners)
                .map(|element| element.rect)
                .collect(),
        }
    }

    /// The feature picture `at`/`size` showing `uv`, cut to the bracketed panel its top left
    /// corner is in, as `(at, size, uv)` of the kept part. `None` when nothing of it is inside
    /// the panel. `ink` is the box of the picture's texels that show against the ground, in sheet pixels.
    ///
    /// The picture is drawn at the size the original draws it, which is wider than its panel.
    /// Its ground is opaque: Fury's is the screen's own black, which shows nothing, and HD's is
    /// white, which fills the padding and covers the description beside it. Cutting to the
    /// panel is chosen, not measured: how the original treats the spill is unread. A picture
    /// whose visible texels are already inside, or that is in no panel, is returned as it came,
    /// to the bit.
    fn fit_to_panel(
        &self,
        at: [f32; 2],
        size: [f32; 2],
        uv: [f32; 4],
        ink: Option<[f32; 4]>,
    ) -> Option<([f32; 2], [f32; 2], [f32; 4])> {
        let whole = Some((at, size, uv));
        let Some([px0, py0, px1, py1]) = self.panels.iter().copied().find(|panel| {
            panel[0] <= at[0] && at[0] < panel[2] && panel[1] <= at[1] && at[1] < panel[3]
        }) else {
            return whole;
        };
        let (x1, y1) = (at[0] + size[0], at[1] + size[1]);
        let Some(ink) = ink else {
            return whole;
        };
        let (gx, gy) = (size[0] / uv[2], size[1] / uv[3]);
        let seen = [
            at[0] + (ink[0].max(uv[0]) - uv[0]) * gx,
            at[1] + (ink[1].max(uv[1]) - uv[1]) * gy,
            at[0] + ((ink[0] + ink[2]).min(uv[0] + uv[2]) - uv[0]) * gx,
            at[1] + ((ink[1] + ink[3]).min(uv[1] + uv[3]) - uv[1]) * gy,
        ];
        if seen[0] >= px0 && seen[1] >= py0 && seen[2] <= px1 && seen[3] <= py1 {
            return whole;
        }
        let (cx0, cy0) = (at[0].max(px0), at[1].max(py0));
        let (cx1, cy1) = (x1.min(px1), y1.min(py1));
        if (cx0, cy0, cx1, cy1) == (at[0], at[1], x1, y1) {
            return whole;
        }
        if cx1 <= cx0 || cy1 <= cy0 {
            return None;
        }
        // Texels per grid unit stay what they were, so the kept part is the same pixels.
        let (tx, ty) = (uv[2] / size[0], uv[3] / size[1]);
        Some((
            [cx0, cy0],
            [cx1 - cx0, cy1 - cy0],
            [
                uv[0] + (cx0 - at[0]) * tx,
                uv[1] + (cy0 - at[1]) * ty,
                (cx1 - cx0) * tx,
                (cy1 - cy0) * ty,
            ],
        ))
    }

    /// A grid point in screen space.
    fn point(&self, x: f32, y: f32) -> [f32; 2] {
        [
            self.grid[0] / 2.0 + (x - self.grid[0] / 2.0) * self.safe,
            self.grid[1] / 2.0 + (y - self.grid[1] / 2.0) * self.safe,
        ]
    }

    /// A grid rectangle in screen space.
    fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> [f32; 4] {
        let [sx, sy] = self.point(x, y);
        [sx, sy, w * self.safe, h * self.safe]
    }

    /// A text scale: `1.0` is the face at its own pixels.
    fn text_scale(&self, scale: f32) -> f32 {
        scale * self.safe
    }

    /// A length in the grid, under the Safe Area.
    fn along(&self, length: f32) -> f32 {
        length * self.safe
    }
}

/// `[x, y, w, h]` of `uv` (a sheet rectangle in sheet pixels) cut to the element's
/// fractions `[u0, v0, u1, v1]`.
fn cut(uv: [f32; 4], fractions: Option<[f32; 4]>) -> [f32; 4] {
    match fractions {
        None => uv,
        Some([u0, v0, u1, v1]) => [
            uv[0] + uv[2] * u0,
            uv[1] + uv[3] * v0,
            uv[2] * (u1 - u0),
            uv[3] * (v1 - v0),
        ],
    }
}

/// `wrap`, with the width test the original's wrapped lines fit: a word stays on the line while
/// the line **and the space after it** are inside the width.
///
/// Read off the RPCS3 frame of the Barrel Roll paragraph, not off the code: with the wrap width
/// 780 from x 975, "with" would end at 1750 of the 1755 edge and the original breaks before it,
/// while "cost" ends at 1738 and stays. A trailing space (about 10 px at this face) separates the
/// two. Two lines of one frame, so inferred, not measured.
fn wrap_spaced(atlas: &Atlas, text: &str, scale: f32, width: f32) -> Vec<String> {
    wrap(
        atlas,
        text,
        scale,
        width - oag_ui::font::measure(atlas, " ") * scale,
    )
}

/// The sheet rectangle of a stretched sprite, drawn inside its texels: a texture drawn many
/// times its size is clamped at its own edge by the original, where this sheet's neighbours
/// (a gutter of transparent texels) would fade the end of a long rule. Half a texel in on an
/// axis it is stretched along; a sprite drawn 1:1 is left alone.
fn clamped(uv: [f32; 4], drawn: [f32; 2]) -> [f32; 4] {
    let mut uv = uv;
    if drawn[0] > uv[2] * 1.5 && uv[2] > 1.0 {
        uv[0] += 0.5;
        uv[2] -= 1.0;
    }
    if drawn[1] > uv[3] * 1.5 && uv[3] > 1.0 {
        uv[1] += 0.5;
        uv[3] -= 1.0;
    }
    uv
}

/// A text face: HD's `Title` role has its own atlas, every other role is the primary face.
fn face(role: Option<&str>) -> Option<&'static str> {
    role.filter(|role| role.eq_ignore_ascii_case("title"))
        .map(|_| "Title")
}

impl Screen {
    /// The layout that answers this race, when the title ships a file.
    fn layout(&self) -> Option<&Layout> {
        let name = self.mode_name.unwrap_or("single_race");
        self.layouts.as_ref()?.for_race(name, self.surface_aspect)
    }

    /// The grid the screen's draws are in, which the renderer is told: the layout's own where
    /// the screen is drawn from one (HD's 1920x1080, shown as itself), the PSP's otherwise.
    #[must_use]
    pub fn space(&self) -> Space {
        match self.layout().filter(|_| self.is_laid_out()) {
            Some(layout) => Space {
                size: (layout.grid[0], layout.grid[1]),
                display_aspect: layout.grid[0] / layout.grid[1],
            },
            None => Space::PSP,
        }
    }

    /// Whether this screen is drawn from a layout file: the title has art for one.
    pub(super) fn is_laid_out(&self) -> bool {
        self.illustration.is_some() || self.layouts.is_some()
    }

    /// The art a role names: `None` when the title has none, and the element is skipped.
    fn art(&self, role: &str) -> Option<[f32; 4]> {
        match role {
            "rule" => self.rule,
            "corner" => self.corner,
            "dot" => self.dot,
            "square" => self.square,
            "title-arrow" => self.title_arrow,
            "subtitle-arrow" => self.subtitle_arrow,
            "@feature" => self.illustration,
            "@mode-icon" => self.mode_still,
            _ => None,
        }
    }

    /// The colour a role names, faded.
    fn role_colour(&self, role: Option<&str>, fade: f32) -> [f32; 4] {
        let palette = self.palette;
        let [r, g, b, a] = match role.unwrap_or("ink") {
            "accent" => palette.map_or(super::BAR_FILL, |p| p.accent),
            "trough" => palette.map_or(super::BAR_DOTS, |p| p.dim),
            "ground" => palette.map_or(super::BACKDROP, |p| p.background),
            _ => palette.map_or(super::ON_BACKDROP, |p| p.ink),
        };
        [r, g, b, a * fade]
    }

    /// What a source name stands for this frame, or `None` when it has nothing to say.
    fn source_text(&self, source: &str, phase: Phase, progress: &Progress) -> Option<String> {
        match source {
            "caption" => Some(self.heading_text(phase, progress)),
            "feature.title" => self.feature_title.clone(),
            "feature.heading" => self.feature_heading.clone(),
            "feature.text" => self.tip().map(str::to_string),
            "counts" => counted(progress).then(|| counts(phase, progress, &self.strings)),
            "percentage" => counted(progress).then(|| percentage(phase, progress)),
            "current" => progress
                .current
                .clone()
                .filter(|_| !self.caption_leads(progress)),
            "step" => step_line(phase, &self.strings),
            _ => None,
        }
    }

    /// The fraction of the bar the screen shows.
    fn bar_fraction(&self, phase: Phase, progress: &Progress) -> f32 {
        if counted(progress) {
            fraction(phase, progress)
        } else if let (Phase::Race, Some(fill)) = (phase, &self.fill) {
            // The whole bar once the work is over is this build's own (chosen, not measured);
            // the steps before it are the loader's.
            if self.fade > 0 { 1.0 } else { fill.fraction() }
        } else if matches!(phase, Phase::Race) {
            estimated_fraction(self.frames, self.fade > 0)
        } else {
            0.0
        }
    }

    /// The whole screen, from the layout file; the ground alone when the title names none
    /// that loaded (said once, at load, in the loader report).
    pub(super) fn laid_out_list(
        &self,
        phase: Phase,
        progress: &Progress,
        atlas: &Atlas,
    ) -> Vec<Draw> {
        let fade = self.opacity();
        let space = self.space().size;
        let mut out = vec![Draw::Fill {
            rect: [0.0, 0.0, space.0, space.1],
            color: self.role_colour(Some("ground"), fade),
        }];
        let Some(layout) = self.layout() else {
            return out;
        };
        let frame = Frame::new(layout, self.safe_area);
        for element in &layout.elements {
            self.element(element, &frame, phase, progress, atlas, fade, &mut out);
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn element(
        &self,
        element: &Element,
        frame: &Frame,
        phase: Phase,
        progress: &Progress,
        atlas: &Atlas,
        fade: f32,
        out: &mut Vec<Draw>,
    ) {
        let colour = self.role_colour(element.colour.as_deref(), fade);
        let [x, y] = element.at;
        match element.draw {
            Kind::Image => {
                let Some(uv) = element.asset.as_deref().and_then(|role| self.art(role)) else {
                    return;
                };
                let feature = element.asset.as_deref() == Some("@feature");
                let (at, size, uv) = if feature {
                    let Some(fitted) = frame.fit_to_panel(
                        element.at,
                        element.size,
                        cut(uv, element.uv),
                        self.illustration_ink,
                    ) else {
                        return;
                    };
                    fitted
                } else {
                    (element.at, element.size, cut(uv, element.uv))
                };
                out.push(Draw::Sprite {
                    rect: frame.rect(at[0], at[1], size[0], size[1]),
                    uv: clamped(uv, [size[0] * frame.safe, size[1] * frame.safe]),
                    color: if feature {
                        [1.0, 1.0, 1.0, fade]
                    } else {
                        colour
                    },
                });
            }
            Kind::Text => {
                let text = match (&element.source, &element.text) {
                    (Some(source), _) => self.source_text(source, phase, progress),
                    (None, Some(text)) => Some(text.clone()),
                    (None, None) => None,
                };
                let Some(text) = text else { return };
                self.put_text(element, frame, atlas, text, colour, out);
            }
            Kind::Label => {
                let text = match (&element.string_id, &element.text) {
                    (Some(id), _) => self
                        .layout_strings
                        .iter()
                        .find(|(known, _)| known == id)
                        .map(|(_, text)| text.clone()),
                    (None, Some(text)) => Some(text.clone()),
                    (None, None) => None,
                };
                let Some(text) = text else { return };
                if let Some(uv) = element.asset.as_deref().and_then(|role| self.art(role)) {
                    out.push(Draw::Sprite {
                        rect: frame.rect(x, y, element.size[0], element.size[1]),
                        uv,
                        color: colour,
                    });
                }
                let mut run = element.clone();
                run.at = [x + element.gap, y];
                run.align = At::Left;
                run.wrap = None;
                self.put_text(&run, frame, atlas, text, colour, out);
            }
            Kind::Corners => {
                let Some(uv) = element.asset.as_deref().and_then(|role| self.art(role)) else {
                    return;
                };
                let [x0, y0, x1, y1] = element.rect;
                let [w, h] = element.size;
                for (cx, cy, flip) in [
                    (x0, y0, [false, false]),
                    (x1, y0, [true, false]),
                    (x0, y1, [false, true]),
                    (x1, y1, [true, true]),
                ] {
                    let at = [
                        cx - if flip[0] { element.back } else { 0.0 },
                        cy - if flip[1] { element.back } else { 0.0 },
                    ];
                    // The bracket texture is a `L`: its natural orientation is the bottom left
                    // corner. The original mirrors it for a bracket on the right edge (the
                    // flag it passes) and for one on the **top** edge (the flag clear), read off
                    // an RPCS3 frame where the top left bracket is a `Γ`. A negative uv extent
                    // draws a sprite mirrored.
                    let mut rect = frame.rect(at[0], at[1], w, h);
                    let mut uv = uv;
                    if flip[0] {
                        uv[0] += uv[2];
                        uv[2] = -uv[2];
                    }
                    if !flip[1] {
                        uv[1] += uv[3];
                        uv[3] = -uv[3];
                    }
                    rect[2] = rect[2].abs();
                    rect[3] = rect[3].abs();
                    out.push(Draw::Sprite {
                        rect,
                        uv,
                        color: colour,
                    });
                }
            }
            Kind::Dots => {
                let Some(uv) = element.asset.as_deref().and_then(|role| self.art(role)) else {
                    return;
                };
                let columns = element.columns as f32;
                let rows = element.rows as f32;
                let lit = if element.fill.as_deref() == Some("progress") {
                    (self.bar_fraction(phase, progress).clamp(0.0, 1.0) * columns).floor()
                } else {
                    0.0
                };
                let unlit = self.role_colour(element.unlit.as_deref(), fade);
                for (first, count, color) in [(0.0, lit, colour), (lit, columns - lit, unlit)] {
                    if count > 0.0 {
                        out.push(Draw::TiledSprite {
                            rect: frame.rect(
                                x + first * element.tile,
                                y,
                                count * element.tile,
                                rows * element.tile,
                            ),
                            uv,
                            repeat: [count, rows],
                            color,
                        });
                    }
                }
            }
            Kind::Clip => {
                // A video quad has no alpha to fade with, and nothing is drawn until the
                // clip has a picture in the renderer's planes: an empty plane set is green.
                if element.source.as_deref() != Some("mode-icon")
                    || !self.mode_icon
                    || self.fade > 0
                {
                    return;
                }
                out.push(Draw::Video {
                    rect: frame.rect(x, y, element.size[0], element.size[1]),
                    frame: self.frames as usize,
                    position: u64::from(self.frames),
                    source: oag_ui::frontend::Video::ModeIcon,
                });
            }
        }
    }

    /// One text element: wrapped at its `wrap` and `pitch` where it has them (a paragraph),
    /// elided from the left at its `wrap` where it has only that (a file name), else one line.
    fn put_text(
        &self,
        element: &Element,
        frame: &Frame,
        atlas: &Atlas,
        text: String,
        colour: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        let scale = frame.text_scale(element.scale);
        let [x, y] = frame.point(element.at[0], element.at[1]);
        let align = match element.align {
            At::Left => Align::Left,
            At::Centre => Align::Centre,
            At::Right => Align::Right,
        };
        let role = face(element.font.as_deref());
        let mut line = |at_y: f32, text: String| {
            out.push(match role {
                Some(role) => Draw::FacedText {
                    role,
                    x,
                    y: at_y,
                    scale,
                    color: colour,
                    border: None,
                    align,
                    text,
                    wrap_width: None,
                },
                None => Draw::Text {
                    x,
                    y: at_y,
                    scale,
                    color: colour,
                    border: None,
                    align,
                    text,
                    wrap_width: None,
                },
            });
        };
        match (element.wrap, element.pitch) {
            (Some(width), Some(pitch)) => {
                let width = frame.along(width);
                let pitch = frame.along(pitch);
                for (row, text) in wrap_spaced(atlas, &text, scale, width)
                    .into_iter()
                    .enumerate()
                {
                    line(y + row as f32 * pitch, text);
                }
            }
            (Some(width), None) => line(y, elide(atlas, &text, scale, frame.along(width))),
            _ => line(y, text),
        }
    }

    /// Tells the screen the mode icon's clip has a picture in the renderer's planes, or has not.
    /// See [`super::icon`].
    pub fn set_mode_icon(&mut self, shown: bool) {
        self.mode_icon = shown;
    }

    /// The player's Safe Area as a scale about the screen's centre (`1.0` is the whole
    /// screen), which the original applies to this screen as to its HUD.
    pub fn set_safe_area(&mut self, scale: f32) {
        self.safe_area = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
    }

    /// The width over height of the surface the screen is drawn on, which picks among the
    /// layouts of a file that has several. `16:9` until a caller says.
    pub fn set_surface_aspect(&mut self, aspect: f32) {
        if aspect.is_finite() && aspect > 0.0 {
            self.surface_aspect = aspect;
        }
    }
}
