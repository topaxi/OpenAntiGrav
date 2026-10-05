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
        self.with_plan_course_on(&self.sim.racing_line, &self.sim.ai_order, slot, None, f)
    }

    /// The same on any line and its sample order - the ring's or a route's
    /// (`crate::routes`). `start` places a fresh craft at that line index
    /// instead of starting from `slot`'s own pose, for a line the grid is not
    /// on.
    fn with_plan_course_on<T>(
        &self,
        line: &oag_ai::Line,
        order: &[u32],
        slot: usize,
        start: Option<usize>,
        f: impl FnOnce(&oag_ai::plan::Course<'_, CollisionWorld>, &oag_ai::plan::Craft) -> T,
    ) -> T {
        let sample_at = |index: usize| {
            (!order.is_empty())
                .then(|| order[index % order.len()] as usize)
                .and_then(|i| self.sim.spline.sample(i))
        };
        let samples: Vec<Option<oag_physics::TrackSample>> = (0..line.len())
            .map(|index| sample_at(index).map(Spline::track_sample))
            .collect();
        let ship = &self.sim.world.ships[slot];
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
                oag_physics::reset::contact(state, &handling, env, &self.sim.collision, before)
                    .is_some()
            };
        // `Race::pad_sweep` and `Race::test_speedup_pads` without their
        // per-slot caches: the same sweep and the same first-pad-wins test.
        let pad = |from: Option<oag_core::math::Vec3>, to: oag_core::math::Vec3| {
            let sweep: Vec<oag_core::math::Vec3> = match from {
                Some(from)
                    if (0.0..Self::PAD_SWEEP_LIMIT).contains(&from.distance(to))
                        && from.distance(to) > 0.0 =>
                {
                    (1..=Self::PAD_SWEEP_STEPS)
                        .map(|step| from.lerp(to, step as f32 / Self::PAD_SWEEP_STEPS as f32))
                        .collect()
                }
                _ => vec![to],
            };
            self.sim.speedup_pads.iter().find_map(|pad| {
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
            raycaster: &self.sim.collision,
            env: Environment {
                class_gravity_scale: self.sim.class_gravity_scale,
                damage_rules: self.sim.damage_rules(),
                ..Environment::default()
            },
            dt: self.sim.dt,
            off_line: self.sim.rescue_distance,
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

    /// The plan the field drives, or `None`.
    ///
    /// Built from slot 1, the first opponent: every opponent flies the same
    /// handling (`Race::start` seats them all with it), so one plan serves the
    /// grid. **Only a plan that verified clean is used** - two laps from the
    /// grid with no wall touched and no rescue - so a layout the search could
    /// not solve keeps the corner model it drove before, rather than a plan
    /// known to put the craft into a wall. `05_Track` is the standing case;
    /// `speed_plan_ground_truth.rs` names all of them.
    pub(super) fn field_speed_plan(&self) -> Option<oag_ai::SpeedPlan> {
        if self.sim.world.ship_count < 2 || self.sim.racing_line.len() < 3 {
            return None;
        }
        let (plan, report) = self.build_speed_plan(1);
        let clean = report.verify_failures == 0 && report.verify_respawns == 0;
        info!(
            "ai speed plan: {} steps, lap {:?} ticks, {} unresolved, verification {} - {}",
            report.steps,
            report.verify_lap_ticks,
            report.unresolved.len(),
            if clean { "clean" } else { "not clean" },
            if clean {
                "followed"
            } else {
                "not used, corner model instead"
            }
        );
        clean.then_some(plan)
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
                let (plan, report) = self.with_plan_course_on(
                    &route.line,
                    &route.order,
                    1,
                    Some(start),
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

    /// The plan the field is following, if any. Read-only, for a diagnostic
    /// that has to compare a craft's speed against the target it was given.
    #[must_use]
    pub fn speed_plan(&self) -> Option<&oag_ai::SpeedPlan> {
        self.sim.speed_plan.as_ref()
    }
}
