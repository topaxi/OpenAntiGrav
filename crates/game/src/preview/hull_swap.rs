//! What HD/Fury's `Team Selection` does to the craft when the screen opens
//! and when the selection steps (measured off RPCS3).
//!
//! The pose is fixed ([`super::hull_orbit`]); what moves is the *swap*:
//!
//! - **Opening**: the craft is absent while the screen's own particle
//!   transition plays and cuts in, at its rest pose and with no settle, about a
//!   second after the press that opened the screen.
//! - **Stepping** (either direction): the old craft is cut in one frame, the
//!   frame is empty for a few frames, and the new craft cuts in, already
//!   solid. It is slightly off its rest size and eases to it; that is the
//!   screen's depth fit, not a curve of its own ([`super::ship_model::FitMotion`]).
//!
//! This module holds the timings, which are measured.

use super::ship_model::FRAME_SECONDS;

/// Seconds from the screen opening to the craft's first frame. One press at
/// frame 90 of a 30 fps film, craft first present at frame 119-120: 0.97 s,
/// less an input latency of about 0.1 s (confidence 60: one boot, two films).
pub const ENTRY_SECONDS: f32 = 0.9;

/// Screen frames the picture holds no craft after a step, counted from the
/// frame the selection moves: the craft is absent at the original's RAM frames
/// 3 to 5 and present from 6 (the header dots change at frame 3), so the cut
/// is 3 frames from the selection whatever the picture's absolute lag (three
/// of four grabs at frame 6, all four at frame 7; confidence 70,
/// `docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`, "The step
/// against RAM"). Frames of the screen's own 60 Hz update
/// ([`super::ship_model::FRAME_SECONDS`]), not a seconds value fitted to one
/// capture.
pub const GAP_FRAMES: u32 = 3;

/// The share of a frame a tick may fall short of a frame boundary and still
/// count it, as the fit's own frame loop does.
const FRAME_TOLERANCE: f32 = 0.999;

/// What to draw of the craft this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phase {
    /// Whether the craft is on screen at all.
    pub visible: bool,
}

/// The craft's phase `open` seconds after the screen opened and `since`
/// seconds after the selection last moved (equal until the first step).
#[must_use]
pub fn phase(open: f32, since: f32) -> Phase {
    let stepped = open - since > 1.0e-3;
    let visible = if stepped {
        since >= GAP_FRAMES as f32 * FRAME_SECONDS * FRAME_TOLERANCE
    } else {
        open >= ENTRY_SECONDS
    };
    Phase { visible }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_opens_empty_then_cuts_the_craft_in() {
        assert!(!phase(0.3, 0.3).visible);
        assert!(phase(ENTRY_SECONDS + 0.01, ENTRY_SECONDS + 0.01).visible);
    }

    /// The craft is absent on the step's frames 0 to 2 and present from frame 3,
    /// the original's 3 absent frames from the selection (`team-selection-model.md`,
    /// "The step against RAM", item 4). A longer or shorter gap moves the frame-6
    /// step pair, where the original already shows the new craft.
    #[test]
    fn a_step_cuts_the_craft_for_three_frames() {
        let after = |frames: u32| phase(5.0, frames as f32 * FRAME_SECONDS).visible;
        assert_eq!(GAP_FRAMES, 3);
        assert_eq!(
            [after(0), after(1), after(2), after(3), after(4), after(30)],
            [false, false, false, true, true, true]
        );
        assert!(phase(5.0, 0.05).visible, "the CLI's 0.05 s is frame 3");
        assert!(!phase(5.0, 0.0333).visible, "0.0333 s is frame 2");
    }
}
