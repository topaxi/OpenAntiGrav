//! Hardening a plan against a driver that takes a little less of it.
//!
//! A plan is learned and verified by a craft taking all of it, and the top
//! levels take a margin under that ([`crate::driver`]'s `plan_margin`, 0.964
//! for a balanced Ace at its lowest commitment). A few units/s less through a
//! corner is a few units of line, and that can be enough to meet what the plan's
//! own craft never did. Pulse `06_Track` RAPIER: an Ace 3 % under the trough
//! plan sat 3-4 units wide at sample 729, took speed pad 4, which the plan's
//! own lap missed, and reached the corner at 779 at 171 units/s against that
//! lap's 155, into the wall. The plan's ceilings there were never lowered, so
//! nothing braked for it.
//!
//! So a ring plan [`SpeedPlan::build`] takes only from the trough search
//! keeps learning at [`MARGIN`] until it verifies clean there as well as at
//! full share. Every other plan is untouched, and a fork route's
//! ([`SpeedPlan::build_within`]) never takes one. See `docs/gameplay/ai.md`,
//! "Where only the trough plan verifies".

use oag_physics::collide::Raycaster;

use super::{Course, Craft, Lowering, MAX_PASSES, Pass, Report, SpeedPlan, TROUGH_WIN, Tuning};

/// The share of the plan the hardening laps drive: under the 0.964 a balanced
/// Ace takes at its lowest commitment (`sqrt(0.93)`). **Chosen, not measured.**
pub(super) const MARGIN: f32 = 0.96;

impl SpeedPlan {
    /// [`Self::build_within`], and with `trough_only` [`Self::build`]'s
    /// hardened trough plan where only the trough search verifies.
    pub(super) fn choose<R: Raycaster + ?Sized>(
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        budget: u64,
        trough_only: bool,
    ) -> (Self, Report) {
        let clean = |r: &Report| r.verify_failures == 0 && r.verify_respawns == 0;
        let (window, window_report) = Self::search(course, craft, tuning, budget, Lowering::Window);
        // The trough plan only ever replaces a clean whole-window one, so a
        // course that one does not verify on (2048 `sol` and its routes, 30,000
        // to 330,000 steps each) is not searched twice - except, with
        // `trough_only`, one that failed on walls alone.
        if !clean(&window_report) && (!trough_only || window_report.verify_respawns > 0) {
            return (window, window_report);
        }
        let (trough, trough_report) =
            Self::search(course, craft, tuning, budget, Lowering::FromTrough);
        let steps = window_report.steps + trough_report.steps;
        if !clean(&window_report) {
            return Self::harden_or(
                course,
                craft,
                tuning,
                budget,
                (window, window_report),
                (trough, trough_report),
            );
        }
        let lap = |r: &Report| u64::from(r.verify_lap_ticks.unwrap_or(u32::MAX));
        // A clear win only: the trough plan's verification lap at least
        // [`TROUGH_WIN`] faster, so a layout within a hair of the old plan keeps
        // it bit for bit.
        let wins = clean(&window_report)
            && clean(&trough_report)
            && lap(&trough_report) * TROUGH_WIN.1 < lap(&window_report) * TROUGH_WIN.0;
        let (plan, mut report) = if wins {
            (trough, trough_report)
        } else {
            (window, window_report)
        };
        report.steps = steps;
        (plan, report)
    }

    /// The trough plan, hardened ([`Self::harden`]), where the whole-window
    /// one did not verify; the whole-window plan if the trough plan did not
    /// verify or would not harden. The report counts both searches' steps and
    /// the hardening's.
    pub(super) fn harden_or<R: Raycaster + ?Sized>(
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        budget: u64,
        window: (Self, Report),
        trough: (Self, Report),
    ) -> (Self, Report) {
        let clean = |r: &Report| r.verify_failures == 0 && r.verify_respawns == 0;
        let (window, window_report) = window;
        let (mut hardened, mut report) = trough;
        let total = report.steps + budget;
        let kept = clean(&report) && hardened.harden(course, craft, tuning, total, &mut report);
        let steps = window_report.steps + report.steps;
        let (plan, mut report) = if kept {
            (hardened, report)
        } else {
            (window, window_report)
        };
        report.steps = steps;
        (plan, report)
    }

    /// Learns `self` further at [`MARGIN`] until a verification at that share
    /// and one at full share both come back clean. Returns whether they did;
    /// `report` carries the full-share verification either way, and every step.
    pub(super) fn harden<R: Raycaster + ?Sized>(
        &mut self,
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        budget: u64,
        report: &mut Report,
    ) -> bool {
        let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
        let clean = |r: &Report| r.verify_failures == 0 && r.verify_respawns == 0;
        for _ in 0..MAX_PASSES {
            if report.steps >= budget {
                break;
            }
            report.passes += 1;
            let pass = Pass {
                budget,
                lowering: Lowering::FromTrough,
                margin: MARGIN,
            };
            self.learn(course, craft, tuning, yaw, report, pass);
            self.verify(course, craft, tuning, yaw, report, MARGIN);
            if !clean(report) {
                continue;
            }
            // Last, so the pace and tight samples the race reads are the
            // full-share lap's, as on every other plan.
            self.verify(course, craft, tuning, yaw, report, 1.0);
            if clean(report) {
                return true;
            }
        }
        // Leave the report on a full-share verification of what was learned.
        self.verify(course, craft, tuning, yaw, report, 1.0);
        false
    }
}
