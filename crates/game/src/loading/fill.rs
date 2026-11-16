//! How a stage-driven progression bar fills.
//!
//! Wipeout HD's bar does not follow a clock. The screen is built with a first
//! target (`0x002b9a28`); the loader calls `LoadingScreen_SetProgressTarget`
//! (`0x002b2c60`) at each stage; and once a frame the shown column count moves
//! toward the target (`0x002b6c88`). Read off the disassembly, confidence 80:
//!
//! - **Setting a target** stores `fraction * columns`, and **doubles the rate**
//!   when the target is ahead of the shown count and the shown count is above
//!   the zero constant at TOC `+0x60d4`, so a bar that has fallen behind the
//!   loader catches up faster with each milestone.
//! - **A frame** adds the rate to the shown count; past the target it clamps
//!   to the target and the rate returns to its base (`obj+0x110`, `0.1`).
//!
//! The shown count is a float and the draw lights `(int)` of it columns.
//!
//! Wipeout 2048 runs the same class with three differences, which are the title's
//! [`oag_title::loading::Rules`]: a raised target rescales the rate by the gap instead of
//! doubling it, a step lasts 1/30 s, and the last stage freezes the rate reached as the one a
//! clamp returns to (`docs/ghidra/functions/vita-2048-eu-v104/race-load.md`, "The meter's law").
//!
//! What the screen does when the load finishes (the whole bar, then the fade)
//! is this build's own, **chosen, not measured**: it is [`super::Screen`]'s.

use oag_title::loading::{Progression, Raise};

/// The bar's shown count and target, in columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Fill {
    law: Progression,
    shown: f32,
    target: f32,
    rate: f32,
    /// What the rate returns to on reaching the target.
    reset: f32,
    /// How many load stages have been applied.
    stage: u8,
    /// Whether the rate has been frozen as the reset rate.
    frozen: bool,
    /// Whether the load has reported the stage after the last milestone.
    done: bool,
    /// Frames into the current step.
    phase: u8,
}

impl Fill {
    /// A bar at its construction state: nothing shown, the first target set.
    pub(super) fn new(law: Progression) -> Self {
        Self {
            law,
            shown: 0.0,
            target: law.start * law.columns,
            rate: law.rate,
            reset: law.rate,
            stage: 0,
            frozen: false,
            done: false,
            phase: 0,
        }
    }

    /// Applies every stage up to `reached` that has not been applied, in
    /// order, as the loader's own calls would have. A stage already applied is
    /// ignored, so this is safe to call every frame, and a target never goes
    /// back.
    pub(super) fn reach(&mut self, reached: u8) {
        while self.stage < reached {
            let Some(&fraction) = self.law.milestones.get(usize::from(self.stage)) else {
                break;
            };
            self.stage += 1;
            self.set_target(fraction);
        }
        if usize::from(reached) > self.law.milestones.len() {
            self.done = true;
            self.freeze_rate();
        }
    }

    fn set_target(&mut self, fraction: f32) {
        let old_gap = self.target - self.shown;
        let target = (fraction * self.law.columns).max(self.target);
        self.target = target;
        match self.law.rules.raise {
            Raise::Double => {
                if target > self.shown && self.shown > 0.0 {
                    self.rate += self.rate;
                }
            }
            Raise::Rescale => {
                let new_gap = target - self.shown;
                if old_gap > 0.0 && self.shown > 0.0 && new_gap > 0.0 {
                    self.rate *= (old_gap + new_gap) / new_gap;
                }
            }
        }
    }

    /// The race manager's constructor: the rate reached is the one a clamp returns to.
    /// Once, and only on a title whose rules say so.
    fn freeze_rate(&mut self) {
        if self.law.rules.freezes_rate && !self.frozen {
            self.frozen = true;
            self.reset = self.rate;
        }
    }

    /// Whether every stage the title lists has been reached and one more: the load is done
    /// building, which is when the original's race manager exists.
    pub(super) fn stages_done(&self) -> bool {
        self.done
    }

    /// One frame of the screen's own 60 Hz; a step is `frames_per_step` of them.
    pub(super) fn step(&mut self) {
        let first = self.phase == 0;
        self.phase = (self.phase + 1) % self.law.rules.frames_per_step.max(1);
        if !first {
            return;
        }
        self.shown += self.rate;
        if self.shown > self.target {
            self.shown = self.target;
            self.rate = self.reset;
        }
    }

    /// The shown count as the fraction of the bar the original's meter draws with.
    pub(super) fn exact_fraction(&self) -> f32 {
        self.shown / self.law.columns
    }

    /// The readout, `(int)(shown / columns * 100)`.
    pub(super) fn percent(&self) -> u32 {
        (self.exact_fraction() * 100.0) as u32
    }

    /// The bar as a fraction, for [`super::bar::lit_columns`]: the middle of
    /// the lit column's own width, so the floor lands on the shown count's
    /// integer part whatever the rounding.
    pub(super) fn fraction(&self) -> f32 {
        (self.shown.floor() + 0.5) / self.law.columns
    }

    /// The shown count in columns, for tests.
    #[cfg(test)]
    pub(super) fn shown(&self) -> f32 {
        self.shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAW: Progression = Progression {
        start: 0.2,
        milestones: &[0.4, 0.5, 0.75, 0.95],
        rate: 0.1,
        columns: 166.0,
        rules: oag_title::loading::Rules::HD,
    };

    #[test]
    fn the_bar_eases_a_tenth_of_a_column_a_frame_toward_a_first_target_of_a_fifth() {
        let mut fill = Fill::new(LAW);
        for _ in 0..100 {
            fill.step();
        }
        assert!((fill.shown() - 10.0).abs() < 1e-3, "{}", fill.shown());
        for _ in 0..10_000 {
            fill.step();
        }
        assert!((fill.shown() - 33.2).abs() < 1e-3, "clamped at 20 %");
    }

    #[test]
    fn a_milestone_ahead_of_a_started_bar_doubles_the_rate_and_catching_up_resets_it() {
        let mut fill = Fill::new(LAW);
        fill.reach(1);
        fill.step();
        assert!(
            (fill.shown() - 0.1).abs() < 1e-6,
            "nothing is shown when the first milestone lands, so nothing doubles"
        );
        fill.step();
        fill.reach(2);
        fill.step();
        assert!((fill.shown() - 0.4).abs() < 1e-5, "0.1 + 0.1 + 0.2");
        fill.reach(3);
        fill.step();
        assert!(
            (fill.shown() - 0.8).abs() < 1e-5,
            "the rate doubled again, to 0.4"
        );
        for _ in 0..10_000 {
            fill.step();
        }
        assert!((fill.shown() - 0.75 * 166.0).abs() < 1e-3);
        assert!((fill.rate - 0.1).abs() < 1e-6, "back to the base rate");
    }

    #[test]
    fn a_stage_is_applied_once_and_the_target_never_goes_back() {
        let mut fill = Fill::new(LAW);
        fill.reach(4);
        fill.reach(2);
        fill.reach(4);
        assert!((fill.target - 0.95 * 166.0).abs() < 1e-3);
        assert_eq!(fill.stage, 4);
        fill.reach(9);
        assert_eq!(fill.stage, 4, "past the table's end nothing is invented");
    }

    #[test]
    fn the_lit_count_is_the_integer_part_of_the_shown_columns() {
        let mut fill = Fill::new(LAW);
        fill.shown = 33.0;
        let lit = (fill.fraction() * 166.0).floor();
        assert!((lit - 33.0).abs() < f32::EPSILON, "{lit}");
    }

    /// Wipeout 2048's race load: ceiling 57 % at the start, three raises, a frozen rate
    /// (`docs/ghidra/functions/vita-2048-eu-v104/race-load.md`, "The meter's law").
    const VITA: Progression = Progression {
        start: 94.62 / 166.0,
        milestones: &[0.75, 0.95, 0.98],
        rate: 0.115,
        columns: 166.0,
        rules: oag_title::loading::Rules {
            raise: Raise::Rescale,
            frames_per_step: 2,
            freezes_rate: true,
        },
    };

    #[test]
    fn a_raised_ceiling_rescales_the_rate_to_the_three_rates_the_original_was_read_at() {
        let mut fill = Fill::new(VITA);
        assert!((fill.rate - 0.115).abs() < 1e-6);
        // The live table: the shown count at each of the three calls, and the rate read after.
        for (shown, stage, rate) in [
            (4.140, 1, 0.20145),
            (9.982, 2, 0.35762),
            (17.135, 3, 0.70301),
        ] {
            fill.shown = shown;
            fill.reach(stage);
            assert!(
                (fill.rate - rate).abs() < 2e-5,
                "stage {stage}: {} vs {rate}",
                fill.rate
            );
        }
        assert!((fill.target - 0.98 * 166.0).abs() < 1e-3, "98 %");
    }

    #[test]
    fn the_second_run_rates_reproduce_too() {
        let mut fill = Fill::new(VITA);
        for (shown, stage, rate) in [
            (3.565, 1, 0.20159),
            (7.194, 2, 0.35871),
            (11.139, 3, 0.70562),
        ] {
            fill.shown = shown;
            fill.reach(stage);
            assert!(
                (fill.rate - rate).abs() < 2e-5,
                "stage {stage}: {} vs {rate}",
                fill.rate
            );
        }
    }

    #[test]
    fn a_step_lasts_two_frames_and_the_last_stage_freezes_the_rate_it_reached() {
        let mut fill = Fill::new(VITA);
        fill.step();
        fill.step();
        assert!(
            (fill.shown() - 0.115).abs() < 1e-6,
            "one step in two frames"
        );
        fill.reach(3);
        let reached = fill.rate;
        assert!(
            (fill.reset - 0.115).abs() < 1e-6,
            "not frozen before the last stage"
        );
        fill.reach(4);
        assert!(
            (fill.reset - reached).abs() < 1e-9,
            "frozen at the rate reached"
        );
        fill.reach(4);
        fill.rate = 9.0;
        fill.reach(4);
        assert!((fill.reset - reached).abs() < 1e-9, "frozen once");
        for _ in 0..100_000 {
            fill.step();
        }
        assert!(
            (fill.shown() - 0.98 * 166.0).abs() < 1e-3,
            "waits at 98 % until the load is done"
        );
        assert_eq!(fill.percent(), 98);
        assert!((fill.rate - reached).abs() < 1e-9, "and keeps its rate");
    }

    #[test]
    fn hd_does_not_freeze_and_doubles() {
        let mut fill = Fill::new(LAW);
        fill.reach(5);
        assert!((fill.reset - LAW.rate).abs() < 1e-9);
    }

    #[test]
    fn the_readout_is_the_integer_part_of_the_percentage() {
        let mut fill = Fill::new(VITA);
        fill.shown = 0.0;
        assert_eq!(fill.percent(), 0);
        fill.shown = 166.0 * 0.0899;
        assert_eq!(fill.percent(), 8);
        fill.shown = 0.98 * 166.0;
        assert_eq!(fill.percent(), 98);
    }
}
