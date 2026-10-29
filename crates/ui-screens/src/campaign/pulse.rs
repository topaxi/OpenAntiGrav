//! `Selector`'s pulse clock: `GridController_UpdateSelectorPulse`
//! (`0x088a5700`, confidence 85) drives the selected hex's colour off a phase
//! advanced `elapsed * 2.0` and wrapped at `2.0`, a 1 s triangle wave between
//! [`CYAN`] and white. See `docs/ui/campaign-screens.md`.

/// The pulse's low endpoint, a literal in `0x088a5700`.
const CYAN: u32 = 0xff33_a6b9;
const WHITE: u32 = 0xffff_ffff;

/// Seconds after the screen opened that the original holds the selector at
/// cyan before the triangle starts: PPSSPP frames grabbed 0.04-0.9 s after
/// `Grid Selection` replaced `Main Menu` (7 entries, `scripts/psp-menu-pair.py`'s
/// entry clock) read cyan until 0.3-0.4 s, then rise to white by 0.7-0.9 s.
/// Fitted to those frames (+-0.1 s, wall clock), not read from the executable:
/// the entrance animation that holds it is not located. Confidence 50.
const ENTRY_HOLD: f32 = 0.35;

/// Seconds since the screen opened, owned by the `Grid Selection`/`Cell
/// Selection` model so the draw list needs no clock argument. The law (rate,
/// endpoints) is `0x088a5700`'s; [`ENTRY_HOLD`] is the fitted entry delay.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectorPulse {
    seconds: f32,
}

impl SelectorPulse {
    pub fn tick(&mut self, dt: f32) {
        self.seconds += dt.max(0.0);
    }

    pub fn set_seconds(&mut self, seconds: f32) {
        self.seconds = seconds.max(0.0);
    }

    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.seconds
    }

    /// The pulse's position in its triangle, `0.0` at cyan to `1.0` at white: the `t` of [`Self::tint`].
    fn level(&self) -> f32 {
        let phase = ((self.seconds - ENTRY_HOLD).max(0.0) * 2.0) % 2.0;
        if phase > 1.0 { 2.0 - phase } else { phase }
    }

    /// The glow-mask byte `Selector`'s quad stamps now: `80 + 150 * t`, `t` being [`Self::level`].
    ///
    /// **Measured**, not read: 150 EDRAM samples of Grid Selection's alpha over the Selector's
    /// box ran 80 to 229 in steps of 10 per game frame (`150 / 15`), and four GE dumps put the
    /// reference (`0xff9801`, `0xffb101`, `0xff6601`, Cell Selection `0xffb501`) within one of
    /// `80 + 150 t` for the tint each frame drew. The code that issues it is not located
    /// (`FUN_0889812c` stamps `(1 - f) * 150 + 70` for an `Image` with flag `0x200`, 70 to 220,
    /// which is not the range read here).
    #[must_use]
    pub fn stamp(&self) -> u8 {
        (80.0 + 150.0 * self.level()).round().clamp(0.0, 255.0) as u8
    }

    /// The glow-mask byte a pulsing `Text` stamps now: `(220 - s) * multiplier`, `s` a byte that
    /// `Text_Draw` (`0x088ce668`, confidence 85) steps by 15 per `1/20 s` tick, up from 0 to 150 and
    /// back, so the value runs 220 down to 70 and back over one second in 15-unit levels. The
    /// shape and the levels are read (and met in EDRAM: Grid Selection's `GRID 1` read 115 and
    /// 130, both on the ladder). **Chosen, not measured**: the phase. The original's `s` starts at
    /// 0 on the text's first draw but is caught up to the game clock's tick count, so it has no
    /// law against the screen's entry; ours starts at 0 when the screen opens.
    #[must_use]
    pub fn text_stamp(&self, multiplier: f32) -> u8 {
        let step = (self.seconds.max(0.0) * 20.0) as u32 % 20;
        let s = 15 * if step <= 10 { step } else { 20 - step };
        ((220 - s) as f32 * multiplier).clamp(0.0, 255.0) as u8
    }

    /// `Selector`'s ARGB tint now: `lerp(phase, cyan, white)` for the first
    /// half of the 2.0-wide phase, `lerp(phase - 1, white, cyan)` for the rest.
    #[must_use]
    pub fn tint(&self) -> u32 {
        let phase = ((self.seconds - ENTRY_HOLD).max(0.0) * 2.0) % 2.0;
        if phase > 1.0 {
            lerp_argb(phase - 1.0, WHITE, CYAN)
        } else {
            lerp_argb(phase, CYAN, WHITE)
        }
    }
}

fn lerp_argb(t: f32, from: u32, to: u32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let mut out = 0u32;
    for shift in [24u32, 16, 8, 0] {
        let a = ((from >> shift) & 0xff) as f32;
        let b = ((to >> shift) & 0xff) as f32;
        let c = (a + (b - a) * t) as u32 & 0xff;
        out |= c << shift;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tint_is_a_one_second_triangle_between_cyan_and_white() {
        let mut p = SelectorPulse::default();
        assert_eq!(p.tint(), CYAN);
        p.set_seconds(ENTRY_HOLD);
        assert_eq!(p.tint(), CYAN, "held cyan through the entry");
        p.set_seconds(ENTRY_HOLD + 0.5);
        assert_eq!(p.tint(), WHITE);
        p.set_seconds(ENTRY_HOLD + 1.0);
        assert_eq!(p.tint(), CYAN);
        p.set_seconds(ENTRY_HOLD + 0.25);
        let quarter = p.tint();
        assert!(quarter != CYAN && quarter != WHITE);
        p.set_seconds(ENTRY_HOLD + 0.75);
        assert_eq!(p.tint(), quarter, "the wave is symmetric about 0.5 s");
    }

    #[test]
    fn the_stamp_follows_the_pulse_from_80_to_230() {
        let mut p = SelectorPulse::default();
        assert_eq!(p.stamp(), 80, "held at cyan: the low end");
        p.set_seconds(ENTRY_HOLD + 0.5);
        assert_eq!(p.stamp(), 230, "white: the high end");
        p.set_seconds(ENTRY_HOLD + 0.25);
        let rising = p.stamp();
        p.set_seconds(ENTRY_HOLD + 0.75);
        assert_eq!(p.stamp(), rising, "the same triangle as the tint");
        assert_eq!(rising, 155);
    }

    #[test]
    fn a_pulsing_texts_stamp_is_a_220_to_70_ladder_over_one_second() {
        let mut p = SelectorPulse::default();
        assert_eq!(p.text_stamp(1.0), 220, "s starts at 0");
        p.set_seconds(0.5);
        assert_eq!(p.text_stamp(1.0), 70, "s tops out at 150 after ten ticks");
        p.set_seconds(0.25);
        assert_eq!(p.text_stamp(1.0), 145);
        p.set_seconds(0.75);
        assert_eq!(p.text_stamp(1.0), 145, "the way back is the way out");
        p.set_seconds(1.0);
        assert_eq!(p.text_stamp(1.0), 220);
        assert_eq!(p.text_stamp(0.5), 110, "PulseMultiplier scales it");
    }

    #[test]
    fn tick_moves_the_tint() {
        let mut p = SelectorPulse::default();
        let before = p.tint();
        for _ in 0..40 {
            p.tick(1.0 / 60.0);
        }
        assert_ne!(p.tint(), before);
    }
}
