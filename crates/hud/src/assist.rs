//! Pilot Assist's HUD indicator: a background while the assist is on, a main
//! icon for three seconds after it corrects, and an arrow blinking at 4 Hz for
//! a second and a half on the side of the wall it steered away from.
//!
//! HD's `Hud_UpdateAssistIndicator` (`0x00089c90`) and 2048's (`0x81152488`) are
//! the same function: `3.0`, `1.5`, `4.0` and `0.5` at `0x008a7768`..`0x008a7770`
//! and `0x008a764c`. Evidence: `docs/ghidra/functions/ps3-hdfury-eu/pilot-assist.md`.
//! The widget names are the title's ([`oag_title::hud::AssistIndicator`]).

use crate::draw::{Context, sprite_draw};
use crate::{Draw, Readout};

/// How long the main icon stays up after a correction (`0x008a7768`).
const MAIN_SECONDS: f32 = 3.0;
/// How long the side arrow blinks after one (`0x008a776c`).
const SIDE_SECONDS: f32 = 1.5;
/// Blinks per second (`0x008a7770`).
const BLINKS_PER_SECOND: f32 = 4.0;
/// The part of a blink the arrow is up for (`0x008a764c`).
const BLINK_ON: f32 = 0.5;

/// What the indicator shows this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AssistReadout {
    /// The option is on: the background is up.
    pub enabled: bool,
    /// A correction within the last three seconds: the main icon is up.
    pub main: bool,
    /// The left arrow, on a blink's up half.
    pub left: bool,
    /// The right arrow, on a blink's up half.
    pub right: bool,
}

/// The indicator's own timers, `hud+0x608`..`+0x614`. Render-side: never hashed.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Indicator {
    main: f32,
    left: f32,
    right: f32,
    phase: f32,
    enabled: bool,
}

impl Indicator {
    /// One frame. `acting` is the craft's correction sign in HD's convention:
    /// `-1` turned it left off a right-hand wall, which blinks the **right** arrow.
    pub fn advance(&mut self, enabled: bool, acting: i8, dt: f32) {
        self.enabled = enabled;
        if !enabled {
            return;
        }
        match acting {
            -1 => {
                self.main = MAIN_SECONDS;
                self.right = SIDE_SECONDS;
            }
            1 => {
                self.main = MAIN_SECONDS;
                self.left = SIDE_SECONDS;
            }
            _ => {}
        }
        self.main = (self.main - dt).max(0.0);
        self.left = (self.left - dt).max(0.0);
        self.right = (self.right - dt).max(0.0);
        self.phase = if self.main == 0.0 {
            0.0
        } else {
            self.phase + dt
        };
    }

    /// What to show.
    #[must_use]
    pub fn readout(&self) -> AssistReadout {
        let blink = (self.phase * BLINKS_PER_SECOND).fract() < BLINK_ON;
        AssistReadout {
            enabled: self.enabled,
            main: self.enabled && self.main != 0.0,
            left: self.enabled && self.left != 0.0 && blink,
            right: self.enabled && self.right != 0.0 && blink,
        }
    }
}

/// The indicator's widgets that are up this frame, on a title that authors them.
pub(super) fn sprites(cx: &Context<'_>, readout: &Readout) -> Vec<Draw> {
    let Some(names) = cx.art.runtime.and_then(|runtime| runtime.assist) else {
        return Vec::new();
    };
    let state = readout.assist;
    let mut up: Vec<&str> = Vec::new();
    if state.enabled {
        up.push(names.background);
    }
    if state.main {
        up.push(names.main);
    }
    if state.left {
        up.extend(names.left);
    }
    if state.right {
        up.extend(names.right);
    }
    up.into_iter()
        .filter_map(|name| cx.layout.sprite(name))
        .flat_map(|sprite| sprite_draw(sprite, cx.sheet))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 0.25;

    #[test]
    fn off_it_shows_nothing_whatever_the_craft_does() {
        let mut indicator = Indicator::default();
        indicator.advance(false, -1, DT);
        assert_eq!(indicator.readout(), AssistReadout::default());
    }

    #[test]
    fn on_and_idle_it_shows_the_background_alone() {
        let mut indicator = Indicator::default();
        indicator.advance(true, 0, DT);
        let shown = indicator.readout();
        assert!(shown.enabled && !shown.main && !shown.left && !shown.right);
    }

    /// A correction off the right-hand wall: main for three seconds, the right
    /// arrow blinking for one and a half.
    #[test]
    fn a_correction_lights_main_and_blinks_the_wall_side() {
        let mut indicator = Indicator::default();
        indicator.advance(true, -1, 0.01);
        let shown = indicator.readout();
        assert!(shown.main && shown.right && !shown.left);
        indicator.advance(true, 0, 0.125);
        assert!(!indicator.readout().right, "the blink's off half");
        indicator.advance(true, 0, 0.125);
        assert!(indicator.readout().right, "and on again");
        for _ in 0..10 {
            indicator.advance(true, 0, 0.125);
        }
        let shown = indicator.readout();
        assert!(shown.main && !shown.right, "past 1.5 s the arrow is done");
        for _ in 0..12 {
            indicator.advance(true, 0, 0.125);
        }
        assert!(!indicator.readout().main, "past 3 s the icon is down");
    }

    #[test]
    fn the_other_wall_blinks_the_other_arrow() {
        let mut indicator = Indicator::default();
        indicator.advance(true, 1, 0.01);
        let shown = indicator.readout();
        assert!(shown.left && !shown.right);
    }
}
