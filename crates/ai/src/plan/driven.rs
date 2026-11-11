//! What a run from the grid did, tick by tick, so the next run from the grid
//! does not drive again what it would drive identically.
//!
//! A search drives the same craft from the same grid pose many times: the brake
//! calibration's run-up, each learning pass, each verification. A tick of any of
//! them is a pure function of the run's state and of the throttle and brake the
//! plan asked for ([`Course::tick`]), and every run starts in the same state. So
//! by induction a later run repeats an earlier one tick for tick for as long as
//! its plan asks for the same throttle and brake at each recorded tick and
//! answers each failure the same way. [`Driven::keep_for`] finds that stretch by
//! asking the plan again ([`SpeedPlan::follow_at`], no physics), and the later
//! run reads the stretch back and drives on from its last mark.
//!
//! Nothing a build returns changes: the steps a read-back tick would have cost
//! are counted as if driven (`docs/gameplay/ai.md`, "What a build does not
//! drive twice").

use oag_physics::Body;

use super::{CHECK_EVERY, Course, Craft, Failure, Run, SpeedPlan, Tick, Tuning, forward_speed};
use oag_physics::Raycaster;

/// One tick of a run.
#[derive(Debug, Clone, Copy)]
pub(super) struct Lived {
    /// What the plan was asked with: the line index the driver stood on, and
    /// the craft's forward speed and whether it was airborne before the step.
    pub(super) index: u32,
    speed: f32,
    airborne: bool,
    /// The throttle and brake it answered.
    pub(super) asked: (f32, f32),
    pub(super) contact: bool,
    pub(super) failure: Option<Failure>,
    /// Whether the run was put back on the line after this tick.
    pub(super) respawned: bool,
    /// After the step: [`Run::progress`], forward speed, seconds airborne, and
    /// the body, for the clearance test.
    pub(super) progress: i64,
    pub(super) pace: f32,
    pub(super) air: f32,
    pub(super) body: Body,
}

/// A run as it stood after a tick: a place a later run can continue from.
#[derive(Debug, Clone, Copy)]
pub(super) struct Mark {
    pub(super) run: Run,
    /// Whether this is the run as put back on the line after its tick, not as
    /// the tick left it.
    pub(super) respawned: bool,
}

/// Who reads a record back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reader {
    /// A learning pass: it answers any failure in its own way, so it reads
    /// back up to the first.
    Learn,
    /// A verification: a wall is driven through and anything else is put back
    /// on the line, so it reads back through every failure answered so.
    Verify,
}

/// A run from the grid: every tick it kept, and its marks.
///
/// `ticks[k]` is tick `k + 1`. The marks are in tick order: the grid, then the
/// run after each tick that is a multiple of [`CHECK_EVERY`] and before the
/// tick's failure is answered, and the run as put back after a rescue. A
/// learning pass rewinds onto them, so a record a pass reads back has to hold
/// exactly the marks the pass would have made.
#[derive(Debug, Clone, Default)]
pub(super) struct Driven {
    pub(super) ticks: Vec<Lived>,
    pub(super) marks: Vec<Mark>,
}

impl Driven {
    /// Starts a record at `run`, on the grid.
    pub(super) fn begin(&mut self, run: Run) {
        self.ticks.clear();
        self.marks.clear();
        self.marks.push(Mark {
            run,
            respawned: false,
        });
    }

    /// Records a tick and, on a multiple of [`CHECK_EVERY`], its mark.
    pub(super) fn push(&mut self, lived: Lived, run: &Run) {
        self.ticks.push(lived);
        if run.tick.is_multiple_of(CHECK_EVERY) {
            self.marks.push(Mark {
                run: *run,
                respawned: false,
            });
        }
    }

    /// Records that the last tick's run was put back on the line as `run`.
    pub(super) fn respawn(&mut self, run: Run) {
        if let Some(last) = self.ticks.last_mut() {
            last.respawned = true;
        }
        self.marks.push(Mark {
            run,
            respawned: true,
        });
    }

    /// Rewinds to the last mark whose progress is at most `resume`, the grid
    /// at the earliest, and returns its run.
    pub(super) fn rewind(&mut self, resume: i64) -> Run {
        while self.marks.len() > 1
            && self
                .marks
                .last()
                .is_some_and(|mark| mark.run.progress > resume)
        {
            self.marks.pop();
        }
        let mark = *self.marks.last().expect("the grid mark is never popped");
        self.ticks.truncate(mark.run.tick as usize);
        // A tick that was rescued has two marks, and the rewind may have landed
        // on the one from before the rescue.
        if let Some(last) = self.ticks.last_mut() {
            last.respawned = mark.respawned;
        }
        mark.run
    }

    /// Cuts the record to what a run of `plan` at `margin`, with `stuck` for
    /// its [`Run::stuck_limit`], would drive tick for tick from the grid, at
    /// most `cap` ticks and ending on a mark. Returns the run at that mark, or
    /// `None` where there is no record of such a run or the course asks for
    /// every run in full ([`Course::drive_again`]).
    pub(super) fn keep_for<R: Raycaster + ?Sized>(
        &mut self,
        plan: &SpeedPlan,
        course: &Course<'_, R>,
        margin: f32,
        stuck: u32,
        cap: u64,
        reader: Reader,
    ) -> Option<Run> {
        if course.drive_again || self.marks.first()?.run.stuck_limit != stuck {
            return None;
        }
        let dt = course.dt;
        let same = self
            .ticks
            .iter()
            .position(|lived| {
                let asked = plan.follow_at(lived.index, lived.speed, lived.airborne, dt, margin);
                let answered = match lived.failure {
                    None => true,
                    Some(_) if reader == Reader::Learn => false,
                    Some(failure) => failure == Failure::Wall || lived.respawned,
                };
                asked.0.to_bits() != lived.asked.0.to_bits()
                    || asked.1.to_bits() != lived.asked.1.to_bits()
                    || !answered
            })
            .unwrap_or(self.ticks.len());
        let same = (same as u64).min(cap);
        let at = self
            .marks
            .iter()
            .rposition(|mark| u64::from(mark.run.tick) <= same)?;
        self.marks.truncate(at + 1);
        let run = self.marks[at].run;
        self.ticks.truncate(run.tick as usize);
        Some(run)
    }
}

impl<R: Raycaster + ?Sized> Course<'_, R> {
    /// [`Self::tick`] with `ask` deciding throttle and brake, and the tick as
    /// a record keeps it.
    pub(super) fn lived(
        &self,
        run: &mut Run,
        craft: &Craft,
        tuning: &Tuning,
        ceiling: f32,
        ask: impl Fn(&Run) -> (f32, f32),
    ) -> (Tick, Lived) {
        let seen = std::cell::Cell::new((0, 0.0, false, (1.0, 0.0)));
        let tick = self.tick(run, craft, tuning, ceiling, |run, _| {
            let asked = ask(run);
            seen.set((
                run.driver.index,
                forward_speed(&run.state),
                run.state.time_airborne > 0.0,
                asked,
            ));
            asked
        });
        let (index, speed, airborne, asked) = seen.get();
        let lived = Lived {
            index,
            speed,
            airborne,
            asked,
            contact: tick.contact,
            failure: tick.failure,
            respawned: false,
            progress: run.progress,
            pace: forward_speed(&run.state),
            air: run.state.time_airborne,
            body: run.state.body,
        };
        (tick, lived)
    }
}
