//! Pure's storage warning, drawn with this build's own wording. See
//! [`Frontend::draw_storage_warning`].
//!
//! Split out of `draw.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

impl Frontend {
    /// This build's own storage warning, at the disc's own geometry.
    ///
    /// **The wording is deliberately not the disc's.** Its six `MSInfo`/`MSWarning`
    /// strings are about a Memory Stick Duo being physically removed mid-write;
    /// this is a reimplementation on hardware where storage is assumed present, so
    /// the screen keeps its structure, its colours, its rules and its cross gate,
    /// and says what is actually true here instead. A product decision, recorded
    /// in [`pure_states::MEMORY_STICK_WARNING`] so it is not "fixed" back to the
    /// disc's strings by someone reading the XML.
    ///
    /// Everything geometric *is* the disc's: the two rules at y=10 and y=240, the
    /// body at x=15 from y=30, the prompt at y=245, and the
    /// `MSWarningColour1`/`MSWarningColour2`/`MSWarningScale` globals the screen's
    /// own widgets reference.
    pub(super) fn draw_storage_warning(&self, out: &mut Vec<Draw>) {
        let global = |name: &str, fallback: u32| {
            self.screens
                .globals
                .get(name)
                .and_then(|value| parse_argb(value))
                .unwrap_or(fallback)
        };
        let heading = argb_to_rgba(global("MSWarningColour1", 0xFF00_AEEF));
        let body = argb_to_rgba(global("MSWarningColour2", 0xFF00_AEEF));
        let scale = self
            .screens
            .globals
            .get("MSWarningScale")
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(1.0);

        for y in [10.0, 240.0] {
            out.push(Draw::Fill {
                rect: [0.0, y, self.space.size.0, 1.0],
                color: heading,
            });
        }
        for (index, line) in [
            "THIS GAME SAVES AUTOMATICALLY.",
            "PROGRESS IS WRITTEN WHEN A RACE ENDS AND",
            "WHEN A SETTING CHANGES.",
        ]
        .iter()
        .enumerate()
        {
            out.push(Draw::Text {
                x: 15.0,
                y: 30.0 + index as f32 * 15.0,
                scale,
                color: body,
                border: None,
                align: Align::Left,
                text: (*line).to_string(),
                wrap_width: None,
            });
        }
        out.push(Draw::Text {
            x: 15.0,
            y: 245.0,
            scale,
            color: heading,
            border: None,
            align: Align::Left,
            text: "PRESS X TO CONTINUE".to_string(),
            wrap_width: None,
        });
    }
}
