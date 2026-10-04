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
//! What the screen does when the load finishes (the whole bar, then the fade)
//! is this build's own, **chosen, not measured**: it is [`super::Screen`]'s.

use oag_title::loading::Progression;

/// The bar's shown count and target, in columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Fill {
    law: Progression,
    shown: f32,
    target: f32,
    rate: f32,
    /// How many load stages have been applied.
    stage: u8,
}

impl Fill {
    /// A bar at its construction state: nothing shown, the first target set.
    pub(super) fn new(law: Progression) -> Self {
        Self {
            law,
            shown: 0.0,
            target: law.start * law.columns,
            rate: law.rate,
            stage: 0,
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
    }

    fn set_target(&mut self, fraction: f32) {
        let target = (fraction * self.law.columns).max(self.target);
        self.target = target;
        if target > self.shown && self.shown > 0.0 {
            self.rate += self.rate;
        }
    }

    /// One frame of easing.
    pub(super) fn step(&mut self) {
        self.shown += self.rate;
        if self.shown > self.target {
            self.shown = self.target;
            self.rate = self.law.rate;
        }
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
        let lit = super::super::bar::lit_columns(fill.fraction());
        assert!((lit - 33.0).abs() < f32::EPSILON, "{lit}");
    }
}
