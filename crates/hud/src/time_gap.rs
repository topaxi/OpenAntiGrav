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
//! - then the gap is held for 3.5 s in green (the player is ahead) or red (the
//!   player crossed behind), flashing between two shades at 5 Hz.
//!
//! It is the gap to **the first craft over that line**, not necessarily the one
//! directly ahead, and the leader's stamp is a whole second (the table stores
//! `(int)clock`), so a gap reads up to a second long.
//!
//! The three titles that have been read share this machine and differ in four
//! places, carried by [`oag_title::lap_hud::SplitGapRule`]: Pulse words the
//! held gap `"- 2.8"`/`"+ 2.8"` and Pure as a sentence, Pulse alone ends a won
//! hold when the player stops being first, HD alone runs the count-up in Time
//! Trial and Speed Lap with no other craft live, and HD gaps three times a lap.
//! `docs/ghidra/functions/{psp-pulse-usa,psp-pure-usa,ps3-hdfury-eu}/hud-message-triggers.md`.

use oag_title::lap_hud::{GapText, SplitGapRule};
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

/// Which of the three readings a line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapKind {
    /// The running count from the player's own crossing.
    Count,
    /// The held gap of a player who crossed first.
    Ahead,
    /// The held gap of a player who crossed second.
    Behind,
}

/// The line to draw: what it says, in what colour, and whether the icon goes
/// with it.
#[derive(Debug, Clone, PartialEq)]
pub struct GapLine {
    /// Which reading this is.
    pub kind: GapKind,
    /// Seconds, as drawn to one decimal.
    pub value: f32,
    /// How this title words it.
    pub words: GapText,
    /// Its colour, also the icon's tint.
    pub color: [f32; 4],
    /// The held gap carries the icon; the running count does not.
    pub icon: bool,
}

impl GapLine {
    /// The text to draw, or `None` where the title's phrase is not in its
    /// table (a raw id on screen reads as a fault).
    #[must_use]
    pub fn text(&self, strings: &oag_ui::language::StringTable) -> Option<String> {
        let number = format!("{:.1}", self.value);
        match (self.kind, self.words) {
            (GapKind::Count, _) => Some(number),
            (GapKind::Ahead, GapText::Signed) => Some(format!("- {number}")),
            (GapKind::Behind, GapText::Signed) => Some(format!("+ {number}")),
            (kind, GapText::Phrases { ahead, behind }) => {
                let id = if kind == GapKind::Ahead {
                    ahead
                } else {
                    behind
                };
                strings
                    .get(id)
                    .map(|phrase| phrase.replacen("%.1f", &number, 1))
            }
        }
    }
}

/// The display state machine (`hud+0x9c..0xb0`).
#[derive(Debug, Clone, PartialEq)]
pub struct TimeGap {
    rule: SplitGapRule,
    leading: bool,
    lead_time: f32,
    holding: bool,
    ahead: bool,
    gap: f32,
    timer: f32,
    line: Option<GapLine>,
}

impl TimeGap {
    /// A display obeying one title's rule.
    #[must_use]
    pub fn new(rule: SplitGapRule) -> Self {
        Self {
            rule,
            leading: false,
            lead_time: 0.0,
            holding: false,
            ahead: false,
            gap: 0.0,
            timer: 0.0,
            line: None,
        }
    }

    /// One tick. `now` is the race clock in seconds, `live` the craft still in
    /// the race, `place` the player's place (1 = first) and `timed` whether the
    /// mode is Time Trial or Speed Lap.
    pub fn advance(
        &mut self,
        dt: f32,
        now: f32,
        events: GapEvents,
        live: usize,
        place: u32,
        timed: bool,
    ) {
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
        let alone_ok = timed && self.rule.count_up_alone_in_timed_modes;
        if self.leading && (live >= 2 || alone_ok) {
            let count = now - self.lead_time;
            if count < SHOW_BELOW {
                self.line = Some(GapLine {
                    kind: GapKind::Count,
                    value: count,
                    words: self.rule.text,
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
        let (kind, argb) = match (self.ahead, flash) {
            (false, false) => (GapKind::Behind, 0xFFC0_1010),
            (false, true) => (GapKind::Behind, 0xFFFF_3030),
            (true, false) => (GapKind::Ahead, 0xFF10_C010),
            (true, true) => (GapKind::Ahead, 0xFF30_FF30),
        };
        if self.ahead && self.rule.green_hold_needs_first_place {
            self.holding = place == 1;
        }
        if -SHOW_BELOW < self.gap && self.gap < SHOW_BELOW {
            self.line = Some(GapLine {
                kind,
                value: self.gap,
                words: self.rule.text,
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
