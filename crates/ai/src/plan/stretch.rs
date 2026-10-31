//! A plan learned on one stretch of its line, not over two laps: a fork
//! route's, which a craft drives only from where it moves onto the route's
//! line to just past the merge.
//!
//! The craft starts a short lead-in before the stretch, already at the speed
//! it carries there in a race, and drives the lead-in on targets the caller
//! pins (the ring plan's, which is what a racing craft follows until it
//! switches). From the stretch's first sample on, the search lowers ceilings
//! as [`SpeedPlan::learn`] always does; outside the stretch nothing is lowered
//! and the ceilings are the seed's. A failure still lowers the samples behind
//! it, so braking for a route's first corners lands in the approach, which is
//! part of the stretch. Verification is one clean run over the stretch: no wall
//! touched, nothing rescued, the end reached. **Ours, chosen, not measured.**

use oag_physics::{Raycaster, ShipState};

use super::lower::Lowering;
use super::{
    Course, Craft, Decel, Failure, MAX_PASSES, NEVER_STUCK, Pass, RESPAWN_SKIP, Report, Run,
    SpeedPlan, TROUGH_WIN, clearance, forward_speed,
};
use crate::{Driver, Line, Tuning};

/// Where on its line a plan is learned.
#[derive(Debug, Clone, PartialEq)]
pub struct Stretch {
    /// The line index the craft starts on, `lead_targets.len()` samples before
    /// [`Self::from`]: the craft's start pose is the caller's ([`Craft::start`]),
    /// its index must be this.
    pub start: usize,
    /// The first line index the search learns: where a racing craft starts
    /// following this plan.
    pub from: usize,
    /// How many samples from [`Self::start`] the run covers, lead-in included.
    pub span: usize,
    /// The targets the lead-in is driven on, one per sample from
    /// [`Self::start`]: what a racing craft follows before it switches.
    pub lead_targets: Vec<f32>,
}

/// What [`SpeedPlan::learn`] needs of a [`Stretch`], resolved against the line.
#[derive(Debug)]
pub(super) struct Mask {
    /// Progress at which the pass ends.
    pub(super) goal: i64,
    /// Progress below which the craft is in the lead-in.
    pub(super) lead: i64,
    /// Per line sample, whether the search may move its ceiling.
    pub(super) lowerable: Vec<bool>,
    /// `(line index, target)` held over every re-derivation.
    pinned: Vec<(usize, f32)>,
}

impl Mask {
    fn of(stretch: &Stretch, n: usize) -> Self {
        let mut lowerable = vec![false; n];
        let lead = stretch.from.wrapping_sub(stretch.start) % n.max(1);
        for k in lead..stretch.span {
            lowerable[(stretch.start + k) % n] = true;
        }
        let pinned = stretch
            .lead_targets
            .iter()
            .enumerate()
            .map(|(k, &target)| ((stretch.start + k) % n, target))
            .collect();
        Self {
            goal: stretch.span as i64,
            lead: lead as i64,
            lowerable,
            pinned,
        }
    }

    /// Puts the lead-in's targets back after a re-derivation.
    pub(super) fn pin(&self, plan: &mut SpeedPlan) {
        for &(index, target) in &self.pinned {
            plan.target[index] = target;
        }
    }
}

/// How far past a wanted index, in samples, a craft passing it may be found
/// ([`SpeedPlan::arrivals`]): a fast craft covers several samples a tick.
const ARRIVAL_WINDOW: usize = 16;

impl SpeedPlan {
    /// Where a craft following this plan on `course` (its own line) from
    /// `craft`'s grid pose is on its flying lap as it passes each of `at`:
    /// the line index it was found on and its state, `None` where it never
    /// passed. Followed at `share` of its targets but on a takeoff's run-up, as
    /// [`Self::verify_at`] follows it: what a racing craft on this plan carries
    /// into a fork ([`Self::build_stretch`]). Also the physics steps it took.
    #[must_use]
    pub fn arrivals<R: Raycaster + ?Sized>(
        &self,
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        share: f32,
        at: &[usize],
    ) -> (Vec<Option<(usize, ShipState)>>, u64) {
        let n = self.len();
        let mut found: Vec<Option<(usize, ShipState)>> = vec![None; at.len()];
        if n < 3 || n != course.line.len() {
            return (found, 0);
        }
        let mut plan = self.clone();
        for (index, target) in plan.target.iter_mut().enumerate() {
            if !course
                .line
                .takeoff_within(index, crate::driver::planned::RUN_UP_REACH)
            {
                *target *= share;
            }
        }
        let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
        let mut run = Self::start_run(craft);
        let mut steps = 0u64;
        let limit = (4 * n as u64).max(4_000) as u32 * 2;
        while run.progress < 2 * n as i64 && run.tick < limit && found.iter().any(Option::is_none) {
            let tick = course.tick(&mut run, craft, tuning, yaw, |run, _| {
                plan.follow(run, course.dt, 1.0)
            });
            steps += 1;
            let index = run.driver.index as usize;
            if run.progress >= n as i64 {
                for (want, slot) in at.iter().zip(found.iter_mut()) {
                    if slot.is_none() && (index + n - want % n) % n < ARRIVAL_WINDOW {
                        *slot = Some((index, run.state));
                    }
                }
            }
            match tick.failure {
                None | Some(Failure::Wall) => {}
                Some(_) => {
                    let to = (index + RESPAWN_SKIP) % n;
                    run = Run {
                        state: (course.respawn)(to),
                        driver: Driver {
                            index: to as u32,
                            ..Driver::default()
                        },
                        progress: run.progress + RESPAWN_SKIP as i64,
                        tick: run.tick,
                        slow: 0,
                        best: run.progress + RESPAWN_SKIP as i64,
                        best_tick: run.tick,
                        stuck_limit: run.stuck_limit,
                        last_position: None,
                    };
                }
            }
        }
        (found, steps)
    }

    /// The braking `craft` measures on `course` from its start, as every
    /// whole-line build calibrates it, and the physics steps it took. For a
    /// stretch's caller, which calibrates where the whole-line build of the
    /// same line would have ([`Self::build_stretch`]).
    #[must_use]
    pub fn calibrate<R: Raycaster + ?Sized>(
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
    ) -> (Decel, u64) {
        let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
        super::brake::calibrate(course, craft, tuning, yaw)
    }

    /// This plan with `decel` as its braking, its targets re-derived.
    #[must_use]
    pub fn with_decel(mut self, decel: Decel) -> Self {
        self.decel = decel;
        self.derive_targets();
        self
    }

    /// This plan carried onto `line`: sample `i` takes the ceiling, run-up
    /// hold, pace and tightness of this plan's sample `map(i)`, and a sample
    /// that maps to none starts unlimited. The braking, yaw and share are
    /// this plan's. For seeding a route's plan with the ring's
    /// ([`Self::build_stretch`]).
    #[must_use]
    pub fn carried_onto(&self, line: &Line, map: impl Fn(usize) -> Option<usize>) -> Self {
        let mut plan = Self::unlimited(line);
        for i in 0..plan.len() {
            if let Some(j) = map(i).filter(|&j| j < self.len()) {
                plan.ceiling[i] = self.ceiling[j];
                plan.hold[i] = self.hold[j];
                plan.pace[i] = self.pace[j];
                plan.tight[i] = self.tight[j];
            }
        }
        plan.decel = self.decel;
        plan.yaw = self.yaw;
        plan.share = self.share;
        plan.derive_targets();
        plan
    }

    /// A plan for `stretch` of `course`, seeded with `seed` (on the same line,
    /// [`Self::carried_onto`]): `seed`'s ceilings outside the stretch and its
    /// braking, the stretch itself searched from unlimited as
    /// [`Self::build_within`] searches a whole line. The same two searches, the
    /// trough one only after a clean whole-window one and kept only on a
    /// [`TROUGH_WIN`] faster run; no hardening. The braking is not calibrated
    /// again: the seed's is the same hull's.
    #[must_use]
    pub fn build_stretch<R: Raycaster + ?Sized>(
        seed: &Self,
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        budget: u64,
        stretch: &Stretch,
    ) -> (Self, Report) {
        let clean = |r: &Report| r.verify_failures == 0 && r.verify_respawns == 0;
        let (window, window_report) = Self::search_stretch(
            seed,
            course,
            craft,
            tuning,
            budget,
            stretch,
            Lowering::Window,
        );
        if !clean(&window_report) {
            return (window, window_report);
        }
        let (trough, trough_report) = Self::search_stretch(
            seed,
            course,
            craft,
            tuning,
            budget,
            stretch,
            Lowering::FromTrough,
        );
        let steps = window_report.steps + trough_report.steps;
        let ticks = |r: &Report| u64::from(r.verify_lap_ticks.unwrap_or(u32::MAX));
        let wins = clean(&trough_report)
            && ticks(&trough_report) * TROUGH_WIN.1 < ticks(&window_report) * TROUGH_WIN.0;
        let (plan, mut report) = if wins {
            (trough, trough_report)
        } else {
            (window, window_report)
        };
        report.steps = steps;
        (plan, report)
    }

    #[allow(clippy::too_many_arguments)]
    fn search_stretch<R: Raycaster + ?Sized>(
        seed: &Self,
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        budget: u64,
        stretch: &Stretch,
        lowering: Lowering,
    ) -> (Self, Report) {
        let mut plan = seed.clone();
        let mut report = Report::default();
        let n = plan.len();
        if n < 3 || n != course.line.len() {
            return (plan, report);
        }
        let mask = Mask::of(stretch, n);
        for i in 0..n {
            if mask.lowerable[i] {
                plan.ceiling[i] = f32::INFINITY;
                plan.hold[i] = false;
            }
        }
        let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
        plan.yaw = Some(yaw);
        plan.share = 1.0;
        plan.derive_targets();
        mask.pin(&mut plan);
        for _ in 0..MAX_PASSES {
            report.passes += 1;
            plan.learn(
                course,
                craft,
                tuning,
                yaw,
                &mut report,
                Pass {
                    budget,
                    stuck: NEVER_STUCK,
                    lowering,
                    margin: 1.0,
                    stretch: Some(&mask),
                },
            );
            let failures = plan.verify_stretch(course, craft, tuning, yaw, &mut report, &mask);
            if failures == 0 || report.steps >= budget {
                break;
            }
        }
        (plan, report)
    }

    /// One run over the stretch following the plan, changing nothing but the
    /// pace and tightness it records there. A failure is a wall touched, a
    /// rescue, or the end not reached, from the stretch's first sample on; the
    /// lead-in's walls are the ring plan's. `verify_lap_ticks` is the ticks
    /// from the stretch's first sample to its end.
    fn verify_stretch<R: Raycaster + ?Sized>(
        &mut self,
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        yaw: f32,
        report: &mut Report,
        mask: &Mask,
    ) -> u32 {
        let n = self.len();
        let mut run = Self::start_run(craft);
        let (mut failures, mut contacts) = (0u32, 0u32);
        let mut lead_mark: Option<u32> = None;
        let mut respawned = false;
        report.verify_lap_ticks = None;
        report.verify_respawns = 0;
        let mut pace = self.pace.clone();
        let mut tight = self.tight.clone();
        for i in 0..n {
            if mask.lowerable[i] {
                pace[i] = f32::INFINITY;
                tight[i] = false;
            }
        }
        let limit = (8 * mask.goal as u64).max(4_000) as u32;
        while run.progress < mask.goal && run.tick < limit {
            let tick = course.tick(&mut run, craft, tuning, yaw, |run, _| {
                self.follow(run, course.dt, 1.0)
            });
            report.steps += 1;
            let index = run.driver.index as usize;
            let past_lead = run.progress >= mask.lead;
            if past_lead && lead_mark.is_none() {
                lead_mark = Some(run.tick);
            }
            if past_lead && mask.lowerable[index] {
                contacts += u32::from(tick.contact);
                pace[index] = forward_speed(&run.state);
                if !tick.contact
                    && run.state.time_airborne <= 0.0
                    && clearance::too_close(
                        &run.state,
                        &craft.handling,
                        course.env.self_collider,
                        course.raycaster,
                    )
                {
                    tight[index] = true;
                }
            }
            let Some(failure) = tick.failure else {
                continue;
            };
            if failure == Failure::Wall {
                failures += u32::from(past_lead);
                continue;
            }
            report.verify_respawns += 1;
            failures += 1;
            respawned = true;
            let to = (index + RESPAWN_SKIP) % n;
            run = Run {
                state: (course.respawn)(to),
                driver: Driver {
                    index: to as u32,
                    ..Driver::default()
                },
                progress: run.progress + RESPAWN_SKIP as i64,
                tick: run.tick,
                slow: 0,
                best: run.progress + RESPAWN_SKIP as i64,
                best_tick: run.tick,
                stuck_limit: run.stuck_limit,
                last_position: None,
            };
        }
        if run.progress < mask.goal {
            failures += 1;
        } else if !respawned {
            report.verify_lap_ticks = lead_mark.map(|mark| run.tick - mark);
        }
        self.pace = pace;
        self.tight = tight;
        report.verify_failures = failures;
        report.verify_contacts = contacts;
        failures
    }
}
