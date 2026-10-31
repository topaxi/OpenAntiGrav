//! Building an `oag_ai::SpeedPlan` for a race: the one place that knows both
//! the plan's inputs and where a race keeps them.
//!
//! The plan itself lives in `oag-ai` and knows nothing of `Spline`, `ai_order`
//! or the collision world's concrete type; this maps the race's own line,
//! samples and rules onto its `Course`. See `docs/gameplay/ai.md`, "The speed
//! plan".

use oag_physics::Environment;

use log::{info, warn};

use super::*;

/// The step budget a route's plan gets, against the ring's 120,000. **Chosen,
/// not measured**: Pulse's routes that lap clean build in 8,000-13,000 steps,
/// and `07_Track`'s centre ramp, which never laps, spent 145,728 on every load
/// before this.
const ROUTE_PLAN_BUDGET: u64 = 40_000;

/// The step budget re-learning the field's plan for one more hull gets
/// ([`oag_ai::SpeedPlan::adapt`]). **Chosen, not measured.**
const HULL_PLAN_BUDGET: u64 = 40_000;

/// The step budget checking the reference's ring plan on a hull its adapted
/// copy failed on ([`Race::fallback_ring`]) gets, on top of
/// [`HULL_PLAN_BUDGET`]. Two laps at each of [`FALLBACK_SHARES`] is about
/// 20,000 steps on Pulse. **Chosen, not measured.**
const FALLBACK_BUDGET: u64 = 40_000;

/// The further shares of the reference's ring plan a hull its adapted copy
/// failed on is verified at, in order, after the yaw ratio alone: steps of
/// `0.96`, the margin a hardened plan is learned at (`oag_ai::plan::harden`).
/// **Chosen, not measured.**
const FALLBACK_SHARES: [f32; 4] = [0.96, 0.9216, 0.884_736, 0.849_346_6];

/// The wall ticks a fallback verification may scrape and still be followed
/// at its share ([`Race::fallback_ring`]): the clean-lap gate's own contact
/// tolerance. Over the gate's 45 perturbed runs a Triakis on the reference at
/// its yaw share scraped 0-7 ticks on Pulse `07_Track` RAPIER, which the race
/// drives cleanly (0-8), and 55-94 on `06_Track` RAPIER, which it does not
/// (260-850). **Chosen, not measured.**
const FALLBACK_SCRAPE: u32 = 10;

/// What a hull drives round the ring, where [`Race::other_hulls`] decided.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) enum HullRing {
    /// The reference's plan at [`oag_ai::SpeedPlan::share_for`].
    #[default]
    Reference,
    /// A plan of its own: the reference's adapted, or the reference's at a
    /// lower share ([`oag_ai::SpeedPlan::at_share`]).
    Own(Box<oag_ai::SpeedPlan>),
}

/// One hull's own plans, where [`Race::other_hulls`] made them: the ring's
/// and each route's, `None` where this hull follows the reference's.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct HullPlans {
    pub(super) ring: HullRing,
    pub(super) routes: Vec<Option<oag_ai::SpeedPlan>>,
    /// Physics steps adapting each route's plan to this hull took.
    pub(super) route_steps: Vec<u64>,
}

/// Physics steps learning a race's plans took, as `oag_ai::plan::Report::steps`
/// counts them (calibration, every learning tick including rewound ones, every
/// verification tick, both searches where both ran). A diagnostic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanSteps {
    /// Each route's reference plan, in route order.
    pub routes: Vec<u64>,
    /// Each other hull's adaptation of each route's plan, hull by hull.
    pub hull_routes: Vec<Vec<u64>>,
    /// Whether each route's reference plan verified (is offered to the coin).
    pub routes_clean: Vec<bool>,
}

/// The plans of the hull `chosen` names in `hull_plans`, if any.
pub(super) fn hull_of(hull_plans: &[HullPlans], chosen: Option<u8>) -> Option<&HullPlans> {
    chosen.and_then(|k| hull_plans.get(usize::from(k)))
}

impl RaceSim {
    /// Whether anything in this race will read the speed plans; see
    /// [`Race::ensure_speed_plans`].
    fn needs_speed_plans(&self) -> bool {
        self.world.ship_count >= 2
            && (self.original_ai.is_none() || self.autopilot || !self.routes.is_empty())
    }

    /// Takes the plans a thread learned on a copy of this race.
    fn adopt_plans(&mut self, learned: LearnedPlans) {
        self.plans_built = true;
        self.speed_plan = learned.speed_plan;
        self.plan_reference = learned.plan_reference;
        self.hull_plans = learned.hull_plans;
        self.ship_plan = learned.ship_plan;
        self.plan_steps = learned.plan_steps;
        for (route, plan) in self.routes.iter_mut().zip(learned.route_plans) {
            route.plan = plan;
        }
    }
}

/// Learns the ring's, each hull's and the routes' speed plans into `sim`. The
/// one place they are learned, on the frame thread
/// ([`Race::ensure_speed_plans`]) or on a copy of `sim` on another
/// ([`Race::begin_speed_plans`]): the same function on the same state.
///
/// Three stages, each a pure function of the one before, so they finish in the
/// same plans whatever order the threads run in:
///
/// 1. the ring's plan, which picks the reference hull ([`Race::field_speed_plan`]);
/// 2. every route's plan on the reference ([`Race::route_speed_plans`]) and every
///    other hull's ring plan ([`Race::hull_ring_for`]), all at once: neither reads
///    the other;
/// 3. every other hull's copy of each route's plan ([`Race::hull_routes_for`]),
///    which needs that route's plan from stage 2.
///
/// Until 2026-10-31 stage 2 learned the routes one after another and then the hulls;
/// written back in route and hull order as then. See `docs/gameplay/ai.md`, "The
/// speed plan".
pub(super) fn learn_speed_plans(sim: &mut RaceSim) {
    sim.plans_built = true;
    sim.speed_plan = Race::field_speed_plan(sim);
    let hulls = Race::other_hulls(sim);
    let (learned, rings) = {
        let shared = &*sim;
        par_join(
            || Race::route_speed_plans(shared),
            || match (shared.speed_plan.as_ref(), shared.plan_reference) {
                (Some(reference_plan), Some(_)) => par_map(&hulls.0, |&handling| {
                    Race::hull_ring_for(shared, reference_plan, handling)
                }),
                _ => Vec::new(),
            },
        )
    };
    sim.plan_steps.routes = learned.iter().map(|(_, steps)| *steps).collect();
    sim.plan_steps.routes_clean = learned.iter().map(|(plan, _)| plan.is_some()).collect();
    for (route, (plan, _)) in learned.into_iter().enumerate() {
        sim.routes[route].plan = plan;
    }
    if sim.speed_plan.is_some() && sim.plan_reference.is_some() {
        let shared = &*sim;
        let routes = par_map(&hulls.0, |&handling| {
            Race::hull_routes_for(shared, handling)
        });
        sim.hull_plans = rings
            .into_iter()
            .zip(routes)
            .map(|(ring, (routes, route_steps))| HullPlans {
                ring,
                routes,
                route_steps,
            })
            .collect();
        sim.ship_plan = hulls.1;
    }
    sim.plan_steps.hull_routes = sim
        .hull_plans
        .iter()
        .map(|h| h.route_steps.clone())
        .collect();
}

/// [`learn_speed_plans`] as it ran before 2026-10-31, one stage after another
/// and the routes one after another: what the overlapped learner is checked
/// against.
#[cfg(test)]
pub(super) fn learn_speed_plans_serially(sim: &mut RaceSim) {
    sim.plans_built = true;
    sim.speed_plan = Race::field_speed_plan(sim);
    let learned: Vec<_> = (0..sim.routes.len())
        .map(|k| Race::route_speed_plan(sim, k))
        .collect();
    sim.plan_steps.routes = learned.iter().map(|(_, steps)| *steps).collect();
    sim.plan_steps.routes_clean = learned.iter().map(|(plan, _)| plan.is_some()).collect();
    for (route, (plan, _)) in learned.into_iter().enumerate() {
        sim.routes[route].plan = plan;
    }
    let (Some(reference_plan), Some(_)) = (sim.speed_plan.clone(), sim.plan_reference) else {
        return;
    };
    let (hulls, owner) = Race::other_hulls(sim);
    sim.hull_plans = hulls
        .iter()
        .map(|&handling| {
            let ring = Race::hull_ring_for(sim, &reference_plan, handling);
            let (routes, route_steps) = Race::hull_routes_for(sim, handling);
            HullPlans {
                ring,
                routes,
                route_steps,
            }
        })
        .collect();
    sim.ship_plan = owner;
    sim.plan_steps.hull_routes = sim
        .hull_plans
        .iter()
        .map(|h| h.route_steps.clone())
        .collect();
}

/// What [`learn_speed_plans`] wrote into a [`RaceSim`], and nothing else of it.
#[derive(Debug, PartialEq)]
pub(super) struct LearnedPlans {
    speed_plan: Option<oag_ai::SpeedPlan>,
    plan_reference: Option<oag_physics::Handling>,
    hull_plans: Vec<HullPlans>,
    ship_plan: [Option<u8>; oag_gameplay::MAX_SHIPS],
    route_plans: Vec<Option<oag_ai::SpeedPlan>>,
    plan_steps: PlanSteps,
}

impl LearnedPlans {
    #[cfg(test)]
    pub(super) fn speed_plan(&self) -> Option<&oag_ai::SpeedPlan> {
        self.speed_plan.as_ref()
    }

    /// How many route plans verified. For the tests.
    #[cfg(test)]
    pub(super) fn route_plan_count(&self) -> usize {
        self.route_plans.iter().flatten().count()
    }

    pub(super) fn of(sim: RaceSim) -> Self {
        Self {
            speed_plan: sim.speed_plan,
            plan_reference: sim.plan_reference,
            hull_plans: sim.hull_plans,
            ship_plan: sim.ship_plan,
            plan_steps: sim.plan_steps,
            route_plans: sim.routes.into_iter().map(|route| route.plan).collect(),
        }
    }
}

/// Plans being learned on a thread, and the state of the race they were
/// learned from.
#[derive(Debug)]
pub(super) struct SpeedPlanJob {
    pub(super) task: oag_thread::Task<LearnedPlans>,
    pub(super) inputs: PlanInputs,
}

/// Everything of a race the plans are learned from that can still change
/// between [`Race::start`] and the first tick: each grid slot's craft as
/// [`learn_speed_plans`] reads it. The rest it reads (the line, the routes, the
/// spline, the collision world, the pads, the class's gravity, the laws, the
/// damage rules, `dt`, the rescue distance) is fixed at [`Race::start`].
///
/// **Not the state hash**, which it was until 2026-10-31: the launch's own
/// setters move that without touching a plan input. A mod event's pilot writes
/// each opponent's `driver.pilot` ([`Race::set_opponent_pilot`]), and the
/// original driver ([`Race::use_original_ai`]) only decides whether a plan is
/// read, which [`Race::ensure_speed_plans`] asks itself. Split seating does
/// change one (`person_flown`, which orders the reference's candidates in
/// [`Race::field_speed_plan`]), and so does anything that moves a craft.
#[derive(Debug, PartialEq)]
pub(super) struct PlanInputs {
    ship_count: u8,
    /// Per slot: handling, the physics state's bits, the physics state by
    /// value (the bits miss nothing the hash covers; the value covers what
    /// it may not), the line index, active, flown by a person.
    slots: Vec<(
        oag_physics::Handling,
        u64,
        oag_physics::ShipState,
        u32,
        bool,
        bool,
    )>,
}

impl PlanInputs {
    pub(super) fn of(sim: &RaceSim) -> Self {
        let world = &sim.world;
        let slots = (0..usize::from(world.ship_count).min(oag_gameplay::MAX_SHIPS))
            .map(|slot| {
                let ship = &world.ships[slot];
                let mut hasher = oag_core::hash::StateHasher::new();
                oag_physics::probe::hash_state(&mut hasher, &ship.physics);
                (
                    ship.handling,
                    hasher.finish(),
                    ship.physics,
                    ship.driver.index,
                    ship.active,
                    world.person_flown(slot),
                )
            })
            .collect();
        Self {
            ship_count: world.ship_count,
            slots,
        }
    }
}

impl SpeedPlanJob {
    /// Whether `sim` is the race the plans were learned from.
    pub(super) fn matches(&self, sim: &RaceSim) -> bool {
        self.inputs == PlanInputs::of(sim)
    }
}

/// `f` over every item, each on a thread of its own natively, in order. On the
/// web the race starts on a worker already (`worker.rs`), so there they run one
/// after another. Each result is a pure function of its item, so the order
/// they finish in changes nothing.
fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let f = &f;
        std::thread::scope(|scope| {
            let jobs: Vec<_> = items
                .iter()
                .map(|item| scope.spawn(move || f(item)))
                .collect();
            jobs.into_iter()
                .map(|job| job.join().expect("a speed plan build panicked"))
                .collect()
        })
    }
    #[cfg(target_arch = "wasm32")]
    items.iter().map(f).collect()
}

/// `a` and `b` at once, natively; one after the other on the web, for
/// [`par_map`]'s reason.
fn par_join<A: Send, B: Send>(
    a: impl FnOnce() -> A + Send,
    b: impl FnOnce() -> B + Send,
) -> (A, B) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::scope(|scope| {
            let a = scope.spawn(a);
            let b = b();
            (a.join().expect("a speed plan build panicked"), b)
        })
    }
    #[cfg(target_arch = "wasm32")]
    (a(), b())
}

/// Whether a ring plan's verification came back clean.
fn ring_clean(report: &oag_ai::plan::Report) -> bool {
    report.verify_failures == 0 && report.verify_respawns == 0
}

/// Whether a plan adapted for another hull is fit to follow: no wall touched
/// on either verification lap and a flying lap completed without a rescue. A
/// rescue on the standing lap is allowed, the underpowered case the search
/// itself accepts (`oag_ai::SpeedPlan::learn`): the alternative is the
/// reference at the hull's yaw share, and on Pulse `01_Track` RAPIER that put
/// Triakis short of the jump at samples 330-354 on every lap, where its adapted
/// plan is rescued once, from the grid. **Chosen, not measured.**
fn adapted_clean(report: &oag_ai::plan::Report) -> bool {
    report.verify_failures == 0 && report.verify_lap_ticks.is_some()
}

/// Whether a route plan's did: stricter, see [`Race::route_speed_plans`].
fn route_clean(report: &oag_ai::plan::Report) -> bool {
    ring_clean(report) && report.verify_contacts == 0 && report.verify_lap_ticks.is_some()
}

impl Race {
    /// Builds the speed plan for the craft in `slot`, driven from its own grid
    /// pose with its own handling.
    ///
    /// The steering tuning is the top level's, `oag_ai::Tuning::default()`,
    /// whatever this race's difficulty: one plan serves every level, and a
    /// level handicaps it at the driver.
    #[must_use]
    pub fn build_speed_plan(&self, slot: usize) -> (oag_ai::SpeedPlan, oag_ai::plan::Report) {
        self.with_plan_course(slot, |course, craft| {
            oag_ai::SpeedPlan::build(course, craft, &oag_ai::Tuning::default())
        })
    }

    /// The plan for the **primary person's** craft, driven from slot 1's grid
    /// pose: the first candidate [`Self::field_speed_plan`] learns on, and what
    /// the speed plan ground truth asks of every layout.
    ///
    /// One plan, not one per person: a plan belongs to a hull, not to a slot.
    /// [`Self::other_hulls`] gives every distinct hull on the grid its own
    /// copy, a second person's included (it walks slots `0..ship_count`, not
    /// the opponents), so a second person's autopilot follows a plan for their
    /// own hull; this is the diagnostic that builds the first candidate's from
    /// scratch.
    #[must_use]
    pub fn build_player_speed_plan(&self) -> (oag_ai::SpeedPlan, oag_ai::plan::Report) {
        let player = self.sim.world.ships[self.sim.world.primary_slot()].handling;
        Self::with_plan_course_on(
            &self.sim,
            &self.sim.racing_line,
            &self.sim.ai_order,
            1,
            None,
            Some(player),
            |course, craft| oag_ai::SpeedPlan::build(course, craft, &oag_ai::Tuning::default()),
        )
    }

    /// Drives slot `slot`'s craft from the grid at a speed held to `speed`,
    /// with the plan's own steering, logging every tick. A diagnostic: see
    /// `oag_ai::plan::probe`.
    #[must_use]
    pub fn probe_speed_plan(
        &self,
        slot: usize,
        speed: f32,
        ticks: u32,
    ) -> Vec<oag_ai::plan::probe::Row> {
        self.with_plan_course(slot, |course, craft| {
            oag_ai::plan::probe::drive(course, craft, &oag_ai::Tuning::default(), speed, ticks, 32)
        })
    }

    /// The plan's `Course` and `Craft` for slot `slot`, handed to `f`.
    fn with_plan_course<T>(
        &self,
        slot: usize,
        f: impl FnOnce(&oag_ai::plan::Course<'_, CollisionWorld>, &oag_ai::plan::Craft) -> T,
    ) -> T {
        Self::with_plan_course_on(
            &self.sim,
            &self.sim.racing_line,
            &self.sim.ai_order,
            slot,
            None,
            None,
            f,
        )
    }

    /// The same on any line and its sample order - the ring's or a route's
    /// (`crate::routes`). `start` places a fresh craft at that line index
    /// instead of starting from `slot`'s own pose, for a line the grid is not
    /// on.
    fn with_plan_course_on<T>(
        sim: &RaceSim,
        line: &oag_ai::Line,
        order: &[u32],
        slot: usize,
        start: Option<usize>,
        flown_by: Option<oag_physics::Handling>,
        f: impl FnOnce(&oag_ai::plan::Course<'_, CollisionWorld>, &oag_ai::plan::Craft) -> T,
    ) -> T {
        let sample_at = |index: usize| {
            (!order.is_empty())
                .then(|| order[index % order.len()] as usize)
                .and_then(|i| sim.spline.sample(i))
        };
        let samples: Vec<Option<oag_physics::TrackSample>> = (0..line.len())
            .map(|index| sample_at(index).map(Spline::track_sample))
            .collect();
        let mut flown = sim.world.ships[slot];
        if let Some(handling) = flown_by {
            flown.handling = handling;
        }
        let ship = &flown;
        // The race's own rescue pose: the racing line at spawn height, the
        // whole physics state reset. See `Race::respawn`.
        let respawn = |index: usize| {
            let mut placed = *ship;
            if let Some(sample) = sample_at(index) {
                let height = rig_spawn_height(&placed.handling, None, &placed.physics.hover_rig);
                placed.place_at(Pose::from_sample(sample, sample.racing_line, height));
            }
            placed.physics
        };
        let handling = ship.handling;
        let reset =
            |state: &oag_physics::ShipState, env: &Environment, before: oag_core::math::Vec3| {
                oag_physics::reset::contact(state, &handling, env, &sim.collision, before).is_some()
            };
        // `Race::pad_sweep` and `Race::test_speedup_pads` without their
        // per-slot caches: the same sweep and the same first-pad-wins test.
        let pad = |from: Option<oag_core::math::Vec3>, to: oag_core::math::Vec3| {
            let sweep: Vec<oag_core::math::Vec3> = match from {
                Some(from)
                    if (0.0..Race::PAD_SWEEP_LIMIT).contains(&from.distance(to))
                        && from.distance(to) > 0.0 =>
                {
                    (1..=Race::PAD_SWEEP_STEPS)
                        .map(|step| from.lerp(to, step as f32 / Race::PAD_SWEEP_STEPS as f32))
                        .collect()
                }
                _ => vec![to],
            };
            sim.speedup_pads.iter().find_map(|pad| {
                sweep
                    .iter()
                    .any(|point| pad.contains(point.to_array()))
                    .then(|| pad.direction())
                    .flatten()
                    .map(oag_core::math::Vec3::from_array)
            })
        };
        let course = oag_ai::plan::Course {
            line,
            samples: &samples,
            raycaster: &sim.collision,
            env: Environment {
                class_gravity_scale: sim.class_gravity_scale,
                class_gravity_term: sim.class_gravity_term,
                laws: sim.physics_laws,
                damage_rules: sim.damage_rules(),
                barrel_rolls: sim.barrel_rolls,
                ..Environment::default()
            },
            dt: sim.dt,
            off_line: sim.rescue_distance,
            respawn: &respawn,
            reset: &reset,
            max_airborne: oag_race::recovery::AIRBORNE_RESET_SECONDS,
            pad: &pad,
        };
        let craft = match start {
            Some(index) => oag_ai::plan::Craft {
                handling: ship.handling,
                start: respawn(index),
                start_index: u32::try_from(index).unwrap_or(0),
            },
            None => oag_ai::plan::Craft {
                handling: ship.handling,
                start: ship.physics,
                start_index: ship.driver.index,
            },
        };
        f(&course, &craft)
    }

    /// Whether anything in this race will read the speed plans: an opponent on
    /// this project's driver (`--ai ours`, a fallback from
    /// [`Race::use_original_ai`], a mode the original does not fly), the
    /// player's autopilot, or a circuit with forks.
    ///
    /// **Forks always build them**, though the original driver reads no plan
    /// of the ring: a route is offered to the coin only when its plan verified
    /// ([`Race::steer_branching`]), and the route plans are learned on the
    /// ring plan's reference hull, so skipping them would change which way an
    /// original opponent goes round a fork. A circuit without one pays nothing
    /// (HD's default track: 3.0 to 3.7 s of a 5.2 to 5.9 s start).
    fn needs_speed_plans(&self) -> bool {
        self.sim.needs_speed_plans()
    }

    /// Learns the ring's, each hull's and the routes' speed plans, once, if
    /// anything will read them ([`Self::needs_speed_plans`]). Called at the
    /// head of [`Race::tick`] and by [`Self::speed_plan`] and
    /// [`Self::plan_of`]. On a race that has not ticked it is before any craft
    /// has moved, so the plans are learned from the grid `Race::start` used to
    /// learn them from and are bit-identical to what it built. (A player's
    /// autopilot switched on mid-race would learn them from wherever slot 1
    /// then is, and a caller that deactivates slots before the first tick
    /// learns them without those hulls.)
    ///
    /// A job started by [`Self::begin_speed_plans`] is adopted instead of
    /// learning again: joined if it is still running (the load screen waits
    /// for it, so a tick does not), and dropped if the race has changed under
    /// it ([`SpeedPlanJob::matches`]), in which case the plans are learned
    /// here as ever.
    pub fn ensure_speed_plans(&mut self) {
        if self.sim.plans_built || !self.needs_speed_plans() {
            return;
        }
        if let Some(mut job) = self.speed_plan_job.take() {
            let learned = job.task.join().and_then(Result::ok);
            if let Some(learned) = learned
                && job.matches(&self.sim)
            {
                self.sim.adopt_plans(learned);
                return;
            }
            warn!("the speed plans learned ahead no longer fit the race, learning them again");
        }
        learn_speed_plans(&mut self.sim);
    }

    /// Whether a job [`Self::begin_speed_plans`] started is still the race's
    /// plans (`Some(true)`), was started on a race that has since changed
    /// (`Some(false)`), or there is none. A diagnostic for the tests.
    #[must_use]
    pub fn speed_plan_job_fits(&self) -> Option<bool> {
        self.speed_plan_job
            .as_ref()
            .map(|job| job.matches(&self.sim))
    }

    /// [`Self::begin_speed_plans`] straight after [`Race::start`], before the
    /// launch has seated anyone or chosen the driver, for a circuit with a
    /// fork: that always reads the plans, whichever driver flies
    /// ([`Self::needs_speed_plans`]). The race-load thread calls it, so the
    /// plans learn beside the scene build and its warm-up rather than after
    /// them. A circuit without a fork waits for the launch's own call, which
    /// knows whether anything reads them.
    pub fn begin_speed_plans_ahead(&mut self) {
        if !self.sim.routes.is_empty() {
            self.begin_speed_plans();
        }
    }

    /// Starts learning the speed plans on a thread of their own, from a copy of
    /// the simulation as it stands, if anything will read them. For a caller
    /// that has finished setting the race up (the load screen, after
    /// [`Race::use_original_ai`]) and wants the first tick not to pay for them;
    /// poll [`Self::speed_plans_ready`]. The result is the same one
    /// [`Self::ensure_speed_plans`] would have learned on the frame thread:
    /// one function learns both, on a copy of the same state, and a race that
    /// changed in between ([`SpeedPlanJob::matches`]) learns them again.
    ///
    /// A job already running is kept when the race still fits it
    /// ([`PlanInputs`]) and replaced when it does not, so a launch that changed
    /// a plan input after an early start ([`Self::begin_speed_plans_ahead`])
    /// learns again here, behind the loading screen, and not on the first tick.
    ///
    /// Not on the web, where the race already starts on a worker.
    pub fn begin_speed_plans(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if !self.sim.plans_built && self.needs_speed_plans() {
            if let Some(job) = self.speed_plan_job.as_ref() {
                if job.matches(&self.sim) {
                    return;
                }
                info!("the race changed under the speed plans being learned ahead, starting again");
            }
            let inputs = PlanInputs::of(&self.sim);
            let mut copy = self.sim.clone();
            let task = oag_thread::Task::spawn("speed-plans", move || {
                learn_speed_plans(&mut copy);
                LearnedPlans::of(copy)
            });
            self.speed_plan_job = Some(SpeedPlanJob { task, inputs });
        }
    }

    /// Whether the plans [`Self::begin_speed_plans`] started are in, or nothing
    /// was started. Never waits.
    #[must_use]
    pub fn speed_plans_ready(&self) -> bool {
        self.speed_plan_job
            .as_ref()
            .is_none_or(|job| job.task.is_finished())
    }

    /// The plan the field drives, or `None`: the **reference** every other
    /// hull's plans are re-learned from ([`Self::other_hulls`]).
    ///
    /// A person's craft where its plan verifies clean (two laps from the grid
    /// with no wall touched and no rescue), else the AI slot with the most
    /// agile hull (`oag_ai::hull_yaw_ceiling`, highest first, slot order
    /// breaking a tie) whose plan does; a layout none of them solves keeps the
    /// corner model it drove before, rather than a plan known to put the craft
    /// into a wall. `05_Track` is the standing case; `speed_plan_ground_truth.rs`
    /// names all of them. The route plans are learned on its handling
    /// ([`Self::route_speed_plans`]). **Chosen, not measured.**
    fn field_speed_plan(sim: &mut RaceSim) -> Option<oag_ai::SpeedPlan> {
        if sim.world.ship_count < 2 || sim.racing_line.len() < 3 {
            return None;
        }
        let yaw = |sim: &RaceSim, slot: usize| {
            let ship = &sim.world.ships[slot];
            oag_ai::hull_yaw_ceiling(&ship.handling, &ship.physics.body)
        };
        // The people's craft first, in slot order; with one person in slot 0
        // that is `[0]` and then slots `1..` as it always was.
        let mut candidates: Vec<usize> = (0..usize::from(sim.world.ship_count))
            .filter(|&slot| sim.world.person_flown(slot))
            .collect();
        let mut opponents: Vec<usize> = (0..usize::from(sim.world.ship_count))
            .filter(|&slot| sim.world.ships[slot].active && !sim.world.person_flown(slot))
            .collect();
        opponents.sort_by(|&a, &b| yaw(sim, b).total_cmp(&yaw(sim, a)).then(a.cmp(&b)));
        candidates.extend(opponents);
        let mut tried: Vec<oag_physics::Handling> = Vec::new();
        for slot in candidates {
            let reference = sim.world.ships[slot].handling;
            if tried.contains(&reference) {
                continue;
            }
            tried.push(reference);
            let (plan, report) = Self::with_plan_course_on(
                sim,
                &sim.racing_line,
                &sim.ai_order,
                1,
                None,
                Some(reference),
                |course, craft| oag_ai::SpeedPlan::build(course, craft, &oag_ai::Tuning::default()),
            );
            let clean = ring_clean(&report);
            info!(
                "ai speed plan (learned on slot {slot}): {} steps, lap {:?} ticks, {} unresolved, \
                 {} of {} samples tight, verification {} - {}",
                report.steps,
                report.verify_lap_ticks,
                report.unresolved.len(),
                plan.tight_samples(),
                plan.len(),
                if clean { "clean" } else { "not clean" },
                if clean {
                    "the reference, followed"
                } else {
                    "not used, next candidate"
                }
            );
            if clean {
                sim.plan_reference = Some(reference);
                return Some(plan);
            }
        }
        None
    }

    /// Gives every hull other than the reference its own copy of the
    /// reference's ring and route plans, filling [`RaceSim::hull_plans`] and
    /// [`RaceSim::ship_plan`]; run once those are built.
    ///
    /// Each is the reference's re-learned for this hull
    /// ([`oag_ai::SpeedPlan::adapt`]: ceilings and takeoff run-ups kept, braking
    /// re-measured, lowered only where this hull fails), followed where it
    /// verifies clean; a hull whose copy does not follows the reference at
    /// [`oag_ai::SpeedPlan::share_for`] or below it ([`Self::fallback_ring`]).
    /// A hull's own plan searched from
    /// scratch was measured as the better plan where it verifies (Pulse
    /// Triakis at RAPIER: 0 wall contact ticks on `02_Track` against 288
    /// adapted) but costs a whole search per hull: 12 to 24 times the CPU of a
    /// race start (release, `06_Track` RAPIER 30 s against 1.3 s), so it waits
    /// on plans computed ahead of the race. Each hull on a thread natively
    /// ([`par_map`]), written back in slot order ([`learn_speed_plans`]).
    /// **Chosen, not measured.**
    ///
    /// This is the hull list: every distinct hull on the grid other than the
    /// reference's, in slot order, and which of them each slot flies. Empty
    /// without a reference.
    #[allow(clippy::type_complexity)]
    fn other_hulls(
        sim: &RaceSim,
    ) -> (
        Vec<oag_physics::Handling>,
        [Option<u8>; oag_gameplay::MAX_SHIPS],
    ) {
        let mut hulls: Vec<oag_physics::Handling> = Vec::new();
        let mut owner: [Option<u8>; oag_gameplay::MAX_SHIPS] = [None; oag_gameplay::MAX_SHIPS];
        let (Some(_), Some(reference)) = (sim.speed_plan.as_ref(), sim.plan_reference) else {
            return (hulls, owner);
        };
        for (slot, chosen) in owner
            .iter_mut()
            .enumerate()
            .take(usize::from(sim.world.ship_count))
        {
            let ship = &sim.world.ships[slot];
            if (slot > 0 && !ship.active) || ship.handling == reference {
                continue;
            }
            let hull = hulls
                .iter()
                .position(|h| *h == ship.handling)
                .unwrap_or_else(|| {
                    hulls.push(ship.handling);
                    hulls.len() - 1
                });
            *chosen = u8::try_from(hull).ok();
        }
        (hulls, owner)
    }

    /// What a hull whose adapted ring plan failed drives: the reference's at
    /// its yaw share where that laps on this hull touching the wall no more
    /// than [`FALLBACK_SCRAPE`] ticks, else at the first of [`FALLBACK_SHARES`]
    /// below it that does. Where none does, the share whose verification laps
    /// with the fewest wall ticks, the yaw share alone on a tie or where none
    /// laps at all.
    ///
    /// Until 2026-10-21 the reference at its yaw share was followed unchecked.
    /// On Pulse `06_Track` RAPIER the lone Triakis followed it at `0.774` in 14
    /// of the clean-lap gate's 45 perturbed runs and touched the wall 260-850
    /// ticks at samples 800-960, at full lock and drifting wide, where the
    /// reference hull's own verification lap went 160-183 units/s
    /// (`docs/gameplay/ai.md`, "The fallback plan is verified"). The corner
    /// model in place of an unclean share was measured and is worse: `01_Track`
    /// PHANTOM `Eliminated`, `07_Track` RAPIER and `14_Track` PHANTOM 180-240
    /// ticks a lap slower. **Chosen, not measured.**
    fn fallback_ring(
        sim: &RaceSim,
        reference_plan: &oag_ai::SpeedPlan,
        handling: oag_physics::Handling,
    ) -> HullRing {
        let tuning = oag_ai::Tuning::default();
        let mut steps = 0;
        // The least-failing share that lapped, as `(failures, share)`.
        let mut best: Option<(u32, f32)> = None;
        for share in std::iter::once(1.0).chain(FALLBACK_SHARES) {
            let report = Self::with_plan_course_on(
                sim,
                &sim.racing_line,
                &sim.ai_order,
                1,
                None,
                Some(handling),
                |course, craft| reference_plan.verify_at(course, craft, &tuning, share),
            );
            steps += report.steps;
            let clean = adapted_clean(&report);
            info!(
                "ai speed plan for a hull of turn {:.3}, the reference's at {share} of its yaw \
                 share: lap {:?} ticks, {} wall ticks, verification {}",
                handling.turning.amount,
                report.verify_lap_ticks,
                report.verify_failures,
                if clean {
                    "clean, followed"
                } else {
                    "not clean"
                },
            );
            if clean
                || report.verify_lap_ticks.is_some() && report.verify_failures <= FALLBACK_SCRAPE
            {
                best = Some((0, share));
                break;
            }
            if report.verify_lap_ticks.is_some()
                && best.is_none_or(|(failures, _)| report.verify_failures < failures)
            {
                best = Some((report.verify_failures, share));
            }
            if steps >= FALLBACK_BUDGET {
                break;
            }
        }
        match best {
            Some((_, share)) if share < 1.0 => {
                HullRing::Own(Box::new(reference_plan.at_share(share)))
            }
            _ => HullRing::Reference,
        }
    }

    /// One hull's ring plan: see [`Self::other_hulls`].
    fn hull_ring_for(
        sim: &RaceSim,
        reference_plan: &oag_ai::SpeedPlan,
        handling: oag_physics::Handling,
    ) -> HullRing {
        let tuning = oag_ai::Tuning::default();
        {
            let (adapted, report) = Self::with_plan_course_on(
                sim,
                &sim.racing_line,
                &sim.ai_order,
                1,
                None,
                Some(handling),
                |course, craft| reference_plan.adapt(course, craft, &tuning, HULL_PLAN_BUDGET),
            );
            let clean = adapted_clean(&report);
            info!(
                "ai speed plan for a hull of turn {:.3}, the reference's adapted: {} steps, \
                 lap {:?} ticks, verification {} - {}",
                handling.turning.amount,
                report.steps,
                report.verify_lap_ticks,
                if clean { "clean" } else { "not clean" },
                if clean {
                    "followed"
                } else {
                    "the reference's checked at its yaw share"
                }
            );
            if clean {
                HullRing::Own(Box::new(adapted))
            } else {
                Self::fallback_ring(sim, reference_plan, handling)
            }
        }
    }

    /// One hull's copy of each route's plan, `None` where the route has none
    /// or the copy did not verify, and the steps each took: see
    /// [`Self::other_hulls`]. Run once the routes' own plans are in.
    fn hull_routes_for(
        sim: &RaceSim,
        handling: oag_physics::Handling,
    ) -> (Vec<Option<oag_ai::SpeedPlan>>, Vec<u64>) {
        let tuning = oag_ai::Tuning::default();
        sim.routes
            .iter()
            .enumerate()
            .map(|(k, route)| {
                let Some(reference_route) = route.plan.as_ref() else {
                    return (None, 0);
                };
                let (plan, report) = Self::with_plan_course_on(
                    sim,
                    &route.line,
                    &route.order,
                    1,
                    Some(0),
                    Some(handling),
                    |course, craft| {
                        reference_route.adapt(course, craft, &tuning, ROUTE_PLAN_BUDGET)
                    },
                );
                let clean = route_clean(&report);
                info!(
                    "ai speed plan for a hull of turn {:.3}, route {k}, adapted: {} steps, \
                     verification {}",
                    handling.turning.amount,
                    report.steps,
                    if clean {
                        "clean, driven"
                    } else {
                        "not clean, the reference's driven"
                    }
                );
                (clean.then_some(plan), report.steps)
            })
            .unzip()
    }

    /// One plan per route (`crate::routes`), built and verified the way the
    /// ring's is, or `None` for a route that did not verify: not clean, with
    /// any wall contact (stricter than the ring, which may keep a plan that
    /// scrapes), **or clean without ever completing its verification lap** - a craft that
    /// stalls without touching a wall is neither a failure nor a respawn, and
    /// that is what `07_Track`'s centre ramp does to ours (2026-10-05). A route
    /// with no plan is not offered to the coin at all - see
    /// `Race::steer_branching`.
    ///
    /// **Started from the route line's own first sample rather than from the
    /// grid**: the grid can sit in the stretch a route bypasses (`05_Track`'s
    /// start is inside its fork), and a craft placed off its own line is a
    /// rescue, not a lap. Line index 0 is the route's merge, so the craft runs
    /// the whole shared stretch before it reaches the fork. Ours.
    ///
    /// Each route on a thread of its own natively ([`par_map`]), in route order.
    fn route_speed_plans(sim: &RaceSim) -> Vec<(Option<oag_ai::SpeedPlan>, u64)> {
        let routes: Vec<usize> = (0..sim.routes.len()).collect();
        par_map(&routes, |&k| Self::route_speed_plan(sim, k))
    }

    /// Route `k`'s plan: see [`Self::route_speed_plans`].
    fn route_speed_plan(sim: &RaceSim, k: usize) -> (Option<oag_ai::SpeedPlan>, u64) {
        if sim.world.ship_count < 2 {
            return (None, 0);
        }
        let route = &sim.routes[k];
        let start = 0;
        let (plan, report) = Self::with_plan_course_on(
            sim,
            &route.line,
            &route.order,
            1,
            Some(start),
            sim.plan_reference,
            |c, craft| {
                oag_ai::SpeedPlan::build_within(
                    c,
                    craft,
                    &oag_ai::Tuning::default(),
                    ROUTE_PLAN_BUDGET,
                )
            },
        );
        let clean = report.verify_failures == 0
            && report.verify_respawns == 0
            && report.verify_contacts == 0
            && report.verify_lap_ticks.is_some();
        info!(
            "ai speed plan, route {k}: {} steps, lap {:?} ticks, {} contact ticks, \
             verification {}",
            report.steps,
            report.verify_lap_ticks,
            report.verify_contacts,
            if clean {
                "clean, driven"
            } else {
                "did not lap clean, so no craft is sent down it"
            }
        );
        (clean.then_some(plan), report.steps)
    }

    /// The plan the field is following, if any, learned first if nothing has
    /// yet ([`Self::ensure_speed_plans`]). For a diagnostic that has to compare
    /// a craft's speed against the target it was given; `None` where the
    /// original driver flies a circuit without a fork.
    #[must_use]
    pub fn speed_plan(&mut self) -> Option<&oag_ai::SpeedPlan> {
        self.ensure_speed_plans();
        self.sim.speed_plan.as_ref()
    }

    /// What learning this race's route plans cost, learned first if nothing
    /// has yet. A diagnostic for `route_plan_board`.
    #[must_use]
    pub fn plan_steps(&mut self) -> &PlanSteps {
        self.ensure_speed_plans();
        &self.sim.plan_steps
    }
}
