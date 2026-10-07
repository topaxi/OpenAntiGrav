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

use oag_input::touch::{self, Control, Touches};
use oag_ui::frontend::{Align, Draw};

const IDLE: [f32; 4] = [1.0, 1.0, 1.0, 0.08];
const HELD: [f32; 4] = [0.3, 0.9, 1.0, 0.5];
const EDGE: [f32; 4] = [1.0, 1.0, 1.0, 0.3];
const LABEL: [f32; 4] = [1.0, 1.0, 1.0, 0.75];
const STICK_RING: [f32; 4] = [1.0, 1.0, 1.0, 0.22];
const STICK_DOT: [f32; 4] = [1.0, 1.0, 1.0, 0.55];

/// Border width in grid units.
const BORDER: f32 = 1.0;
/// Label scale: the overlay's built-in glyphs are already small.
const LABEL_SCALE: f32 = 1.0;
/// Approximate cap height of the built-in face, in grid units, to centre a
/// label vertically.
const LABEL_HEIGHT: f32 = 8.0;

/// Every control and the stick, for a window of `size` pixels, in a grid
/// `scale` units per pixel. While `paused` the pause button reads RESUME.
#[must_use]
pub fn draw(touches: &Touches, size: (f32, f32), scale: f32, paused: bool) -> Vec<Draw> {
    let mut out = Vec::new();
    for (control, rect) in touch::layout(size) {
        let held = touches.is_down(control);
        let (x, y, w, h) = (
            rect.x * scale,
            rect.y * scale,
            rect.w * scale,
            rect.h * scale,
        );
        out.push(Draw::Fill {
            rect: [x, y, w, h],
            color: EDGE,
        });
        out.push(Draw::Fill {
            rect: [x + BORDER, y + BORDER, w - 2.0 * BORDER, h - 2.0 * BORDER],
            color: if held { HELD } else { IDLE },
        });
        out.push(Draw::Text {
            x: x + w / 2.0,
            y: y + (h - LABEL_HEIGHT) / 2.0,
            scale: LABEL_SCALE,
            color: LABEL,
            border: None,
            align: Align::Centre,
            text: if paused && control == Control::Pause {
                "RESUME"
            } else {
                control.label()
            }
            .to_string(),
            wrap_width: None,
        });
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
mod tests {
    use super::*;

    const SIZE: (f32, f32) = (2400.0, 1080.0);

    #[test]
    fn an_idle_overlay_draws_every_control_and_no_stick() {
        let list = draw(&Touches::default(), SIZE, 272.0 / SIZE.1, false);
        let texts = list
            .iter()
            .filter(|d| matches!(d, Draw::Text { .. }))
            .count();
        assert_eq!(texts, oag_input::touch::Control::ALL.len());
        assert_eq!(list.len(), oag_input::touch::Control::ALL.len() * 3);
    }

    #[test]
    fn a_held_control_draws_brighter_and_a_stick_adds_two_shapes() {
        let scale = 272.0 / SIZE.1;
        let idle = draw(&Touches::default(), SIZE, scale, false);
        let mut touches = Touches::default();
        let accel = touch::layout(SIZE)[0].1.centre();
        touches.down(1, accel, SIZE);
        touches.down(2, (300.0, 800.0), SIZE);
        let list = draw(&touches, SIZE, scale, false);
        assert_eq!(list.len(), idle.len() + 2);
        let fill_alpha = |list: &[Draw], at: usize| match &list[at] {
            Draw::Fill { color, .. } => color[3],
            other => panic!("{other:?}"),
        };
        assert!(fill_alpha(&list, 1) > fill_alpha(&idle, 1));
    }

    #[test]
    fn the_pause_button_says_resume_while_paused() {
        let words = |paused: bool| {
            draw(&Touches::default(), SIZE, 0.25, paused)
                .into_iter()
                .filter_map(|d| match d {
                    Draw::Text { text, .. } => Some(text),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(words(false).contains(&"PAUSE".to_string()));
        assert!(words(true).contains(&"RESUME".to_string()));
        assert!(!words(true).contains(&"PAUSE".to_string()));
    }

    #[test]
    fn everything_stays_inside_the_grid() {
        let scale = 272.0 / SIZE.1;
        let width = SIZE.0 * scale;
        for draw in draw(&Touches::default(), SIZE, scale, false) {
            if let Draw::Fill { rect, .. } = draw {
                assert!(rect[0] >= 0.0 && rect[0] + rect[2] <= width + 0.01);
                assert!(rect[1] >= 0.0 && rect[1] + rect[3] <= 272.01);
            }
        }
    }
}
