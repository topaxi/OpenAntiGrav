//! How much of a selection screen's description Wipeout Pure's `TextInfo` has typed
//! `since` seconds after the screen appeared.
//!
//! Measured off PPSSPP v1.20.4 (software renderer, USA disc, frame recordings at 30 fps
//! and stopped-CPU grabs whose clock is the emulator's own tick counter, which ran at
//! 1.0000 of wall time). The line is not typed once: it **loops** - types, holds, clears
//! and types again from its first character.
//!
//! | Quantity | Value | Evidence |
//! | --- | --- | --- |
//! | rate | 58 characters a second | Class (86 characters) and League (135) loop every 4.607 s and 5.441 s over 11 and 10 cycles; the 49-character difference is 0.834 s (58.8 a second), and eight stopped grabs of Class between 0.18 s and 1.32 s fit 57.1 |
//! | hold | 3.12 s from the last character to the clear | the same two periods less their typing time |
//! | start | 0.067 s after the pad press | the mean of the eight grabs; includes the press-to-screen latency, which the grabs cannot separate |
//! | cursor | `_` after the last typed character, solid while typing, then blinking 0.5 s on and 0.5 s off | every partial grab shows it; the blink's phase is not tied to the screen (cycles 2 and 3 of one recording toggle in opposite senses) |
//!
//! The rate is within a frame of one character per 60 Hz frame; the fit prefers 58. The
//! characters are counted as the wrapped text lays them out, a wrap's space included.
//! The authored `TextInfo delay="1.6"` matches none of these and is not used.

/// Characters typed a second.
pub const CHARS_PER_SECOND: f32 = 58.0;

/// Seconds the finished line stays before it clears and types again.
pub const HOLD: f32 = 3.12;

/// Seconds from the screen appearing to the first character.
pub const START: f32 = 0.067;

/// Seconds the cursor is on, and off, once the line is complete.
pub const BLINK_HALF: f32 = 0.5;

/// What the line shows at an instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Typed {
    /// Characters of the wrapped text visible.
    pub chars: usize,
    /// Whether the `_` cursor is drawn after them.
    pub cursor: bool,
}

/// The line of `total` characters `since` seconds after its screen appeared.
///
/// Nothing is shown before [`START`]. The cycle is the typing time plus [`HOLD`], and then
/// begins again with no gap.
#[must_use]
pub fn typed(total: usize, since: f32) -> Typed {
    let nothing = Typed {
        chars: 0,
        cursor: false,
    };
    if total == 0 || since < START {
        return nothing;
    }
    let typing = total as f32 / CHARS_PER_SECOND;
    let phase = (since - START) % (typing + HOLD);
    if phase < typing {
        return Typed {
            chars: (((phase * CHARS_PER_SECOND) as usize) + 1).min(total),
            cursor: true,
        };
    }
    Typed {
        chars: total,
        cursor: ((since / BLINK_HALF) as u32).is_multiple_of(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_typed_before_the_start_delay() {
        assert_eq!(typed(86, 0.0).chars, 0);
        assert_eq!(typed(86, START - 0.001).chars, 0);
        assert!(!typed(86, 0.0).cursor);
    }

    #[test]
    fn the_line_types_at_the_measured_rate_with_a_solid_cursor() {
        let at = |seconds: f32| typed(86, START + seconds);
        assert_eq!(
            at(0.0),
            Typed {
                chars: 1,
                cursor: true
            }
        );
        assert_eq!(at(0.5).chars, 30, "0.5 s at 58 a second, plus the first");
        assert!(at(1.0).cursor);
        assert_eq!(at(1.4).chars, 82);
    }

    #[test]
    fn the_whole_line_holds_and_the_cursor_blinks() {
        let done = START + 86.0 / CHARS_PER_SECOND;
        assert_eq!(typed(86, done + 0.1).chars, 86);
        let flips: Vec<bool> = (0..6)
            .map(|i| typed(86, 1.75 + 0.5 * i as f32).cursor)
            .collect();
        assert_eq!(flips, [false, true, false, true, false, true]);
        assert_eq!(typed(86, done + HOLD - 0.01).chars, 86);
    }

    #[test]
    fn after_the_hold_it_clears_and_types_again_from_the_first_character() {
        let period = START + 86.0 / CHARS_PER_SECOND + HOLD;
        let again = typed(86, period + 0.2);
        assert_eq!(again.chars, 12);
        assert!(again.cursor);
        assert_eq!(
            typed(135, START + 135.0 / CHARS_PER_SECOND + HOLD + 0.001).chars,
            1
        );
    }

    #[test]
    fn a_longer_line_takes_longer_to_come_round() {
        let early = |total| typed(total, 5.0).chars;
        assert_eq!(
            early(86),
            20,
            "Class has come round once by 5 s (4.6 s period)"
        );
        assert_eq!(early(135), 135, "League is still holding its first line");
    }
}
