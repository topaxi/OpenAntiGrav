//! The on-screen racing controls, as a draw list.
//!
//! **Plain shapes, chosen not measured.** No title ships a touch scheme, so
//! there is no original art to play. The project's button-prompt glyphs
//! (`oag_ui::prompt`) are PromptFont codepoints drawn by the text pass for a
//! pad's face buttons; none of them is a thrust, shield or pause symbol, so
//! each control carries a vector glyph built from the same row fills as its
//! body (see [`shape`]): chevrons, a shield, a crosshair, side arrows, pause
//! bars. The only words are GO and the L and R of the airbrakes: the other glyphs
//! (crosshair, shield, pause bars, eye) say which button they are alone.
//!
//! The look follows the common phone overlay: round translucent buttons with
//! a thin soft outline and a dark low-opacity body (so the white glyph reads
//! on HD's bright track), a pressed state that is brighter but never
//! opaque, and a stick that is a ring with a knob where the thumb landed.
//! Where the layout is and what it presses is [`oag_input::touch`]'s; this
//! only draws what it is told.
//!
//! Drawn in the overlay's grid (see `oag_present::perf::grid`), whose height
//! is fixed, so `scale` is grid units per window pixel.

mod shape;

use oag_input::touch::{self, Control, GoZone, Touches};
use oag_ui::frontend::{Align, Draw};

use shape::Rounded;

/// Colours, all translucent. A dark body keeps the white glyph legible over
/// HD's bright track; the held state is a cyan wash well short of opaque.
/// Chosen, not measured.
const BODY: [f32; 4] = [0.0, 0.0, 0.0, 0.28];
const BODY_HELD: [f32; 4] = [0.2, 0.8, 1.0, 0.40];
const EDGE: [f32; 4] = [1.0, 1.0, 1.0, 0.60];
const EDGE_HELD: [f32; 4] = [0.7, 1.0, 1.0, 0.80];
const GLYPH: [f32; 4] = [1.0, 1.0, 1.0, 0.85];
const DIVIDER: [f32; 4] = [1.0, 1.0, 1.0, 0.30];
const ZONE_LIT: [f32; 4] = [1.0, 0.75, 0.2, 0.40];
const STICK_BODY: [f32; 4] = [1.0, 1.0, 1.0, 0.08];
const STICK_EDGE: [f32; 4] = [1.0, 1.0, 1.0, 0.35];
const STICK_KNOB: [f32; 4] = [1.0, 1.0, 1.0, 0.40];

/// Outline width in grid units.
const BORDER: f32 = 1.0;
/// Label scale: the overlay's built-in glyphs are already small.
const LABEL_SCALE: f32 = 1.0;
/// Approximate cap height of the built-in face, in grid units.
const LABEL_HEIGHT: f32 = 8.0;
/// GO's corner radius as a share of its side.
const GO_ROUND: f32 = 0.30;

fn scaled(mut color: [f32; 4], opacity: f32) -> [f32; 4] {
    color[3] *= opacity;
    color
}

fn text(x: f32, y: f32, word: &str, opacity: f32) -> Draw {
    Draw::Text {
        x,
        y,
        scale: LABEL_SCALE,
        color: scaled(GLYPH, opacity),
        border: None,
        align: Align::Centre,
        text: word.to_string(),
        wrap_width: None,
    }
}

fn fill(rect: [f32; 4], color: [f32; 4]) -> Draw {
    Draw::Fill { rect, color }
}

/// GO's brake corners: the active one lit inside GO's own outline, a faint
/// divider either side of the dead centre, and the `L` and `R` letters.
fn go_zones(touches: &Touches, body: Rounded, opacity: f32, out: &mut Vec<Draw>) {
    let [x, y, w, h] = body.rect;
    let side = w * touch::GO_ZONE_SIDE;
    let top = y + h * (1.0 - touch::GO_ZONE_BOTTOM);
    let band = y + h - top;
    let inner = body.inset(BORDER);
    for (zone, cell_x, word) in [(GoZone::Left, x, "L"), (GoZone::Right, x + w - side, "R")] {
        if touches.zone_down(zone) {
            shape::fill(
                inner,
                Some([cell_x, top, side, band]),
                scaled(ZONE_LIT, opacity),
                out,
            );
        }
        out.push(text(
            cell_x + side / 2.0,
            top + (band - LABEL_HEIGHT) / 2.0,
            word,
            opacity,
        ));
    }
    for line_x in [x + side, x + w - side] {
        out.push(fill(
            [line_x, top + 2.0, 0.5, band - 2.0 - BORDER - 2.0],
            scaled(DIVIDER, opacity),
        ));
    }
}

/// The glyph for `control`, centred in `rect`, and the word under it if it
/// has one. GO's glyph stays in its upper part so the lower band keeps its
/// brake corners.
fn glyph(control: Control, rect: [f32; 4], paused: bool, opacity: f32, out: &mut Vec<Draw>) {
    let [x, y, w, h] = rect;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let u = w.min(h);
    let ink = scaled(GLYPH, opacity);
    match control {
        Control::Accelerate => {
            let g = u * 0.30;
            let top = y + h * 0.07;
            for k in 0..2 {
                shape::triangle(
                    [cx - g / 2.0, top + k as f32 * g * 0.62, g, g * 0.7],
                    true,
                    ink,
                    out,
                );
            }
        }
        Control::Fire => {
            let ring = Rounded {
                rect: [cx - u * 0.25, cy - u * 0.25, u * 0.5, u * 0.5],
                radius: u * 0.25,
            };
            shape::button(ring, 1.5, ink, [0.0; 4], out);
            let dot = u * 0.06;
            let d = Rounded {
                rect: [cx - dot, cy - dot, 2.0 * dot, 2.0 * dot],
                radius: dot,
            };
            shape::fill(d, None, ink, out);
        }
        Control::Absorb => {
            let g = u * 0.40;
            shape::shield([cx - g / 2.0, cy - g * 0.55, g, g * 1.1], ink, out);
        }
        Control::AirbrakeLeft | Control::AirbrakeRight => {
            let right = control == Control::AirbrakeRight;
            let g = u * 0.30;
            shape::arrow_side(
                [cx - g / 2.0, cy - g * 0.6 - 2.0, g, g * 1.2],
                right,
                ink,
                out,
            );
            out.push(text(
                cx,
                cy + u * 0.18,
                if right { "R" } else { "L" },
                opacity,
            ));
        }
        Control::Pause => {
            let bar_w = u * 0.10;
            let bar_h = u * 0.40;
            if paused {
                shape::arrow_side(
                    [cx - bar_h * 0.4, cy - bar_h / 2.0, bar_h * 0.8, bar_h],
                    true,
                    ink,
                    out,
                );
            } else {
                for dx in [-1.6, 0.6] {
                    out.push(fill(
                        [cx + dx * bar_w, cy - bar_h / 2.0, bar_w * 1.0, bar_h],
                        ink,
                    ));
                }
            }
        }
        Control::Camera => {
            let body = Rounded {
                rect: [cx - u * 0.28, cy - u * 0.19, u * 0.56, u * 0.38],
                radius: u * 0.07,
            };
            shape::button(body, 1.2, ink, [0.0; 4], out);
            let lens = u * 0.10;
            shape::button(
                Rounded {
                    rect: [cx - lens, cy - lens, 2.0 * lens, 2.0 * lens],
                    radius: lens,
                },
                1.2,
                ink,
                [0.0; 4],
                out,
            );
        }
    }
}

/// A held-finger pose to draw in a capture, since a headless run has no
/// fingers: `--touch-overlay`. Same [`Touches`] a real finger would build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Demo {
    /// Nothing held.
    Idle,
    /// A finger in GO's bottom-left corner.
    GoLeft,
    /// A finger in GO's bottom-right corner.
    GoRight,
    /// A finger in GO's centre, with another on the floating stick pushed
    /// right.
    StickAndGo,
}

impl Demo {
    /// The pose named on the command line.
    ///
    /// # Errors
    /// Names the accepted values when `text` is none of them.
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "idle" => Ok(Self::Idle),
            "go-left" => Ok(Self::GoLeft),
            "go-right" => Ok(Self::GoRight),
            "stick-go" => Ok(Self::StickAndGo),
            other => Err(format!(
                "unknown touch pose {other:?}: idle, go-left, go-right or stick-go"
            )),
        }
    }

    /// The fingers for a window of `size` pixels.
    #[must_use]
    pub fn touches(self, size: (f32, f32), zones: bool) -> Touches {
        let mut touches = Touches::default();
        touches.set_go_zones(zones, size);
        let go = touch::layout(size)[0].1;
        let at = |fx: f32, fy: f32| (go.x + go.w * fx, go.y + go.h * fy);
        match self {
            Self::Idle => {}
            Self::GoLeft => touches.down(1, at(0.12, 0.88), size),
            Self::GoRight => touches.down(1, at(0.88, 0.88), size),
            Self::StickAndGo => {
                touches.down(1, at(0.5, 0.35), size);
                let origin = (size.0 * 0.14, size.1 * 0.62);
                touches.down(2, origin, size);
                touches.moved(
                    2,
                    (origin.0 + size.1 * 0.10, origin.1 - size.1 * 0.03),
                    size,
                );
            }
        }
        touches
    }
}

/// Every control and the stick, for a window of `size` pixels, in a grid
/// `scale` units per pixel. While `paused` the pause button is a play
/// triangle; `zones` is whether GO carries its brake corners; `opacity`
/// scales every alpha (1.0 is the design).
#[must_use]
pub fn draw(
    touches: &Touches,
    size: (f32, f32),
    scale: f32,
    paused: bool,
    zones: bool,
    opacity: f32,
) -> Vec<Draw> {
    let mut out = Vec::new();
    for (control, rect) in touch::layout(size) {
        let held = touches.is_down(control);
        let rect = [
            rect.x * scale,
            rect.y * scale,
            rect.w * scale,
            rect.h * scale,
        ];
        let radius = if control == Control::Accelerate {
            rect[2] * GO_ROUND
        } else {
            rect[2].min(rect[3]) / 2.0
        };
        let body = Rounded { rect, radius };
        let (edge, fill_colour) = if held {
            (EDGE_HELD, BODY_HELD)
        } else {
            (EDGE, BODY)
        };
        shape::button(
            body,
            BORDER,
            scaled(edge, opacity),
            scaled(fill_colour, opacity),
            &mut out,
        );
        glyph(control, rect, paused, opacity, &mut out);
        if zones && control == Control::Accelerate {
            out.push(text(
                rect[0] + rect[2] / 2.0,
                rect[1] + rect[3] * 0.55,
                "GO",
                opacity,
            ));
            go_zones(touches, body, opacity, &mut out);
        } else if control == Control::Accelerate {
            out.push(text(
                rect[0] + rect[2] / 2.0,
                rect[1] + rect[3] * 0.62,
                "GO",
                opacity,
            ));
        }
    }
    if let Some((origin, at)) = touches.stick() {
        let radius = size.1 * touch::STICK_RADIUS * scale;
        let o = (origin.0 * scale, origin.1 * scale);
        let base = Rounded {
            rect: [o.0 - radius, o.1 - radius, 2.0 * radius, 2.0 * radius],
            radius,
        };
        shape::button(
            base,
            BORDER,
            scaled(STICK_EDGE, opacity),
            scaled(STICK_BODY, opacity),
            &mut out,
        );
        let dx = (at.0 - origin.0) * scale;
        let dy = (at.1 - origin.1) * scale;
        let reach = (dx * dx + dy * dy).sqrt();
        let k = if reach > radius { radius / reach } else { 1.0 };
        let knob = radius * 0.38;
        shape::fill(
            Rounded {
                rect: [
                    o.0 + dx * k - knob,
                    o.1 + dy * k - knob,
                    2.0 * knob,
                    2.0 * knob,
                ],
                radius: knob,
            },
            None,
            scaled(STICK_KNOB, opacity),
            &mut out,
        );
    }
    out
}

#[cfg(test)]
mod tests;
