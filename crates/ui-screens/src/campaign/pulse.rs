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
    fn tick_moves_the_tint() {
        let mut p = SelectorPulse::default();
        let before = p.tint();
        for _ in 0..40 {
            p.tick(1.0 / 60.0);
        }
        assert_ne!(p.tint(), before);
    }
}
