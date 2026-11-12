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

/// Seconds from the screen opening to the craft's first frame. One press at
/// frame 90 of a 30 fps film, craft first present at frame 119-120: 0.97 s,
/// less an input latency of about 0.1 s (confidence 60: one boot, two films).
pub const ENTRY_SECONDS: f32 = 0.9;

/// Seconds the frame holds no craft after a step: 3 captured frames at 60 fps
/// (two steps), 4 once, so 50-67 ms (confidence 70). The same gap read 3
/// frames at 30 fps (100 ms) under a heavier capture, so it is counted in
/// emulator frames and varies with the emulator's speed.
pub const GAP_SECONDS: f32 = 0.06;

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
        since >= GAP_SECONDS
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

    #[test]
    fn a_step_cuts_the_craft_for_the_gap() {
        assert!(!phase(5.0, 0.02).visible);
        assert!(phase(5.0, GAP_SECONDS + 0.001).visible);
        assert!(phase(5.0, 1.0).visible);
    }
}
