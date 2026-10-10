//! What HD/Fury's `Team Selection` does to the craft when the screen opens
//! and when the selection steps (measured off RPCS3, 2026-10-10).
//!
//! The pose is fixed ([`super::hull_orbit`]); what moves is the *swap*:
//!
//! - **Opening**: the craft is absent while the screen's own particle
//!   transition plays and cuts in, at its rest pose and with no settle, about a
//!   second after the press that opened the screen.
//! - **Stepping** (either direction): the old craft is cut in one frame, the
//!   frame is empty for a few frames, and the new craft cuts in, already
//!   solid, slightly off its rest size and eases back to it.
//!
//! The timings are measured; the size the settle starts from is **chosen, not
//! measured** - see [`SETTLE_START_SCALE`].

/// Seconds from the screen opening to the craft's first frame. One press at
/// frame 90 of a 30 fps film, craft first present at frame 119-120: 0.97 s,
/// less an input latency of about 0.1 s (confidence 60: one boot, two films).
pub const ENTRY_SECONDS: f32 = 0.9;

/// Seconds the frame holds no craft after a step: 3 captured frames at 60 fps
/// (two steps), 4 once, so 50-67 ms (confidence 70). The same gap read 3
/// frames at 30 fps (100 ms) under a heavier capture, so it is counted in
/// emulator frames and varies with the emulator's speed.
pub const GAP_SECONDS: f32 = 0.06;

/// Spacing of [`SETTLE`]'s samples, seconds: three captured frames at 60 fps.
const SETTLE_STEP_SECONDS: f32 = 0.05;

/// How far the new craft still is from its rest size, as a fraction of where it
/// started, every [`SETTLE_STEP_SECONDS`] after it appears. The mean of two
/// right steps onto the same team at 60 fps, read off the hull silhouette's
/// width; a third, to a different team, eased the same way over the same time
/// (5% left at 0.45 s) with a faster first half. Confidence 65.
const SETTLE: [f32; 13] = [
    1.0, 1.0, 0.97, 0.83, 0.56, 0.39, 0.24, 0.17, 0.10, 0.06, 0.03, 0.01, 0.0,
];

/// The size a craft starts its settle at, against its rest size.
///
/// **Chosen, not measured.** The original's start differs per team and was
/// seen at 0.95 (Feisar), 1.08 and 1.12 (two other teams) of rest width, and
/// both directions of step produce both signs, so it is a per-team start pose
/// this build does not recover. 1.06 is a middle of those.
pub const SETTLE_START_SCALE: f32 = 1.06;

/// What to draw of the craft this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phase {
    /// Whether the craft is on screen at all.
    pub visible: bool,
    /// Size against rest, 1.0 at rest.
    pub scale: f32,
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
    let scale = if stepped && visible {
        let index = (since - GAP_SECONDS) / SETTLE_STEP_SECONDS;
        let low = index.floor() as usize;
        let excess = if low + 1 >= SETTLE.len() {
            0.0
        } else {
            let t = index - low as f32;
            SETTLE[low] + (SETTLE[low + 1] - SETTLE[low]) * t
        };
        1.0 + (SETTLE_START_SCALE - 1.0) * excess
    } else {
        1.0
    };
    Phase { visible, scale }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_opens_empty_then_cuts_the_craft_in_at_rest() {
        assert!(!phase(0.3, 0.3).visible);
        let on = phase(ENTRY_SECONDS + 0.01, ENTRY_SECONDS + 0.01);
        assert!(on.visible);
        assert_eq!(on.scale, 1.0);
    }

    #[test]
    fn a_step_cuts_the_craft_for_the_gap_then_settles_to_rest() {
        assert!(!phase(5.0, 0.02).visible);
        let first = phase(5.0, GAP_SECONDS + 0.001);
        assert!(first.visible);
        assert!((first.scale - SETTLE_START_SCALE).abs() < 1.0e-3);
        assert!(phase(5.0, GAP_SECONDS + 0.2).scale < first.scale);
        assert_eq!(phase(5.0, 1.0).scale, 1.0);
    }
}
