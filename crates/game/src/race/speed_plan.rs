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

impl Race {
    /// Builds the speed plan for the craft in `slot`, driven from its own grid
    /// pose with its own handling.
    ///
    /// The steering tuning is the top level's, `oag_ai::Tuning::default()`,
    /// whatever this race's difficulty: one plan serves every level, and a
    /// level handicaps it at the driver.
    #[must_use]
    pub fn build_speed_plan(&self, slot: usize) -> (oag_ai::SpeedPlan, oag_ai::plan::Report) {
        let line = &self.sim.racing_line;
        let samples: Vec<Option<oag_physics::TrackSample>> = (0..line.len())
            .map(|index| self.ai_sample(index).map(Spline::track_sample))
            .collect();
        let ship = &self.sim.world.ships[slot];
        // The race's own rescue pose: the racing line at spawn height, the
        // whole physics state reset. See `Race::respawn`.
        let respawn = |index: usize| {
            let mut placed = *ship;
            if let Some(sample) = self.ai_sample(index) {
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
        let craft = oag_ai::plan::Craft {
            handling: ship.handling,
            start: ship.physics,
            start_index: ship.driver.index,
        };
        oag_ai::SpeedPlan::build(&course, &craft, &oag_ai::Tuning::default())
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
            "ai speed plan: {} steps, {} unresolved, verification {} - {}",
            report.steps,
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
}
