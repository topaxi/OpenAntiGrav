//! A burst whose particle count is scaled - see [`System::ignite_counting`].
//!
//! Split out of `psys.rs`, which is at the 1,500-line ceiling.

use oag_core::math::Vec3;

use super::{Effect, Playing, Stage, System};

impl System {
    /// [`Self::ignite`] at the neutral severity with each emission's particle count scaled by
    /// `count_scale`: `ceil(count_scale * drawn count)` particles, none at `0.0`.
    ///
    /// HD's attached collision spark plays this way: `ShipCollisionFx_Trigger` writes its
    /// strength to scale-block slot 5, instance `+0x44`, and the emitter update multiplies the
    /// drawn per-emission count by it and rounds up (`0x002869b0`, `0x00286b04`;
    /// `docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md`, "Slot 5 is the particle
    /// count").
    pub fn ignite_counting(&mut self, effect: &Effect, point: Vec3, count_scale: f32) {
        self.ignite(effect, point, 1.0);
        self.count_scale = count_scale;
    }

    /// The particles one emission throws, from the count it drew: `ceil(count_scale * drawn)`.
    pub(super) fn emission_count(&self, drawn: f32) -> usize {
        (drawn * self.count_scale).ceil() as usize
    }

    /// The multiple each emission's particle count is scaled by - see [`Self::ignite_counting`].
    #[must_use]
    pub fn count_scale(&self) -> f32 {
        self.count_scale
    }

    /// Changes the multiple for the emissions that follow.
    pub fn set_count_scale(&mut self, count_scale: f32) {
        self.count_scale = count_scale;
    }
}

impl Stage {
    /// Sets an attached instance's particle-count multiple - see [`System::ignite_counting`].
    ///
    /// A no-op on a stale handle.
    pub fn set_count_scale(&mut self, playing: Playing, count_scale: f32) {
        if let Some(instance) = self.get_mut(playing) {
            instance.system.set_count_scale(count_scale);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{effect, run};
    use super::*;
    use oag_core::rng::Rng;

    fn three_a_tick() -> std::sync::Arc<Effect> {
        let mut spec = (*effect("WO_TEST_COUNT", true, 0.0)).clone();
        spec.emitters[0].per_emission = (3, 3);
        std::sync::Arc::new(spec)
    }

    /// **The multiple scales the per-emission count and rounds up**: `ceil(scale * 3)`, so a
    /// scale above zero never silences an emitter and zero does.
    #[test]
    fn the_count_scale_multiplies_each_emissions_particles_and_rounds_up() {
        let effect = three_a_tick();
        let mut alive = Vec::new();
        for scale in [0.0, 0.2, 1.0, 2.0] {
            let mut system = System::new();
            system.ignite_counting(&effect, Vec3::ZERO, scale);
            run(&mut system, &effect, 1, &mut Rng::new(1));
            alive.push(system.alive_count());
        }
        assert_eq!(alive, [0, 1, 3, 6]);
    }

    /// Plain ignition keeps the authored count, and re-igniting resets a scaled system.
    #[test]
    fn ignition_without_a_scale_keeps_the_authored_count() {
        let effect = three_a_tick();
        let mut system = System::new();
        system.ignite_counting(&effect, Vec3::ZERO, 2.0);
        system.ignite(&effect, Vec3::ZERO, 1.0);
        assert_eq!(system.count_scale(), 1.0);
        run(&mut system, &effect, 1, &mut Rng::new(1));
        assert_eq!(system.alive_count(), 3);
    }
}
