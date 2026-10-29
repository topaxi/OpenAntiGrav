//! A one-shot burst fired off something that moves, whose particles carry part
//! of that motion - see [`Stage::play_carried`].
//!
//! Split out of `psys.rs`, which is at the 1,500-line ceiling.

use oag_core::math::Vec3;

use super::{Effect, Playing, Stage, System};

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
        if let Some(instance) = self.instances.get_mut(usize::from(playing.index)) {
            instance.system.carry(effect, velocity);
        }
        Some(playing)
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
}
