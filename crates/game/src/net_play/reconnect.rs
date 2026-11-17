//! The RECONNECTING overlay: drawn over the last frame of a network race whose link is gone,
//! while [`super::NetPlay`] dials for its seat. A pure draw list, so a test can read it.

use oag_ui::frontend::{Align, Draw};

const DIM: [f32; 4] = [0.0, 0.0, 0.0, 0.6];
const TEXT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const HINT: [f32; 4] = [0.8, 0.8, 0.8, 1.0];

/// The overlay for a `grid` of the overlay's size: the whole frame dimmed, `title` and the
/// seconds left centred, `hint` (how to give up) under them.
#[must_use]
pub fn draw(grid: (f32, f32), title: &str, seconds: u32, hint: &str) -> Vec<Draw> {
    let text = |y: f32, scale: f32, color: [f32; 4], text: String| Draw::Text {
        x: grid.0 / 2.0,
        y,
        scale,
        color,
        border: None,
        align: Align::Centre,
        text,
        wrap_width: Some(grid.0 * 0.9),
    };
    vec![
        Draw::Fill {
            rect: [0.0, 0.0, grid.0, grid.1],
            color: DIM,
        },
        text(grid.1 * 0.38, 2.0, TEXT, title.to_string()),
        text(grid.1 * 0.38 + 30.0, 1.0, TEXT, format!("{seconds} S")),
        text(grid.1 * 0.38 + 50.0, 1.0, HINT, hint.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overlay_dims_the_frame_and_says_the_seconds() {
        let list = draw((480.0, 272.0), "RECONNECTING", 17, "PRESS BACK TO LEAVE");
        assert!(matches!(
            list[0],
            Draw::Fill {
                rect: [0.0, 0.0, 480.0, 272.0],
                ..
            }
        ));
        let words: Vec<&str> = list
            .iter()
            .filter_map(|d| match d {
                Draw::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(words, ["RECONNECTING", "17 S", "PRESS BACK TO LEAVE"]);
    }
}
