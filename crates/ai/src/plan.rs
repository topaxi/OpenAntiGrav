//! A speed plan: the fastest speed this craft can carry at each point of the
//! racing line, found by driving the line in our own physics.
//!
//! The driver's own target (`driver::pace::corner_target`) is a model, and every
//! gap between it and the physics is a wall the craft finds: a crest read as a
//! corner, an apex the chord averages away, a braking zone that starts late.
//! [`SpeedPlan::build`] instead drives the authored line with the race's own
//! steering controller and `oag_physics::step` and **learns** a ceiling per
//! sample: where the craft touches a wall or leaves its line, the ceilings over
//! the stretch that led there are lowered, a backward braking pass re-derives
//! where braking must start, and the run rewinds to a checkpoint before that
//! zone. What survives is a profile the craft has been *seen* to drive cleanly.
//!
//! Nothing is invented: line, collision and handling come off the disc, speeds
//! from our own physics. The search's own knobs below are all **chosen, not
//! measured**. **It is not the original's behaviour**, which schedules speed on
//! the player's position (`docs/gameplay/ai.md`, "Speed, in Pulse") and which
//! this project refuses to port; the plan uses nothing a player's craft lacks.
//!
//! # Determinism
//!
//! `f32` only, no `mul_add`, no hashed containers, no clock, no randomness: a
//! pure function of its inputs. The cross-platform gate is
//! `crates/ai/tests/determinism.rs`'s `the_speed_plan_matches_the_committed_reference`
//! (the plan learned on `probe::circuit` hashed against a committed constant on
//! all three CI platforms); `plan/tests.rs` only checks two builds in one
//! process agree.

use oag_physics::{CraftState, Environment, Handling, Raycaster, ShipControls, ShipState};

use std::collections::BTreeMap;

use crate::{Context, Driver, Line, Tuning};

mod brake;
pub mod probe;
#[cfg(test)]
mod tests;

pub use brake::Decel;

/// Where the plan is driven: the line, what is under it, and how the world
/// reads to the physics.
#[allow(missing_debug_implementations)]
pub struct Course<'a, R: Raycaster + ?Sized> {
    pub line: &'a Line,
    /// The track sample under each line point, **index-parallel to `line`**, or
    /// empty for a course with no magstrips to hold.
    ///
    /// The caller maps it: on a real circuit the line is a permutation of the
    /// spline (`ai_order`), and the spline's own order would put every craft on
    /// the wrong track piece.
    pub samples: &'a [Option<oag_physics::TrackSample>],
    pub raycaster: &'a R,
    /// Everything else the force law reads: class gravity, damage rules. The
    /// track samples in it are overwritten per tick.
    pub env: Environment,
    /// The fixed timestep. ADR-0007.
    pub dt: f32,
    /// How far from its own line point a craft may get before the plan treats
    /// it as having left the circuit. The race passes its own rescue distance.
    pub off_line: f32,
    /// A fresh craft placed on the line at an index, as the race's own rescue
    /// places one. See [`SpeedPlan::learn`] for when the search uses it.
    pub respawn: &'a dyn Fn(usize) -> ShipState,
    /// Whether the craft touched a reset volume this tick, given its state, the
    /// environment and where it started the tick (the race passes
    /// `oag_physics::reset::contact`).
    pub reset: &'a dyn Fn(&ShipState, &Environment, oag_core::math::Vec3) -> bool,
    /// Seconds without a hover contact after which the race rescues a craft.
    pub max_airborne: f32,
    /// The push direction of the speed pad the craft is inside, given where it
    /// started the previous tick and this one: the race's own pad test, so the
    /// plan is learned with the boosts the race gives.
    pub pad: &'a dyn Fn(
        Option<oag_core::math::Vec3>,
        oag_core::math::Vec3,
    ) -> Option<oag_core::math::Vec3>,
}

/// The craft the plan is for, and where it starts.
#[derive(Debug, Clone, Copy)]
pub struct Craft {
    pub handling: Handling,
    pub start: ShipState,
    pub start_index: u32,
}

/// The fastest speed to carry at each line sample, braking zones included.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeedPlan {
    /// What the search learned the craft can carry *at* each sample.
    ceiling: Vec<f32>,
    /// `ceiling` with the braking zones added: the speed at each sample from
    /// which every ceiling ahead can still be met. What a driver follows.
    target: Vec<f32>,
    /// Samples the backward pass may not brake through, because a takeoff
    /// ahead needs its run-up. See [`SpeedPlan::build`].
    hold: Vec<bool>,
    /// Distance along the line from each sample to the next.
    spacing: Vec<f32>,
    /// The forward speed the last verification run did at each sample on its
    /// flying lap, or infinity where it never stood: what `Tuning::pace_share`
    /// is a share of.
    pace: Vec<f32>,
    /// The braking the backward pass was built with.
    decel: Decel,
}

/// What a build did, for a loader report and for the tests.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// Physics steps the whole build took, calibration and every rewind
    /// included. Multiply by the per-step cost for a build time.
    pub steps: u64,
    pub passes: u32,
    pub lowerings: u32,
    /// Line samples where the search gave up: the craft still failed there
    /// after [`MAX_TRIES`] lowerings. Each is a corner the plan cannot promise.
    pub unresolved: Vec<(u32, Failure)>,
    /// Times the search put the craft back on the line past a spot no speed
    /// could get it through - see [`SpeedPlan::learn`]'s doc.
    pub respawns: u32,
    /// Failure ticks on the final, non-learning verification lap pair.
    pub verify_failures: u32,
    pub verify_contacts: u32,
    /// Times the verification run was put back on the line, either lap.
    pub verify_respawns: u32,
    /// Ticks the verification run's second lap took, when it completed one
    /// without being put back.
    pub verify_lap_ticks: Option<u32>,
}

/// How much a failure lowers the ceilings behind it, as a fraction of the speed
/// there. **Chosen, not measured**: small enough to stay near the limit, large
/// enough that a corner settles in a handful of tries.
const LOWER: f32 = 0.96;

/// Seconds in the air past which a wall contact is a fall, not a hop.
/// **Chosen, not measured**: `06_Track`'s gap at samples 1196-1200 falls over a
/// second, a crest hop lands inside a quarter.
const FELL_SECONDS: f32 = 0.5;

/// How far past a spot no speed gets through the search puts the craft back
/// down, in samples. **Chosen, not measured.**
const RESPAWN_SKIP: usize = 32;

/// How far back, in ticks of travel, a failure's lowering reaches. About the
/// length of a corner entry. **Chosen, not measured.**
const LOOKBACK_TICKS: usize = 45;

/// How many lowerings one place on the line gets before the search accepts it
/// as unresolved and drives on. **Chosen, not measured.**
pub const MAX_TRIES: u32 = 14;

/// How wide "one place" is, in samples, for counting tries. **Chosen, not
/// measured.**
const SITE: u32 = 24;

/// Ticks between checkpoints. **Chosen, not measured.**
const CHECK_EVERY: u32 = 15;

/// Learning passes before the build stops whatever the verification says.
/// **Chosen, not measured.**
const MAX_PASSES: u32 = 4;

/// How far ahead, in ticks of travel, the follower reads the plan, for the
/// airbrakes' ramp. **Chosen, not measured.**
pub const LEAD_TICKS: f32 = 6.0;

/// How hard the follower brakes per unit of fractional overspeed. **Chosen,
/// not measured.**
const BRAKE_GAIN: f32 = 25.0;

/// Ticks a craft may sit below [`STALL_SPEED`] before the plan calls it
/// stalled. **Chosen, not measured.**
const STALL_TICKS: u32 = 120;

/// Forward speed under which a craft counts as stopped. **Chosen.**
const STALL_SPEED: f32 = 4.0;

/// Ticks from the grid before a stall can be called, for the standing start.
const STALL_GRACE: u32 = 240;

/// A target past this is no target (no class reaches it, pad boost included),
/// so the backward pass does not carry a braking zone round the ring.
/// **Chosen, not measured.**
const UNLIMITED: f32 = 1_000.0;

/// A hard budget on a build, in physics steps, so a course nothing converges
/// on still returns. **Chosen, not measured**: about thirty laps.
const STEP_BUDGET: u64 = 120_000;

impl SpeedPlan {
    /// A plan that never asks for less than full throttle.
    #[must_use]
    pub fn unlimited(line: &Line) -> Self {
        let n = line.len();
        Self {
            ceiling: vec![f32::INFINITY; n],
            target: vec![f32::INFINITY; n],
            hold: vec![false; n],
            spacing: spacing_of(line),
            pace: vec![f32::INFINITY; n],
            decel: Decel::FALLBACK,
        }
    }

    /// Samples in the plan; the same as the line's.
    #[must_use]
    pub fn len(&self) -> usize {
        self.target.len()
    }

    /// Whether the plan is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.target.is_empty()
    }

    /// The speed to be doing at `index`, braking zones included.
    #[must_use]
    pub fn target(&self, index: usize) -> f32 {
        if self.target.is_empty() {
            return f32::INFINITY;
        }
        self.target[index % self.target.len()]
    }

    /// The learned ceiling at `index`, before braking zones.
    #[must_use]
    pub fn ceiling(&self, index: usize) -> f32 {
        if self.ceiling.is_empty() {
            return f32::INFINITY;
        }
        self.ceiling[index % self.ceiling.len()]
    }

    /// Whether `index` is a takeoff's run-up, which nothing brakes through.
    #[must_use]
    pub fn holds(&self, index: usize) -> bool {
        !self.hold.is_empty() && self.hold[index % self.hold.len()]
    }

    /// The forward speed the plan's own verification lap did at `index`, or
    /// infinity where it has none.
    #[must_use]
    pub fn pace(&self, index: usize) -> f32 {
        if self.pace.is_empty() {
            return f32::INFINITY;
        }
        self.pace[index % self.pace.len()]
    }

    /// The braking the plan was built with.
    #[must_use]
    pub fn decel(&self) -> &Decel {
        &self.decel
    }

    /// The lowest target over the stretch a craft at `speed` covers in
    /// `lead_ticks` ticks from `index`: what a follower compares its speed to,
    /// since the airbrakes ramp in and reading only the sample under the craft
    /// starts every brake late.
    #[must_use]
    pub fn target_ahead(&self, index: usize, speed: f32, lead_ticks: f32, dt: f32) -> f32 {
        let n = self.target.len();
        if n == 0 {
            return f32::INFINITY;
        }
        let reach = speed.max(0.0) * lead_ticks * dt;
        let mut best = self.target[index % n];
        let mut travelled = 0.0;
        let mut at = index % n;
        for _ in 0..n {
            if travelled >= reach {
                break;
            }
            travelled += self.spacing[at];
            at = (at + 1) % n;
            best = best.min(self.target[at]);
        }
        best
    }

    /// Rebuilds [`Self::target`] from the ceilings: the backward braking pass.
    ///
    /// How fast could the craft be one sample earlier and still brake to this
    /// target? `v0^2 = v1^2 + 2 a ds`, `a` the measured deceleration at `v1`.
    /// Twice round the ring so a corner past the start line brakes the samples
    /// before it. A [`Self::holds`] sample keeps its own ceiling.
    fn derive_targets(&mut self) {
        let n = self.ceiling.len();
        self.target.clone_from(&self.ceiling);
        if n < 2 {
            return;
        }
        for _ in 0..2 {
            for k in (0..n).rev() {
                let next = self.target[(k + 1) % n];
                if self.hold[k] || !next.is_finite() {
                    continue;
                }
                let a = self.decel.at(next);
                let reach = (next * next + 2.0 * a * self.spacing[k]).sqrt();
                if reach < self.target[k] && reach < UNLIMITED {
                    self.target[k] = reach;
                }
            }
        }
    }
}

/// Throttle and symmetric brake for a craft at `speed` that wants `target`:
/// full throttle under it, over it the airbrakes come in in proportion to the
/// overspeed. **Ours, chosen, not measured.**
#[must_use]
pub fn longitudinal(speed: f32, target: f32) -> (f32, f32) {
    if speed <= target {
        return (1.0, 0.0);
    }
    let over = speed / target - 1.0;
    (0.0, (over * BRAKE_GAIN).min(1.0))
}

/// Folds a symmetric brake and a signed differential into the two airbrakes.
/// `differential` is `right - left`, `Driver::drive`'s convention; the
/// difference between the sides survives the brake whole.
#[must_use]
pub fn airbrakes(brake: f32, differential: f32) -> (f32, f32) {
    let width = differential.abs().min(1.0);
    let low = brake.clamp(0.0, 1.0 - width);
    let high = low + width;
    if differential >= 0.0 {
        (low, high)
    } else {
        (high, low)
    }
}

fn spacing_of(line: &Line) -> Vec<f32> {
    let n = line.len();
    (0..n)
        .map(|i| line.point(i).distance(line.point((i + 1) % n)))
        .collect()
}

/// Forward speed, the quantity the driver and the plan both speak.
fn forward_speed(state: &ShipState) -> f32 {
    state
        .body
        .linear_velocity
        .dot(state.body.forward())
        .max(0.0)
}

/// One craft being driven along the course: the state a checkpoint copies.
#[derive(Debug, Clone, Copy)]
struct Run {
    state: ShipState,
    driver: Driver,
    /// Samples travelled since the grid, signed.
    progress: i64,
    /// Ticks since the grid.
    tick: u32,
    /// Consecutive ticks under [`STALL_SPEED`].
    slow: u32,
    /// Where the craft started the previous tick, for the pad sweep; `None`
    /// on the grid and after a rescue, as the race's own is.
    last_position: Option<oag_core::math::Vec3>,
}

/// Why a tick counted as a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// In contact with a wall: what the clean-lap board counts.
    Wall,
    /// Further from its line point than [`Course::off_line`].
    OffLine,
    /// No longer racing: the pool ran out.
    Wrecked,
    /// Stopped for longer than [`STALL_TICKS`].
    Stalled,
    /// Something the race rescues a craft for that is not a wall: a reset
    /// volume touched, or longer in the air than [`Course::max_airborne`].
    Rescued,
}

/// What one tick did.
struct Tick {
    failure: Option<Failure>,
    contact: bool,
    /// The steering the driver asked for, for [`probe`].
    steer: f32,
}

impl<R: Raycaster + ?Sized> Course<'_, R> {
    fn env_at(&self, index: usize) -> Environment {
        let n = self.samples.len();
        if n == 0 {
            return self.env;
        }
        Environment {
            track_sample: self.samples[index % n],
            track_sample_next: self.samples[(index + 1) % n].or(self.samples[index % n]),
            ..self.env
        }
    }

    /// Steps `run` one tick: the driver steers, `longitudinal` decides
    /// throttle and brake, the physics moves it.
    fn tick(
        &self,
        run: &mut Run,
        craft: &Craft,
        tuning: &Tuning,
        ceiling: f32,
        speed_of: impl Fn(&Run, &ShipControls) -> (f32, f32),
    ) -> Tick {
        let ctx = Context {
            yaw_ceiling: Some(ceiling),
            ..Context::new(self.line, tuning)
        };
        let before = run.driver.index;
        let mut controls = run.driver.drive(&run.state, &ctx);
        let (thrust, brake) = speed_of(run, &controls);
        let differential = controls.airbrake_right - controls.airbrake_left;
        let (left, right) = airbrakes(brake, differential);
        controls.thrust = thrust;
        controls.airbrake_left = left;
        controls.airbrake_right = right;

        let index = run.driver.index as usize;
        let position_before = run.state.body.position;
        let env = Environment {
            pad_hit: (self.pad)(run.last_position, position_before),
            ..self.env_at(index)
        };
        run.last_position = Some(position_before);
        let evaluated = oag_physics::step(
            &mut run.state,
            &controls,
            &craft.handling,
            &env,
            self.raycaster,
            self.dt,
        );
        run.tick += 1;
        run.progress += ring_delta(before, run.driver.index, self.line.len());

        let speed = forward_speed(&run.state);
        run.slow = if speed < STALL_SPEED { run.slow + 1 } else { 0 };
        let contact = evaluated.wall.contacts > evaluated.wall.floor_contacts;
        let away = run.state.body.position.distance(self.line.point(index));
        let failure = if run.state.craft_state != CraftState::Racing {
            Some(Failure::Wrecked)
        } else if contact {
            Some(Failure::Wall)
        } else if away > self.off_line || !run.state.body.position.is_finite() {
            Some(Failure::OffLine)
        } else if run.state.time_airborne > self.max_airborne
            || (self.reset)(&run.state, &env, position_before)
        {
            // The race's own two rescues: a reset volume, and four seconds
            // without a hover contact. A plan counting only walls drove
            // `01_Track` at PHANTOM clean in isolation and was rescued four
            // times in the race, in the air off a crest.
            Some(Failure::Rescued)
        } else if run.tick > STALL_GRACE && run.slow > STALL_TICKS {
            Some(Failure::Stalled)
        } else {
            None
        };
        Tick {
            failure,
            contact,
            steer: controls.steer_x,
        }
    }
}

/// Signed samples from `from` to `to` round a ring of `n`, the short way.
fn ring_delta(from: u32, to: u32, n: usize) -> i64 {
    if n == 0 {
        return 0;
    }
    let n = n as i64;
    let mut d = (i64::from(to) - i64::from(from)).rem_euclid(n);
    if d > n / 2 {
        d -= n;
    }
    d
}

impl SpeedPlan {
    /// Drives `course` with `craft` until it has a plan the craft laps cleanly
    /// on, or the search runs out of tries.
    ///
    /// `tuning` steers; pass the top level's and handicap the plan at the
    /// driver, so one plan serves every difficulty.
    #[must_use]
    pub fn build<R: Raycaster + ?Sized>(
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
    ) -> (Self, Report) {
        Self::build_within(course, craft, tuning, STEP_BUDGET)
    }

    /// [`Self::build`] with its own step budget, for a caller that would
    /// rather give up early - a fork's route that cannot be lapped.
    #[must_use]
    pub fn build_within<R: Raycaster + ?Sized>(
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        budget: u64,
    ) -> (Self, Report) {
        let mut plan = Self::unlimited(course.line);
        let mut report = Report::default();
        if plan.len() < 3 {
            return (plan, report);
        }
        let yaw = crate::hull_yaw_ceiling(&craft.handling);
        let (decel, steps) = brake::calibrate(course, craft, tuning, yaw);
        report.steps += steps;
        plan.decel = decel;
        plan.derive_targets();

        for _ in 0..MAX_PASSES {
            report.passes += 1;
            plan.learn(course, craft, tuning, yaw, &mut report, budget);
            let verify = plan.verify(course, craft, tuning, yaw, &mut report);
            if verify == 0 || report.steps >= budget {
                break;
            }
        }
        (plan, report)
    }

    fn start_run(craft: &Craft) -> Run {
        Run {
            state: craft.start,
            driver: Driver {
                index: craft.start_index,
                ..Driver::default()
            },
            progress: 0,
            tick: 0,
            slow: 0,
            last_position: None,
        }
    }

    /// Two laps from the grid that follow the plan and change nothing; returns
    /// the failure ticks and fills the verification fields of `report`.
    fn verify<R: Raycaster + ?Sized>(
        &mut self,
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        yaw: f32,
        report: &mut Report,
    ) -> u32 {
        let n = self.len() as i64;
        let mut run = Self::start_run(craft);
        let (mut failures, mut contacts) = (0u32, 0u32);
        let mut lap_mark: Option<u32> = None;
        report.verify_lap_ticks = None;
        report.verify_respawns = 0;
        let mut pace = vec![f32::INFINITY; self.len()];
        let mut respawned_on_flying_lap = false;
        let limit = (4 * n as u64).max(4_000) as u32 * 2;
        while run.progress < 2 * n && run.tick < limit {
            let tick = course.tick(&mut run, craft, tuning, yaw, |run, _| {
                self.follow(run, course.dt)
            });
            report.steps += 1;
            contacts += u32::from(tick.contact);
            if run.progress >= n {
                pace[run.driver.index as usize] = forward_speed(&run.state);
            }
            if lap_mark.is_none() && run.progress >= n {
                lap_mark = Some(run.tick);
            }
            let Some(failure) = tick.failure else {
                continue;
            };
            if failure == Failure::Wall {
                failures += 1;
                continue;
            }
            // Put back past the spot, as the race's rescue would: on the
            // standing lap the underpowered case `learn` accepts, on the flying
            // lap a failure of the plan.
            report.verify_respawns += 1;
            if run.progress >= n {
                failures += 1;
                respawned_on_flying_lap = true;
            }
            let to = (run.driver.index as usize + RESPAWN_SKIP) % n as usize;
            run = Run {
                state: (course.respawn)(to),
                driver: Driver {
                    index: to as u32,
                    ..Driver::default()
                },
                progress: run.progress + RESPAWN_SKIP as i64,
                tick: run.tick,
                slow: 0,
                last_position: None,
            };
        }
        if run.progress >= 2 * n && !respawned_on_flying_lap {
            report.verify_lap_ticks = lap_mark.map(|mark| run.tick - mark);
        }
        self.pace = pace;
        report.verify_failures = failures;
        report.verify_contacts = contacts;
        failures
    }

    fn follow(&self, run: &Run, dt: f32) -> (f32, f32) {
        let speed = forward_speed(&run.state);
        if run.state.time_airborne > 0.0 {
            // No grip to brake against in the air; the same rule the driver
            // keeps. See `Driver::drive`.
            return (1.0, 0.0);
        }
        let target = self.target_ahead(run.driver.index as usize, speed, LEAD_TICKS, dt);
        longitudinal(speed, target)
    }

    /// One learning pass: drive two laps from the grid, lowering ceilings and
    /// rewinding wherever the craft fails. By what led to a failure:
    ///
    /// - **Off a jump, after braking for it**: the run-up is marked
    ///   [`Self::holds`] and its ceilings lifted; a craft falling short needs
    ///   more speed at the lip.
    /// - **Off a jump, or stopped, at full throttle**: no plan fixes it (the
    ///   standing start's first lap up `05_Track`'s crest). The craft is put
    ///   back past the spot as the race's rescue would, and the spot recorded.
    /// - **Anything else**: ceilings over the stretch are lowered to [`LOWER`]
    ///   of what the craft did, braking zones re-derived, and the run rewinds
    ///   to before the earliest one that moved.
    fn learn<R: Raycaster + ?Sized>(
        &mut self,
        course: &Course<'_, R>,
        craft: &Craft,
        tuning: &Tuning,
        yaw: f32,
        report: &mut Report,
        budget: u64,
    ) {
        let n = self.len();
        let goal = 2 * n as i64;
        let mut run = Self::start_run(craft);
        let mut checkpoints: Vec<Run> = vec![run];
        // What each recent tick did, so a failure knows what led to it.
        let mut recent: Vec<Trace> = Vec::new();
        let mut tries: BTreeMap<u32, u32> = BTreeMap::new();
        // Past this much progress, wall contact is forgiven: a site the search
        // gave up on is driven through rather than retried forever.
        let mut forgive_until = i64::MIN;

        while run.progress < goal && report.steps < budget {
            // Read inside the tick, after the driver has located itself, exactly
            // as `verify` does - and kept, so the trace knows what was asked.
            let asked = std::cell::Cell::new((1.0, 0.0));
            let tick = course.tick(&mut run, craft, tuning, yaw, |run, _| {
                asked.set(self.follow(run, course.dt));
                asked.get()
            });
            let (thrust, brake) = asked.get();
            report.steps += 1;
            recent.push(Trace {
                tick: run.tick,
                index: run.driver.index,
                speed: forward_speed(&run.state),
                airborne: run.state.time_airborne > 0.0,
                full: thrust >= 1.0 && brake <= 0.0,
            });
            if recent.len() > LOOKBACK_TICKS * 6 {
                recent.drain(..recent.len() - LOOKBACK_TICKS * 3);
            }
            if run.tick.is_multiple_of(CHECK_EVERY) {
                checkpoints.push(run);
            }
            let Some(failure) = tick.failure else {
                continue;
            };
            if run.progress <= forgive_until && failure == Failure::Wall {
                continue;
            }
            let site = run.driver.index / SITE;
            let count = tries.entry(site).or_insert(0);
            *count += 1;
            let exhausted = *count > MAX_TRIES;
            let cause = Cause::of(failure, run.state.time_airborne, &recent);
            if exhausted && failure == Failure::Wall {
                note_unresolved(report, run.driver.index, failure);
                forgive_until = run.progress + i64::from(SITE) * 2;
                continue;
            }
            if exhausted || cause == Cause::Underpowered {
                if run.progress >= n as i64 || exhausted {
                    note_unresolved(report, run.driver.index, failure);
                }
                report.respawns += 1;
                *count = 0;
                let to = (run.driver.index as usize + RESPAWN_SKIP) % n;
                run = Run {
                    state: (course.respawn)(to),
                    driver: Driver {
                        index: to as u32,
                        ..Driver::default()
                    },
                    progress: run.progress + RESPAWN_SKIP as i64,
                    tick: run.tick,
                    slow: 0,
                    last_position: None,
                };
                recent.clear();
                checkpoints.push(run);
                continue;
            }

            let old = self.target.clone();
            report.lowerings += match cause {
                Cause::ShortOfTheLip(takeoff) => self.hold_run_up(&recent[..takeoff]),
                _ => self.lower(&recent, tries.get(&site).copied().unwrap_or(1)),
            };
            self.derive_targets();

            // Rewind to before the earliest sample whose target moved, so the
            // craft meets the new braking zone from its start.
            let back = earliest_change(&old, &self.target, run.driver.index as usize);
            let resume = run.progress - back as i64 - 8;
            while checkpoints.len() > 1 && checkpoints.last().is_some_and(|c| c.progress > resume) {
                checkpoints.pop();
            }
            run = *checkpoints
                .last()
                .expect("the grid checkpoint is never popped");
            // The trace rewinds too, so the next failure still sees the run-up
            // that led to it, not only the ticks spent already too fast since
            // the checkpoint.
            recent.retain(|trace| trace.tick <= run.tick);
        }
    }

    /// Lowers the ceilings over the stretch `recent` covers, to [`LOWER`] of the
    /// speed the craft actually did there. Returns how many moved.
    fn lower(&mut self, recent: &[Trace], tries: u32) -> u32 {
        // Harder the more often this spot has failed: one [`LOWER`] for the
        // first three tries, two for the next three, and so on, so a corner
        // needing half the top speed fits inside [`MAX_TRIES`]. A loop of
        // multiplies, not `powi`: an intrinsic with no portability promise.
        let mut factor = LOWER;
        for _ in 0..tries / 3 {
            factor *= LOWER;
        }
        let mut moved = 0;
        // The last [`LOOKBACK_TICKS`] ticks **on the ground**: a craft that
        // left the road over a crest and hit a wall spent its recent past in the
        // air, where a ceiling means nothing; the takeoff speed has to come down.
        for trace in recent
            .iter()
            .rev()
            .filter(|trace| !trace.airborne)
            .take(LOOKBACK_TICKS)
        {
            let i = trace.index as usize;
            if self.hold[i] {
                continue;
            }
            // Of the lower of what it did and what it was allowed, so a retry
            // always moves.
            let lowered = (trace.speed.min(self.ceiling[i]) * factor).max(STALL_SPEED * 4.0);
            if lowered < self.ceiling[i] {
                self.ceiling[i] = lowered;
                moved += 1;
            }
        }
        moved
    }

    /// Marks the run-up to a takeoff as a stretch nothing brakes through, and
    /// lifts its ceilings. `run_up` ends at the last grounded tick before it.
    fn hold_run_up(&mut self, run_up: &[Trace]) -> u32 {
        let from = run_up.len().saturating_sub(LOOKBACK_TICKS * 2);
        let mut moved = 0;
        for trace in &run_up[from..] {
            let i = trace.index as usize;
            if !self.hold[i] {
                self.hold[i] = true;
                self.ceiling[i] = f32::INFINITY;
                moved += 1;
            }
        }
        moved
    }
}

/// One tick of a learning run, as a failure looks back on it.
#[derive(Debug, Clone, Copy)]
struct Trace {
    tick: u32,
    index: u32,
    speed: f32,
    airborne: bool,
    /// Whether the plan had the craft at full throttle with no brake.
    full: bool,
}

/// What a failure is put down to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cause {
    /// Too fast for what came next: lower the ceilings.
    TooFast,
    /// Airborne in the lookback after the plan braked on the run-up, which
    /// ended at this index into the trace: hold the run-up.
    ShortOfTheLip(usize),
    /// Stopped or off the circuit at full throttle all along: no plan helps.
    Underpowered,
}

impl Cause {
    fn of(failure: Failure, airborne: f32, recent: &[Trace]) -> Self {
        let from = recent.len().saturating_sub(LOOKBACK_TICKS);
        let window = &recent[from..];
        if failure == Failure::Wall && airborne < FELL_SECONDS {
            // A wall on the road or after a short hop is answered with less
            // speed: most are a corner taken too fast. Holding the run-up for
            // every airborne wall cost ten clean verifications across the 96
            // plans.
            return Self::TooFast;
        }
        if window.iter().any(|t| t.airborne) {
            // The takeoff is the end of the last grounded stretch before the
            // flight the craft is in, however long that flight has been.
            let last_air = recent.iter().rposition(|t| t.airborne).unwrap_or(0);
            let first_air = recent[..last_air]
                .iter()
                .rposition(|t| !t.airborne)
                .map_or(0, |grounded| grounded + 1);
            let run_up_from = first_air.saturating_sub(LOOKBACK_TICKS * 2);
            if recent[run_up_from..first_air].iter().any(|t| !t.full) {
                return Self::ShortOfTheLip(first_air);
            }
        }
        // Stopped with the throttle open all along is beyond a speed plan; an
        // excursion or a wreck is not (rescuing full-throttle excursions cost
        // four clean verifications).
        if failure == Failure::Stalled && window.iter().all(|t| t.full) {
            return Self::Underpowered;
        }
        Self::TooFast
    }
}

fn note_unresolved(report: &mut Report, index: u32, failure: Failure) {
    if !report.unresolved.iter().any(|&(at, _)| at == index) {
        report.unresolved.push((index, failure));
    }
}

/// How many samples back from `at` the targets differ, the furthest one.
fn earliest_change(old: &[f32], new: &[f32], at: usize) -> usize {
    let n = old.len();
    let mut furthest = 0;
    for back in 0..n {
        let i = (at + n - back % n) % n;
        if old[i] != new[i] {
            furthest = back;
        }
    }
    furthest.min(n / 2)
}
