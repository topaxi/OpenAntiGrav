//! The in-race message slots: `Info1` to `Info4`, the four lines the original
//! flashes down the middle of the screen for an event ("personal lap",
//! "new lap record", "perfect zone", ...).
//!
//! **The presentation is decompiled, the triggers are not.** The slot logic is
//! `Hud_UpdateMessages` (`FUN_0881f148`, `0x0881f148`, `psp-pulse-usa`), read in
//! Ghidra on 2026-10-06: a flag word at `*(hud+0x3c)+0xe0` is drained once a
//! tick, each set bit pushing one string id through `FUN_0881b52c` into the
//! first free of four slots (`hud+0x128`, sixteen bytes apiece, text widgets
//! `Info1`..`Info4` bound by `Hud_BindWidgets`). The bits it names are
//! `IG_HUD_PLAP`, `IG_HUD_NEWLAP_REC`, `IG_HUD_FIN_LAP`, `IG_HUD_CONT_ELIM`,
//! `IG_HUD_PERF_ZONE`, `IG_HUD_PERF_BOOST`, `IG_HUD_NEW_ZONE_RECORD` and
//! `IG_HUD_NEW_SCORE_RECORD`. **None of them is a medal**, and nothing on the
//! disc that evaluates a medal (`Cell_EvaluateMedal`, `0x088bf620`) is called
//! from the race HUD - see `docs/ui/hud.md`'s "Medal messages".
//!
//! What the slot does once it holds a message, all of it read off that decompile
//! and **not yet seen on a live frame**:
//!
//! - A slot waits until the shared gate (`hud+0x124`) is below zero, then shows
//!   and sets the gate to `0.8` s, so two messages raised on one tick appear
//!   0.8 s apart rather than together. The gate counts down only while it is
//!   zero or above.
//! - A shown message lives `4.0` s (`8.0` for game modes from `0xe`, which are
//!   not Pulse's).
//! - Its alpha is `sin(pi * (1 - (t / life)^6))`, `t` the time it has left: up to
//!   full in the first half second, then a long slow fade. The border follows.
//! - Green (`0x30ff30`) for a good message, red (`0xff3030`) for a bad one -
//!   `FUN_0881b52c`'s third argument, `1` for everything but `IG_HUD_CONT_ELIM`'s
//!   second use.
//! - The original also plays the `MESSAGE` cue as it shows. **Not played here**:
//!   nothing in this crate reaches a mixer, and [`MessageBoard::advance`]
//!   reports a show through [`MessageBoard::just_shown`] so the host can.
//!
//! Which tick a message is raised on is the host's business, and the one trigger
//! this build raises - a medal earned mid-race - is **chosen, not measured**.

use oag_ui::screen::argb_to_rgba;

/// How many lines the HUD holds at once: `Info1` to `Info4`.
pub const SLOTS: usize = 4;

/// The `Info1`..`Info4` widget names, in slot order.
pub const WIDGETS: [&str; SLOTS] = ["Info1", "Info2", "Info3", "Info4"];

/// Seconds a message lives once shown.
pub const LIFE_SECONDS: f32 = 4.0;

/// The pause between one message showing and the next: `0x3f4ccccd`.
pub const GATE_SECONDS: f32 = 0.8;

/// The tick length the board advances by.
const TICK: f32 = 1.0 / 60.0;

/// Which slot a widget name is, if it is one.
#[must_use]
pub fn slot_of(name: &str) -> Option<usize> {
    WIDGETS.iter().position(|widget| *widget == name)
}

/// A slot's colour, alpha included, or `None` for any other widget.
#[must_use]
pub(super) fn colour(readout: &super::Readout, name: &str) -> Option<[f32; 4]> {
    readout.messages[slot_of(name)?]
        .as_ref()
        .map(|line| line.color)
}

/// The outline of a slot's text, its alpha following the line's own
/// (`FUN_088cd3a4`); every other widget keeps `border` as it was.
#[must_use]
pub(super) fn fade_border(readout: &super::Readout, name: &str, border: [f32; 4]) -> [f32; 4] {
    match colour(readout, name) {
        Some(color) => [border[0], border[1], border[2], border[3] * color[3]],
        None => border,
    }
}

/// A good message's colour, `0x30ff30` - before the alpha is applied.
const GOOD: u32 = 0xFF30_FF30;
/// A bad message's colour, `0xff3030`.
const BAD: u32 = 0xFFFF_3030;

/// One line, ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageLine {
    /// What it says: a language-table id, resolved when drawn. An id the table
    /// lacks draws nothing - a raw id on screen reads as a fault.
    pub text: String,
    /// Its colour, alpha included. The border takes the same alpha.
    pub color: [f32; 4],
}

/// What the HUD shows of the board this frame: one entry per slot.
pub type Lines = [Option<MessageLine>; SLOTS];

#[derive(Debug, Clone, PartialEq)]
struct Slot {
    text: String,
    good: bool,
    /// `None` while the slot waits for the gate, else the seconds left.
    left: Option<f32>,
}

/// The four slots and the gate between them.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageBoard {
    slots: [Option<Slot>; SLOTS],
    gate: f32,
    shown: bool,
    standing: Option<Standing>,
}

/// A line that stays for the rest of the race, in the last slot.
///
/// **Chosen, not measured.** The original's slots all expire; this one holds the
/// best medal a campaign race has earned so far (`RaceStage` raises it), so the
/// player can still tell later which medal is safe once its four-second banner
/// is gone. It takes the last of the four widgets the layouts already author
/// (`Info4`), so it lands where no other widget of any layout is - and a
/// transient message that has reached that slot hides it until it expires.
#[derive(Debug, Clone, PartialEq)]
struct Standing {
    text: String,
    /// Opaque RGB; the line is drawn at full alpha.
    rgb: [f32; 3],
}

impl Default for MessageBoard {
    fn default() -> Self {
        Self {
            slots: [None, None, None, None],
            // Below zero: the first message shows at once.
            gate: -1.0,
            shown: false,
            standing: None,
        }
    }
}

impl MessageBoard {
    /// Raises a message: the first free slot takes it, and a board with all
    /// four busy drops it, as `FUN_0881b52c` does.
    pub fn push(&mut self, id: impl Into<String>, good: bool) {
        // A standing line keeps the last slot for itself while nothing else
        // needs it, so a transient message queues in the first three.
        let usable = if self.standing.is_some() {
            SLOTS - 1
        } else {
            SLOTS
        };
        if let Some(slot) = self.slots[..usable].iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(Slot {
                text: id.into(),
                good,
                left: None,
            });
        }
    }

    /// Sets the line that stays up for the rest of the race, replacing any
    /// earlier one: `id` is a language-table id and `rgb` its colour.
    pub fn set_standing(&mut self, id: impl Into<String>, rgb: [f32; 3]) {
        self.standing = Some(Standing {
            text: id.into(),
            rgb,
        });
    }

    /// Whether a message started showing on the last [`Self::advance`] - the
    /// moment the original plays its `MESSAGE` cue.
    #[must_use]
    pub fn just_shown(&self) -> bool {
        self.shown
    }

    /// One tick (1/60 s) of the slot loop.
    pub fn advance(&mut self) {
        self.shown = false;
        let mut gate_open = self.gate < 0.0;
        if !gate_open {
            self.gate -= TICK;
        }
        for slot in &mut self.slots {
            let Some(held) = slot else { continue };
            match &mut held.left {
                // Waiting: shows only when the gate is open, and closes it.
                None => {
                    if gate_open {
                        held.left = Some(LIFE_SECONDS);
                        self.gate = GATE_SECONDS;
                        gate_open = false;
                        self.shown = true;
                    }
                }
                Some(left) if *left < 0.0 => *slot = None,
                Some(left) => *left -= TICK,
            }
        }
    }

    /// The lines to draw now. A slot that has just been shown is at
    /// alpha zero, which is what the original's curve gives at its first tick.
    #[must_use]
    pub fn lines(&self) -> Lines {
        let mut out: Lines = [None, None, None, None];
        for (line, slot) in out.iter_mut().zip(&self.slots) {
            let Some(held) = slot else { continue };
            let Some(left) = held.left else { continue };
            let ratio = (left / LIFE_SECONDS).clamp(0.0, 1.0);
            let ramp = (1.0 - ratio.powi(6)).max(0.0);
            let alpha = (ramp * std::f32::consts::PI).sin().max(0.0);
            let mut color = argb_to_rgba(if held.good { GOOD } else { BAD });
            color[3] = alpha;
            *line = Some(MessageLine {
                text: held.text.clone(),
                color,
            });
        }
        // Not while the same words are already up as a banner: the line would
        // read twice, one above the other.
        let repeated = |text: &str| out.iter().flatten().any(|line| line.text == text);
        if let Some(standing) = self.standing.as_ref().filter(|s| !repeated(&s.text))
            && let last @ None = &mut out[SLOTS - 1]
        {
            *last = Some(MessageLine {
                text: standing.text.clone(),
                color: [standing.rgb[0], standing.rgb[1], standing.rgb[2], 1.0],
            });
        }
        out
    }
}

#[cfg(test)]
mod tests;
