//! Building an `oag_ai::SpeedPlan` for a race: the one place that knows both
//! the plan's inputs and where a race keeps them.
//!
//! The plan itself lives in `oag-ai` and knows nothing of `Spline`, `ai_order`
//! or the collision world's concrete type; this maps the race's own line,
//! samples and rules onto its `Course`. See `docs/gameplay/ai.md`, "The speed
//! plan".

use oag_physics::Environment;

use log::info;

use super::*;

/// The step budget a route's plan gets, against the ring's 120,000. **Chosen,
/// not measured**: Pulse's routes that lap clean build in 8,000-13,000 steps,
/// and `07_Track`'s centre ramp, which never laps, spent 145,728 on every load
/// before this.
const ROUTE_PLAN_BUDGET: u64 = 40_000;

/// The step budget re-learning the field's plan for one more hull gets
/// ([`oag_ai::SpeedPlan::adapt`]). **Chosen, not measured.**
const HULL_PLAN_BUDGET: u64 = 40_000;

/// One hull's own plans, where [`Race::adapt_hull_plans`] made them: the ring's
/// and each route's, `None` where this hull follows the reference's.
#[derive(Debug, Clone, Default)]
pub(super) struct HullPlans {
    pub(super) ring: Option<oag_ai::SpeedPlan>,
    pub(super) routes: Vec<Option<oag_ai::SpeedPlan>>,
}

/// The plans of the hull `chosen` names in `hull_plans`, if any.
pub(super) fn hull_of(hull_plans: &[HullPlans], chosen: Option<u8>) -> Option<&HullPlans> {
    chosen.and_then(|k| hull_plans.get(usize::from(k)))
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

    /// The plan for the **player's** craft, driven from slot 1's grid pose: the
    /// first candidate [`Self::field_speed_plan`] learns on, and what the speed
    /// plan ground truth asks of every layout.
    #[must_use]
    pub fn build_player_speed_plan(&self) -> (oag_ai::SpeedPlan, oag_ai::plan::Report) {
        let player = self.sim.world.ships[0].handling;
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
                let height = spawn_height(&placed.handling);
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
                damage_rules: sim.damage_rules(),
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
    /// **Forks always build them**, though the original driver reads no plan:
    /// a route is offered to the coin only when its plan verified
    /// ([`Race::steer_branching`]), and the route plans are learned on the
    /// ring plan's reference hull, so skipping them would change which way an
    /// original opponent goes round a fork. A circuit without one pays nothing
    /// (HD's default track: 3.0 to 3.7 s of a 5.2 to 5.9 s start).
    fn needs_speed_plans(&self) -> bool {
        self.sim.world.ship_count >= 2
            && (self.sim.original_ai.is_none() || self.sim.autopilot || !self.sim.routes.is_empty())
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
    pub fn ensure_speed_plans(&mut self) {
        if self.sim.plans_built || !self.needs_speed_plans() {
            return;
        }
        self.sim.plans_built = true;
        self.sim.speed_plan = self.field_speed_plan();
        for (route, plan) in self
            .route_speed_plans()
            .into_iter()
            .enumerate()
            .collect::<Vec<_>>()
        {
            self.sim.routes[route].plan = plan;
        }
        self.adapt_hull_plans();
    }

    /// The plan the field drives, or `None`: the **reference** every other
    /// hull's plans are re-learned from ([`Self::adapt_hull_plans`]).
    ///
    /// The player's craft where its plan verifies clean (two laps from the grid
    /// with no wall touched and no rescue), else the AI slot with the most
    /// agile hull (`oag_ai::hull_yaw_ceiling`, highest first, slot order
    /// breaking a tie) whose plan does; a layout none of them solves keeps the
    /// corner model it drove before, rather than a plan known to put the craft
    /// into a wall. `05_Track` is the standing case; `speed_plan_ground_truth.rs`
    /// names all of them. The route plans are learned on its handling
    /// ([`Self::route_speed_plans`]). **Chosen, not measured.**
    pub(super) fn field_speed_plan(&mut self) -> Option<oag_ai::SpeedPlan> {
        if self.sim.world.ship_count < 2 || self.sim.racing_line.len() < 3 {
            return None;
        }
        let yaw = |race: &Self, slot: usize| {
            let ship = &race.sim.world.ships[slot];
            oag_ai::hull_yaw_ceiling(&ship.handling, &ship.physics.body)
        };
        let mut candidates: Vec<usize> = vec![0];
        let mut opponents: Vec<usize> = (1..usize::from(self.sim.world.ship_count))
            .filter(|&slot| self.sim.world.ships[slot].active)
            .collect();
        opponents.sort_by(|&a, &b| yaw(self, b).total_cmp(&yaw(self, a)).then(a.cmp(&b)));
        candidates.extend(opponents);
        let mut tried: Vec<oag_physics::Handling> = Vec::new();
        for slot in candidates {
            let reference = self.sim.world.ships[slot].handling;
            if tried.contains(&reference) {
                continue;
            }
            tried.push(reference);
            let (plan, report) = Self::with_plan_course_on(
                &self.sim,
                &self.sim.racing_line,
                &self.sim.ai_order,
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
                self.sim.plan_reference = Some(reference);
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
    /// [`oag_ai::SpeedPlan::share_for`]. A hull's own plan searched from
    /// scratch was measured as the better plan where it verifies (Pulse
    /// Triakis at RAPIER: 0 wall contact ticks on `02_Track` against 288
    /// adapted) but costs a whole search per hull: 12 to 24 times the CPU of a
    /// race start (release, `06_Track` RAPIER 30 s against 1.3 s), so it waits
    /// on plans computed ahead of the race. Each hull on a thread natively
    /// ([`par_map`]), written back in slot order. **Chosen, not measured.**
    pub(super) fn adapt_hull_plans(&mut self) {
        let (Some(reference_plan), Some(reference)) =
            (self.sim.speed_plan.as_ref(), self.sim.plan_reference)
        else {
            return;
        };
        let mut hulls: Vec<oag_physics::Handling> = Vec::new();
        let mut owner: [Option<u8>; oag_gameplay::MAX_SHIPS] = [None; oag_gameplay::MAX_SHIPS];
        for (slot, chosen) in owner
            .iter_mut()
            .enumerate()
            .take(usize::from(self.sim.world.ship_count))
        {
            let ship = &self.sim.world.ships[slot];
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
        let sim = &self.sim;
        let built = par_map(&hulls, |&handling| {
            Self::hull_plans_for(sim, reference_plan, handling)
        });
        self.sim.hull_plans = built;
        self.sim.ship_plan = owner;
    }

    /// One hull's plans: see [`Self::adapt_hull_plans`].
    fn hull_plans_for(
        sim: &RaceSim,
        reference_plan: &oag_ai::SpeedPlan,
        handling: oag_physics::Handling,
    ) -> HullPlans {
        let tuning = oag_ai::Tuning::default();
        let ring = {
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
                    "the reference's followed at its yaw share"
                }
            );
            clean.then_some(adapted)
        };
        let routes = sim
            .routes
            .iter()
            .enumerate()
            .map(|(k, route)| {
                let reference_route = route.plan.as_ref()?;
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
                clean.then_some(plan)
            })
            .collect();
        HullPlans { ring, routes }
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
    pub(super) fn route_speed_plans(&self) -> Vec<Option<oag_ai::SpeedPlan>> {
        if self.sim.world.ship_count < 2 {
            return vec![None; self.sim.routes.len()];
        }
        self.sim
            .routes
            .iter()
            .enumerate()
            .map(|(k, route)| {
                let start = 0;
                let (plan, report) = Self::with_plan_course_on(
                    &self.sim,
                    &route.line,
                    &route.order,
                    1,
                    Some(start),
                    self.sim.plan_reference,
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
                clean.then_some(plan)
            })
            .collect()
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
}
