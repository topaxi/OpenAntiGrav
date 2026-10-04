//! What the player's d-pad does to the director on `Race End Photo`.
//!
//! `RaceManager_Update` (`0x08829778`) reads four buttons while the front end is in
//! `Race End Photo` (`manager+0x19ed`, set when the state name compares equal) and the race
//! runs on behind it: up and down call `Camera_CycleModeForward` (`0x088807c8`) and
//! `Camera_CycleModeBack` (`0x08880838`), left and right hand the camera the previous or next
//! craft (`Camera_SetSubject`, `0x0888058c`). **Measured** on PPSSPP 2026-10-04: up took the
//! mode `4 -> 3 -> 2`, down `2 -> 3`, right and left stepped the watched craft one slot either
//! way (`docs/ghidra/functions/psp-pulse-usa/race-finish.md`).
//!
//! The cycle stores the mode word directly, not through `Camera_SetMode`, so the view width a
//! node mode last set is kept: cycling into mode 7 frames with whatever width the director last
//! used. The chosen mode lasts until the director's next cut on its subject, which rolls a new
//! one as before.

use super::{FinishCamera, Subject, ViewMode};

impl FinishCamera {
    /// Up (`forward`) or down on `Race End Photo`: the next mode in
    /// `1 -> 4 -> 3 -> 2 -> 7 -> 1`, or the previous. Mode 6 goes to 1 forward and 2 back, as 7
    /// does; the death camera (5) has no case and stays. `false` before the director runs.
    pub fn cycle(&mut self, forward: bool) -> bool {
        if !self.running {
            return false;
        }
        self.mode = if forward {
            match self.mode {
                ViewMode::Nose => ViewMode::Chase,
                ViewMode::Chase => ViewMode::Front,
                ViewMode::Front => ViewMode::Rear,
                ViewMode::Rear => ViewMode::Track,
                ViewMode::Close | ViewMode::Track => ViewMode::Nose,
                ViewMode::Death => ViewMode::Death,
            }
        } else {
            match self.mode {
                ViewMode::Nose => ViewMode::Track,
                ViewMode::Rear => ViewMode::Front,
                ViewMode::Front => ViewMode::Chase,
                ViewMode::Chase => ViewMode::Nose,
                ViewMode::Close | ViewMode::Track => ViewMode::Rear,
                ViewMode::Death => ViewMode::Death,
            }
        };
        true
    }

    /// Left (`forward == false`) or right on `Race End Photo`: watch the grid slot before or
    /// after the current subject, wrapping over `slots` (the original steps over every racer,
    /// live or not). The camera takes the node nearest that craft and starts its shot afresh,
    /// the craft shown becomes it at once, and the mode is kept (`Camera_SetMode` with the mode
    /// it had, so a node mode re-applies its width).
    pub fn watch_step(&mut self, forward: bool, slots: usize, live: &[Subject]) -> bool {
        if !self.running || slots == 0 {
            return false;
        }
        let slot = if forward {
            (self.subject + 1) % slots
        } else {
            (self.subject + slots - 1) % slots
        };
        let position = live
            .iter()
            .find(|s| s.slot == slot)
            .map_or(super::Vec3::ZERO, |s| s.position);
        self.subject = slot;
        self.previous = Some(slot);
        if let Some(node) = self.nearest_node(position) {
            self.node = Some(node);
            self.take_node(position);
        }
        if let Some(width) = self.mode.width() {
            self.width = width;
        }
        true
    }
}
