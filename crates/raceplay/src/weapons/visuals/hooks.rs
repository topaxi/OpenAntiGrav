//! What a disc-backed test needs to see of the weapon visuals, and nothing the
//! game itself calls. Split out of `visuals.rs`, which is at the size limit.

use super::*;

impl Race {
    /// The LeachBeam's `WO_LEACHBEAM_ENERGY` instance's handle and the live
    /// particles of that effect on the stage, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn leach_energy_for_tests(&self) -> (Option<oag_fx::psys::Playing>, usize) {
        let alive = self
            .view
            .handles
            .get(Trigger::LeachbeamEnergy)
            .map_or(0, |effect| self.view.stage.alive_of_effect(effect));
        (self.view.leach_beam_effect, alive)
    }

    /// Puts a locked LeachBeam from `owner` on `target` into the world, off the
    /// race's own weapon table: a verification aid (`--force-leach-lock`, and
    /// the disc-backed tests), and nothing the game itself calls. A slot that
    /// is not racing is left alone, as [`Race::force_destroy`] leaves one, and
    /// so is a title that authors no LeachBeam; both answer `false`.
    pub fn force_leach_lock(&mut self, owner: u8, target: u8) -> bool {
        if !self.ship_active(usize::from(owner)) || !self.ship_active(usize::from(target)) {
            return false;
        }
        let Some(stats) = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::leach_beam)
        else {
            return false;
        };
        self.sim.world.leach_beam = Some(oag_weapons::projectile::leach_beam::Beam::locked(
            owner, target, &stats,
        ));
        true
    }

    /// The loaded particle effect a trigger plays, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn effect_for_tests(
        &self,
        trigger: Trigger,
    ) -> Option<std::sync::Arc<oag_fx::psys::Effect>> {
        self.view.handles.get(trigger).cloned()
    }
}
