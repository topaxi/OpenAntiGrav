//! `Selector`'s pulse clock: the widget update at `0x088a3e8c` (live on Grid
//! and Cell Selection, PPSSPP breakpoint; its twin `0x088a5700` never runs
//! there) drives the selected hex's colour off a phase advanced
//! `elapsed * 2.0` and wrapped at `2.0`, a 1 s triangle wave between
//! [`CYAN`] and white. The phase starts at `0.0` and runs from the first
//! frame: there is no entry hold. The original keeps the phase in the widget
//! across visits, so a re-entry resumes where it was left; [`SelectorPhases`]
//! carries it between visits, and a cold first entry since boot is `0.0`. See
//! `docs/ui/campaign-screens.md`.

/// The pulse's low endpoint, a literal in `0x088a5700`.
const CYAN: u32 = 0xff33_a6b9;
const WHITE: u32 = 0xffff_ffff;

/// Seconds since the screen opened, owned by the `Grid Selection`/`Cell
/// Selection` model so the draw list needs no clock argument. The law (rate,
/// endpoints, no hold) is `0x088a3e8c`'s.
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
        let phase = (self.seconds * 2.0) % 2.0;
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

    /// `Selector`'s ARGB tint now: `lerp(phase, cyan, white)` for the first
    /// half of the 2.0-wide phase, `lerp(phase - 1, white, cyan)` for the rest.
    #[must_use]
    pub fn tint(&self) -> u32 {
        let phase = (self.seconds * 2.0) % 2.0;
        if phase > 1.0 {
            lerp_argb(phase - 1.0, WHITE, CYAN)
        } else {
            lerp_argb(phase, CYAN, WHITE)
        }
    }
}

/// Where `Grid Selection`'s and `Cell Selection`'s pulses stand, kept by whoever outlives both
/// screens (the front end's session) so a re-entry resumes the phase it left. The original's
/// two `Selector` widgets are built once per boot and keep `+0x9c` while the screen is away
/// (frozen: the update runs but the flags bit `0x200` is clear), so each screen has its own
/// phase and a relaunch is a cold start at `0.0`. Not saved to disk.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectorPhases {
    pub grid: SelectorPulse,
    pub cell: SelectorPulse,
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
        p.set_seconds(0.0);
        assert_eq!(p.tint(), CYAN);
        p.set_seconds(0.02);
        assert_ne!(p.tint(), CYAN, "no entry hold: the rise starts at once");
        p.set_seconds(0.5);
        assert_eq!(p.tint(), WHITE);
        p.set_seconds(1.0);
        assert_eq!(p.tint(), CYAN);
        p.set_seconds(0.25);
        let quarter = p.tint();
        assert!(quarter != CYAN && quarter != WHITE);
        p.set_seconds(0.75);
        assert_eq!(p.tint(), quarter, "the wave is symmetric about 0.5 s");
    }

    #[test]
    fn the_stamp_follows_the_pulse_from_80_to_230() {
        let mut p = SelectorPulse::default();
        assert_eq!(p.stamp(), 80, "cyan: the low end");
        p.set_seconds(0.5);
        assert_eq!(p.stamp(), 230, "white: the high end");
        p.set_seconds(0.25);
        let rising = p.stamp();
        p.set_seconds(0.75);
        assert_eq!(p.stamp(), rising, "the same triangle as the tint");
        assert_eq!(rising, 155);
    }

    #[test]
    fn the_two_screens_keep_separate_phases_and_default_to_cold() {
        let mut phases = SelectorPhases::default();
        assert_eq!(phases.grid.seconds(), 0.0);
        assert_eq!(phases.cell.seconds(), 0.0);
        phases.grid.set_seconds(1.1388 / 2.0);
        assert_eq!(phases.cell.seconds(), 0.0, "Cell's widget is not Grid's");
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
