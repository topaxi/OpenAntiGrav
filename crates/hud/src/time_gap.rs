//! The lap-line time gap: `TimeDiffText` and `TimeDiffIcon`.
//!
//! The law is `FUN_0881f890` (`0x0881f890`, psp-pulse-usa), fed by the flags
//! `Craft_UpdateLapProgress` leaves in the player's status block. Each lap
//! line crossing is stamped with the race clock of the first craft over it;
//! the next craft over it, if one of the two is the player, makes a gap:
//!
//! - the player leads: from the player's crossing the line counts up
//!   (`"%.1f"`, light green) while the field has two or more live craft, until
//!   the next craft crosses;
//! - then the gap is held for 3.5 s with a `"- %.1f"` in green (the player is
//!   ahead) or `"+ %.1f"` in red (the player crossed behind), flashing between
//!   two shades at 5 Hz.
//!
//! It is the gap to **the first craft over that line**, not necessarily the one
//! directly ahead, and the leader's stamp is a whole second (the table stores
//! `(int)clock`), so a gap reads up to a second long.

use oag_ui::screen::argb_to_rgba;

/// How long a held gap stays up.
const HOLD_SECONDS: f32 = 3.5;
/// A gap at or beyond this many seconds is not drawn (`-30 < gap < 30`).
const SHOW_BELOW: f32 = 30.0;

/// What the lap code reports this tick, the original's `PLAYER_HUD+0x54..0x56`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GapEvents {
    /// The player crossed first: the stamp (whole seconds) of the crossing.
    pub led_at: Option<f32>,
    /// A gap was made: seconds, and whether the player is ahead.
    pub gap: Option<(f32, bool)>,
}

/// The line to draw: text, colour (RGBA) and whether the icon goes with it.
#[derive(Debug, Clone, PartialEq)]
pub struct GapLine {
    /// The text.
    pub text: String,
    /// Its colour, also the icon's tint.
    pub color: [f32; 4],
    /// The held gap carries the icon; the running count does not.
    pub icon: bool,
}

/// The display state machine (`hud+0x9c..0xb0`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimeGap {
    leading: bool,
    lead_time: f32,
    holding: bool,
    ahead: bool,
    gap: f32,
    timer: f32,
    line: Option<GapLine>,
}

impl TimeGap {
    /// One tick. `now` is the race clock in seconds, `live` the craft still in
    /// the race and `place` the player's place (1 = first).
    pub fn advance(&mut self, dt: f32, now: f32, events: GapEvents, live: usize, place: u32) {
        if let Some(stamp) = events.led_at {
            self.leading = true;
            self.lead_time = stamp;
        }
        if let Some((gap, ahead)) = events.gap {
            self.leading = false;
            self.holding = true;
            self.gap = gap;
            self.ahead = ahead;
            self.timer = 0.0;
        }
        self.line = None;
        if self.leading && live >= 2 {
            let count = now - self.lead_time;
            if count < SHOW_BELOW {
                self.line = Some(GapLine {
                    text: format!("{count:.1}"),
                    color: argb_to_rgba(0xFF60_FF60),
                    icon: false,
                });
            }
            return;
        }
        if !self.holding {
            return;
        }
        let flash = (self.timer * 5.0) as i64 & 1 == 1;
        let (prefix, argb) = match (self.ahead, flash) {
            (false, false) => ("+ ", 0xFFC0_1010),
            (false, true) => ("+ ", 0xFFFF_3030),
            (true, false) => ("- ", 0xFF10_C010),
            (true, true) => ("- ", 0xFF30_FF30),
        };
        if self.ahead {
            self.holding = place == 1;
        }
        if -SHOW_BELOW < self.gap && self.gap < SHOW_BELOW {
            self.line = Some(GapLine {
                text: format!("{prefix}{:.1}", self.gap),
                color: argb_to_rgba(argb),
                icon: true,
            });
        }
        self.timer += dt;
        if self.timer > HOLD_SECONDS {
            self.holding = false;
        }
    }

    /// What to draw this frame.
    #[must_use]
    pub fn line(&self) -> Option<&GapLine> {
        self.line.as_ref()
    }
}

/// The text widget's colour while a line is up.
#[must_use]
pub(super) fn colour(readout: &super::Readout, name: &str) -> Option<[f32; 4]> {
    (name == "TimeDiffText")
        .then(|| readout.time_gap.as_ref().map(|line| line.color))
        .flatten()
}

#[cfg(test)]
mod tests;
