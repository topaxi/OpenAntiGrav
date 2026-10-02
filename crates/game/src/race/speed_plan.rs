//! Building an `oag_ai::SpeedPlan` for a race: the one place that knows both
//! the plan's inputs and where a race keeps them.
//!
//! The plan itself lives in `oag-ai` and knows nothing of `Spline`, `ai_order`
//! or the collision world's concrete type; this maps the race's own line,
//! samples and rules onto its `Course`. See `docs/gameplay/ai.md`, "The speed
//! plan".

use oag_physics::Environment;

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
        };
        let craft = oag_ai::plan::Craft {
            handling: ship.handling,
            start: ship.physics,
            start_index: ship.driver.index,
        };
        oag_ai::SpeedPlan::build(&course, &craft, &oag_ai::Tuning::default())
    }
}
