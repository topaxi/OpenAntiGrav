//! A plan learned on one hull, re-learned for another.
//!
//! Every opponent flies its own team's handling, and the grid's hulls differ
//! in width, grip and turn rate: a plan learned on one of them carries another
//! into walls it never touched (Pulse's Triakis, 6.0 wide and the slowest to
//! turn, on a plan learned on the player's craft). Searching every hull from
//! scratch was tried and is both costly and chaotic: the slowest-turning hulls
//! failed to verify on jumps the reference hull clears (`01`, `07`, `13`,
//! `14_Track`). [`SpeedPlan::adapt`] starts from the reference plan instead,
//! keeping its ceilings and its takeoff run-ups, and only lowers where this
//! hull fails. **Ours, chosen, not measured.**

use oag_physics::Raycaster;

use super::lower::Lowering;
use super::{Course, Craft, MAX_PASSES, Pass, Report, STUCK_TICKS, SpeedPlan, brake};
use crate::Tuning;

impl SpeedPlan {
    /// This plan, re-learned for `craft`'s hull within `budget` physics steps.
    ///
    /// The braking is re-measured on this hull and the targets re-derived,
    /// then the plan is verified as it stands: a hull that laps it clean takes
    /// it with nothing lowered. Otherwise up to [`MAX_PASSES`] whole-window
    /// learning passes lower the ceilings where this hull fails, each followed
    /// by a verification. The caller decides what an unclean result means; the
    /// race checks the reference plan on this hull ([`Self::verify_at`]) and
    /// follows it at the share that laps.
    #[must_use]
    pub fn adapt<R: Raycaster + ?Sized>(
        &self,
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        budget: u64,
    ) -> (Self, Report) {
        let mut plan = self.clone();
        let mut report = Report::default();
        if plan.len() != course.line.len() || plan.len() < 3 {
            return (plan, report);
        }
        let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
        let (decel, steps) = brake::calibrate(course, craft, tuning, yaw);
        report.steps += steps;
        plan.decel = decel;
        plan.yaw = Some(yaw);
        plan.derive_targets();
        let mut failures =
            plan.verify_stuck(course, craft, tuning, yaw, &mut report, 1.0, STUCK_TICKS);
        while failures > 0 && report.passes < MAX_PASSES && report.steps < budget {
            report.passes += 1;
            plan.learn(
                course,
                craft,
                tuning,
                yaw,
                &mut report,
                Pass {
                    budget,
                    stuck: STUCK_TICKS,
                    lowering: Lowering::Window,
                    margin: 1.0,
                    stretch: None,
                },
            );
            failures = plan.verify_stuck(course, craft, tuning, yaw, &mut report, 1.0, STUCK_TICKS);
        }
        (plan, report)
    }

    /// Two laps from `craft`'s grid pose following this plan **as it stands**,
    /// at [`Self::share_for`] its hull's yaw ceiling times `share`, nothing
    /// learned: what a hull that follows this plan unadapted will meet. The
    /// verification [`Self::adapt`] runs on its own copy, here on the plan a
    /// race falls back to when that copy does not verify.
    ///
    /// The share is left off a takeoff's run-up, as `Driver::drive` leaves it
    /// off, so the verification flies what the race will. **Chosen, not
    /// measured.**
    #[must_use]
    pub fn verify_at<R: Raycaster + ?Sized>(
        &self,
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        share: f32,
    ) -> Report {
        let mut plan = self.clone();
        let mut report = Report::default();
        if plan.len() != course.line.len() || plan.len() < 3 {
            return report;
        }
        let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
        let margin = plan.share_for(Some(yaw)) * share;
        for (index, target) in plan.target.iter_mut().enumerate() {
            if !course
                .line
                .takeoff_within(index, crate::driver::planned::RUN_UP_REACH)
            {
                *target *= margin;
            }
        }
        plan.verify_stuck(course, craft, tuning, yaw, &mut report, 1.0, STUCK_TICKS);
        report
    }
}
