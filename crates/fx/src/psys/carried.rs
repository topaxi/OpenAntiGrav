//! A one-shot burst fired off something that moves, whose particles carry part
//! of that motion - see [`Stage::play_carried`].
//!
//! Split out of `psys.rs`, which is at the 1,500-line ceiling.

use oag_core::math::Vec3;

use super::{Effect, Playing, Stage, System, Tint};

impl System {
    /// Gives every running root emitter an inherited velocity of `velocity`
    /// times that emitter's own `+0x4d0` ([`super::EmitterSpec::velocity_inherit`]),
    /// which each particle it spawns from now on adds to its own ejection.
    ///
    /// **Wipeout HD's law, read on its executable (confidence 75).** A system
    /// instance keeps a velocity at `+0x160`; the per-particle spawn
    /// (`FUN_00289890`, `0x002899d4`-`0x00289a18`) adds `instance+0x160 *
    /// record+0x4d0` to the newborn's velocity, so the record's `+0x4d0` scales
    /// what a *root's* particles inherit as well as a child's. `Trail_HitShipEffect`
    /// (`0x002d9fb4`-`0x002d9ff4`) and `ShipCollisionFx_Trigger` (`0x002d9b8c`-`0x002d9bb8`)
    /// write the craft's own velocity there as they spawn. See
    /// `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`, "The trail-hit
    /// burst, measured".
    pub fn carry(&mut self, effect: &Effect, velocity: Vec3) {
        for state in &mut self.emitters {
            if state.active && !state.is_child {
                let spec = &effect.emitters[usize::from(state.spec)];
                state.inherited = velocity * spec.velocity_inherit;
            }
        }
    }
}

impl Stage {
    /// Fires `effect` at `point` as [`Self::play`] does, with its root
    /// particles carrying `velocity` (world units per second) scaled by each
    /// emitter's own `+0x4d0` - see [`System::carry`].
    pub fn play_carried(
        &mut self,
        effect: &std::sync::Arc<Effect>,
        point: Vec3,
        scale: f32,
        velocity: Vec3,
    ) -> Option<Playing> {
        let playing = self.play(effect, point, scale)?;
        self.carry(playing, effect, velocity);
        Some(playing)
    }

    /// [`Self::play_carried`] through [`Self::play_expendable`], drawn in `tint`'s hue when given.
    pub fn play_carried_expendable(
        &mut self,
        effect: &std::sync::Arc<Effect>,
        point: Vec3,
        scale: f32,
        velocity: Vec3,
        tint: Option<Tint>,
    ) -> Option<Playing> {
        let playing = self.play_expendable(effect, point, scale)?;
        self.carry(playing, effect, velocity);
        if let Some(instance) = self.instances.get_mut(usize::from(playing.index)) {
            instance.tint = tint;
        }
        Some(playing)
    }

    fn carry(&mut self, playing: Playing, effect: &Effect, velocity: Vec3) {
        if let Some(instance) = self.instances.get_mut(usize::from(playing.index)) {
            instance.system.carry(effect, velocity);
        }
    }
}

#[cfg(test)]
mod tests {
    use oag_core::Rng;

    use super::super::tests::{DT, effect};
    use super::*;

    /// The farthest a live particle of a one-tick burst has travelled along
    /// `+X`, ten ticks after it was carried at `velocity_inherit` and 60 units
    /// a second.
    fn drift_after_ten_ticks(velocity_inherit: f32) -> f32 {
        let mut built = (*effect("burst", false, 1.0)).clone();
        built.emitters[0].velocity_inherit = velocity_inherit;
        let burst = std::sync::Arc::new(built);
        let mut stage = Stage::new();
        stage
            .play_carried(&burst, Vec3::ZERO, 1.0, Vec3::new(60.0, 0.0, 0.0))
            .expect("a free stage plays");
        let mut rng = Rng::new(7);
        for _ in 0..10 {
            stage.advance(DT, &mut rng);
        }
        stage
            .instances
            .iter()
            .flat_map(|instance| instance.system.particles.iter())
            .filter(|particle| particle.life > 0.0)
            .map(|particle| particle.position.x)
            .fold(0.0, f32::max)
    }

    /// **The root's own `+0x4d0` scales what its particles inherit.** A still
    /// emitter (no ejection speed) carried at 60 units a second with
    /// `+0x4d0 = 0.5` drifts at 30: about 5 units in ten ticks. With `+0x4d0
    /// = 0` (`CORE_IMPACT`'s value) the same call leaves it where it spawned,
    /// so the law is the record's and not the call's.
    #[test]
    fn a_carried_burst_drifts_by_its_own_inherit_share() {
        let half = drift_after_ten_ticks(0.5);
        assert!((4.0..=5.5).contains(&half), "drifted {half}");
        assert_eq!(drift_after_ten_ticks(0.0), 0.0);
    }

    /// The colour of every vertex `stage` draws, additive list.
    fn drawn_colours(stage: &Stage) -> Vec<[f32; 3]> {
        let (additive, _) = stage.vertices(Vec3::X, Vec3::Y);
        additive
            .iter()
            .map(|v| [v.colour[0], v.colour[1], v.colour[2]])
            .collect()
    }

    fn a_blue_burst() -> std::sync::Arc<Effect> {
        let mut built = (*effect("blue", false, 1.0)).clone();
        *built.emitters[0].palette = [[0.1, 0.25, 1.0, 1.0]; 256];
        std::sync::Arc::new(built)
    }

    /// **A tint changes the hue of what the instance draws and nothing else's, and a recycled
    /// slot does not keep it.** Red to green: dropping the call's tint leaves the blue.
    #[test]
    fn a_tint_recolours_its_own_instance_only_and_a_recycled_slot_forgets_it() {
        let burst = a_blue_burst();
        let tint = Tint::towards([1.0, 0.1, 0.1]);
        let mut stage = Stage::new();
        stage
            .play_carried_expendable(&burst, Vec3::ZERO, 1.0, Vec3::ZERO, Some(tint))
            .expect("a free stage plays");
        stage
            .play_carried_expendable(&burst, Vec3::X, 1.0, Vec3::ZERO, None)
            .expect("a free stage plays");
        let mut rng = Rng::new(1);
        stage.advance(DT, &mut rng);
        let drawn = drawn_colours(&stage);
        assert_eq!(drawn.len(), 12, "two quads");
        assert_eq!(drawn[0], tint.apply([0.1, 0.25, 1.0]));
        assert_eq!(
            drawn[6],
            [0.1, 0.25, 1.0],
            "the untinted burst is as authored"
        );
        assert!(drawn[0][0] > drawn[0][2], "red now");

        let mut stage = Stage::new();
        for _ in 0..crate::psys::MAX_INSTANCES {
            stage.play_carried_expendable(&burst, Vec3::ZERO, 1.0, Vec3::ZERO, Some(tint));
        }
        stage.play_carried_expendable(&burst, Vec3::ZERO, 1.0, Vec3::ZERO, None);
        stage.advance(DT, &mut Rng::new(1));
        let plain = drawn_colours(&stage)
            .into_iter()
            .filter(|c| *c == [0.1, 0.25, 1.0])
            .count();
        assert_eq!(plain, 6, "the recycled slot took no tint with it");
    }
}
