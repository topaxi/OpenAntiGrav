//! The on-screen racing controls, as a draw list.
//!
//! **Plain shapes and plain words, chosen not measured.** No title ships a
//! touch scheme, so there is no original art to play: each control is a flat
//! translucent rectangle with a short label in the overlay's built-in face,
//! and the steering stick is a ring with a dot. Where the layout is and what
//! it presses is [`oag_input::touch`]'s; this only draws what it is told.
//!
//! Drawn in the overlay's grid (see `oag_present::perf::grid`), whose height
//! is fixed, so `scale` is grid units per window pixel.

use oag_input::touch::{self, Control, GoZone, Touches};
use oag_ui::frontend::{Align, Draw};

/// Semi-transparent fills, stronger outlines, and a pressed state that is
/// both brighter and a different colour. Chosen, not measured.
const IDLE: [f32; 4] = [1.0, 1.0, 1.0, 0.07];
const HELD: [f32; 4] = [0.3, 0.9, 1.0, 0.38];
const EDGE: [f32; 4] = [1.0, 1.0, 1.0, 0.45];
const EDGE_HELD: [f32; 4] = [0.6, 1.0, 1.0, 0.95];
const LABEL: [f32; 4] = [1.0, 1.0, 1.0, 0.8];
const DIVIDER: [f32; 4] = [1.0, 1.0, 1.0, 0.35];
const ZONE_LIT: [f32; 4] = [1.0, 0.75, 0.2, 0.75];
const STICK_RING: [f32; 4] = [1.0, 1.0, 1.0, 0.22];
const STICK_DOT: [f32; 4] = [1.0, 1.0, 1.0, 0.55];

/// Border width in grid units.
const BORDER: f32 = 2.0;
/// Label scale: the overlay's built-in glyphs are already small.
const LABEL_SCALE: f32 = 1.0;
/// Approximate cap height of the built-in face, in grid units, to centre a
/// label vertically.
const LABEL_HEIGHT: f32 = 8.0;

fn text(x: f32, y: f32, text: &str) -> Draw {
    Draw::Text {
        x,
        y,
        scale: LABEL_SCALE,
        color: LABEL,
        border: None,
        align: Align::Centre,
        text: text.to_string(),
        wrap_width: None,
    }
}

fn fill(rect: [f32; 4], color: [f32; 4]) -> Draw {
    Draw::Fill { rect, color }
}

/// GO's brake corners: a divider along the top of the bottom band and one
/// either side of the dead centre column, an `L` and an `R` in the corners,
/// and the corner a finger is in lit.
fn go_zones(touches: &Touches, rect: [f32; 4], out: &mut Vec<Draw>) {
    let [x, y, w, h] = rect;
    let side = w * touch::GO_ZONE_SIDE;
    let top = y + h * (1.0 - touch::GO_ZONE_BOTTOM);
    let band = y + h - top;
    for (zone, cell_x, word) in [(GoZone::Left, x, "L"), (GoZone::Right, x + w - side, "R")] {
        if touches.zone_down(zone) {
            out.push(fill([cell_x, top, side, band], ZONE_LIT));
        }
        out.push(text(
            cell_x + side / 2.0,
            top + (band - LABEL_HEIGHT) / 2.0,
            word,
        ));
    }
    out.push(fill([x, top, w, 1.0], DIVIDER));
    out.push(fill([x + side, top, 1.0, band], DIVIDER));
    out.push(fill([x + w - side, top, 1.0, band], DIVIDER));
}

/// Every control and the stick, for a window of `size` pixels, in a grid
/// `scale` units per pixel. While `paused` the pause button reads RESUME;
/// `zones` is whether GO carries its brake corners.
#[must_use]
pub fn draw(
    touches: &Touches,
    size: (f32, f32),
    scale: f32,
    paused: bool,
    zones: bool,
) -> Vec<Draw> {
    let mut out = Vec::new();
    for (control, rect) in touch::layout(size) {
        let held = touches.is_down(control);
        let (x, y, w, h) = (
            rect.x * scale,
            rect.y * scale,
            rect.w * scale,
            rect.h * scale,
        );
        out.push(fill([x, y, w, h], if held { EDGE_HELD } else { EDGE }));
        out.push(fill(
            [x + BORDER, y + BORDER, w - 2.0 * BORDER, h - 2.0 * BORDER],
            if held { HELD } else { IDLE },
        ));
        let with_zones = zones && control == Control::Accelerate;
        let label_h = if with_zones {
            h * (1.0 - touch::GO_ZONE_BOTTOM)
        } else {
            h
        };
        let word = if paused && control == Control::Pause {
            "RESUME"
        } else {
            control.label()
        };
        out.push(text(x + w / 2.0, y + (label_h - LABEL_HEIGHT) / 2.0, word));
        if with_zones {
            go_zones(touches, [x, y, w, h], &mut out);
        }
    }
    if let Some((origin, at)) = touches.stick() {
        let radius = size.1 * touch::STICK_RADIUS * scale;
        let o = (origin.0 * scale, origin.1 * scale);
        out.push(Draw::Fill {
            rect: [o.0 - radius, o.1 - radius, 2.0 * radius, 2.0 * radius],
            color: STICK_RING,
        });
        let dx = (at.0 - origin.0) * scale;
        let dy = (at.1 - origin.1) * scale;
        let reach = (dx * dx + dy * dy).sqrt();
        let k = if reach > radius { radius / reach } else { 1.0 };
        let dot = radius * 0.35;
        out.push(Draw::Fill {
            rect: [o.0 + dx * k - dot, o.1 + dy * k - dot, 2.0 * dot, 2.0 * dot],
            color: STICK_DOT,
        });
    }
    out
}

#[cfg(test)]
mod tests;
